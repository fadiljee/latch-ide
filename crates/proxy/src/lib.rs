#![forbid(unsafe_code)]
//! Reverse proxy LLM lokal untuk telemetri token/biaya dan circuit breaker (Fase 2).

use latch_core::model::ProxyMode;
use thiserror::Error;

/// Kesalahan subsistem proxy LLM.
#[derive(Debug, Error)]
pub enum ProxyError {
    /// Proxy belum aktif pada rilis v0.1.
    #[error("local LLM proxy is scheduled for v0.2")]
    NotImplemented,
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

    use super::SessionProxyRoute;

    #[test]
    fn formats_route_prefix() {
        let route = SessionProxyRoute {
            session_token: "tok123".to_string(),
            provider: "anthropic".to_string(),
            mode: ProxyMode::Injected,
        };
        assert_eq!(route.route_prefix(), "/s/tok123/anthropic");
    }
}
