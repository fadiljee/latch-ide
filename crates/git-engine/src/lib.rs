#![forbid(unsafe_code)]
//! Pengelola repositori Git, worktree terisolasi, diff, simulasi merge-tree, dan merge/discard (T-110..T-114).

pub mod ide;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use latch_core::id::new_id;
use latch_core::model::{
    DiffFileStat, DiffFileStatus, MergeCheckResult, MergeStrategy, SessionDiff,
};
use thiserror::Error;
use tokio::process::Command;

/// Kesalahan operasi Git Engine.
#[derive(Debug, Error)]
pub enum GitError {
    /// Versi `git` di host kurang dari syarat minimum 2.38 atau tidak terpasang.
    #[error("git >= 2.38 is required (found: {0})")]
    UnsupportedGitVersion(String),
    /// Path bukan repositori Git yang valid.
    #[error("not a valid git repository: {0}")]
    NotARepo(String),
    /// Perintah `git` gagal dijalankan.
    #[error("git command '{cmd}' failed: {stderr}")]
    CommandFailed {
        /// Subcommand yang gagal.
        cmd: String,
        /// Pesan galat stderr.
        stderr: String,
    },
    /// Galat I/O filesystem.
    #[error("git I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Terdapat konflik merge pada branch target.
    #[error("merge conflicts detected in: {}", .0.join(", "))]
    MergeConflict(Vec<String>),
    /// Working tree repositori utama kotor sehingga tidak aman untuk melakukan merge.
    #[error(
        "primary repository working tree has uncommitted changes; commit or stash before merging"
    )]
    DirtyMainWorktree,
}

/// Informasi repositori yang telah divalidasi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoInfo {
    /// Jalur akar canonical repositori.
    pub root: PathBuf,
    /// Branch default hasil deteksi.
    pub default_branch: String,
}

/// Entri hasil `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreeEntry {
    /// Lokasi direktori worktree.
    pub path: PathBuf,
    /// Commit HEAD saat ini jika ada.
    pub head: Option<String>,
    /// Nama branch pendek jika terikat branch.
    pub branch: Option<String>,
    /// Apakah worktree utama bersifat bare.
    pub is_bare: bool,
}

