//! Server HTTP REST + WebSocket + penyajian Web PWA tersemat (`rust-embed`) + CSP (T-220, T-221, T-224).

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{ConnectInfo, Path as AxumPath, Query, State};
use axum::http::{header, HeaderMap, HeaderValue, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post};
use axum::{Json, Router};
use base64::engine::general_purpose::{STANDARD as BASE64_STD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use latch_core::id::{ms_to_rfc3339, now_ms};
use latch_core::model::{AuditEntry, MergeStrategy, Session, SessionTelemetrySummary};
use latch_core::protocol::{
    ProjectAddParams, SessionDiffParams, SessionListParams, SessionMergeParams, SessionRunParams,
};
use latch_pty_pool::PtyFrame;
use rand::RngCore;
use rust_embed::RustEmbed;
use serde::{Deserialize, Serialize};
use serde_json::json;
use subtle::ConstantTimeEq;
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::sync::Notify;
use tracing::warn;

use crate::session_manager::{DaemonEvent, ManagerError, SessionManager};

const AUTH_FAIL_LIMIT_PER_MINUTE: usize = 5;
const AUTH_WINDOW: Duration = Duration::from_secs(60);
const WS_AUTH_TIMEOUT: Duration = Duration::from_secs(5);
const CSP_HEADER_VALUE: &str = "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; connect-src 'self' ws: wss:; img-src 'self' data:; object-src 'none'; frame-ancestors 'none'; base-uri 'self'";

#[derive(RustEmbed)]
#[folder = "../../web/dist"]
#[allow_missing = true]
struct WebAssets;

/// Kesalahan server HTTP/WebSocket daemon.
#[derive(Debug, Error)]
pub enum HttpError {
    /// Menolak bind alamat non-loopback demi keamanan bawaan.
    #[error("refusing to bind HTTP server on non-loopback address: {0}")]
    NonLoopbackBind(SocketAddr),
    /// Galat I/O saat membaca/menulis token atau bind socket.
    #[error("HTTP server I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Memuat bearer token 256-bit dari disk atau membuat yang baru dengan izin `0600`.
pub fn ensure_or_load_token(token_path: &Path) -> Result<String, HttpError> {
    if token_path.exists() {
        let existing = std::fs::read_to_string(token_path)?.trim().to_string();
        if existing.len() >= 32 {
            return Ok(existing);
        }
    }
    rotate_token(token_path)
}

/// Menghasilkan bearer token acak 256-bit baru dan menyimpannya dengan izin `0600`.
pub fn rotate_token(token_path: &Path) -> Result<String, HttpError> {
    if let Some(parent) = token_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut raw = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut raw);
    let token = URL_SAFE_NO_PAD.encode(raw);
    std::fs::write(token_path, format!("{token}\n"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(token_path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(token)
}

/// Perbandingan token waktu-konstan (*constant-time compare*).
pub fn verify_token_constant_time(expected: &str, candidate: &str) -> bool {
    let a = expected.as_bytes();
    let b = candidate.as_bytes();
    if a.len() != b.len() {
        return false;
    }
    bool::from(a.ct_eq(b))
}

#[derive(Default)]
struct AuthRateLimiter {
    failures: Mutex<HashMap<IpAddr, Vec<Instant>>>,
}

impl AuthRateLimiter {
    fn is_rate_limited(&self, ip: IpAddr) -> bool {
        let Ok(mut guard) = self.failures.lock() else {
            return false;
        };
        let now = Instant::now();
        let entry = guard.entry(ip).or_default();
        entry.retain(|t| now.duration_since(*t) < AUTH_WINDOW);
        entry.len() >= AUTH_FAIL_LIMIT_PER_MINUTE
    }

    fn record_failure(&self, ip: IpAddr) -> bool {
        let Ok(mut guard) = self.failures.lock() else {
            return false;
        };
        let now = Instant::now();
        let entry = guard.entry(ip).or_default();
        entry.retain(|t| now.duration_since(*t) < AUTH_WINDOW);
        entry.push(now);
        entry.len() >= AUTH_FAIL_LIMIT_PER_MINUTE
    }
}

#[derive(Clone)]
struct HttpState {
    manager: SessionManager,
    token: Arc<String>,
    limiter: Arc<AuthRateLimiter>,
    event_seq: Arc<AtomicU64>,
}

/// Handle server HTTP/WS yang sedang berjalan.
pub struct HttpServerHandle {
    addr: SocketAddr,
}

impl HttpServerHandle {
    /// Alamat loopback server HTTP/WS.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

/// Memulai server HTTP REST + WebSocket + Web PWA tersemat pada alamat loopback.
pub async fn start_http_server(
    bind_addr: SocketAddr,
    manager: SessionManager,
    token: String,
    shutdown: Arc<Notify>,
) -> Result<HttpServerHandle, HttpError> {
    if !bind_addr.ip().is_loopback() {
        return Err(HttpError::NonLoopbackBind(bind_addr));
    }

    let state = HttpState {
        manager,
        token: Arc::new(token),
        limiter: Arc::new(AuthRateLimiter::default()),
        event_seq: Arc::new(AtomicU64::new(1)),
    };

    let app = Router::new()
        .route("/api/v1/health", get(handle_health))
        .route("/api/v1/metrics", get(handle_metrics))
        .route(
            "/api/v1/projects",
            get(handle_list_projects).post(handle_add_project),
        )
        .route("/api/v1/projects/{id}", delete(handle_remove_project))
        .route(
            "/api/v1/sessions",
            get(handle_list_sessions).post(handle_run_session),
        )
        .route("/api/v1/sessions/{id}", get(handle_get_session))
        .route("/api/v1/sessions/{id}/pause", post(handle_pause_session))
        .route("/api/v1/sessions/{id}/resume", post(handle_resume_session))
        .route("/api/v1/sessions/{id}/kill", post(handle_kill_session))
        .route("/api/v1/sessions/{id}/input", post(handle_input_session))
        .route("/api/v1/sessions/{id}/resize", post(handle_resize_session))
        .route("/api/v1/sessions/{id}/diff", get(handle_diff_session))
        .route("/api/v1/sessions/{id}/merge-check", get(handle_merge_check))
        .route("/api/v1/sessions/{id}/merge", post(handle_merge_session))
        .route(
            "/api/v1/sessions/{id}/discard",
            post(handle_discard_session),
        )
        .route(
            "/api/v1/sessions/{id}/telemetry",
            get(handle_session_telemetry),
        )
        .route("/api/v1/sessions/{id}/screen", get(handle_session_screen))
        .route("/api/v1/ws", get(handle_events_ws))
        .route("/api/v1/sessions/{id}/pty", get(handle_pty_ws))
        .fallback(get(handle_static_asset))
        .with_state(state);

    let listener = TcpListener::bind(bind_addr).await?;
    let addr = listener.local_addr()?;

    tokio::spawn(async move {
        let server = axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async move {
            shutdown.notified().await;
        });
        if let Err(err) = server.await {
            warn!(error = %err, "HTTP/WS server exited with error");
        }
    });

    Ok(HttpServerHandle { addr })
}

#[allow(clippy::result_large_err)]
fn check_origin(headers: &HeaderMap) -> Result<(), Response> {
    let Some(origin) = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok()) else {
        return Ok(());
    };
    let lower = origin.to_ascii_lowercase();
    let allowed = lower.starts_with("http://127.0.0.1")
        || lower.starts_with("http://localhost")
        || lower.starts_with("http://[::1]")
        || lower.starts_with("https://127.0.0.1")
        || lower.starts_with("https://localhost")
        || lower.ends_with(".ts.net");
    if allowed {
        Ok(())
    } else {
        Err(api_error(
            StatusCode::FORBIDDEN,
            "forbidden_origin",
            "request origin is not allowed",
        ))
    }
}

#[allow(clippy::result_large_err)]
async fn authorize_http(
    state: &HttpState,
    headers: &HeaderMap,
    peer: SocketAddr,
) -> Result<(), Response> {
    check_origin(headers)?;
    if state.limiter.is_rate_limited(peer.ip()) {
        return Err(api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many failed authentication attempts",
        ));
    }

    let presented = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|raw| raw.strip_prefix("Bearer "))
        .unwrap_or("");

    if verify_token_constant_time(&state.token, presented) {
        return Ok(());
    }

    let hit_limit = state.limiter.record_failure(peer.ip());
    let _ = state
        .manager
        .record_audit(AuditEntry {
            id: None,
            session_id: None,
            kind: "auth_failure".to_string(),
            detail: format!("failed bearer authentication from {}", peer.ip()),
            timestamp: now_ms(),
        })
        .await;

    if hit_limit {
        Err(api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many failed authentication attempts",
        ))
    } else {
        Err(api_error(
            StatusCode::UNAUTHORIZED,
            "unauthorized",
            "missing or invalid bearer token",
        ))
    }
}

