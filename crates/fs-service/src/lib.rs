#![forbid(unsafe_code)]
//! Layanan operasi berkas aman di dalam akar worktree untuk Agent IDE (`IT-201`, `IT-202`, `IT-213`).
//!
//! Menegakkan isolasi akar worktree, pencegahan symlink escape (`IT1`), larangan penuh akses/tulis `.git/`
//! (`IT2`), kontrol konkurensi ETag SHA-256 (`IT9`), cadangan undo satu langkah (`0600`), serta batas ukuran.

pub mod watcher;

use std::fmt::Write as _;
use std::fs::{self, Metadata};
use std::path::{Component, Path, PathBuf};
use std::time::UNIX_EPOCH;

use base64::engine::general_purpose::STANDARD as BASE64_STD;
use base64::Engine as _;
use latch_core::id::{new_id, now_ms};
use latch_core::redact::redact_secrets;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

/// Batas maksimum body penulisan berkas (`IDE_API.md` §11: 5 MiB).
pub const MAX_WRITE_BYTES: usize = 5 * 1024 * 1024;
/// Batas maksimum berkas teks yang dimuat langsung ke editor (`IT-208`: 2 MiB).
pub const MAX_READ_TEXT_BYTES: usize = 2 * 1024 * 1024;
/// Batas maksimum unduhan berkas mentah/biner (`/fs/raw`: 16 MiB).
pub const MAX_RAW_BYTES: usize = 16 * 1024 * 1024;
/// Batas maksimum entri per permintaan pohon direktori (`IDE_API.md` §11).
pub const MAX_TREE_ENTRIES: usize = 500;

/// Kesalahan terstruktur pada `latch-fs-service`.
#[derive(Debug, Error)]
pub enum FsServiceError {
    /// Jalur keluar dari akar worktree, jalur absolut, atau symlink mengarah keluar akar (`400 path_outside_root`).
    #[error("path resolves outside worktree root: {0}")]
    PathOutsideRoot(String),
    /// Akses atau penulisan ke direktori `.git/` dilarang keras (`403 git_dir_protected`).
    #[error("access to .git directory is forbidden: {0}")]
    GitDirProtected(String),
    /// Berkas atau direktori tidak ditemukan (`404 not_found`).
    #[error("path not found: {0}")]
    NotFound(String),
    /// Berkas atau direktori sudah ada saat `create`/`mkdir` (`409 already_exists`).
    #[error("path already exists: {0}")]
    AlreadyExists(String),
    /// Konflik konkurensi ETag saat `PUT /fs/file` (`409 etag_mismatch`).
    #[error("etag mismatch; file changed on disk")]
    EtagMismatch {
        /// ETag terbaru di disk.
        current_etag: String,
        /// Isi teks terbaru di disk (jika berupa teks <= 2 MiB) untuk resolusi 3-arah.
        current_content: Option<String>,
    },
    /// Ukuran berkas melampaui batas aman (`413 payload_too_large`).
    #[error("payload exceeds maximum limit of {limit} bytes (got {actual})")]
    PayloadTooLarge {
        /// Batas maksimum byte.
        limit: usize,
        /// Ukuran aktual byte.
        actual: usize,
    },
    /// Permintaan operasi tidak valid (`400 invalid_request`).
    #[error("invalid filesystem request: {0}")]
    InvalidRequest(String),
    /// Kesalahan I/O OS.
    #[error("filesystem I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Jenis entri pada pohon direktori.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsEntryKind {
    /// Berkas reguler.
    File,
    /// Direktori.
    Dir,
    /// Tautan simbolik yang tetap berada di dalam akar worktree.
    Symlink,
}

/// Satu entri dalam respons `GET /sessions/{sid}/fs/tree`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsTreeEntry {
    /// Nama berkas/direktori.
    pub name: String,
    /// Jalur relatif terhadap akar worktree (memakai `/`).
    pub path: String,
    /// Jenis entri (`file`, `dir`, `symlink`).
    pub kind: FsEntryKind,
    /// Ukuran berkas dalam byte (0 untuk direktori).
    pub size: u64,
    /// Waktu modifikasi terakhir (UTC epoch ms).
    pub mtime_ms: i64,
    /// Apakah entri berupa symlink.
    pub is_symlink: bool,
    /// Flag status Git opsional (diisi lapisan daemon bila tersedia).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_status: Option<String>,
    /// Menandakan berkas pernah diubah agen pada sesi aktif (`IT-207`).
    #[serde(default)]
    pub agent_touched: bool,
    /// Anak direktori bila `depth > 1`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub children: Vec<FsTreeEntry>,
}

/// Respons `GET /sessions/{sid}/fs/tree`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsTreeResponse {
    /// Jalur direktori relatif yang dipindai (`""` untuk akar).
    pub path: String,
    /// Daftar entri terurut (direktori dahulu, lalu abjad).
    pub entries: Vec<FsTreeEntry>,
    /// Benar bila daftar dipotong karena mencapai batas paginasi.
    pub truncated: bool,
}

