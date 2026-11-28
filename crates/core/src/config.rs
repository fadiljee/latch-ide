//! Parser, validasi baris/field, dan resolusi jalur konfigurasi Latch (T-102).

use std::collections::BTreeMap;
use std::fs;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::integrations::{is_valid_env_name, is_valid_identifier};
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

    /// Mengembalikan jalur berkas `pricing.toml`.
    pub fn pricing_file(&self) -> PathBuf {
        self.config_dir.join("pricing.toml")
    }

    /// Mengembalikan jalur berkas `mcp.toml`.
    pub fn mcp_file(&self) -> PathBuf {
        self.config_dir.join("mcp.toml")
    }

    /// Mengembalikan jalur berkas `plugins.toml`.
    pub fn plugins_file(&self) -> PathBuf {
        self.config_dir.join("plugins.toml")
    }

    /// Mengembalikan jalur manifest JSON runtime server MCP aktif.
    pub fn mcp_runtime_manifest_file(&self) -> PathBuf {
        self.state_dir.join("mcp-enabled.json")
    }

    /// Mengembalikan jalur berkas basis data SQLite.
    pub fn db_file(&self) -> PathBuf {
        self.state_dir.join("latch.db")
    }

    /// Mengembalikan jalur berkas bearer token Web/WS (`0600`).
    pub fn token_file(&self) -> PathBuf {
        self.state_dir.join("token")
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

/// Mode autentikasi profil agen (bawaan: OAuth native CLI agen).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentAuthMode {
    /// Autentikasi OAuth native CLI agen (default; berjalan langsung lewat `proxy_mode = "bypass"`).
    #[default]
    #[serde(rename = "oauth", alias = "o_auth")]
    OAuth,
    /// Autentikasi memakai API key (env var / memori sesi daemon; memakai proxy lokal bila provider tersedia).
    ApiKey,
    /// Tanpa autentikasi (mis. shell lokal).
    None,
}

/// Sumber API key terdeteksi (tanpa pernah membocorkan nilai kunci).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiKeySource {
    /// Tersedia melalui variabel lingkungan host.
    Env,
    /// Tersedia sementara di memori RAM daemon (hilang saat daemon berhenti).
    RuntimeMemory,
    /// Belum terkonfigurasi.
    None,
}

/// Satu opsi model yang dilaporkan oleh `agy models`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntigravityModelOption {
    /// ID model untuk diteruskan ke `agy --model <id>`.
    pub id: String,
    /// Label tampilan ramah pengguna.
    pub label: String,
}

/// Informasi runtime Antigravity CLI (`agy`) untuk panel & peluncur IDE.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AntigravityCliInfo {
    /// Apakah biner `agy` terdeteksi pada host.
    pub installed: bool,
    /// Jalur eksekusi `agy` yang ditemukan.
    pub command_path: Option<String>,
    /// Versi Antigravity CLI (`agy --version`).
    pub version: Option<String>,
    /// Menandakan dukungan subcommand `agy acp`.
    pub acp_supported: bool,
    /// Mode eksekusi yang didukung (`--mode`).
    pub modes: Vec<String>,
    /// Tingkat penalaran yang didukung (`--effort`).
    pub efforts: Vec<String>,
    /// Daftar model tersedia dari `agy models`.
    pub models: Vec<AntigravityModelOption>,
}

/// Mem-parse keluaran baris `agy models` (`<id>\t<label>`).
pub fn parse_agy_models_output(raw: &str) -> Vec<AntigravityModelOption> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty()
            || trimmed.starts_with("Fetching")
            || trimmed.starts_with("Warning")
            || trimmed.starts_with("Error")
        {
            continue;
        }
        let (id, label) = if let Some((left, right)) = trimmed.split_once('\t') {
            (left.trim(), right.trim())
        } else if let Some((left, right)) = trimmed.split_once("  ") {
            (left.trim(), right.trim())
        } else {
            continue;
        };
        if !id.is_empty()
            && !id.contains(' ')
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'/'))
        {
            out.push(AntigravityModelOption {
                id: id.to_string(),
                label: if label.is_empty() {
                    id.to_string()
                } else {
                    label.to_string()
                },
            });
        }
    }
    out
}

