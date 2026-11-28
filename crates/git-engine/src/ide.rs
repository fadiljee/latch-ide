//! Operasi Git lokal worktree, review per-hunk (terima/tolak/undo), penanda berisiko, dan sinkronisasi base
//! untuk Agent IDE (`IT-210`, `IT-302`, `IT-305`, `IT-408`).
//!
//! Seluruh perintah commit/merge mematikan hook (`core.hooksPath=/dev/null`), dan tidak pernah menyediakan
//! endpoint push/pull/remote (`IDE_API.md` §5).

use std::fmt::Write as _;
use std::path::Path;
use std::process::Stdio;

use latch_core::id::new_id;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::{run_git, GitError};

/// Satu berkas dalam status Git lokal worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitStatusEntry {
    /// Jalur relatif terhadap akar worktree.
    pub path: String,
    /// Kode status dua karakter porcelain (`XY`).
    pub code: String,
}

/// Ringkasan `GET /sessions/{sid}/git/status` (`IT-302`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitWorktreeStatus {
    /// Nama branch worktree saat ini.
    pub branch: String,
    /// Branch dasar proyek.
    pub base_branch: String,
    /// Jumlah commit di depan `base_branch`.
    pub ahead: u32,
    /// Jumlah commit di belakang `base_branch`.
    pub behind: u32,
    /// Daftar berkas yang sudah di-stage.
    pub staged: Vec<GitStatusEntry>,
    /// Daftar berkas berubah belum di-stage.
    pub unstaged: Vec<GitStatusEntry>,
    /// Daftar berkas untracked.
    pub untracked: Vec<String>,
    /// Daftar berkas berkonflik merge.
    pub conflicted: Vec<String>,
}

/// Hasil `POST /sessions/{sid}/git/commit`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitCommitResult {
    /// OID commit baru.
    pub commit_oid: String,
    /// Pesan commit.
    pub message: String,
}

/// Satu entri riwayat commit (`GET /sessions/{sid}/git/log`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitLogEntry {
    /// Hash penuh commit.
    pub oid: String,
    /// Hash pendek 8 karakter.
    pub short_oid: String,
    /// Nama penulis.
    pub author: String,
    /// Waktu commit (ISO 8601).
    pub date: String,
    /// Baris subjek commit.
    pub subject: String,
}

/// Satu baris anotasi blame (`GET /sessions/{sid}/git/blame`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitBlameLine {
    /// Nomor baris berbasis 1.
    pub line: usize,
    /// Hash pendek commit.
    pub short_oid: String,
    /// Penulis baris tersebut.
    pub author: String,
    /// Ringkasan commit.
    pub summary: String,
}

/// Hasil `POST /sessions/{sid}/git/sync-base` (`IT-408`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SyncBaseResult {
    /// `"clean"` atau `"conflicts"`.
    pub status: String,
    /// Daftar berkas konflik bila `status == "conflicts"`.
    pub conflicts: Vec<String>,
}

/// Satu hunk diff dengan ID stabil untuk aksi accept/reject/undo (`IT-210`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewHunk {
    /// ID stabil (SHA-256 16-hex dari path + konteks hunk).
    pub hunk_id: String,
    /// Jalur berkas relatif.
    pub path: String,
    /// Header baris `@@ -a,b +c,d @@`.
    pub header: String,
    /// Baris awal lama.
    pub old_start: u32,
    /// Baris awal baru.
    pub new_start: u32,
    /// Jumlah baris ditambahkan pada hunk ini.
    pub additions: u32,
    /// Jumlah baris dihapus pada hunk ini.
    pub deletions: u32,
    /// Patch lengkap siap diterapkan ke `git apply`.
    pub patch: String,
}

/// Satu berkas dalam respons `GET /sessions/{sid}/review` (`IT-210`, `IT-305`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewFile {
    /// Jalur berkas relatif.
    pub path: String,
    /// Jumlah baris ditambah.
    pub additions: u32,
    /// Jumlah baris dihapus.
    pub deletions: u32,
    /// Penanda risiko otomatis (mis. `security`, `config`, `migration`, `dependency`, `large_diff`).
    pub risk_markers: Vec<String>,
    /// Daftar hunk pada berkas ini.
    pub hunks: Vec<ReviewHunk>,
}