/// Respons `GET /sessions/{sid}/fs/stat`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsStatResponse {
    /// Jalur relatif terhadap akar worktree.
    pub path: String,
    /// Jenis entri.
    pub kind: FsEntryKind,
    /// Ukuran dalam byte.
    pub size: u64,
    /// Waktu modifikasi terakhir (UTC epoch ms).
    pub mtime_ms: i64,
    /// Hash SHA-256 isi berkas (`None` untuk direktori atau berkas sangat besar).
    pub etag: Option<String>,
    /// Apakah berkas kemungkinan biner.
    pub is_binary: bool,
    /// Peringatan bahwa nama berkas cocok pola sensitif (`.env`, `*.key`, `*.pem`, dll.) (`IT12`).
    pub is_sensitive_hint: bool,
}

/// Respons `GET /sessions/{sid}/fs/file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsReadResponse {
    /// Jalur relatif terhadap akar worktree.
    pub path: String,
    /// ETag SHA-256 isi berkas untuk header `If-Match`.
    pub etag: String,
    /// Pengkodean isi (`"utf-8"` atau `"binary_omitted"` / `"too_large"`).
    pub encoding: String,
    /// Ukuran aktual dalam byte.
    pub size: u64,
    /// Waktu modifikasi terakhir (UTC epoch ms).
    pub mtime_ms: i64,
    /// Menandakan berkas biner (tidak dirender di editor teks).
    pub is_binary: bool,
    /// Menandakan berkas melampaui batas editor teks (`MAX_READ_TEXT_BYTES`).
    pub is_large: bool,
    /// Peringatan berkas sensitif (`IT12`).
    pub is_sensitive_hint: bool,
    /// Isi teks UTF-8 (`None` bila biner atau terlalu besar).
    pub content: Option<String>,
}

/// Payload `PUT /sessions/{sid}/fs/file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsWriteRequest {
    /// Isi teks UTF-8 (saling eksklusif dengan `content_base64`).
    #[serde(default)]
    pub content: Option<String>,
    /// Isi mentah base64 (opsional untuk berkas non-UTF-8 terbatas).
    #[serde(default)]
    pub content_base64: Option<String>,
}

/// Respons sukses `PUT /sessions/{sid}/fs/file`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsWriteResponse {
    /// Jalur relatif yang ditulis.
    pub path: String,
    /// ETag SHA-256 baru sesudah penulisan.
    pub etag: String,
    /// Ukuran byte yang ditulis.
    pub size: u64,
    /// Waktu modifikasi baru (UTC epoch ms).
    pub mtime_ms: i64,
    /// ID cadangan undo satu langkah (jika menimpa berkas lama).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_id: Option<String>,
}

/// Jenis operasi struktural `POST /sessions/{sid}/fs/ops`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FsOpKind {
    /// Buat berkas kosong baru (gagal jika sudah ada).
    Create,
    /// Buat direktori baru (rekursif).
    Mkdir,
    /// Ganti nama atau pindahkan berkas/direktori di dalam worktree.
    Rename,
    /// Hapus berkas atau direktori (selain akar dan `.git`).
    Delete,
}

/// Permintaan `POST /sessions/{sid}/fs/ops`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsOpRequest {
    /// Jenis operasi.
    pub op: FsOpKind,
    /// Jalur target relatif terhadap akar worktree.
    pub path: String,
    /// Jalur tujuan (wajib untuk `rename`).
    #[serde(default)]
    pub to: Option<String>,
}

/// Respons `POST /sessions/{sid}/fs/ops`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FsOpResponse {
    /// Jenis operasi yang dijalankan.
    pub op: FsOpKind,
    /// Jalur utama yang terpengaruh.
    pub path: String,
    /// Jalur tujuan (untuk `rename`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<String>,
    /// ID cadangan undo (jika menghapus berkas).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub undo_id: Option<String>,
}

/// Berkas mentah hasil `GET /sessions/{sid}/fs/raw`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FsRawFile {
    /// Jalur relatif kanonis.
    pub path: String,
    /// Nama berkas aman untuk `Content-Disposition`.
    pub file_name: String,
    /// Tipe MIME aman (tidak pernah `text/html` atau `image/svg+xml` secara inline).
    pub content_type: &'static str,
    /// Apakah aman ditampilkan inline (hanya gambar raster aman).
    pub safe_inline: bool,
    /// ETag SHA-256.
    pub etag: String,
    /// Byte mentah berkas.
    pub bytes: Vec<u8>,
}

