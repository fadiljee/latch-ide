//! Parser, validasi baris/field, dan resolusi jalur konfigurasi Latch (T-102).

use std::collections::BTreeMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::model::ProxyMode;

/// Kesalahan parsing atau validasi konfigurasi beserta konteks field/baris.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// Gagal membaca berkas konfigurasi.
    #[error("failed to read {path}: {source}")]
    Io {
        /// Jalur berkas yang dibaca.
        path: String,
        /// Galat I/O sumber.
        #[source]
        source: std::io::Error,
    },
    /// Galat sintaks atau tipe TOML pada posisi tertentu.
    #[error("{path}:{line}:{column}: invalid TOML ({field}): {message}")]
    Toml {
        /// Nama/jalur berkas.
        path: String,
        /// Nomor baris berbasis 1 (jika diketahui).
        line: usize,
        /// Nomor kolom berbasis 1 (jika diketahui).
        column: usize,
        /// Petunjuk field terkait.
        field: String,
        /// Pesan kesalahan.
        message: String,
    },
    /// Galat validasi semantik konfigurasi.
    #[error("{path}:{line}: validation error on field '{field}': {message}")]
    Validation {
        /// Nama/jalur berkas.
        path: String,
        /// Nomor baris berbasis 1.
        line: usize,
        /// Nama field yang melanggar.
        field: String,
        /// Penjelasan kesalahan.
        message: String,
    },
}

/// Resolusi direktori konfigurasi, state, dan socket XDG Latch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimePaths {
    /// Direktori konfigurasi (`~/.config/latch` atau `LATCH_CONFIG_DIR`).
    pub config_dir: PathBuf,
    /// Direktori state (`~/.local/state/latch` atau `LATCH_STATE_DIR`).
    pub state_dir: PathBuf,
    /// Lokasi Unix Domain Socket (`LATCH_SOCKET` atau `$XDG_RUNTIME_DIR/latch.sock`).
    pub socket_path: PathBuf,
}

impl RuntimePaths {
    /// Memecahkan jalur runtime dari variabel lingkungan atau standar XDG Linux/Unix.
    pub fn resolve() -> Self {
        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from("/tmp"));

        let config_dir = std::env::var_os("LATCH_CONFIG_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("XDG_CONFIG_HOME")
                    .map(|p| PathBuf::from(p).join("latch"))
                    .unwrap_or_else(|| home.join(".config").join("latch"))
            });

        let state_dir = std::env::var_os("LATCH_STATE_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("XDG_STATE_HOME")
                    .map(|p| PathBuf::from(p).join("latch"))
                    .unwrap_or_else(|| home.join(".local").join("state").join("latch"))
            });

        let socket_path = std::env::var_os("LATCH_SOCKET")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                std::env::var_os("XDG_RUNTIME_DIR")
                    .map(|p| PathBuf::from(p).join("latch.sock"))
                    .unwrap_or_else(|| state_dir.join("daemon.sock"))
            });

        Self {
            config_dir,
            state_dir,
            socket_path,
        }
    }

    /// Mengembalikan jalur berkas `config.toml`.
    pub fn config_file(&self) -> PathBuf {
        self.config_dir.join("config.toml")
    }

    /// Mengembalikan jalur berkas `agents.toml`.
    pub fn agents_file(&self) -> PathBuf {
        self.config_dir.join("agents.toml")
    }

    /// Mengembalikan jalur berkas basis data SQLite.
    pub fn db_file(&self) -> PathBuf {
        self.state_dir.join("latch.db")
    }

    /// Mengembalikan direktori log daemon & PTY.
    pub fn logs_dir(&self) -> PathBuf {
        self.state_dir.join("logs")
    }

    /// Mengembalikan direktori worktree sesi.
    pub fn worktrees_dir(&self) -> PathBuf {
        self.state_dir.join("worktrees")
    }

    /// Mengembalikan direktori tmp terisolasi per sesi.
    pub fn tmp_dir(&self) -> PathBuf {
        self.state_dir.join("tmp")
    }

    /// Mengembangkan awalan `~/` jika ada di string konfigurasi.
    pub fn expand_tilde(raw: &str) -> PathBuf {
        if let Some(rest) = raw.strip_prefix("~/") {
            let home = std::env::var_os("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("/tmp"));
            home.join(rest)
        } else {
            PathBuf::from(raw)
        }
    }
}