/// Ringkasan profil agen berikut status deteksi biner dan autentikasi untuk API/UI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProfileStatus {
    /// Nama/ID profil (mis. `antigravity`, `claude`, `codex`).
    pub id: String,
    /// Definisi lengkap profil agen.
    pub profile: AgentProfile,
    /// Apakah perintah/biner agen ditemukan pada `PATH` sistem.
    pub installed: bool,
    /// Jalur biner terdeteksi jika tersedia.
    pub command_path: Option<String>,
    /// Menandakan apakah profil termasuk bawaan Latch.
    pub builtin: bool,
    /// Menandakan apakah API key (sesuai `api_key_env`) sudah tersedia.
    pub api_key_configured: bool,
    /// Asal API key terdeteksi (`env`, `runtime_memory`, atau `none`).
    pub api_key_source: ApiKeySource,
}

fn default_agent_category() -> String {
    "coding_agent".to_string()
}

/// Kumpulan profil agen (`agents.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct AgentsConfig {
    /// Peta nama profil -> definisi eksekusi agen.
    pub profiles: BTreeMap<String, AgentProfile>,
}

impl AgentsConfig {
    /// Profil bawaan komprehensif untuk seluruh agen coding populer (OAuth default) + utilitas lokal.
    pub fn with_builtin_defaults() -> Self {
        let mut profiles = BTreeMap::new();
        profiles.insert(
            "sleeper".to_string(),
            AgentProfile {
                display_name: Some("Sleeper (Test)".to_string()),
                command: "sh".to_string(),
                args: vec!["-c".to_string(), "sleep 3600".to_string()],
                auth_mode: AgentAuthMode::None,
                proxy_mode: ProxyMode::Bypass,
                category: "shell".to_string(),
                description: Some("Background sleep session for testing".to_string()),
                ..AgentProfile::default()
            },
        );
        profiles.insert(
            "shell".to_string(),
            AgentProfile {
                display_name: Some("Local Shell".to_string()),
                command: "sh".to_string(),
                auth_mode: AgentAuthMode::None,
                proxy_mode: ProxyMode::Bypass,
                category: "shell".to_string(),
                description: Some("Interactive POSIX shell inside isolated worktree".to_string()),
                ..AgentProfile::default()
            },
        );

        struct BuiltinSpec<'a> {
            id: &'a str,
            display: &'a str,
            cmd: &'a str,
            args: &'a [&'a str],
            task_arg: Option<&'a str>,
            resume_args: &'a [&'a str],
            provider: Option<&'a str>,
            api_key_env: Option<&'a str>,
            login_cmd: &'a [&'a str],
            extra_rw: &'a [&'a str],
            extra_ro: &'a [&'a str],
            desc: &'a str,
        }

        let specs = [
            BuiltinSpec {
                id: "antigravity",
                display: "Antigravity (agy)",
                cmd: "agy",
                args: &[],
                task_arg: Some("--prompt-interactive"),
                resume_args: &["--continue"],
                provider: Some("google"),
                api_key_env: Some("GEMINI_API_KEY"),
                login_cmd: &["agy"],
                extra_rw: &["~/.gemini", "~/.config/antigravity"],
                extra_ro: &["~/.local/bin"],
                desc: "Google Antigravity coding agent CLI (OAuth default, API key optional)",
            },
            BuiltinSpec {
                id: "claude",
                display: "Claude Code",
                cmd: "claude",
                args: &[],
                task_arg: None,
                resume_args: &["--continue"],
                provider: Some("anthropic"),
                api_key_env: Some("ANTHROPIC_API_KEY"),
                login_cmd: &["claude", "login"],
                extra_rw: &["~/.claude"],
                extra_ro: &[],
                desc: "Anthropic Claude Code CLI (OAuth default, ANTHROPIC_API_KEY optional)",
            },
            BuiltinSpec {
                id: "codex",
                display: "OpenAI Codex",
                cmd: "codex",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("openai"),
                api_key_env: Some("OPENAI_API_KEY"),
                login_cmd: &["codex", "login"],
                extra_rw: &["~/.codex"],
                extra_ro: &[],
                desc: "OpenAI Codex CLI (ChatGPT OAuth default, OPENAI_API_KEY optional)",
            },
            BuiltinSpec {
                id: "gemini",
                display: "Gemini CLI",
                cmd: "gemini",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("google"),
                api_key_env: Some("GEMINI_API_KEY"),
                login_cmd: &["gemini"],
                extra_rw: &["~/.gemini"],
                extra_ro: &[],
                desc: "Google Gemini CLI (Google OAuth default, GEMINI_API_KEY optional)",
            },
            BuiltinSpec {
                id: "opencode",
                display: "OpenCode",
                cmd: "opencode",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("openai"),
                api_key_env: Some("OPENAI_API_KEY"),
                login_cmd: &["opencode", "auth", "login"],
                extra_rw: &["~/.config/opencode", "~/.local/share/opencode"],
                extra_ro: &[],
                desc: "OpenCode multi-provider coding CLI (OAuth or API key)",
            },
            BuiltinSpec {
                id: "aider",
                display: "Aider",
                cmd: "aider",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("openai"),
                api_key_env: Some("OPENAI_API_KEY"),
                login_cmd: &["aider"],
                extra_rw: &[],
                extra_ro: &[],
                desc: "Aider AI pair programmer in terminal",
            },
            BuiltinSpec {
                id: "goose",
                display: "Goose",
                cmd: "goose",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("openai"),
                api_key_env: Some("OPENAI_API_KEY"),
                login_cmd: &["goose", "configure"],
                extra_rw: &["~/.config/goose"],
                extra_ro: &[],
                desc: "Block Goose extensible developer agent with native MCP",
            },
            BuiltinSpec {
                id: "amp",
                display: "Amp",
                cmd: "amp",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("anthropic"),
                api_key_env: Some("AMP_API_KEY"),
                login_cmd: &["amp", "login"],
                extra_rw: &["~/.config/amp"],
                extra_ro: &[],
                desc: "Sourcegraph Amp coding agent CLI",
            },
            BuiltinSpec {
                id: "cline",
                display: "Cline CLI",
                cmd: "cline",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("anthropic"),
                api_key_env: Some("ANTHROPIC_API_KEY"),
                login_cmd: &["cline", "auth"],
                extra_rw: &["~/.cline"],
                extra_ro: &[],
                desc: "Cline autonomous coding agent CLI",
            },
            BuiltinSpec {
                id: "cursor",
                display: "Cursor Agent",
                cmd: "cursor-agent",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("openai"),
                api_key_env: Some("CURSOR_API_KEY"),
                login_cmd: &["cursor-agent", "login"],
                extra_rw: &["~/.cursor"],
                extra_ro: &[],
                desc: "Cursor headless CLI agent",
            },
            BuiltinSpec {
                id: "copilot",
                display: "GitHub Copilot CLI",
                cmd: "gh",
                args: &["copilot"],
                task_arg: None,
                resume_args: &[],
                provider: None,
                api_key_env: Some("GITHUB_TOKEN"),
                login_cmd: &["gh", "auth", "login", "--web"],
                extra_rw: &["~/.config/gh"],
                extra_ro: &[],
                desc: "GitHub Copilot CLI via GitHub OAuth",
            },
            BuiltinSpec {
                id: "kimi",
                display: "Kimi CLI",
                cmd: "kimi",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("openai"),
                api_key_env: Some("MOONSHOT_API_KEY"),
                login_cmd: &["kimi", "login"],
                extra_rw: &["~/.kimi"],
                extra_ro: &[],
                desc: "Moonshot Kimi coding agent CLI",
            },
            BuiltinSpec {
                id: "qwen",
                display: "Qwen Code",
                cmd: "qwen",
                args: &[],
                task_arg: None,
                resume_args: &[],
                provider: Some("openai"),
                api_key_env: Some("DASHSCOPE_API_KEY"),
                login_cmd: &["qwen"],
                extra_rw: &["~/.qwen"],
                extra_ro: &[],
                desc: "Qwen Code terminal coding agent",
            },
        ];

        for spec in specs {
            profiles.insert(
                spec.id.to_string(),
                AgentProfile {
                    display_name: Some(spec.display.to_string()),
                    command: spec.cmd.to_string(),
                    args: spec.args.iter().map(|s| (*s).to_string()).collect(),
                    task_arg: spec.task_arg.map(str::to_string),
                    env: BTreeMap::new(),
                    pass_env: Vec::new(),
                    auth_mode: AgentAuthMode::OAuth,
                    api_key_env: spec.api_key_env.map(str::to_string),
                    login_command: spec.login_cmd.iter().map(|s| (*s).to_string()).collect(),
                    proxy_mode: ProxyMode::Bypass,
                    proxy_provider: spec.provider.map(str::to_string),
                    resume_args: spec.resume_args.iter().map(|s| (*s).to_string()).collect(),
                    extra_rw: spec.extra_rw.iter().map(|s| (*s).to_string()).collect(),
                    extra_ro: spec.extra_ro.iter().map(|s| (*s).to_string()).collect(),
                    category: "coding_agent".to_string(),
                    description: Some(spec.desc.to_string()),
                },
            );
        }
        Self { profiles }
    }

    /// Menentukan apakah nama profil merupakan profil bawaan Latch.
    pub fn is_builtin_profile(name: &str) -> bool {
        Self::with_builtin_defaults().profiles.contains_key(name)
    }
}

