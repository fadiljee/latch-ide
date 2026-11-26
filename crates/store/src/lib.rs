#![forbid(unsafe_code)]
//! Lapisan persistensi SQLite dengan aktor penulis tunggal dan migrasi skema (T-103).

use std::path::Path;
use std::sync::Arc;

use latch_core::id::now_ms;
use latch_core::model::{
    can_transition, AuditEntry, Project, ProxyMode, Session, SessionStatus, TripReason,
};
use latch_core::redact::redact_secrets;
use rusqlite::{params, Connection, OptionalExtension};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

const MIGRATION_0001: &str = include_str!("../migrations/0001_initial.sql");
const STORE_CHANNEL_CAPACITY: usize = 256;

/// Kesalahan operasi basis data SQLite.
#[derive(Debug, Error)]
pub enum StoreError {
    /// Galat dari driver SQLite.
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Galat I/O saat menyiapkan direktori DB.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Pemeriksaan integritas basis data gagal.
    #[error("database integrity check failed: {0}")]
    Integrity(String),
    /// Entitas tidak ditemukan.
    #[error("{entity} not found: {key}")]
    NotFound {
        /// Jenis entitas.
        entity: &'static str,
        /// Kunci pencarian.
        key: String,
    },
    /// Transisi status sesi tidak sah.
    #[error("invalid session status transition: {from} -> {to}")]
    InvalidTransition {
        /// Status asal.
        from: SessionStatus,
        /// Status tujuan.
        to: SessionStatus,
    },
    /// Aktor penyimpanan sudah berhenti.
    #[error("store actor thread is closed")]
    Closed,
}

type Reply<T> = oneshot::Sender<Result<T, StoreError>>;

enum StoreCmd {
    AddProject {
        project: Project,
        reply: Reply<Project>,
    },
    GetProject {
        id_or_name: String,
        reply: Reply<Project>,
    },
    ListProjects {
        reply: Reply<Vec<Project>>,
    },
    RemoveProject {
        id_or_name: String,
        reply: Reply<()>,
    },
    InsertSession {
        session: Session,
        reply: Reply<Session>,
    },
    GetSession {
        id_or_prefix: String,
        reply: Reply<Session>,
    },
    ListSessions {
        project_id: Option<String>,
        status: Option<SessionStatus>,
        reply: Reply<Vec<Session>>,
    },
    TransitionSession {
        session_id: String,
        to: SessionStatus,
        exit_code: Option<i32>,
        trip_reason: Option<TripReason>,
        ended_at: Option<i64>,
        clear_pid: bool,
        reply: Reply<Session>,
    },
    DeleteSession {
        session_id: String,
        reply: Reply<()>,
    },
    InsertAudit {
        entry: AuditEntry,
        reply: Reply<()>,
    },
    ListAudit {
        limit: usize,
        reply: Reply<Vec<AuditEntry>>,
    },
    Shutdown {
        reply: Reply<()>,
    },
}

/// Handle thread-safe asinkron menuju aktor penulis tunggal SQLite.
#[derive(Clone)]
pub struct StoreHandle {
    tx: mpsc::Sender<StoreCmd>,
    _thread_guard: Arc<ThreadGuard>,
}

struct ThreadGuard {
    tx: mpsc::Sender<StoreCmd>,
}

impl Drop for ThreadGuard {
    fn drop(&mut self) {
        let (reply, _) = oneshot::channel();
        let _ = self.tx.try_send(StoreCmd::Shutdown { reply });
    }
}

impl std::fmt::Debug for StoreHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoreHandle").finish_non_exhaustive()
    }
}

impl StoreHandle {
    /// Membuka atau membuat berkas SQLite di `db_path`, menjalankan `integrity_check` dan migrasi.
    pub fn open(db_path: &Path) -> Result<Self, StoreError> {
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(db_path)?;
        Self::from_connection(conn)
    }

