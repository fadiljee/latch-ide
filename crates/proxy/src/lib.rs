#![forbid(unsafe_code)]
//! Reverse proxy LLM lokal untuk telemetri token/biaya, penerusan SSE, dan circuit breaker (T-201..T-210).

pub mod circuit_breaker;
pub mod server;
pub mod sse;

use std::net::SocketAddr;

use latch_core::model::ProxyMode;
use thiserror::Error;

pub use circuit_breaker::{BypassLoopDetector, CircuitBreakerState, SessionLimits, TripEvent};
pub use server::{
    generate_session_token, proxy_env_vars, start_proxy_server, ProxyRegistry, ProxyServerHandle,
};
pub use sse::{hash_fingerprint, ParsedResponseUsage, ResponseTeeParser};

/// Kesalahan subsistem proxy LLM.
#[derive(Debug, Error)]
pub enum ProxyError {
    /// Menolak bind alamat non-loopback demi keamanan bawaan.
    #[error("refusing to bind proxy on non-loopback address: {0}")]
    NonLoopbackBind(SocketAddr),
    /// Gagal menginisialisasi klien HTTP upstream (`reqwest` + `rustls`).
    #[error("failed to build upstream HTTP client: {0}")]
    ClientBuild(String),
    /// Galat I/O socket listener.
    #[error("proxy listener I/O error: {0}")]
    Io(#[from] std::io::Error),
}

/// Konfigurasi rute proxy per sesi.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionProxyRoute {
    /// Token sesi acak (bukan API key).
    pub session_token: String,
    /// Nama penyedia (`openai`, `anthropic`, `google`).
    pub provider: String,
    /// Mode proxy profil agen.
    pub mode: ProxyMode,
}

impl SessionProxyRoute {
    /// Membentuk path dasar rute proxy (`/s/<token>/<provider>`).
    pub fn route_prefix(&self) -> String {
        format!("/s/{}/{}", self.session_token, self.provider)
    }
}

#[cfg(test)]
mod tests {
    use latch_core::model::ProxyMode;

    use super::{proxy_env_vars, SessionProxyRoute};

    #[test]
    fn formats_route_prefix_and_env_vars() {
        let route = SessionProxyRoute {
            session_token: "tok123".to_string(),
            provider: "anthropic".to_string(),
            mode: ProxyMode::Injected,
        };
        assert_eq!(route.route_prefix(), "/s/tok123/anthropic");

        let openai_env = proxy_env_vars("http://127.0.0.1:7879", "tok123", "openai");
        assert_eq!(
            openai_env,
            vec![(
                "OPENAI_BASE_URL".to_string(),
                "http://127.0.0.1:7879/s/tok123/openai/v1".to_string()
            )]
        );

        let ant_env = proxy_env_vars("http://127.0.0.1:7879", "tok123", "anthropic");
        assert_eq!(
            ant_env,
            vec![(
                "ANTHROPIC_BASE_URL".to_string(),
                "http://127.0.0.1:7879/s/tok123/anthropic".to_string()
            )]
        );
    }
}
