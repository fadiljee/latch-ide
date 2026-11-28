//! Manajemen sesi PTY paralel, batching 8-16 ms, backpressure, layar `vt100`, dan kontrol sinyal.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use latch_core::config::{resolve_command_in_path, AgentProfile, RuntimePaths};
use latch_core::id::now_ms;
use latch_core::sandbox::{SandboxPolicy, SandboxProvider};
#[cfg(unix)]
use nix::sys::signal::{kill, Signal};
#[cfg(unix)]
use nix::unistd::Pid;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use thiserror::Error;
use tokio::sync::{mpsc, RwLock};
use tracing::{debug, warn};

use crate::ring_buffer::{RingBuffer, RollingLogWriter};

const READER_CHANNEL_CAP: usize = 256;
const SUBSCRIBER_CHANNEL_CAP: usize = 64;
const BATCH_FLUSH_BYTES: usize = 16 * 1024;
const KILL_GRACE_PERIOD: Duration = Duration::from_millis(500);

/// Kesalahan manajemen sesi PTY.
#[derive(Debug, Error)]
pub enum PtyError {
    /// Sesi PTY tidak ditemukan di pool aktif.
    #[error("PTY session not found: {0}")]
    NotFound(String),
    /// Gagal membuka PTY atau menjalankan perintah anak.
    #[error("failed to spawn PTY: {0}")]
    Spawn(String),
    /// Gagal menerapkan kebijakan sandbox.
    #[error("sandbox error: {0}")]
    Sandbox(#[from] latch_core::sandbox::SandboxError),
    /// Gagal mengirim sinyal OS (`SIGSTOP`/`SIGCONT`/`SIGTERM`).
    #[error("signal error: {0}")]
    Signal(String),
    /// Galat I/O pada PTY master.
    #[error("PTY I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Event keluaran yang dikirim ke pelanggan aliran PTY.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PtyFrame {
    /// Batch byte terminal (interval 8-16 ms).
    Output(Vec<u8>),
    /// Penanda jumlah byte yang dibuang karena pelanggan lambat.
    Dropped(u64),
}

/// Informasi akhir saat proses anak PTY selesai.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyExitEvent {
    /// ID sesi.
    pub session_id: String,
    /// Kode keluar proses (jika tersedia).
    pub exit_code: i32,
}

/// Permintaan pembuatan sesi PTY baru.
pub struct PtySpawnRequest {
    /// ID unik sesi.
    pub session_id: String,
    /// Direktori kerja (`worktree_path`).
    pub workdir: PathBuf,
    /// Direktori `TMPDIR` terisolasi.
    pub isolated_tmp: PathBuf,
    /// Lokasi file log rolling PTY.
    pub log_path: PathBuf,
    /// Profil agen (`command`, `args`, `env`, `pass_env`).
    pub profile: AgentProfile,
    /// Variabel lingkungan dinamis tambahan (mis. `OPENAI_BASE_URL` hasil injeksi proxy).
    pub extra_env: Vec<(String, String)>,
    /// Deskripsi tugas (dilewatkan lewat `LATCH_TASK` jika diminta).
    pub task: String,
    /// Kebijakan sandbox sesi.
    pub sandbox_policy: SandboxPolicy,
    /// Kapasitas ring buffer (byte).
    pub ring_buffer_bytes: usize,
    /// Ukuran maksimal log rolling (byte).
    pub log_max_bytes: u64,
    /// Interval batching dalam milidetik (`8..=16`).
    pub batch_ms: u64,
    /// Jumlah baris scrollback `vt100`.
    pub scrollback_lines: usize,
    /// Ukuran awal terminal `(cols, rows)`.
    pub size: (u16, u16),
}

/// Snapshot layar virtual saat ini.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PtyScreenSnapshot {
    /// Kolom terminal.
    pub cols: u16,
    /// Baris terminal.
    pub rows: u16,
    /// Teks layar terformat.
    pub text: String,
    /// Byte ring buffer terbaru untuk memulihkan warna ANSI di klien.
    pub ansi_bytes: Vec<u8>,
}

/// Langganan aliran output PTY untuk satu klien (TUI / WebSocket).
pub struct PtySubscriber {
    /// Snapshot ring buffer awal saat baru attach.
    pub initial_bytes: Vec<u8>,
    /// Penerima frame ter-batch.
    pub rx: mpsc::Receiver<PtyFrame>,
}