/// Ringkasan review terstruktur untuk satu sesi (`GET /sessions/{sid}/review`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionReview {
    /// Branch dasar perbandingan.
    pub base_branch: String,
    /// Total berkas berubah.
    pub total_files: usize,
    /// Total baris ditambahkan.
    pub total_additions: u32,
    /// Total baris dihapus.
    pub total_deletions: u32,
    /// Jumlah berkas yang memiliki penanda berisiko.
    pub risky_files: usize,
    /// Rincian berkas & hunk.
    pub files: Vec<ReviewFile>,
}

fn validate_git_rel_path(path: &str) -> Result<&str, GitError> {
    let trimmed = path.trim();
    if trimmed.is_empty()
        || trimmed.starts_with('-')
        || trimmed.starts_with('/')
        || trimmed.contains("..")
        || trimmed.contains('\0')
        || trimmed.eq_ignore_ascii_case(".git")
        || trimmed.starts_with(".git/")
    {
        return Err(GitError::CommandFailed {
            cmd: "path-validation".to_string(),
            stderr: format!("invalid worktree relative path: '{trimmed}'"),
        });
    }
    Ok(trimmed)
}

/// Memeriksa status Git worktree (staged, unstaged, untracked, conflicted, ahead/behind base) (`IT-302`).
pub async fn worktree_status(
    worktree_path: &Path,
    base_branch: &str,
) -> Result<GitWorktreeStatus, GitError> {
    let branch = run_git(worktree_path, &["symbolic-ref", "--short", "HEAD"])
        .await
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|_| "HEAD".to_string());

    let (behind, ahead) = match run_git(
        worktree_path,
        &[
            "rev-list",
            "--left-right",
            "--count",
            &format!("{base_branch}...HEAD"),
        ],
    )
    .await
    {
        Ok(raw) => {
            let mut parts = raw.split_whitespace();
            let b = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            let a = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
            (b, a)
        }
        Err(_) => (0, 0),
    };

    let porcelain = run_git(worktree_path, &["status", "--porcelain=v1"]).await?;
    let mut staged = Vec::new();
    let mut unstaged = Vec::new();
    let mut untracked = Vec::new();
    let mut conflicted = Vec::new();

    for line in porcelain.lines() {
        if line.len() < 4 {
            continue;
        }
        let xy = &line[..2];
        let raw_path = line[3..].trim();
        let path = raw_path
            .rsplit(" -> ")
            .next()
            .unwrap_or(raw_path)
            .to_string();
        let mut chars = xy.chars();
        let x = chars.next().unwrap_or(' ');
        let y = chars.next().unwrap_or(' ');

        if matches!(xy, "UU" | "AA" | "DD" | "AU" | "UA" | "DU" | "UD") {
            conflicted.push(path);
        } else if xy == "??" {
            untracked.push(path);
        } else {
            if x != ' ' {
                staged.push(GitStatusEntry {
                    path: path.clone(),
                    code: xy.to_string(),
                });
            }
            if y != ' ' {
                unstaged.push(GitStatusEntry {
                    path,
                    code: xy.to_string(),
                });
            }
        }
    }

    Ok(GitWorktreeStatus {
        branch,
        base_branch: base_branch.to_string(),
        ahead,
        behind,
        staged,
        unstaged,
        untracked,
        conflicted,
    })
}

/// Men-stage daftar berkas relatif di worktree (`POST /sessions/{sid}/git/stage`).
pub async fn stage_paths(worktree_path: &Path, paths: &[String]) -> Result<(), GitError> {
    if paths.is_empty() {
        run_git(worktree_path, &["add", "-A"]).await?;
        return Ok(());
    }
    let mut args: Vec<&str> = vec!["add", "--"];
    for p in paths {
        args.push(validate_git_rel_path(p)?);
    }
    run_git(worktree_path, &args).await?;
    Ok(())
}

/// Membatalkan stage daftar berkas relatif di worktree (`POST /sessions/{sid}/git/unstage`).
pub async fn unstage_paths(worktree_path: &Path, paths: &[String]) -> Result<(), GitError> {
    if paths.is_empty() {
        run_git(worktree_path, &["reset", "HEAD"]).await?;
        return Ok(());
    }
    let mut args: Vec<&str> = vec!["reset", "HEAD", "--"];
    for p in paths {
        args.push(validate_git_rel_path(p)?);
    }
    run_git(worktree_path, &args).await?;
    Ok(())
}