/// Definisi satu profil agen CLI di `agents.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Nama tampilan ramah pengguna (opsional).
    #[serde(default)]
    pub display_name: Option<String>,
    /// Perintah/biner yang dijalankan di PTY.
    pub command: String,
    /// Argumen dasar perintah.
    #[serde(default)]
    pub args: Vec<String>,
    /// Flag CLI opsional untuk meneruskan prompt tugas awal saat sesi dibuka (mis. `--prompt-interactive` untuk `agy`).
    #[serde(default)]
    pub task_arg: Option<String>,
    /// Variabel lingkungan statis tambahan (jangan isi API key di sini).
    #[serde(default)]
    pub env: BTreeMap<String, String>,
    /// Nama variabel lingkungan host yang diizinkan diteruskan ke sesi.
    #[serde(default)]
    pub pass_env: Vec<String>,
    /// Mode autentikasi agen (`oauth` [default], `api_key`, atau `none`).
    #[serde(default)]
    pub auth_mode: AgentAuthMode,
    /// Nama variabel lingkungan API key (hanya nama variabel, bukan nilainya).
    #[serde(default)]
    pub api_key_env: Option<String>,
    /// Perintah login interaktif OAuth untuk CLI agen (mis. `["claude", "login"]`).
    #[serde(default)]
    pub login_command: Vec<String>,
    /// Mode injeksi proxy (`bypass` untuk OAuth atau `injected` untuk telemetri API key).
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
    /// Kategori profil (`coding_agent`, `shell`, `custom`).
    #[serde(default = "default_agent_category")]
    pub category: String,
    /// Deskripsi singkat opsional.
    #[serde(default)]
    pub description: Option<String>,
}

