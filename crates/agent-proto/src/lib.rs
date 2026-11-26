#![forbid(unsafe_code)]
//! Adaptor protokol agen terstruktur (ACP / JSON-RPC stdio) untuk Agent IDE (milestone I4).

use latch_core::model::SessionStatus;
use thiserror::Error;

/// Kesalahan adaptor protokol agen terstruktur.
#[derive(Debug, Error)]
pub enum AgentProtoError {
    /// Adaptor ACP dijadwalkan untuk milestone I4.
    #[error("structured agent protocol adapter is scheduled for IDE milestone I4")]
    NotImplemented,
}

/// Memetakan permintaan izin protokol agen ke status sesi Latch.
pub const fn permission_wait_status() -> SessionStatus {
    SessionStatus::WaitingApproval
}

#[cfg(test)]
mod tests {
    use latch_core::model::SessionStatus;

    use super::{permission_wait_status, AgentProtoError};

    #[test]
    fn formats_error() {
        assert!(!AgentProtoError::NotImplemented.to_string().is_empty());
        assert_eq!(permission_wait_status(), SessionStatus::WaitingApproval);
    }
}
