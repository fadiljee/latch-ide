//! Utilitas pembangkit ID UUID v7 dan waktu UTC (epoch milidetik & RFC 3339).

use std::time::{SystemTime, UNIX_EPOCH};

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
    let Ok(dur) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return 0;
    };
    i64::try_from(dur.as_millis()).unwrap_or(i64::MAX)
}

/// Memformat epoch milidetik UTC menjadi string RFC 3339 (algoritma kalender Gregorian Howard Hinnant).
pub fn ms_to_rfc3339(epoch_ms: i64) -> String {
    let total_secs = epoch_ms.div_euclid(1_000);
    let sub_ms = epoch_ms.rem_euclid(1_000);
    let days = total_secs.div_euclid(86_400);
    let secs_of_day = total_secs.rem_euclid(86_400);

    let (year, month, day) = civil_from_days(days);
    let hour = secs_of_day / 3_600;
    let minute = (secs_of_day % 3_600) / 60;
    let second = secs_of_day % 60;

    if sub_ms == 0 {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}+00:00")
    } else {
        format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{sub_ms:03}+00:00")
    }
}

/// Mengonversi jumlah hari sejak 1970-01-01 menjadi `(tahun, bulan, hari)` UTC.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if m <= 2 { y + 1 } else { y };
    (
        year,
        u32::try_from(m).unwrap_or(1),
        u32::try_from(d).unwrap_or(1),
    )
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
        assert_eq!(
            ms_to_rfc3339(1_709_164_800_500),
            "2024-02-29T00:00:00.500+00:00"
        );
        assert_eq!(
            ms_to_rfc3339(1_767_225_600_000),
            "2026-01-01T00:00:00+00:00"
        );
        assert!(now_ms() > 0);
    }
}
