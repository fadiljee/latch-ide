#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;

use axum::http::StatusCode;
use latch_core::config::{AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::model::TokenScope;
use latch_core::protocol::{ProjectAddParams, SessionRunParams};
use latch_daemon::http_server::{ensure_or_load_token, start_http_server};
use latch_daemon::session_manager::SessionManager;
use latch_sandbox::NoopProvider;
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::Notify;

fn init_git_repo(path: &Path) {
    std::fs::create_dir_all(path).unwrap();
    for args in [
        &["init", "-b", "main"][..],
        &["config", "user.name", "Latch Test"][..],
        &["config", "user.email", "test@latch.local"][..],
    ] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(path)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(path.join("lib.rs"), "pub fn alpha() {}\n").unwrap();
    for args in [&["add", "."][..], &["commit", "-m", "init"][..]] {
        assert!(Command::new("git")
            .args(args)
            .current_dir(path)
            .status()
            .unwrap()
            .success());
    }
}

#[tokio::test]
async fn enforces_host_shell_lock_sandbox_terminal_search_prefs_review_messages_and_workflow_validate(
) {
    let dir = TempDir::new().unwrap();
    let repo_dir = dir.path().join("repo");
    init_git_repo(&repo_dir);

    let paths = RuntimePaths {
        config_dir: dir.path().join("config"),
        state_dir: dir.path().join("state"),
        socket_path: dir.path().join("latch.sock"),
    };
    let owner_token = ensure_or_load_token(&paths.token_file()).unwrap();
    let shutdown = Arc::new(Notify::new());

    let manager = SessionManager::start(
        paths,
        LatchConfig::default(), // ide.allow_host_shell = false bawaan!
        AgentsConfig::with_builtin_defaults(),
        Arc::new(NoopProvider),
        Arc::clone(&shutdown),
    )
    .await
    .unwrap();

    let proj = manager
        .add_project(ProjectAddParams {
            repo_path: repo_dir.to_string_lossy().to_string(),
            name: Some("ide-adv".to_string()),
            default_branch: Some("main".to_string()),
        })
        .await
        .unwrap();

    let ses = manager
        .run_session(SessionRunParams {
            project: proj.name,
            profile: "sleeper".to_string(),
            task: "ide test".to_string(),
            base_ref: None,
            extra_args: Vec::new(),
        })
        .await
        .unwrap();

    let host_shell_tok = manager
        .create_scoped_token(
            "host-try".to_string(),
            vec![TokenScope::Read, TokenScope::ShellHost],
            Some(3600),
        )
        .await
        .unwrap();
    let mobile_tok = manager
        .create_scoped_token(
            "mobile".to_string(),
            vec![TokenScope::Read, TokenScope::Control],
            Some(3600),
        )
        .await
        .unwrap();

    let server = start_http_server(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        manager.clone(),
        owner_token.clone(),
        Arc::clone(&shutdown),
    )
    .await
    .unwrap();
    let base = format!("http://{}", server.addr());
    let owner_auth = format!("Bearer {owner_token}");
    let host_auth = format!("Bearer {}", host_shell_tok.token);
    let mobile_auth = format!("Bearer {}", mobile_tok.token);
    let client = reqwest::Client::new();

    // 1. shell-host tanpa aktivasi di config ditolak 403 host_shell_disabled (IDE_SECURITY §7)
    let host_resp = client
        .post(format!("{base}/api/v1/sessions/{}/terminals", ses.id))
        .header("authorization", &host_auth)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "kind": "host", "cols": 80, "rows": 24 })).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(host_resp.status(), StatusCode::FORBIDDEN);
    let host_body: serde_json::Value =
        serde_json::from_slice(&host_resp.bytes().await.unwrap()).unwrap();
    assert_eq!(host_body["error"]["code"], "host_shell_disabled");

    // 2. Token read+control ditolak 403 saat membuka sandbox terminal (IDE_SECURITY §7)
    let denied_term = client
        .post(format!("{base}/api/v1/sessions/{}/terminals", ses.id))
        .header("authorization", &mobile_auth)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "kind": "sandbox", "cols": 80, "rows": 24 })).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(denied_term.status(), StatusCode::FORBIDDEN);

    // 3. Pemilik lokal membuka sandbox terminal (201) lalu menutupnya (204)
    let ok_term = client
        .post(format!("{base}/api/v1/sessions/{}/terminals", ses.id))
        .header("authorization", &owner_auth)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "kind": "sandbox", "cols": 80, "rows": 24 })).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(ok_term.status(), StatusCode::CREATED);
    let term_json: serde_json::Value =
        serde_json::from_slice(&ok_term.bytes().await.unwrap()).unwrap();
    let tid = term_json["terminal_id"].as_str().unwrap();
    let del_term = client
        .delete(format!("{base}/api/v1/terminals/{tid}"))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(del_term.status(), StatusCode::NO_CONTENT);

    // 4. Search files, search text, prefs, workspaces, LSP servers, & workflow validation
    let sf = client
        .get(format!(
            "{base}/api/v1/sessions/{}/search/files?q=lib",
            ses.id
        ))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(sf.status(), StatusCode::OK);
    let sf_json: serde_json::Value = serde_json::from_slice(&sf.bytes().await.unwrap()).unwrap();
    assert_eq!(sf_json["files"][0]["path"], "lib.rs");

    let st = client
        .post(format!("{base}/api/v1/sessions/{}/search/text", ses.id))
        .header("authorization", &owner_auth)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "query": "alpha" })).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(st.status(), StatusCode::OK);
    let st_json: serde_json::Value = serde_json::from_slice(&st.bytes().await.unwrap()).unwrap();
    assert_eq!(st_json["matches"].as_array().unwrap().len(), 1);

    let prefs_put = client
        .put(format!("{base}/api/v1/prefs"))
        .header("authorization", &owner_auth)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "theme": "bento", "fontSize": 13 })).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(prefs_put.status(), StatusCode::OK);

    let ws_resp = client
        .get(format!("{base}/api/v1/workspaces"))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(ws_resp.status(), StatusCode::OK);

    let lsp_resp = client
        .get(format!("{base}/api/v1/lsp/servers"))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(lsp_resp.status(), StatusCode::OK);

    let wf_invalid = client
        .post(format!("{base}/api/v1/workflows/validate"))
        .header("authorization", &owner_auth)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "workflow_toml": "not valid toml ==" })).unwrap())
        .send()
        .await
        .unwrap();
    let wf_json: serde_json::Value =
        serde_json::from_slice(&wf_invalid.bytes().await.unwrap()).unwrap();
    assert_eq!(wf_json["valid"], false);

    // 5. Antrean pesan + aktivitas + interupsi
    let q_resp = client
        .post(format!("{base}/api/v1/sessions/{}/messages", ses.id))
        .header("authorization", &owner_auth)
        .header("content-type", "application/json")
        .body(
            serde_json::to_vec(&json!({
                "text": "Review lib.rs",
                "attachments": ["lib.rs"],
                "queue_only": true
            }))
            .unwrap(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(q_resp.status(), StatusCode::OK);
    let act_resp = client
        .get(format!("{base}/api/v1/sessions/{}/activity", ses.id))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    let act_json: serde_json::Value =
        serde_json::from_slice(&act_resp.bytes().await.unwrap()).unwrap();
    assert_eq!(act_json["queue"].as_array().unwrap().len(), 1);

    manager.request_shutdown().await;
}
