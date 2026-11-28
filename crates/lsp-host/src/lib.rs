#![forbid(unsafe_code)]
//! Host proses LSP tersandbox, registri `lsp.toml` pengguna, dan filter metode JSON-RPC
//! untuk Agent IDE (`IT-501`, `IT-502`, `IT-503`, `IDE_SECURITY.md` IT8 & §7).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use latch_core::config::{resolve_command_in_path, AgentProfile, SandboxConfig};
use latch_core::sandbox::SandboxProvider;
use latch_pty_pool::PtyError;
use latch_sandbox::{build_session_policy, NoopProvider};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Batas ukuran maksimum satu pesan JSON-RPC LSP (`IDE_API.md` §9: 4 MiB).
pub const MAX_LSP_MESSAGE_BYTES: usize = 4 * 1024 * 1024;
/// Batas memori default proses LSP per bahasa dalam MiB (`IT-501`).
pub const DEFAULT_LSP_MEMORY_LIMIT_MIB: u32 = 512;
/// Waktu idle default sebelum proses LSP ditutup otomatis (detik).
pub const DEFAULT_LSP_IDLE_TIMEOUT_SECS: u64 = 600;

/// Metode LSP berbahaya yang selalu diblokir kecuali diizinkan eksplisit (`IDE_SECURITY.md` IT8 & §7).
const DANGEROUS_LSP_METHODS: &[&str] = &["workspace/executeCommand", "rust-analyzer/runSingle"];

/// Kesalahan subsistem LSP host.
#[derive(Debug, Error)]
pub enum LspHostError {
    /// Server bahasa tidak terdaftar atau binernya tidak terpasang di host.
    #[error("LSP server unavailable for language '{0}'")]
    ServerUnavailable(String),
    /// Metode JSON-RPC berbahaya diblokir oleh kebijakan keamanan (`IDE_SECURITY.md` §7).
    #[error("LSP method '{0}' is blocked by security policy")]
    MethodBlocked(String),
    /// Pesan LSP terlalu besar atau JSON tidak valid.
    #[error("invalid LSP message: {0}")]
    InvalidMessage(String),
    /// Galat proses pembungkus server bahasa.
    #[error("LSP PTY error: {0}")]
    Pty(#[from] PtyError),
    /// Galat sandbox saat menyiapkan proses LSP.
    #[error("LSP sandbox error: {0}")]
    Sandbox(#[from] latch_core::sandbox::SandboxError),
}

/// Konfigurasi satu server LSP di `~/.config/latch/lsp.toml` (`IT-503`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspServerConfig {
    /// Nama bahasa (mis. `rust`, `typescript`, `python`, `go`).
    pub language: String,
    /// Perintah biner server LSP (mis. `rust-analyzer`).
    pub command: String,
    /// Argumen baris perintah server LSP.
    #[serde(default)]
    pub args: Vec<String>,
    /// Batas memori lembut dalam MiB (`IT-501`).
    #[serde(default = "default_memory_mib")]
    pub memory_limit_mib: u32,
    /// Timeout idle dalam detik sebelum server dimatikan.
    #[serde(default = "default_idle_secs")]
    pub idle_timeout_secs: u64,
    /// Apakah `workspace/executeCommand` diizinkan (WAJIB `false` secara bawaan).
    #[serde(default)]
    pub allow_execute_command: bool,
}

const fn default_memory_mib() -> u32 {
    DEFAULT_LSP_MEMORY_LIMIT_MIB
}

const fn default_idle_secs() -> u64 {
    DEFAULT_LSP_IDLE_TIMEOUT_SECS
}

/// Registri server LSP pengguna (`~/.config/latch/lsp.toml`, TIDAK pernah dibaca dari repo tak tepercaya).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LspRegistryConfig {
    /// Peta bahasa -> definisi server LSP.
    pub servers: BTreeMap<String, LspServerConfig>,
}

impl LspRegistryConfig {
    /// Daftar bawaan server LSP umum dengan kebijakan default-safe (`allow_execute_command = false`).
    pub fn with_builtin_defaults() -> Self {
        let mut servers = BTreeMap::new();
        for (lang, cmd, args) in [
            ("rust", "rust-analyzer", &[][..]),
            ("typescript", "typescript-language-server", &["--stdio"][..]),
            ("python", "pyright-langserver", &["--stdio"][..]),
            ("go", "gopls", &["serve"][..]),
        ] {
            servers.insert(
                lang.to_string(),
                LspServerConfig {
                    language: lang.to_string(),
                    command: cmd.to_string(),
                    args: args.iter().map(|s| (*s).to_string()).collect(),
                    memory_limit_mib: DEFAULT_LSP_MEMORY_LIMIT_MIB,
                    idle_timeout_secs: DEFAULT_LSP_IDLE_TIMEOUT_SECS,
                    allow_execute_command: false,
                },
            );
        }
        Self { servers }
    }
}

/// Memuat `~/.config/latch/lsp.toml` digabung di atas registri bawaan (`IT-503`).
pub fn load_lsp_registry(path: &Path) -> LspRegistryConfig {
    let mut merged = LspRegistryConfig::with_builtin_defaults();
    let Ok(raw) = fs::read_to_string(path) else {
        return merged;
    };
    if let Ok(user) = toml::from_str::<LspRegistryConfig>(&raw) {
        for (lang, mut cfg) in user.servers {
            if cfg.language.trim().is_empty() {
                cfg.language = lang.clone();
            }
            merged.servers.insert(lang, cfg);
        }
    }
    merged
}