    /// Membuka basis data SQLite in-memory (untuk unit test cepat).
    pub fn open_in_memory() -> Result<Self, StoreError> {
        let conn = Connection::open_in_memory()?;
        Self::from_connection(conn)
    }

    fn from_connection(conn: Connection) -> Result<Self, StoreError> {
        configure_and_migrate(&conn)?;
        let (tx, rx) = mpsc::channel(STORE_CHANNEL_CAPACITY);
        let guard_tx = tx.clone();

        std::thread::Builder::new()
            .name("latch-store".to_string())
            .stack_size(512 * 1024)
            .spawn(move || run_actor(conn, rx))?;

        Ok(Self {
            tx,
            _thread_guard: Arc::new(ThreadGuard { tx: guard_tx }),
        })
    }

    async fn request<T>(
        &self,
        make_cmd: impl FnOnce(Reply<T>) -> StoreCmd,
    ) -> Result<T, StoreError> {
        let (reply_tx, reply_rx) = oneshot::channel();
        self.tx
            .send(make_cmd(reply_tx))
            .await
            .map_err(|_| StoreError::Closed)?;
        reply_rx.await.map_err(|_| StoreError::Closed)?
    }

    /// Menyimpan proyek baru.
    pub async fn add_project(&self, project: Project) -> Result<Project, StoreError> {
        self.request(|reply| StoreCmd::AddProject { project, reply })
            .await
    }

    /// Mencari proyek berdasarkan ID atau nama.
    pub async fn get_project(&self, id_or_name: &str) -> Result<Project, StoreError> {
        let key = id_or_name.to_string();
        self.request(|reply| StoreCmd::GetProject {
            id_or_name: key,
            reply,
        })
        .await
    }

    /// Mengambil daftar semua proyek.
    pub async fn list_projects(&self) -> Result<Vec<Project>, StoreError> {
        self.request(|reply| StoreCmd::ListProjects { reply }).await
    }

    /// Menghapus proyek (gagal jika masih ada sesi aktif).
    pub async fn remove_project(&self, id_or_name: &str) -> Result<(), StoreError> {
        let key = id_or_name.to_string();
        self.request(|reply| StoreCmd::RemoveProject {
            id_or_name: key,
            reply,
        })
        .await
    }

    /// Menyimpan sesi baru (dengan penyamaran rahasia pada ringkasan `task`).
    pub async fn insert_session(&self, session: Session) -> Result<Session, StoreError> {
        self.request(|reply| StoreCmd::InsertSession { session, reply })
            .await
    }

    /// Mengambil sesi berdasarkan ID lengkap atau awalan unik.
    pub async fn get_session(&self, id_or_prefix: &str) -> Result<Session, StoreError> {
        let key = id_or_prefix.to_string();
        self.request(|reply| StoreCmd::GetSession {
            id_or_prefix: key,
            reply,
        })
        .await
    }

    /// Mengambil daftar sesi dengan filter opsional.
    pub async fn list_sessions(
        &self,
        project_id: Option<String>,
        status: Option<SessionStatus>,
    ) -> Result<Vec<Session>, StoreError> {
        self.request(|reply| StoreCmd::ListSessions {
            project_id,
            status,
            reply,
        })
        .await
    }

    /// Mengubah status sesi dengan memvalidasi aturan `can_transition`.
    pub async fn transition_session(
        &self,
        session_id: &str,
        to: SessionStatus,
        exit_code: Option<i32>,
        trip_reason: Option<TripReason>,
        ended_at: Option<i64>,
        clear_pid: bool,
    ) -> Result<Session, StoreError> {
        let id = session_id.to_string();
        self.request(|reply| StoreCmd::TransitionSession {
            session_id: id,
            to,
            exit_code,
            trip_reason,
            ended_at,
            clear_pid,
            reply,
        })
        .await
    }

    /// Menghapus entri sesi dari SQLite.
    pub async fn delete_session(&self, session_id: &str) -> Result<(), StoreError> {
        let id = session_id.to_string();
        self.request(|reply| StoreCmd::DeleteSession {
            session_id: id,
            reply,
        })
        .await
    }