/// Membuat commit lokal di worktree dengan hook dimatikan (`POST /sessions/{sid}/git/commit`).
pub async fn commit_local(
    worktree_path: &Path,
    message: &str,
    paths: &[String],
) -> Result<GitCommitResult, GitError> {
    let trimmed = message.trim();
    if trimmed.is_empty() {
        return Err(GitError::CommandFailed {
            cmd: "commit".to_string(),
            stderr: "commit message must not be empty".to_string(),
        });
    }
    if !paths.is_empty() {
        stage_paths(worktree_path, paths).await?;
    }
    run_git(
        worktree_path,
        &["-c", "core.hooksPath=/dev/null", "commit", "-m", trimmed],
    )
    .await?;
    let oid = run_git(worktree_path, &["rev-parse", "HEAD"])
        .await?
        .trim()
        .to_string();
    Ok(GitCommitResult {
        commit_oid: oid,
        message: trimmed.to_string(),
    })
}

/// Membaca riwayat commit ringkas (`GET /sessions/{sid}/git/log`).
pub async fn worktree_log(
    worktree_path: &Path,
    path: Option<&str>,
    limit: usize,
) -> Result<Vec<GitLogEntry>, GitError> {
    let cap = limit.clamp(1, 100);
    let max_arg = format!("-n{cap}");
    let mut args = vec!["log", &max_arg, "--format=%H%x1f%h%x1f%an%x1f%aI%x1f%s"];
    if let Some(p) = path.filter(|s| !s.trim().is_empty()) {
        args.push("--");
        args.push(validate_git_rel_path(p)?);
    }
    let raw = run_git(worktree_path, &args).await?;
    let mut entries = Vec::new();
    for line in raw.lines() {
        let cols: Vec<&str> = line.split('\x1f').collect();
        if cols.len() >= 5 {
            entries.push(GitLogEntry {
                oid: cols[0].to_string(),
                short_oid: cols[1].to_string(),
                author: cols[2].to_string(),
                date: cols[3].to_string(),
                subject: cols[4].to_string(),
            });
        }
    }
    Ok(entries)
}

/// Membaca anotasi blame ringan untuk satu berkas (`GET /sessions/{sid}/git/blame`).
pub async fn worktree_blame(
    worktree_path: &Path,
    path: &str,
) -> Result<Vec<GitBlameLine>, GitError> {
    let rel = validate_git_rel_path(path)?;
    let raw = run_git(worktree_path, &["blame", "--line-porcelain", "--", rel]).await?;
    let mut out = Vec::new();
    let mut cur_oid = String::new();
    let mut cur_author = String::new();
    let mut cur_summary = String::new();
    let mut cur_line = 0usize;

    for line in raw.lines() {
        if line.starts_with('\t') {
            if cur_line > 0 {
                out.push(GitBlameLine {
                    line: cur_line,
                    short_oid: cur_oid.chars().take(8).collect(),
                    author: cur_author.clone(),
                    summary: cur_summary.clone(),
                });
            }
        } else if let Some(a) = line.strip_prefix("author ") {
            cur_author = a.to_string();
        } else if let Some(s) = line.strip_prefix("summary ") {
            cur_summary = s.to_string();
        } else {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3
                && parts[0].len() == 40
                && parts[0].bytes().all(|b| b.is_ascii_hexdigit())
            {
                cur_oid = parts[0].to_string();
                cur_line = parts[2].parse().unwrap_or(0);
            }
        }
    }
    Ok(out)
}

/// Menyinkronkan branch worktree dengan `base_branch` terbaru dan melaporkan status bersih/konflik (`IT-408`).
pub async fn sync_base_branch(
    worktree_path: &Path,
    base_branch: &str,
) -> Result<SyncBaseResult, GitError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .args([
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=Latch",
            "-c",
            "user.email=latch@localhost",
            "merge",
            "--no-edit",
            base_branch,
        ])
        .output()
        .await?;

    if out.status.success() {
        return Ok(SyncBaseResult {
            status: "clean".to_string(),
            conflicts: Vec::new(),
        });
    }

    let conflicted = run_git(worktree_path, &["diff", "--name-only", "--diff-filter=U"])
        .await
        .unwrap_or_default();
    let conflicts: Vec<String> = conflicted
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if !conflicts.is_empty() {
        return Ok(SyncBaseResult {
            status: "conflicts".to_string(),
            conflicts,
        });
    }

    Err(GitError::CommandFailed {
        cmd: format!("merge {base_branch}"),
        stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
    })
}

