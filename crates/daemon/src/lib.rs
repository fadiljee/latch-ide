#![forbid(unsafe_code)]
//! Orkestrator daemon Latch: Session Manager, server UDS JSON-RPC, logging ter-redaksi,
//! dan rekonsiliasi startup.

pub mod http_server;
pub mod logging;
pub mod notify;
pub mod session_manager;
pub mod shim;
pub mod uds_server;

use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Context;
use latch_core::config::{load_agents_config, load_latch_config, RuntimePaths};
use latch_sandbox::select_provider;
use tokio::sync::Notify;
use tracing::{info, warn};

use crate::http_server::{ensure_or_load_token, start_http_server};
use crate::logging::init_logging;
use crate::notify::spawn_notification_dispatcher;
use crate::session_manager::SessionManager;
use crate::uds_server::serve_uds;

/// Deskripsi status modul terjadwal pada daemon agar keterhubungan crate terjaga.
pub fn planned_subsystems() -> Vec<(&'static str, &'static str)> {
    let mut list = vec![
        ("proxy", std::any::type_name::<latch_proxy::ProxyError>()),
        ("dag", std::any::type_name::<latch_dag::DagError>()),
    ];
    #[cfg(feature = "ide")]
    {
        list.push((
            "fs-service",
            std::any::type_name::<latch_fs_service::FsServiceError>(),
        ));
        list.push(("search", std::any::type_name::<latch_search::SearchError>()));
        list.push((
            "agent-proto",
            std::any::type_name::<latch_agent_proto::AgentProtoError>(),
        ));
        list.push((
            "workbench-api",
            std::any::type_name::<latch_workbench_api::TokenScope>(),
        ));
    }
    #[cfg(feature = "lsp")]
    {
        list.push((
            "lsp-host",
            std::any::type_name::<latch_lsp_host::LspHostError>(),
        ));
    }
    list
}

/// Menjalankan daemon Latch secara foreground di atas runtime Tokio beranggaran RAM rendah.
pub fn run_foreground(paths: RuntimePaths) -> anyhow::Result<()> {
    let cfg = load_latch_config(&paths.config_file()).context("failed loading config.toml")?;
    let agents = load_agents_config(&paths.agents_file()).context("failed loading agents.toml")?;

    let effective_state_dir = cfg
        .daemon
        .state_dir
        .as_deref()
        .map(RuntimePaths::expand_tilde)
        .unwrap_or_else(|| paths.state_dir.clone());
    let paths = RuntimePaths {
        state_dir: effective_state_dir,
        ..paths
    };

    std::fs::create_dir_all(&paths.state_dir)?;
    std::fs::create_dir_all(paths.logs_dir())?;
    let _log_guard = init_logging(&paths.logs_dir())?;

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(cfg.daemon.worker_threads)
        .thread_stack_size(512 * 1024)
        .enable_all()
        .build()
        .context("failed building Tokio runtime")?;

    rt.block_on(async move {
        let sandbox: Arc<dyn latch_core::sandbox::SandboxProvider> = Arc::from(
            select_provider(cfg.sandbox.require_sandbox)
                .context("sandbox requirement check failed")?,
        );
        if !sandbox.capabilities().filesystem_isolation {
            warn!(
                provider = sandbox.name(),
                "running without kernel filesystem sandbox isolation"
            );
        }

        let shutdown = Arc::new(Notify::new());
        let http_bind: SocketAddr = cfg
            .daemon
            .http_bind
            .parse()
            .context("invalid daemon.http_bind address")?;
        let notifications_cfg = cfg.notifications.clone();

        let manager =
            SessionManager::start(paths.clone(), cfg, agents, sandbox, Arc::clone(&shutdown))
                .await
                .context("failed initializing SessionManager")?;

        let web_token =
            ensure_or_load_token(&paths.token_file()).context("failed preparing web auth token")?;
        let http_handle =
            start_http_server(http_bind, manager.clone(), web_token, Arc::clone(&shutdown))
                .await
                .context("failed starting local HTTP/WS server")?;

        spawn_notification_dispatcher(
            notifications_cfg,
            manager.subscribe_events(),
            manager.clone(),
        );

        info!(
            socket = %paths.socket_path.display(),
            http = %http_handle.addr(),
            proxy = %manager.proxy_base_url(),
            pid = std::process::id(),
            "latch daemon ready"
        );

        let shutdown_signal = Arc::clone(&shutdown);
        tokio::spawn(async move {
            wait_for_termination_signal().await;
            shutdown_signal.notify_waiters();
        });

        serve_uds(&paths.socket_path, manager, shutdown).await?;
        Ok(())
    })
}

async fn wait_for_termination_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).ok();
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = async {
                if let Some(ref mut s) = sigterm {
                    s.recv().await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {}
        }
    }
    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}
