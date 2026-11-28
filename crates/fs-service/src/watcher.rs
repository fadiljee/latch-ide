//! Pemantau perubahan berkas worktree berbatas memori dengan debounce (`IT-203`, `IT-207`).
//!
//! Menggunakan snapshot metadata ringan tanpa dependensi tambahan, melewatkan `.git/`, `target/`,
//! `node_modules/`, serta membedakan penulisan oleh pengguna (`fs_write`) vs perubahan oleh agen.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;
use std::time::UNIX_EPOCH;

/// Batas maksimum berkas yang dipantau per worktree agar RAM daemon tetap ramping (`NFR-I05`).
pub const MAX_WATCHED_FILES: usize = 20_000;
/// Batas maksimum jalur yang dikirim dalam satu batch event `fs.changed`.
pub const MAX_BATCH_PATHS: usize = 128;

/// Direktori berat atau terlarang yang otomatis dilewati pemantau.
const IGNORED_DIRS: &[&str] = &[
    ".git",
    "target",
    "node_modules",
    ".venv",
    "venv",
    "__pycache__",
    ".next",
    "dist",
    "build",
];

/// Metadata ringkas untuk mendeteksi perubahan satu berkas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileStamp {
    /// Ukuran berkas (byte).
    pub size: u64,
    /// Waktu modifikasi terakhir (UTC epoch ms).
    pub mtime_ms: i64,
}

/// Snapshot metadata seluruh berkas worktree yang dipantau untuk satu sesi.
#[derive(Debug, Clone, Default)]
pub struct WorktreeSnapshot {
    files: HashMap<String, FileStamp>,
}

impl WorktreeSnapshot {
    /// Memindai worktree dan membangun snapshot awal berbatas `MAX_WATCHED_FILES`.
    pub fn scan(root: &Path) -> Self {
        let mut files = HashMap::new();
        if let Ok(canonical) = fs::canonicalize(root) {
            scan_dir(&canonical, "", &mut files);
        }
        Self { files }
    }

    /// Jumlah berkas dalam snapshot saat ini.
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Apakah snapshot kosong.
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    /// Membandingkan snapshot lama dengan snapshot baru dan mengembalikan daftar jalur relatif yang berubah.
    pub fn diff_changed_paths(&self, next: &Self) -> Vec<String> {
        let mut changed: Vec<String> = Vec::new();
        for (path, stamp) in &next.files {
            if self.files.get(path) != Some(stamp) {
                changed.push(path.clone());
            }
        }
        for path in self.files.keys() {
            if !next.files.contains_key(path) {
                changed.push(path.clone());
            }
        }
        changed.sort();
        changed.truncate(MAX_BATCH_PATHS);
        changed
    }
}

/// Pelacak perubahan worktree per sesi sekaligus pencatat berkas yang disentuh agen (`IT-207`).
#[derive(Debug, Default)]
pub struct SessionWatchTracker {
    snapshots: HashMap<String, WorktreeSnapshot>,
    recent_user_writes: HashMap<String, HashMap<String, i64>>,
    agent_touched: HashMap<String, HashSet<String>>,
}

impl SessionWatchTracker {
    /// Mencatat penulisan pengguna (`fs_write`/`fs_op`) agar tidak disalahartikan sebagai sentuhan agen.
    pub fn record_user_write(&mut self, session_id: &str, paths: &[String], now_ms: i64) {
        let entry = self
            .recent_user_writes
            .entry(session_id.to_string())
            .or_default();
        for p in paths {
            entry.insert(p.clone(), now_ms);
        }
        entry.retain(|_, ts| now_ms.saturating_sub(*ts) < 5_000);
    }

