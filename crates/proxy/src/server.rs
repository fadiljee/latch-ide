//! Server reverse proxy HTTP/SSE lokal dengan `axum` + `reqwest` (`rustls`) (T-201..T-208).

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::body::Body;
use axum::extract::{Path, Request, State};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::any;
use axum::Router;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;
use bytes::Bytes;
use futures_util::StreamExt;
use latch_core::id::now_ms;
use latch_core::model::TelemetryRecord;
use latch_core::pricing::PricingTable;
use rand::RngCore;
use serde_json::Value;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Notify, RwLock};
use tracing::{debug, warn};

use crate::circuit_breaker::{CircuitBreakerState, SessionLimits, TripEvent};
use crate::sse::ResponseTeeParser;
use crate::ProxyError;

const MAX_REQUEST_BODY_BYTES: usize = 32 * 1024 * 1024;
const MAX_MODEL_PEEK_BYTES: usize = 256 * 1024;

/// Membuat token sesi acak 256-bit (base64url tanpa padding) untuk rute `/s/<token>/...`.
pub fn generate_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

/// Menghasilkan pasangan variabel lingkungan `*_BASE_URL` untuk penyedia proxy yang diminta (T-205).
pub fn proxy_env_vars(
    proxy_base_url: &str,
    session_token: &str,
    provider: &str,
) -> Vec<(String, String)> {
    let base = proxy_base_url.trim_end_matches('/');
    let prov = provider.trim().to_ascii_lowercase();
    match prov.as_str() {
        "openai" => vec![(
            "OPENAI_BASE_URL".to_string(),
            format!("{base}/s/{session_token}/openai/v1"),
        )],
        "anthropic" => vec![(
            "ANTHROPIC_BASE_URL".to_string(),
            format!("{base}/s/{session_token}/anthropic"),
        )],
        "google" | "gemini" => vec![
            (
                "GEMINI_API_BASE_URL".to_string(),
                format!("{base}/s/{session_token}/google"),
            ),
            (
                "GOOGLE_GEMINI_BASE_URL".to_string(),
                format!("{base}/s/{session_token}/google"),
            ),
        ],
        other => vec![(
            "OPENAI_BASE_URL".to_string(),
            format!("{base}/s/{session_token}/{other}/v1"),
        )],
    }
}

struct RegisteredSession {
    session_id: String,
    provider: String,
    breaker: Mutex<CircuitBreakerState>,
}

/// Registri token sesi aktif dan peta URL upstream penyedia LLM.
#[derive(Clone)]
pub struct ProxyRegistry {
    sessions_by_token: Arc<RwLock<HashMap<String, Arc<RegisteredSession>>>>,
    token_by_session: Arc<RwLock<HashMap<String, String>>>,
    upstreams: Arc<RwLock<HashMap<String, String>>>,
    allowed_domains: Arc<RwLock<Vec<String>>>,
}