/// Memeriksa ketersediaan `git >= 2.38` di host.
pub async fn check_git_version() -> Result<(u32, u32), GitError> {
    let out = Command::new("git")
        .arg("--version")
        .output()
        .await
        .map_err(|e| GitError::UnsupportedGitVersion(e.to_string()))?;
    if !out.status.success() {
        return Err(GitError::UnsupportedGitVersion(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    parse_git_version(&text)
}

/// Mem-parse string keluaran `git --version` dan memeriksa `>= 2.38`.
pub fn parse_git_version(raw: &str) -> Result<(u32, u32), GitError> {
    let ver_token = raw
        .split_whitespace()
        .find(|tok| tok.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .ok_or_else(|| GitError::UnsupportedGitVersion(raw.trim().to_string()))?;

    let mut parts = ver_token.split('.');
    let major: u32 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| GitError::UnsupportedGitVersion(raw.trim().to_string()))?;
    let minor: u32 = parts
        .next()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| GitError::UnsupportedGitVersion(raw.trim().to_string()))?;

    if major > 2 || (major == 2 && minor >= 38) {
        Ok((major, minor))
    } else {
        Err(GitError::UnsupportedGitVersion(format!("{major}.{minor}")))
    }
}

/// Memvalidasi repositori Git, memeriksa versi `git`, dan menentukan branch default.
pub async fn inspect_repo(repo_path: &Path) -> Result<RepoInfo, GitError> {
    check_git_version().await?;

    let top = run_git(repo_path, &["rev-parse", "--show-toplevel"])
        .await
        .map_err(|_| GitError::NotARepo(repo_path.display().to_string()))?;
    let root = PathBuf::from(top.trim());

    // Pastikan HEAD valid (repo memiliki minimal satu commit agar worktree dapat dibuat)
    run_git(&root, &["rev-parse", "--verify", "HEAD"])
        .await
        .map_err(|_| GitError::CommandFailed {
            cmd: "rev-parse --verify HEAD".to_string(),
            stderr: "repository has no commits yet; create an initial commit first".to_string(),
        })?;

    let default_branch = if let Ok(sym) = run_git(
        &root,
        &["symbolic-ref", "--short", "refs/remotes/origin/HEAD"],
    )
    .await
    {
        sym.trim()
            .strip_prefix("origin/")
            .unwrap_or(sym.trim())
            .to_string()
    } else if let Ok(curr) = run_git(&root, &["symbolic-ref", "--short", "HEAD"]).await {
        curr.trim().to_string()
    } else {
        "main".to_string()
    };

    Ok(RepoInfo {
        root,
        default_branch,
    })
}

/// Membuat worktree dan branch terisolasi untuk satu sesi agen.
pub async fn create_worktree(
    repo_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
    base_ref: &str,
) -> Result<(), GitError> {
    if let Some(parent) = worktree_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    let wt_str = worktree_path.to_string_lossy().to_string();
    run_git(
        repo_path,
        &[
            "-c",
            "core.hooksPath=/dev/null",
            "worktree",
            "add",
            "-b",
            branch_name,
            &wt_str,
            base_ref,
        ],
    )
    .await?;
    Ok(())
}

/// Menghapus worktree dan branch sesi secara idempoten.
pub async fn remove_worktree(
    repo_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
) -> Result<(), GitError> {
    if worktree_path.exists() {
        let wt_str = worktree_path.to_string_lossy().to_string();
        let _ = run_git(repo_path, &["worktree", "remove", "--force", &wt_str]).await;
        if worktree_path.exists() {
            let _ = tokio::fs::remove_dir_all(worktree_path).await;
        }
    }
    let _ = run_git(repo_path, &["worktree", "prune"]).await;
    let _ = run_git(repo_path, &["branch", "-D", branch_name]).await;
    Ok(())
}

/// Mendaftar semua worktree pada repositori menggunakan `--porcelain`.
pub async fn list_worktrees(repo_path: &Path) -> Result<Vec<WorktreeEntry>, GitError> {
    let raw = run_git(repo_path, &["worktree", "list", "--porcelain"]).await?;
    let mut entries = Vec::new();
    let mut current: Option<WorktreeEntry> = None;

    for line in raw.lines() {
        if line.is_empty() {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(WorktreeEntry {
                path: PathBuf::from(path),
                head: None,
                branch: None,
                is_bare: false,
            });
        } else if let Some(entry) = current.as_mut() {
            if let Some(head) = line.strip_prefix("HEAD ") {
                entry.head = Some(head.to_string());
            } else if let Some(branch) = line.strip_prefix("branch ") {
                entry.branch = Some(
                    branch
                        .strip_prefix("refs/heads/")
                        .unwrap_or(branch)
                        .to_string(),
                );
            } else if line == "bare" {
                entry.is_bare = true;
            }
        }
    }
    if let Some(entry) = current {
        entries.push(entry);
    }
    Ok(entries)
}

/// Menghitung daftar perubahan dan patch unified worktree dibanding `base_branch` (termasuk berkas untracked).
pub async fn worktree_diff(
    worktree_path: &Path,
    base_branch: &str,
    file_filter: Option<&str>,
    max_bytes: usize,
) -> Result<SessionDiff, GitError> {
    let mut numstat_map: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    let numstat_raw = match file_filter {
        Some(f) => {
            run_git(
                worktree_path,
                &["diff", "--numstat", "-M", base_branch, "--", f],
            )
            .await?
        }
        None => run_git(worktree_path, &["diff", "--numstat", "-M", base_branch]).await?,
    };
    for line in numstat_raw.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() >= 3 {
            let adds = cols[0].parse::<u32>().unwrap_or(0);
            let dels = cols[1].parse::<u32>().unwrap_or(0);
            let path = cols.last().copied().unwrap_or_default().to_string();
            numstat_map.insert(path, (adds, dels));
        }
    }

    let name_status_raw = match file_filter {
        Some(f) => {
            run_git(
                worktree_path,
                &["diff", "--name-status", "-M", base_branch, "--", f],
            )
            .await?
        }
        None => run_git(worktree_path, &["diff", "--name-status", "-M", base_branch]).await?,
    };

    let mut files = Vec::new();
    for line in name_status_raw.lines() {
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 2 {
            continue;
        }
        let code = cols[0];
        let (status, old_path, path) = if code.starts_with('R') && cols.len() >= 3 {
            (
                DiffFileStatus::Renamed,
                Some(cols[1].to_string()),
                cols[2].to_string(),
            )
        } else {
            let st = match code.chars().next() {
                Some('A') => DiffFileStatus::Added,
                Some('D') => DiffFileStatus::Deleted,
                _ => DiffFileStatus::Modified,
            };
            (st, None, cols[1].to_string())
        };
        let (additions, deletions) = numstat_map.get(&path).copied().unwrap_or((0, 0));
        files.push(DiffFileStat {
            path,
            old_path,
            status,
            additions,
            deletions,
        });
    }

    let mut full_patch = match file_filter {
        Some(f) => run_git(worktree_path, &["diff", "-M", base_branch, "--", f]).await?,
        None => run_git(worktree_path, &["diff", "-M", base_branch]).await?,
    };

    // Sertakan berkas baru (untracked) yang belum di-commit agen
    let untracked_raw = run_git(
        worktree_path,
        &["ls-files", "--others", "--exclude-standard"],
    )
    .await?;
    for rel in untracked_raw.lines().filter(|l| !l.is_empty()) {
        if let Some(filter) = file_filter {
            if rel != filter {
                continue;
            }
        }
        let abs = worktree_path.join(rel);
        let content_lines = tokio::fs::read(&abs)
            .await
            .map(|bytes| {
                String::from_utf8_lossy(&bytes)
                    .lines()
                    .count()
                    .try_into()
                    .unwrap_or(0)
            })
            .unwrap_or(0);
        files.push(DiffFileStat {
            path: rel.to_string(),
            old_path: None,
            status: DiffFileStatus::Added,
            additions: content_lines,
            deletions: 0,
        });

        let patch_piece = run_git_allow_exit_1(
            worktree_path,
            &["diff", "--no-index", "--", "/dev/null", rel],
        )
        .await?;
        if !patch_piece.is_empty() {
            if !full_patch.is_empty() && !full_patch.ends_with('\n') {
                full_patch.push('\n');
            }
            full_patch.push_str(&patch_piece);
        }
    }

    let total_patch_bytes = full_patch.len();
    let (patch, truncated) = if total_patch_bytes > max_bytes {
        let mut end = max_bytes;
        while end > 0 && !full_patch.is_char_boundary(end) {
            end -= 1;
        }
        (full_patch[..end].to_string(), true)
    } else {
        (full_patch, false)
    };

    Ok(SessionDiff {
        base_branch: base_branch.to_string(),
        files,
        patch,
        total_patch_bytes,
        truncated,
    })
}

/// Mensimulasikan penggabungan sesi ke `default_branch` memakai `git merge-tree --write-tree`
/// tanpa mengubah working tree utama atau worktree sesi.
pub async fn merge_check(
    repo_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
    default_branch: &str,
) -> Result<MergeCheckResult, GitError> {
    let source_commit = build_effective_commit(repo_path, worktree_path, branch_name).await?;
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_path)
        .args(["merge-tree", "--write-tree", default_branch, &source_commit])
        .output()
        .await?;

    if out.status.success() {
        return Ok(MergeCheckResult {
            clean: true,
            conflicts: Vec::new(),
        });
    }

    let code = out.status.code().unwrap_or(2);
    if code != 1 {
        return Err(GitError::CommandFailed {
            cmd: "merge-tree --write-tree".to_string(),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        });
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut conflicts = Vec::new();
    for line in stdout.lines().skip(1) {
        if let Some(rest) = line.strip_prefix("CONFLICT ") {
            if let Some(idx) = rest.rfind(" in ") {
                let path = rest[idx + 4..].trim().to_string();
                if !conflicts.contains(&path) {
                    conflicts.push(path);
                }
            }
        }
    }
    if conflicts.is_empty() {
        // Fallback jika format CONFLICT berbeda
        for line in stdout.lines().skip(1) {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.contains('\t') {
                conflicts.push(trimmed.to_string());
                break;
            }
        }
    }

    Ok(MergeCheckResult {
        clean: false,
        conflicts,
    })
}

/// Menggabungkan hasil sesi ke `default_branch` setelah verifikasi bersih `merge-tree`,
/// lalu membersihkan worktree dan branch sesi.
pub async fn merge_session(
    repo_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
    default_branch: &str,
    strategy: MergeStrategy,
    message: Option<&str>,
) -> Result<(), GitError> {
    let main_dirty = run_git(repo_path, &["status", "--porcelain"]).await?;
    if !main_dirty.trim().is_empty() {
        return Err(GitError::DirtyMainWorktree);
    }

    // Jika worktree memiliki perubahan yang belum di-commit, commit pada branch sesi terlebih dahulu
    commit_worktree_if_dirty(
        worktree_path,
        message.unwrap_or("chore: commit latch session changes"),
    )
    .await?;

    let check = merge_check(repo_path, worktree_path, branch_name, default_branch).await?;
    if !check.clean {
        return Err(GitError::MergeConflict(check.conflicts));
    }

    let current_main = run_git(repo_path, &["symbolic-ref", "--short", "HEAD"])
        .await
        .unwrap_or_default();
    if current_main.trim() != default_branch {
        run_git(
            repo_path,
            &["-c", "core.hooksPath=/dev/null", "checkout", default_branch],
        )
        .await?;
    }

    let commit_msg = message
        .map(str::to_string)
        .unwrap_or_else(|| format!("chore: merge session {branch_name}"));

    match strategy {
        MergeStrategy::Squash => {
            run_git(
                repo_path,
                &[
                    "-c",
                    "core.hooksPath=/dev/null",
                    "merge",
                    "--squash",
                    branch_name,
                ],
            )
            .await?;
            // Cek apakah ada perubahan yang di-stage sebelum commit
            let staged = run_git(repo_path, &["diff", "--cached", "--name-only"]).await?;
            if !staged.trim().is_empty() {
                run_git(
                    repo_path,
                    &[
                        "-c",
                        "core.hooksPath=/dev/null",
                        "-c",
                        "user.name=Latch",
                        "-c",
                        "user.email=latch@localhost",
                        "commit",
                        "-m",
                        &commit_msg,
                    ],
                )
                .await?;
            }
        }
        MergeStrategy::Merge => {
            run_git(
                repo_path,
                &[
                    "-c",
                    "core.hooksPath=/dev/null",
                    "-c",
                    "user.name=Latch",
                    "-c",
                    "user.email=latch@localhost",
                    "merge",
                    "--no-ff",
                    "-m",
                    &commit_msg,
                    branch_name,
                ],
            )
            .await?;
        }
    }

    remove_worktree(repo_path, worktree_path, branch_name).await?;
    Ok(())
}

async fn commit_worktree_if_dirty(worktree_path: &Path, message: &str) -> Result<(), GitError> {
    let status = run_git(worktree_path, &["status", "--porcelain"]).await?;
    if status.trim().is_empty() {
        return Ok(());
    }
    run_git(worktree_path, &["add", "-A"]).await?;
    run_git(
        worktree_path,
        &[
            "-c",
            "core.hooksPath=/dev/null",
            "-c",
            "user.name=Latch",
            "-c",
            "user.email=latch@localhost",
            "commit",
            "-m",
            message,
        ],
    )
    .await?;
    Ok(())
}

/// Menghasilkan commit OID efektif untuk `merge-tree` tanpa memodifikasi working tree
/// atau ref apabila worktree belum melakukan commit.
async fn build_effective_commit(
    repo_path: &Path,
    worktree_path: &Path,
    branch_name: &str,
) -> Result<String, GitError> {
    if !worktree_path.exists() {
        return Ok(run_git(repo_path, &["rev-parse", branch_name])
            .await?
            .trim()
            .to_string());
    }
    let status = run_git(worktree_path, &["status", "--porcelain"]).await?;
    let head = run_git(worktree_path, &["rev-parse", "HEAD"])
        .await?
        .trim()
        .to_string();
    if status.trim().is_empty() {
        return Ok(head);
    }

    let tmp_idx = std::env::temp_dir().join(format!("latch-git-idx-{}", new_id()));
    let idx_val = tmp_idx.to_string_lossy().to_string();
    let res = async {
        // Salin HEAD ke index sementara lalu add seluruh berkas worktree
        run_git_env(
            worktree_path,
            &["read-tree", &head],
            &[("GIT_INDEX_FILE", &idx_val)],
        )
        .await?;
        run_git_env(
            worktree_path,
            &["add", "-A"],
            &[("GIT_INDEX_FILE", &idx_val)],
        )
        .await?;
        let tree = run_git_env(
            worktree_path,
            &["write-tree"],
            &[("GIT_INDEX_FILE", &idx_val)],
        )
        .await?
        .trim()
        .to_string();
        let commit = run_git_env(
            worktree_path,
            &["commit-tree", &tree, "-p", &head, "-m", "latch-merge-check"],
            &[
                ("GIT_INDEX_FILE", &idx_val),
                ("GIT_AUTHOR_NAME", "Latch"),
                ("GIT_AUTHOR_EMAIL", "latch@localhost"),
                ("GIT_COMMITTER_NAME", "Latch"),
                ("GIT_COMMITTER_EMAIL", "latch@localhost"),
            ],
        )
        .await?;
        Ok::<String, GitError>(commit.trim().to_string())
    }
    .await;
    let _ = tokio::fs::remove_file(&tmp_idx).await;
    res
}

async fn run_git(cwd: &Path, args: &[&str]) -> Result<String, GitError> {
    run_git_env(cwd, args, &[]).await
}

async fn run_git_env(cwd: &Path, args: &[&str], envs: &[(&str, &str)]) -> Result<String, GitError> {
    let mut cmd = Command::new("git");
    cmd.arg("-C").arg(cwd).args(args);
    for (k, v) in envs {
        cmd.env(k, v);
    }
    let out = cmd.output().await?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(GitError::CommandFailed {
            cmd: args.join(" "),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })
    }
}

