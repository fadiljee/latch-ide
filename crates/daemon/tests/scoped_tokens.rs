#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::StatusCode;
use latch_core::config::{AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::model::{CreatedToken, ScopedToken, TokenScope};
use latch_daemon::http_server::{ensure_or_load_token, start_http_server};
use latch_daemon::session_manager::SessionManager;
use latch_sandbox::NoopProvider;
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::Notify;

#[tokio::test]
async fn scoped_tokens_enforce_403_insufficient_scope_hash_only_storage_and_security_headers() {
    let dir = TempDir::new().unwrap();
    let paths = RuntimePaths {
        config_dir: dir.path().join("config"),
        state_dir: dir.path().join("state"),
        socket_path: dir.path().join("latch.sock"),
    };
    let owner_token = ensure_or_load_token(&paths.token_file()).unwrap();
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
        owner_token.clone(),
        Arc::clone(&shutdown),
    )
    .await
    .unwrap();
    let base = format!("http://{}", server.addr());
    let owner_auth = format!("Bearer {owner_token}");
    let client = reqwest::Client::new();

    // 1. Header keamanan workbench lengkap (IT-116, IDE_SECURITY §5)
    let index = client.get(format!("{base}/")).send().await.unwrap();
    assert_eq!(index.status(), StatusCode::OK);
    assert_eq!(
        index
            .headers()
            .get("x-frame-options")
            .and_then(|v| v.to_str().ok()),
        Some("DENY")
    );
    assert_eq!(
        index
            .headers()
            .get("cross-origin-opener-policy")
            .and_then(|v| v.to_str().ok()),
        Some("same-origin")
    );
    assert_eq!(
        index
            .headers()
            .get("cross-origin-resource-policy")
            .and_then(|v| v.to_str().ok()),
        Some("same-origin")
    );

    // 2. Buat token `read`-only via POST /api/v1/tokens (butuh Admin)
    let resp = client
        .post(format!("{base}/api/v1/tokens"))
        .header("authorization", &owner_auth)
        .header("content-type", "application/json")
        .body(
            serde_json::to_vec(&json!({
                "label": "monitor-ro",
                "scopes": ["read"],
                "expires_in_secs": 3600
            }))
            .unwrap(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let ro_created: CreatedToken = serde_json::from_slice(&resp.bytes().await.unwrap()).unwrap();
    assert_eq!(ro_created.metadata.scopes, vec![TokenScope::Read]);
    assert!(ro_created.token.len() >= 32);

    // Token mentah TIDAK BOLEH muncul di SQLite (hanya hash SHA-256)
    let db_bytes = std::fs::read(paths.db_file()).unwrap();
    assert!(
        !db_bytes
            .windows(ro_created.token.len())
            .any(|w| w == ro_created.token.as_bytes()),
        "raw scoped token leaked into SQLite database!"
    );

    let ro_auth = format!("Bearer {}", ro_created.token);

    // 3. Token `read` boleh GET /sessions (200), tapi ditolak 403 insufficient_scope pada Control & Admin
    let get_sessions = client
        .get(format!("{base}/api/v1/sessions"))
        .header("authorization", &ro_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(get_sessions.status(), StatusCode::OK);

    let post_session = client
        .post(format!("{base}/api/v1/sessions"))
        .header("authorization", &ro_auth)
        .header("content-type", "application/json")
        .body(r#"{"project":"x","profile":"shell","task":"hi"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(post_session.status(), StatusCode::FORBIDDEN);
    let err_body: serde_json::Value =
        serde_json::from_slice(&post_session.bytes().await.unwrap()).unwrap();
    assert_eq!(err_body["error"]["code"], "insufficient_scope");

    let post_gc = client
        .post(format!("{base}/api/v1/gc"))
        .header("authorization", &ro_auth)
        .header("content-type", "application/json")
        .body(r#"{"dry_run":true}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(post_gc.status(), StatusCode::FORBIDDEN);

    // 4. Token `read,control` boleh mengakses rute Control, tetapi ditolak 403 pada rute Admin
    let resp = client
        .post(format!("{base}/api/v1/tokens"))
        .header("authorization", &owner_auth)
        .header("content-type", "application/json")
        .body(
            serde_json::to_vec(&json!({
                "label": "iphone-remote",
                "scopes": ["read", "control"]
            }))
            .unwrap(),
        )
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::CREATED);
    let phone_created: CreatedToken = serde_json::from_slice(&resp.bytes().await.unwrap()).unwrap();
    let phone_auth = format!("Bearer {}", phone_created.token);

    let phone_tokens_list = client
        .get(format!("{base}/api/v1/tokens"))
        .header("authorization", &phone_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(phone_tokens_list.status(), StatusCode::FORBIDDEN);

    // 5. GET /api/v1/tokens dengan owner token menampilkan daftar metadata (tanpa nilai mentah)
    let list_resp = client
        .get(format!("{base}/api/v1/tokens"))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(list_resp.status(), StatusCode::OK);
    let listed: Vec<ScopedToken> =
        serde_json::from_slice(&list_resp.bytes().await.unwrap()).unwrap();
    assert_eq!(listed.len(), 2);

    // 6. Revoke token `read`-only -> langsung ditolak 401 unauthorized
    let revoke_resp = client
        .delete(format!("{base}/api/v1/tokens/{}", ro_created.metadata.id))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(revoke_resp.status(), StatusCode::OK);

    let after_revoke = client
        .get(format!("{base}/api/v1/sessions"))
        .header("authorization", &ro_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(after_revoke.status(), StatusCode::UNAUTHORIZED);

    // Audit log mencatat token_created, scope_denied, dan token_revoked
    let audits = manager.list_audit(50).await.unwrap();
    assert!(audits.iter().any(|a| a.kind == "token_created"));
    assert!(audits.iter().any(|a| a.kind == "scope_denied"));
    assert!(audits.iter().any(|a| a.kind == "token_revoked"));

    manager.request_shutdown().await;
}