/// Status server LSP yang dilaporkan `GET /api/v1/lsp/servers` (`IDE_API.md` §9).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LspServerStatus {
    /// Bahasa target.
    pub language: String,
    /// Perintah server LSP.
    pub command: String,
    /// Argumen server LSP.
    pub args: Vec<String>,
    /// Apakah biner terdeteksi pada `PATH`.
    pub installed: bool,
    /// Jalur biner bila terpasang.
    pub command_path: Option<String>,
    /// Batas memori MiB.
    pub memory_limit_mib: u32,
    /// Timeout idle dalam detik.
    pub idle_timeout_secs: u64,
    /// Apakah `workspace/executeCommand` diizinkan.
    pub allow_execute_command: bool,
}

/// Mengembalikan daftar seluruh server LSP beserta status deteksi binernya.
pub fn list_lsp_server_statuses(registry: &LspRegistryConfig) -> Vec<LspServerStatus> {
    registry
        .servers
        .values()
        .map(|cfg| {
            let resolved = resolve_command_in_path(&cfg.command);
            LspServerStatus {
                language: cfg.language.clone(),
                command: cfg.command.clone(),
                args: cfg.args.clone(),
                installed: resolved.is_some(),
                command_path: resolved.map(|p| p.to_string_lossy().to_string()),
                memory_limit_mib: cfg.memory_limit_mib,
                idle_timeout_secs: cfg.idle_timeout_secs,
                allow_execute_command: cfg.allow_execute_command,
            }
        })
        .collect()
}

/// Memeriksa pesan JSON-RPC LSP dari klien WebSocket dan memblokir metode berbahaya (`IT-502`, `IDE_SECURITY.md` §7).
pub fn filter_lsp_client_message(
    raw_json: &str,
    allow_execute_command: bool,
) -> Result<(), LspHostError> {
    if raw_json.len() > MAX_LSP_MESSAGE_BYTES {
        return Err(LspHostError::InvalidMessage(format!(
            "LSP message exceeds {} bytes",
            MAX_LSP_MESSAGE_BYTES
        )));
    }
    let val: serde_json::Value =
        serde_json::from_str(raw_json).map_err(|e| LspHostError::InvalidMessage(e.to_string()))?;
    if let Some(method) = val.get("method").and_then(|m| m.as_str()) {
        if !allow_execute_command && DANGEROUS_LSP_METHODS.contains(&method) {
            return Err(LspHostError::MethodBlocked(method.to_string()));
        }
    }
    Ok(())
}

/// Menyiapkan `std::process::Command` untuk menjalankan server LSP di dalam sandbox worktree (`IT-501`).
pub fn prepare_sandboxed_lsp_command(
    cfg: &LspServerConfig,
    worktree_path: &Path,
    isolated_tmp: &Path,
    sandbox_cfg: &SandboxConfig,
    sandbox: &dyn SandboxProvider,
) -> Result<std::process::Command, LspHostError> {
    let bin = resolve_command_in_path(&cfg.command)
        .ok_or_else(|| LspHostError::ServerUnavailable(cfg.language.clone()))?;
    let mut cmd = std::process::Command::new(bin);
    cmd.args(&cfg.args);
    cmd.current_dir(worktree_path);
    cmd.env("TMPDIR", isolated_tmp);
    let profile = AgentProfile {
        command: cfg.command.clone(),
        args: cfg.args.clone(),
        ..AgentProfile::default()
    };
    let policy = build_session_policy(worktree_path, isolated_tmp, sandbox_cfg, &profile);
    sandbox.prepare(&policy, &mut cmd)?;
    Ok(cmd)
}

/// Menyebutkan provider sandbox default yang dipakai host LSP saat inisialisasi fallback.
pub fn default_lsp_sandbox_name() -> &'static str {
    SandboxProvider::name(&NoopProvider)
}

#[cfg(test)]
mod tests {
    use super::{
        default_lsp_sandbox_name, filter_lsp_client_message, list_lsp_server_statuses,
        LspHostError, LspRegistryConfig,
    };

    #[test]
    fn blocks_workspace_execute_command_by_default_and_lists_builtin_servers() {
        assert_eq!(default_lsp_sandbox_name(), "noop");

        let reg = LspRegistryConfig::with_builtin_defaults();
        let statuses = list_lsp_server_statuses(&reg);
        assert!(statuses
            .iter()
            .any(|s| s.language == "rust" && !s.allow_execute_command));
        assert!(statuses.iter().any(|s| s.language == "typescript"));

        // IDE_SECURITY.md §7: LSP dengan executeCommand wajib diblokir secara default
        let blocked = filter_lsp_client_message(
            r#"{"jsonrpc":"2.0","id":1,"method":"workspace/executeCommand","params":{}}"#,
            false,
        );
        assert!(
            matches!(blocked, Err(LspHostError::MethodBlocked(m)) if m == "workspace/executeCommand")
        );

        // Hover / completion normal tetap diizinkan
        assert!(filter_lsp_client_message(
            r#"{"jsonrpc":"2.0","id":2,"method":"textDocument/hover","params":{}}"#,
            false,
        )
        .is_ok());
    }
}
