//! Konfigurasi dan model integrasi MCP Servers, Plugins, dan GitHub CLI (`mcp.toml`, `plugins.toml`).

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

fn default_true() -> bool {
    true
}

/// Kesalahan pemuatan, validasi, atau penyimpanan konfigurasi integrasi.
#[derive(Debug, Error)]
pub enum IntegrationError {
    /// Galat I/O berkas konfigurasi.
    #[error("integration I/O error at {path}: {source}")]
    Io {
        /// Jalur berkas terkait.
        path: String,
        /// Galat I/O sumber.
        #[source]
        source: std::io::Error,
    },
    /// Galat parsing atau serialisasi TOML/JSON.
    #[error("integration format error: {0}")]
    Format(String),
    /// Galat validasi konfigurasi integrasi.
    #[error("integration validation error ({field}): {message}")]
    Validation {
        /// Nama field yang tidak valid.
        field: String,
        /// Penjelasan galat.
        message: String,
    },
}

/// Jenis transport server Model Context Protocol (MCP).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpTransport {
    /// Server MCP lokal berbasis proses stdio.
    #[default]
    Stdio,
    /// Server MCP jarak jauh/lokal berbasis HTTP/SSE.
    Http,
}

/// Definisi satu server MCP pada `mcp.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Pengenal unik server (`[a-zA-Z0-9_-]`).
    pub id: String,
    /// Nama tampilan server MCP.
    pub name: String,
    /// Jenis transport (`stdio` atau `http`).
    #[serde(default)]
    pub transport: McpTransport,
    /// Perintah/biner untuk transport `stdio` (mis. `npx`, `uvx`, `docker`).
    #[serde(default)]
    pub command: Option<String>,
    /// Argumen perintah untuk transport `stdio`.
    #[serde(default)]
    pub args: Vec<String>,
    /// URL endpoint untuk transport `http` (`http://` atau `https://`).
    #[serde(default)]
    pub url: Option<String>,
    /// Daftar NAMA variabel lingkungan yang dibutuhkan server (bukan nilai rahasianya).
    #[serde(default)]
    pub env_keys: Vec<String>,
    /// Menandakan apakah server aktif diteruskan ke sesi agen.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Deskripsi singkat opsional.
    #[serde(default)]
    pub description: Option<String>,
}

/// Kumpulan konfigurasi server MCP (`~/.config/latch/mcp.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct McpConfig {
    /// Peta ID server -> konfigurasi MCP.
    pub servers: BTreeMap<String, McpServerConfig>,
}

/// Kategori plugin Latch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginKind {
    /// Bundel server MCP tambahan.
    #[default]
    McpBundle,
    /// Adaptor CLI agen tambahan.
    AgentAdapter,
    /// Kumpulan template `workflow.toml`.
    WorkflowPack,
    /// Hook otomatisasi lokal.
    Hook,
}

/// Definisi satu plugin pada `plugins.toml`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PluginConfig {
    /// Pengenal unik plugin (`[a-zA-Z0-9_-]`).
    pub id: String,
    /// Nama tampilan plugin.
    pub name: String,
    /// Kategori plugin.
    #[serde(default)]
    pub kind: PluginKind,
    /// Sumber instalasi (URL repo GitHub, paket npm/cargo, atau path lokal).
    #[serde(default)]
    pub source: String,
    /// Perintah/biner opsional yang dijalankan plugin.
    #[serde(default)]
    pub command: Option<String>,
    /// Status aktif/nonaktif plugin.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Deskripsi singkat opsional.
    #[serde(default)]
    pub description: Option<String>,
}

/// Kumpulan konfigurasi plugin (`~/.config/latch/plugins.toml`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct PluginsConfig {
    /// Peta ID plugin -> konfigurasi plugin.
    pub plugins: BTreeMap<String, PluginConfig>,
}

/// Status integrasi GitHub CLI (`gh`) tanpa pernah mengekspos token autentikasi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubStatus {
    /// Apakah biner `gh` ditemukan pada `PATH`.
    pub gh_installed: bool,
    /// Path biner `gh` jika terpasang.
    pub gh_path: Option<String>,
    /// Apakah akun GitHub sudah terautentikasi via `gh auth`.
    pub authenticated: bool,
    /// Host GitHub aktif (bawaan `github.com`).
    pub host: String,
    /// Nama akun GitHub yang sedang aktif (jika terautentikasi).
    pub account: Option<String>,
    /// Protokol operasi git (`https` atau `ssh`).
    pub protocol: Option<String>,
    /// Daftar scope token (hanya nama scope, bukan nilai token).
    pub scopes: Vec<String>,
}