/// Konfigurasi utama Latch (`config.toml`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct LatchConfig {
    /// Pengaturan daemon & runtime.
    pub daemon: DaemonConfig,
    /// Batas sumber daya sesi & PTY.
    pub limits: LimitsConfig,
    /// Pengaturan sandbox.
    pub sandbox: SandboxConfig,
    /// Pengaturan circuit breaker.
    pub circuit_breaker: CircuitBreakerConfig,
    /// Pengaturan notifikasi.
    pub notifications: NotificationsConfig,
    /// Kebijakan retensi data.
    pub retention: RetentionConfig,
}

/// Bagian `[daemon]` pada `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct DaemonConfig {
    /// Alamat bind HTTP/WS lokal (harus loopback secara bawaan).
    pub http_bind: String,
    /// Jumlah worker thread Tokio (bawaan 2 untuk RSS rendah).
    pub worker_threads: usize,
    /// Opsional penimpaan direktori state.
    pub state_dir: Option<String>,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            http_bind: "127.0.0.1:7878".to_string(),
            worker_threads: 2,
            state_dir: None,
        }
    }
}

/// Bagian `[limits]` pada `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct LimitsConfig {
    /// Maksimum sesi aktif paralel.
    pub max_sessions: usize,
    /// Kapasitas ring buffer output PTY dalam KiB (bawaan 1024 = 1 MiB).
    pub pty_ring_buffer_kib: usize,
    /// Batas ukuran file log rolling PTY dalam MiB.
    pub pty_log_max_mib: usize,
    /// Interval batching frame PTY dalam milidetik (8..=16 ms).
    pub pty_batch_ms: u64,
    /// Jumlah baris scrollback layar virtual `vt100`.
    pub pty_scrollback_lines: usize,
}

impl Default for LimitsConfig {
    fn default() -> Self {
        Self {
            max_sessions: 8,
            pty_ring_buffer_kib: 1024,
            pty_log_max_mib: 50,
            pty_batch_ms: 12,
            pty_scrollback_lines: 500,
        }
    }
}

/// Bagian `[sandbox]` pada `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct SandboxConfig {
    /// Tolak start sesi bila sandbox kernel tidak tersedia.
    pub require_sandbox: bool,
    /// Daftar preset baca bawaan (`system`, `rust`, `node`, `python`, `git`).
    pub presets: Vec<String>,
    /// Jalur tambahan read-only.
    pub extra_ro: Vec<String>,
    /// Jalur tambahan read-write.
    pub extra_rw: Vec<String>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            require_sandbox: false,
            presets: vec![
                "system".to_string(),
                "rust".to_string(),
                "node".to_string(),
                "python".to_string(),
                "git".to_string(),
            ],
            extra_ro: Vec::new(),
            extra_rw: Vec::new(),
        }
    }
}

/// Bagian `[circuit_breaker]` pada `config.toml`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct CircuitBreakerConfig {
    /// Jumlah error beruntun sebelum trip.
    pub max_consecutive_errors: u32,
    /// Batas anggaran biaya per sesi (USD).
    pub max_session_budget_usd: f64,
    /// Ambang waktu idle sebelum hibernasi (menit).
    pub idle_hibernate_minutes: u64,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            max_consecutive_errors: 3,
            max_session_budget_usd: 2.0,
            idle_hibernate_minutes: 15,
        }
    }
}