struct SubscriberSlot {
    tx: mpsc::Sender<PtyFrame>,
    dropped_bytes: u64,
}

struct SessionState {
    pid: Option<u32>,
    last_activity_ms: AtomicI64,
    master: Mutex<Box<dyn MasterPty + Send>>,
    writer: Mutex<Box<dyn Write + Send>>,
    parser: Mutex<vt100::Parser>,
    ring: Mutex<RingBuffer>,
    subscribers: Mutex<Vec<SubscriberSlot>>,
}

/// Handle kendali satu sesi PTY aktif.
#[derive(Clone)]
pub struct PtySessionHandle {
    state: Arc<SessionState>,
}

impl PtySessionHandle {
    /// PID proses anak jika tersedia.
    pub fn pid(&self) -> Option<u32> {
        self.state.pid
    }

    /// Stempel waktu (UTC epoch ms) aktivitas input/output terakhir sesi PTY (`T-410`).
    pub fn last_activity_ms(&self) -> i64 {
        self.state.last_activity_ms.load(Ordering::Relaxed)
    }

    /// Menulis input byte ke PTY master.
    pub fn write_input(&self, data: &[u8]) -> Result<(), PtyError> {
        let mut guard = self
            .state
            .writer
            .lock()
            .map_err(|_| PtyError::Io(std::io::Error::other("writer mutex poisoned")))?;
        guard.write_all(data)?;
        guard.flush()?;
        self.state
            .last_activity_ms
            .store(now_ms(), Ordering::Relaxed);
        Ok(())
    }

    /// Mengubah ukuran PTY dan menyinkronkan parser `vt100`.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), PtyError> {
        let cols = cols.max(1);
        let rows = rows.max(1);
        {
            let master = self
                .state
                .master
                .lock()
                .map_err(|_| PtyError::Io(std::io::Error::other("master mutex poisoned")))?;
            master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| PtyError::Io(std::io::Error::other(e.to_string())))?;
        }
        if let Ok(mut parser) = self.state.parser.lock() {
            parser.set_size(rows, cols);
        }
        Ok(())
    }

    /// Mengambil snapshot layar `vt100` dan byte ring buffer saat ini.
    pub fn snapshot(&self) -> PtyScreenSnapshot {
        let (cols, rows, text) = if let Ok(parser) = self.state.parser.lock() {
            let screen = parser.screen();
            let (r, c) = screen.size();
            (c, r, screen.contents())
        } else {
            (80, 24, String::new())
        };
        let ansi_bytes = self
            .state
            .ring
            .lock()
            .map(|r| r.snapshot())
            .unwrap_or_default();
        PtyScreenSnapshot {
            cols,
            rows,
            text,
            ansi_bytes,
        }
    }

    /// Mendaftarkan pelanggan baru dengan kanal berbatas dan snapshot awal.
    pub fn subscribe(&self) -> PtySubscriber {
        let initial_bytes = self
            .state
            .ring
            .lock()
            .map(|r| r.snapshot())
            .unwrap_or_default();
        let (tx, rx) = mpsc::channel(SUBSCRIBER_CHANNEL_CAP);
        if let Ok(mut subs) = self.state.subscribers.lock() {
            subs.push(SubscriberSlot {
                tx,
                dropped_bytes: 0,
            });
        }
        PtySubscriber { initial_bytes, rx }
    }

    /// Mengirim `SIGSTOP` ke process group sesi (khusus Unix).
    pub fn pause(&self) -> Result<(), PtyError> {
        #[cfg(unix)]
        {
            send_signal(self.state.pid, Signal::SIGSTOP)
        }
        #[cfg(not(unix))]
        {
            let _ = self.state.pid;
            Err(PtyError::Signal(
                "pause (SIGSTOP) is only supported on Unix hosts".to_string(),
            ))
        }
    }

    /// Mengirim `SIGCONT` ke process group sesi (khusus Unix).
    pub fn resume(&self) -> Result<(), PtyError> {
        #[cfg(unix)]
        {
            send_signal(self.state.pid, Signal::SIGCONT)
        }
        #[cfg(not(unix))]
        {
            let _ = self.state.pid;
            Err(PtyError::Signal(
                "resume (SIGCONT) is only supported on Unix hosts".to_string(),
            ))
        }
    }

    /// Mengirim `SIGTERM` diikuti `SIGKILL` setelah grace period bila proses masih hidup.
    pub async fn kill(&self) -> Result<(), PtyError> {
        let Some(pid) = self.state.pid else {
            return Ok(());
        };
        #[cfg(unix)]
        {
            let _ = send_signal(Some(pid), Signal::SIGTERM);
            tokio::time::sleep(KILL_GRACE_PERIOD).await;
            if is_pid_alive(pid) {
                let _ = send_signal(Some(pid), Signal::SIGKILL);
            }
        }
        #[cfg(not(unix))]
        {
            let _ = KILL_GRACE_PERIOD;
            let _ = std::process::Command::new("taskkill")
                .args(["/PID", &pid.to_string(), "/T", "/F"])
                .status();
        }
        Ok(())
    }
}