/// Menghitung ETag SHA-256 hex untuk isi berkas.
pub fn compute_etag(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

/// Memeriksa apakah isi byte kemungkinan merupakan berkas biner.
pub fn is_likely_binary(bytes: &[u8]) -> bool {
    let sample_len = bytes.len().min(8192);
    let sample = &bytes[..sample_len];
    if sample.contains(&0) {
        return true;
    }
    std::str::from_utf8(bytes).is_err()
}

/// Mendeteksi pola nama berkas sensitif untuk peringatan UI (`IDE_SECURITY.md` IT12).
pub fn is_sensitive_filename(rel_path: &str) -> bool {
    let lower = rel_path.to_ascii_lowercase();
    let base = lower.rsplit('/').next().unwrap_or(&lower);
    base == ".env"
        || base.starts_with(".env.")
        || base.ends_with(".pem")
        || base.ends_with(".key")
        || base.ends_with(".p12")
        || base.ends_with(".pfx")
        || base == "id_rsa"
        || base == "id_ed25519"
        || base == "credentials.json"
        || base == ".npmrc"
        || base == ".netrc"
}

/// Validasi jalur relatif murni (kompatibilitas balik dengan API publik awal).
pub fn validate_relative_subpath(root: &Path, relative: &Path) -> Result<PathBuf, FsServiceError> {
    let rel_str = relative.to_string_lossy();
    let (_, clean_rel) = parse_clean_relative(&rel_str)?;
    Ok(root.join(clean_rel))
}

/// Mengurai dan menormalisasi string jalur relatif serta menolak `..`, jalur absolut, NUL, dan segmen `.git`.
fn parse_clean_relative(raw: &str) -> Result<(String, PathBuf), FsServiceError> {
    let trimmed = raw.trim();
    if trimmed.contains('\0') {
        return Err(FsServiceError::PathOutsideRoot(redact_secrets(trimmed)));
    }
    // Tolak backslash ala Windows traversal (`..\secret`) atau awalan drive/UNC secara konsisten di semua OS.
    let normalized = trimmed.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with('~') {
        return Err(FsServiceError::PathOutsideRoot(redact_secrets(trimmed)));
    }
    // Tolak awalan huruf drive Windows (mis. `C:/...`).
    if normalized.len() >= 2 && normalized.as_bytes()[1] == b':' {
        return Err(FsServiceError::PathOutsideRoot(redact_secrets(trimmed)));
    }

    let mut clean_parts: Vec<String> = Vec::new();
    let mut clean_path = PathBuf::new();

    for comp in Path::new(&normalized).components() {
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(FsServiceError::PathOutsideRoot(redact_secrets(trimmed)));
            }
            Component::CurDir => {}
            Component::Normal(seg) => {
                let s = seg.to_string_lossy();
                if s.eq_ignore_ascii_case(".git") {
                    return Err(FsServiceError::GitDirProtected(redact_secrets(trimmed)));
                }
                clean_parts.push(s.to_string());
                clean_path.push(seg);
            }
        }
    }

    Ok((clean_parts.join("/"), clean_path))
}

/// Memecahkan jalur di dalam akar worktree kanonis sekaligus memblokir symlink escape (`IT1`) dan `.git/` (`IT2`).
///
/// Jika `allow_missing_leaf == true`, komponen akhir (atau sub-jalur baru untuk `mkdir`/`write`)
/// boleh belum ada asalkan leluhur terdalam tetap berada di dalam `canonical_root` dan tidak melalui symlink keluar.
pub fn resolve_beneath_root(
    root: &Path,
    raw_rel: &str,
    allow_missing_leaf: bool,
) -> Result<(String, PathBuf), FsServiceError> {
    let canonical_root = fs::canonicalize(root)?;
    let (norm_rel, clean_rel) = parse_clean_relative(raw_rel)?;
    if norm_rel.is_empty() {
        return Ok((String::new(), canonical_root));
    }

    let mut cursor = canonical_root.clone();
    for comp in clean_rel.components() {
        let Component::Normal(seg) = comp else {
            return Err(FsServiceError::PathOutsideRoot(redact_secrets(raw_rel)));
        };
        if seg.to_string_lossy().eq_ignore_ascii_case(".git") {
            return Err(FsServiceError::GitDirProtected(redact_secrets(raw_rel)));
        }
        cursor.push(seg);

        match fs::symlink_metadata(&cursor) {
            Ok(meta) => {
                if meta.file_type().is_symlink() {
                    // Setiap symlink harus terpecah ke dalam `canonical_root` dan tidak boleh masuk `.git/`.
                    let resolved = fs::canonicalize(&cursor)
                        .map_err(|_| FsServiceError::PathOutsideRoot(redact_secrets(raw_rel)))?;
                    verify_inside_canonical_root(&canonical_root, &resolved, raw_rel)?;
                }
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                if !allow_missing_leaf {
                    return Err(FsServiceError::NotFound(norm_rel));
                }
                // Lanjutkan sisa komponen untuk verifikasi tidak ada `.git` (sudah divalidasi di atas),
                // tanpa mengizinkan symlink perantara keluar akar (leluhur sudah terverifikasi).
                continue;
            }
            Err(err) => return Err(FsServiceError::Io(err)),
        }
    }

    if cursor.exists() {
        let resolved = fs::canonicalize(&cursor)?;
        verify_inside_canonical_root(&canonical_root, &resolved, raw_rel)?;
        return Ok((norm_rel, resolved));
    }

    if !allow_missing_leaf {
        return Err(FsServiceError::NotFound(norm_rel));
    }

    // Pastikan leluhur terdekat yang ada di disk memang di dalam `canonical_root`.
    let mut ancestor = cursor.as_path();
    while let Some(parent) = ancestor.parent() {
        if parent.exists() {
            let resolved_parent = fs::canonicalize(parent)?;
            verify_inside_canonical_root(&canonical_root, &resolved_parent, raw_rel)?;
            break;
        }
        ancestor = parent;
    }

    Ok((norm_rel, cursor))
}