/// Bagian `[notifications]` pada `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct NotificationsConfig {
    /// Aktifkan notifikasi desktop.
    pub desktop: bool,
    /// URL webhook opsional.
    pub webhook_url: String,
    /// Nama variabel env untuk secret HMAC webhook.
    pub webhook_secret_env: String,
}

impl Default for NotificationsConfig {
    fn default() -> Self {
        Self {
            desktop: true,
            webhook_url: String::new(),
            webhook_secret_env: "LATCH_WEBHOOK_SECRET".to_string(),
        }
    }
}

/// Bagian `[retention]` pada `config.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct RetentionConfig {
    /// Lama simpan telemetri proxy (hari).
    pub telemetry_days: u32,
    /// Lama simpan log audit (hari).
    pub audit_days: u32,
}

impl Default for RetentionConfig {
    fn default() -> Self {
        Self {
            telemetry_days: 30,
            audit_days: 90,
        }
    }
}

/// Kumpulan profil agen (`agents.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgentsConfig {
    /// Peta nama profil -> definisi eksekusi agen.
    pub profiles: BTreeMap<String, AgentProfile>,
}

impl AgentsConfig {
    /// Profil bawaan minimal untuk pengujian/benchmark bila `agents.toml` belum dibuat.
    pub fn with_builtin_defaults() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "sleeper".to_string(),
            AgentProfile {
                command: "sh".to_string(),
                args: vec!["-c".to_string(), "sleep 3600".to_string()],
                env: BTreeMap::new(),
                pass_env: Vec::new(),
                proxy_mode: ProxyMode::Bypass,
                proxy_provider: None,
                resume_args: Vec::new(),
                extra_rw: Vec::new(),
                extra_ro: Vec::new(),
            },
        );
        profiles.insert(
            "shell".to_string(),
            AgentProfile {
                command: "sh".to_string(),
                args: Vec::new(),
                env: BTreeMap::new(),
                pass_env: Vec::new(),
                proxy_mode: ProxyMode::Bypass,
                proxy_provider: None,
                resume_args: Vec::new(),
                extra_rw: Vec::new(),
                extra_ro: Vec::new(),
            },
        );
        Self { profiles }
    }
}

/// Definisi satu profil agen CLI di `agents.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Perintah/biner yang dijalankan di PTY.
    pub command: String,
    /// Argumen dasar perintah.
    #[serde(default)]
    pub args: Vec<String>,
    /// Variabel lingkungan statis tambahan (jangan isi API key di sini).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Nama variabel lingkungan host yang diizinkan diteruskan ke sesi.
    #[serde(default)]
    pub pass_env: Vec<String>,
    /// Mode injeksi proxy (`injected` atau `bypass`).
    #[serde(default)]
    pub proxy_mode: ProxyMode,
    /// Penyedia proxy (`openai`, `anthropic`, `google`) bila `proxy_mode = "injected"`.
    #[serde(default)]
    pub proxy_provider: Option<String>,
    /// Argumen tambahan saat melanjutkan sesi dari hibernasi.
    #[serde(default)]
    pub resume_args: Vec<String>,
    /// Jalur baca-tulis tambahan khusus profil.
    #[serde(default)]
    pub extra_rw: Vec<String>,
    /// Jalur baca saja tambahan khusus profil.
    #[serde(default)]
    pub extra_ro: Vec<String>,
}

/// Memuat dan memvalidasi `config.toml` dari disk, atau mengembalikan default jika tidak ada.
pub fn load_latch_config(path: &Path) -> Result<LatchConfig, ConfigError> {
    if !path.exists() {
        return Ok(LatchConfig::default());
    }
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse_latch_config_str(&content, &path.display().to_string())
}

/// Mem-parse dan memvalidasi isi string `config.toml` dengan pelaporan baris & field.
pub fn parse_latch_config_str(src: &str, path_label: &str) -> Result<LatchConfig, ConfigError> {
    let cfg: LatchConfig =
        toml::from_str(src).map_err(|err| toml_err_to_config(src, path_label, &err))?;
    validate_latch_config(&cfg, src, path_label)?;
    Ok(cfg)
}

