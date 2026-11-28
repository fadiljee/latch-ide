#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use latch_core::config::{
    AgentAuthMode, AgentProfile, AgentProfileStatus, AgentsConfig, AntigravityCliInfo,
    ApiKeySource, LatchConfig, RuntimePaths,
};
use latch_core::integrations::{
    GitHubStatus, McpServerConfig, McpTransport, PluginConfig, PluginKind,
};
use latch_core::model::ProxyMode;
use latch_daemon::http_server::{ensure_or_load_token, start_http_server};
use latch_daemon::session_manager::SessionManager;
use latch_sandbox::NoopProvider;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::Notify;

async fn parse_json<T: DeserializeOwned>(resp: reqwest::Response) -> T {
    let status = resp.status();
    let bytes = resp.bytes().await.unwrap();
    assert!(
        status.is_success(),
        "expected success status, got {status}: {}",
        String::from_utf8_lossy(&bytes)
    );
    serde_json::from_slice(&bytes).unwrap()
}

fn with_json<T: Serialize>(req: reqwest::RequestBuilder, body: &T) -> reqwest::RequestBuilder {
    req.header("content-type", "application/json")
        .body(serde_json::to_vec(body).unwrap())
}

#[tokio::test]
async fn agents_mcp_plugins_and_github_endpoints_enforce_validation_and_secret_hygiene() {
    let dir = TempDir::new().unwrap();
    let paths = RuntimePaths {
        config_dir: dir.path().join("config"),
        state_dir: dir.path().join("state"),
        socket_path: dir.path().join("latch.sock"),
    };
    let token = ensure_or_load_token(&paths.token_file()).unwrap();
    let shutdown = Arc::new(Notify::new());

    let manager = SessionManager::start(
        paths.clone(),
        LatchConfig::default(),
        AgentsConfig::with_builtin_defaults(),
        Arc::new(NoopProvider),
        Arc::clone(&shutdown),
    )
    .await
    .unwrap();

    let server = start_http_server(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        manager.clone(),
        token.clone(),
        Arc::clone(&shutdown),
    )
    .await
    .unwrap();
    let base = format!("http://{}", server.addr());
    let auth = format!("Bearer {token}");
    let client = reqwest::Client::new();

    // 1. GET /api/v1/agents memuat profil bawaan termasuk antigravity & claude
    let resp = client
        .get(format!("{base}/api/v1/agents"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    let agents: Vec<AgentProfileStatus> = parse_json(resp).await;
    let agy = agents
        .iter()
        .find(|a| a.id == "antigravity" && a.builtin)
        .expect("antigravity builtin agent");
    assert_eq!(agy.profile.command, "agy");
    assert_eq!(
        agy.profile.task_arg.as_deref(),
        Some("--prompt-interactive")
    );
    assert_eq!(agy.profile.resume_args, vec!["--continue".to_string()]);
    assert!(agents.iter().any(|a| a.id == "claude" && a.builtin));

    // 1b. GET /api/v1/agents/antigravity/info mengembalikan metadata Antigravity CLI (modes, efforts, status)
    let resp = client
        .get(format!("{base}/api/v1/agents/antigravity/info"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    let agy_info: AntigravityCliInfo = parse_json(resp).await;
    assert!(agy_info.modes.iter().any(|m| m == "plan"));
    assert!(agy_info.efforts.iter().any(|e| e == "high"));

    // 2. PUT /api/v1/agents/{id} menambah profil custom dan sinkron proxy_mode otomatis
    let custom_profile = AgentProfile {
        display_name: Some("Custom Agent".to_string()),
        command: "sh".to_string(),
        args: vec!["-c".to_string(), "echo hi".to_string()],
        task_arg: None,
        env: BTreeMap::new(),
        pass_env: Vec::new(),
        auth_mode: AgentAuthMode::ApiKey,
        api_key_env: Some("LATCH_TEST_CUSTOM_KEY".to_string()),
        login_command: Vec::new(),
        proxy_mode: ProxyMode::Bypass,
        proxy_provider: Some("openai".to_string()),
        resume_args: Vec::new(),
        extra_rw: Vec::new(),
        extra_ro: Vec::new(),
        category: "custom".to_string(),
        description: Some("Integration test agent".to_string()),
    };
    let resp = with_json(
        client
            .put(format!("{base}/api/v1/agents/custom-test"))
            .header("authorization", &auth),
        &custom_profile,
    )
    .send()
    .await
    .unwrap();
    let upserted: AgentProfileStatus = parse_json(resp).await;
    assert_eq!(upserted.id, "custom-test");
    assert!(!upserted.builtin);
    assert_eq!(upserted.profile.proxy_mode, ProxyMode::Injected);

    // 3. POST /api/v1/agents/{id}/runtime-key menyimpan di RAM saja, tidak pernah ditulis ke disk/SQLite (NFR-05)
    let canary = "sk-canary-never-persist-99887766";
    let resp = with_json(
        client
            .post(format!("{base}/api/v1/agents/custom-test/runtime-key"))
            .header("authorization", &auth),
        &json!({ "api_key": canary }),
    )
    .send()
    .await
    .unwrap();
    let with_key: AgentProfileStatus = parse_json(resp).await;
    assert!(with_key.api_key_configured);
    assert_eq!(with_key.api_key_source, ApiKeySource::RuntimeMemory);

    let agents_toml = std::fs::read_to_string(paths.agents_file()).unwrap();
    assert!(!agents_toml.contains(canary));
    let db_bytes = std::fs::read(paths.db_file()).unwrap();
    assert!(!db_bytes
        .windows(canary.len())
        .any(|w| w == canary.as_bytes()));

    // Hapus runtime key
    let resp = with_json(
        client
            .post(format!("{base}/api/v1/agents/custom-test/runtime-key"))
            .header("authorization", &auth),
        &json!({ "api_key": null }),
    )
    .send()
    .await
    .unwrap();
    let cleared: AgentProfileStatus = parse_json(resp).await;
    assert!(!cleared.api_key_configured);
    assert_eq!(cleared.api_key_source, ApiKeySource::None);

    // 4. DELETE profil custom sukses (204), profil bawaan ditolak (422)
    let del_builtin = client
        .delete(format!("{base}/api/v1/agents/claude"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(del_builtin.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let del_custom = client
        .delete(format!("{base}/api/v1/agents/custom-test"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(del_custom.status(), StatusCode::NO_CONTENT);

    // 5. MCP CRUD + manifest runtime + tolak env key dengan nilai mentah
    let bad_mcp = McpServerConfig {
        id: "bad-mcp".to_string(),
        name: "Bad MCP".to_string(),
        transport: McpTransport::Stdio,
        command: Some("npx".to_string()),
        args: Vec::new(),
        url: None,
        env_keys: vec!["TOKEN=secret_value".to_string()],
        enabled: true,
        description: None,
    };
    let bad_mcp_resp = with_json(
        client
            .put(format!("{base}/api/v1/mcp/bad-mcp"))
            .header("authorization", &auth),
        &bad_mcp,
    )
    .send()
    .await
    .unwrap();
    assert_eq!(bad_mcp_resp.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let good_mcp = McpServerConfig {
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
        description: Some("GitHub MCP tools".to_string()),
    };
    let resp = with_json(
        client
            .put(format!("{base}/api/v1/mcp/github"))
            .header("authorization", &auth),
        &good_mcp,
    )
    .send()
    .await
    .unwrap();
    let saved_mcp: McpServerConfig = parse_json(resp).await;
    assert_eq!(saved_mcp, good_mcp);
    let manifest = std::fs::read_to_string(paths.mcp_runtime_manifest_file()).unwrap();
    assert!(manifest.contains("github"));

    let resp = client
        .get(format!("{base}/api/v1/mcp"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    let mcp_list: Vec<McpServerConfig> = parse_json(resp).await;
    assert_eq!(mcp_list.len(), 1);

    let del_mcp = client
        .delete(format!("{base}/api/v1/mcp/github"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(del_mcp.status(), StatusCode::NO_CONTENT);

    // 6. Plugins CRUD
    let plugin = PluginConfig {
        id: "qa-pack".to_string(),
        name: "QA Pack".to_string(),
        kind: PluginKind::WorkflowPack,
        source: "github:latch-ide/qa-pack".to_string(),
        command: None,
        enabled: true,
        description: Some("Workflow presets".to_string()),
    };
    let resp = with_json(
        client
            .put(format!("{base}/api/v1/plugins/qa-pack"))
            .header("authorization", &auth),
        &plugin,
    )
    .send()
    .await
    .unwrap();
    let saved_plugin: PluginConfig = parse_json(resp).await;
    assert_eq!(saved_plugin, plugin);

    let resp = client
        .get(format!("{base}/api/v1/plugins"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    let plugin_list: Vec<PluginConfig> = parse_json(resp).await;
    assert_eq!(plugin_list.len(), 1);

    let del_plugin = client
        .delete(format!("{base}/api/v1/plugins/qa-pack"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    assert_eq!(del_plugin.status(), StatusCode::NO_CONTENT);

    // 7. GET /api/v1/integrations/github mengembalikan struktur status valid
    let resp = client
        .get(format!("{base}/api/v1/integrations/github"))
        .header("authorization", &auth)
        .send()
        .await
        .unwrap();
    let gh: GitHubStatus = parse_json(resp).await;
    assert!(!gh.host.is_empty());

    manager.request_shutdown().await;
}
