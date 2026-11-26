//! Abstraksi sandbox lintas OS sesuai `docs/ARCHITECTURE.md` §4.4 dan FR-16.

use std::path::PathBuf;
use std::process::Command;

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Kesalahan penyiapan atau eksekusi sandbox.
#[derive(Debug, Error)]
pub enum SandboxError {
    /// Sandbox diwajibkan oleh konfigurasi, tetapi host tidak mendukungnya.
    #[error("sandbox is required by configuration, but provider '{0}' cannot isolate")]
    Unavailable(String),
    /// Gagal menerapkan aturan sandbox ke proses anak.
    #[error("failed to apply sandbox policy: {0}")]
    ApplyFailed(String),
}

/// Kemampuan sandbox yang tersedia di sistem operasi saat ini.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxCapabilities {
    /// Apakah pembatasan filesystem tingkat kernel tersedia.
    pub filesystem_isolation: bool,
    /// Nama detail mekanisme (mis. `landlock-abi-v3` atau `none`).
    pub detail: String,
}

/// Kebijakan pembatasan sumber daya untuk satu sesi agen.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxPolicy {
    /// Jalur yang diizinkan untuk baca-tulis (+ eksekusi di worktree).
    pub rw_paths: Vec<PathBuf>,
    /// Jalur yang hanya boleh dibaca/eksekusi.
    pub ro_paths: Vec<PathBuf>,
    /// Daftar jalur sensitif yang harus dipastikan tidak masuk izin.
    pub deny_hints: Vec<PathBuf>,
    /// Allowlist domain jaringan keluar.
    pub allow_network: Vec<String>,
}

/// Kontrak implementasi sandbox per platform (`LandlockProvider`, `NoopProvider`, dll.).
pub trait SandboxProvider: Send + Sync {
    /// Nama provider, misalnya `"landlock"` atau `"noop"`.
    fn name(&self) -> &'static str;

    /// Kemampuan sandbox yang didukung pada host saat ini.
    fn capabilities(&self) -> SandboxCapabilities;

    /// Menyiapkan `std::process::Command` agar kebijakan diterapkan sebelum `exec`.
    fn prepare(&self, policy: &SandboxPolicy, cmd: &mut Command) -> Result<(), SandboxError>;
}