impl Default for AgentProfile {
    fn default() -> Self {
        Self {
            display_name: None,
            command: String::new(),
            args: Vec::new(),
            task_arg: None,
            env: BTreeMap::new(),
            pass_env: Vec::new(),
            auth_mode: AgentAuthMode::OAuth,
            api_key_env: None,
            login_command: Vec::new(),
            proxy_mode: ProxyMode::Bypass,
            proxy_provider: None,
            resume_args: Vec::new(),
            extra_rw: Vec::new(),
            extra_ro: Vec::new(),
            category: default_agent_category(),
            description: None,
        }
    }
}

/// Mencari jalur eksekusi perintah pada `PATH` sistem dan direktori biner pengguna umum (`~/.local/bin`).
pub fn resolve_command_in_path(command: &str) -> Option<PathBuf> {
    let trimmed = command.trim();
    if trimmed.is_empty() {
        return None;
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() || trimmed.contains(std::path::MAIN_SEPARATOR) {
        return is_executable_file(&candidate).then_some(candidate);
    }
    if let Some(path_var) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let full = dir.join(trimmed);
            if is_executable_file(&full) {
                return Some(full);
            }
        }
    }
    // Cadangan untuk peluncur GUI/Tauri yang kerap menjalankan daemon tanpa `~/.local/bin` di PATH.
    for fallback_dir in [
        RuntimePaths::expand_tilde("~/.local/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/opt/homebrew/bin"),
    ] {
        let full = fallback_dir.join(trimmed);
        if is_executable_file(&full) {
            return Some(full);
        }
    }
    None
}

fn is_executable_file(path: &Path) -> bool {
    let Ok(meta) = fs::metadata(path) else {
        return false;
    };
    if !meta.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        meta.permissions().mode() & 0o111 != 0
    }
    #[cfg(not(unix))]
    {
        true
    }
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