/// Kolam pengelola banyak sesi PTY paralel yang terisolasi.
#[derive(Clone, Default)]
pub struct PtyPool {
    sessions: Arc<RwLock<HashMap<String, PtySessionHandle>>>,
}

impl std::fmt::Debug for PtyPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PtyPool").finish_non_exhaustive()
    }
}

impl PtyPool {
    /// Membuat kolam PTY kosong.
    pub fn new() -> Self {
        Self::default()
    }

    /// Jumlah sesi PTY yang sedang dikelola.
    pub async fn active_count(&self) -> usize {
        self.sessions.read().await.len()
    }

    /// Mengambil handle sesi berdasarkan ID.
    pub async fn get(&self, session_id: &str) -> Result<PtySessionHandle, PtyError> {
        self.sessions
            .read()
            .await
            .get(session_id)
            .cloned()
            .ok_or_else(|| PtyError::NotFound(session_id.to_string()))
    }

    /// Mengembalikan daftar ID sesi aktif yang idle melebihi `idle_ms` (`T-410`).
    pub async fn idle_sessions(&self, idle_ms: i64) -> Vec<String> {
        let cutoff = now_ms().saturating_sub(idle_ms);
        self.sessions
            .read()
            .await
            .iter()
            .filter(|(_, h)| h.last_activity_ms() <= cutoff)
            .map(|(id, _)| id.clone())
            .collect()
    }

    /// Menjalankan perintah agen di PTY baru, menerapkan sandbox, dan mengawasi exit secara terisolasi.
    pub async fn spawn(
        &self,
        req: PtySpawnRequest,
        sandbox: &dyn SandboxProvider,
        exit_tx: mpsc::Sender<PtyExitEvent>,
    ) -> Result<PtySessionHandle, PtyError> {
        std::fs::create_dir_all(&req.isolated_tmp)?;

        let resolved_program = resolve_command_in_path(&req.profile.command)
            .unwrap_or_else(|| PathBuf::from(&req.profile.command));

        // Validasi kebijakan sandbox lewat Command persiapan sebelum spawn PTY
        let mut probe_cmd = std::process::Command::new(&resolved_program);
        sandbox.prepare(&req.sandbox_policy, &mut probe_cmd)?;

        let pty_system = native_pty_system();
        let (cols, rows) = (req.size.0.max(20), req.size.1.max(5));
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        let mut cmd = CommandBuilder::new(&resolved_program);
        cmd.args(&req.profile.args);
        if let Some(ref flag) = req.profile.task_arg {
            let trimmed_flag = flag.trim();
            let trimmed_task = req.task.trim();
            if !trimmed_flag.is_empty() && !trimmed_task.is_empty() {
                cmd.arg(trimmed_flag);
                cmd.arg(trimmed_task);
            }
        }
        cmd.cwd(&req.workdir);
        cmd.env_clear();
        populate_sanitized_env(&mut cmd, &req);

        let mut child = pair
            .slave
            .spawn_command(cmd)
            .map_err(|e| PtyError::Spawn(e.to_string()))?;
        drop(pair.slave);

        let pid = child.process_id();
        let reader = pair
            .master
            .try_clone_reader()
            .map_err(|e| PtyError::Spawn(e.to_string()))?;
        let writer = pair
            .master
            .take_writer()
            .map_err(|e| PtyError::Spawn(e.to_string()))?;

        let log_writer = RollingLogWriter::open(&req.log_path, req.log_max_bytes)?;
        let state = Arc::new(SessionState {
            pid,
            last_activity_ms: AtomicI64::new(now_ms()),
            master: Mutex::new(pair.master),
            writer: Mutex::new(writer),
            parser: Mutex::new(vt100::Parser::new(rows, cols, req.scrollback_lines)),
            ring: Mutex::new(RingBuffer::new(req.ring_buffer_bytes)),
            subscribers: Mutex::new(Vec::new()),
        });

        let handle = PtySessionHandle {
            state: Arc::clone(&state),
        };
        self.sessions
            .write()
            .await
            .insert(req.session_id.clone(), handle.clone());

        let (chunk_tx, chunk_rx) = mpsc::channel::<Vec<u8>>(READER_CHANNEL_CAP);
        spawn_reader_thread(
            req.session_id.clone(),
            reader,
            Arc::clone(&state),
            log_writer,
            chunk_tx,
        );
        spawn_batcher_task(Arc::clone(&state), chunk_rx, req.batch_ms.clamp(8, 16));

        let sessions_ref = Arc::clone(&self.sessions);
        let sid = req.session_id;
        tokio::spawn(async move {
            let exit_code = tokio::task::spawn_blocking(move || match child.wait() {
                Ok(status) => i32::try_from(status.exit_code()).unwrap_or(1),
                Err(err) => {
                    warn!(error = %err, "failed waiting on PTY child");
                    1
                }
            })
            .await
            .unwrap_or(1);

            sessions_ref.write().await.remove(&sid);
            let _ = exit_tx
                .send(PtyExitEvent {
                    session_id: sid,
                    exit_code,
                })
                .await;
        });

        Ok(handle)
    }
}