/// Mendeteksi penanda risiko perubahan pada sebuah berkas (`IT-305`).
pub fn detect_risk_markers(path: &str, additions: u32, deletions: u32) -> Vec<String> {
    let lower = path.to_ascii_lowercase();
    let mut markers = Vec::new();
    if lower.contains("auth")
        || lower.contains("security")
        || lower.contains("sandbox")
        || lower.contains("permission")
        || lower.contains("token")
    {
        markers.push("security".to_string());
    }
    if lower.contains("migration") || lower.ends_with(".sql") {
        markers.push("migration".to_string());
    }
    if lower.ends_with("cargo.toml")
        || lower.ends_with("cargo.lock")
        || lower.ends_with("package.json")
        || lower.ends_with("package-lock.json")
    {
        markers.push("dependency".to_string());
    }
    if lower.starts_with(".github/workflows/")
        || lower.contains("dockerfile")
        || lower.ends_with(".env")
        || lower.contains(".env.")
    {
        markers.push("infra_config".to_string());
    }
    if additions.saturating_add(deletions) >= 300 {
        markers.push("large_diff".to_string());
    }
    markers
}

/// Menghasilkan review terstruktur per file dan per hunk dengan `hunk_id` stabil (`IT-210`, `IT-305`).
pub async fn build_session_review(
    worktree_path: &Path,
    base_branch: &str,
) -> Result<SessionReview, GitError> {
    let raw_diff = run_git(worktree_path, &["diff", "--no-color", "-U3", base_branch]).await?;
    Ok(parse_unified_review(&raw_diff, base_branch))
}

/// Mem-parse output unified diff menjadi `SessionReview` berstruktur hunk.
pub fn parse_unified_review(raw_diff: &str, base_branch: &str) -> SessionReview {
    let mut files: Vec<ReviewFile> = Vec::new();
    let mut cur_path: Option<String> = None;
    let mut cur_file_header: Vec<String> = Vec::new();
    let mut cur_hunks: Vec<ReviewHunk> = Vec::new();
    let mut cur_hunk_header: Option<String> = None;
    let mut cur_hunk_lines: Vec<String> = Vec::new();

    let flush_hunk = |path_opt: &Option<String>,
                      file_hdr: &[String],
                      hunk_hdr_opt: &mut Option<String>,
                      hunk_lines: &mut Vec<String>,
                      hunks_out: &mut Vec<ReviewHunk>| {
        if let (Some(path), Some(hdr)) = (path_opt, hunk_hdr_opt.take()) {
            let mut adds = 0u32;
            let mut dels = 0u32;
            for l in hunk_lines.iter() {
                if l.starts_with('+') && !l.starts_with("+++") {
                    adds += 1;
                } else if l.starts_with('-') && !l.starts_with("---") {
                    dels += 1;
                }
            }
            let (old_start, new_start) = parse_hunk_starts(&hdr);
            let mut patch = String::new();
            for fl in file_hdr {
                patch.push_str(fl);
                patch.push('\n');
            }
            patch.push_str(&hdr);
            patch.push('\n');
            for l in hunk_lines.iter() {
                patch.push_str(l);
                patch.push('\n');
            }
            let hunk_id = compute_hunk_id(path, &hdr, &patch);
            hunks_out.push(ReviewHunk {
                hunk_id,
                path: path.clone(),
                header: hdr,
                old_start,
                new_start,
                additions: adds,
                deletions: dels,
                patch,
            });
            hunk_lines.clear();
        }
    };

    let flush_file = |path_opt: &mut Option<String>,
                      file_hdr: &mut Vec<String>,
                      hunks: &mut Vec<ReviewHunk>,
                      files_out: &mut Vec<ReviewFile>| {
        if let Some(path) = path_opt.take() {
            let adds: u32 = hunks.iter().map(|h| h.additions).sum();
            let dels: u32 = hunks.iter().map(|h| h.deletions).sum();
            let risk_markers = detect_risk_markers(&path, adds, dels);
            files_out.push(ReviewFile {
                path,
                additions: adds,
                deletions: dels,
                risk_markers,
                hunks: std::mem::take(hunks),
            });
            file_hdr.clear();
        }
    };

    for line in raw_diff.lines() {
        if line.starts_with("diff --git ") {
            flush_hunk(
                &cur_path,
                &cur_file_header,
                &mut cur_hunk_header,
                &mut cur_hunk_lines,
                &mut cur_hunks,
            );
            flush_file(
                &mut cur_path,
                &mut cur_file_header,
                &mut cur_hunks,
                &mut files,
            );
            let path = line.split(" b/").nth(1).unwrap_or("").trim().to_string();
            cur_path = (!path.is_empty()).then_some(path);
            cur_file_header.push(line.to_string());
        } else if line.starts_with("@@ ") {
            flush_hunk(
                &cur_path,
                &cur_file_header,
                &mut cur_hunk_header,
                &mut cur_hunk_lines,
                &mut cur_hunks,
            );
            cur_hunk_header = Some(line.to_string());
        } else if cur_hunk_header.is_some() {
            cur_hunk_lines.push(line.to_string());
        } else if cur_path.is_some() {
            cur_file_header.push(line.to_string());
        }
    }

    flush_hunk(
        &cur_path,
        &cur_file_header,
        &mut cur_hunk_header,
        &mut cur_hunk_lines,
        &mut cur_hunks,
    );
    flush_file(
        &mut cur_path,
        &mut cur_file_header,
        &mut cur_hunks,
        &mut files,
    );

    let total_files = files.len();
    let total_additions = files.iter().map(|f| f.additions).sum();
    let total_deletions = files.iter().map(|f| f.deletions).sum();
    let risky_files = files.iter().filter(|f| !f.risk_markers.is_empty()).count();

    SessionReview {
        base_branch: base_branch.to_string(),
        total_files,
        total_additions,
        total_deletions,
        risky_files,
        files,
    }
}

