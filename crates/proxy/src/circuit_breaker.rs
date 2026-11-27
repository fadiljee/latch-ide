//! Detektor loop error/tool-call dan circuit breaker per sesi (T-206, T-207, T-208).

use latch_core::model::{TelemetryRecord, TripReason};

use crate::sse::hash_fingerprint;

/// Ambang batas circuit breaker untuk satu sesi.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SessionLimits {
    /// Jumlah error beruntun identik sebelum trip (`0` = nonaktif).
    pub max_consecutive_errors: u32,
    /// Batas anggaran biaya USD per sesi (`0.0` = tidak dibatasi).
    pub max_session_budget_usd: f64,
}

impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            max_consecutive_errors: 3,
            max_session_budget_usd: 2.0,
        }
    }
}

/// Event trip yang dikirim proxy ke `SessionManager` untuk menghentikan sesi < 1 detik.
#[derive(Debug, Clone, PartialEq)]
pub struct TripEvent {
    /// ID sesi yang melampaui ambang.
    pub session_id: String,
    /// Alasan penghentian.
    pub reason: TripReason,
    /// Ringkasan bebas rahasia untuk audit log.
    pub detail: String,
    /// Total estimasi biaya sesi saat trip terjadi.
    pub total_cost_usd: f64,
}

/// State evaluasi circuit breaker per sesi.
#[derive(Debug, Clone)]
pub struct CircuitBreakerState {
    session_id: String,
    limits: SessionLimits,
    total_cost_usd: f64,
    last_error_fp: Option<String>,
    consecutive_errors: u32,
    tripped: bool,
}

impl CircuitBreakerState {
    /// Membuat state circuit breaker baru untuk satu sesi.
    pub fn new(session_id: String, limits: SessionLimits) -> Self {
        Self {
            session_id,
            limits,
            total_cost_usd: 0.0,
            last_error_fp: None,
            consecutive_errors: 0,
            tripped: false,
        }
    }

    /// Total biaya kumulatif sesi sejauh ini.
    pub fn total_cost_usd(&self) -> f64 {
        self.total_cost_usd
    }

    /// Mengevaluasi satu catatan telemetri dan mengembalikan `Some(TripEvent)` jika ambang terlampaui.
    pub fn record_telemetry(&mut self, rec: &TelemetryRecord) -> Option<TripEvent> {
        if self.tripped {
            return None;
        }

        if rec.estimated_cost_usd > 0.0 && rec.estimated_cost_usd.is_finite() {
            self.total_cost_usd += rec.estimated_cost_usd;
        }

        if self.limits.max_session_budget_usd > 0.0
            && self.total_cost_usd >= self.limits.max_session_budget_usd
        {
            self.tripped = true;
            return Some(TripEvent {
                session_id: self.session_id.clone(),
                reason: TripReason::Budget,
                detail: format!(
                    "session cost ${:.4} exceeded budget ${:.4}",
                    self.total_cost_usd, self.limits.max_session_budget_usd
                ),
                total_cost_usd: self.total_cost_usd,
            });
        }

        let is_error = rec.error_kind.is_some() || rec.http_status.is_some_and(|code| code >= 400);
        if is_error {
            let fp = rec
                .request_fingerprint
                .clone()
                .or_else(|| rec.error_kind.clone())
                .unwrap_or_else(|| "err:unknown".to_string());
            if self.last_error_fp.as_deref() == Some(fp.as_str()) {
                self.consecutive_errors = self.consecutive_errors.saturating_add(1);
            } else {
                self.last_error_fp = Some(fp.clone());
                self.consecutive_errors = 1;
            }

            if self.limits.max_consecutive_errors > 0
                && self.consecutive_errors >= self.limits.max_consecutive_errors
            {
                self.tripped = true;
                return Some(TripEvent {
                    session_id: self.session_id.clone(),
                    reason: TripReason::ErrorLoop,
                    detail: format!(
                        "detected {} consecutive identical upstream errors ({})",
                        self.consecutive_errors, fp
                    ),
                    total_cost_usd: self.total_cost_usd,
                });
            }
        } else {
            self.last_error_fp = None;
            self.consecutive_errors = 0;
        }

        None
    }
}

/// Detektor heuristik berbasis output PTY untuk sesi dengan `proxy_mode = "bypass"` (T-206).
#[derive(Debug, Clone)]
pub struct BypassLoopDetector {
    session_id: String,
    max_consecutive_errors: u32,
    last_line_hash: Option<String>,
    consecutive_hits: u32,
    tripped: bool,
}

impl BypassLoopDetector {
    /// Membuat detektor heuristik untuk sesi bypass.
    pub fn new(session_id: String, max_consecutive_errors: u32) -> Self {
        Self {
            session_id,
            max_consecutive_errors,
            last_line_hash: None,
            consecutive_hits: 0,
            tripped: false,
        }
    }

