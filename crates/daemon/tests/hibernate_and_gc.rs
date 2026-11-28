#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use latch_core::config::{AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::model::{AuditEntry, SessionStatus};
use latch_core::protocol::{ProjectAddParams, SessionRunParams};
use latch_daemon::session_manager::SessionManager;
use latch_sandbox::NoopProvider;
use tempfile::TempDir;
use tokio::sync::Notify;

fn init_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    for args in [
        &["init", "-b", "main"][..],
        &["config", "user.name", "HibTest"],
        &["config", "user.email", "hib@localhost"],
    ] {
        assert!(Command::new("git")
            .current_dir(dir.path())
            .args(args)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(dir.path().join("README.md"), "hibernate\n").unwrap();
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
async fn hibernates_resumes_session_and_runs_retention_gc() {
    let repo = init_repo();
    let dir = TempDir::new().unwrap();
    let paths = RuntimePaths {
        config_dir: dir.path().join("config"),
        state_dir: dir.path().join("state"),
        socket_path: dir.path().join("latch.sock"),
    };

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

    manager
        .add_project(ProjectAddParams {
            repo_path: repo.path().to_string_lossy().to_string(),
            name: Some("hibdemo".to_string()),
            default_branch: Some("main".to_string()),
        })
        .await
        .unwrap();

    // 1. Uji T-410: jalankan sesi -> hibernate -> pastikan PID lepas & status Hibernated -> resume kembali Running
    let ses = manager
        .run_session(SessionRunParams {
            project: "hibdemo".to_string(),
            profile: "sleeper".to_string(),
            task: "idle hibernate test".to_string(),
            base_ref: None,
            extra_args: Vec::new(),
        })
        .await
        .unwrap();
    assert_eq!(ses.status, SessionStatus::Running);

    let hib = manager.hibernate_session(&ses.id).await.unwrap();
    assert_eq!(hib.status, SessionStatus::Hibernated);
    assert!(hib.pid.is_none());

    // Tunggu sebentar untuk memastikan exit watcher tidak menimpa status Hibernated menjadi Failed
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert_eq!(
        manager.get_session(&ses.id).await.unwrap().status,
        SessionStatus::Hibernated
    );

    let resumed = manager.resume_session(&ses.id).await.unwrap();
    assert_eq!(resumed.status, SessionStatus::Running);
    assert!(resumed.pid.is_some());

    // 2. Uji T-411: buat audit lama + direktori tmp yatim + log yatim, uji dry-run lalu GC nyata
    manager
        .record_audit(AuditEntry {
            id: None,
            session_id: None,
            kind: "old_event".to_string(),
            detail: "very old audit entry".to_string(),
            timestamp: 1_000,
        })
        .await
        .unwrap();
    let stale_tmp = paths.tmp_dir().join("stale-session-id");
    std::fs::create_dir_all(&stale_tmp).unwrap();
    std::fs::write(stale_tmp.join("junk.bin"), vec![0u8; 2048]).unwrap();
    let stale_log = paths.logs_dir().join("pty-stale-session-id.log");
    std::fs::write(&stale_log, b"old log").unwrap();

    let dry = manager.run_gc(true).await.unwrap();
    assert!(dry.dry_run);
    assert!(dry.audit_deleted >= 1);
    assert_eq!(dry.tmp_dirs_removed, 1);
    assert_eq!(dry.log_files_removed, 1);
    assert!(stale_tmp.exists());
    assert!(stale_log.exists());

    let applied = manager.run_gc(false).await.unwrap();
    assert!(!applied.dry_run);
    assert!(applied.audit_deleted >= 1);
    assert_eq!(applied.tmp_dirs_removed, 1);
    assert_eq!(applied.log_files_removed, 1);
    assert!(applied.bytes_freed >= 2048);
    assert!(!stale_tmp.exists());
    assert!(!stale_log.exists());

    manager.request_shutdown().await;
}