    /// Mencatat peristiwa audit (detail otomatis melewati `redact_secrets`).
    pub async fn insert_audit(&self, entry: AuditEntry) -> Result<(), StoreError> {
        self.request(|reply| StoreCmd::InsertAudit { entry, reply })
            .await
    }

    /// Mengambil entri audit terbaru.
    pub async fn list_audit(&self, limit: usize) -> Result<Vec<AuditEntry>, StoreError> {
        self.request(|reply| StoreCmd::ListAudit { limit, reply })
            .await
    }
}

fn configure_and_migrate(conn: &Connection) -> Result<(), StoreError> {
    conn.execute_batch(
        "PRAGMA journal_mode=WAL;
         PRAGMA synchronous=NORMAL;
         PRAGMA foreign_keys=ON;
         PRAGMA busy_timeout=5000;
         PRAGMA cache_size=-2000;",
    )?;

    let check: String = conn.query_row("PRAGMA integrity_check;", [], |row| row.get(0))?;
    if check != "ok" {
        return Err(StoreError::Integrity(check));
    }

    let has_migrations: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='schema_migrations')",
        [],
        |row| row.get(0),
    )?;

    if !has_migrations {
        conn.execute_batch(MIGRATION_0001)?;
        conn.execute(
            "INSERT INTO schema_migrations (version, name, applied_at) VALUES (?1, ?2, ?3)",
            params![1i64, "0001_initial", now_ms()],
        )?;
    }

    Ok(())
}

fn run_actor(mut conn: Connection, mut rx: mpsc::Receiver<StoreCmd>) {
    while let Some(cmd) = rx.blocking_recv() {
        match cmd {
            StoreCmd::AddProject { project, reply } => {
                let _ = reply.send(exec_add_project(&conn, project));
            }
            StoreCmd::GetProject { id_or_name, reply } => {
                let _ = reply.send(exec_get_project(&conn, &id_or_name));
            }
            StoreCmd::ListProjects { reply } => {
                let _ = reply.send(exec_list_projects(&conn));
            }
            StoreCmd::RemoveProject { id_or_name, reply } => {
                let _ = reply.send(exec_remove_project(&conn, &id_or_name));
            }
            StoreCmd::InsertSession { session, reply } => {
                let _ = reply.send(exec_insert_session(&conn, session));
            }
            StoreCmd::GetSession {
                id_or_prefix,
                reply,
            } => {
                let _ = reply.send(exec_get_session(&conn, &id_or_prefix));
            }
            StoreCmd::ListSessions {
                project_id,
                status,
                reply,
            } => {
                let _ = reply.send(exec_list_sessions(&conn, project_id.as_deref(), status));
            }
            StoreCmd::TransitionSession {
                session_id,
                to,
                exit_code,
                trip_reason,
                ended_at,
                clear_pid,
                reply,
            } => {
                let _ = reply.send(exec_transition_session(
                    &mut conn,
                    &session_id,
                    to,
                    exit_code,
                    trip_reason,
                    ended_at,
                    clear_pid,
                ));
            }
            StoreCmd::DeleteSession { session_id, reply } => {
                let _ = reply.send(exec_delete_session(&conn, &session_id));
            }
            StoreCmd::InsertAudit { entry, reply } => {
                let _ = reply.send(exec_insert_audit(&conn, entry));
            }
            StoreCmd::ListAudit { limit, reply } => {
                let _ = reply.send(exec_list_audit(&conn, limit));
            }
            StoreCmd::Shutdown { reply } => {
                let _ = reply.send(Ok(()));
                break;
            }
        }
    }
}

fn exec_add_project(conn: &Connection, project: Project) -> Result<Project, StoreError> {
    conn.execute(
        "INSERT INTO projects (id, name, repo_path, default_branch, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            project.id,
            project.name,
            project.repo_path,
            project.default_branch,
            project.created_at
        ],
    )?;
    Ok(project)
}

