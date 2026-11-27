#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use latch_core::config::{AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::model::{ApprovalActionType, ApprovalDecision, SessionStatus};
use latch_core::protocol::{ProjectAddParams, SessionRunParams};
use latch_daemon::http_server::{ensure_or_load_token, start_http_server};
use latch_daemon::session_manager::SessionManager;
use latch_daemon::shim::write_git_push_shim;
use latch_sandbox::NoopProvider;
use tempfile::TempDir;
use tokio::sync::Notify;

fn init_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    for args in [
        &["init", "-b", "main"][..],
        &["config", "user.name", "SecTest"],
        &["config", "user.email", "sec@localhost"],
    ] {
        assert!(Command::new("git")
            .current_dir(dir.path())
            .args(args)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(dir.path().join("README.md"), "sec\n").unwrap();
    assert!(Command::new("git")
        .current_dir(dir.path())
        .args(["add", "."])
        .status()
        .unwrap()
        .success());
    assert!(Command::new("git")
        .current_dir(dir.path())
        .args(["commit", "-m", "init"])
        .status()
        .unwrap()
        .success());
    dir
}

#[tokio::test]
async fn approval_gate_approves_under_500ms_denies_on_timeout_and_blocks_disallowed_egress() {
    let repo = init_repo();
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

    let http = start_http_server(
        SocketAddr::from(([127, 0, 0, 1], 0)),
        manager.clone(),
        token.clone(),
        Arc::clone(&shutdown),
    )
    .await
    .unwrap();
    let base = format!("http://{}", http.addr());
    let client = reqwest::Client::new();

    manager
        .add_project(ProjectAddParams {
            repo_path: repo.path().to_string_lossy().to_string(),
            name: Some("sec".to_string()),
            default_branch: Some("main".to_string()),
        })
        .await
        .unwrap();

    let ses = manager
        .run_session(SessionRunParams {
            project: "sec".to_string(),
            profile: "sleeper".to_string(),
            task: "approval test".to_string(),
            base_ref: None,
        })
        .await
        .unwrap();

    // 1. Buat approval blocking (wait=true) lalu approve lewat REST dan verifikasi latensi < 500 ms
    let mgr_waiter = manager.clone();
    let sid = ses.id.clone();
    let waiter = tokio::spawn(async move {
        mgr_waiter
            .create_approval(
                &sid,
                ApprovalActionType::ShellExec,
                "git push origin latch/sec/1234".to_string(),
                10,
                true,
            )
            .await
            .unwrap()
    });

    tokio::time::sleep(Duration::from_millis(60)).await;
    let pending = manager
        .list_approvals(Some(ApprovalDecision::Pending))
        .await
        .unwrap();
    assert_eq!(pending.len(), 1);
    let approval_id = pending[0].id.clone();

    let t0 = Instant::now();
    let dec_resp = client
        .post(format!("{base}/api/v1/approvals/{approval_id}/decision"))
        .header("authorization", format!("Bearer {token}"))
        .header("content-type", "application/json")
        .body(r#"{"decision":"approved","decided_by":"web:test"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(dec_resp.status(), StatusCode::OK);

    let decided = waiter.await.unwrap();
    assert_eq!(decided.decision, ApprovalDecision::Approved);
    assert!(
        t0.elapsed() < Duration::from_millis(500),
        "approval wakeup latency exceeded 500ms: {:?}",
        t0.elapsed()
    );
    assert_eq!(
        manager.get_session(&ses.id).await.unwrap().status,
        SessionStatus::Running
    );

    // 2. Timeout default deny
    let timed_out = manager
        .create_approval(
            &ses.id,
            ApprovalActionType::NetEgress,
            "https://unknown.example".to_string(),
            1,
            true,
        )
        .await
        .unwrap();
    assert_eq!(timed_out.decision, ApprovalDecision::Denied);
    assert_eq!(timed_out.decided_by.as_deref(), Some("timeout"));

    // 3. Egress domain-allowlist di proxy menolak domain di luar allowlist (403)
    manager
        .proxy_registry()
        .set_allowed_domains(vec!["api.anthropic.com".to_string()])
        .await;
    let tok = manager
        .proxy_registry()
        .register_session(
            ses.id.clone(),
            "openai".to_string(),
            latch_proxy::SessionLimits::default(),
        )
        .await;
    let blocked = client
        .post(format!(
            "{}/s/{tok}/openai/v1/chat/completions",
            manager.proxy_base_url()
        ))
        .send()
        .await
        .unwrap();
    assert_eq!(blocked.status(), StatusCode::FORBIDDEN);

    // 4. Generator PATH shim menghasilkan skrip executable untuk mencegat `git push`
    let shim_path =
        write_git_push_shim(&dir.path().join("shims"), "/usr/bin/git", &base, &ses.id).unwrap();
    assert!(shim_path.exists());

    manager.request_shutdown().await;
}