fn api_error(status: StatusCode, code: &str, message: impl Into<String>) -> Response {
    (
        status,
        Json(json!({
            "error": {
                "code": code,
                "message": message.into(),
                "details": {}
            }
        })),
    )
        .into_response()
}

fn manager_err_to_response(err: ManagerError) -> Response {
    match &err {
        ManagerError::Store(latch_store::StoreError::NotFound { entity, .. }) => api_error(
            StatusCode::NOT_FOUND,
            &format!("{entity}_not_found"),
            err.to_string(),
        ),
        ManagerError::Store(latch_store::StoreError::InvalidTransition { .. })
        | ManagerError::InvalidState { .. } => api_error(
            StatusCode::CONFLICT,
            "invalid_state_transition",
            err.to_string(),
        ),
        ManagerError::Git(latch_git_engine::GitError::MergeConflict { .. }) => {
            api_error(StatusCode::CONFLICT, "merge_conflict", err.to_string())
        }
        ManagerError::Config(_) => api_error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "profile_invalid",
            err.to_string(),
        ),
        ManagerError::MaxSessionsReached(_) => api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "max_sessions_reached",
            err.to_string(),
        ),
        _ => api_error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal",
            err.to_string(),
        ),
    }
}

async fn handle_health(State(state): State<HttpState>) -> impl IntoResponse {
    let h = state.manager.health().await;
    Json(json!({
        "status": h.status,
        "version": h.version,
        "sandbox": h.sandbox,
        "pid": h.pid,
        "active_sessions": h.active_sessions,
        "api_versions": ["v1"]
    }))
}

