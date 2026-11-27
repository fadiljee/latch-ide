#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::sync::Arc;

use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use axum::Router;
use latch_core::config::{AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::model::SessionStatus;
use latch_daemon::http_server::{ensure_or_load_token, rotate_token, start_http_server};
use latch_daemon::notify::{send_webhook, sign_webhook_payload, WebhookPayload};
use latch_daemon::session_manager::SessionManager;
use latch_sandbox::NoopProvider;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::{Mutex, Notify};

#[tokio::test]
async fn http_auth_origin_rate_limit_csp_and_webhook_hmac() {
    let dir = TempDir::new().unwrap();
    let paths = RuntimePaths {
        config_dir: dir.path().join("config"),
        state_dir: dir.path().join("state"),
        socket_path: dir.path().join("latch.sock"),
    };

    let token = ensure_or_load_token(&paths.token_file()).unwrap();
    assert!(token.len() >= 32);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(paths.token_file())
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o600);
    }

    let rotated = rotate_token(&paths.token_file()).unwrap();
    assert_ne!(rotated, token);

    let shutdown = Arc::new(Notify::new());
    let manager = SessionManager::start(
        paths,
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
        rotated.clone(),
        Arc::clone(&shutdown),
    )
    .await
    .unwrap();
    let base = format!("http://{}", server.addr());
    let client = reqwest::Client::new();

    // 1. /api/v1/health tanpa auth -> 200
    let health = client
        .get(format!("{base}/api/v1/health"))
        .send()
        .await
        .unwrap();
    assert_eq!(health.status(), StatusCode::OK);

    // 2. Aset statis `/` punya header keamanan CSP ketat
    let index = client.get(format!("{base}/")).send().await.unwrap();
    assert_eq!(index.status(), StatusCode::OK);
    assert!(index.headers().contains_key("content-security-policy"));
    assert_eq!(
        index
            .headers()
            .get("x-content-type-options")
            .and_then(|v| v.to_str().ok()),
        Some("nosniff")
    );

    // 3. Cross-origin ditolak (403)
    let bad_origin = client
        .get(format!("{base}/api/v1/sessions"))
        .header("origin", "https://evil.example")
        .header("authorization", format!("Bearer {rotated}"))
        .send()
        .await
        .unwrap();
    assert_eq!(bad_origin.status(), StatusCode::FORBIDDEN);

    // 4. Token sah lolos (200)
    let ok_sessions = client
        .get(format!("{base}/api/v1/sessions"))
        .header("authorization", format!("Bearer {rotated}"))
        .send()
        .await
        .unwrap();
    assert_eq!(ok_sessions.status(), StatusCode::OK);

    // 5. 5 kegagalan auth memicu rate limit 429 dan tercatat di audit_log
    for idx in 0..5 {
        let resp = client
            .get(format!("{base}/api/v1/sessions"))
            .header("authorization", "Bearer wrong-token")
            .send()
            .await
            .unwrap();
        if idx < 4 {
            assert_eq!(resp.status(), StatusCode::UNAUTHORIZED);
        } else {
            assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        }
    }
    let audits = manager.list_audit(20).await.unwrap();
    assert!(audits.iter().any(|a| a.kind == "auth_failure"));

    // 6. Webhook HMAC-SHA256 diterima dengan tanda tangan yang valid
    let captured = Arc::new(Mutex::new(Option::<(String, Vec<u8>)>::None));
    let cap_clone = Arc::clone(&captured);
    let hook_app = Router::new().route(
        "/hook",
        post(move |headers: HeaderMap, body: axum::body::Bytes| {
            let cap = Arc::clone(&cap_clone);
            async move {
                let sig = headers
                    .get("x-latch-signature")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                *cap.lock().await = Some((sig, body.to_vec()));
                StatusCode::NO_CONTENT
            }
        }),
    );
    let hook_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let hook_addr = hook_listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(hook_listener, hook_app).await;
    });

    let payload = WebhookPayload {
        event: "session.tripped".to_string(),
        session_id: "ses-hook".to_string(),
        status: SessionStatus::Tripped,
        reason: Some("budget".to_string()),
        cost_usd: 2.05,
        timestamp: "2026-11-27T00:00:00Z".to_string(),
    };
    let status = send_webhook(
        &client,
        &format!("http://{hook_addr}/hook"),
        Some("my-hmac-secret"),
        &payload,
    )
    .await
    .unwrap();
    assert_eq!(status, StatusCode::NO_CONTENT);

    let (sig, raw_body) = captured.lock().await.clone().unwrap();
    assert_eq!(sig, sign_webhook_payload("my-hmac-secret", &raw_body));

    manager.request_shutdown().await;
}