    /// Memindai potongan byte output PTY; mengembalikan `TripEvent` bila pola error identik berulang.
    pub fn observe_pty_bytes(&mut self, bytes: &[u8]) -> Option<TripEvent> {
        if self.tripped || self.max_consecutive_errors == 0 {
            return None;
        }
        let text = String::from_utf8_lossy(bytes);
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if looks_like_error_line(trimmed) {
                let fp = hash_fingerprint("pty", trimmed);
                if self.last_line_hash.as_deref() == Some(fp.as_str()) {
                    self.consecutive_hits = self.consecutive_hits.saturating_add(1);
                } else {
                    self.last_line_hash = Some(fp.clone());
                    self.consecutive_hits = 1;
                }
                if self.consecutive_hits >= self.max_consecutive_errors {
                    self.tripped = true;
                    return Some(TripEvent {
                        session_id: self.session_id.clone(),
                        reason: TripReason::ErrorLoop,
                        detail: format!(
                            "bypass heuristic detected {} identical PTY error lines ({})",
                            self.consecutive_hits, fp
                        ),
                        total_cost_usd: 0.0,
                    });
                }
            }
        }
        None
    }
}

fn looks_like_error_line(line: &str) -> bool {
    let lower = line.to_ascii_lowercase();
    lower.contains("error:")
        || lower.contains("rate limit")
        || lower.contains("429 too many requests")
        || lower.contains("traceback (most recent call last)")
        || lower.contains("fatal:")
}

#[cfg(test)]
mod tests {
    use latch_core::model::{TelemetryRecord, TripReason, UsageSource};

    use super::{BypassLoopDetector, CircuitBreakerState, SessionLimits};

    fn sample_record(
        cost: f64,
        status: i32,
        err: Option<&str>,
        fp: Option<&str>,
    ) -> TelemetryRecord {
        TelemetryRecord {
            id: None,
            session_id: "ses-1".to_string(),
            provider: "openai".to_string(),
            model: Some("gpt-4o".to_string()),
            prompt_tokens: 100,
            completion_tokens: 50,
            estimated_cost_usd: cost,
            http_status: Some(status),
            latency_ms: Some(12),
            ttfb_ms: Some(5),
            usage_source: UsageSource::Reported,
            error_kind: err.map(str::to_string),
            request_fingerprint: fp.map(str::to_string),
            timestamp: 1,
        }
    }

    #[test]
    fn trips_on_consecutive_identical_errors_and_resets_on_success() {
        let mut cb = CircuitBreakerState::new(
            "ses-1".to_string(),
            SessionLimits {
                max_consecutive_errors: 3,
                max_session_budget_usd: 10.0,
            },
        );
        assert!(cb
            .record_telemetry(&sample_record(
                0.0,
                429,
                Some("rate_limit"),
                Some("err:abc")
            ))
            .is_none());
        assert!(cb
            .record_telemetry(&sample_record(
                0.0,
                429,
                Some("rate_limit"),
                Some("err:abc")
            ))
            .is_none());
        // Sukses di tengah mereset penghitung
        assert!(cb
            .record_telemetry(&sample_record(0.01, 200, None, None))
            .is_none());
        // 3 error identik beruntun memicu trip
        assert!(cb
            .record_telemetry(&sample_record(
                0.0,
                500,
                Some("server_error"),
                Some("err:xyz")
            ))
            .is_none());
        assert!(cb
            .record_telemetry(&sample_record(
                0.0,
                500,
                Some("server_error"),
                Some("err:xyz")
            ))
            .is_none());
        let tripped = cb
            .record_telemetry(&sample_record(
                0.0,
                500,
                Some("server_error"),
                Some("err:xyz"),
            ))
            .unwrap();
        assert_eq!(tripped.reason, TripReason::ErrorLoop);
    }

    #[test]
    fn trips_on_budget_threshold() {
        let mut cb = CircuitBreakerState::new(
            "ses-1".to_string(),
            SessionLimits {
                max_consecutive_errors: 3,
                max_session_budget_usd: 0.50,
            },
        );
        assert!(cb
            .record_telemetry(&sample_record(0.30, 200, None, None))
            .is_none());
        let tripped = cb
            .record_telemetry(&sample_record(0.25, 200, None, None))
            .unwrap();
        assert_eq!(tripped.reason, TripReason::Budget);
        assert!((tripped.total_cost_usd - 0.55).abs() < 1e-6);
    }

    #[test]
    fn bypass_heuristic_trips_on_repeated_error_lines() {
        let mut det = BypassLoopDetector::new("ses-bypass".to_string(), 3);
        assert!(det
            .observe_pty_bytes(b"Error: connection refused\n")
            .is_none());
        assert!(det
            .observe_pty_bytes(b"Error: connection refused\n")
            .is_none());
        let trip = det
            .observe_pty_bytes(b"Error: connection refused\n")
            .unwrap();
        assert_eq!(trip.reason, TripReason::ErrorLoop);
    }
}