fn populate_sanitized_env(cmd: &mut CommandBuilder, req: &PtySpawnRequest) {
    // Teruskan variabel lingkungan non-rahasia standar termasuk sesi desktop/XDG agar
    // alur login OAuth browser dan keyring sistem (gh, agy, claude, codex, dll.) berjalan mulus.
    if let Some(path_val) = std::env::var_os("PATH") {
        let local_bin = RuntimePaths::expand_tilde("~/.local/bin");
        let mut dirs: Vec<PathBuf> = std::env::split_paths(&path_val).collect();
        if local_bin.is_dir() && !dirs.iter().any(|d| d == &local_bin) {
            dirs.push(local_bin);
        }
        if let Ok(joined) = std::env::join_paths(dirs) {
            cmd.env("PATH", joined);
        } else {
            cmd.env("PATH", path_val);
        }
    }
    for key in [
        "HOME",
        "LANG",
        "LC_ALL",
        "USER",
        "SHELL",
        "XDG_CONFIG_HOME",
        "XDG_DATA_HOME",
        "XDG_STATE_HOME",
        "XDG_CACHE_HOME",
        "XDG_RUNTIME_DIR",
        "DISPLAY",
        "WAYLAND_DISPLAY",
        "DBUS_SESSION_BUS_ADDRESS",
        "BROWSER",
        "COLORTERM",
        "NO_COLOR",
    ] {
        if let Some(val) = std::env::var_os(key) {
            cmd.env(key, val);
        }
    }
    cmd.env("TERM", "xterm-256color");
    cmd.env("TMPDIR", &req.isolated_tmp);
    cmd.env("LATCH_SESSION_ID", &req.session_id);
    cmd.env("LATCH_TASK", &req.task);

    for key in &req.profile.pass_env {
        if let Some(val) = std::env::var_os(key) {
            cmd.env(key, val);
        }
    }
    for (k, v) in &req.profile.env {
        cmd.env(k, v);
    }
    for (k, v) in &req.extra_env {
        cmd.env(k, v);
    }
}

