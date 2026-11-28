//! Pemeriksaan versi rilis terbaru (hanya notifikasi, tidak mengunduh atau memasang apa pun).
//!
//! Daemon bertanya ke GitHub Releases paling sering sekali per 6 jam dan menyimpan hasilnya di RAM.
//! Permintaan hanya membawa `User-Agent`; tidak ada token, path proyek, atau data pengguna.
//! Dapat dimatikan lewat `[updates] check = false` di `config.toml`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::Mutex;

/// Repositori sumber rilis.
pub const RELEASES_REPO: &str = "fadiljee/latch-ide";
/// Masa berlaku cache hasil sukses.
const CACHE_TTL: Duration = Duration::from_secs(6 * 60 * 60);
/// Masa berlaku cache saat pengecekan gagal (hindari mengulang terus saat offline).
const FAILURE_TTL: Duration = Duration::from_secs(30 * 60);
/// Batas waktu satu permintaan ke GitHub.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(5);
/// Batas ukuran body respons yang diterima.
const MAX_BODY_BYTES: usize = 512 * 1024;

/// Hasil pemeriksaan versi untuk klien (UI/CLI).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateInfo {
    /// Versi daemon yang sedang berjalan.
    pub current: String,
    /// Versi rilis terbaru bila berhasil diperiksa.
    pub latest: Option<String>,
    /// `true` bila `latest` lebih baru daripada `current`.
    pub update_available: bool,
    /// Tautan halaman rilis terbaru.
    pub release_url: Option<String>,
    /// Apakah pemeriksaan versi aktif di konfigurasi.
    pub check_enabled: bool,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    fetched_at: Instant,
    ttl: Duration,
    latest: Option<String>,
}

/// Pemeriksa versi dengan cache di RAM (aman di-clone).
#[derive(Clone)]
pub struct UpdateChecker {
    enabled: bool,
    current: String,
    cache: Arc<Mutex<Option<CacheEntry>>>,
}

impl UpdateChecker {
    /// Buat pemeriksa untuk versi `current` (tanpa awalan `v`).
    pub fn new(enabled: bool, current: &str) -> Self {
        Self {
            enabled,
            current: current.trim_start_matches('v').to_string(),
            cache: Arc::new(Mutex::new(None)),
        }
    }

    /// Ambil info versi; memakai cache bila masih berlaku. Tidak pernah gagal ke pemanggil.
    pub async fn info(&self) -> UpdateInfo {
        if !self.enabled {
            return self.build(None);
        }
        let mut guard = self.cache.lock().await;
        if let Some(entry) = guard.as_ref() {
            if entry.fetched_at.elapsed() < entry.ttl {
                return self.build(entry.latest.clone());
            }
        }
        let latest = fetch_latest_tag().await;
        let ttl = if latest.is_some() {
            CACHE_TTL
        } else {
            FAILURE_TTL
        };
        *guard = Some(CacheEntry {
            fetched_at: Instant::now(),
            ttl,
            latest: latest.clone(),
        });
        self.build(latest)
    }

    fn build(&self, latest: Option<String>) -> UpdateInfo {
        let update_available = latest
            .as_deref()
            .is_some_and(|tag| is_newer(tag, &self.current));
        let release_url = latest
            .as_ref()
            .map(|tag| format!("https://github.com/{RELEASES_REPO}/releases/tag/{tag}"));
        UpdateInfo {
            current: self.current.clone(),
            latest,
            update_available,
            release_url,
            check_enabled: self.enabled,
        }
    }
}

async fn fetch_latest_tag() -> Option<String> {
    let client = reqwest::Client::builder()
        .timeout(REQUEST_TIMEOUT)
        .user_agent(concat!("latch-ide/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    let url = format!("https://api.github.com/repos/{RELEASES_REPO}/releases/latest");
    let resp = client
        .get(url)
        .header("accept", "application/vnd.github+json")
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let bytes = resp.bytes().await.ok()?;
    if bytes.len() > MAX_BODY_BYTES {
        return None;
    }
    let text = std::str::from_utf8(&bytes).ok()?;
    parse_tag_name(text)
}

/// Ambil `tag_name` dari JSON respons GitHub Releases; tolak tag yang bukan `vX.Y.Z`.
pub fn parse_tag_name(json: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(json).ok()?;
    let tag = value.get("tag_name")?.as_str()?.trim();
    parse_version(tag)?;
    Some(tag.to_string())
}

/// Parse `v1.2.3` / `1.2.3` (sufiks pra-rilis `-rc1` diabaikan) menjadi tuple.
fn parse_version(raw: &str) -> Option<(u64, u64, u64)> {
    let core = raw.trim().trim_start_matches('v');
    let core = core.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some((major, minor, patch))
}

/// `true` bila `latest` benar-benar lebih baru daripada `current`.
pub fn is_newer(latest: &str, current: &str) -> bool {
    match (parse_version(latest), parse_version(current)) {
        (Some(l), Some(c)) => l > c,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_newer, parse_tag_name, UpdateChecker};

    #[test]
    fn compares_versions_numerically() {
        assert!(is_newer("v1.3.0", "1.2.0"));
        assert!(is_newer("v1.10.0", "1.9.9"));
        assert!(!is_newer("v1.2.0", "1.2.0"));
        assert!(!is_newer("v1.1.9", "1.2.0"));
        assert!(!is_newer("garbage", "1.2.0"));
    }

    #[test]
    fn parses_only_valid_release_tags() {
        assert_eq!(
            parse_tag_name(r#"{"tag_name":"v1.3.0"}"#).as_deref(),
            Some("v1.3.0")
        );
        assert_eq!(parse_tag_name(r#"{"tag_name":"nightly"}"#), None);
        assert_eq!(parse_tag_name(r#"{"name":"x"}"#), None);
        assert_eq!(parse_tag_name("not json"), None);
    }

    #[tokio::test]
    async fn disabled_checker_never_reports_update() {
        let checker = UpdateChecker::new(false, "1.2.0");
        let info = checker.info().await;
        assert!(!info.check_enabled);
        assert!(!info.update_available);
        assert_eq!(info.latest, None);
    }

    #[test]
    fn build_marks_update_available() {
        let checker = UpdateChecker::new(true, "v1.2.0");
        let info = checker.build(Some("v1.3.0".to_string()));
        assert!(info.update_available);
        assert_eq!(info.current, "1.2.0");
        assert!(info.release_url.is_some());
    }
}