/// Memuat dan memvalidasi `agents.toml` dari disk, atau memakai profil bawaan jika belum ada.
pub fn load_agents_config(path: &Path) -> Result<AgentsConfig, ConfigError> {
    if !path.exists() {
        return Ok(AgentsConfig::with_builtin_defaults());
    }
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    parse_agents_config_str(&content, &path.display().to_string())
}

/// Mem-parse dan memvalidasi isi string `agents.toml` dengan pelaporan baris & field.
pub fn parse_agents_config_str(src: &str, path_label: &str) -> Result<AgentsConfig, ConfigError> {
    let cfg: AgentsConfig =
        toml::from_str(src).map_err(|err| toml_err_to_config(src, path_label, &err))?;
    validate_agents_config(&cfg, src, path_label)?;
    Ok(cfg)
}

fn validate_latch_config(
    cfg: &LatchConfig,
    src: &str,
    path_label: &str,
) -> Result<(), ConfigError> {
    if cfg.daemon.http_bind.parse::<SocketAddr>().is_err() {
        return Err(ConfigError::Validation {
            path: path_label.to_string(),
            line: find_field_line(src, "http_bind"),
            field: "daemon.http_bind".to_string(),
            message: format!("invalid socket address '{}'", cfg.daemon.http_bind),
        });
    }
    if cfg.daemon.worker_threads == 0 || cfg.daemon.worker_threads > 64 {
        return Err(ConfigError::Validation {
            path: path_label.to_string(),
            line: find_field_line(src, "worker_threads"),
            field: "daemon.worker_threads".to_string(),
            message: "must be between 1 and 64".to_string(),
        });
    }
    if cfg.limits.max_sessions == 0 {
        return Err(ConfigError::Validation {
            path: path_label.to_string(),
            line: find_field_line(src, "max_sessions"),
            field: "limits.max_sessions".to_string(),
            message: "must be at least 1".to_string(),
        });
    }
    if cfg.limits.pty_ring_buffer_kib == 0 {
        return Err(ConfigError::Validation {
            path: path_label.to_string(),
            line: find_field_line(src, "pty_ring_buffer_kib"),
            field: "limits.pty_ring_buffer_kib".to_string(),
            message: "must be greater than 0".to_string(),
        });
    }
    if !(8..=16).contains(&cfg.limits.pty_batch_ms) {
        return Err(ConfigError::Validation {
            path: path_label.to_string(),
            line: find_field_line(src, "pty_batch_ms"),
            field: "limits.pty_batch_ms".to_string(),
            message: "must be within 8..=16 ms per NFR-03".to_string(),
        });
    }
    if cfg.circuit_breaker.max_session_budget_usd < 0.0
        || cfg.circuit_breaker.max_session_budget_usd.is_nan()
    {
        return Err(ConfigError::Validation {
            path: path_label.to_string(),
            line: find_field_line(src, "max_session_budget_usd"),
            field: "circuit_breaker.max_session_budget_usd".to_string(),
            message: "must be a non-negative number".to_string(),
        });
    }
    Ok(())
}

fn validate_agents_config(
    cfg: &AgentsConfig,
    src: &str,
    path_label: &str,
) -> Result<(), ConfigError> {
    for (name, profile) in &cfg.profiles {
        if profile.command.trim().is_empty() {
            return Err(ConfigError::Validation {
                path: path_label.to_string(),
                line: find_profile_field_line(src, name, "command"),
                field: format!("profiles.{name}.command"),
                message: "command cannot be empty".to_string(),
            });
        }
        if profile.proxy_mode == ProxyMode::Injected
            && profile
                .proxy_provider
                .as_deref()
                .is_none_or(|p| p.trim().is_empty())
        {
            return Err(ConfigError::Validation {
                path: path_label.to_string(),
                line: find_profile_field_line(src, name, "proxy_provider"),
                field: format!("profiles.{name}.proxy_provider"),
                message: "proxy_provider is required when proxy_mode = \"injected\"".to_string(),
            });
        }
    }
    Ok(())
}