/// Memuat `agents.toml` dari disk digabung di atas profil bawaan agar profil baru (seperti Antigravity)
/// selalu tersedia sembari mempertahankan seluruh kustomisasi pengguna.
pub fn load_agents_config(path: &Path) -> Result<AgentsConfig, ConfigError> {
    let mut merged = AgentsConfig::with_builtin_defaults();
    if !path.exists() {
        return Ok(merged);
    }
    let content = fs::read_to_string(path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let user_cfg = parse_agents_config_str(&content, &path.display().to_string())?;
    for (name, profile) in user_cfg.profiles {
        merged.profiles.insert(name, profile);
    }
    Ok(merged)
}

/// Menyimpan seluruh konfigurasi profil agen ke `agents.toml` secara aman (`0600`).
pub fn save_agents_config(path: &Path, cfg: &AgentsConfig) -> Result<(), ConfigError> {
    validate_agents_config(cfg, "", &path.display().to_string())?;
    let serialized = toml::to_string_pretty(cfg).map_err(|err| ConfigError::Validation {
        path: path.display().to_string(),
        line: 1,
        field: "profiles".to_string(),
        message: err.to_string(),
    })?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| ConfigError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, serialized).map_err(|source| ConfigError::Io {
        path: tmp_path.display().to_string(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600)).map_err(|source| {
            ConfigError::Io {
                path: tmp_path.display().to_string(),
                source,
            }
        })?;
    }
    fs::rename(&tmp_path, path).map_err(|source| ConfigError::Io {
        path: path.display().to_string(),
        source,
    })
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
        if !is_valid_identifier(name) {
            return Err(ConfigError::Validation {
                path: path_label.to_string(),
                line: find_profile_field_line(src, name, "command"),
                field: format!("profiles.{name}"),
                message: "profile name must contain only ASCII letters, digits, '-' or '_'"
                    .to_string(),
            });
        }
        if profile.command.trim().is_empty() {
            return Err(ConfigError::Validation {
                path: path_label.to_string(),
                line: find_profile_field_line(src, name, "command"),
                field: format!("profiles.{name}.command"),
                message: "command cannot be empty".to_string(),
            });
        }
        if let Some(ref env_key) = profile.api_key_env {
            if !env_key.trim().is_empty() && !is_valid_env_name(env_key) {
                return Err(ConfigError::Validation {
                    path: path_label.to_string(),
                    line: find_profile_field_line(src, name, "api_key_env"),
                    field: format!("profiles.{name}.api_key_env"),
                    message: "api_key_env must be an env var name, never a raw secret value"
                        .to_string(),
                });
            }
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

    #[test]
    fn validates_shipped_example_configs() {
        let cfg = parse_latch_config_str(
            include_str!("../../../examples/config.toml"),
            "examples/config.toml",
        )
        .unwrap();
        assert_eq!(cfg.limits.max_sessions, 4);

        let agents = parse_agents_config_str(
            include_str!("../../../examples/agents.toml"),
            "examples/agents.toml",
        )
        .unwrap();
        assert!(agents.profiles.contains_key("antigravity"));
        assert!(agents.profiles.contains_key("claude"));
        assert!(agents.profiles.contains_key("claude-api"));
        assert!(agents.profiles.contains_key("codex"));
    }

    #[test]
    fn validates_all_builtin_agent_profiles_invariants() {
        let builtins = super::AgentsConfig::with_builtin_defaults();
        assert!(builtins.profiles.len() >= 15);
        let agy = builtins
            .profiles
            .get("antigravity")
            .expect("antigravity builtin must exist");
        assert_eq!(agy.command, "agy");
        assert_eq!(agy.task_arg.as_deref(), Some("--prompt-interactive"));
        assert_eq!(agy.resume_args, vec!["--continue".to_string()]);
        assert!(agy.extra_ro.iter().any(|p| p == "~/.local/bin"));

        let parsed_models = super::parse_agy_models_output(
            "Fetching available models...\ngemini-4-neon-high\tGemini 4 Neon (High)\nclaude-opus-5-5-high\tClaude Opus 5.5 (High)\n",
        );
        assert_eq!(parsed_models.len(), 2);
        assert_eq!(parsed_models[0].id, "gemini-4-neon-high");
        assert_eq!(parsed_models[0].label, "Gemini 4 Neon (High)");
        assert_eq!(parsed_models[1].id, "claude-opus-5-5-high");
        for (id, profile) in &builtins.profiles {
            assert!(!profile.command.trim().is_empty(), "empty command for {id}");
            assert!(
                profile
                    .display_name
                    .as_deref()
                    .is_some_and(|s| !s.is_empty()),
                "missing display_name for {id}"
            );
            assert!(
                profile
                    .description
                    .as_deref()
                    .is_some_and(|s| !s.is_empty()),
                "missing description for {id}"
            );
            if profile.category == "coding_agent" {
                assert_eq!(
                    profile.auth_mode,
                    super::AgentAuthMode::OAuth,
                    "coding agent {id} must default to OAuth"
                );
                assert!(
                    !profile.login_command.is_empty(),
                    "coding agent {id} must define login_command"
                );
            }
        }
    }
}