/// Menerima satu hunk (stage ke indeks Git via `git apply --cached`) (`IT-210`).
pub async fn accept_hunk_patch(worktree_path: &Path, patch: &str) -> Result<(), GitError> {
    apply_patch_stdin(worktree_path, &["apply", "--cached", "--recount"], patch).await
}

/// Menolak/merevert satu hunk via `git apply -R` dan menyimpan patch cadangan undo (`0600`) (`IT-210`).
pub async fn reject_hunk_patch(
    worktree_path: &Path,
    undo_dir: &Path,
    patch: &str,
) -> Result<String, GitError> {
    apply_patch_stdin(worktree_path, &["apply", "-R", "--recount"], patch).await?;
    tokio::fs::create_dir_all(undo_dir).await?;
    let token = new_id();
    let undo_file = undo_dir.join(format!("hunk-{token}.patch"));
    tokio::fs::write(&undo_file, patch.as_bytes()).await?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&undo_file, std::fs::Permissions::from_mode(0o600));
    }
    Ok(token)
}

/// Mengembalikan hunk yang sebelumnya ditolak menggunakan `undo_token` (`POST /review/undo`).
pub async fn undo_rejected_hunk(
    worktree_path: &Path,
    undo_dir: &Path,
    undo_token: &str,
) -> Result<(), GitError> {
    let trimmed = undo_token.trim();
    if trimmed.is_empty()
        || !trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
    {
        return Err(GitError::CommandFailed {
            cmd: "review-undo".to_string(),
            stderr: "invalid undo_token format".to_string(),
        });
    }
    let undo_file = undo_dir.join(format!("hunk-{trimmed}.patch"));
    let patch =
        tokio::fs::read_to_string(&undo_file)
            .await
            .map_err(|_| GitError::CommandFailed {
                cmd: "review-undo".to_string(),
                stderr: "undo token not found or already applied".to_string(),
            })?;
    apply_patch_stdin(worktree_path, &["apply", "--recount"], &patch).await?;
    let _ = tokio::fs::remove_file(&undo_file).await;
    Ok(())
}

async fn apply_patch_stdin(
    worktree_path: &Path,
    args: &[&str],
    patch: &str,
) -> Result<(), GitError> {
    let mut child = Command::new("git")
        .arg("-C")
        .arg(worktree_path)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(patch.as_bytes()).await?;
        stdin.shutdown().await?;
    }
    let out = child.wait_with_output().await?;
    if out.status.success() {
        Ok(())
    } else {
        Err(GitError::CommandFailed {
            cmd: args.join(" "),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })
    }
}