fn exec_get_project(conn: &Connection, id_or_name: &str) -> Result<Project, StoreError> {
    conn.query_row(
        "SELECT id, name, repo_path, default_branch, created_at
         FROM projects WHERE id = ?1 OR name = ?1 LIMIT 1",
        params![id_or_name],
        row_to_project,
    )
    .optional()?
    .ok_or_else(|| StoreError::NotFound {
        entity: "project",
        key: id_or_name.to_string(),
    })
}

fn exec_list_projects(conn: &Connection) -> Result<Vec<Project>, StoreError> {
    let mut stmt = conn.prepare(
        "SELECT id, name, repo_path, default_branch, created_at FROM projects ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map([], row_to_project)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn exec_remove_project(conn: &Connection, id_or_name: &str) -> Result<(), StoreError> {
    let project = exec_get_project(conn, id_or_name)?;
    let active_count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM sessions WHERE project_id = ?1 AND status IN ('running','paused','waiting_approval','hibernated')",
        params![project.id],
        |row| row.get(0),
    )?;
    if active_count > 0 {
        return Err(StoreError::Integrity(format!(
            "cannot remove project '{}' with {} active session(s)",
            project.name, active_count
        )));
    }
    conn.execute("DELETE FROM projects WHERE id = ?1", params![project.id])?;
    Ok(())
}

fn exec_insert_session(conn: &Connection, mut session: Session) -> Result<Session, StoreError> {
    session.task = session.task.map(|t| truncate_redacted(&t, 512));
    conn.execute(
        "INSERT INTO sessions (
            id, project_id, agent_profile, worktree_path, branch_name, status,
            pid, started_at, ended_at, task, pipeline_run_id, step_id,
            exit_code, trip_reason, proxy_mode, sandbox_provider
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
        params![
            session.id,
            session.project_id,
            session.agent_profile,
            session.worktree_path,
            session.branch_name,
            session.status.as_str(),
            session.pid,
            session.started_at,
            session.ended_at,
            session.task,
            session.pipeline_run_id,
            session.step_id,
            session.exit_code,
            session.trip_reason.map(TripReason::as_str),
            session.proxy_mode.as_str(),
            session.sandbox_provider,
        ],
    )?;
    Ok(session)
}

fn exec_get_session(conn: &Connection, id_or_prefix: &str) -> Result<Session, StoreError> {
    if let Some(exact) = conn
        .query_row(
            "SELECT id, project_id, agent_profile, worktree_path, branch_name, status,
                    pid, started_at, ended_at, task, pipeline_run_id, step_id,
                    exit_code, trip_reason, proxy_mode, sandbox_provider
             FROM sessions WHERE id = ?1 LIMIT 1",
            params![id_or_prefix],
            row_to_session,
        )
        .optional()?
    {
        return Ok(exact);
    }

    let like_pattern = format!("{id_or_prefix}%");
    let mut stmt = conn.prepare(
        "SELECT id, project_id, agent_profile, worktree_path, branch_name, status,
                pid, started_at, ended_at, task, pipeline_run_id, step_id,
                exit_code, trip_reason, proxy_mode, sandbox_provider
         FROM sessions WHERE id LIKE ?1 LIMIT 2",
    )?;
    let mut rows = stmt.query_map(params![like_pattern], row_to_session)?;
    match (rows.next(), rows.next()) {
        (Some(first), None) => Ok(first?),
        (Some(_), Some(_)) => Err(StoreError::Integrity(format!(
            "ambiguous session id prefix '{id_or_prefix}'"
        ))),
        _ => Err(StoreError::NotFound {
            entity: "session",
            key: id_or_prefix.to_string(),
        }),
    }
}