fn verify_inside_canonical_root(
    canonical_root: &Path,
    candidate: &Path,
    raw_rel: &str,
) -> Result<(), FsServiceError> {
    if !candidate.starts_with(canonical_root) {
        return Err(FsServiceError::PathOutsideRoot(redact_secrets(raw_rel)));
    }
    if let Ok(rel) = candidate.strip_prefix(canonical_root) {
        for comp in rel.components() {
            if let Component::Normal(seg) = comp {
                if seg.to_string_lossy().eq_ignore_ascii_case(".git") {
                    return Err(FsServiceError::GitDirProtected(redact_secrets(raw_rel)));
                }
            }
        }
    }
    Ok(())
}

fn metadata_mtime_ms(meta: &Metadata) -> i64 {
    meta.modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| i64::try_from(d.as_millis()).ok())
        .unwrap_or(0)
}

/// Membaca daftar pohon direktori worktree secara aman dengan batas kedalaman & paginasi (`IT-201`).
pub fn list_tree(
    root: &Path,
    raw_rel: &str,
    depth: usize,
    limit: usize,
) -> Result<FsTreeResponse, FsServiceError> {
    let canonical_root = fs::canonicalize(root)?;
    let (norm_rel, target) = resolve_beneath_root(&canonical_root, raw_rel, false)?;
    if !target.is_dir() {
        return Err(FsServiceError::InvalidRequest(format!(
            "'{}' is not a directory",
            norm_rel
        )));
    }
    let max_depth = depth.clamp(1, 4);
    let max_items = limit.clamp(1, MAX_TREE_ENTRIES);
    let mut counter = 0usize;
    let mut truncated = false;
    let entries = collect_tree_dir(
        &canonical_root,
        &target,
        &norm_rel,
        max_depth,
        max_items,
        &mut counter,
        &mut truncated,
    )?;

    Ok(FsTreeResponse {
        path: norm_rel,
        entries,
        truncated,
    })
}

