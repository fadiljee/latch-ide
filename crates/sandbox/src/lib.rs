#![forbid(unsafe_code)]
//! Provider sandbox untuk mengisolasi proses agen (`NoopProvider` di v0.1, Landlock penuh di v0.3).

use std::path::{Path, PathBuf};
use std::process::Command;

use latch_core::config::{AgentProfile, RuntimePaths, SandboxConfig};
use latch_core::sandbox::{SandboxCapabilities, SandboxError, SandboxPolicy, SandboxProvider};
use tracing::warn;

/// Provider cadangan tanpa isolasi kernel (untuk v0.1 atau OS non-Linux).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProvider;

impl SandboxProvider for NoopProvider {
    fn name(&self) -> &'static str {
        "noop"
    }

    fn capabilities(&self) -> SandboxCapabilities {
        SandboxCapabilities {
            filesystem_isolation: false,
            detail: "noop (unisolated; Landlock enforcement scheduled for v0.3)".to_string(),
        }
    }

    fn prepare(&self, policy: &SandboxPolicy, _cmd: &mut Command) -> Result<(), SandboxError> {
        warn!(
            rw_paths = policy.rw_paths.len(),
            "session running with NoopProvider; process is not filesystem-isolated"
        );
        Ok(())
    }
}

/// Membentuk kebijakan sandbox bawaan untuk satu sesi sesuai konfigurasi dan profil agen.
pub fn build_session_policy(
    worktree_path: &Path,
    session_tmp_dir: &Path,
    sandbox_cfg: &SandboxConfig,
    profile: &AgentProfile,
) -> SandboxPolicy {
    let mut rw_paths = vec![worktree_path.to_path_buf(), session_tmp_dir.to_path_buf()];
    for p in &sandbox_cfg.extra_rw {
        rw_paths.push(RuntimePaths::expand_tilde(p));
    }
    for p in &profile.extra_rw {
        rw_paths.push(RuntimePaths::expand_tilde(p));
    }

    let mut ro_paths = vec![
        PathBuf::from("/usr"),
        PathBuf::from("/lib"),
        PathBuf::from("/lib64"),
        PathBuf::from("/bin"),
        PathBuf::from("/sbin"),
    ];
    for p in &sandbox_cfg.extra_ro {
        ro_paths.push(RuntimePaths::expand_tilde(p));
    }
    for p in &profile.extra_ro {
        ro_paths.push(RuntimePaths::expand_tilde(p));
    }

    let deny_hints = [
        "~/.ssh",
        "~/.aws",
        "~/.gnupg",
        "~/.config/gcloud",
        "~/.kube",
        "~/.docker",
        "~/.netrc",
        "~/.git-credentials",
    ]
    .into_iter()
    .map(RuntimePaths::expand_tilde)
    .collect();

    SandboxPolicy {
        rw_paths,
        ro_paths,
        deny_hints,
        allow_network: Vec::new(),
    }
}

/// Memilih provider sandbox aktif dan memastikan syarat `require_sandbox` terpenuhi.
pub fn select_provider(require_sandbox: bool) -> Result<Box<dyn SandboxProvider>, SandboxError> {
    let provider = NoopProvider;
    if require_sandbox && !provider.capabilities().filesystem_isolation {
        return Err(SandboxError::Unavailable(provider.name().to_string()));
    }
    Ok(Box::new(provider))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::path::Path;
    use std::process::Command;

    use latch_core::config::{AgentProfile, SandboxConfig};
    use latch_core::model::ProxyMode;
    use latch_core::sandbox::SandboxProvider;

    use super::{build_session_policy, select_provider, NoopProvider};

    #[test]
    fn builds_policy_and_respects_require_sandbox_flag() {
        let profile = AgentProfile {
            command: "sh".to_string(),
            args: Vec::new(),
            env: BTreeMap::new(),
            pass_env: Vec::new(),
            proxy_mode: ProxyMode::Bypass,
            proxy_provider: None,
            resume_args: Vec::new(),
            extra_rw: vec!["~/.claude".to_string()],
            extra_ro: vec!["/opt/tool".to_string()],
        };
        let policy = build_session_policy(
            Path::new("/tmp/wt"),
            Path::new("/tmp/ses-tmp"),
            &SandboxConfig::default(),
            &profile,
        );
        assert!(policy
            .rw_paths
            .contains(&Path::new("/tmp/wt").to_path_buf()));
        assert!(policy
            .ro_paths
            .contains(&Path::new("/opt/tool").to_path_buf()));
        assert!(!policy.deny_hints.is_empty());

        let mut cmd = Command::new("true");
        assert!(NoopProvider.prepare(&policy, &mut cmd).is_ok());
        assert!(select_provider(false).is_ok());
        assert!(select_provider(true).is_err());
    }
}