fn exec_list_sessions(
    conn: &Connection,
    project_id: Option<&str>,
    status: Option<SessionStatus>,
) -> Result<Vec<Session>, StoreError> {
    let status_str = status.map(SessionStatus::as_str);
    let mut stmt = conn.prepare(
        "SELECT id, project_id, agent_profile, worktree_path, branch_name, status,
                pid, started_at, ended_at, task, pipeline_run_id, step_id,
                exit_code, trip_reason, proxy_mode, sandbox_provider
         FROM sessions
         WHERE (?1 IS NULL OR project_id = ?1)
           AND (?2 IS NULL OR status = ?2)
         ORDER BY started_at DESC",
    )?;
    let rows = stmt.query_map(params![project_id, status_str], row_to_session)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn exec_transition_session(
    conn: &mut Connection,
    session_id: &str,
    to: SessionStatus,
    exit_code: Option<i32>,
    trip_reason: Option<TripReason>,
    ended_at: Option<i64>,
    clear_pid: bool,
) -> Result<Session, StoreError> {
    let tx = conn.transaction()?;
    let current = exec_get_session(&tx, session_id)?;
    if !can_transition(current.status, to) {
        return Err(StoreError::InvalidTransition {
            from: current.status,
            to,
        });
    }

    let final_pid = if clear_pid { None } else { current.pid };
    let final_ended_at = ended_at.or_else(|| {
        if to.is_terminal() {
            Some(now_ms())
        } else {
            current.ended_at
        }
    });
    let final_exit = exit_code.or(current.exit_code);
    let final_trip = trip_reason.or(current.trip_reason);

    tx.execute(
        "UPDATE sessions
         SET status = ?2, pid = ?3, ended_at = ?4, exit_code = ?5, trip_reason = ?6
         WHERE id = ?1",
        params![
            current.id,
            to.as_str(),
            final_pid,
            final_ended_at,
            final_exit,
            final_trip.map(TripReason::as_str),
        ],
    )?;
    let updated = exec_get_session(&tx, &current.id)?;
    tx.commit()?;
    Ok(updated)
}

fn exec_delete_session(conn: &Connection, session_id: &str) -> Result<(), StoreError> {
    let affected = conn.execute("DELETE FROM sessions WHERE id = ?1", params![session_id])?;
    if affected == 0 {
        return Err(StoreError::NotFound {
            entity: "session",
            key: session_id.to_string(),
        });
    }
    Ok(())
}

fn exec_insert_audit(conn: &Connection, entry: AuditEntry) -> Result<(), StoreError> {
    let sanitized = truncate_redacted(&entry.detail, 1024);
    conn.execute(
        "INSERT INTO audit_log (session_id, kind, detail, timestamp) VALUES (?1, ?2, ?3, ?4)",
        params![entry.session_id, entry.kind, sanitized, entry.timestamp],
    )?;
    Ok(())
}

fn exec_list_audit(conn: &Connection, limit: usize) -> Result<Vec<AuditEntry>, StoreError> {
    let lim = i64::try_from(limit).unwrap_or(100);
    let mut stmt = conn.prepare(
        "SELECT id, session_id, kind, detail, timestamp FROM audit_log ORDER BY id DESC LIMIT ?1",
    )?;
    let rows = stmt.query_map(params![lim], |row| {
        Ok(AuditEntry {
            id: row.get(0)?,
            session_id: row.get(1)?,
            kind: row.get(2)?,
            detail: row.get(3)?,
            timestamp: row.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for row in rows {
        out.push(row?);
    }
    Ok(out)
}

fn row_to_project(row: &rusqlite::Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        repo_path: row.get(2)?,
        default_branch: row.get(3)?,
        created_at: row.get(4)?,
    })
}

fn row_to_session(row: &rusqlite::Row<'_>) -> rusqlite::Result<Session> {
    let status_raw: String = row.get(5)?;
    let status = status_raw.parse::<SessionStatus>().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(5, rusqlite::types::Type::Text, Box::new(e))
    })?;

    let trip_raw: Option<String> = row.get(13)?;
    let trip_reason = match trip_raw {
        Some(s) => Some(s.parse::<TripReason>().map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(13, rusqlite::types::Type::Text, Box::new(e))
        })?),
        None => None,
    };

    let proxy_raw: String = row.get(14)?;
    let proxy_mode = proxy_raw.parse::<ProxyMode>().map_err(|e| {
        rusqlite::Error::FromSqlConversionFailure(14, rusqlite::types::Type::Text, Box::new(e))
    })?;

    Ok(Session {
        id: row.get(0)?,
        project_id: row.get(1)?,
        agent_profile: row.get(2)?,
        worktree_path: row.get(3)?,
        branch_name: row.get(4)?,
        status,
        pid: row.get(6)?,
        started_at: row.get(7)?,
        ended_at: row.get(8)?,
        task: row.get(9)?,
        pipeline_run_id: row.get(10)?,
        step_id: row.get(11)?,
        exit_code: row.get(12)?,
        trip_reason,
        proxy_mode,
        sandbox_provider: row.get(15)?,
    })
}

