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
use latch_fs_service::{FsReadResponse, FsTreeResponse, FsWriteResponse};
use latch_sandbox::NoopProvider;
use serde::de::DeserializeOwned;
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
        let st = Command::new("git")
            .args(args)
            .current_dir(path)
            .status()
            .unwrap();
        assert!(st.success());
    }
    std::fs::write(path.join("hello.txt"), "initial line\n").unwrap();
    std::fs::write(path.join("index.html"), "<script>alert(1)</script>").unwrap();
    for args in [&["add", "."][..], &["commit", "-m", "initial commit"][..]] {
        let st = Command::new("git")
            .args(args)
            .current_dir(path)
            .status()
            .unwrap();
        assert!(st.success());
    }
}

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

#[tokio::test]
async fn fs_endpoints_enforce_scope_git_protection_symlink_jail_etag_and_canary_hygiene() {
    let dir = TempDir::new().unwrap();
    let repo_dir = dir.path().join("repo");
    init_git_repo(&repo_dir);

    let outside_secret = dir.path().join("ssh_id_rsa");
    std::fs::write(&outside_secret, "PRIVATE_OUTSIDE_SECRET").unwrap();

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

    let proj = manager
        .add_project(ProjectAddParams {
            repo_path: repo_dir.to_string_lossy().to_string(),
            name: Some("fssec".to_string()),
            default_branch: Some("main".to_string()),
        })
        .await
        .unwrap();

    let ses = manager
        .run_session(SessionRunParams {
            project: proj.name,
            profile: "sleeper".to_string(),
            task: "fs security test".to_string(),
            base_ref: None,
            extra_args: Vec::new(),
        })
        .await
        .unwrap();

    // Buat token read+control (tanpa write) sesuai IDE_SECURITY.md §7
    let read_ctrl = manager
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
    let mobile_auth = format!("Bearer {}", read_ctrl.token);
    let client = reqwest::Client::new();

    // 1. GET /fs/tree menyembunyikan .git dan menampilkan hello.txt
    let resp = client
        .get(format!("{base}/api/v1/sessions/{}/fs/tree", ses.id))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    let tree: FsTreeResponse = parse_json(resp).await;
    assert!(!tree.entries.iter().any(|e| e.name == ".git"));
    assert!(tree.entries.iter().any(|e| e.name == "hello.txt"));

    // 2. GET /fs/file mengembalikan isi + ETag
    let resp = client
        .get(format!(
            "{base}/api/v1/sessions/{}/fs/file?path=hello.txt",
            ses.id
        ))
        .header("authorization", &mobile_auth)
        .send()
        .await
        .unwrap();
    let read: FsReadResponse = parse_json(resp).await;
    assert_eq!(read.content.as_deref(), Some("initial line\n"));

    // 3. Token read+control ditolak (403 insufficient_scope) saat PUT /fs/file atau POST /fs/ops
    let denied_put = client
        .put(format!(
            "{base}/api/v1/sessions/{}/fs/file?path=hello.txt",
            ses.id
        ))
        .header("authorization", &mobile_auth)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "content": "nope" })).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(denied_put.status(), StatusCode::FORBIDDEN);

    let denied_ops = client
        .post(format!("{base}/api/v1/sessions/{}/fs/ops", ses.id))
        .header("authorization", &mobile_auth)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "op": "create", "path": "x.txt" })).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(denied_ops.status(), StatusCode::FORBIDDEN);

    // 4. PUT ke .git/config & .git/hooks/x ditolak 403 git_dir_protected (IT2)
    for git_target in [".git/config", ".git/hooks/pre-commit", ".GIT/config"] {
        let resp = client
            .put(format!(
                "{base}/api/v1/sessions/{}/fs/file?path={git_target}",
                ses.id
            ))
            .header("authorization", &owner_auth)
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&json!({ "content": "evil" })).unwrap())
            .send()
            .await
            .unwrap();
        assert_eq!(
            resp.status(),
            StatusCode::FORBIDDEN,
            "expected 403 for {git_target}"
        );
    }

    // 5. Path traversal `../` dan symlink keluar worktree ditolak 400 path_outside_root (IT1)
    let trav = client
        .get(format!(
            "{base}/api/v1/sessions/{}/fs/file?path=../outside",
            ses.id
        ))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(trav.status(), StatusCode::BAD_REQUEST);

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            &outside_secret,
            Path::new(&ses.worktree_path).join("ssh_link"),
        )
        .unwrap();
        let sym_resp = client
            .get(format!(
                "{base}/api/v1/sessions/{}/fs/file?path=ssh_link",
                ses.id
            ))
            .header("authorization", &owner_auth)
            .send()
            .await
            .unwrap();
        assert_eq!(sym_resp.status(), StatusCode::BAD_REQUEST);
    }

    // 6. ETag mismatch -> 409 etag_mismatch dengan current_etag & current_content
    let conflict = client
        .put(format!(
            "{base}/api/v1/sessions/{}/fs/file?path=hello.txt",
            ses.id
        ))
        .header("authorization", &owner_auth)
        .header("if-match", "stale-etag")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "content": "stale write" })).unwrap())
        .send()
        .await
        .unwrap();
    assert_eq!(conflict.status(), StatusCode::CONFLICT);
    let conflict_json: serde_json::Value =
        serde_json::from_slice(&conflict.bytes().await.unwrap()).unwrap();
    assert_eq!(conflict_json["error"]["code"], "etag_mismatch");
    assert_eq!(conflict_json["error"]["details"]["current_etag"], read.etag);

    // 7. PUT valid dengan rahasia kanari di isi file -> tidak pernah muncul di SQLite/audit_log (IDE_SECURITY §7)
    let canary = "sk-fs-canary-never-in-db-77665544";
    let ok_put = client
        .put(format!(
            "{base}/api/v1/sessions/{}/fs/file?path=hello.txt",
            ses.id
        ))
        .header("authorization", &owner_auth)
        .header("if-match", &read.etag)
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&json!({ "content": format!("token={canary}\n") })).unwrap())
        .send()
        .await
        .unwrap();
    let written: FsWriteResponse = parse_json(ok_put).await;
    assert_ne!(written.etag, read.etag);
    assert!(written.undo_id.is_some());

    let db_bytes = std::fs::read(paths.db_file()).unwrap();
    assert!(
        !db_bytes
            .windows(canary.len())
            .any(|w| w == canary.as_bytes()),
        "SECURITY VIOLATION: file content canary leaked into SQLite DB"
    );

    // 8. GET /fs/raw menyajikan HTML sebagai attachment + nosniff (anti-XSS)
    let raw_html = client
        .get(format!(
            "{base}/api/v1/sessions/{}/fs/raw?path=index.html",
            ses.id
        ))
        .header("authorization", &owner_auth)
        .send()
        .await
        .unwrap();
    assert_eq!(raw_html.status(), StatusCode::OK);
    assert_eq!(
        raw_html.headers().get("x-content-type-options").unwrap(),
        "nosniff"
    );
    assert_eq!(
        raw_html.headers().get("content-type").unwrap(),
        "application/octet-stream"
    );
    assert!(raw_html
        .headers()
        .get("content-disposition")
        .unwrap()
        .to_str()
        .unwrap()
        .starts_with("attachment;"));

    manager.request_shutdown().await;
}
