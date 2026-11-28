#![forbid(unsafe_code)]
//! Layanan pencarian nama berkas fuzzy (`IT-211`), pencarian teks linear-regex (`IT-301`),
//! dan ekstraksi outline simbol ringan (`IT-509`) untuk Agent IDE.

use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use latch_core::id::now_ms;
use regex::RegexBuilder;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Batas ukuran berkas maksimum yang dipindai oleh pencarian teks (`IDE_API.md` §4: 2 MiB).
pub const MAX_SEARCH_FILE_BYTES: u64 = 2 * 1024 * 1024;
/// Batas default jumlah hasil pencarian teks (`IDE_API.md` §4).
pub const DEFAULT_MAX_RESULTS: usize = 2_000;
/// Batas maksimum waktu eksekusi pencarian teks (`IDE_API.md` §4: 10 detik).
pub const MAX_SEARCH_DURATION: Duration = Duration::from_secs(10);
/// Batas maksimum berkas yang diindeks untuk Quick Open (`NFR-I05`).
pub const MAX_INDEXED_FILES: usize = 50_000;

const IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    "dist",
];

/// Kesalahan subsistem pencarian.
#[derive(Debug, Error)]
pub enum SearchError {
    /// Query pencarian kosong atau terlalu panjang.
    #[error("invalid search query: {0}")]
    InvalidQuery(String),
    /// Ekspresi reguler tidak valid atau melampaui batas ukuran DFA.
    #[error("invalid regex pattern: {0}")]
    InvalidRegex(String),
    /// Galat I/O saat memindai worktree.
    #[error("search I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Metadata indeks pencarian untuk satu workspace.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Waktu pembuatan indeks (UTC epoch ms).
    pub created_at: i64,
}

impl IndexMetadata {
    /// Membuat metadata indeks dengan stempel waktu UTC saat ini.
    pub fn new_now() -> Self {
        Self {
            created_at: now_ms(),
        }
    }
}

/// Satu hasil pencarian nama berkas fuzzy (`GET /sessions/{sid}/search/files`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileSearchHit {
    /// Jalur relatif terhadap akar worktree.
    pub path: String,
    /// Skor kecocokan fuzzy (lebih besar = lebih relevan).
    pub score: i32,
}

/// Permintaan pencarian teks (`POST /sessions/{sid}/search/text`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSearchRequest {
    /// Kata kunci atau pola regex.
    pub query: String,
    /// Perlakukan `query` sebagai ekspresi reguler linear (non-backtracking).
    #[serde(default)]
    pub regex: bool,
    /// Pencocokan peka huruf besar/kecil.
    #[serde(default)]
    pub case_sensitive: bool,
    /// Filter pola akhiran/jalur opsional (mis. `*.rs`, `src/`).
    #[serde(default)]
    pub globs: Vec<String>,
    /// Batas hasil (dibatasi `1..=2000`).
    #[serde(default)]
    pub max_results: Option<usize>,
}

/// Satu baris kecocokan pencarian teks (`search.result`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSearchMatch {
    /// Jalur relatif berkas.
    pub path: String,
    /// Nomor baris berbasis 1.
    pub line: usize,
    /// Nomor kolom berbasis 1.
    pub column: usize,
    /// Cuplikan baris teks (dipotong aman maks 240 karakter).
    pub text: String,
}

/// Ringkasan hasil pencarian teks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextSearchSummary {
    /// Daftar kecocokan.
    pub matches: Vec<TextSearchMatch>,
    /// Benar bila pencarian dipotong karena mencapai `max_results`, timeout, atau pembatalan.
    pub truncated: bool,
    /// Benar bila dihentikan oleh batas waktu 10 detik.
    pub timed_out: bool,
    /// Jumlah berkas yang dipindai.
    pub scanned_files: usize,
}

/// Satu simbol dasar untuk tampilan Outline (`IT-509`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutlineSymbol {
    /// Nama simbol (fungsi, struct, class, heading).
    pub name: String,
    /// Jenis simbol (`fn`, `struct`, `enum`, `trait`, `class`, `interface`, `heading`, dll.).
    pub kind: String,
    /// Nomor baris berbasis 1.
    pub line: usize,
}