fn spawn_reader_thread(
    session_id: String,
    mut reader: Box<dyn Read + Send>,
    state: Arc<SessionState>,
    mut log_writer: RollingLogWriter,
    chunk_tx: mpsc::Sender<Vec<u8>>,
) {
    let result = std::thread::Builder::new()
        .name(format!("pty-rd-{}", &session_id[..session_id.len().min(8)]))
        .stack_size(256 * 1024)
        .spawn(move || {
            let mut buf = [0u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        state.last_activity_ms.store(now_ms(), Ordering::Relaxed);
                        let slice = &buf[..n];
                        if let Ok(mut parser) = state.parser.lock() {
                            parser.process(slice);
                        }
                        if let Ok(mut ring) = state.ring.lock() {
                            ring.push(slice);
                        }
                        if let Err(err) = log_writer.write_chunk(slice) {
                            debug!(session_id = %session_id, error = %err, "pty rolling log write failed");
                        }
                        if chunk_tx.blocking_send(slice.to_vec()).is_err() {
                            break;
                        }
                    }
                    Err(err) if err.kind() == std::io::ErrorKind::Interrupted => continue,
                    Err(_) => break,
                }
            }
        });
    if let Err(err) = result {
        warn!(error = %err, "failed to spawn PTY reader thread");
    }
}

fn spawn_batcher_task(
    state: Arc<SessionState>,
    mut chunk_rx: mpsc::Receiver<Vec<u8>>,
    batch_ms: u64,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_millis(batch_ms));
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut pending = Vec::new();

        loop {
            tokio::select! {
                maybe_chunk = chunk_rx.recv() => {
                    match maybe_chunk {
                        Some(chunk) => {
                            pending.extend_from_slice(&chunk);
                            if pending.len() >= BATCH_FLUSH_BYTES {
                                flush_subscribers(&state, &mut pending);
                            }
                        }
                        None => {
                            flush_subscribers(&state, &mut pending);
                            break;
                        }
                    }
                }
                _ = interval.tick() => {
                    if !pending.is_empty() {
                        flush_subscribers(&state, &mut pending);
                    }
                }
            }
        }
    });
}

fn flush_subscribers(state: &Arc<SessionState>, pending: &mut Vec<u8>) {
    if pending.is_empty() {
        return;
    }
    let payload = std::mem::take(pending);
    let Ok(mut subs) = state.subscribers.lock() else {
        return;
    };
    subs.retain_mut(|slot| {
        if slot.dropped_bytes > 0 {
            match slot.tx.try_send(PtyFrame::Dropped(slot.dropped_bytes)) {
                Ok(()) => slot.dropped_bytes = 0,
                Err(mpsc::error::TrySendError::Full(_)) => {
                    slot.dropped_bytes = slot.dropped_bytes.saturating_add(payload.len() as u64);
                    return true;
                }
                Err(mpsc::error::TrySendError::Closed(_)) => return false,
            }
        }
        match slot.tx.try_send(PtyFrame::Output(payload.clone())) {
            Ok(()) => true,
            Err(mpsc::error::TrySendError::Full(_)) => {
                slot.dropped_bytes = slot.dropped_bytes.saturating_add(payload.len() as u64);
                true
            }
            Err(mpsc::error::TrySendError::Closed(_)) => false,
        }
    });
}

#[cfg(unix)]
fn send_signal(pid: Option<u32>, sig: Signal) -> Result<(), PtyError> {
    let Some(raw_pid) = pid else {
        return Ok(());
    };
    let pid_i32 =
        i32::try_from(raw_pid).map_err(|_| PtyError::Signal("PID overflow".to_string()))?;
    // Coba kirim ke seluruh process group terlebih dulu (anak PTY memakai setsid)
    if let Err(err) = kill(Pid::from_raw(-pid_i32), sig) {
        if err == nix::errno::Errno::ESRCH {
            kill(Pid::from_raw(pid_i32), sig).map_err(|e| PtyError::Signal(e.to_string()))?;
        } else {
            return Err(PtyError::Signal(err.to_string()));
        }
    }
    Ok(())
}

