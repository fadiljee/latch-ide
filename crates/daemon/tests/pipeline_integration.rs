#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::net::SocketAddr;
use std::process::Command;
use std::sync::Arc;
use std::time::Duration;

use axum::http::StatusCode;
use latch_core::config::{AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::id::{new_id, now_ms};
use latch_core::model::{PipelineRun, PipelineStatus};
use latch_core::protocol::{PipelineListParams, PipelineRunParams, ProjectAddParams};
use latch_daemon::http_server::{ensure_or_load_token, start_http_server};
use latch_daemon::session_manager::SessionManager;
use latch_sandbox::NoopProvider;
use latch_store::StoreHandle;
use tempfile::TempDir;
use tokio::sync::Notify;

fn init_repo() -> TempDir {
    let dir = TempDir::new().unwrap();
    for args in [
        &["init", "-b", "main"][..],
        &["config", "user.name", "PipeTest"],
        &["config", "user.email", "pipe@localhost"],
    ] {
        assert!(Command::new("git")
            .current_dir(dir.path())
            .args(args)
            .status()
            .unwrap()
            .success());
    }
    std::fs::write(dir.path().join("README.md"), "pipeline\n").unwrap();
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
async fn runs_pipeline_via_rest_and_recovers_interrupted_pipeline_on_restart() {
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

    let proj = manager
        .add_project(ProjectAddParams {
            repo_path: repo.path().to_string_lossy().to_string(),
            name: Some("pipedemo".to_string()),
            default_branch: Some("main".to_string()),
        })
        .await
        .unwrap();

    // 1. Workflow tidak valid (siklus) harus ditolak dengan HTTP 400
    let bad_wf = r#"
[workflow]
name = "bad-cycle"
[[steps]]
id = "a"
type = "shell"
depends_on = ["b"]
command = "echo a"
[[steps]]
id = "b"
type = "shell"
depends_on = ["a"]
command = "echo b"
"#;
    let bad_body = serde_json::to_vec(&PipelineRunParams {
        project: "pipedemo".to_string(),
        workflow_toml: Some(bad_wf.to_string()),
        workflow_path: None,
        task: "bad".to_string(),
    })
    .unwrap();
    let bad_resp = client
        .post(format!("{base}/api/v1/pipelines"))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body(bad_body)
        .send()
        .await
        .unwrap();
    assert_eq!(bad_resp.status(), StatusCode::BAD_REQUEST);

    // 2. Jalankan workflow shell 2-langkah yang valid lewat REST
    let good_wf = r#"
[workflow]
name = "shell-smoke"

[[steps]]
id = "check_readme"
type = "shell"
command = "test -f README.md && echo ok"

[[steps]]
id = "verify"
type = "shell"
depends_on = ["check_readme"]
command = "echo done"
"#;
    let good_body = serde_json::to_vec(&PipelineRunParams {
        project: "pipedemo".to_string(),
        workflow_toml: Some(good_wf.to_string()),
        workflow_path: None,
        task: "verify repo".to_string(),
    })
    .unwrap();
    let create_resp = client
        .post(format!("{base}/api/v1/pipelines"))
        .bearer_auth(&token)
        .header("content-type", "application/json")
        .body(good_body)
        .send()
        .await
        .unwrap();
    assert_eq!(create_resp.status(), StatusCode::CREATED);
    let created: PipelineRun = serde_json::from_slice(&create_resp.bytes().await.unwrap()).unwrap();

    // Tunggu sampai pipeline selesai
    let mut final_run = created.clone();
    for _ in 0..40 {
        let resp = client
            .get(format!("{base}/api/v1/pipelines/{}", created.id))
            .bearer_auth(&token)
            .send()
            .await
            .unwrap();
        final_run = serde_json::from_slice(&resp.bytes().await.unwrap()).unwrap();
        if final_run.status != PipelineStatus::Running
            && final_run.status != PipelineStatus::Pending
        {
            break;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(final_run.status, PipelineStatus::Succeeded);

    let listed = manager
        .list_pipelines(PipelineListParams {
            project: Some("pipedemo".to_string()),
        })
        .await
        .unwrap();
    assert_eq!(listed.len(), 1);

    // 3. Uji T-408: simulasikan pipeline 'running' yang terputus saat daemon crash/restart
    manager.request_shutdown().await;
    drop(manager);
    {
        let store = StoreHandle::open(&paths.db_file()).unwrap();
        let interrupted = PipelineRun {
            id: new_id(),
            project_id: proj.id.clone(),
            workflow_name: "crashed-wf".to_string(),
            dag_state_json: r#"{"workflow_name":"crashed-wf","status":"running","total_cost_usd":0.12,"reason":null,"steps":{}}"#.to_string(),
            status: PipelineStatus::Running,
            created_at: now_ms(),
            updated_at: None,
            task: Some("interrupted task".to_string()),
            total_cost_usd: 0.12,
        };
        let interrupted_id = interrupted.id.clone();
        store.insert_pipeline_run(interrupted).await.unwrap();
        drop(store);

        let shutdown2 = Arc::new(Notify::new());
        let restarted = SessionManager::start(
            paths.clone(),
            LatchConfig::default(),
            AgentsConfig::with_builtin_defaults(),
            Arc::new(NoopProvider),
            Arc::clone(&shutdown2),
        )
        .await
        .unwrap();

        let recovered = restarted.get_pipeline(&interrupted_id).await.unwrap();
        assert_eq!(recovered.status, PipelineStatus::Failed);
        assert!(recovered.dag_state_json.contains("daemon restarted"));
        let audits = restarted.list_audit(20).await.unwrap();
        assert!(audits.iter().any(|a| a.kind == "reconcile_pipeline"));

        restarted.request_shutdown().await;
    }
}