async fn handle_metrics(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    let h = state.manager.health().await;
    let rss_kb = read_self_rss_kb();
    Json(json!({
        "version": h.version,
        "pid": h.pid,
        "active_sessions": h.active_sessions,
        "rss_kb": rss_kb,
        "server_time": ms_to_rfc3339(now_ms())
    }))
    .into_response()
}

fn read_self_rss_kb() -> Option<u64> {
    let status = std::fs::read_to_string("/proc/self/status").ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.split_whitespace().next()?.parse::<u64>().ok();
        }
    }
    None
}

async fn handle_list_projects(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.list_projects().await {
        Ok(list) => Json(list).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_add_project(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(params): Json<ProjectAddParams>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.add_project(params).await {
        Ok(proj) => (StatusCode::CREATED, Json(proj)).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_remove_project(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.remove_project(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

#[derive(Deserialize)]
struct SessionListQuery {
    project: Option<String>,
    status: Option<String>,
}

async fn handle_list_sessions(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Query(q): Query<SessionListQuery>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state
        .manager
        .list_sessions(SessionListParams {
            project: q.project,
            status: q.status,
        })
        .await
    {
        Ok(list) => Json(list).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_run_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(params): Json<SessionRunParams>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.run_session(params).await {
        Ok(ses) => (StatusCode::CREATED, Json(ses)).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

#[derive(Serialize)]
struct SessionDetailResponse {
    #[serde(flatten)]
    session: Session,
    telemetry: SessionTelemetrySummary,
}

async fn handle_get_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.get_session(&id).await {
        Ok(session) => {
            let telemetry = state
                .manager
                .session_telemetry_summary(&session.id)
                .await
                .unwrap_or_default();
            Json(SessionDetailResponse { session, telemetry }).into_response()
        }
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_pause_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.pause_session(&id).await {
        Ok(ses) => Json(ses).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_resume_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.resume_session(&id).await {
        Ok(ses) => Json(ses).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_kill_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.kill_session(&id).await {
        Ok(ses) => Json(ses).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

#[derive(Deserialize)]
struct InputBody {
    data_base64: String,
}

async fn handle_input_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<InputBody>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    let Ok(bytes) = BASE64_STD.decode(&body.data_base64) else {
        return api_error(
            StatusCode::BAD_REQUEST,
            "invalid_request",
            "invalid base64 payload",
        );
    };
    match state.manager.send_input(&id, &bytes).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

#[derive(Deserialize)]
struct ResizeBody {
    cols: u16,
    rows: u16,
}

async fn handle_resize_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<ResizeBody>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state
        .manager
        .resize_session(&id, body.cols, body.rows)
        .await
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

#[derive(Deserialize)]
struct DiffQuery {
    file: Option<String>,
    max_bytes: Option<usize>,
}

async fn handle_diff_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Query(q): Query<DiffQuery>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state
        .manager
        .diff_session(SessionDiffParams {
            session_id: id,
            file: q.file,
            max_bytes: q.max_bytes,
        })
        .await
    {
        Ok(diff) => Json(diff).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_merge_check(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.merge_check_session(&id).await {
        Ok(res) => Json(res).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

#[derive(Deserialize)]
struct MergeBody {
    #[serde(default)]
    strategy: MergeStrategy,
    message: Option<String>,
}

async fn handle_merge_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Json(body): Json<MergeBody>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state
        .manager
        .merge_session(SessionMergeParams {
            session_id: id,
            strategy: body.strategy,
            message: body.message,
        })
        .await
    {
        Ok(ses) => Json(ses).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_discard_session(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.discard_session(&id).await {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

#[derive(Deserialize)]
struct TelemetryQuery {
    since: Option<i64>,
}

async fn handle_session_telemetry(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    Query(q): Query<TelemetryQuery>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.session_telemetry(&id, q.since).await {
        Ok(records) => {
            let summary = state
                .manager
                .session_telemetry_summary(&id)
                .await
                .unwrap_or_default();
            Json(json!({ "summary": summary, "records": records })).into_response()
        }
        Err(err) => manager_err_to_response(err),
    }
}

async fn handle_session_screen(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
) -> Response {
    if let Err(resp) = authorize_http(&state, &headers, peer).await {
        return resp;
    }
    match state.manager.screen_snapshot(&id).await {
        Ok(snap) => Json(snap).into_response(),
        Err(err) => manager_err_to_response(err),
    }
}

fn extract_ws_subprotocol_token(headers: &HeaderMap) -> Option<String> {
    let raw = headers
        .get(header::SEC_WEBSOCKET_PROTOCOL)
        .and_then(|v| v.to_str().ok())?;
    for part in raw.split(',') {
        if let Some(tok) = part.trim().strip_prefix("bearer.") {
            return Some(tok.to_string());
        }
    }
    None
}

async fn authenticate_ws_socket(
    socket: &mut WebSocket,
    state: &HttpState,
    pre_auth: bool,
    peer: SocketAddr,
) -> bool {
    if pre_auth {
        return true;
    }
    let first = tokio::time::timeout(WS_AUTH_TIMEOUT, socket.recv()).await;
    let Ok(Some(Ok(Message::Text(text)))) = first else {
        return false;
    };
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&text);
    let ok = parsed.ok().is_some_and(|v| {
        v.get("type").and_then(|t| t.as_str()) == Some("auth")
            && v.get("token")
                .and_then(|t| t.as_str())
                .is_some_and(|tok| verify_token_constant_time(&state.token, tok))
    });
    if !ok {
        state.limiter.record_failure(peer.ip());
        let _ = state
            .manager
            .record_audit(AuditEntry {
                id: None,
                session_id: None,
                kind: "auth_failure".to_string(),
                detail: format!("failed WS authentication from {}", peer.ip()),
                timestamp: now_ms(),
            })
            .await;
    }
    ok
}

async fn handle_events_ws(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(resp) = check_origin(&headers) {
        return resp;
    }
    if state.limiter.is_rate_limited(peer.ip()) {
        return api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many failed authentication attempts",
        );
    }

    let pre_auth = extract_ws_subprotocol_token(&headers)
        .is_some_and(|t| verify_token_constant_time(&state.token, &t));
    let mut upgrade = ws;
    if pre_auth {
        upgrade = upgrade.protocols(["agentd.v1"]);
    }
    upgrade.on_upgrade(move |mut socket| async move {
        if !authenticate_ws_socket(&mut socket, &state, pre_auth, peer).await {
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
        let hello = json!({
            "type": "hello",
            "version": env!("CARGO_PKG_VERSION"),
            "server_time": ms_to_rfc3339(now_ms())
        });
        if socket
            .send(Message::Text(hello.to_string().into()))
            .await
            .is_err()
        {
            return;
        }

        let mut rx = state.manager.subscribe_events();
        loop {
            tokio::select! {
                ev = rx.recv() => {
                    match ev {
                        Ok(DaemonEvent::SessionCreated { session_id }) => {
                            let seq = state.event_seq.fetch_add(1, Ordering::Relaxed);
                            let msg = json!({
                                "seq": seq,
                                "type": "session.created",
                                "session_id": session_id
                            });
                            if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                        Ok(DaemonEvent::SessionStatusChanged { session_id, from, to }) => {
                            let seq = state.event_seq.fetch_add(1, Ordering::Relaxed);
                            let msg = json!({
                                "seq": seq,
                                "type": "session.status",
                                "session_id": session_id,
                                "from": from.as_str(),
                                "to": to.as_str()
                            });
                            if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    }
                }
                incoming = socket.recv() => {
                    match incoming {
                        Some(Ok(Message::Ping(p))) => {
                            let _ = socket.send(Message::Pong(p)).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
            }
        }
    })
}

async fn handle_pty_ws(
    State(state): State<HttpState>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    AxumPath(id): AxumPath<String>,
    ws: WebSocketUpgrade,
) -> Response {
    if let Err(resp) = check_origin(&headers) {
        return resp;
    }
    if state.limiter.is_rate_limited(peer.ip()) {
        return api_error(
            StatusCode::TOO_MANY_REQUESTS,
            "rate_limited",
            "too many failed authentication attempts",
        );
    }

    let pre_auth = extract_ws_subprotocol_token(&headers)
        .is_some_and(|t| verify_token_constant_time(&state.token, &t));
    let mut upgrade = ws;
    if pre_auth {
        upgrade = upgrade.protocols(["agentd.v1"]);
    }
    upgrade.on_upgrade(move |mut socket| async move {
        if !authenticate_ws_socket(&mut socket, &state, pre_auth, peer).await {
            let _ = socket.send(Message::Close(None)).await;
            return;
        }
        let Ok((_ses, handle, mut sub)) = state.manager.attach_session(&id).await else {
            let _ = socket.send(Message::Close(None)).await;
            return;
        };

        // Frame pertama: snapshot ring buffer agar klien langsung sinkron (T-221)
        if !sub.initial_bytes.is_empty()
            && socket
                .send(Message::Binary(sub.initial_bytes.into()))
                .await
                .is_err()
        {
            return;
        }

        loop {
            tokio::select! {
                frame = sub.rx.recv() => {
                    match frame {
                        Some(PtyFrame::Output(bytes)) => {
                            if socket.send(Message::Binary(bytes.into())).await.is_err() {
                                break;
                            }
                        }
                        Some(PtyFrame::Dropped(bytes)) => {
                            let msg = json!({ "type": "dropped", "bytes": bytes });
                            if socket.send(Message::Text(msg.to_string().into())).await.is_err() {
                                break;
                            }
                        }
                        None => break,
                    }
                }
                incoming = socket.recv() => {
                    match incoming {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
                                match val.get("type").and_then(|v| v.as_str()) {
                                    Some("input") => {
                                        if let Some(data) = val.get("data").and_then(|v| v.as_str()) {
                                            let _ = handle.write_input(data.as_bytes());
                                        }
                                    }
                                    Some("resize") => {
                                        let cols = val.get("cols").and_then(|v| v.as_u64()).and_then(|n| u16::try_from(n).ok());
                                        let rows = val.get("rows").and_then(|v| v.as_u64()).and_then(|n| u16::try_from(n).ok());
                                        if let (Some(c), Some(r)) = (cols, rows) {
                                            let _ = handle.resize(c, r);
                                        }
                                    }
                                    Some("resync") => {
                                        let snap = handle.snapshot();
                                        if socket.send(Message::Binary(snap.ansi_bytes.into())).await.is_err() {
                                            break;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                        Some(Ok(Message::Ping(p))) => {
                            let _ = socket.send(Message::Pong(p)).await;
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        _ => {}
                    }
                }
            }
        }
    })
}

async fn handle_static_asset(uri: Uri) -> Response {
    let raw_path = uri.path().trim_start_matches('/');
    let path = if raw_path.is_empty() {
        "index.html"
    } else {
        raw_path
    };

    let asset = WebAssets::get(path).or_else(|| WebAssets::get("index.html"));
    let (body, mime) = match asset {
        Some(file) => (file.data.into_owned(), guess_mime(path)),
        None => (
            b"<!doctype html><html><head><meta charset=\"utf-8\"><title>Latch</title></head><body><h1>Latch Daemon</h1><p>Build web/dist to serve the embedded Workbench PWA.</p></body></html>"
                .to_vec(),
            "text/html; charset=utf-8",
        ),
    };

    let mut resp = Response::new(Body::from(body));
    let headers = resp.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(mime));
    headers.insert(
        header::CONTENT_SECURITY_POLICY,
        HeaderValue::from_static(CSP_HEADER_VALUE),
    );
    headers.insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        header::REFERRER_POLICY,
        HeaderValue::from_static("no-referrer"),
    );
    resp
}

fn guess_mime(path: &str) -> &'static str {
    if path.ends_with(".html") || !path.contains('.') {
        "text/html; charset=utf-8"
    } else if path.ends_with(".js") || path.ends_with(".mjs") {
        "application/javascript; charset=utf-8"
    } else if path.ends_with(".css") {
        "text/css; charset=utf-8"
    } else if path.ends_with(".json") || path.ends_with(".webmanifest") {
        "application/json; charset=utf-8"
    } else if path.ends_with(".svg") {
        "image/svg+xml"
    } else if path.ends_with(".png") {
        "image/png"
    } else if path.ends_with(".ico") {
        "image/x-icon"
    } else {
        "application/octet-stream"
    }
}
