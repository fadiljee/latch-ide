//! Model data domain dan mesin status sesi sesuai `docs/DATA_MODEL.md`.

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Kesalahan parsing enum domain.
#[derive(Debug, Error, PartialEq, Eq)]
#[error("invalid {kind} value: {value}")]
pub struct ParseEnumError {
    /// Nama enum target.
    pub kind: &'static str,
    /// Nilai string mentah yang ditolak.
    pub value: String,
}

/// Proyek Git yang terdaftar di daemon Latch.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Project {
    /// ID unik UUID v7.
    pub id: String,
    /// Nama pendek unik proyek.
    pub name: String,
    /// Path absolut akar repositori Git utama.
    pub repo_path: String,
    /// Nama branch utama target merge (mis. `main` atau `master`).
    pub default_branch: String,
    /// Waktu pembuatan (UTC epoch ms).
    pub created_at: i64,
}

/// Status siklus hidup sesi agen sesuai `DATA_MODEL.md` §5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// Proses agen aktif berjalan di PTY.
    Running,
    /// Proses dihentikan sementara (`SIGSTOP`).
    Paused,
    /// Menunggu keputusan manusia pada approval gate.
    WaitingApproval,
    /// Dihentikan sementara karena idle dengan snapshot tersimpan.
    Hibernated,
    /// Selesai bersih (exit code 0).
    Completed,
    /// Dihentikan paksa oleh circuit breaker.
    Tripped,
    /// Berakhir gagal (exit non-nol, crash, atau rekonsiliasi mati).
    Failed,
}

impl SessionStatus {
    /// Seluruh varian `SessionStatus` untuk pengujian menyeluruh.
    pub const ALL: [Self; 7] = [
        Self::Running,
        Self::Paused,
        Self::WaitingApproval,
        Self::Hibernated,
        Self::Completed,
        Self::Tripped,
        Self::Failed,
    ];

    /// Representasi snake_case untuk penyimpanan SQLite dan API.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Running => "running",
            Self::Paused => "paused",
            Self::WaitingApproval => "waiting_approval",
            Self::Hibernated => "hibernated",
            Self::Completed => "completed",
            Self::Tripped => "tripped",
            Self::Failed => "failed",
        }
    }

    /// Menandakan apakah status sudah merupakan status akhir.
    pub const fn is_terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Tripped | Self::Failed)
    }

    /// Menandakan apakah sesi dianggap masih aktif (berpotensi memegang worktree/proses).
    pub const fn is_active(self) -> bool {
        matches!(
            self,
            Self::Running | Self::Paused | Self::WaitingApproval | Self::Hibernated
        )
    }
}

impl fmt::Display for SessionStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SessionStatus {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "running" => Ok(Self::Running),
            "paused" => Ok(Self::Paused),
            "waiting_approval" => Ok(Self::WaitingApproval),
            "hibernated" => Ok(Self::Hibernated),
            "completed" => Ok(Self::Completed),
            "tripped" => Ok(Self::Tripped),
            "failed" => Ok(Self::Failed),
            _ => Err(ParseEnumError {
                kind: "session_status",
                value: s.to_string(),
            }),
        }
    }
}

/// Aturan transisi status sesi yang diizinkan sesuai `DATA_MODEL.md` §5.
pub const fn can_transition(from: SessionStatus, to: SessionStatus) -> bool {
    matches!(
        (from, to),
        (
            SessionStatus::Running,
            SessionStatus::Paused
                | SessionStatus::WaitingApproval
                | SessionStatus::Tripped
                | SessionStatus::Hibernated
                | SessionStatus::Completed
                | SessionStatus::Failed
        ) | (
            SessionStatus::Paused,
            SessionStatus::Running | SessionStatus::Tripped | SessionStatus::Failed
        ) | (
            SessionStatus::WaitingApproval,
            SessionStatus::Running | SessionStatus::Tripped | SessionStatus::Failed
        ) | (SessionStatus::Hibernated, SessionStatus::Running)
    )
}

/// Mode injeksi proxy untuk profil agen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProxyMode {
    /// Menyuntikkan `*_BASE_URL` ke lingkungan sesi.
    #[default]
    Injected,
    /// Tidak menyuntikkan proxy (untuk CLI OAuth tertutup / certificate pinning).
    Bypass,
}

impl ProxyMode {
    /// Representasi teks untuk SQLite.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Injected => "injected",
            Self::Bypass => "bypass",
        }
    }
}

impl FromStr for ProxyMode {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "injected" => Ok(Self::Injected),
            "bypass" => Ok(Self::Bypass),
            _ => Err(ParseEnumError {
                kind: "proxy_mode",
                value: s.to_string(),
            }),
        }
    }
}

