#![forbid(unsafe_code)]
//! Rute REST/WebSocket Agent IDE dan otorisasi token ber-scope (milestone I1+).

use latch_agent_proto::AgentProtoError;
pub use latch_core::model::TokenScope;
use latch_fs_service::FsServiceError;
use latch_search::SearchError;
use thiserror::Error;

/// Kesalahan API workbench IDE.
#[derive(Debug, Error)]
pub enum WorkbenchApiError {
    /// Rute HTTP/WS IDE belum diaktifkan pada build ini.
    #[error("workbench HTTP/WS route is not implemented yet")]
    NotImplemented,
    /// Galat diteruskan dari `fs-service`.
    #[error(transparent)]
    Fs(#[from] FsServiceError),
    /// Galat diteruskan dari `search`.
    #[error(transparent)]
    Search(#[from] SearchError),
    /// Galat diteruskan dari `agent-proto`.
    #[error(transparent)]
    AgentProto(#[from] AgentProtoError),
}

#[cfg(test)]
mod tests {
    use super::TokenScope;

    #[test]
    fn scope_equality() {
        assert_ne!(TokenScope::Read, TokenScope::Write);
        assert!(TokenScope::owner_default_scopes().contains(&TokenScope::Admin));
        assert!(!TokenScope::owner_default_scopes().contains(&TokenScope::ShellHost));
    }
}
