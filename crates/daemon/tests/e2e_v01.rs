#![allow(clippy::unwrap_used, clippy::expect_used)]
//! Tes integrasi end-to-end Fase 1 (`T-150`): 3 sesi paralel, diff/merge/discard,
//! isolasi crash `kill -9` (NFR-06), rekonsiliasi restart daemon (T-104), dan tes kanari (NFR-05).

use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use latch_core::config::{AgentProfile, AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::model::{MergeStrategy, ProxyMode, SessionStatus};
use latch_core::protocol::{
    ProjectAddParams, SessionDiffParams, SessionListParams, SessionMergeParams, SessionRunParams,
    UdsClient,
};
use latch_daemon::session_manager::SessionManager;
use latch_daemon::uds_server::serve_uds;
use latch_pty_pool::session::is_pid_alive;
use latch_sandbox::NoopProvider;
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use tempfile::TempDir;
use tokio::sync::Notify;

fn init_test_repo(dir: &Path) {
    let run = |args: &[&str]| {
        let st = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .status()
            .unwrap();
        assert!(st.success());
    };
    run(&["init", "-b", "main"]);
    run(&["config", "user.name", "Latch Test"]);
    run(&["config", "user.email", "test@localhost"]);
    std::fs::write(dir.join("base.txt"), "initial\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "initial"]);
}

fn test_agents_config() -> AgentsConfig {
    let mut profiles = BTreeMap::new();
    profiles.insert(
        "writer-a".to_string(),
        AgentProfile {
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "echo 'from session A' > a.txt && printf '\\033[32mDONE-A\\033[0m\\n'".to_string(),
            ],
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
        "writer-b".to_string(),
        AgentProfile {
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "echo 'from session B' > b.txt && sleep 30".to_string(),
            ],
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
        "sleeper".to_string(),
        AgentProfile {
            command: "sh".to_string(),
            args: vec!["-c".to_string(), "sleep 30".to_string()],
            env: BTreeMap::new(),
            pass_env: Vec::new(),
            proxy_mode: ProxyMode::Bypass,
            proxy_provider: None,
            resume_args: Vec::new(),
            extra_rw: Vec::new(),
            extra_ro: Vec::new(),
        },
    );
    AgentsConfig { profiles }
}

#[tokio::test]
async fn e2e_three_parallel_sessions_crash_isolation_and_restart_reconcile() {
    let tmp = TempDir::new().unwrap();
    let repo_dir = tmp.path().join("repo");
    std::fs::create_dir_all(&repo_dir).unwrap();
    init_test_repo(&repo_dir);

    let paths = RuntimePaths {
        config_dir: tmp.path().join("cfg"),
        state_dir: tmp.path().join("state"),
        socket_path: tmp.path().join("state/latch.sock"),
    };
    let shutdown = Arc::new(Notify::new());
    let mgr = SessionManager::start(
        paths.clone(),
        LatchConfig::default(),
        test_agents_config(),
        Arc::new(NoopProvider),
        Arc::clone(&shutdown),
    )
    .await
    .unwrap();

    let sock_for_srv = paths.socket_path.clone();
    let srv_shutdown = Arc::clone(&shutdown);
    let srv_task = tokio::spawn(async move { serve_uds(&sock_for_srv, mgr, srv_shutdown).await });

    // Tunggu socket siap
    for _ in 0..50 {
        if paths.socket_path.exists() {
            break;
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }

    let client = UdsClient::new(&paths.socket_path);
    let health = client.health().await.unwrap();
    assert_eq!(health.status, "ok");

    let proj = client
        .project_add(&ProjectAddParams {
            repo_path: repo_dir.to_string_lossy().to_string(),
            name: Some("demo".to_string()),
            default_branch: Some("main".to_string()),
        })
        .await
        .unwrap();

    // Jalankan 3 sesi paralel pada satu repo dengan kunci kanari di deskripsi tugas (NFR-05)
    let s1 = client
        .session_run(&SessionRunParams {
            project: proj.name.clone(),
            profile: "writer-a".to_string(),
            task: "task-a sk-test-CANARY11223344".to_string(),
            base_ref: None,
        })
        .await
        .unwrap();
    let s2 = client
        .session_run(&SessionRunParams {
            project: proj.name.clone(),
            profile: "writer-b".to_string(),
            task: "task-b".to_string(),
            base_ref: None,
        })
        .await
        .unwrap();
    let s3 = client
        .session_run(&SessionRunParams {
            project: proj.name.clone(),
            profile: "sleeper".to_string(),
            task: "task-c".to_string(),
            base_ref: None,
        })
        .await
        .unwrap();

    assert_ne!(s1.worktree_path, s2.worktree_path);
    assert_ne!(s2.worktree_path, s3.worktree_path);
    assert!(!s1
        .task
        .as_deref()
        .unwrap_or_default()
        .contains("CANARY11223344"));

    // Tunggu s1 selesai dengan status completed
    let mut s1_done = false;
    for _ in 0..50 {
        let cur = client.session_get(&s1.id).await.unwrap();
        if cur.status == SessionStatus::Completed {
            s1_done = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(s1_done, "session s1 did not complete in time");

    // Diff & merge s1 ke main
    let diff1 = client
        .session_diff(&SessionDiffParams {
            session_id: s1.id.clone(),
            file: None,
            max_bytes: None,
        })
        .await
        .unwrap();
    assert!(diff1.files.iter().any(|f| f.path == "a.txt"));

    let chk1 = client.session_merge_check(&s1.id).await.unwrap();
    assert!(chk1.clean);
    client
        .session_merge(&SessionMergeParams {
            session_id: s1.id.clone(),
            strategy: MergeStrategy::Squash,
            message: Some("feat: add a.txt".to_string()),
        })
        .await
        .unwrap();
    assert!(repo_dir.join("a.txt").exists());

    // Simulasi crash mendadak (kill -9) pada proses anak s2 (NFR-06) -> daemon dan s3 tetap hidup!
    let s2_pid = u32::try_from(s2.pid.unwrap()).unwrap();
    kill(Pid::from_raw(-(s2_pid as i32)), Signal::SIGKILL).unwrap();
    let mut s2_failed = false;
    for _ in 0..50 {
        let cur = client.session_get(&s2.id).await.unwrap();
        if cur.status == SessionStatus::Failed {
            s2_failed = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert!(s2_failed, "killed child s2 must transition to failed");
    assert_eq!(
        client.session_get(&s3.id).await.unwrap().status,
        SessionStatus::Running
    );

    // Pause/Resume s3 lalu matikan daemon (simulasi restart)
    let paused = client.session_pause(&s3.id).await.unwrap();
    assert_eq!(paused.status, SessionStatus::Paused);
    let resumed = client.session_resume(&s3.id).await.unwrap();
    assert_eq!(resumed.status, SessionStatus::Running);

    // Discard s2 sebelum simulasi restart daemon
    client.session_discard(&s2.id).await.unwrap();

    // Hentikan server pertama, lalu matikan proses s3 dari luar untuk menguji rekonsiliasi restart
    shutdown.notify_waiters();
    let _ = srv_task.await;
    let s3_pid = u32::try_from(s3.pid.unwrap()).unwrap();
    let _ = kill(Pid::from_raw(-(s3_pid as i32)), Signal::SIGKILL);
    for _ in 0..30 {
        if !is_pid_alive(s3_pid) {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    // Buat worktree yatim di disk untuk menguji deteksi & pembersihan orphan saat startup
    let orphan_wt = paths.worktrees_dir().join("demo").join("orphan-001");
    Command::new("git")
        .arg("-C")
        .arg(&repo_dir)
        .args([
            "worktree",
            "add",
            "-b",
            "latch/demo/orphan",
            orphan_wt.to_str().unwrap(),
            "main",
        ])
        .status()
        .unwrap();
    assert!(orphan_wt.exists());

    // Start ulang SessionManager -> s3 harus direkonsiliasi menjadi failed & orphan dibersihkan!
    let shutdown2 = Arc::new(Notify::new());
    let mgr2 = SessionManager::start(
        paths.clone(),
        LatchConfig::default(),
        test_agents_config(),
        Arc::new(NoopProvider),
        shutdown2,
    )
    .await
    .unwrap();
    assert!(
        !orphan_wt.exists(),
        "orphan worktree must be cleaned on startup"
    );

    let list = mgr2
        .list_sessions(SessionListParams::default())
        .await
        .unwrap();
    let s3_reconciled = list.iter().find(|s| s.id == s3.id).unwrap();
    assert_eq!(s3_reconciled.status, SessionStatus::Failed);
    assert!(s3_reconciled.pid.is_none());

    // Pindai berkas SQLite mentah terhadap string kanari (NFR-05)
    let db_bytes = std::fs::read(paths.db_file()).unwrap();
    assert!(
        !String::from_utf8_lossy(&db_bytes).contains("CANARY11223344"),
        "canary secret leaked into SQLite database"
    );
}