/// Alasan penghentian oleh circuit breaker.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TripReason {
    /// Melampaui anggaran USD sesi/pipeline.
    Budget,
    /// Melampaui batas token.
    Tokens,
    /// Error identik berturut-turut.
    ErrorLoop,
    /// Pengulangan aksi berkas tidak produktif.
    FileLoop,
}

impl TripReason {
    /// Representasi snake_case.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Budget => "budget",
            Self::Tokens => "tokens",
            Self::ErrorLoop => "error_loop",
            Self::FileLoop => "file_loop",
        }
    }
}

impl FromStr for TripReason {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "budget" => Ok(Self::Budget),
            "tokens" => Ok(Self::Tokens),
            "error_loop" => Ok(Self::ErrorLoop),
            "file_loop" => Ok(Self::FileLoop),
            _ => Err(ParseEnumError {
                kind: "trip_reason",
                value: s.to_string(),
            }),
        }
    }
}

/// Satu sesi eksekusi agen pada worktree terisolasi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// ID unik UUID v7.
    pub id: String,
    /// ID proyek induk.
    pub project_id: String,
    /// Nama profil agen yang digunakan.
    pub agent_profile: String,
    /// Path absolut direktori worktree sesi.
    pub worktree_path: String,
    /// Nama branch Git khusus sesi.
    pub branch_name: String,
    /// Status siklus hidup sesi.
    pub status: SessionStatus,
    /// PID proses utama agen bila masih aktif.
    pub pid: Option<i64>,
    /// Waktu mulai (UTC epoch ms).
    pub started_at: i64,
    /// Waktu berakhir (UTC epoch ms) jika berstatus akhir.
    pub ended_at: Option<i64>,
    /// Ringkasan deskripsi tugas (terbatas dan bebas rahasia).
    pub task: Option<String>,
    /// ID pipeline bila dijalankan sebagai bagian dari DAG.
    pub pipeline_run_id: Option<String>,
    /// ID step di dalam pipeline.
    pub step_id: Option<String>,
    /// Kode keluar proses agen.
    pub exit_code: Option<i32>,
    /// Alasan trip bila `status == Tripped`.
    pub trip_reason: Option<TripReason>,
    /// Mode proxy (`injected` atau `bypass`).
    pub proxy_mode: ProxyMode,
    /// Nama provider sandbox (`landlock` atau `noop`).
    pub sandbox_provider: String,
}

/// Jenis aksi pada approval gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalActionType {
    /// Penulisan berkas di luar aturan standar.
    FsWrite,
    /// Eksekusi perintah shell berisiko.
    ShellExec,
    /// Akses jaringan keluar ke domain baru.
    NetEgress,
}

impl ApprovalActionType {
    /// Representasi teks untuk SQLite.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::FsWrite => "fs_write",
            Self::ShellExec => "shell_exec",
            Self::NetEgress => "net_egress",
        }
    }
}

impl FromStr for ApprovalActionType {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "fs_write" => Ok(Self::FsWrite),
            "shell_exec" => Ok(Self::ShellExec),
            "net_egress" => Ok(Self::NetEgress),
            _ => Err(ParseEnumError {
                kind: "approval_action_type",
                value: s.to_string(),
            }),
        }
    }
}

/// Keputusan pada permintaan approval.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDecision {
    /// Menunggu keputusan manusia.
    Pending,
    /// Disetujui.
    Approved,
    /// Ditolak.
    Denied,
}

impl ApprovalDecision {
    /// Representasi teks untuk SQLite.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Denied => "denied",
        }
    }
}

impl FromStr for ApprovalDecision {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "denied" => Ok(Self::Denied),
            _ => Err(ParseEnumError {
                kind: "approval_decision",
                value: s.to_string(),
            }),
        }
    }
}

/// Permintaan persetujuan aksi berisiko.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    /// ID unik UUID v7.
    pub id: String,
    /// ID sesi pemohon.
    pub session_id: String,
    /// Kategori aksi.
    pub action_type: ApprovalActionType,
    /// Ringkasan aksi (sudah disaring, maksimal 1 KB).
    pub payload_summary: String,
    /// Status keputusan saat ini.
    pub decision: ApprovalDecision,
    /// Pelaku keputusan (`tui`, `web:<label>`, `timeout`, `policy`).
    pub decided_by: Option<String>,
    /// Waktu keputusan (UTC epoch ms).
    pub decided_at: Option<i64>,
    /// Waktu pembuatan (UTC epoch ms).
    pub created_at: i64,
    /// Waktu kedaluwarsa (UTC epoch ms).
    pub expires_at: Option<i64>,
}