fn parse_hunk_starts(header: &str) -> (u32, u32) {
    let parts: Vec<&str> = header.split_whitespace().collect();
    let old_start = parts
        .get(1)
        .and_then(|s| s.strip_prefix('-'))
        .and_then(|s| s.split(',').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1);
    let new_start = parts
        .get(2)
        .and_then(|s| s.strip_prefix('+'))
        .and_then(|s| s.split(',').next())
        .and_then(|n| n.parse().ok())
        .unwrap_or(1);
    (old_start, new_start)
}

fn compute_hunk_id(path: &str, header: &str, patch: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(path.as_bytes());
    hasher.update(b"\n");
    hasher.update(header.as_bytes());
    hasher.update(b"\n");
    hasher.update(patch.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(16);
    for b in &digest[..8] {
        let _ = write!(&mut hex, "{b:02x}");
    }
    hex
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    use tempfile::TempDir;

    use super::{
        accept_hunk_patch, build_session_review, commit_local, detect_risk_markers,
        reject_hunk_patch, stage_paths, sync_base_branch, undo_rejected_hunk, unstage_paths,
        worktree_blame, worktree_log, worktree_status,
    };

    fn run_cmd(dir: &Path, args: &[&str]) {
        let st = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(st.success(), "git {:?} failed", args);
    }

    #[tokio::test]
    async fn local_git_ops_hunk_reject_undo_and_sync_base_work() {
        let dir = TempDir::new().unwrap();
        let repo = dir.path().join("repo");
        let undo = dir.path().join("undo");
        std::fs::create_dir_all(&repo).unwrap();
        run_cmd(&repo, &["init", "-b", "main"]);
        run_cmd(&repo, &["config", "user.name", "Latch"]);
        run_cmd(&repo, &["config", "user.email", "latch@localhost"]);
        std::fs::write(repo.join("auth.rs"), "fn a() {}\nfn b() {}\n").unwrap();
        run_cmd(&repo, &["add", "."]);
        run_cmd(&repo, &["commit", "-m", "init"]);

        // Buat branch kerja
        run_cmd(&repo, &["checkout", "-b", "latch/feat"]);
        std::fs::write(repo.join("auth.rs"), "fn a() { /* edited */ }\nfn b() {}\n").unwrap();

        let status = worktree_status(&repo, "main").await.unwrap();
        assert_eq!(status.unstaged.len(), 1);
        assert!(detect_risk_markers("src/auth.rs", 2, 1).contains(&"security".to_string()));

        // Review & reject hunk -> revert file -> undo -> restore edit!
        let review = build_session_review(&repo, "main").await.unwrap();
        assert_eq!(review.total_files, 1);
        assert_eq!(review.risky_files, 1);
        let hunk = &review.files[0].hunks[0];

        let undo_token = reject_hunk_patch(&repo, &undo, &hunk.patch).await.unwrap();
        assert_eq!(
            std::fs::read_to_string(repo.join("auth.rs")).unwrap(),
            "fn a() {}\nfn b() {}\n"
        );
        undo_rejected_hunk(&repo, &undo, &undo_token).await.unwrap();
        assert!(std::fs::read_to_string(repo.join("auth.rs"))
            .unwrap()
            .contains("/* edited */"));

        // Accept hunk (stage), unstage, stage, commit, log, blame, sync-base
        accept_hunk_patch(&repo, &hunk.patch).await.unwrap();
        unstage_paths(&repo, &["auth.rs".to_string()])
            .await
            .unwrap();
        stage_paths(&repo, &["auth.rs".to_string()]).await.unwrap();
        let committed = commit_local(&repo, "feat: edit auth", &[]).await.unwrap();
        assert!(!committed.commit_oid.is_empty());

        let log = worktree_log(&repo, Some("auth.rs"), 10).await.unwrap();
        assert_eq!(log.len(), 2);
        let blame = worktree_blame(&repo, "auth.rs").await.unwrap();
        assert_eq!(blame.len(), 2);

        let sync = sync_base_branch(&repo, "main").await.unwrap();
        assert_eq!(sync.status, "clean");
    }
}
