//! Provider sandbox lintas OS: `LandlockProvider` (Linux >= 5.13) dan `NoopProvider` (fallback) (T-302..T-304).

use std::path::{Path, PathBuf};
use std::process::Command;

use latch_core::config::{AgentProfile, RuntimePaths, SandboxConfig};
use latch_core::sandbox::{SandboxCapabilities, SandboxError, SandboxPolicy, SandboxProvider};
use tracing::warn;

/// Laporan dry-run kebijakan sandbox sebelum diterapkan ke proses anak (T-304).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxDryRunReport {
    /// Nama provider aktif.
    pub provider: &'static str,
    /// Apakah isolasi kernel benar-benar aktif.
    pub kernel_enforced: bool,
    /// Jalur baca-tulis efektif (sudah disaring terhadap denylist).
    pub effective_rw: Vec<PathBuf>,
    /// Jalur baca-saja efektif (sudah disaring terhadap denylist).
    pub effective_ro: Vec<PathBuf>,
    /// Jalur sensitif yang diblokir / tidak diizinkan.
    pub denied_paths: Vec<PathBuf>,
}

/// Provider cadangan tanpa isolasi kernel (OS non-Linux atau kernel tanpa Landlock).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopProvider;

impl SandboxProvider for NoopProvider {
    fn name(&self) -> &'static str {
        "noop"
    }

    fn capabilities(&self) -> SandboxCapabilities {
        SandboxCapabilities {
            filesystem_isolation: false,
            detail: "noop (unisolated; Landlock unavailable on this host)".to_string(),
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

/// Provider sandbox Linux Landlock default-deny (`T-303`, `FR-14`).
#[cfg(target_os = "linux")]
#[derive(Debug, Clone, Copy)]
pub struct LandlockProvider {
    abi: landlock::ABI,
}

#[cfg(target_os = "linux")]
impl LandlockProvider {
    /// Mendeteksi dukungan Landlock ABI pada kernel Linux yang sedang berjalan.
    pub fn probe() -> Option<Self> {
        use landlock::{Access, AccessFs, Ruleset, RulesetAttr, ABI};
        for abi in [ABI::V5, ABI::V4, ABI::V3, ABI::V2, ABI::V1] {
            if Ruleset::default()
                .handle_access(AccessFs::from_all(abi))
                .and_then(|r| r.create())
                .is_ok()
            {
                return Some(Self { abi });
            }
        }
        None
    }

    /// Menerapkan aturan Landlock pada thread/proses saat ini (`NO_NEW_PRIVS` otomatis oleh crate `landlock`).
    pub fn apply_current_thread(&self, policy: &SandboxPolicy) -> Result<(), SandboxError> {
        apply_landlock_ruleset(self.abi, policy)
    }
}

#[cfg(target_os = "linux")]
fn apply_landlock_ruleset(abi: landlock::ABI, policy: &SandboxPolicy) -> Result<(), SandboxError> {
    use landlock::{
        path_beneath_rules, Access, AccessFs, Ruleset, RulesetAttr, RulesetCreatedAttr,
        RulesetStatus,
    };

    let (rw_paths, ro_paths) = filter_denied_paths(policy);
    let existing_rw: Vec<&Path> = rw_paths
        .iter()
        .map(PathBuf::as_path)
        .filter(|p| p.exists())
        .collect();
    let existing_ro: Vec<&Path> = ro_paths
        .iter()
        .map(PathBuf::as_path)
        .filter(|p| p.exists())
        .collect();

    let status = Ruleset::default()
        .handle_access(AccessFs::from_all(abi))
        .map_err(|e| SandboxError::ApplyFailed(e.to_string()))?
        .create()
        .map_err(|e| SandboxError::ApplyFailed(e.to_string()))?
        .add_rules(path_beneath_rules(existing_rw, AccessFs::from_all(abi)))
        .map_err(|e| SandboxError::ApplyFailed(e.to_string()))?
        .add_rules(path_beneath_rules(existing_ro, AccessFs::from_read(abi)))
        .map_err(|e| SandboxError::ApplyFailed(e.to_string()))?
        .restrict_self()
        .map_err(|e| SandboxError::ApplyFailed(e.to_string()))?;

    if status.ruleset == RulesetStatus::NotEnforced {
        return Err(SandboxError::Unavailable(
            "landlock ruleset was not enforced by kernel".to_string(),
        ));
    }
    Ok(())
}

#[cfg(target_os = "linux")]
impl SandboxProvider for LandlockProvider {
    fn name(&self) -> &'static str {
        "landlock"
    }

    fn capabilities(&self) -> SandboxCapabilities {
        SandboxCapabilities {
            filesystem_isolation: true,
            detail: format!("landlock-{:?}", self.abi).to_ascii_lowercase(),
        }
    }

    fn prepare(&self, policy: &SandboxPolicy, cmd: &mut Command) -> Result<(), SandboxError> {
        use std::os::unix::process::CommandExt;

        let abi = self.abi;
        let cloned_policy = policy.clone();
        // SAFETY: Closure `pre_exec` hanya memanggil syscall Landlock + prctl(NO_NEW_PRIVS)
        // pada proses anak sebelum `execve`, dan mengembalikan io::Error jika gagal tanpa panic.
        unsafe {
            cmd.pre_exec(move || {
                apply_landlock_ruleset(abi, &cloned_policy)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e))
            });
        }
        Ok(())
    }
}