impl Default for ProxyRegistry {
    fn default() -> Self {
        let mut upstreams = HashMap::new();
        upstreams.insert("openai".to_string(), "https://api.openai.com".to_string());
        upstreams.insert(
            "anthropic".to_string(),
            "https://api.anthropic.com".to_string(),
        );
        upstreams.insert(
            "google".to_string(),
            "https://generativelanguage.googleapis.com".to_string(),
        );
        Self {
            sessions_by_token: Arc::new(RwLock::new(HashMap::new())),
            token_by_session: Arc::new(RwLock::new(HashMap::new())),
            upstreams: Arc::new(RwLock::new(upstreams)),
            allowed_domains: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

impl ProxyRegistry {
    /// Membuat registri baru dengan URL upstream standar (OpenAI, Anthropic, Google).
    pub fn new() -> Self {
        Self::default()
    }

    /// Menetapkan allowlist domain egress kooperatif (kosong = izinkan upstream terdaftar bawaan) (`T-307`).
    pub async fn set_allowed_domains(&self, domains: Vec<String>) {
        let normalized = domains
            .into_iter()
            .map(|d| d.trim().to_ascii_lowercase())
            .filter(|d| !d.is_empty())
            .collect();
        *self.allowed_domains.write().await = normalized;
    }

    async fn is_upstream_domain_allowed(&self, upstream_url: &str) -> bool {
        let allow = self.allowed_domains.read().await;
        if allow.is_empty() {
            return true;
        }
        let Some(host) = extract_url_host(upstream_url) else {
            return false;
        };
        allow
            .iter()
            .any(|allowed| host == *allowed || host.ends_with(&format!(".{allowed}")))
    }

    /// Menimpa atau mendaftarkan URL upstream untuk satu penyedia (berguna untuk pengujian/gateway lokal).
    pub async fn set_upstream(&self, provider: &str, base_url: impl Into<String>) {
        self.upstreams.write().await.insert(
            provider.trim().to_ascii_lowercase(),
            base_url.into().trim_end_matches('/').to_string(),
        );
    }

    /// Mendaftarkan sesi baru ke proxy dan mengembalikan token acak 256-bit yang valid selama sesi hidup.
    pub async fn register_session(
        &self,
        session_id: String,
        provider: String,
        limits: SessionLimits,
    ) -> String {
        let token = generate_session_token();
        let entry = Arc::new(RegisteredSession {
            session_id: session_id.clone(),
            provider: provider.trim().to_ascii_lowercase(),
            breaker: Mutex::new(CircuitBreakerState::new(session_id.clone(), limits)),
        });
        self.sessions_by_token
            .write()
            .await
            .insert(token.clone(), entry);
        self.token_by_session
            .write()
            .await
            .insert(session_id, token.clone());
        token
    }

    /// Mencabut token sesi segera setelah sesi selesai/dibunuh/tripped.
    pub async fn revoke_session(&self, session_id: &str) {
        if let Some(token) = self.token_by_session.write().await.remove(session_id) {
            self.sessions_by_token.write().await.remove(&token);
        }
    }

    async fn lookup_token(&self, token: &str) -> Option<Arc<RegisteredSession>> {
        self.sessions_by_token.read().await.get(token).cloned()
    }

    async fn upstream_for(&self, provider: &str) -> Option<String> {
        self.upstreams
            .read()
            .await
            .get(&provider.trim().to_ascii_lowercase())
            .cloned()
    }
}

#[derive(Clone)]
struct ProxyAppState {
    registry: ProxyRegistry,
    pricing: Arc<PricingTable>,
    client: reqwest::Client,
    telemetry_tx: mpsc::Sender<TelemetryRecord>,
    trip_tx: mpsc::Sender<TripEvent>,
}

/// Handle server proxy yang sedang berjalan di latar belakang.
pub struct ProxyServerHandle {
    addr: SocketAddr,
    shutdown: Arc<Notify>,
}

impl ProxyServerHandle {
    /// Alamat loopback tempat proxy mendengarkan.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    /// URL dasar lokal proxy (`http://127.0.0.1:<port>`).
    pub fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    /// Memicu penutupan bersih server proxy.
    pub fn shutdown(&self) {
        self.shutdown.notify_waiters();
    }
}

/// Menjalankan reverse proxy LLM lokal pada alamat loopback (`127.0.0.1`).
pub async fn start_proxy_server(
    bind_addr: SocketAddr,
    registry: ProxyRegistry,
    pricing: PricingTable,
    telemetry_tx: mpsc::Sender<TelemetryRecord>,
    trip_tx: mpsc::Sender<TripEvent>,
) -> Result<ProxyServerHandle, ProxyError> {
    if !bind_addr.ip().is_loopback() {
        return Err(ProxyError::NonLoopbackBind(bind_addr));
    }

    let client = reqwest::Client::builder()
        .pool_idle_timeout(Duration::from_secs(90))
        .tcp_nodelay(true)
        .no_proxy()
        .connect_timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| ProxyError::ClientBuild(e.to_string()))?;

    let state = ProxyAppState {
        registry,
        pricing: Arc::new(pricing),
        client,
        telemetry_tx,
        trip_tx,
    };

    let app = Router::new()
        .route("/s/{token}/{provider}/{*rest}", any(handle_proxy_with_rest))
        .route("/s/{token}/{provider}", any(handle_proxy_root))
        .with_state(state);

    let listener = TcpListener::bind(bind_addr).await?;
    let addr = listener.local_addr()?;
    let nodelay_listener = NoDelayListener(listener);
    let shutdown = Arc::new(Notify::new());
    let shutdown_wait = Arc::clone(&shutdown);

    tokio::spawn(async move {
        let server = axum::serve(nodelay_listener, app).with_graceful_shutdown(async move {
            shutdown_wait.notified().await;
        });
        if let Err(err) = server.await {
            warn!(error = %err, "local LLM proxy server exited with error");
        }
    });

    Ok(ProxyServerHandle { addr, shutdown })
}

struct NoDelayListener(TcpListener);

impl axum::serve::Listener for NoDelayListener {
    type Io = tokio::net::TcpStream;
    type Addr = SocketAddr;

    async fn accept(&mut self) -> (Self::Io, Self::Addr) {
        loop {
            match self.0.accept().await {
                Ok((stream, addr)) => {
                    let _ = stream.set_nodelay(true);
                    return (stream, addr);
                }
                Err(err) => {
                    warn!(error = %err, "proxy accept error; retrying");
                    tokio::time::sleep(Duration::from_millis(10)).await;
                }
            }
        }
    }

    fn local_addr(&self) -> std::io::Result<Self::Addr> {
        self.0.local_addr()
    }
}

async fn handle_proxy_with_rest(
    State(state): State<ProxyAppState>,
    Path((token, provider, rest)): Path<(String, String, String)>,
    req: Request,
) -> Response {
    forward_request(state, token, provider, Some(rest), req).await
}

async fn handle_proxy_root(
    State(state): State<ProxyAppState>,
    Path((token, provider)): Path<(String, String)>,
    req: Request,
) -> Response {
    forward_request(state, token, provider, None, req).await
}

async fn forward_request(
    state: ProxyAppState,
    token: String,
    provider: String,
    rest: Option<String>,
    req: Request,
) -> Response {
    let Some(session_entry) = state.registry.lookup_token(&token).await else {
        return (StatusCode::UNAUTHORIZED, "invalid or expired session token").into_response();
    };

    let normalized_provider = provider.trim().to_ascii_lowercase();
    let Some(upstream_base) = state.registry.upstream_for(&normalized_provider).await else {
        return (StatusCode::BAD_GATEWAY, "unknown LLM provider").into_response();
    };
    if !state
        .registry
        .is_upstream_domain_allowed(&upstream_base)
        .await
    {
        return (
            StatusCode::FORBIDDEN,
            "upstream domain blocked by egress allowlist",
        )
            .into_response();
    }

    let query = req.uri().query().map(str::to_string);
    let method = req.method().clone();
    let req_headers = req.headers().clone();

    let mut upstream_url = upstream_base;
    if let Some(ref suffix) = rest {
        upstream_url.push('/');
        upstream_url.push_str(suffix.trim_start_matches('/'));
    }
    if let Some(q) = query {
        upstream_url.push('?');
        upstream_url.push_str(&q);
    }

    let body_bytes = match axum::body::to_bytes(req.into_body(), MAX_REQUEST_BODY_BYTES).await {
        Ok(b) => b,
        Err(_) => return (StatusCode::PAYLOAD_TOO_LARGE, "request body too large").into_response(),
    };

    let model_hint = extract_model_hint(&body_bytes);
    let request_bytes_len = body_bytes.len();
    let started = Instant::now();

    let mut upstream_req = state.client.request(method, &upstream_url);
    for (name, value) in &req_headers {
        if is_hop_by_hop_header(name) {
            continue;
        }
        // Teruskan semua header (termasuk Authorization / x-api-key) apa adanya tanpa mencatat nilainya.
        upstream_req = upstream_req.header(name, value);
    }
    if !body_bytes.is_empty() {
        upstream_req = upstream_req.body(body_bytes);
    }

    let upstream_resp = match upstream_req.send().await {
        Ok(r) => r,
        Err(err) => {
            let latency_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
            let rec = TelemetryRecord {
                id: None,
                session_id: session_entry.session_id.clone(),
                provider: session_entry.provider.clone(),
                model: model_hint,
                prompt_tokens: 0,
                completion_tokens: 0,
                estimated_cost_usd: 0.0,
                http_status: Some(502),
                latency_ms: Some(latency_ms),
                ttfb_ms: Some(latency_ms),
                usage_source: latch_core::model::UsageSource::Estimated,
                error_kind: Some("upstream_connect_error".to_string()),
                request_fingerprint: Some("err:upstream_connect".to_string()),
                timestamp: now_ms(),
            };
            emit_telemetry_and_check_breaker(&state, &session_entry, rec);
            debug!(
                session_id = %session_entry.session_id,
                provider = %normalized_provider,
                error = %err,
                "upstream proxy request failed"
            );
            return (StatusCode::BAD_GATEWAY, "upstream LLM request failed").into_response();
        }
    };

    let ttfb_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
    let status = upstream_resp.status();
    let resp_headers = filter_response_headers(upstream_resp.headers());
    let is_sse = upstream_resp
        .headers()
        .get(axum::http::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.to_ascii_lowercase().contains("text/event-stream"));

    let mut upstream_stream = upstream_resp.bytes_stream();
    let (chunk_tx, chunk_rx) = mpsc::channel::<Result<Bytes, std::io::Error>>(32);

    let state_bg = state.clone();
    let session_bg = Arc::clone(&session_entry);
    tokio::spawn(async move {
        let mut parser = ResponseTeeParser::new(is_sse, request_bytes_len, model_hint);
        while let Some(item) = upstream_stream.next().await {
            match item {
                Ok(bytes) => {
                    parser.push_chunk(&bytes);
                    if chunk_tx.send(Ok(bytes)).await.is_err() {
                        break;
                    }
                }
                Err(err) => {
                    let _ = chunk_tx
                        .send(Err(std::io::Error::other(err.to_string())))
                        .await;
                    break;
                }
            }
        }
        let latency_ms = i64::try_from(started.elapsed().as_millis()).unwrap_or(i64::MAX);
        let parsed = parser.finish(status.as_u16());
        let cost = state_bg.pricing.estimate_cost(
            parsed.model.as_deref(),
            parsed.prompt_tokens,
            parsed.completion_tokens,
        );
        let rec = TelemetryRecord {
            id: None,
            session_id: session_bg.session_id.clone(),
            provider: session_bg.provider.clone(),
            model: parsed.model,
            prompt_tokens: parsed.prompt_tokens,
            completion_tokens: parsed.completion_tokens,
            estimated_cost_usd: cost.cost_usd,
            http_status: Some(i32::from(status.as_u16())),
            latency_ms: Some(latency_ms),
            ttfb_ms: Some(ttfb_ms),
            usage_source: parsed.usage_source,
            error_kind: parsed.error_kind,
            request_fingerprint: parsed.loop_fingerprint,
            timestamp: now_ms(),
        };
        emit_telemetry_and_check_breaker(&state_bg, &session_bg, rec);
    });

    let body_stream = futures_util::stream::unfold(chunk_rx, |mut rx| async move {
        rx.recv().await.map(|item| (item, rx))
    });
    let mut response = Response::new(Body::from_stream(body_stream));
    *response.status_mut() = status;
    *response.headers_mut() = resp_headers;
    response
}

fn emit_telemetry_and_check_breaker(
    state: &ProxyAppState,
    session_entry: &RegisteredSession,
    rec: TelemetryRecord,
) {
    let trip_opt = session_entry
        .breaker
        .lock()
        .ok()
        .and_then(|mut cb| cb.record_telemetry(&rec));

    // Penulisan telemetri non-blocking: tidak pernah menahan jalur forwarding.
    if state.telemetry_tx.try_send(rec).is_err() {
        debug!("telemetry channel full or closed; dropping record");
    }

    if let Some(trip) = trip_opt {
        let _ = state.trip_tx.try_send(trip);
    }
}

fn extract_model_hint(body: &[u8]) -> Option<String> {
    if body.is_empty() || body.len() > MAX_MODEL_PEEK_BYTES {
        return None;
    }
    let val: Value = serde_json::from_slice(body).ok()?;
    val.get("model")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty() && s.len() <= 128)
        .map(str::to_string)
}

fn is_hop_by_hop_header(name: &HeaderName) -> bool {
    matches!(
        name.as_str(),
        "host"
            | "connection"
            | "keep-alive"
            | "proxy-authenticate"
            | "proxy-authorization"
            | "te"
            | "trailers"
            | "transfer-encoding"
            | "upgrade"
            | "content-length"
    )
}

fn extract_url_host(url: &str) -> Option<String> {
    let after_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))?;
    let authority = after_scheme.split('/').next()?;
    let host_port = authority.rsplit('@').next()?;
    let host = if host_port.starts_with('[') {
        host_port.split(']').next()?.trim_start_matches('[')
    } else {
        host_port.split(':').next()?
    };
    Some(host.to_ascii_lowercase())
}

fn filter_response_headers(headers: &HeaderMap) -> HeaderMap {
    let mut out = HeaderMap::new();
    for (name, value) in headers {
        if is_hop_by_hop_header(name) && name != axum::http::header::CONTENT_LENGTH {
            continue;
        }
        if let Ok(v) = HeaderValue::from_bytes(value.as_bytes()) {
            out.append(name.clone(), v);
        }
    }
    out
}