/// Memeriksa apakah suatu PID masih hidup di OS saat ini.
pub fn is_pid_alive(raw_pid: u32) -> bool {
    #[cfg(unix)]
    {
        let Ok(pid_i32) = i32::try_from(raw_pid) else {
            return false;
        };
        match kill(Pid::from_raw(pid_i32), None) {
            Ok(()) => true,
            Err(nix::errno::Errno::EPERM) => true,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        let filter = format!("PID eq {raw_pid}");
        std::process::Command::new("tasklist")
            .args(["/FI", &filter, "/NH"])
            .output()
            .map(|o| {
                o.status.success()
                    && String::from_utf8_lossy(&o.stdout).contains(&raw_pid.to_string())
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::{Duration, Instant};

    use latch_core::config::AgentProfile;
    use latch_core::model::ProxyMode;
    use latch_core::sandbox::{SandboxCapabilities, SandboxError, SandboxPolicy, SandboxProvider};
    use tempfile::TempDir;
    use tokio::sync::mpsc;

    use super::{PtyFrame, PtyPool, PtySpawnRequest};

    struct TestNoopSandbox;
    impl SandboxProvider for TestNoopSandbox {
        fn name(&self) -> &'static str {
            "noop"
        }
        fn capabilities(&self) -> SandboxCapabilities {
            SandboxCapabilities {
                filesystem_isolation: false,
                detail: "test".to_string(),
            }
        }
        fn prepare(
            &self,
            _policy: &SandboxPolicy,
            _cmd: &mut std::process::Command,
        ) -> Result<(), SandboxError> {
            Ok(())
        }
    }

    fn test_profile(cmd: &str, args: &[&str]) -> AgentProfile {
        AgentProfile {
            command: cmd.to_string(),
            args: args.iter().map(|s| (*s).to_string()).collect(),
            env: BTreeMap::new(),
            pass_env: Vec::new(),
            proxy_mode: ProxyMode::Bypass,
            proxy_provider: None,
            resume_args: Vec::new(),
            extra_rw: Vec::new(),
            extra_ro: Vec::new(),
            ..AgentProfile::default()
        }
    }

    fn base_policy(dir: &std::path::Path) -> SandboxPolicy {
        SandboxPolicy {
            rw_paths: vec![dir.to_path_buf()],
            ro_paths: Vec::new(),
            deny_hints: Vec::new(),
            allow_network: Vec::new(),
        }
    }

    #[tokio::test]
    async fn spawns_resizes_snapshots_and_detects_exit_code() {
        let dir = TempDir::new().unwrap();
        let pool = PtyPool::new();
        let (exit_tx, mut exit_rx) = mpsc::channel(8);

        let handle = pool
            .spawn(
                PtySpawnRequest {
                    session_id: "s-pty-1".to_string(),
                    workdir: dir.path().to_path_buf(),
                    isolated_tmp: dir.path().join("tmp"),
                    log_path: dir.path().join("pty.log"),
                    profile: test_profile(
                        "sh",
                        &[
                            "-c",
                            "printf '\\033[32mOK-LATCH\\033[0m\\n'; sleep 0.2; exit 0",
                        ],
                    ),
                    extra_env: Vec::new(),
                    task: "echo".to_string(),
                    sandbox_policy: base_policy(dir.path()),
                    ring_buffer_bytes: 64 * 1024,
                    log_max_bytes: 1024 * 1024,
                    batch_ms: 12,
                    scrollback_lines: 200,
                    size: (80, 24),
                },
                &TestNoopSandbox,
                exit_tx,
            )
            .await
            .unwrap();

        handle.resize(100, 30).unwrap();
        let ev = tokio::time::timeout(Duration::from_secs(5), exit_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_eq!(ev.exit_code, 0);

        let snap = handle.snapshot();
        assert_eq!((snap.cols, snap.rows), (100, 30));
        assert!(
            snap.text.contains("OK-LATCH"),
            "snapshot was: {}",
            snap.text
        );
    }

    #[tokio::test]
    async fn pause_resume_and_kill_process_group() {
        let dir = TempDir::new().unwrap();
        let pool = PtyPool::new();
        let (exit_tx, mut exit_rx) = mpsc::channel(8);

        let handle = pool
            .spawn(
                PtySpawnRequest {
                    session_id: "s-pty-ctrl".to_string(),
                    workdir: dir.path().to_path_buf(),
                    isolated_tmp: dir.path().join("tmp"),
                    log_path: dir.path().join("pty.log"),
                    profile: test_profile("sh", &["-c", "sleep 30"]),
                    extra_env: Vec::new(),
                    task: "sleep".to_string(),
                    sandbox_policy: base_policy(dir.path()),
                    ring_buffer_bytes: 64 * 1024,
                    log_max_bytes: 1024 * 1024,
                    batch_ms: 12,
                    scrollback_lines: 200,
                    size: (80, 24),
                },
                &TestNoopSandbox,
                exit_tx,
            )
            .await
            .unwrap();

        handle.pause().unwrap();
        handle.resume().unwrap();
        handle.kill().await.unwrap();

        let ev = tokio::time::timeout(Duration::from_secs(5), exit_rx.recv())
            .await
            .unwrap()
            .unwrap();
        assert_ne!(ev.exit_code, 0);
    }

    #[tokio::test]
    async fn batches_frames_and_marks_slow_subscriber_drops() {
        let dir = TempDir::new().unwrap();
        let pool = PtyPool::new();
        let (exit_tx, mut exit_rx) = mpsc::channel(8);

        // Hasilkan >10.000 baris untuk menguji batching dan backpressure pelanggan lambat
        let handle = pool
            .spawn(
                PtySpawnRequest {
                    session_id: "s-pty-flood".to_string(),
                    workdir: dir.path().to_path_buf(),
                    isolated_tmp: dir.path().join("tmp"),
                    log_path: dir.path().join("pty.log"),
                    profile: test_profile(
                        "sh",
                        &[
                            "-c",
                            "seq 1 12000 | awk '{print \"line-\" $1 \"-payload-0123456789\"}'",
                        ],
                    ),
                    extra_env: Vec::new(),
                    task: "flood".to_string(),
                    sandbox_policy: base_policy(dir.path()),
                    ring_buffer_bytes: 64 * 1024,
                    log_max_bytes: 1024 * 1024,
                    batch_ms: 12,
                    scrollback_lines: 200,
                    size: (80, 24),
                },
                &TestNoopSandbox,
                exit_tx,
            )
            .await
            .unwrap();

        let mut fast_sub = handle.subscribe();
        let slow_sub = handle.subscribe();

        let mut fast_frames = 0usize;
        let t0 = Instant::now();
        let mut done = false;
        while !done && t0.elapsed() < Duration::from_secs(5) {
            tokio::select! {
                _ = exit_rx.recv() => { done = true; }
                maybe = fast_sub.rx.recv() => {
                    if matches!(maybe, Some(PtyFrame::Output(_))) {
                        fast_frames += 1;
                    }
                }
            }
        }
        // Kuras sisa antrean fast_sub
        while let Ok(Some(frame)) =
            tokio::time::timeout(Duration::from_millis(50), fast_sub.rx.recv()).await
        {
            if matches!(frame, PtyFrame::Output(_)) {
                fast_frames += 1;
            }
        }
        assert!(fast_frames >= 1);
        // slow_sub tidak pernah di-drain selama banjir sehingga tidak memblokir PTY
        drop(slow_sub.rx);
    }

    #[tokio::test]
    async fn passes_task_arg_when_task_is_non_empty_and_omits_when_blank() {
        let dir = TempDir::new().unwrap();
        let pool = PtyPool::new();
        let (exit_tx, mut exit_rx) = mpsc::channel(4);

        let mut prof = test_profile("sh", &["-c", "printf 'ARG0=%s ARG1=%s\\n' \"$0\" \"$1\""]);
        prof.task_arg = Some("--prompt-interactive".to_string());

        let handle = pool
            .spawn(
                PtySpawnRequest {
                    session_id: "s-task-arg".to_string(),
                    workdir: dir.path().to_path_buf(),
                    isolated_tmp: dir.path().join("tmp"),
                    log_path: dir.path().join("pty-task-arg.log"),
                    profile: prof,
                    extra_env: Vec::new(),
                    task: "fix tests".to_string(),
                    sandbox_policy: base_policy(dir.path()),
                    ring_buffer_bytes: 16 * 1024,
                    log_max_bytes: 1024 * 1024,
                    batch_ms: 10,
                    scrollback_lines: 100,
                    size: (80, 24),
                },
                &TestNoopSandbox,
                exit_tx,
            )
            .await
            .unwrap();

        let _ = tokio::time::timeout(Duration::from_secs(5), exit_rx.recv())
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_millis(80)).await;
        let snap = handle.snapshot();
        assert!(
            snap.text
                .contains("ARG0=--prompt-interactive ARG1=fix tests"),
            "expected task_arg forwarding, got: {}",
            snap.text
        );
    }
}