/// Menghitung skor kecocokan fuzzy subsequence (peka batas segmen & prefiks nama berkas).
pub fn fuzzy_score(candidate: &str, query: &str) -> Option<i32> {
    let q = query.trim().to_ascii_lowercase();
    if q.is_empty() {
        return Some(0);
    }
    let cand_lower = candidate.to_ascii_lowercase();
    let cand_bytes = cand_lower.as_bytes();
    let q_bytes = q.as_bytes();

    let mut score = 0i32;
    let mut ci = 0usize;
    let mut consecutive = 0i32;
    let file_name_start = cand_lower.rfind('/').map_or(0, |p| p + 1);

    for &qb in q_bytes {
        let mut found = None;
        while ci < cand_bytes.len() {
            if cand_bytes[ci] == qb {
                found = Some(ci);
                ci += 1;
                break;
            }
            ci += 1;
            consecutive = 0;
        }
        let pos = found?;
        score += 10;
        if pos >= file_name_start {
            score += 6;
        }
        if pos == 0
            || pos == file_name_start
            || matches!(
                cand_bytes.get(pos.saturating_sub(1)),
                Some(b'/' | b'_' | b'-' | b'.')
            )
        {
            score += 12;
        }
        consecutive += 1;
        score += consecutive * 3;
    }

    // Bonus kuat untuk kecocokan substring langsung pada nama berkas (agar `main.rs` mengungguli `my_app_init.rs`).
    let file_name = &cand_lower[file_name_start..];
    if file_name.starts_with(&q) {
        score += 50;
    } else if file_name.contains(&q) {
        score += 30;
    }

    // Beri bonus untuk jalur yang lebih ringkas.
    score -= i32::try_from(candidate.len().min(200)).unwrap_or(200) / 4;
    Some(score)
}

/// Mencari berkas secara fuzzy di dalam worktree (`IT-211`).
pub fn search_files(
    root: &Path,
    query: &str,
    limit: usize,
) -> Result<Vec<FileSearchHit>, SearchError> {
    let cap = limit.clamp(1, 200);
    let mut all_paths = Vec::new();
    if let Ok(canonical) = fs::canonicalize(root) {
        collect_paths(&canonical, "", &mut all_paths);
    }

    let mut hits: Vec<FileSearchHit> = all_paths
        .into_iter()
        .filter_map(|path| fuzzy_score(&path, query).map(|score| FileSearchHit { path, score }))
        .collect();

    hits.sort_by(|a, b| b.score.cmp(&a.score).then_with(|| a.path.cmp(&b.path)));
    hits.truncate(cap);
    Ok(hits)
}