/// Status eksekusi pipeline DAG.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PipelineStatus {
    /// Menunggu eksekusi.
    Pending,
    /// Sedang berjalan.
    Running,
    /// Selesai sukses.
    Succeeded,
    /// Gagal.
    Failed,
    /// Dihentikan circuit breaker.
    Tripped,
    /// Dibatalkan pengguna.
    Cancelled,
}

impl PipelineStatus {
    /// Representasi teks untuk SQLite.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Running => "running",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Tripped => "tripped",
            Self::Cancelled => "cancelled",
        }
    }
}

impl FromStr for PipelineStatus {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "pending" => Ok(Self::Pending),
            "running" => Ok(Self::Running),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "tripped" => Ok(Self::Tripped),
            "cancelled" => Ok(Self::Cancelled),
            _ => Err(ParseEnumError {
                kind: "pipeline_status",
                value: s.to_string(),
            }),
        }
    }
}

/// Rekaman satu eksekusi pipeline DAG.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PipelineRun {
    /// ID unik UUID v7.
    pub id: String,
    /// ID proyek target.
    pub project_id: String,
    /// Nama workflow.
    pub workflow_name: String,
    /// State serial DAG dalam JSON.
    pub dag_state_json: String,
    /// Status eksekusi pipeline.
    pub status: PipelineStatus,
    /// Waktu dibuat (UTC epoch ms).
    pub created_at: i64,
    /// Waktu pembaruan terakhir (UTC epoch ms).
    pub updated_at: Option<i64>,
    /// Deskripsi tugas pipeline.
    pub task: Option<String>,
    /// Total estimasi biaya USD.
    pub total_cost_usd: f64,
}

/// Sumber penghitungan penggunaan token.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UsageSource {
    /// Dilaporkan langsung oleh API penyedia LLM.
    Reported,
    /// Diperkirakan lokal saat respons tidak menyertakan usage.
    Estimated,
}

impl UsageSource {
    /// Representasi teks untuk SQLite.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Reported => "reported",
            Self::Estimated => "estimated",
        }
    }
}

impl FromStr for UsageSource {
    type Err = ParseEnumError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "reported" => Ok(Self::Reported),
            "estimated" => Ok(Self::Estimated),
            _ => Err(ParseEnumError {
                kind: "usage_source",
                value: s.to_string(),
            }),
        }
    }
}

/// Catatan telemetri satu request proxy LLM (tanpa header rahasia / isi prompt).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TelemetryRecord {
    /// ID auto-increment dari SQLite (`None` sebelum disimpan).
    pub id: Option<i64>,
    /// ID sesi terkait.
    pub session_id: String,
    /// Penyedia (`openai`, `anthropic`, `google`).
    pub provider: String,
    /// Nama model.
    pub model: Option<String>,
    /// Jumlah token input.
    pub prompt_tokens: i64,
    /// Jumlah token output.
    pub completion_tokens: i64,
    /// Estimasi biaya USD.
    pub estimated_cost_usd: f64,
    /// Status HTTP respons.
    pub http_status: Option<i32>,
    /// Latensi total (ms).
    pub latency_ms: Option<i64>,
    /// Time to first byte (ms).
    pub ttfb_ms: Option<i64>,
    /// Sumber metrik usage.
    pub usage_source: UsageSource,
    /// Kategori error bila gagal.
    pub error_kind: Option<String>,
    /// Fingerprint hash untuk deteksi loop (bukan isi prompt).
    pub request_fingerprint: Option<String>,
    /// Waktu pencatatan (UTC epoch ms).
    pub timestamp: i64,
}

/// Ringkasan agregat telemetri proxy untuk satu sesi.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct SessionTelemetrySummary {
    /// ID sesi terkait.
    pub session_id: String,
    /// Jumlah total request LLM tercatat.
    pub request_count: i64,
    /// Total token prompt.
    pub prompt_tokens: i64,
    /// Total token completion.
    pub completion_tokens: i64,
    /// Total estimasi biaya USD.
    pub estimated_cost_usd: f64,
    /// Jumlah request yang berakhir error (`http_status >= 400` atau `error_kind IS NOT NULL`).
    pub error_count: i64,
}

/// Entri log audit sistem (`DATA_MODEL.md` §3.2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuditEntry {
    /// ID auto-increment (`None` sebelum disimpan).
    pub id: Option<i64>,
    /// ID sesi terkait jika ada.
    pub session_id: Option<String>,
    /// Jenis peristiwa (`sandbox_denied`, `approval`, `trip`, `auth_failure`, `merge`, `discard`, ...).
    pub kind: String,
    /// Ringkasan detail bebas rahasia.
    pub detail: String,
    /// Waktu pencatatan (UTC epoch ms).
    pub timestamp: i64,
}