/// Memfilter `rw_paths` dan `ro_paths` agar jalur sensitif pada `deny_hints` tidak pernah diizinkan.
pub fn filter_denied_paths(policy: &SandboxPolicy) -> (Vec<PathBuf>, Vec<PathBuf>) {
    let is_denied = |candidate: &Path| -> bool {
        policy
            .deny_hints
            .iter()
            .any(|denied| candidate == denied || candidate.starts_with(denied))
    };
    let rw = policy
        .rw_paths
        .iter()
        .filter(|p| !is_denied(p))
        .cloned()
        .collect();
    let ro = policy
        .ro_paths
        .iter()
        .filter(|p| !is_denied(p))
        .cloned()
        .collect();
    (rw, ro)
}

/// Menghasilkan laporan dry-run kebijakan sandbox tanpa menerapkan aturan kernel (`T-304`).
pub fn dry_run_policy(
    provider: &dyn SandboxProvider,
    policy: &SandboxPolicy,
) -> SandboxDryRunReport {
    let (effective_rw, effective_ro) = filter_denied_paths(policy);
    SandboxDryRunReport {
        provider: provider.name(),
        kernel_enforced: provider.capabilities().filesystem_isolation,
        effective_rw,
        effective_ro,
        denied_paths: policy.deny_hints.clone(),
    }
}

/// Mengembalikan daftar jalur baca-saja untuk satu preset toolchain (`T-304`).
pub fn preset_ro_paths(preset: &str) -> Vec<PathBuf> {
    let raw: &[&str] = match preset.trim().to_ascii_lowercase().as_str() {
        "system" => &[
            "/usr",
            "/lib",
            "/lib64",
            "/bin",
            "/sbin",
            "/etc/ssl",
            "/etc/ca-certificates",
            "/etc/resolv.conf",
            "/etc/hosts",
            "/etc/passwd",
            "/etc/group",
            "/dev/null",
            "/dev/zero",
            "/dev/urandom",
            "/proc/self",
        ],
        "rust" => &["~/.cargo", "~/.rustup"],
        "node" => &["~/.nvm", "~/.npm", "~/.local/share/pnpm", "/usr/local"],
        "python" => &["~/.pyenv", "~/.cache/pip", "~/.local"],
        "git" => &["/etc/gitconfig", "~/.gitconfig"],
        _ => &[],
    };
    raw.iter().map(|p| RuntimePaths::expand_tilde(p)).collect()
}

/// Membentuk kebijakan sandbox bawaan untuk satu sesi sesuai konfigurasi, preset toolchain, dan profil agen.
pub fn build_session_policy(
    worktree_path: &Path,
    session_tmp_dir: &Path,
    sandbox_cfg: &SandboxConfig,
    profile: &AgentProfile,
) -> SandboxPolicy {
    let mut rw_paths = vec![
        worktree_path.to_path_buf(),
        session_tmp_dir.to_path_buf(),
        PathBuf::from("/dev/null"),
    ];
    for p in &sandbox_cfg.extra_rw {
        rw_paths.push(RuntimePaths::expand_tilde(p));
    }
    for p in &profile.extra_rw {
        rw_paths.push(RuntimePaths::expand_tilde(p));
    }

    let mut ro_paths = Vec::new();
    for preset in &sandbox_cfg.presets {
        ro_paths.extend(preset_ro_paths(preset));
    }
    if ro_paths.is_empty() {
        ro_paths.extend(preset_ro_paths("system"));
    }
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

/// Memilih provider sandbox terbaik (`LandlockProvider` di Linux >= 5.13, fallback `NoopProvider`).
pub fn select_provider(require_sandbox: bool) -> Result<Box<dyn SandboxProvider>, SandboxError> {
    #[cfg(target_os = "linux")]
    {
        if let Some(landlock) = LandlockProvider::probe() {
            return Ok(Box::new(landlock));
        }
    }

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

    use latch_core::config::{AgentProfile, RuntimePaths, SandboxConfig};
    use latch_core::model::ProxyMode;
    use latch_core::sandbox::SandboxProvider;

    use super::{build_session_policy, dry_run_policy, filter_denied_paths, NoopProvider};

    #[test]
    fn builds_policy_with_presets_and_strips_denied_paths() {
        let profile = AgentProfile {
            command: "sh".to_string(),
            args: Vec::new(),
            env: BTreeMap::new(),
            pass_env: Vec::new(),
            proxy_mode: ProxyMode::Bypass,
            proxy_provider: None,
            resume_args: Vec::new(),
            extra_rw: vec!["~/.ssh".to_string(), "~/.claude".to_string()],
            extra_ro: vec!["/opt/tool".to_string()],
            ..AgentProfile::default()
        };
        let policy = build_session_policy(
            Path::new("/tmp/wt"),
            Path::new("/tmp/ses-tmp"),
            &SandboxConfig::default(),
            &profile,
        );
        let (eff_rw, eff_ro) = filter_denied_paths(&policy);
        assert!(eff_rw.contains(&Path::new("/tmp/wt").to_path_buf()));
        assert!(!eff_rw.contains(&RuntimePaths::expand_tilde("~/.ssh")));
        assert!(eff_ro.contains(&Path::new("/opt/tool").to_path_buf()));
        assert!(eff_ro.contains(&Path::new("/usr").to_path_buf()));

        let report = dry_run_policy(&NoopProvider, &policy);
        assert_eq!(report.provider, "noop");
        assert!(!report.denied_paths.is_empty());

        let mut cmd = Command::new("true");
        assert!(NoopProvider.prepare(&policy, &mut cmd).is_ok());
    }
}