/// Ringkasan satu repositori GitHub milik pengguna dari `gh repo list`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitHubRepoItem {
    /// Nama lengkap `owner/repo`.
    pub name_with_owner: String,
    /// URL repositori.
    pub url: String,
    /// Branch bawaan repositori (jika diketahui).
    pub default_branch: Option<String>,
    /// Menandakan apakah repositori bersifat privat.
    pub is_private: bool,
    /// Deskripsi singkat repositori.
    pub description: Option<String>,
}

/// Memvalidasi identifier konfigurasi (`[a-zA-Z0-9_-]`, panjang 1..=64).
pub fn is_valid_identifier(raw: &str) -> bool {
    let trimmed = raw.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 64
        && trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Memvalidasi nama variabel lingkungan (`[A-Z_][A-Z0-9_]*` atau huruf/angka/`_`).
pub fn is_valid_env_name(raw: &str) -> bool {
    let trimmed = raw.trim();
    !trimmed.is_empty()
        && trimmed.len() <= 128
        && !trimmed.contains('=')
        && trimmed
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

/// Memvalidasi satu entri server MCP sebelum disimpan.
pub fn validate_mcp_server(server: &McpServerConfig) -> Result<(), IntegrationError> {
    if !is_valid_identifier(&server.id) {
        return Err(IntegrationError::Validation {
            field: "id".to_string(),
            message: "id must contain only ASCII letters, digits, '-' or '_'".to_string(),
        });
    }
    if server.name.trim().is_empty() {
        return Err(IntegrationError::Validation {
            field: "name".to_string(),
            message: "name cannot be empty".to_string(),
        });
    }
    match server.transport {
        McpTransport::Stdio => {
            if server
                .command
                .as_deref()
                .is_none_or(|c| c.trim().is_empty())
            {
                return Err(IntegrationError::Validation {
                    field: "command".to_string(),
                    message: "command is required for stdio MCP servers".to_string(),
                });
            }
        }
        McpTransport::Http => {
            let valid_url = server.url.as_deref().is_some_and(|u| {
                let t = u.trim();
                t.starts_with("http://") || t.starts_with("https://")
            });
            if !valid_url {
                return Err(IntegrationError::Validation {
                    field: "url".to_string(),
                    message: "url must start with http:// or https:// for http MCP servers"
                        .to_string(),
                });
            }
        }
    }
    for key in &server.env_keys {
        if !is_valid_env_name(key) {
            return Err(IntegrationError::Validation {
                field: "env_keys".to_string(),
                message: format!(
                    "invalid env var name '{key}' (store variable names only, never raw secret values)"
                ),
            });
        }
    }
    Ok(())
}

/// Memvalidasi satu entri plugin sebelum disimpan.
pub fn validate_plugin(plugin: &PluginConfig) -> Result<(), IntegrationError> {
    if !is_valid_identifier(&plugin.id) {
        return Err(IntegrationError::Validation {
            field: "id".to_string(),
            message: "id must contain only ASCII letters, digits, '-' or '_'".to_string(),
        });
    }
    if plugin.name.trim().is_empty() {
        return Err(IntegrationError::Validation {
            field: "name".to_string(),
            message: "name cannot be empty".to_string(),
        });
    }
    if plugin.source.trim().is_empty() {
        return Err(IntegrationError::Validation {
            field: "source".to_string(),
            message: "source cannot be empty".to_string(),
        });
    }
    Ok(())
}

/// Memuat konfigurasi MCP dari disk (atau kosong bila berkas belum ada).
pub fn load_mcp_config(path: &Path) -> Result<McpConfig, IntegrationError> {
    if !path.exists() {
        return Ok(McpConfig::default());
    }
    let content = fs::read_to_string(path).map_err(|source| IntegrationError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let cfg: McpConfig =
        toml::from_str(&content).map_err(|e| IntegrationError::Format(e.to_string()))?;
    for srv in cfg.servers.values() {
        validate_mcp_server(srv)?;
    }
    Ok(cfg)
}

/// Menyimpan konfigurasi MCP ke disk dengan izin aman (`0600` di Unix).
pub fn save_mcp_config(path: &Path, cfg: &McpConfig) -> Result<(), IntegrationError> {
    for srv in cfg.servers.values() {
        validate_mcp_server(srv)?;
    }
    let serialized =
        toml::to_string_pretty(cfg).map_err(|e| IntegrationError::Format(e.to_string()))?;
    write_secure_file(path, serialized.as_bytes())
}

/// Menulis manifest JSON standar (`mcpServers`) hanya untuk server MCP yang aktif (tanpa nilai rahasia).
pub fn write_enabled_mcp_manifest(path: &Path, cfg: &McpConfig) -> Result<(), IntegrationError> {
    let mut map = serde_json::Map::new();
    for srv in cfg.servers.values().filter(|s| s.enabled) {
        let entry = match srv.transport {
            McpTransport::Stdio => serde_json::json!({
                "transport": "stdio",
                "command": srv.command,
                "args": srv.args,
                "envKeys": srv.env_keys,
            }),
            McpTransport::Http => serde_json::json!({
                "transport": "http",
                "url": srv.url,
                "envKeys": srv.env_keys,
            }),
        };
        map.insert(srv.id.clone(), entry);
    }
    let payload = serde_json::to_vec_pretty(&serde_json::json!({ "mcpServers": map }))
        .map_err(|e| IntegrationError::Format(e.to_string()))?;
    write_secure_file(path, &payload)
}

/// Memuat konfigurasi plugin dari disk (atau kosong bila berkas belum ada).
pub fn load_plugins_config(path: &Path) -> Result<PluginsConfig, IntegrationError> {
    if !path.exists() {
        return Ok(PluginsConfig::default());
    }
    let content = fs::read_to_string(path).map_err(|source| IntegrationError::Io {
        path: path.display().to_string(),
        source,
    })?;
    let cfg: PluginsConfig =
        toml::from_str(&content).map_err(|e| IntegrationError::Format(e.to_string()))?;
    for plugin in cfg.plugins.values() {
        validate_plugin(plugin)?;
    }
    Ok(cfg)
}

/// Menyimpan konfigurasi plugin ke disk dengan izin aman (`0600` di Unix).
pub fn save_plugins_config(path: &Path, cfg: &PluginsConfig) -> Result<(), IntegrationError> {
    for plugin in cfg.plugins.values() {
        validate_plugin(plugin)?;
    }
    let serialized =
        toml::to_string_pretty(cfg).map_err(|e| IntegrationError::Format(e.to_string()))?;
    write_secure_file(path, serialized.as_bytes())
}

/// Mem-parse keluaran `gh auth status` secara aman (mengekstrak akun aktif tanpa pernah membaca token).
pub fn parse_gh_auth_status_output(output: &str, gh_path: Option<String>) -> GitHubStatus {
    let mut host = "github.com".to_string();
    let mut current_account: Option<String> = None;
    let mut active_account: Option<String> = None;
    let mut first_account: Option<String> = None;
    let mut protocol: Option<String> = None;
    let mut scopes: Vec<String> = Vec::new();
    let mut current_active = false;

    for raw_line in output.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }
        if !raw_line.starts_with(' ')
            && !raw_line.starts_with('\t')
            && line.contains('.')
            && !line.contains(' ')
        {
            host = line.to_string();
            continue;
        }
        if let Some(idx) = line.find("Logged in to ") {
            let rest = &line[idx + "Logged in to ".len()..];
            if let Some(acct_idx) = rest.find(" account ") {
                let parsed_host = rest[..acct_idx].trim();
                if !parsed_host.is_empty() {
                    host = parsed_host.to_string();
                }
                let after = rest[acct_idx + " account ".len()..].trim();
                let acct = after
                    .split_whitespace()
                    .next()
                    .unwrap_or("")
                    .trim_end_matches(',')
                    .to_string();
                if !acct.is_empty() {
                    if first_account.is_none() {
                        first_account = Some(acct.clone());
                    }
                    current_account = Some(acct);
                    current_active = false;
                }
            }
            continue;
        }
        if line.contains("Active account: true") {
            current_active = true;
            if let Some(ref acct) = current_account {
                active_account = Some(acct.clone());
            }
            continue;
        }
        if let Some(proto) = line
            .strip_prefix("- Git operations protocol:")
            .or_else(|| line.strip_prefix("Git operations protocol:"))
        {
            if current_active || protocol.is_none() {
                let val = proto.trim();
                if !val.is_empty() {
                    protocol = Some(val.to_string());
                }
            }
            continue;
        }
        if let Some(raw_scopes) = line
            .strip_prefix("- Token scopes:")
            .or_else(|| line.strip_prefix("Token scopes:"))
        {
            if current_active || scopes.is_empty() {
                scopes = raw_scopes
                    .split(',')
                    .map(|s| s.trim().trim_matches('\'').trim_matches('"').to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
        }
    }

    let resolved_account = active_account.or(first_account);
    GitHubStatus {
        gh_installed: gh_path.is_some(),
        gh_path,
        authenticated: resolved_account.is_some(),
        host,
        account: resolved_account,
        protocol,
        scopes,
    }
}

fn write_secure_file(path: &Path, bytes: &[u8]) -> Result<(), IntegrationError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|source| IntegrationError::Io {
            path: parent.display().to_string(),
            source,
        })?;
    }
    let tmp_path = path.with_extension("tmp");
    fs::write(&tmp_path, bytes).map_err(|source| IntegrationError::Io {
        path: tmp_path.display().to_string(),
        source,
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp_path, fs::Permissions::from_mode(0o600)).map_err(|source| {
            IntegrationError::Io {
                path: tmp_path.display().to_string(),
                source,
            }
        })?;
    }
    fs::rename(&tmp_path, path).map_err(|source| IntegrationError::Io {
        path: path.display().to_string(),
        source,
    })
}

