#![forbid(unsafe_code)]
//! Host proses LSP tersandbox dan jembatan JSON-RPC WebSocket (milestone I5).

use latch_pty_pool::PtyError;
use latch_sandbox::NoopProvider;
use thiserror::Error;

/// Kesalahan subsistem LSP host.
#[derive(Debug, Error)]
pub enum LspHostError {
    /// Fitur LSP host dijadwalkan untuk milestone I5.
    #[error("sandboxed LSP host is scheduled for IDE milestone I5")]
    NotImplemented,
    /// Galat proses PTY pembungkus server bahasa.
    #[error("LSP PTY error: {0}")]
    Pty(#[from] PtyError),
}

/// Menyebutkan provider sandbox default yang dipakai host LSP saat inisialisasi stub.
pub fn default_lsp_sandbox_name() -> &'static str {
    latch_core::sandbox::SandboxProvider::name(&NoopProvider)
}

#[cfg(test)]
mod tests {
    use super::{default_lsp_sandbox_name, LspHostError};

    #[test]
    fn formats_error() {
        assert!(LspHostError::NotImplemented.to_string().contains("I5"));
        assert_eq!(default_lsp_sandbox_name(), "noop");
    }
}
