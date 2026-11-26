#![forbid(unsafe_code)]
//! Rute REST/WebSocket Agent IDE dan otorisasi token ber-scope (milestone I1+).

use latch_agent_proto::AgentProtoError;
use latch_fs_service::FsServiceError;
use latch_search::SearchError;
use thiserror::Error;

/// Kesalahan API workbench IDE.
#[derive(Debug, Error)]
pub enum WorkbenchApiError {
    /// Rute HTTP/WS IDE dijadwalkan mulai fase Web v0.2 / I1.
    #[error("workbench HTTP/WS routes are scheduled for v0.2 / IDE I1")]
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

/// Scope izin token workbench sesuai `IDE_SECURITY.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenScope {
    /// Baca status, diff, dan isi berkas.
    Read,
    /// Kontrol sesi (run, pause, resume, kill, approve).
    Control,
    /// Tulis berkas lewat `fs-service`.
    Write,
    /// Akses shell tersandbox.
    ShellSandbox,
    /// Akses shell host (nonaktif bawaan).
    ShellHost,
    /// Administrasi penuh termasuk rotasi token.
    Admin,
}

#[cfg(test)]
mod tests {
    use super::TokenScope;

    #[test]
    fn scope_equality() {
        assert_ne!(TokenScope::Read, TokenScope::Write);
    }
}