/// Menjalankan pencarian teks/regex linear-time dengan batas waktu, ukuran berkas, dan pembatalan (`IT-301`, `IT11`).
pub fn search_text<F>(
    root: &Path,
    req: &TextSearchRequest,
    cancel: &AtomicBool,
    mut on_match: F,
) -> Result<TextSearchSummary, SearchError>
where
    F: FnMut(&TextSearchMatch),
{
    let query = req.query.trim();
    if query.is_empty() {
        return Err(SearchError::InvalidQuery(
            "query must not be empty".to_string(),
        ));
    }
    if query.len() > 512 {
        return Err(SearchError::InvalidQuery(
            "query exceeds 512 characters".to_string(),
        ));
    }

    let pattern = if req.regex {
        query.to_string()
    } else {
        regex::escape(query)
    };
    let re = RegexBuilder::new(&pattern)
        .case_insensitive(!req.case_sensitive)
        .size_limit(2 * 1024 * 1024)
        .dfa_size_limit(2 * 1024 * 1024)
        .build()
        .map_err(|e| SearchError::InvalidRegex(e.to_string()))?;

    let max_results = req
        .max_results
        .unwrap_or(DEFAULT_MAX_RESULTS)
        .clamp(1, DEFAULT_MAX_RESULTS);
    let started = Instant::now();

    let canonical = fs::canonicalize(root)?;
    let mut paths = Vec::new();
    collect_paths(&canonical, "", &mut paths);

    let mut matches = Vec::new();
    let mut truncated = false;
    let mut timed_out = false;
    let mut scanned_files = 0usize;

    for rel in paths {
        if cancel.load(Ordering::Relaxed) {
            truncated = true;
            break;
        }
        if started.elapsed() >= MAX_SEARCH_DURATION {
            timed_out = true;
            truncated = true;
            break;
        }
        if !matches_globs(&rel, &req.globs) {
            continue;
        }
        let full = canonical.join(&rel);
        let Ok(meta) = fs::metadata(&full) else {
            continue;
        };
        if !meta.is_file() || meta.len() > MAX_SEARCH_FILE_BYTES {
            continue;
        }
        let Ok(bytes) = fs::read(&full) else {
            continue;
        };
        let sample_len = bytes.len().min(4096);
        if bytes[..sample_len].contains(&0) {
            continue;
        }
        let Ok(content) = std::str::from_utf8(&bytes) else {
            continue;
        };
        scanned_files += 1;

        for (idx, line_str) in content.lines().enumerate() {
            if let Some(m) = re.find(line_str) {
                let col = line_str[..m.start()].chars().count() + 1;
                let snippet: String = line_str.chars().take(240).collect();
                let hit = TextSearchMatch {
                    path: rel.clone(),
                    line: idx + 1,
                    column: col,
                    text: snippet,
                };
                on_match(&hit);
                matches.push(hit);
                if matches.len() >= max_results {
                    truncated = true;
                    break;
                }
            }
        }
        if truncated {
            break;
        }
    }

    Ok(TextSearchSummary {
        matches,
        truncated,
        timed_out,
        scanned_files,
    })
}

/// Mengekstrak simbol dasar (fungsi, struct, class, interface, heading Markdown) tanpa LSP (`IT-509`).
pub fn extract_outline(path: &str, content: &str) -> Vec<OutlineSymbol> {
    let ext = path
        .rsplit('.')
        .next()
        .map(str::to_ascii_lowercase)
        .unwrap_or_default();
    let mut out = Vec::new();
    for (idx, raw_line) in content.lines().enumerate() {
        let line_no = idx + 1;
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if matches!(ext.as_str(), "md" | "markdown") {
            if let Some(rest) = line.strip_prefix('#') {
                let title = rest.trim_start_matches('#').trim();
                if !title.is_empty() {
                    out.push(OutlineSymbol {
                        name: title.to_string(),
                        kind: "heading".to_string(),
                        line: line_no,
                    });
                }
            }
            continue;
        }
        let cleaned = line
            .strip_prefix("pub(crate) ")
            .or_else(|| line.strip_prefix("pub(super) "))
            .or_else(|| line.strip_prefix("pub "))
            .or_else(|| line.strip_prefix("export default "))
            .or_else(|| line.strip_prefix("export "))
            .unwrap_or(line);
        let cleaned = cleaned.strip_prefix("async ").unwrap_or(cleaned);
        let cleaned = cleaned.strip_prefix("const ").unwrap_or(cleaned);

        for (kw, kind) in [
            ("fn ", "fn"),
            ("struct ", "struct"),
            ("enum ", "enum"),
            ("trait ", "trait"),
            ("impl ", "impl"),
            ("function ", "function"),
            ("class ", "class"),
            ("interface ", "interface"),
            ("type ", "type"),
            ("def ", "def"),
            ("func ", "func"),
        ] {
            if let Some(after) = cleaned.strip_prefix(kw) {
                let name: String = after
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | ':' | '<' | '>'))
                    .collect();
                if !name.is_empty() {
                    out.push(OutlineSymbol {
                        name,
                        kind: kind.to_string(),
                        line: line_no,
                    });
                }
                break;
            }
        }
    }
    out
}

