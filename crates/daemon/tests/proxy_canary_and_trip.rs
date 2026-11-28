#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::collections::BTreeMap;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::http::StatusCode;
use axum::response::Response;
use axum::routing::post;
use axum::Router;
use latch_core::config::{AgentProfile, AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::model::{ProxyMode, SessionStatus, TripReason};
use latch_core::protocol::{ProjectAddParams, SessionRunParams};
use latch_daemon::logging::init_logging;
use latch_daemon::session_manager::SessionManager;
use latch_sandbox::NoopProvider;
use tempfile::TempDir;
use tokio::net::TcpListener;
use tokio::sync::Notify;

fn init_temp_git_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    let run = |args: &[&str]| {
        let status = Command::new("git")
            .current_dir(dir.path())
            .args(args)
            .status()
            .unwrap();
        assert!(status.success(), "git {args:?} failed");
    };
    run(&["init", "-b", "main"]);
    run(&["config", "user.name", "Test"]);
    run(&["config", "user.email", "test@localhost"]);
    std::fs::write(dir.path().join("README.md"), "hello\n").unwrap();
    run(&["add", "."]);
    run(&["commit", "-m", "initial"]);
    dir
}

#[tokio::test]
async fn secret_canary_never_leaks_to_sqlite_or_logs_and_breaker_trips_session() {
    let canary = "LATCHCANARY9876543210XYZ";
    let repo = init_temp_git_repo();
    let state_root = TempDir::new().unwrap();
    let paths = RuntimePaths {
        config_dir: state_root.path().join("config"),
        state_dir: state_root.path().join("state"),
        socket_path: state_root.path().join("latch.sock"),
    };
    std::fs::create_dir_all(paths.logs_dir()).unwrap();
    let _log_guard = init_logging(&paths.logs_dir()).unwrap();

    // Mock upstream yang mengembalikan error berulang dan menyisipkan string kanari pada body error
    let canary_owned = canary.to_string();
    let mock_app = Router::new().route(
        "/v1/chat/completions",
        post(move || {
            let c = canary_owned.clone();
            async move {
                Response::builder()
                    .status(StatusCode::TOO_MANY_REQUESTS)
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"error":{{"type":"rate_limit_error","code":"limit","message":"leaked sk-{c}"}}}}"#
                    )))
                    .unwrap()
            }
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, mock_app).await;
    });

    let mut cfg = LatchConfig::default();
    cfg.circuit_breaker.max_consecutive_errors = 3;
    cfg.circuit_breaker.max_session_budget_usd = 5.0;

    let mut profiles = BTreeMap::new();
    // Profil injected yang mencetak OPENAI_BASE_URL lalu tidur sampai dihentikan circuit breaker
    profiles.insert(
        "injected-agent".to_string(),
        AgentProfile {
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "printf 'PROXY_URL=%s\\n' \"$OPENAI_BASE_URL\"; sleep 30".to_string(),
            ],
            env: BTreeMap::new(),
            pass_env: Vec::new(),
            proxy_mode: ProxyMode::Injected,
            proxy_provider: Some("openai".to_string()),
            resume_args: Vec::new(),
            extra_rw: Vec::new(),
            extra_ro: Vec::new(),
            ..AgentProfile::default()
        },
    );
    // Profil bypass yang membuktikan OPENAI_BASE_URL TIDAK disuntikkan
    profiles.insert(
        "bypass-agent".to_string(),
        AgentProfile {
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "printf 'BYPASS_URL=%s\\n' \"${OPENAI_BASE_URL:-UNSET}\"; sleep 0.2".to_string(),
            ],
            env: BTreeMap::new(),
            pass_env: Vec::new(),
            proxy_mode: ProxyMode::Bypass,
            proxy_provider: None,
            resume_args: Vec::new(),
            extra_rw: Vec::new(),
            extra_ro: Vec::new(),
            ..AgentProfile::default()
        },
    );

    let shutdown = Arc::new(Notify::new());
    let manager = SessionManager::start(
        paths.clone(),
        cfg,
        AgentsConfig { profiles },
        Arc::new(NoopProvider),
        shutdown,
    )
    .await
    .unwrap();

    manager
        .proxy_registry()
        .set_upstream("openai", format!("http://{mock_addr}"))
        .await;

    manager
        .add_project(ProjectAddParams {
            repo_path: repo.path().to_string_lossy().to_string(),
            name: Some("demo".to_string()),
            default_branch: Some("main".to_string()),
        })
        .await
        .unwrap();

    // 1. Uji mode bypass tidak menyuntikkan OPENAI_BASE_URL
    let bypass_ses = manager
        .run_session(SessionRunParams {
            project: "demo".to_string(),
            profile: "bypass-agent".to_string(),
            task: "verify bypass".to_string(),
            base_ref: None,
        })
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(350)).await;
    let bypass_log =
        std::fs::read_to_string(paths.logs_dir().join(format!("pty-{}.log", bypass_ses.id)))
            .unwrap_or_default();
    assert!(
        bypass_log.contains("BYPASS_URL=UNSET"),
        "bypass mode should not inject proxy env, log was: {bypass_log}"
    );

    // 2. Uji mode injected menyuntikkan OPENAI_BASE_URL, mengirim kanari, dan circuit breaker trip < 1s
    let injected_ses = manager
        .run_session(SessionRunParams {
            project: "demo".to_string(),
            profile: "injected-agent".to_string(),
            task: format!("test Bearer sk-{canary}"),
            base_ref: None,
        })
        .await
        .unwrap();

    // Tunggu sampai PTY mencetak OPENAI_BASE_URL ke snapshot
    let mut injected_url = None;
    for _ in 0..40 {
        if let Ok(snap) = manager.screen_snapshot(&injected_ses.id).await {
            if let Some(line) = snap.text.lines().find(|l| l.contains("PROXY_URL=")) {
                if let Some(val) = line.split("PROXY_URL=").nth(1) {
                    let trimmed = val.trim();
                    if trimmed.starts_with("http://127.0.0.1:") {
                        injected_url = Some(trimmed.to_string());
                        break;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let proxy_url = injected_url.expect("OPENAI_BASE_URL was not injected into PTY");

    // Kirim 3 request error berturut-turut membawa rahasia kanari di Authorization & x-api-key
    let client = reqwest::Client::new();
    let trip_timer = Instant::now();
    for _ in 0..3 {
        let resp = client
            .post(format!("{proxy_url}/chat/completions"))
            .header("authorization", format!("Bearer sk-{canary}"))
            .header("x-api-key", format!("sk-ant-{canary}"))
            .header("content-type", "application/json")
            .body(format!(
                r#"{{"model":"gpt-4o","messages":[{{"role":"user","content":"secret {canary}"}}]}}"#
            ))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), StatusCode::TOO_MANY_REQUESTS);
        let _ = resp.bytes().await.unwrap();
    }

    // Pastikan sesi berpindah ke Tripped dengan TripReason::ErrorLoop dalam < 1 detik
    let mut tripped = false;
    while trip_timer.elapsed() < Duration::from_secs(2) {
        let current = manager.get_session(&injected_ses.id).await.unwrap();
        if current.status == SessionStatus::Tripped {
            assert_eq!(current.trip_reason, Some(TripReason::ErrorLoop));
            tripped = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
    }
    assert!(tripped, "session did not trip after 3 consecutive errors");

    // Tunggu flush batch telemetri (200 ms) lalu verifikasi tersimpan di SQLite
    tokio::time::sleep(Duration::from_millis(350)).await;
    let summary = manager
        .session_telemetry_summary(&injected_ses.id)
        .await
        .unwrap();
    assert_eq!(summary.request_count, 3);
    assert_eq!(summary.error_count, 3);

    // T-210: Pindai seluruh berkas SQLite mentah dan seluruh log daemon untuk memastikan kanari NIHIL!
    let db_bytes = std::fs::read(paths.db_file()).unwrap();
    let db_lossy = String::from_utf8_lossy(&db_bytes);
    assert!(
        !db_lossy.contains(canary),
        "SECURITY VIOLATION (NFR-05): canary secret found inside SQLite database!"
    );

    for entry in std::fs::read_dir(paths.logs_dir()).unwrap() {
        let path = entry.unwrap().path();
        if path.is_file() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            assert!(
                !content.contains(canary),
                "SECURITY VIOLATION (NFR-05): canary secret found in log file {}",
                path.display()
            );
        }
    }

    manager.request_shutdown().await;
}

#[tokio::test]
async fn budget_circuit_breaker_trips_session_under_one_second() {
    let repo = init_temp_git_repo();
    let state_root = TempDir::new().unwrap();
    let paths = RuntimePaths {
        config_dir: state_root.path().join("config"),
        state_dir: state_root.path().join("state"),
        socket_path: state_root.path().join("latch.sock"),
    };

    // Mock upstream mengembalikan usage besar (gpt-4o: 200k input + 100k output = $1.50 per call)
    let mock_app = Router::new().route(
        "/v1/chat/completions",
        post(|| async {
            Response::builder()
                .status(StatusCode::OK)
                .header("content-type", "application/json")
                .body(Body::from(
                    r#"{"id":"c-budget","model":"gpt-4o","choices":[],"usage":{"prompt_tokens":200000,"completion_tokens":100000}}"#,
                ))
                .unwrap()
        }),
    );
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let mock_addr = listener.local_addr().unwrap();
    tokio::spawn(async move {
        let _ = axum::serve(listener, mock_app).await;
    });

    let mut cfg = LatchConfig::default();
    cfg.circuit_breaker.max_session_budget_usd = 1.0;

    let mut profiles = BTreeMap::new();
    profiles.insert(
        "budget-agent".to_string(),
        AgentProfile {
            command: "sh".to_string(),
            args: vec![
                "-c".to_string(),
                "printf 'PROXY_URL=%s\\n' \"$OPENAI_BASE_URL\"; sleep 30".to_string(),
            ],
            env: BTreeMap::new(),
            pass_env: Vec::new(),
            proxy_mode: ProxyMode::Injected,
            proxy_provider: Some("openai".to_string()),
            resume_args: Vec::new(),
            extra_rw: Vec::new(),
            extra_ro: Vec::new(),
            ..AgentProfile::default()
        },
    );

    let shutdown = Arc::new(Notify::new());
    let manager = SessionManager::start(
        paths,
        cfg,
        AgentsConfig { profiles },
        Arc::new(NoopProvider),
        shutdown,
    )
    .await
    .unwrap();
    manager
        .proxy_registry()
        .set_upstream("openai", format!("http://{mock_addr}"))
        .await;
    manager
        .add_project(ProjectAddParams {
            repo_path: repo.path().to_string_lossy().to_string(),
            name: Some("budget-demo".to_string()),
            default_branch: Some("main".to_string()),
        })
        .await
        .unwrap();

    let ses = manager
        .run_session(SessionRunParams {
            project: "budget-demo".to_string(),
            profile: "budget-agent".to_string(),
            task: "verify budget trip".to_string(),
            base_ref: None,
        })
        .await
        .unwrap();

    let mut injected_url = None;
    for _ in 0..40 {
        if let Ok(snap) = manager.screen_snapshot(&ses.id).await {
            if let Some(line) = snap.text.lines().find(|l| l.contains("PROXY_URL=")) {
                if let Some(val) = line.split("PROXY_URL=").nth(1) {
                    let trimmed = val.trim();
                    if trimmed.starts_with("http://127.0.0.1:") {
                        injected_url = Some(trimmed.to_string());
                        break;
                    }
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    let proxy_url = injected_url.expect("OPENAI_BASE_URL was not injected");

    let client = reqwest::Client::new();
    let timer = Instant::now();
    let resp = client
        .post(format!("{proxy_url}/chat/completions"))
        .header("content-type", "application/json")
        .body(r#"{"model":"gpt-4o"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let _ = resp.bytes().await.unwrap();

    let mut tripped = false;
    while timer.elapsed() < Duration::from_secs(1) {
        let cur = manager.get_session(&ses.id).await.unwrap();
        if cur.status == SessionStatus::Tripped {
            assert_eq!(cur.trip_reason, Some(TripReason::Budget));
            tripped = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    assert!(tripped, "session did not trip on budget exceeded under 1s");

    manager.request_shutdown().await;
}
