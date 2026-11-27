//! Pengelola siklus hidup sesi, worktree, PTY, dan rekonsiliasi startup (T-104, T-131).

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use latch_core::config::{AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::id::{new_id, now_ms, short_id};
use latch_core::model::{
    ApprovalActionType, ApprovalDecision, ApprovalRequest, AuditEntry, GcReport, MergeCheckResult,
    MergeStrategy, PipelineRun, PipelineStatus, Project, ProxyMode, Session, SessionDiff,
    SessionSnapshot, SessionStatus, SessionTelemetrySummary, TelemetryRecord, TripReason,
};
use latch_core::pricing::{load_pricing_table, PricingTable};
use latch_core::protocol::{
    DaemonHealth, PipelineArenaSelectParams, PipelineListParams, PipelineRunParams,
    ProjectAddParams, ScreenSnapshot, SessionDiffParams, SessionListParams, SessionMergeParams,
    SessionRunParams,
};
use latch_core::sandbox::SandboxProvider;
use latch_dag::{execute_workflow, parse_and_validate_workflow, DagError, DagRuntimeState};
use latch_git_engine::{
    create_worktree, inspect_repo, list_worktrees, merge_check, merge_session, remove_worktree,
    worktree_diff, GitError,
};
use latch_proxy::{
    proxy_env_vars, start_proxy_server, BypassLoopDetector, ProxyError, ProxyRegistry,
    ProxyServerHandle, SessionLimits, TripEvent,
};
use latch_pty_pool::session::{is_pid_alive, PtyExitEvent, PtyFrame, PtySubscriber};
use latch_pty_pool::{PtyError, PtyPool, PtySpawnRequest};
use latch_sandbox::build_session_policy;
use latch_store::{StoreError, StoreHandle};
use thiserror::Error;
use tokio::sync::{broadcast, mpsc, oneshot, Mutex as AsyncMutex, Notify, RwLock};
use tracing::{info, warn};

const EXIT_EVENT_CAP: usize = 64;
const DAEMON_EVENT_CAP: usize = 128;
const TELEMETRY_CHANNEL_CAP: usize = 512;
const TELEMETRY_BATCH_MAX: usize = 32;
const TELEMETRY_FLUSH_INTERVAL: Duration = Duration::from_millis(200);
const TRIP_EVENT_CAP: usize = 64;
const DEFAULT_DIFF_MAX_BYTES: usize = 256 * 1024;

/// Kesalahan orkestrasi pada `SessionManager`.
#[derive(Debug, Error)]
pub enum ManagerError {
    /// Galat persistensi SQLite.
    #[error(transparent)]
    Store(#[from] StoreError),
    /// Galat operasi Git.
    #[error(transparent)]
    Git(#[from] GitError),
    /// Galat manajemen PTY.
    #[error(transparent)]
    Pty(#[from] PtyError),
    /// Galat reverse proxy LLM lokal.
    #[error(transparent)]
    Proxy(#[from] ProxyError),
    /// Galat parser/eksekutor pipeline DAG.
    #[error(transparent)]
    Dag(#[from] DagError),
    /// Galat validasi konfigurasi/profil.
    #[error("configuration error: {0}")]
    Config(String),
    /// Batas sesi aktif tercapai.
    #[error("maximum active sessions limit reached ({0})")]
    MaxSessionsReached(usize),
    /// Status sesi tidak mengizinkan aksi yang diminta.
    #[error("session '{id}' in state '{status}' cannot perform '{action}'")]
    InvalidState {
        /// ID sesi.
        id: String,
        /// Status saat ini.
        status: SessionStatus,
        /// Aksi yang diminta.
        action: &'static str,
    },
}

/// Event daemon yang dipublikasikan ke pelanggan/pengamat.
#[derive(Debug, Clone)]
pub enum DaemonEvent {
    /// Sesi baru dibuat dan dijalankan.
    SessionCreated { session_id: String },
    /// Transisi status sesi.
    SessionStatusChanged {
        session_id: String,
        from: SessionStatus,
        to: SessionStatus,
    },
    /// Permintaan approval baru menunggu keputusan manusia.
    ApprovalRequested { approval: ApprovalRequest },
    /// Permintaan approval telah diputuskan.
    ApprovalDecided { approval: ApprovalRequest },
    /// Status pipeline DAG diperbarui.
    PipelineUpdated {
        pipeline_id: String,
        status: PipelineStatus,
    },
}

/// Koordinator state sesi daemon (`store` + `git-engine` + `pty-pool` + `sandbox` + `proxy`).
#[derive(Clone)]
pub struct SessionManager {
    paths: RuntimePaths,
    config: LatchConfig,
    agents: Arc<RwLock<AgentsConfig>>,
    store: StoreHandle,
    pty_pool: PtyPool,
    sandbox: Arc<dyn SandboxProvider>,
    proxy_registry: ProxyRegistry,
    proxy_handle: Arc<ProxyServerHandle>,
    approval_waiters: Arc<AsyncMutex<HashMap<String, oneshot::Sender<ApprovalRequest>>>>,
    pipeline_tasks: Arc<AsyncMutex<HashMap<String, tokio::task::JoinHandle<()>>>>,
    exit_tx: mpsc::Sender<PtyExitEvent>,
    trip_tx: mpsc::Sender<TripEvent>,
    events_tx: broadcast::Sender<DaemonEvent>,
    shutdown: Arc<Notify>,
}

impl SessionManager {
    /// Memulai `SessionManager`, membuka SQLite, menyalakan proxy LLM lokal, serta menjalankan rekonsiliasi startup.
    pub async fn start(
        paths: RuntimePaths,
        config: LatchConfig,
        agents: AgentsConfig,
        sandbox: Arc<dyn SandboxProvider>,
        shutdown: Arc<Notify>,
    ) -> Result<Self, ManagerError> {
        let store = StoreHandle::open(&paths.db_file())?;
        let pty_pool = PtyPool::new();
        let (exit_tx, exit_rx) = mpsc::channel(EXIT_EVENT_CAP);
        let (telemetry_tx, telemetry_rx) = mpsc::channel(TELEMETRY_CHANNEL_CAP);
        let (trip_tx, trip_rx) = mpsc::channel(TRIP_EVENT_CAP);
        let (events_tx, _) = broadcast::channel(DAEMON_EVENT_CAP);

        let pricing = match load_pricing_table(&paths.pricing_file()) {
            Ok(table) => table,
            Err(err) => {
                warn!(error = %err, "failed loading pricing.toml; using built-in pricing defaults");
                PricingTable::default()
            }
        };

        let proxy_registry = ProxyRegistry::new();
        let proxy_handle = start_proxy_server(
            SocketAddr::from(([127, 0, 0, 1], 0)),
            proxy_registry.clone(),
            pricing,
            telemetry_tx,
            trip_tx.clone(),
        )
        .await?;

        let manager = Self {
            paths,
            config,
            agents: Arc::new(RwLock::new(agents)),
            store,
            pty_pool,
            sandbox,
            proxy_registry,
            proxy_handle: Arc::new(proxy_handle),
            approval_waiters: Arc::new(AsyncMutex::new(HashMap::new())),
            pipeline_tasks: Arc::new(AsyncMutex::new(HashMap::new())),
            exit_tx,
            trip_tx,
            events_tx,
            shutdown,
        };

        manager.reconcile_startup().await?;
        manager.spawn_exit_watcher(exit_rx);
        manager.spawn_telemetry_batch_writer(telemetry_rx);
        manager.spawn_trip_watcher(trip_rx);
        manager.spawn_idle_hibernate_watcher();
        Ok(manager)
    }

    /// URL dasar reverse proxy LLM lokal (`http://127.0.0.1:<port>`).
    pub fn proxy_base_url(&self) -> String {
        self.proxy_handle.base_url()
    }

    /// Akses ke registri proxy lokal (untuk penimpaan upstream pada pengujian atau integrasi).
    pub fn proxy_registry(&self) -> &ProxyRegistry {
        &self.proxy_registry
    }

    /// Mengambil daftar catatan telemetri proxy untuk sebuah sesi.
    pub async fn session_telemetry(
        &self,
        session_id: &str,
        since: Option<i64>,
    ) -> Result<Vec<TelemetryRecord>, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        Ok(self.store.list_telemetry(&ses.id, since).await?)
    }

    /// Mengambil ringkasan total token, biaya, dan error untuk sebuah sesi.
    pub async fn session_telemetry_summary(
        &self,
        session_id: &str,
    ) -> Result<SessionTelemetrySummary, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        Ok(self.store.session_telemetry_summary(&ses.id).await?)
    }

    /// Mencatat peristiwa audit ke SQLite (otomatis melewati redaksi rahasia).
    pub async fn record_audit(&self, entry: AuditEntry) -> Result<(), ManagerError> {
        Ok(self.store.insert_audit(entry).await?)
    }

    /// Mengambil entri audit terbaru.
    pub async fn list_audit(&self, limit: usize) -> Result<Vec<AuditEntry>, ManagerError> {
        Ok(self.store.list_audit(limit).await?)
    }

    /// Membuat permintaan approval baru (`T-308`); bila `wait == true`, menunggu keputusan atau timeout (default deny).
    pub async fn create_approval(
        &self,
        session_id: &str,
        action_type: ApprovalActionType,
        payload_summary: String,
        timeout_secs: u64,
        wait: bool,
    ) -> Result<ApprovalRequest, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        let now = now_ms();
        let expires_at = if timeout_secs > 0 {
            Some(now + i64::try_from(timeout_secs.saturating_mul(1000)).unwrap_or(300_000))
        } else {
            None
        };
        let req = ApprovalRequest {
            id: new_id(),
            session_id: ses.id.clone(),
            action_type,
            payload_summary,
            decision: ApprovalDecision::Pending,
            decided_by: None,
            decided_at: None,
            created_at: now,
            expires_at,
        };
        let saved = self.store.insert_approval(req).await?;
        if ses.status == SessionStatus::Running {
            let _ = self
                .store
                .transition_session(
                    &ses.id,
                    SessionStatus::WaitingApproval,
                    None,
                    None,
                    None,
                    false,
                )
                .await;
            let _ = self.events_tx.send(DaemonEvent::SessionStatusChanged {
                session_id: ses.id.clone(),
                from: SessionStatus::Running,
                to: SessionStatus::WaitingApproval,
            });
        }
        let _ = self.events_tx.send(DaemonEvent::ApprovalRequested {
            approval: saved.clone(),
        });

        if !wait {
            return Ok(saved);
        }

        let (tx, rx) = oneshot::channel();
        self.approval_waiters
            .lock()
            .await
            .insert(saved.id.clone(), tx);

        let wait_dur = Duration::from_secs(timeout_secs.max(1));
        match tokio::time::timeout(wait_dur, rx).await {
            Ok(Ok(decided)) => Ok(decided),
            _ => {
                self.approval_waiters.lock().await.remove(&saved.id);
                self.decide_approval(&saved.id, ApprovalDecision::Denied, "timeout")
                    .await
            }
        }
    }

    /// Mengambil daftar permintaan approval dengan filter status opsional.
    pub async fn list_approvals(
        &self,
        decision: Option<ApprovalDecision>,
    ) -> Result<Vec<ApprovalRequest>, ManagerError> {
        Ok(self.store.list_approvals(decision).await?)
    }

    /// Memutuskan permintaan approval (`approved` / `denied`), membangunkan waiter < 50 ms, dan menulis audit.
    pub async fn decide_approval(
        &self,
        id: &str,
        decision: ApprovalDecision,
        decided_by: &str,
    ) -> Result<ApprovalRequest, ManagerError> {
        let updated = self.store.decide_approval(id, decision, decided_by).await?;
        if let Ok(ses) = self.store.get_session(&updated.session_id).await {
            if ses.status == SessionStatus::WaitingApproval {
                let _ = self
                    .store
                    .transition_session(&ses.id, SessionStatus::Running, None, None, None, false)
                    .await;
                let _ = self.events_tx.send(DaemonEvent::SessionStatusChanged {
                    session_id: ses.id,
                    from: SessionStatus::WaitingApproval,
                    to: SessionStatus::Running,
                });
            }
        }
        let _ = self
            .store
            .insert_audit(AuditEntry {
                id: None,
                session_id: Some(updated.session_id.clone()),
                kind: "approval".to_string(),
                detail: format!(
                    "id={} decision={} by={}",
                    updated.id,
                    updated.decision.as_str(),
                    decided_by
                ),
                timestamp: now_ms(),
            })
            .await;
        if let Some(waiter) = self.approval_waiters.lock().await.remove(&updated.id) {
            let _ = waiter.send(updated.clone());
        }
        let _ = self.events_tx.send(DaemonEvent::ApprovalDecided {
            approval: updated.clone(),
        });
        Ok(updated)
    }

    /// Menghentikan paksa sesi akibat circuit breaker (`T-208`) dalam < 1 detik.
    pub async fn trip_session(
        &self,
        session_id: &str,
        reason: TripReason,
        detail: &str,
    ) -> Result<Session, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        self.proxy_registry.revoke_session(&ses.id).await;
        if ses.status.is_terminal() {
            return Ok(ses);
        }
        // Transisikan ke Tripped terlebih dahulu agar exit watcher tidak menimpa menjadi Failed.
        let updated = self
            .store
            .transition_session(
                &ses.id,
                SessionStatus::Tripped,
                Some(-9),
                Some(reason),
                Some(now_ms()),
                true,
            )
            .await?;
        if let Ok(handle) = self.pty_pool.get(&ses.id).await {
            let _ = handle.kill().await;
        }
        let _ = self
            .store
            .insert_audit(AuditEntry {
                id: None,
                session_id: Some(updated.id.clone()),
                kind: "trip".to_string(),
                detail: format!("reason={}: {}", reason.as_str(), detail),
                timestamp: now_ms(),
            })
            .await;
        let _ = self.events_tx.send(DaemonEvent::SessionStatusChanged {
            session_id: updated.id.clone(),
            from: ses.status,
            to: updated.status,
        });
        warn!(
            session_id = %updated.id,
            reason = reason.as_str(),
            "circuit breaker tripped session"
        );
        Ok(updated)
    }

    /// Memeriksa kesehatan daemon dan jumlah sesi aktif.
    pub async fn health(&self) -> DaemonHealth {
        DaemonHealth {
            status: "ok".to_string(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            sandbox: self.sandbox.name().to_string(),
            pid: std::process::id(),
            active_sessions: self.pty_pool.active_count().await,
        }
    }

    /// Memicu penghentian bersih seluruh sesi aktif lalu menutup daemon.
    pub async fn request_shutdown(&self) {
        let active = self
            .store
            .list_sessions(None, None)
            .await
            .unwrap_or_default();
        for ses in active {
            self.proxy_registry.revoke_session(&ses.id).await;
            if let Ok(handle) = self.pty_pool.get(&ses.id).await {
                let _ = handle.kill().await;
            }
        }
        self.proxy_handle.shutdown();
        self.shutdown.notify_waiters();
    }

    /// Berlangganan event status sesi daemon.
    pub fn subscribe_events(&self) -> broadcast::Receiver<DaemonEvent> {
        self.events_tx.subscribe()
    }

    /// Menjalankan rekonsiliasi pasca-crash saat startup (`T-104`, `NFR-06`).
    pub async fn reconcile_startup(&self) -> Result<(), ManagerError> {
        let sessions = self.store.list_sessions(None, None).await?;
        let mut known_worktrees = HashSet::new();

        for ses in sessions {
            known_worktrees.insert(PathBuf::from(&ses.worktree_path));
            if matches!(
                ses.status,
                SessionStatus::Running | SessionStatus::Paused | SessionStatus::WaitingApproval
            ) {
                let alive = ses
                    .pid
                    .and_then(|p| u32::try_from(p).ok())
                    .is_some_and(is_pid_alive);
                if !alive {
                    warn!(session_id = %ses.id, pid = ?ses.pid, "reconciling dead session to failed");
                    let _ = self
                        .store
                        .transition_session(
                            &ses.id,
                            SessionStatus::Failed,
                            Some(-1),
                            None,
                            Some(now_ms()),
                            true,
                        )
                        .await;
                    let _ = self
                        .store
                        .insert_audit(AuditEntry {
                            id: None,
                            session_id: Some(ses.id),
                            kind: "reconcile_dead_session".to_string(),
                            detail: "marked dead session from previous daemon run as failed"
                                .to_string(),
                            timestamp: now_ms(),
                        })
                        .await;
                }
            }
        }

        // Deteksi & bersihkan worktree yatim di bawah <state_dir>/worktrees/
        let wt_root = self.paths.worktrees_dir();
        if wt_root.exists() {
            let projects = self.store.list_projects().await?;
            for proj in &projects {
                let repo = PathBuf::from(&proj.repo_path);
                if let Ok(entries) = list_worktrees(&repo).await {
                    for entry in entries {
                        if entry.path.starts_with(&wt_root)
                            && !known_worktrees.contains(&entry.path)
                        {
                            let branch = entry.branch.as_deref().unwrap_or("");
                            warn!(
                                worktree = %entry.path.display(),
                                branch = %branch,
                                "cleaning orphan git worktree"
                            );
                            let _ = remove_worktree(&repo, &entry.path, branch).await;
                            let _ = self
                                .store
                                .insert_audit(AuditEntry {
                                    id: None,
                                    session_id: None,
                                    kind: "orphan_cleanup".to_string(),
                                    detail: format!(
                                        "cleaned orphan worktree {}",
                                        entry.path.display()
                                    ),
                                    timestamp: now_ms(),
                                })
                                .await;
                        }
                    }
                }
            }

            // Sapu direktori yatim sisa di filesystem yang tidak lagi punya entri sesi
            if let Ok(mut proj_dirs) = tokio::fs::read_dir(&wt_root).await {
                while let Ok(Some(proj_entry)) = proj_dirs.next_entry().await {
                    let proj_path = proj_entry.path();
                    if !proj_path.is_dir() {
                        continue;
                    }
                    if let Ok(mut ses_dirs) = tokio::fs::read_dir(&proj_path).await {
                        while let Ok(Some(ses_entry)) = ses_dirs.next_entry().await {
                            let ses_path = ses_entry.path();
                            if ses_path.is_dir() && !known_worktrees.contains(&ses_path) {
                                let _ = tokio::fs::remove_dir_all(&ses_path).await;
                                let _ = self
                                    .store
                                    .insert_audit(AuditEntry {
                                        id: None,
                                        session_id: None,
                                        kind: "orphan_cleanup".to_string(),
                                        detail: format!(
                                            "removed unreferenced worktree dir {}",
                                            ses_path.display()
                                        ),
                                        timestamp: now_ms(),
                                    })
                                    .await;
                            }
                        }
                    }
                }
            }
        }

        // Pemulihan pipeline yang terputus akibat restart daemon (T-408)
        let pipelines = self.store.list_pipeline_runs(None).await?;
        for run in pipelines {
            if matches!(
                run.status,
                PipelineStatus::Pending | PipelineStatus::Running
            ) {
                warn!(
                    pipeline_id = %run.id,
                    workflow = %run.workflow_name,
                    "reconciling interrupted pipeline run to failed after daemon restart"
                );
                let mut state: DagRuntimeState = serde_json::from_str(&run.dag_state_json)
                    .unwrap_or_else(|_| DagRuntimeState {
                        workflow_name: run.workflow_name.clone(),
                        status: PipelineStatus::Failed,
                        total_cost_usd: run.total_cost_usd,
                        reason: None,
                        steps: HashMap::new(),
                    });
                state.status = PipelineStatus::Failed;
                state.reason = Some("daemon restarted while pipeline was running".to_string());
                let serialized =
                    serde_json::to_string(&state).unwrap_or_else(|_| run.dag_state_json.clone());
                let _ = self
                    .store
                    .update_pipeline_run(
                        &run.id,
                        PipelineStatus::Failed,
                        serialized,
                        state.total_cost_usd,
                    )
                    .await;
                let _ = self
                    .store
                    .insert_audit(AuditEntry {
                        id: None,
                        session_id: None,
                        kind: "reconcile_pipeline".to_string(),
                        detail: format!(
                            "marked interrupted pipeline {} ({}) as failed on startup",
                            run.id, run.workflow_name
                        ),
                        timestamp: now_ms(),
                    })
                    .await;
            }
        }

        Ok(())
    }

    fn spawn_exit_watcher(&self, mut exit_rx: mpsc::Receiver<PtyExitEvent>) {
        let store = self.store.clone();
        let events_tx = self.events_tx.clone();
        let proxy_registry = self.proxy_registry.clone();
        tokio::spawn(async move {
            while let Some(ev) = exit_rx.recv().await {
                proxy_registry.revoke_session(&ev.session_id).await;
                let target = if ev.exit_code == 0 {
                    SessionStatus::Completed
                } else {
                    SessionStatus::Failed
                };
                if let Ok(existing) = store.get_session(&ev.session_id).await {
                    if existing.status.is_terminal() || existing.status == SessionStatus::Hibernated
                    {
                        continue;
                    }
                    let final_status = if existing.status == SessionStatus::Paused
                        && target == SessionStatus::Completed
                    {
                        SessionStatus::Failed
                    } else {
                        target
                    };
                    match store
                        .transition_session(
                            &ev.session_id,
                            final_status,
                            Some(ev.exit_code),
                            None,
                            Some(now_ms()),
                            true,
                        )
                        .await
                    {
                        Ok(updated) => {
                            let _ = events_tx.send(DaemonEvent::SessionStatusChanged {
                                session_id: updated.id,
                                from: existing.status,
                                to: updated.status,
                            });
                        }
                        Err(err) => {
                            warn!(
                                session_id = %ev.session_id,
                                error = %err,
                                "failed updating session status on PTY exit"
                            );
                        }
                    }
                }
            }
        });
    }

    fn spawn_telemetry_batch_writer(&self, mut telemetry_rx: mpsc::Receiver<TelemetryRecord>) {
        let store = self.store.clone();
        tokio::spawn(async move {
            let mut batch: Vec<TelemetryRecord> = Vec::with_capacity(TELEMETRY_BATCH_MAX);
            let mut interval = tokio::time::interval(TELEMETRY_FLUSH_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    maybe_rec = telemetry_rx.recv() => {
                        match maybe_rec {
                            Some(rec) => {
                                batch.push(rec);
                                if batch.len() >= TELEMETRY_BATCH_MAX {
                                    let to_write = std::mem::take(&mut batch);
                                    if let Err(err) = store.insert_telemetry_batch(to_write).await {
                                        warn!(error = %err, "failed persisting proxy telemetry batch");
                                    }
                                }
                            }
                            None => {
                                if !batch.is_empty() {
                                    let _ = store.insert_telemetry_batch(batch).await;
                                }
                                break;
                            }
                        }
                    }
                    _ = interval.tick() => {
                        if !batch.is_empty() {
                            let to_write = std::mem::take(&mut batch);
                            if let Err(err) = store.insert_telemetry_batch(to_write).await {
                                warn!(error = %err, "failed flushing proxy telemetry batch");
                            }
                        }
                    }
                }
            }
        });
    }

    fn spawn_trip_watcher(&self, mut trip_rx: mpsc::Receiver<TripEvent>) {
        let manager = self.clone();
        tokio::spawn(async move {
            while let Some(ev) = trip_rx.recv().await {
                if let Err(err) = manager
                    .trip_session(&ev.session_id, ev.reason, &ev.detail)
                    .await
                {
                    warn!(
                        session_id = %ev.session_id,
                        error = %err,
                        "failed executing circuit breaker trip"
                    );
                }
            }
        });
    }

    fn spawn_idle_hibernate_watcher(&self) {
        let mins = self.config.circuit_breaker.idle_hibernate_minutes;
        if mins == 0 {
            return;
        }
        let idle_ms = i64::try_from(mins.saturating_mul(60_000)).unwrap_or(900_000);
        let manager = self.clone();
        let shutdown = Arc::clone(&self.shutdown);
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs(30));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
            loop {
                tokio::select! {
                    _ = shutdown.notified() => break,
                    _ = interval.tick() => {
                        let idle_ids = manager.pty_pool.idle_sessions(idle_ms).await;
                        for sid in idle_ids {
                            if let Ok(ses) = manager.store.get_session(&sid).await {
                                if ses.status == SessionStatus::Running {
                                    if let Err(err) = manager.hibernate_session(&sid).await {
                                        warn!(session_id = %sid, error = %err, "idle hibernation failed");
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });
    }

    /// Mendaftarkan repositori Git baru.
    pub async fn add_project(&self, params: ProjectAddParams) -> Result<Project, ManagerError> {
        let repo_info = inspect_repo(Path::new(&params.repo_path)).await?;
        let inferred_name = repo_info
            .root
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("project")
            .to_string();
        let project = Project {
            id: new_id(),
            name: params.name.unwrap_or(inferred_name),
            repo_path: repo_info.root.to_string_lossy().to_string(),
            default_branch: params.default_branch.unwrap_or(repo_info.default_branch),
            created_at: now_ms(),
        };
        Ok(self.store.add_project(project).await?)
    }

    /// Mengambil daftar proyek terdaftar.
    pub async fn list_projects(&self) -> Result<Vec<Project>, ManagerError> {
        Ok(self.store.list_projects().await?)
    }

    /// Menghapus proyek terdaftar.
    pub async fn remove_project(&self, id_or_name: &str) -> Result<(), ManagerError> {
        Ok(self.store.remove_project(id_or_name).await?)
    }

    /// Menjalankan satu sesi agen pada worktree + branch baru.
    pub async fn run_session(&self, params: SessionRunParams) -> Result<Session, ManagerError> {
        self.run_session_with_pipeline(params, None, None).await
    }

    /// Menjalankan satu sesi agen yang opsional terikat ke `pipeline_run_id` dan `step_id`.
    pub async fn run_session_with_pipeline(
        &self,
        params: SessionRunParams,
        pipeline_run_id: Option<String>,
        step_id: Option<String>,
    ) -> Result<Session, ManagerError> {
        let active_count = self.pty_pool.active_count().await;
        if active_count >= self.config.limits.max_sessions {
            return Err(ManagerError::MaxSessionsReached(
                self.config.limits.max_sessions,
            ));
        }

        let project = self.store.get_project(&params.project).await?;
        let profile = {
            let guard = self.agents.read().await;
            guard
                .profiles
                .get(&params.profile)
                .cloned()
                .ok_or_else(|| {
                    ManagerError::Config(format!("unknown agent profile '{}'", params.profile))
                })?
        };

        let session_id = new_id();
        let short = short_id(&session_id);
        let branch_name = format!("latch/{}/{}", project.name, short);
        let worktree_path = self
            .paths
            .worktrees_dir()
            .join(&project.name)
            .join(&session_id);
        let isolated_tmp = self.paths.tmp_dir().join(&session_id);
        let log_path = self.paths.logs_dir().join(format!("pty-{session_id}.log"));
        let base_ref = params
            .base_ref
            .as_deref()
            .unwrap_or(&project.default_branch);

        create_worktree(
            Path::new(&project.repo_path),
            &worktree_path,
            &branch_name,
            base_ref,
        )
        .await?;

        let sandbox_policy = build_session_policy(
            &worktree_path,
            &isolated_tmp,
            &self.config.sandbox,
            &profile,
        );

        let extra_env = match profile.proxy_mode {
            ProxyMode::Injected => {
                let provider = profile
                    .proxy_provider
                    .as_deref()
                    .unwrap_or("openai")
                    .to_string();
                let limits = SessionLimits {
                    max_consecutive_errors: self.config.circuit_breaker.max_consecutive_errors,
                    max_session_budget_usd: self.config.circuit_breaker.max_session_budget_usd,
                };
                let token = self
                    .proxy_registry
                    .register_session(session_id.clone(), provider.clone(), limits)
                    .await;
                proxy_env_vars(&self.proxy_handle.base_url(), &token, &provider)
            }
            ProxyMode::Bypass => Vec::new(),
        };

        let spawn_res = self
            .pty_pool
            .spawn(
                PtySpawnRequest {
                    session_id: session_id.clone(),
                    workdir: worktree_path.clone(),
                    isolated_tmp,
                    log_path,
                    profile: profile.clone(),
                    extra_env,
                    task: params.task.clone(),
                    sandbox_policy,
                    ring_buffer_bytes: self.config.limits.pty_ring_buffer_kib * 1024,
                    log_max_bytes: (self.config.limits.pty_log_max_mib as u64) * 1024 * 1024,
                    batch_ms: self.config.limits.pty_batch_ms,
                    scrollback_lines: self.config.limits.pty_scrollback_lines,
                    size: (100, 30),
                },
                self.sandbox.as_ref(),
                self.exit_tx.clone(),
            )
            .await;

        let pty_handle = match spawn_res {
            Ok(h) => h,
            Err(err) => {
                self.proxy_registry.revoke_session(&session_id).await;
                let _ =
                    remove_worktree(Path::new(&project.repo_path), &worktree_path, &branch_name)
                        .await;
                return Err(err.into());
            }
        };

        let session = Session {
            id: session_id.clone(),
            project_id: project.id,
            agent_profile: params.profile,
            worktree_path: worktree_path.to_string_lossy().to_string(),
            branch_name: branch_name.clone(),
            status: SessionStatus::Running,
            pid: pty_handle.pid().map(i64::from),
            started_at: now_ms(),
            ended_at: None,
            task: Some(params.task),
            pipeline_run_id,
            step_id,
            exit_code: None,
            trip_reason: None,
            proxy_mode: profile.proxy_mode,
            sandbox_provider: self.sandbox.name().to_string(),
        };

        let saved = match self.store.insert_session(session).await {
            Ok(s) => s,
            Err(err) => {
                self.proxy_registry.revoke_session(&session_id).await;
                let _ = pty_handle.kill().await;
                let _ =
                    remove_worktree(Path::new(&project.repo_path), &worktree_path, &branch_name)
                        .await;
                return Err(err.into());
            }
        };

        if profile.proxy_mode == ProxyMode::Bypass
            && self.config.circuit_breaker.max_consecutive_errors > 0
        {
            let mut sub = pty_handle.subscribe();
            let mut detector = BypassLoopDetector::new(
                saved.id.clone(),
                self.config.circuit_breaker.max_consecutive_errors,
            );
            let trip_tx = self.trip_tx.clone();
            tokio::spawn(async move {
                while let Some(frame) = sub.rx.recv().await {
                    if let PtyFrame::Output(bytes) = frame {
                        if let Some(trip) = detector.observe_pty_bytes(&bytes) {
                            let _ = trip_tx.try_send(trip);
                            break;
                        }
                    }
                }
            });
        }

        info!(session_id = %saved.id, branch = %saved.branch_name, "session started");
        let _ = self.events_tx.send(DaemonEvent::SessionCreated {
            session_id: saved.id.clone(),
        });
        Ok(saved)
    }

    /// Mengambil daftar sesi dengan filter opsional.
    pub async fn list_sessions(
        &self,
        params: SessionListParams,
    ) -> Result<Vec<Session>, ManagerError> {
        let project_id = match params.project {
            Some(ref key) => Some(self.store.get_project(key).await?.id),
            None => None,
        };
        let status = match params.status {
            Some(ref s) => Some(
                s.parse::<SessionStatus>()
                    .map_err(|e| ManagerError::Config(e.to_string()))?,
            ),
            None => None,
        };
        Ok(self.store.list_sessions(project_id, status).await?)
    }

    /// Mengambil detail satu sesi berdasarkan ID/awalan.
    pub async fn get_session(&self, session_id: &str) -> Result<Session, ManagerError> {
        Ok(self.store.get_session(session_id).await?)
    }

    /// Menjeda sesi aktif (`SIGSTOP`).
    pub async fn pause_session(&self, session_id: &str) -> Result<Session, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        if ses.status != SessionStatus::Running {
            return Err(ManagerError::InvalidState {
                id: ses.id,
                status: ses.status,
                action: "pause",
            });
        }
        let handle = self.pty_pool.get(&ses.id).await?;
        handle.pause()?;
        let updated = self
            .store
            .transition_session(&ses.id, SessionStatus::Paused, None, None, None, false)
            .await?;
        let _ = self.events_tx.send(DaemonEvent::SessionStatusChanged {
            session_id: updated.id.clone(),
            from: ses.status,
            to: updated.status,
        });
        Ok(updated)
    }

    /// Menghibernasi sesi aktif: simpan snapshot layar ke SQLite, lepas proses PTY untuk hemat RAM (`T-410`).
    pub async fn hibernate_session(&self, session_id: &str) -> Result<Session, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        if ses.status != SessionStatus::Running {
            return Err(ManagerError::InvalidState {
                id: ses.id,
                status: ses.status,
                action: "hibernate",
            });
        }
        let snap = if let Ok(handle) = self.pty_pool.get(&ses.id).await {
            let s = handle.snapshot();
            SessionSnapshot {
                session_id: ses.id.clone(),
                screen_blob: s.ansi_bytes,
                worktree_head: None,
                git_status_json: None,
                context_summary: Some(s.text),
                created_at: now_ms(),
            }
        } else {
            SessionSnapshot {
                session_id: ses.id.clone(),
                screen_blob: Vec::new(),
                worktree_head: None,
                git_status_json: None,
                context_summary: None,
                created_at: now_ms(),
            }
        };
        self.store.save_session_snapshot(snap).await?;

        // Transisikan ke Hibernated sebelum membunuh PTY agar exit watcher tidak menimpanya menjadi Failed.
        let updated = self
            .store
            .transition_session(&ses.id, SessionStatus::Hibernated, None, None, None, true)
            .await?;
        self.proxy_registry.revoke_session(&ses.id).await;
        if let Ok(handle) = self.pty_pool.get(&ses.id).await {
            let _ = handle.kill().await;
        }
        let _ = self
            .store
            .insert_audit(AuditEntry {
                id: None,
                session_id: Some(updated.id.clone()),
                kind: "hibernate".to_string(),
                detail: "hibernated idle session and saved terminal snapshot".to_string(),
                timestamp: now_ms(),
            })
            .await;
        let _ = self.events_tx.send(DaemonEvent::SessionStatusChanged {
            session_id: updated.id.clone(),
            from: ses.status,
            to: updated.status,
        });
        Ok(updated)
    }

    /// Melanjutkan sesi yang dijeda (`SIGCONT`) atau membangunkan sesi yang dihibernasi (`T-410`).
    pub async fn resume_session(&self, session_id: &str) -> Result<Session, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        match ses.status {
            SessionStatus::Paused => {
                let handle = self.pty_pool.get(&ses.id).await?;
                handle.resume()?;
                let updated = self
                    .store
                    .transition_session(&ses.id, SessionStatus::Running, None, None, None, false)
                    .await?;
                let _ = self.events_tx.send(DaemonEvent::SessionStatusChanged {
                    session_id: updated.id.clone(),
                    from: ses.status,
                    to: updated.status,
                });
                Ok(updated)
            }
            SessionStatus::Hibernated => {
                let mut profile = {
                    let guard = self.agents.read().await;
                    guard
                        .profiles
                        .get(&ses.agent_profile)
                        .cloned()
                        .ok_or_else(|| {
                            ManagerError::Config(format!(
                                "unknown agent profile '{}'",
                                ses.agent_profile
                            ))
                        })?
                };
                if !profile.resume_args.is_empty() {
                    profile.args = profile.resume_args.clone();
                }

                let worktree_path = PathBuf::from(&ses.worktree_path);
                let isolated_tmp = self.paths.tmp_dir().join(&ses.id);
                let log_path = self.paths.logs_dir().join(format!("pty-{}.log", ses.id));
                let sandbox_policy = build_session_policy(
                    &worktree_path,
                    &isolated_tmp,
                    &self.config.sandbox,
                    &profile,
                );

                let mut extra_env = match profile.proxy_mode {
                    ProxyMode::Injected => {
                        let provider = profile
                            .proxy_provider
                            .as_deref()
                            .unwrap_or("openai")
                            .to_string();
                        let limits = SessionLimits {
                            max_consecutive_errors: self
                                .config
                                .circuit_breaker
                                .max_consecutive_errors,
                            max_session_budget_usd: self
                                .config
                                .circuit_breaker
                                .max_session_budget_usd,
                        };
                        let token = self
                            .proxy_registry
                            .register_session(ses.id.clone(), provider.clone(), limits)
                            .await;
                        proxy_env_vars(&self.proxy_handle.base_url(), &token, &provider)
                    }
                    ProxyMode::Bypass => Vec::new(),
                };
                if let Ok(Some(snap)) = self.store.get_session_snapshot(&ses.id).await {
                    if let Some(ctx) = snap.context_summary {
                        extra_env.push(("LATCH_RESUME_CONTEXT".to_string(), ctx));
                    }
                }

                let pty_handle = self
                    .pty_pool
                    .spawn(
                        PtySpawnRequest {
                            session_id: ses.id.clone(),
                            workdir: worktree_path,
                            isolated_tmp,
                            log_path,
                            profile,
                            extra_env,
                            task: ses.task.clone().unwrap_or_default(),
                            sandbox_policy,
                            ring_buffer_bytes: self.config.limits.pty_ring_buffer_kib * 1024,
                            log_max_bytes: (self.config.limits.pty_log_max_mib as u64)
                                * 1024
                                * 1024,
                            batch_ms: self.config.limits.pty_batch_ms,
                            scrollback_lines: self.config.limits.pty_scrollback_lines,
                            size: (100, 30),
                        },
                        self.sandbox.as_ref(),
                        self.exit_tx.clone(),
                    )
                    .await?;

                self.store
                    .set_session_pid(&ses.id, pty_handle.pid().map(i64::from))
                    .await?;
                let updated = self
                    .store
                    .transition_session(&ses.id, SessionStatus::Running, None, None, None, false)
                    .await?;
                let _ = self
                    .store
                    .insert_audit(AuditEntry {
                        id: None,
                        session_id: Some(updated.id.clone()),
                        kind: "resume_hibernated".to_string(),
                        detail: "resumed hibernated session with saved context".to_string(),
                        timestamp: now_ms(),
                    })
                    .await;
                let _ = self.events_tx.send(DaemonEvent::SessionStatusChanged {
                    session_id: updated.id.clone(),
                    from: ses.status,
                    to: updated.status,
                });
                Ok(updated)
            }
            other => Err(ManagerError::InvalidState {
                id: ses.id,
                status: other,
                action: "resume",
            }),
        }
    }

    /// Menghentikan proses PTY sesi dan menandai sesi `failed` bila masih aktif.
    pub async fn kill_session(&self, session_id: &str) -> Result<Session, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        self.proxy_registry.revoke_session(&ses.id).await;
        if ses.status.is_terminal() {
            return Ok(ses);
        }
        if let Ok(handle) = self.pty_pool.get(&ses.id).await {
            handle.kill().await?;
        }
        let latest = self.store.get_session(&ses.id).await?;
        if !latest.status.is_terminal() {
            return Ok(self
                .store
                .transition_session(
                    &ses.id,
                    SessionStatus::Failed,
                    Some(-9),
                    None,
                    Some(now_ms()),
                    true,
                )
                .await?);
        }
        Ok(latest)
    }

    /// Mengirim input byte ke PTY sesi aktif.
    pub async fn send_input(&self, session_id: &str, data: &[u8]) -> Result<(), ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        let handle = self.pty_pool.get(&ses.id).await?;
        handle.write_input(data)?;
        Ok(())
    }

    /// Mengubah ukuran terminal PTY sesi aktif.
    pub async fn resize_session(
        &self,
        session_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        let handle = self.pty_pool.get(&ses.id).await?;
        handle.resize(cols, rows)?;
        Ok(())
    }

    /// Mendaftarkan subscriber PTY baru untuk mode attach.
    pub async fn attach_session(
        &self,
        session_id: &str,
    ) -> Result<
        (
            Session,
            latch_pty_pool::session::PtySessionHandle,
            PtySubscriber,
        ),
        ManagerError,
    > {
        let ses = self.store.get_session(session_id).await?;
        let handle = self.pty_pool.get(&ses.id).await?;
        let sub = handle.subscribe();
        Ok((ses, handle, sub))
    }

    /// Mengambil snapshot layar `vt100` dari sesi aktif.
    pub async fn screen_snapshot(&self, session_id: &str) -> Result<ScreenSnapshot, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        let handle = self.pty_pool.get(&ses.id).await?;
        let snap = handle.snapshot();
        Ok(ScreenSnapshot {
            session_id: ses.id,
            cols: snap.cols,
            rows: snap.rows,
            text: snap.text,
            ansi_bytes: snap.ansi_bytes,
        })
    }

    /// Menghitung diff worktree sesi terhadap branch default proyek.
    pub async fn diff_session(
        &self,
        params: SessionDiffParams,
    ) -> Result<SessionDiff, ManagerError> {
        let ses = self.store.get_session(&params.session_id).await?;
        let proj = self.store.get_project(&ses.project_id).await?;
        let max_bytes = params.max_bytes.unwrap_or(DEFAULT_DIFF_MAX_BYTES);
        Ok(worktree_diff(
            Path::new(&ses.worktree_path),
            &proj.default_branch,
            params.file.as_deref(),
            max_bytes,
        )
        .await?)
    }

    /// Mensimulasikan merge (`git merge-tree`) untuk memeriksa konflik.
    pub async fn merge_check_session(
        &self,
        session_id: &str,
    ) -> Result<MergeCheckResult, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        let proj = self.store.get_project(&ses.project_id).await?;
        Ok(merge_check(
            Path::new(&proj.repo_path),
            Path::new(&ses.worktree_path),
            &ses.branch_name,
            &proj.default_branch,
        )
        .await?)
    }

    /// Menggabungkan hasil sesi ke branch default setelah sesi tidak lagi aktif.
    pub async fn merge_session(&self, params: SessionMergeParams) -> Result<Session, ManagerError> {
        let ses = self.store.get_session(&params.session_id).await?;
        if ses.status.is_active() {
            return Err(ManagerError::InvalidState {
                id: ses.id,
                status: ses.status,
                action: "merge (kill or wait for session completion first)",
            });
        }
        let proj = self.store.get_project(&ses.project_id).await?;
        merge_session(
            Path::new(&proj.repo_path),
            Path::new(&ses.worktree_path),
            &ses.branch_name,
            &proj.default_branch,
            params.strategy,
            params.message.as_deref(),
        )
        .await?;
        let _ = self
            .store
            .insert_audit(AuditEntry {
                id: None,
                session_id: Some(ses.id.clone()),
                kind: "merge".to_string(),
                detail: format!(
                    "merged {} into {} ({})",
                    ses.branch_name,
                    proj.default_branch,
                    match params.strategy {
                        MergeStrategy::Squash => "squash",
                        MergeStrategy::Merge => "merge",
                    }
                ),
                timestamp: now_ms(),
            })
            .await;
        Ok(ses)
    }

    /// Membuang sesi beserta worktree dan branch-nya secara idempoten.
    pub async fn discard_session(&self, session_id: &str) -> Result<(), ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        self.proxy_registry.revoke_session(&ses.id).await;
        if let Ok(handle) = self.pty_pool.get(&ses.id).await {
            let _ = handle.kill().await;
        }
        let proj = self.store.get_project(&ses.project_id).await?;
        remove_worktree(
            Path::new(&proj.repo_path),
            Path::new(&ses.worktree_path),
            &ses.branch_name,
        )
        .await?;
        self.store.delete_session(&ses.id).await?;
        let _ = self
            .store
            .insert_audit(AuditEntry {
                id: None,
                session_id: Some(ses.id),
                kind: "discard".to_string(),
                detail: format!("discarded worktree and branch {}", ses.branch_name),
                timestamp: now_ms(),
            })
            .await;
        Ok(())
    }

    /// Menjalankan pipeline DAG `workflow.toml` secara asinkron dan menyimpan state ke SQLite (T-402..T-408).
    pub async fn run_pipeline(
        &self,
        params: PipelineRunParams,
    ) -> Result<PipelineRun, ManagerError> {
        const MAX_WORKFLOW_BYTES: usize = 256 * 1024;
        let toml_src = match (params.workflow_toml, params.workflow_path) {
            (Some(raw), _) if !raw.trim().is_empty() => raw,
            (_, Some(path)) if !path.trim().is_empty() => {
                tokio::fs::read_to_string(&path).await.map_err(|e| {
                    ManagerError::Config(format!("failed reading workflow_path '{path}': {e}"))
                })?
            }
            _ => {
                return Err(ManagerError::Config(
                    "either 'workflow_toml' or 'workflow_path' must be provided".to_string(),
                ));
            }
        };
        if toml_src.len() > MAX_WORKFLOW_BYTES {
            return Err(ManagerError::Config(format!(
                "workflow.toml size {} exceeds 256 KiB limit",
                toml_src.len()
            )));
        }

        let (spec, _order) = parse_and_validate_workflow(&toml_src)?;
        let project = self.store.get_project(&params.project).await?;
        let pipeline_id = new_id();
        let initial_state = DagRuntimeState {
            workflow_name: spec.workflow.name.clone(),
            status: PipelineStatus::Running,
            total_cost_usd: 0.0,
            reason: None,
            steps: HashMap::new(),
        };
        let initial_json =
            serde_json::to_string(&initial_state).unwrap_or_else(|_| "{}".to_string());
        let now = now_ms();
        let run = PipelineRun {
            id: pipeline_id.clone(),
            project_id: project.id.clone(),
            workflow_name: spec.workflow.name.clone(),
            dag_state_json: initial_json,
            status: PipelineStatus::Running,
            created_at: now,
            updated_at: Some(now),
            task: Some(params.task.clone()),
            total_cost_usd: 0.0,
        };
        let saved = self.store.insert_pipeline_run(run).await?;
        let _ = self.events_tx.send(DaemonEvent::PipelineUpdated {
            pipeline_id: saved.id.clone(),
            status: saved.status,
        });

        let mgr = self.clone();
        let run_id = saved.id.clone();
        let task_text = params.task;
        let base_ref = spec.workflow.base_ref.clone();
        let handle = tokio::spawn(async move {
            let runner = crate::pipeline_runner::DaemonStepRunner::new(
                mgr.clone(),
                project.clone(),
                run_id.clone(),
                base_ref,
            );
            let final_state = match execute_workflow(
                &spec,
                &task_text,
                &project.name,
                &project.default_branch,
                &runner,
            )
            .await
            {
                Ok(st) => st,
                Err(err) => DagRuntimeState {
                    workflow_name: spec.workflow.name.clone(),
                    status: PipelineStatus::Failed,
                    total_cost_usd: 0.0,
                    reason: Some(err.to_string()),
                    steps: HashMap::new(),
                },
            };

            let state_json =
                serde_json::to_string(&final_state).unwrap_or_else(|_| "{}".to_string());
            let _ = mgr
                .store
                .update_pipeline_run(
                    &run_id,
                    final_state.status,
                    state_json,
                    final_state.total_cost_usd,
                )
                .await;
            mgr.pipeline_tasks.lock().await.remove(&run_id);
            let _ = mgr.events_tx.send(DaemonEvent::PipelineUpdated {
                pipeline_id: run_id,
                status: final_state.status,
            });
        });

        self.pipeline_tasks
            .lock()
            .await
            .insert(saved.id.clone(), handle);

        Ok(saved)
    }

    /// Mengambil daftar pipeline dengan filter proyek opsional.
    pub async fn list_pipelines(
        &self,
        params: PipelineListParams,
    ) -> Result<Vec<PipelineRun>, ManagerError> {
        let project_id = match params.project {
            Some(ref key) => Some(self.store.get_project(key).await?.id),
            None => None,
        };
        Ok(self.store.list_pipeline_runs(project_id).await?)
    }

    /// Mengambil detail satu pipeline berdasarkan ID atau awalan unik.
    pub async fn get_pipeline(&self, pipeline_id: &str) -> Result<PipelineRun, ManagerError> {
        Ok(self.store.get_pipeline_run(pipeline_id).await?)
    }

    /// Membatalkan pipeline yang sedang berjalan beserta sesi turunannya.
    pub async fn cancel_pipeline(&self, pipeline_id: &str) -> Result<PipelineRun, ManagerError> {
        let run = self.store.get_pipeline_run(pipeline_id).await?;
        if let Some(handle) = self.pipeline_tasks.lock().await.remove(&run.id) {
            handle.abort();
        }
        // Hentikan seluruh sesi aktif yang terikat pada pipeline ini.
        if let Ok(sessions) = self
            .store
            .list_sessions(Some(run.project_id.clone()), None)
            .await
        {
            for ses in sessions {
                if ses.pipeline_run_id.as_deref() == Some(run.id.as_str()) && ses.status.is_active()
                {
                    let _ = self.kill_session(&ses.id).await;
                }
            }
        }
        let mut state: DagRuntimeState =
            serde_json::from_str(&run.dag_state_json).unwrap_or_else(|_| DagRuntimeState {
                workflow_name: run.workflow_name.clone(),
                status: PipelineStatus::Cancelled,
                total_cost_usd: run.total_cost_usd,
                reason: None,
                steps: HashMap::new(),
            });
        state.status = PipelineStatus::Cancelled;
        state.reason = Some("cancelled by user".to_string());
        let state_json = serde_json::to_string(&state).unwrap_or(run.dag_state_json);
        let updated = self
            .store
            .update_pipeline_run(
                &run.id,
                PipelineStatus::Cancelled,
                state_json,
                state.total_cost_usd,
            )
            .await?;
        let _ = self.events_tx.send(DaemonEvent::PipelineUpdated {
            pipeline_id: updated.id.clone(),
            status: updated.status,
        });
        Ok(updated)
    }

    /// Memilih pemenang arena pada langkah `fanout` secara manual (`API.md` §4.5, `T-406`).
    pub async fn select_pipeline_arena_winner(
        &self,
        params: PipelineArenaSelectParams,
    ) -> Result<PipelineRun, ManagerError> {
        let run = self.store.get_pipeline_run(&params.pipeline_id).await?;
        let mut state: DagRuntimeState = serde_json::from_str(&run.dag_state_json)
            .map_err(|e| ManagerError::Config(format!("invalid dag_state_json: {e}")))?;
        let step = state.steps.get_mut(&params.step_id).ok_or_else(|| {
            ManagerError::Config(format!(
                "step '{}' not found in pipeline '{}'",
                params.step_id, run.id
            ))
        })?;
        let matched = step
            .candidates
            .iter()
            .find(|c| {
                c.session_id.as_deref() == Some(params.session_id.as_str())
                    || c.profile == params.session_id
            })
            .cloned()
            .ok_or_else(|| {
                ManagerError::Config(format!(
                    "candidate '{}' not found in step '{}'",
                    params.session_id, params.step_id
                ))
            })?;
        step.winner = Some(matched.profile);
        let state_json = serde_json::to_string(&state)
            .map_err(|e| ManagerError::Config(format!("failed serializing dag_state_json: {e}")))?;
        let updated = self
            .store
            .update_pipeline_run(&run.id, run.status, state_json, run.total_cost_usd)
            .await?;
        Ok(updated)
    }

    /// Menjalankan pembersihan kebijakan retensi SQLite serta direktori `tmp/` dan log PTY lama (`T-411`).
    pub async fn run_gc(&self, dry_run: bool) -> Result<GcReport, ManagerError> {
        const DAY_MS: i64 = 86_400_000;
        let now = now_ms();
        let telemetry_cutoff =
            now.saturating_sub(i64::from(self.config.retention.telemetry_days) * DAY_MS);
        let audit_cutoff = now.saturating_sub(i64::from(self.config.retention.audit_days) * DAY_MS);

        let (telemetry_deleted, audit_deleted) = self
            .store
            .prune_retention(telemetry_cutoff, audit_cutoff, dry_run)
            .await?;

        let active_ids: HashSet<String> = self
            .store
            .list_sessions(None, None)
            .await?
            .into_iter()
            .filter(|s| s.status.is_active())
            .map(|s| s.id)
            .collect();

        let mut tmp_dirs_removed = 0u64;
        let mut log_files_removed = 0u64;
        let mut bytes_freed = 0u64;

        let tmp_root = self.paths.tmp_dir();
        if tmp_root.exists() {
            if let Ok(mut entries) = tokio::fs::read_dir(&tmp_root).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if !active_ids.contains(&name) {
                        let path = entry.path();
                        bytes_freed = bytes_freed.saturating_add(dir_size_bytes(&path));
                        tmp_dirs_removed += 1;
                        if !dry_run {
                            let _ = tokio::fs::remove_dir_all(&path).await;
                        }
                    }
                }
            }
        }

        let logs_root = self.paths.logs_dir();
        if logs_root.exists() {
            if let Ok(mut entries) = tokio::fs::read_dir(&logs_root).await {
                while let Ok(Some(entry)) = entries.next_entry().await {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if let Some(rest) = name.strip_prefix("pty-") {
                        let sid = rest.split('.').next().unwrap_or("");
                        if !sid.is_empty() && !active_ids.contains(sid) {
                            let path = entry.path();
                            if let Ok(meta) = entry.metadata().await {
                                bytes_freed = bytes_freed.saturating_add(meta.len());
                            }
                            log_files_removed += 1;
                            if !dry_run {
                                let _ = tokio::fs::remove_file(&path).await;
                            }
                        }
                    }
                }
            }
        }

        if !dry_run {
            let _ = self
                .store
                .insert_audit(AuditEntry {
                    id: None,
                    session_id: None,
                    kind: "gc".to_string(),
                    detail: format!(
                        "gc pruned telemetry={telemetry_deleted} audit={audit_deleted} tmp_dirs={tmp_dirs_removed} logs={log_files_removed} bytes={bytes_freed}"
                    ),
                    timestamp: now_ms(),
                })
                .await;
        }

        Ok(GcReport {
            dry_run,
            telemetry_deleted,
            audit_deleted,
            tmp_dirs_removed,
            log_files_removed,
            bytes_freed,
        })
    }
}

fn dir_size_bytes(path: &Path) -> u64 {
    let Ok(meta) = std::fs::metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    let mut total = 0u64;
    for entry in entries.flatten() {
        total = total.saturating_add(dir_size_bytes(&entry.path()));
    }
    total
}