fn truncate_redacted(raw: &str, max_bytes: usize) -> String {
    let redacted = redact_secrets(raw);
    if redacted.len() <= max_bytes {
        return redacted;
    }
    let mut end = max_bytes;
    while end > 0 && !redacted.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}...", &redacted[..end])
}

#[cfg(test)]
mod tests {
    use latch_core::id::{new_id, now_ms};
    use latch_core::model::{AuditEntry, Project, ProxyMode, Session, SessionStatus};

    use super::{StoreError, StoreHandle};

    #[tokio::test]
    async fn migrates_empty_db_and_enforces_crud_and_transitions() {
        let store = StoreHandle::open_in_memory().unwrap();
        let proj = Project {
            id: new_id(),
            name: "demo".to_string(),
            repo_path: "/tmp/demo".to_string(),
            default_branch: "main".to_string(),
            created_at: now_ms(),
        };
        store.add_project(proj.clone()).await.unwrap();
        assert_eq!(store.list_projects().await.unwrap().len(), 1);

        let ses = Session {
            id: new_id(),
            project_id: proj.id.clone(),
            agent_profile: "sleeper".to_string(),
            worktree_path: "/tmp/wt/1".to_string(),
            branch_name: "latch/demo/12345678".to_string(),
            status: SessionStatus::Running,
            pid: Some(99999),
            started_at: now_ms(),
            ended_at: None,
            task: Some("Bearer sk-test-CANARY0000123456".to_string()),
            pipeline_run_id: None,
            step_id: None,
            exit_code: None,
            trip_reason: None,
            proxy_mode: ProxyMode::Bypass,
            sandbox_provider: "noop".to_string(),
        };
        let saved = store.insert_session(ses).await.unwrap();
        assert!(!saved.task.as_deref().unwrap().contains("CANARY"));

        let paused = store
            .transition_session(&saved.id, SessionStatus::Paused, None, None, None, false)
            .await
            .unwrap();
        assert_eq!(paused.status, SessionStatus::Paused);

        let invalid = store
            .transition_session(
                &saved.id,
                SessionStatus::Completed,
                Some(0),
                None,
                None,
                true,
            )
            .await;
        assert!(matches!(invalid, Err(StoreError::InvalidTransition { .. })));

        let failed = store
            .transition_session(&saved.id, SessionStatus::Failed, Some(1), None, None, true)
            .await
            .unwrap();
        assert_eq!(failed.status, SessionStatus::Failed);
        assert!(failed.pid.is_none());
        assert!(failed.ended_at.is_some());

        store
            .insert_audit(AuditEntry {
                id: None,
                session_id: Some(saved.id.clone()),
                kind: "test".to_string(),
                detail: "secret sk-test-CANARY987654321".to_string(),
                timestamp: now_ms(),
            })
            .await
            .unwrap();
        let logs = store.list_audit(10).await.unwrap();
        assert!(!logs[0].detail.contains("CANARY"));
    }
}
