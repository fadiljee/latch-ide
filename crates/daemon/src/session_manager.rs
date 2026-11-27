//! Pengelola siklus hidup sesi, worktree, PTY, dan rekonsiliasi startup (T-104, T-131).

use std::collections::HashSet;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use latch_core::config::{AgentsConfig, LatchConfig, RuntimePaths};
use latch_core::id::{new_id, now_ms, short_id};
use latch_core::model::{
    AuditEntry, MergeCheckResult, MergeStrategy, Project, ProxyMode, Session, SessionDiff,
    SessionStatus, SessionTelemetrySummary, TelemetryRecord, TripReason,
};
use latch_core::pricing::{load_pricing_table, PricingTable};
use latch_core::protocol::{
    DaemonHealth, ProjectAddParams, ScreenSnapshot, SessionDiffParams, SessionListParams,
    SessionMergeParams, SessionRunParams,
};
use latch_core::sandbox::SandboxProvider;
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
use tokio::sync::{broadcast, mpsc, Notify, RwLock};
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
            exit_tx,
            trip_tx,
            events_tx,
            shutdown,
        };

        manager.reconcile_startup().await?;
        manager.spawn_exit_watcher(exit_rx);
        manager.spawn_telemetry_batch_writer(telemetry_rx);
        manager.spawn_trip_watcher(trip_rx);
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
                    if existing.status.is_terminal() {
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
            pipeline_run_id: None,
            step_id: None,
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

    /// Melanjutkan sesi yang dijeda (`SIGCONT`).
    pub async fn resume_session(&self, session_id: &str) -> Result<Session, ManagerError> {
        let ses = self.store.get_session(session_id).await?;
        if ses.status != SessionStatus::Paused {
            return Err(ManagerError::InvalidState {
                id: ses.id,
                status: ses.status,
                action: "resume",
            });
        }
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
}