fn matches_globs(rel_path: &str, globs: &[String]) -> bool {
    if globs.is_empty() {
        return true;
    }
    globs.iter().any(|raw| {
        let g = raw.trim();
        if g.is_empty() {
            return true;
        }
        if let Some(ext) = g.strip_prefix("*.") {
            rel_path.ends_with(&format!(".{ext}"))
        } else if let Some(prefix) = g.strip_suffix("/**") {
            rel_path.starts_with(prefix)
        } else {
            rel_path.contains(g)
        }
    })
}

fn collect_paths(dir: &Path, rel_prefix: &str, out: &mut Vec<String>) {
    if out.len() >= MAX_INDEXED_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= MAX_INDEXED_FILES {
            break;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        if IGNORED_DIRS
            .iter()
            .any(|ignored| name.eq_ignore_ascii_case(ignored))
        {
            continue;
        }
        let path = entry.path();
        let Ok(meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if meta.file_type().is_symlink() {
            continue;
        }
        let rel = if rel_prefix.is_empty() {
            name
        } else {
            format!("{rel_prefix}/{name}")
        };
        if meta.is_dir() {
            collect_paths(&path, &rel, out);
        } else if meta.is_file() {
            out.push(rel);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::sync::atomic::AtomicBool;
    use std::time::Instant;

    use tempfile::TempDir;

    use super::{
        extract_outline, fuzzy_score, search_files, search_text, IndexMetadata, SearchError,
        TextSearchRequest,
    };

    #[test]
    fn fuzzy_file_search_ranks_exact_and_segment_matches_highest() {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join("src/features")).unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        fs::write(
            dir.path().join("src/features/SessionBoard.tsx"),
            "export {}",
        )
        .unwrap();
        fs::write(dir.path().join("src/other.txt"), "hello").unwrap();
        fs::write(dir.path().join(".git/config"), "hidden").unwrap();

        let hits = search_files(dir.path(), "sesbd", 20).unwrap();
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].path, "src/features/SessionBoard.tsx");
        assert!(
            fuzzy_score("a/b/main.rs", "main").unwrap()
                > fuzzy_score("x/my_app_init.rs", "main").unwrap()
        );
        assert!(IndexMetadata::new_now().created_at > 0);
        assert!(!SearchError::InvalidQuery("x".into()).to_string().is_empty());
    }

    #[test]
    fn text_search_uses_linear_regex_without_catastrophic_backtracking() {
        let dir = TempDir::new().unwrap();
        let long_a = "a".repeat(5_000);
        fs::write(
            dir.path().join("evil.txt"),
            format!("{long_a}!\nneedle_here\n"),
        )
        .unwrap();

        let cancel = AtomicBool::new(false);
        let t0 = Instant::now();
        let res = search_text(
            dir.path(),
            &TextSearchRequest {
                query: "(a+)+b".to_string(),
                regex: true,
                case_sensitive: false,
                globs: Vec::new(),
                max_results: Some(50),
            },
            &cancel,
            |_| {},
        )
        .unwrap();
        assert!(res.matches.is_empty());
        // Mesin regex linear wajib selesai jauh di bawah 1 detik pada pola patologis (IT11)
        assert!(t0.elapsed().as_millis() < 1000);

        let hit = search_text(
            dir.path(),
            &TextSearchRequest {
                query: "needle_here".to_string(),
                regex: false,
                case_sensitive: true,
                globs: vec!["*.txt".to_string()],
                max_results: Some(50),
            },
            &cancel,
            |_| {},
        )
        .unwrap();
        assert_eq!(hit.matches.len(), 1);
        assert_eq!(hit.matches[0].line, 2);
    }

    #[test]
    fn extracts_outline_for_rust_ts_and_markdown() {
        let rs = "pub struct Foo {}\npub async fn bar() {}\n";
        let syms = extract_outline("src/lib.rs", rs);
        assert_eq!(syms.len(), 2);
        assert_eq!(syms[0].name, "Foo");
        assert_eq!(syms[1].name, "bar");

        let md = "# Title\n## Section A\nbody\n";
        let headings = extract_outline("README.md", md);
        assert_eq!(headings.len(), 2);
        assert_eq!(headings[1].name, "Section A");
    }
}
