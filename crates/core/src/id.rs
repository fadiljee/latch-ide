//! Utilitas pembangkit ID UUID v7 dan waktu UTC (epoch milidetik & RFC 3339).

use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Membangkitkan ID unik berbasis UUID v7 (monoton berdasarkan waktu).
pub fn new_id() -> String {
    Uuid::now_v7().to_string()
}

/// Mengambil 8 karakter alfanumerik terakhir dari UUID untuk nama branch/worktree ringkas.
pub fn short_id(id: &str) -> String {
    let cleaned: String = id.chars().filter(char::is_ascii_alphanumeric).collect();
    if cleaned.len() <= 8 {
        cleaned
    } else {
        cleaned[cleaned.len() - 8..].to_string()
    }
}

/// Waktu UTC saat ini dalam milidetik sejak epoch Unix.
pub fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

/// Memformat epoch milidetik UTC menjadi string RFC 3339.
pub fn ms_to_rfc3339(epoch_ms: i64) -> String {
    DateTime::<Utc>::from_timestamp_millis(epoch_ms)
        .unwrap_or(DateTime::<Utc>::UNIX_EPOCH)
        .to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::{ms_to_rfc3339, new_id, now_ms, short_id};

    #[test]
    fn generates_valid_uuid_v7_and_short_id() {
        let id = new_id();
        assert_eq!(id.len(), 36);
        let short = short_id(&id);
        assert_eq!(short.len(), 8);
        assert!(short.chars().all(|c| c.is_ascii_alphanumeric()));
    }

    #[test]
    fn formats_utc_timestamp() {
        assert_eq!(ms_to_rfc3339(0), "1970-01-01T00:00:00+00:00");
        assert!(now_ms() > 0);
    }
}