#[cfg(test)]
mod tests {
    use tempfile::TempDir;

    use super::{
        load_mcp_config, load_plugins_config, parse_gh_auth_status_output, save_mcp_config,
        save_plugins_config, validate_mcp_server, write_enabled_mcp_manifest, McpConfig,
        McpServerConfig, McpTransport, PluginConfig, PluginKind, PluginsConfig,
    };

    #[test]
    fn roundtrips_mcp_and_plugins_configs_and_rejects_raw_secret_env() {
        let dir = TempDir::new().unwrap();
        let mcp_path = dir.path().join("mcp.toml");
        let manifest_path = dir.path().join("mcp-enabled.json");
        let plugins_path = dir.path().join("plugins.toml");

        let mut mcp = McpConfig::default();
        mcp.servers.insert(
            "github".to_string(),
            McpServerConfig {
                id: "github".to_string(),
                name: "GitHub MCP".to_string(),
                transport: McpTransport::Stdio,
                command: Some("npx".to_string()),
                args: vec![
                    "-y".to_string(),
                    "@modelcontextprotocol/server-github".to_string(),
                ],
                url: None,
                env_keys: vec!["GITHUB_PERSONAL_ACCESS_TOKEN".to_string()],
                enabled: true,
                description: Some("GitHub issues and PR tools".to_string()),
            },
        );
        save_mcp_config(&mcp_path, &mcp).unwrap();
        let loaded_mcp = load_mcp_config(&mcp_path).unwrap();
        assert_eq!(loaded_mcp, mcp);

        write_enabled_mcp_manifest(&manifest_path, &loaded_mcp).unwrap();
        let manifest_text = std::fs::read_to_string(&manifest_path).unwrap();
        assert!(manifest_text.contains("github"));
        assert!(manifest_text.contains("GITHUB_PERSONAL_ACCESS_TOKEN"));

        let bad_srv = McpServerConfig {
            id: "bad".to_string(),
            name: "Bad".to_string(),
            transport: McpTransport::Stdio,
            command: Some("npx".to_string()),
            args: Vec::new(),
            url: None,
            env_keys: vec!["SECRET=ghp_123456".to_string()],
            enabled: true,
            description: None,
        };
        assert!(validate_mcp_server(&bad_srv).is_err());

        let mut plugins = PluginsConfig::default();
        plugins.plugins.insert(
            "playwright-pack".to_string(),
            PluginConfig {
                id: "playwright-pack".to_string(),
                name: "Playwright Browser".to_string(),
                kind: PluginKind::McpBundle,
                source: "npm:@playwright/mcp".to_string(),
                command: Some("npx".to_string()),
                enabled: true,
                description: Some("Browser automation plugin".to_string()),
            },
        );
        save_plugins_config(&plugins_path, &plugins).unwrap();
        let loaded_plugins = load_plugins_config(&plugins_path).unwrap();
        assert_eq!(loaded_plugins, plugins);
    }

    #[test]
    fn parses_gh_auth_status_choosing_active_account_without_token() {
        let sample = r#"
github.com
  ✓ Logged in to github.com account fadiljee (keyring)
  - Active account: true
  - Git operations protocol: https
  - Token: gho_************************************
  - Token scopes: 'gist', 'read:org', 'repo', 'workflow'

  ✓ Logged in to github.com account secondary (keyring)
  - Active account: false
"#;
        let status = parse_gh_auth_status_output(sample, Some("/usr/bin/gh".to_string()));
        assert!(status.gh_installed);
        assert!(status.authenticated);
        assert_eq!(status.host, "github.com");
        assert_eq!(status.account.as_deref(), Some("fadiljee"));
        assert_eq!(status.protocol.as_deref(), Some("https"));
        assert_eq!(status.scopes, vec!["gist", "read:org", "repo", "workflow"]);
    }
}