async fn run_git_allow_exit_1(cwd: &Path, args: &[&str]) -> Result<String, GitError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(args)
        .output()
        .await?;
    if out.status.success() || out.status.code() == Some(1) {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(GitError::CommandFailed {
            cmd: args.join(" "),
            stderr: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::process::Command;

    use latch_core::model::{DiffFileStatus, MergeStrategy};
    use tempfile::TempDir;

    use super::{
        check_git_version, create_worktree, inspect_repo, list_worktrees, merge_check,
        merge_session, parse_git_version, remove_worktree, worktree_diff, GitError,
    };

    fn init_repo(dir: &Path) {
        let run = |args: &[&str]| {
            let st = Command::new("git")
                .arg("-C")
                .arg(dir)
                .args(args)
                .status()
                .unwrap();
            assert!(st.success(), "git {:?} failed", args);
        };
        run(&["init", "-b", "main"]);
        run(&["config", "user.name", "Latch Test"]);
        run(&["config", "user.email", "test@localhost"]);
        std::fs::write(dir.join("README.md"), "line 1\nline 2\n").unwrap();
        run(&["add", "."]);
        run(&["commit", "-m", "initial"]);
    }

    #[test]
    fn parses_git_versions() {
        assert_eq!(parse_git_version("git version 2.38.0\n").unwrap(), (2, 38));
        assert_eq!(parse_git_version("git version 2.53.0").unwrap(), (2, 53));
        assert!(parse_git_version("git version 2.37.9").is_err());
    }

    #[tokio::test]
    async fn manages_parallel_worktrees_diff_merge_check_and_squash() {
        check_git_version().await.unwrap();
        let repo_tmp = TempDir::new().unwrap();
        let wt_tmp = TempDir::new().unwrap();
        init_repo(repo_tmp.path());

        let info = inspect_repo(repo_tmp.path()).await.unwrap();
        assert_eq!(info.default_branch, "main");

        let wt1 = wt_tmp.path().join("s1");
        let wt2 = wt_tmp.path().join("s2");
        let wt3 = wt_tmp.path().join("s3");
        create_worktree(&info.root, &wt1, "latch/test/11111111", "main")
            .await
            .unwrap();
        create_worktree(&info.root, &wt2, "latch/test/22222222", "main")
            .await
            .unwrap();
        create_worktree(&info.root, &wt3, "latch/test/33333333", "main")
            .await
            .unwrap();
        assert_eq!(list_worktrees(&info.root).await.unwrap().len(), 4);

        // Sesi 1 mengubah README.md dan menambah file baru tanpa commit
        std::fs::write(wt1.join("README.md"), "line 1 edited by s1\nline 2\n").unwrap();
        std::fs::write(wt1.join("extra.txt"), "hello latch\n").unwrap();

        let diff = worktree_diff(&wt1, "main", None, 65_536).await.unwrap();
        assert!(!diff.truncated);
        assert!(diff
            .files
            .iter()
            .any(|f| f.path == "README.md" && f.status == DiffFileStatus::Modified));
        assert!(diff
            .files
            .iter()
            .any(|f| f.path == "extra.txt" && f.status == DiffFileStatus::Added));

        // Sesi 2 juga mengubah baris yang sama di README.md -> konflik setelah Sesi 1 merge
        std::fs::write(wt2.join("README.md"), "line 1 edited by s2\nline 2\n").unwrap();

        let chk1 = merge_check(&info.root, &wt1, "latch/test/11111111", "main")
            .await
            .unwrap();
        assert!(chk1.clean);

        merge_session(
            &info.root,
            &wt1,
            "latch/test/11111111",
            "main",
            MergeStrategy::Squash,
            Some("feat: apply s1"),
        )
        .await
        .unwrap();
        assert!(!wt1.exists());
        assert!(repo_tmp.path().join("extra.txt").exists());

        // Sekarang Sesi 2 harus terdeteksi konflik tanpa mengubah working tree utama
        let chk2 = merge_check(&info.root, &wt2, "latch/test/22222222", "main")
            .await
            .unwrap();
        assert!(!chk2.clean);
        assert!(chk2.conflicts.contains(&"README.md".to_string()));

        let err = merge_session(
            &info.root,
            &wt2,
            "latch/test/22222222",
            "main",
            MergeStrategy::Squash,
            None,
        )
        .await
        .unwrap_err();
        assert!(matches!(err, GitError::MergeConflict(_)));

        // Cleanup idempoten
        remove_worktree(&info.root, &wt2, "latch/test/22222222")
            .await
            .unwrap();
        remove_worktree(&info.root, &wt2, "latch/test/22222222")
            .await
            .unwrap();
        remove_worktree(&info.root, &wt3, "latch/test/33333333")
            .await
            .unwrap();
    }
}