    /// Memindai ulang satu worktree sesi dan mengembalikan jalur yang berubah di luar penulisan UI (`by = "agent"`).
    pub fn poll_session(
        &mut self,
        session_id: &str,
        worktree_root: &Path,
        now_ms: i64,
    ) -> Vec<String> {
        let next = WorktreeSnapshot::scan(worktree_root);
        let Some(prev) = self.snapshots.get(session_id) else {
            self.snapshots.insert(session_id.to_string(), next);
            return Vec::new();
        };

        let raw_changed = prev.diff_changed_paths(&next);
        self.snapshots.insert(session_id.to_string(), next);
        if raw_changed.is_empty() {
            return Vec::new();
        }

        let mut user_map = self.recent_user_writes.get_mut(session_id);
        if let Some(map) = user_map.as_deref_mut() {
            map.retain(|_, ts| now_ms.saturating_sub(*ts) < 2_000);
        }

        let mut agent_paths = Vec::new();
        for p in raw_changed {
            let is_recent_user = user_map.as_deref().is_some_and(|m| {
                m.get(&p)
                    .is_some_and(|ts| now_ms.saturating_sub(*ts) < 2_000)
            });
            if !is_recent_user {
                agent_paths.push(p);
            }
        }

        if !agent_paths.is_empty() {
            let touched = self
                .agent_touched
                .entry(session_id.to_string())
                .or_default();
            for p in &agent_paths {
                touched.insert(p.clone());
            }
        }

        agent_paths
    }

    /// Daftar berkas yang terdeteksi disentuh agen untuk sesi tertentu (`IT-207`).
    pub fn agent_touched_paths(&self, session_id: &str) -> Vec<String> {
        let mut out: Vec<String> = self
            .agent_touched
            .get(session_id)
            .map(|set| set.iter().cloned().collect())
            .unwrap_or_default();
        out.sort();
        out
    }

    /// Membersihkan state pelacak untuk sesi yang sudah tidak aktif.
    pub fn retain_sessions(&mut self, active_ids: &HashSet<String>) {
        self.snapshots.retain(|id, _| active_ids.contains(id));
        self.recent_user_writes
            .retain(|id, _| active_ids.contains(id));
    }
}

fn scan_dir(dir: &Path, rel_prefix: &str, out: &mut HashMap<String, FileStamp>) {
    if out.len() >= MAX_WATCHED_FILES {
        return;
    }
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        if out.len() >= MAX_WATCHED_FILES {
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
        let Ok(sym_meta) = fs::symlink_metadata(&path) else {
            continue;
        };
        if sym_meta.file_type().is_symlink() {
            continue;
        }
        let child_rel = if rel_prefix.is_empty() {
            name
        } else {
            format!("{rel_prefix}/{name}")
        };
        if sym_meta.is_dir() {
            scan_dir(&path, &child_rel, out);
        } else if sym_meta.is_file() {
            let mtime_ms = sym_meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .and_then(|d| i64::try_from(d.as_millis()).ok())
                .unwrap_or(0);
            out.insert(
                child_rel,
                FileStamp {
                    size: sym_meta.len(),
                    mtime_ms,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::SessionWatchTracker;

    #[test]
    fn detects_agent_file_changes_and_filters_recent_user_writes() {
        let dir = TempDir::new().unwrap();
        let wt = dir.path();
        fs::create_dir_all(wt.join("src")).unwrap();
        fs::create_dir_all(wt.join(".git")).unwrap();
        fs::write(wt.join("src/a.rs"), "v1").unwrap();

        let mut tracker = SessionWatchTracker::default();
        // Poll pertama membangun baseline tanpa event palsu
        assert!(tracker.poll_session("ses-1", wt, 1_000).is_empty());

        // Ubah berkas oleh agen + ubah .git (harus diabaikan) + ubah berkas yang dicatat sebagai user write
        fs::write(wt.join("src/a.rs"), "v2-by-agent").unwrap();
        fs::write(wt.join("src/user.rs"), "by-user").unwrap();
        fs::write(wt.join(".git/index"), "ignored").unwrap();
        tracker.record_user_write("ses-1", &["src/user.rs".to_string()], 1_500);

        let changed = tracker.poll_session("ses-1", wt, 1_800);
        assert_eq!(changed, vec!["src/a.rs".to_string()]);
        assert_eq!(
            tracker.agent_touched_paths("ses-1"),
            vec!["src/a.rs".to_string()]
        );
    }
}
