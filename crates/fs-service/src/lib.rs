#![forbid(unsafe_code)]
//! Layanan operasi berkas aman di dalam akar worktree untuk Agent IDE (milestone I2).

use std::path::{Component, Path, PathBuf};

use latch_core::redact::redact_secrets;
use thiserror::Error;

/// Kesalahan validasi jalur pada `fs-service`.
#[derive(Debug, Error)]
pub enum FsServiceError {
    /// Jalur mencoba keluar dari akar worktree atau menyentuh `.git`.
    #[error("forbidden path outside worktree or inside .git: {0}")]
    ForbiddenPath(String),
}

/// Validasi awal jalur relatif agar tidak mengandung `..` atau segmen `.git`.
pub fn validate_relative_subpath(root: &Path, relative: &Path) -> Result<PathBuf, FsServiceError> {
    if relative.is_absolute() {
        return Err(FsServiceError::ForbiddenPath(redact_secrets(
            &relative.display().to_string(),
        )));
    }
    for comp in relative.components() {
        match comp {
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(FsServiceError::ForbiddenPath(redact_secrets(
                    &relative.display().to_string(),
                )));
            }
            Component::Normal(seg) if seg == ".git" => {
                return Err(FsServiceError::ForbiddenPath(redact_secrets(
                    &relative.display().to_string(),
                )));
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }
    Ok(root.join(relative))
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::validate_relative_subpath;

    #[test]
    fn rejects_parent_traversal_and_git() {
        let root = Path::new("/tmp/wt");
        assert!(validate_relative_subpath(root, Path::new("../secret")).is_err());
        assert!(validate_relative_subpath(root, Path::new(".git/config")).is_err());
        assert!(validate_relative_subpath(root, Path::new("src/main.rs")).is_ok());
    }
}