fn collect_tree_dir(
    canonical_root: &Path,
    dir: &Path,
    rel_prefix: &str,
    depth_remaining: usize,
    max_items: usize,
    counter: &mut usize,
    truncated: &mut bool,
) -> Result<Vec<FsTreeEntry>, FsServiceError> {
    let mut raw_entries: Vec<(String, PathBuf)> = Vec::new();
    for item in fs::read_dir(dir)? {
        let entry = item?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.eq_ignore_ascii_case(".git") {
            continue;
        }
        raw_entries.push((name, entry.path()));
    }
    raw_entries.sort_by(|a, b| {
        let a_dir = a.1.is_dir();
        let b_dir = b.1.is_dir();
        b_dir.cmp(&a_dir).then_with(|| a.0.cmp(&b.0))
    });

    let mut out = Vec::new();
    for (name, path) in raw_entries {
        if *counter >= max_items {
            *truncated = true;
            break;
        }
        let Ok(sym_meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        let is_symlink = sym_meta.file_type().is_symlink();
        if is_symlink {
            // Sembunyikan symlink yang mengarah keluar akar worktree agar tidak membocorkan metadata host.
            let Ok(resolved) = fs::canonicalize(&path) else {
                continue;
            };
            if verify_inside_canonical_root(canonical_root, &resolved, &name).is_err() {
                continue;
            }
        }
        let Ok(meta) = fs::metadata(&path) else {
            continue;
        };
        let kind = if is_symlink {
            FsEntryKind::Symlink
        } else if meta.is_dir() {
            FsEntryKind::Dir
        } else {
            FsEntryKind::File
        };
        let child_rel = if rel_prefix.is_empty() {
            name.clone()
        } else {
            format!("{rel_prefix}/{name}")
        };
        *counter += 1;

        let children = if kind == FsEntryKind::Dir && depth_remaining > 1 {
            collect_tree_dir(
                canonical_root,
                &path,
                &child_rel,
                depth_remaining - 1,
                max_items,
                counter,
                truncated,
            )?
        } else {
            Vec::new()
        };

        out.push(FsTreeEntry {
            name,
            path: child_rel,
            kind,
            size: if kind == FsEntryKind::File {
                meta.len()
            } else {
                0
            },
            mtime_ms: metadata_mtime_ms(&meta),
            is_symlink,
            git_status: None,
            agent_touched: false,
            children,
        });
    }
    Ok(out)
}

/// Mengembalikan metadata ringan satu berkas atau direktori (`GET /sessions/{sid}/fs/stat`).
pub fn stat_path(root: &Path, raw_rel: &str) -> Result<FsStatResponse, FsServiceError> {
    let (norm_rel, target) = resolve_beneath_root(root, raw_rel, false)?;
    let sym_meta = fs::symlink_metadata(&target)?;
    let meta = fs::metadata(&target)?;
    let kind = if sym_meta.file_type().is_symlink() {
        FsEntryKind::Symlink
    } else if meta.is_dir() {
        FsEntryKind::Dir
    } else {
        FsEntryKind::File
    };
    let (etag, is_binary) = if kind == FsEntryKind::File && meta.len() <= MAX_READ_TEXT_BYTES as u64
    {
        let bytes = fs::read(&target)?;
        (Some(compute_etag(&bytes)), is_likely_binary(&bytes))
    } else {
        (None, false)
    };
    Ok(FsStatResponse {
        is_sensitive_hint: is_sensitive_filename(&norm_rel),
        path: norm_rel,
        kind,
        size: if meta.is_file() { meta.len() } else { 0 },
        mtime_ms: metadata_mtime_ms(&meta),
        etag,
        is_binary,
    })
}

/// Membaca berkas teks di dalam worktree lengkap dengan ETag SHA-256, deteksi biner & berkas besar (`IT-201`, `IT-208`).
pub fn read_file(root: &Path, raw_rel: &str) -> Result<FsReadResponse, FsServiceError> {
    let (norm_rel, target) = resolve_beneath_root(root, raw_rel, false)?;
    if norm_rel.is_empty() || !target.is_file() {
        return Err(FsServiceError::InvalidRequest(format!(
            "'{}' is not a readable file",
            norm_rel
        )));
    }
    let meta = fs::metadata(&target)?;
    let size = meta.len();
    let mtime_ms = metadata_mtime_ms(&meta);
    let is_sensitive_hint = is_sensitive_filename(&norm_rel);

    if size > MAX_READ_TEXT_BYTES as u64 {
        // Hitung ETag streaming tanpa menampung isi ke editor UI.
        let etag = hash_file_streaming(&target)?;
        return Ok(FsReadResponse {
            path: norm_rel,
            etag,
            encoding: "too_large".to_string(),
            size,
            mtime_ms,
            is_binary: false,
            is_large: true,
            is_sensitive_hint,
            content: None,
        });
    }

    let bytes = fs::read(&target)?;
    let etag = compute_etag(&bytes);
    if is_likely_binary(&bytes) {
        return Ok(FsReadResponse {
            path: norm_rel,
            etag,
            encoding: "binary_omitted".to_string(),
            size,
            mtime_ms,
            is_binary: true,
            is_large: false,
            is_sensitive_hint,
            content: None,
        });
    }

    let text = String::from_utf8(bytes).map_err(|_| {
        FsServiceError::InvalidRequest("file content is not valid UTF-8".to_string())
    })?;

    Ok(FsReadResponse {
        path: norm_rel,
        etag,
        encoding: "utf-8".to_string(),
        size,
        mtime_ms,
        is_binary: false,
        is_large: false,
        is_sensitive_hint,
        content: Some(text),
    })
}

/// Membaca byte mentah berkas untuk unduhan atau pratinjau gambar raster aman (`GET /sessions/{sid}/fs/raw`).
pub fn read_raw(root: &Path, raw_rel: &str) -> Result<FsRawFile, FsServiceError> {
    let (norm_rel, target) = resolve_beneath_root(root, raw_rel, false)?;
    if norm_rel.is_empty() || !target.is_file() {
        return Err(FsServiceError::InvalidRequest(format!(
            "'{}' is not a readable file",
            norm_rel
        )));
    }
    let meta = fs::metadata(&target)?;
    let size = usize::try_from(meta.len()).unwrap_or(usize::MAX);
    if size > MAX_RAW_BYTES {
        return Err(FsServiceError::PayloadTooLarge {
            limit: MAX_RAW_BYTES,
            actual: size,
        });
    }
    let bytes = fs::read(&target)?;
    let etag = compute_etag(&bytes);
    let file_name = target
        .file_name()
        .map(|n| n.to_string_lossy().replace(['"', '\r', '\n', '\\'], "_"))
        .unwrap_or_else(|| "file".to_string());
    let ext = target
        .extension()
        .and_then(|e| e.to_str())
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();

    // Hanya gambar raster aman yang boleh inline; HTML/JS/SVG selalu application/octet-stream (anti-XSS).
    let (content_type, safe_inline) = match ext.as_str() {
        "png" => ("image/png", true),
        "jpg" | "jpeg" => ("image/jpeg", true),
        "webp" => ("image/webp", true),
        "gif" => ("image/gif", true),
        "ico" => ("image/x-icon", true),
        _ => ("application/octet-stream", false),
    };

    Ok(FsRawFile {
        path: norm_rel,
        file_name,
        content_type,
        safe_inline,
        etag,
        bytes,
    })
}

/// Menulis berkas secara atomik dengan pemeriksaan `If-Match` ETag dan cadangan undo `0600` (`IT-201`, `IT-202`).
pub fn write_file(
    root: &Path,
    undo_dir: &Path,
    raw_rel: &str,
    req: &FsWriteRequest,
    if_match: Option<&str>,
) -> Result<FsWriteResponse, FsServiceError> {
    let payload: Vec<u8> = match (&req.content, &req.content_base64) {
        (Some(text), None) => text.as_bytes().to_vec(),
        (None, Some(b64)) => BASE64_STD
            .decode(b64.trim())
            .map_err(|err| FsServiceError::InvalidRequest(format!("invalid base64: {err}")))?,
        (Some(_), Some(_)) => {
            return Err(FsServiceError::InvalidRequest(
                "provide either 'content' or 'content_base64', not both".to_string(),
            ));
        }
        (None, None) => {
            return Err(FsServiceError::InvalidRequest(
                "missing 'content' or 'content_base64'".to_string(),
            ));
        }
    };

    if payload.len() > MAX_WRITE_BYTES {
        return Err(FsServiceError::PayloadTooLarge {
            limit: MAX_WRITE_BYTES,
            actual: payload.len(),
        });
    }

    let (norm_rel, target) = resolve_beneath_root(root, raw_rel, true)?;
    if norm_rel.is_empty() {
        return Err(FsServiceError::InvalidRequest(
            "cannot overwrite worktree root directory".to_string(),
        ));
    }

    // Tolak menulis melalui symlink langsung agar tidak bisa ditipu mengganti target tak terduga.
    if let Ok(sym_meta) = fs::symlink_metadata(&target) {
        if sym_meta.file_type().is_symlink() {
            return Err(FsServiceError::PathOutsideRoot(redact_secrets(raw_rel)));
        }
        if sym_meta.is_dir() {
            return Err(FsServiceError::InvalidRequest(format!(
                "'{}' is a directory",
                norm_rel
            )));
        }
    }

    let mut undo_id = None;
    if target.exists() {
        let existing = fs::read(&target)?;
        let current_etag = compute_etag(&existing);
        if let Some(expected_raw) = if_match {
            let expected = expected_raw.trim().trim_matches('"');
            if !expected.is_empty() && expected != "*" && expected != current_etag {
                let current_content =
                    if existing.len() <= MAX_READ_TEXT_BYTES && !is_likely_binary(&existing) {
                        String::from_utf8(existing).ok()
                    } else {
                        None
                    };
                return Err(FsServiceError::EtagMismatch {
                    current_etag,
                    current_content,
                });
            }
        }
        undo_id = save_undo_backup(undo_dir, &existing)?;
    }

    if let Some(parent) = target.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp_name = format!(".latch-tmp-{}", new_id());
    let tmp_path = target.with_file_name(tmp_name);
    fs::write(&tmp_path, &payload)?;
    fs::rename(&tmp_path, &target)?;

    let meta = fs::metadata(&target)?;
    Ok(FsWriteResponse {
        path: norm_rel,
        etag: compute_etag(&payload),
        size: payload.len() as u64,
        mtime_ms: metadata_mtime_ms(&meta),
        undo_id,
    })
}

/// Menjalankan operasi struktural `create`, `mkdir`, `rename`, atau `delete` secara aman (`IT-201`, `IT-202`).
pub fn apply_op(
    root: &Path,
    undo_dir: &Path,
    req: &FsOpRequest,
) -> Result<FsOpResponse, FsServiceError> {
    match req.op {
        FsOpKind::Create => {
            let (norm_rel, target) = resolve_beneath_root(root, &req.path, true)?;
            if norm_rel.is_empty() {
                return Err(FsServiceError::InvalidRequest(
                    "file path must not be empty".to_string(),
                ));
            }
            if target.exists() {
                return Err(FsServiceError::AlreadyExists(norm_rel));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&target, b"")?;
            Ok(FsOpResponse {
                op: FsOpKind::Create,
                path: norm_rel,
                to: None,
                undo_id: None,
            })
        }
        FsOpKind::Mkdir => {
            let (norm_rel, target) = resolve_beneath_root(root, &req.path, true)?;
            if norm_rel.is_empty() {
                return Err(FsServiceError::InvalidRequest(
                    "directory path must not be empty".to_string(),
                ));
            }
            if target.exists() {
                return Err(FsServiceError::AlreadyExists(norm_rel));
            }
            fs::create_dir_all(&target)?;
            Ok(FsOpResponse {
                op: FsOpKind::Mkdir,
                path: norm_rel,
                to: None,
                undo_id: None,
            })
        }
        FsOpKind::Rename => {
            let Some(ref raw_to) = req.to else {
                return Err(FsServiceError::InvalidRequest(
                    "'to' path is required for rename".to_string(),
                ));
            };
            let (from_rel, from_target) = resolve_beneath_root(root, &req.path, false)?;
            let (to_rel, to_target) = resolve_beneath_root(root, raw_to, true)?;
            if from_rel.is_empty() || to_rel.is_empty() {
                return Err(FsServiceError::InvalidRequest(
                    "cannot rename worktree root".to_string(),
                ));
            }
            if to_target.exists() {
                return Err(FsServiceError::AlreadyExists(to_rel));
            }
            if let Some(parent) = to_target.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::rename(&from_target, &to_target)?;
            Ok(FsOpResponse {
                op: FsOpKind::Rename,
                path: from_rel,
                to: Some(to_rel),
                undo_id: None,
            })
        }
        FsOpKind::Delete => {
            let (norm_rel, target) = resolve_beneath_root(root, &req.path, false)?;
            if norm_rel.is_empty() {
                return Err(FsServiceError::InvalidRequest(
                    "refusing to delete worktree root".to_string(),
                ));
            }
            let sym_meta = fs::symlink_metadata(&target)?;
            let mut undo_id = None;
            if sym_meta.file_type().is_symlink() || sym_meta.is_file() {
                if sym_meta.is_file() && sym_meta.len() <= MAX_WRITE_BYTES as u64 {
                    if let Ok(existing) = fs::read(&target) {
                        undo_id = save_undo_backup(undo_dir, &existing)?;
                    }
                }
                fs::remove_file(&target)?;
            } else if sym_meta.is_dir() {
                fs::remove_dir_all(&target)?;
            }
            Ok(FsOpResponse {
                op: FsOpKind::Delete,
                path: norm_rel,
                to: None,
                undo_id,
            })
        }
    }
}

fn hash_file_streaming(path: &Path) -> Result<String, FsServiceError> {
    use std::io::Read;
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 32 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for b in digest {
        let _ = write!(&mut hex, "{b:02x}");
    }
    Ok(hex)
}

fn save_undo_backup(undo_dir: &Path, existing: &[u8]) -> Result<Option<String>, FsServiceError> {
    if existing.len() > MAX_WRITE_BYTES {
        return Ok(None);
    }
    fs::create_dir_all(undo_dir)?;
    let id = format!("{}-{}", now_ms(), new_id());
    let backup_path = undo_dir.join(format!("{id}.bak"));
    fs::write(&backup_path, existing)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&backup_path, fs::Permissions::from_mode(0o600));
    }
    Ok(Some(id))
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use proptest::prelude::*;
    use tempfile::TempDir;

    use super::{
        apply_op, list_tree, read_file, resolve_beneath_root, stat_path, validate_relative_subpath,
        write_file, FsOpKind, FsOpRequest, FsServiceError, FsWriteRequest,
    };

    #[test]
    fn rejects_parent_traversal_and_git() {
        let root = Path::new("/tmp/wt");
        assert!(validate_relative_subpath(root, Path::new("../secret")).is_err());
        assert!(validate_relative_subpath(root, Path::new(".git/config")).is_err());
        assert!(validate_relative_subpath(root, Path::new(".GIT/hooks/pre-commit")).is_err());
        assert!(validate_relative_subpath(root, Path::new("src/main.rs")).is_ok());
    }

    #[test]
    fn full_crud_etag_conflict_undo_and_git_protection() {
        let dir = TempDir::new().unwrap();
        let wt = dir.path().join("wt");
        let undo = dir.path().join("undo");
        fs::create_dir_all(wt.join("src")).unwrap();
        fs::create_dir_all(wt.join(".git/hooks")).unwrap();
        fs::write(wt.join(".git/config"), "[core]\n").unwrap();
        fs::write(wt.join("src/lib.rs"), "pub fn hi() {}\n").unwrap();
        fs::write(wt.join(".env.local"), "SECRET=1\n").unwrap();

        // Tree menyembunyikan .git
        let tree = list_tree(&wt, "", 2, 100).unwrap();
        assert!(!tree.entries.iter().any(|e| e.name == ".git"));
        assert!(tree.entries.iter().any(|e| e.name == "src"));

        // Baca + ETag + sensitif hint
        let env_read = read_file(&wt, ".env.local").unwrap();
        assert!(env_read.is_sensitive_hint);
        let lib_read = read_file(&wt, "src/lib.rs").unwrap();
        assert_eq!(lib_read.content.as_deref(), Some("pub fn hi() {}\n"));

        // PUT ke .git/* ditolak dengan GitDirProtected
        let git_err = write_file(
            &wt,
            &undo,
            ".git/hooks/pre-commit",
            &FsWriteRequest {
                content: Some("#!/bin/sh\n".to_string()),
                content_base64: None,
            },
            None,
        )
        .unwrap_err();
        assert!(matches!(git_err, FsServiceError::GitDirProtected(_)));

        // PUT dengan ETag salah mengembalikan 409 EtagMismatch + isi terbaru
        let conflict = write_file(
            &wt,
            &undo,
            "src/lib.rs",
            &FsWriteRequest {
                content: Some("pub fn conflict() {}\n".to_string()),
                content_base64: None,
            },
            Some("deadbeef"),
        )
        .unwrap_err();
        assert!(matches!(
            conflict,
            FsServiceError::EtagMismatch { ref current_etag, ref current_content }
                if current_etag == &lib_read.etag && current_content.as_deref() == Some("pub fn hi() {}\n")
        ));

        // PUT dengan ETag cocok sukses dan membuat cadangan undo
        let saved = write_file(
            &wt,
            &undo,
            "src/lib.rs",
            &FsWriteRequest {
                content: Some("pub fn updated() {}\n".to_string()),
                content_base64: None,
            },
            Some(&lib_read.etag),
        )
        .unwrap();
        assert_ne!(saved.etag, lib_read.etag);
        assert!(saved.undo_id.is_some());

        // Ops: create -> rename -> stat -> delete
        apply_op(
            &wt,
            &undo,
            &FsOpRequest {
                op: FsOpKind::Create,
                path: "notes.txt".to_string(),
                to: None,
            },
        )
        .unwrap();
        apply_op(
            &wt,
            &undo,
            &FsOpRequest {
                op: FsOpKind::Rename,
                path: "notes.txt".to_string(),
                to: Some("docs/notes.txt".to_string()),
            },
        )
        .unwrap();
        let st = stat_path(&wt, "docs/notes.txt").unwrap();
        assert_eq!(st.path, "docs/notes.txt");
        apply_op(
            &wt,
            &undo,
            &FsOpRequest {
                op: FsOpKind::Delete,
                path: "docs/notes.txt".to_string(),
                to: None,
            },
        )
        .unwrap();
        assert!(!wt.join("docs/notes.txt").exists());
    }

    #[cfg(unix)]
    #[test]
    fn blocks_symlink_escape_for_read_write_raw_and_tree() {
        use std::os::unix::fs::symlink;

        use super::read_raw;

        let dir = TempDir::new().unwrap();
        let outside = dir.path().join("outside_secret.txt");
        let wt = dir.path().join("wt");
        let undo = dir.path().join("undo");
        fs::create_dir_all(&wt).unwrap();
        fs::write(&outside, "TOP_SECRET_SSH_KEY").unwrap();
        symlink(&outside, wt.join("leak.txt")).unwrap();

        assert!(matches!(
            read_file(&wt, "leak.txt").unwrap_err(),
            FsServiceError::PathOutsideRoot(_)
        ));
        assert!(matches!(
            read_raw(&wt, "leak.txt").unwrap_err(),
            FsServiceError::PathOutsideRoot(_)
        ));
        assert!(matches!(
            write_file(
                &wt,
                &undo,
                "leak.txt",
                &FsWriteRequest {
                    content: Some("pwn".to_string()),
                    content_base64: None,
                },
                None,
            )
            .unwrap_err(),
            FsServiceError::PathOutsideRoot(_)
        ));
        // Symlink keluar akar tidak muncul pada listing tree
        let tree = list_tree(&wt, "", 1, 100).unwrap();
        assert!(!tree.entries.iter().any(|e| e.name == "leak.txt"));
    }

    proptest! {
        #[test]
        fn fuzz_paths_never_escape_worktree_root(raw in ".*") {
            let dir = TempDir::new().unwrap();
            let wt = dir.path().join("wt");
            fs::create_dir_all(&wt).unwrap();
            let canonical_root = fs::canonicalize(&wt).unwrap();
            if let Ok((_, resolved)) = resolve_beneath_root(&wt, &raw, true) {
                prop_assert!(resolved.starts_with(&canonical_root));
                prop_assert!(
                    !resolved
                        .strip_prefix(&canonical_root)
                        .unwrap()
                        .components()
                        .any(|c| c.as_os_str().to_string_lossy().eq_ignore_ascii_case(".git"))
                );
            }
        }
    }
}