/// Status perubahan satu berkas pada diff worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiffFileStatus {
    /// Berkas baru / untracked.
    Added,
    /// Berkas dimodifikasi.
    Modified,
    /// Berkas dihapus.
    Deleted,
    /// Berkas diganti nama.
    Renamed,
}

/// Ringkasan perubahan satu berkas pada diff worktree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiffFileStat {
    /// Jalur berkas relatif terhadap akar repo.
    pub path: String,
    /// Jalur lama bila `status == Renamed`.
    pub old_path: Option<String>,
    /// Jenis perubahan berkas.
    pub status: DiffFileStatus,
    /// Jumlah baris ditambahkan.
    pub additions: u32,
    /// Jumlah baris dihapus.
    pub deletions: u32,
}

/// Hasil diff lengkap atau terpaginasi untuk sebuah worktree sesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionDiff {
    /// Branch dasar pembanding.
    pub base_branch: String,
    /// Daftar berkas yang berubah.
    pub files: Vec<DiffFileStat>,
    /// Patch unified yang mungkin dipotong sesuai batas ukuran.
    pub patch: String,
    /// Total byte patch sebelum pemotongan.
    pub total_patch_bytes: usize,
    /// Menandakan apakah patch dipotong karena batas ukuran.
    pub truncated: bool,
}

/// Hasil simulasi `git merge-tree --write-tree`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MergeCheckResult {
    /// `true` bila penggabungan bersih tanpa konflik.
    pub clean: bool,
    /// Daftar jalur berkas yang berkonflik.
    pub conflicts: Vec<String>,
}

/// Strategi penggabungan hasil sesi ke branch target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MergeStrategy {
    /// Gabungkan seluruh perubahan sebagai satu commit.
    #[default]
    Squash,
    /// Merge standar (`--no-ff`).
    Merge,
}

/// Snapshot sesi saat hibernasi idle (`session_snapshots`, `T-410`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SessionSnapshot {
    /// ID sesi terkait.
    pub session_id: String,
    /// Byte layar terminal (`ansi_bytes`) saat hibernasi.
    pub screen_blob: Vec<u8>,
    /// Commit HEAD worktree saat hibernasi (opsional).
    pub worktree_head: Option<String>,
    /// Ringkasan `git status` dalam JSON (opsional).
    pub git_status_json: Option<String>,
    /// Ringkasan konteks teks layar untuk resume prompt.
    pub context_summary: Option<String>,
    /// Waktu pembuatan snapshot (UTC epoch ms).
    pub created_at: i64,
}

/// Laporan hasil pembersihan `latch gc` (`T-411`).
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct GcReport {
    /// Apakah hanya simulasi (`--dry-run`) tanpa menghapus data.
    pub dry_run: bool,
    /// Jumlah baris `proxy_telemetry` yang dihapus/akan dihapus.
    pub telemetry_deleted: u64,
    /// Jumlah baris `audit_log` yang dihapus/akan dihapus.
    pub audit_deleted: u64,
    /// Jumlah direktori `tmp/` sesi lama yang dihapus.
    pub tmp_dirs_removed: u64,
    /// Jumlah berkas log PTY yatim yang dihapus.
    pub log_files_removed: u64,
    /// Perkiraan byte disk yang dibebaskan.
    pub bytes_freed: u64,
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::{can_transition, SessionStatus};

    #[test]
    fn verifies_all_49_status_transitions() {
        let mut allowed_count = 0usize;
        for from in SessionStatus::ALL {
            for to in SessionStatus::ALL {
                let ok = can_transition(from, to);
                if ok {
                    allowed_count += 1;
                }
                if from.is_terminal() {
                    assert!(!ok, "terminal state {from} must not transition to {to}");
                }
                if from == to {
                    assert!(!ok, "self-transition {from}->{to} must be disallowed");
                }
            }
        }
        // Running->6, Paused->3, WaitingApproval->3, Hibernated->1 = 13 transisi valid
        assert_eq!(allowed_count, 13);
    }

    proptest! {
        #[test]
        fn terminal_states_never_transition(
            from_idx in 0usize..SessionStatus::ALL.len(),
            to_idx in 0usize..SessionStatus::ALL.len(),
        ) {
            let from = SessionStatus::ALL[from_idx];
            let to = SessionStatus::ALL[to_idx];
            if from.is_terminal() {
                prop_assert!(!can_transition(from, to));
            }
            let parsed = from.as_str().parse::<SessionStatus>();
            prop_assert_eq!(parsed, Ok(from));
        }
    }
}
