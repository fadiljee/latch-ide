//! Dispatcher notifikasi desktop (`notify-send`) dan webhook bertanda tangan HMAC-SHA256 (T-226, FR-23).

use std::time::Duration;

use hmac::{Hmac, Mac};
use latch_core::config::NotificationsConfig;
use latch_core::id::{ms_to_rfc3339, now_ms};
use latch_core::model::SessionStatus;
use serde::Serialize;
use sha2::Sha256;
use tokio::sync::broadcast;
use tracing::{debug, warn};

use crate::session_manager::{DaemonEvent, SessionManager};

type HmacSha256 = Hmac<Sha256>;

/// Payload JSON standar yang dikirim ke webhook (`docs/API.md` §6).
#[derive(Debug, Clone, Serialize)]
pub struct WebhookPayload {
    /// Nama event (`session.completed`, `session.failed`, `session.tripped`).
    pub event: String,
    /// ID sesi terkait.
    pub session_id: String,
    /// Status akhir sesi.
    pub status: SessionStatus,
    /// Alasan trip bila `status == Tripped`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// Total estimasi biaya sesi (USD).
    pub cost_usd: f64,
    /// Stempel waktu RFC 3339 UTC.
    pub timestamp: String,
}

/// Menghitung tanda tangan HMAC-SHA256 dalam format `sha256=<hex>` untuk header `X-Latch-Signature`.
pub fn sign_webhook_payload(secret: &str, body: &[u8]) -> String {
    // HMAC menerima kunci dengan panjang sembarang.
    let Ok(mut mac) = HmacSha256::new_from_slice(secret.as_bytes()) else {
        return String::new();
    };
    mac.update(body);
    let digest = mac.finalize().into_bytes();
    let mut out = String::with_capacity(7 + digest.len() * 2);
    out.push_str("sha256=");
    for b in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{b:02x}");
    }
    out
}

/// Mengirim satu payload webhook secara asinkron dengan tanda tangan HMAC opsional.
pub async fn send_webhook(
    client: &reqwest::Client,
    url: &str,
    secret: Option<&str>,
    payload: &WebhookPayload,
) -> Result<reqwest::StatusCode, reqwest::Error> {
    let body = serde_json::to_vec(payload).unwrap_or_default();
    let mut req = client
        .post(url)
        .header("content-type", "application/json")
        .body(body.clone());
    if let Some(sec) = secret.filter(|s| !s.is_empty()) {
        req = req.header("x-latch-signature", sign_webhook_payload(sec, &body));
    }
    let resp = req.send().await?;
    Ok(resp.status())
}

/// Memulai tugas latar belakang yang mengirim notifikasi desktop & webhook saat sesi selesai/tripped.
pub fn spawn_notification_dispatcher(
    config: NotificationsConfig,
    mut events_rx: broadcast::Receiver<DaemonEvent>,
    manager: SessionManager,
) {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .ok();

    tokio::spawn(async move {
        loop {
            match events_rx.recv().await {
                Ok(DaemonEvent::SessionStatusChanged { session_id, to, .. }) => {
                    if !matches!(
                        to,
                        SessionStatus::Completed
                            | SessionStatus::Failed
                            | SessionStatus::Tripped
                            | SessionStatus::WaitingApproval
                    ) {
                        continue;
                    }
                    let summary = manager
                        .session_telemetry_summary(&session_id)
                        .await
                        .unwrap_or_default();
                    let reason = manager
                        .get_session(&session_id)
                        .await
                        .ok()
                        .and_then(|s| s.trip_reason.map(|r| r.as_str().to_string()));

                    let event_name = format!("session.{}", to.as_str());
                    let title = format!("Latch: {event_name}");
                    let body_text = format!(
                        "Session {} is now {} (cost ${:.4})",
                        &session_id[..session_id.len().min(8)],
                        to.as_str(),
                        summary.estimated_cost_usd
                    );

                    if config.desktop {
                        spawn_desktop_notification(&title, &body_text);
                    }

                    if !config.webhook_url.trim().is_empty() {
                        if let Some(ref http) = client {
                            let secret = if config.webhook_secret_env.trim().is_empty() {
                                None
                            } else {
                                std::env::var(&config.webhook_secret_env).ok()
                            };
                            let payload = WebhookPayload {
                                event: event_name,
                                session_id: session_id.clone(),
                                status: to,
                                reason,
                                cost_usd: summary.estimated_cost_usd,
                                timestamp: ms_to_rfc3339(now_ms()),
                            };
                            if let Err(err) =
                                send_webhook(http, &config.webhook_url, secret.as_deref(), &payload)
                                    .await
                            {
                                warn!(error = %err, "failed delivering notification webhook");
                            }
                        }
                    }
                }
                Ok(DaemonEvent::SessionCreated { .. })
                | Ok(DaemonEvent::ApprovalRequested { .. })
                | Ok(DaemonEvent::ApprovalDecided { .. })
                | Ok(DaemonEvent::PipelineUpdated { .. })
                | Ok(DaemonEvent::FsChanged { .. })
                | Ok(DaemonEvent::SearchResult { .. })
                | Ok(DaemonEvent::PrefsChanged)
                | Ok(DaemonEvent::ReviewCommentAdded { .. }) => {}
                #[cfg(feature = "ide")]
                Ok(DaemonEvent::AgentActivity { .. }) => {}
                Err(broadcast::error::RecvError::Lagged(skipped)) => {
                    debug!(skipped, "notification dispatcher lagged");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            }
        }
    });
}

fn spawn_desktop_notification(title: &str, body: &str) {
    let title = title.to_string();
    let body = body.to_string();
    tokio::spawn(async move {
        let _ = tokio::process::Command::new("notify-send")
            .arg("--app-name=Latch")
            .arg(title)
            .arg(body)
            .status()
            .await;
    });
}

#[cfg(test)]
mod tests {
    use super::sign_webhook_payload;

    #[test]
    fn computes_deterministic_hmac_sha256_signature() {
        let sig = sign_webhook_payload("topsecret", br#"{"event":"session.tripped"}"#);
        assert!(sig.starts_with("sha256="));
        assert_eq!(sig.len(), 7 + 64);
        assert_eq!(
            sig,
            sign_webhook_payload("topsecret", br#"{"event":"session.tripped"}"#)
        );
        assert_ne!(
            sig,
            sign_webhook_payload("other", br#"{"event":"session.tripped"}"#)
        );
    }
}
