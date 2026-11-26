#![forbid(unsafe_code)]
//! Layanan indeks nama berkas fuzzy dan pencarian teks untuk Agent IDE (milestone I2/I3).

use latch_core::id::now_ms;
use thiserror::Error;

/// Kesalahan subsistem pencarian.
#[derive(Debug, Error)]
pub enum SearchError {
    /// Pencarian penuh akan diaktifkan pada milestone IDE I2/I3.
    #[error("search service is scheduled for IDE milestone I2/I3")]
    NotImplemented,
}

/// Metadata awal indeks pencarian kosong untuk satu workspace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexMetadata {
    /// Waktu pembuatan indeks (UTC epoch ms).
    pub created_at: i64,
}

impl IndexMetadata {
    /// Membuat metadata indeks dengan stempel waktu UTC saat ini.
    pub fn new_now() -> Self {
        Self {
            created_at: now_ms(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{IndexMetadata, SearchError};

    #[test]
    fn error_message_is_clear() {
        let err = SearchError::NotImplemented;
        assert!(err.to_string().contains("IDE milestone"));
        assert!(IndexMetadata::new_now().created_at > 0);
    }
}