fn toml_err_to_config(src: &str, path_label: &str, err: &toml::de::Error) -> ConfigError {
    let (line, column) = err
        .span()
        .map_or((1, 1), |span| offset_to_line_col(src, span.start));
    let field = extract_toml_field_hint(src, line);
    ConfigError::Toml {
        path: path_label.to_string(),
        line,
        column,
        field,
        message: err.message().to_string(),
    }
}

fn offset_to_line_col(src: &str, offset: usize) -> (usize, usize) {
    let clamped = offset.min(src.len());
    let prefix = &src[..clamped];
    let line = prefix.bytes().filter(|&b| b == b'\n').count() + 1;
    let last_nl = prefix.rfind('\n').map_or(0, |pos| pos + 1);
    let col = prefix[last_nl..].chars().count() + 1;
    (line, col)
}

fn extract_toml_field_hint(src: &str, line: usize) -> String {
    src.lines()
        .nth(line.saturating_sub(1))
        .and_then(|l| l.split('=').next())
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .unwrap_or("toml")
        .to_string()
}

fn find_field_line(src: &str, key: &str) -> usize {
    for (idx, line) in src.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if let Some(lhs) = trimmed.split('=').next() {
            if lhs.trim() == key {
                return idx + 1;
            }
        }
    }
    1
}

fn find_profile_field_line(src: &str, profile: &str, key: &str) -> usize {
    let header = format!("[profiles.{profile}]");
    let mut in_section = false;
    let mut header_line = 1usize;
    for (idx, line) in src.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            in_section = trimmed == header;
            if in_section {
                header_line = idx + 1;
            }
            continue;
        }
        if in_section {
            if let Some(lhs) = trimmed.split('=').next() {
                if lhs.trim() == key {
                    return idx + 1;
                }
            }
        }
    }
    header_line
}

#[cfg(test)]
mod tests {
    use super::{parse_agents_config_str, parse_latch_config_str, ConfigError};

    #[test]
    fn parses_valid_latch_and_agents_configs() {
        let cfg = parse_latch_config_str(
            r#"
[daemon]
http_bind = "127.0.0.1:7878"
worker_threads = 2

[limits]
max_sessions = 4
pty_batch_ms = 12
"#,
            "config.toml",
        )
        .unwrap();
        assert_eq!(cfg.limits.max_sessions, 4);

        let agents = parse_agents_config_str(
            r#"
[profiles.coder]
command = "claude"
proxy_mode = "injected"
proxy_provider = "anthropic"
pass_env = ["ANTHROPIC_API_KEY"]
"#,
            "agents.toml",
        )
        .unwrap();
        assert!(agents.profiles.contains_key("coder"));
    }

    #[test]
    fn reports_line_and_field_on_invalid_configs() {
        let err =
            parse_latch_config_str("[limits]\npty_batch_ms = 50\n", "config.toml").unwrap_err();
        assert!(matches!(
            err,
            ConfigError::Validation { line: 2, ref field, .. } if field == "limits.pty_batch_ms"
        ));

        let agent_err = parse_agents_config_str(
            "[profiles.bad]\ncommand = \"\"\nproxy_mode = \"bypass\"\n",
            "agents.toml",
        )
        .unwrap_err();
        assert!(matches!(
            agent_err,
            ConfigError::Validation { line: 2, ref field, .. } if field == "profiles.bad.command"
        ));

        let syntax_err = parse_latch_config_str(
            "[daemon]\nworker_threads = \"not_a_number\"\n",
            "config.toml",
        )
        .unwrap_err();
        assert!(matches!(
            syntax_err,
            ConfigError::Toml { line: 2, ref field, .. } if field == "worker_threads"
        ));
    }
}
