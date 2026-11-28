//! Kontrak protokol UDS JSON-RPC 2.0 dan bingkai biner aliran PTY (API.md §3, §5.2).

use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// Stream IPC lokal klien-daemon (Unix Domain Socket di Unix, loopback TCP di Windows).
#[cfg(unix)]
pub type IpcStream = tokio::net::UnixStream;
/// Paruh baca stream IPC lokal.
#[cfg(unix)]
pub type IpcReadHalf = tokio::net::unix::OwnedReadHalf;
/// Paruh tulis stream IPC lokal.
#[cfg(unix)]
pub type IpcWriteHalf = tokio::net::unix::OwnedWriteHalf;

/// Stream IPC lokal klien-daemon (loopback TCP di OS non-Unix).
#[cfg(not(unix))]
pub type IpcStream = tokio::net::TcpStream;
/// Paruh baca stream IPC lokal.
#[cfg(not(unix))]
pub type IpcReadHalf = tokio::net::tcp::OwnedReadHalf;
/// Paruh tulis stream IPC lokal.
#[cfg(not(unix))]
pub type IpcWriteHalf = tokio::net::tcp::OwnedWriteHalf;

async fn connect_ipc(socket_path: &Path) -> Result<IpcStream, std::io::Error> {
    #[cfg(unix)]
    {
        tokio::net::UnixStream::connect(socket_path).await
    }
    #[cfg(not(unix))]
    {
        let raw = std::fs::read_to_string(socket_path)?;
        let addr: std::net::SocketAddr = raw.trim().parse().map_err(|err| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("invalid loopback IPC endpoint: {err}"),
            )
        })?;
        if !addr.ip().is_loopback() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "refusing non-loopback IPC endpoint",
            ));
        }
        tokio::net::TcpStream::connect(addr).await
    }
}

use crate::model::{
    CreatedToken, GcReport, MergeCheckResult, MergeStrategy, PipelineRun, Project, ScopedToken,
    Session, SessionDiff, TokenScope,
};

/// Ukuran bingkai maksimal protokol UDS (16 MiB).
pub const MAX_UDS_FRAME_BYTES: usize = 16 * 1024 * 1024;

/// Kesalahan komunikasi protokol klien-daemon UDS.
#[derive(Debug, Error)]
pub enum ProtocolError {
    /// Gagal koneksi atau I/O pada socket Unix.
    #[error("UDS I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Frame terlalu besar atau korup.
    #[error("invalid UDS frame: {0}")]
    InvalidFrame(String),
    /// Gagal serialisasi atau deserialisasi JSON.
    #[error("JSON codec error: {0}")]
    Json(#[from] serde_json::Error),
    /// Daemon mengembalikan galat JSON-RPC.
    #[error("daemon RPC error ({code}): {message}")]
    Rpc {
        /// Kode galat terstruktur.
        code: i64,
        /// Pesan galat.
        message: String,
    },
}

/// Permintaan JSON-RPC 2.0 ke daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Selalu `"2.0"`.
    pub jsonrpc: String,
    /// Pengenal request.
    pub id: u64,
    /// Nama metode (mis. `project.list`, `session.run`).
    pub method: String,
    /// Parameter opsional JSON.
    #[serde(default)]
    pub params: serde_json::Value,
}

impl JsonRpcRequest {
    /// Membuat permintaan JSON-RPC 2.0 baru.
    pub fn new(id: u64, method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method: method.into(),
            params,
        }
    }
}

/// Objek error JSON-RPC 2.0.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Kode numerik JSON-RPC.
    pub code: i64,
    /// Ringkasan error.
    pub message: String,
    /// Detail tambahan opsional.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// Balasan JSON-RPC 2.0 dari daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Selalu `"2.0"`.
    pub jsonrpc: String,
    /// ID request yang sesuai.
    pub id: u64,
    /// Hasil jika berhasil.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Galat jika gagal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Membangun respons sukses.
    pub fn ok(id: u64, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Membangun respons galat.
    pub fn err(id: u64, code: i64, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// Informasi status dan kesehatan daemon (`daemon.health`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DaemonHealth {
    /// Status layanan (`"ok"`).
    pub status: String,
    /// Versi binary Latch.
    pub version: String,
    /// Nama provider sandbox aktif.
    pub sandbox: String,
    /// PID proses daemon.
    pub pid: u32,
    /// Jumlah sesi aktif saat ini.
    pub active_sessions: usize,
}

/// Parameter `project.add`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectAddParams {
    /// Path repositori Git.
    pub repo_path: String,
    /// Nama pendek proyek opsional.
    pub name: Option<String>,
    /// Branch default opsional.
    pub default_branch: Option<String>,
}

/// Parameter `session.run`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionRunParams {
    /// Nama atau ID proyek.
    pub project: String,
    /// Nama profil di `agents.toml`.
    pub profile: String,
    /// Deskripsi tugas (boleh kosong untuk mode terminal interaktif/REPL).
    #[serde(default)]
    pub task: String,
    /// Ref branch dasar opsional.
    #[serde(default)]
    pub base_ref: Option<String>,
    /// Argumen CLI tambahan khusus sesi (mis. `--mode`, `plan`, `--effort`, `high`, `--model`, `...`).
    #[serde(default)]
    pub extra_args: Vec<String>,
}

/// Parameter `session.list`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionListParams {
    /// Filter nama/ID proyek opsional.
    pub project: Option<String>,
    /// Filter status opsional.
    pub status: Option<String>,
}

/// Parameter umum berbasis ID sesi (`pause`, `resume`, `kill`, `get`, `merge_check`, `discard`, `screen`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionIdParams {
    /// ID sesi (atau awalan unik).
    pub session_id: String,
}

/// Parameter `session.diff`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionDiffParams {
    /// ID sesi.
    pub session_id: String,
    /// Filter satu berkas opsional.
    pub file: Option<String>,
    /// Batas ukuran patch (bawaan 256 KiB).
    pub max_bytes: Option<usize>,
}

/// Parameter `session.merge`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMergeParams {
    /// ID sesi.
    pub session_id: String,
    /// Strategi penggabungan.
    #[serde(default)]
    pub strategy: MergeStrategy,
    /// Pesan commit kustom opsional.
    pub message: Option<String>,
}

/// Parameter `session.resize`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionResizeParams {
    /// ID sesi.
    pub session_id: String,
    /// Kolom terminal.
    pub cols: u16,
    /// Baris terminal.
    pub rows: u16,
}

/// Parameter `pipeline.run` (`API.md` §4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineRunParams {
    /// Nama atau ID proyek.
    pub project: String,
    /// Isi mentah `workflow.toml` (opsional bila `workflow_path` diberikan).
    #[serde(default)]
    pub workflow_toml: Option<String>,
    /// Path berkas `workflow.toml` (opsional bila `workflow_toml` diberikan).
    #[serde(default)]
    pub workflow_path: Option<String>,
    /// Deskripsi tugas pipeline.
    pub task: String,
}

/// Parameter `pipeline.list`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PipelineListParams {
    /// Filter nama/ID proyek opsional.
    pub project: Option<String>,
}

/// Parameter berbasis ID pipeline (`pipeline.get`, `pipeline.cancel`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineIdParams {
    /// ID pipeline (atau awalan unik).
    pub pipeline_id: String,
}

/// Parameter `pipeline.arena_select` (`API.md` §4.5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineArenaSelectParams {
    /// ID pipeline.
    pub pipeline_id: String,
    /// ID langkah `fanout`.
    pub step_id: String,
    /// ID sesi atau nama profil pemenang yang dipilih.
    pub session_id: String,
}

/// Parameter `daemon.gc` (`T-411`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GcParams {
    /// Bila `true`, hanya menghitung kandidat tanpa menghapus data.
    #[serde(default)]
    pub dry_run: bool,
}

/// Parameter pembuatan bearer token ber-scope (`token.create`, `IT-102`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenCreateParams {
    /// Label ramah pengguna (mis. `hp`, `readonly-ci`).
    pub label: String,
    /// Daftar scope izin yang diberikan ke token ini.
    pub scopes: Vec<TokenScope>,
    /// Masa berlaku opsional dalam detik sejak pembuatan.
    #[serde(default)]
    pub expires_in_secs: Option<u64>,
}

/// Parameter pencabutan bearer token ber-scope (`token.revoke`, `IT-102`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TokenRevokeParams {
    /// ID penuh atau prefiks ID token.
    pub id: String,
}

/// Snapshot layar virtual `vt100` dari sesi.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenSnapshot {
    /// ID sesi.
    pub session_id: String,
    /// Lebar layar saat ini.
    pub cols: u16,
    /// Tinggi layar saat ini.
    pub rows: u16,
    /// Teks layar terformat dengan baris.
    pub text: String,
    /// Byte ANSI mentah untuk merender ulang parser klien.
    pub ansi_bytes: Vec<u8>,
}

/// Frame biner pada mode aliran PTY (`session.attach`) sesuai `API.md` §3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachFrame {
    /// `0x01`: byte output terminal terbaru.
    Output(Vec<u8>),
    /// `0x02`: snapshot byte terminal awal/resync.
    Snapshot(Vec<u8>),
    /// `0x03`: penanda jumlah byte yang dibuang karena klien lambat (u64 BE).
    Dropped(u64),
    /// `0x04`: byte input dari klien ke PTY.
    Input(Vec<u8>),
    /// `0x05`: permintaan resize terminal `(cols, rows)` (u16 BE, u16 BE).
    Resize {
        /// Kolom terminal.
        cols: u16,
        /// Baris terminal.
        rows: u16,
    },
    /// `0x06`: sinyal detach bersih dari klien.
    Detach,
}

impl AttachFrame {
    /// Mengkodekan `AttachFrame` ke dalam payload biner untuk dikirim di frame UDS.
    pub fn encode(&self) -> Vec<u8> {
        match self {
            Self::Output(data) => {
                let mut buf = Vec::with_capacity(1 + data.len());
                buf.push(0x01);
                buf.extend_from_slice(data);
                buf
            }
            Self::Snapshot(data) => {
                let mut buf = Vec::with_capacity(1 + data.len());
                buf.push(0x02);
                buf.extend_from_slice(data);
                buf
            }
            Self::Dropped(bytes) => {
                let mut buf = Vec::with_capacity(9);
                buf.push(0x03);
                buf.extend_from_slice(&bytes.to_be_bytes());
                buf
            }
            Self::Input(data) => {
                let mut buf = Vec::with_capacity(1 + data.len());
                buf.push(0x04);
                buf.extend_from_slice(data);
                buf
            }
            Self::Resize { cols, rows } => {
                let mut buf = Vec::with_capacity(5);
                buf.push(0x05);
                buf.extend_from_slice(&cols.to_be_bytes());
                buf.extend_from_slice(&rows.to_be_bytes());
                buf
            }
            Self::Detach => vec![0x06],
        }
    }

    /// Mendekodekan payload biner menjadi `AttachFrame`.
    pub fn decode(raw: &[u8]) -> Result<Self, ProtocolError> {
        let Some((&tag, rest)) = raw.split_first() else {
            return Err(ProtocolError::InvalidFrame(
                "empty attach frame".to_string(),
            ));
        };
        match tag {
            0x01 => Ok(Self::Output(rest.to_vec())),
            0x02 => Ok(Self::Snapshot(rest.to_vec())),
            0x03 => {
                if rest.len() != 8 {
                    return Err(ProtocolError::InvalidFrame(
                        "dropped frame requires 8 bytes".to_string(),
                    ));
                }
                let mut arr = [0u8; 8];
                arr.copy_from_slice(rest);
                Ok(Self::Dropped(u64::from_be_bytes(arr)))
            }
            0x04 => Ok(Self::Input(rest.to_vec())),
            0x05 => {
                if rest.len() != 4 {
                    return Err(ProtocolError::InvalidFrame(
                        "resize frame requires 4 bytes".to_string(),
                    ));
                }
                let cols = u16::from_be_bytes([rest[0], rest[1]]);
                let rows = u16::from_be_bytes([rest[2], rest[3]]);
                Ok(Self::Resize { cols, rows })
            }
            0x06 => Ok(Self::Detach),
            other => Err(ProtocolError::InvalidFrame(format!(
                "unknown attach frame tag 0x{other:02x}"
            ))),
        }
    }
}

/// Menulis satu frame berprefix panjang 4-byte big-endian ke stream async.
pub async fn write_raw_frame<W: AsyncWriteExt + Unpin>(
    writer: &mut W,
    payload: &[u8],
) -> Result<(), ProtocolError> {
    if payload.len() > MAX_UDS_FRAME_BYTES {
        return Err(ProtocolError::InvalidFrame(format!(
            "frame size {} exceeds max {}",
            payload.len(),
            MAX_UDS_FRAME_BYTES
        )));
    }
    let len_u32 = u32::try_from(payload.len())
        .map_err(|_| ProtocolError::InvalidFrame("frame length overflow".to_string()))?;
    writer.write_all(&len_u32.to_be_bytes()).await?;
    writer.write_all(payload).await?;
    writer.flush().await?;
    Ok(())
}

/// Membaca satu frame berprefix panjang 4-byte big-endian dari stream async.
/// Mengembalikan `Ok(None)` bila EOF bersih sebelum header panjang dibaca.
pub async fn read_raw_frame<R: AsyncReadExt + Unpin>(
    reader: &mut R,
) -> Result<Option<Vec<u8>>, ProtocolError> {
    let mut len_buf = [0u8; 4];
    let mut read_so_far = 0usize;
    while read_so_far < 4 {
        let n = reader.read(&mut len_buf[read_so_far..]).await?;
        if n == 0 {
            if read_so_far == 0 {
                return Ok(None);
            }
            return Err(ProtocolError::InvalidFrame(
                "unexpected EOF reading frame length".to_string(),
            ));
        }
        read_so_far += n;
    }

    let frame_len = u32::from_be_bytes(len_buf) as usize;
    if frame_len > MAX_UDS_FRAME_BYTES {
        return Err(ProtocolError::InvalidFrame(format!(
            "frame size {frame_len} exceeds max {MAX_UDS_FRAME_BYTES}"
        )));
    }
    let mut payload = vec![0u8; frame_len];
    reader.read_exact(&mut payload).await?;
    Ok(Some(payload))
}

/// Menulis nilai serialisasi JSON sebagai satu frame UDS.
pub async fn write_json_frame<W: AsyncWriteExt + Unpin, T: Serialize>(
    writer: &mut W,
    value: &T,
) -> Result<(), ProtocolError> {
    let bytes = serde_json::to_vec(value)?;
    write_raw_frame(writer, &bytes).await
}

/// Membaca dan mendeserialisasi nilai JSON dari satu frame UDS.
pub async fn read_json_frame<R: AsyncReadExt + Unpin, T: for<'de> Deserialize<'de>>(
    reader: &mut R,
) -> Result<Option<T>, ProtocolError> {
    let Some(bytes) = read_raw_frame(reader).await? else {
        return Ok(None);
    };
    let parsed = serde_json::from_slice(&bytes)?;
    Ok(Some(parsed))
}

/// Klien UDS tipis untuk `latch-cli` dan `latch-tui` agar tidak bergantung pada `latch-daemon`.
#[derive(Debug, Clone)]
pub struct UdsClient {
    socket_path: std::path::PathBuf,
}

impl UdsClient {
    /// Membuat klien UDS yang mengarah ke path socket daemon.
    pub fn new(socket_path: impl AsRef<Path>) -> Self {
        Self {
            socket_path: socket_path.as_ref().to_path_buf(),
        }
    }

    /// Mengembalikan path socket target.
    pub fn socket_path(&self) -> &Path {
        &self.socket_path
    }

    /// Mengirim satu panggilan JSON-RPC dan menunggu balasan JSON-RPC.
    pub async fn call<P: Serialize, R: for<'de> Deserialize<'de>>(
        &self,
        method: &str,
        params: &P,
    ) -> Result<R, ProtocolError> {
        let mut stream = connect_ipc(&self.socket_path).await?;
        let req = JsonRpcRequest::new(1, method, serde_json::to_value(params)?);
        write_json_frame(&mut stream, &req).await?;

        let resp: JsonRpcResponse = read_json_frame(&mut stream)
            .await?
            .ok_or_else(|| ProtocolError::InvalidFrame("daemon closed connection".to_string()))?;

        if let Some(err) = resp.error {
            return Err(ProtocolError::Rpc {
                code: err.code,
                message: err.message,
            });
        }
        let val = resp.result.unwrap_or(serde_json::Value::Null);
        Ok(serde_json::from_value(val)?)
    }

    /// Membuka stream attach PTY untuk satu sesi (kirim JSON-RPC `session.attach`, lalu pakai frame biner).
    pub async fn attach_stream(&self, session_id: &str) -> Result<IpcStream, ProtocolError> {
        let mut stream = connect_ipc(&self.socket_path).await?;
        let params = serde_json::to_value(SessionIdParams {
            session_id: session_id.to_string(),
        })?;
        let req = JsonRpcRequest::new(1, "session.attach", params);
        write_json_frame(&mut stream, &req).await?;

        let resp: JsonRpcResponse = read_json_frame(&mut stream).await?.ok_or_else(|| {
            ProtocolError::InvalidFrame("daemon closed during attach".to_string())
        })?;

        if let Some(err) = resp.error {
            return Err(ProtocolError::Rpc {
                code: err.code,
                message: err.message,
            });
        }
        Ok(stream)
    }

    /// Mengecek kesehatan daemon.
    pub async fn health(&self) -> Result<DaemonHealth, ProtocolError> {
        self.call("daemon.health", &serde_json::json!({})).await
    }

    /// Mendaftarkan proyek ke daemon.
    pub async fn project_add(&self, params: &ProjectAddParams) -> Result<Project, ProtocolError> {
        self.call("project.add", params).await
    }

    /// Mengambil daftar proyek terdaftar.
    pub async fn project_list(&self) -> Result<Vec<Project>, ProtocolError> {
        self.call("project.list", &serde_json::json!({})).await
    }

    /// Memulai sesi agen baru.
    pub async fn session_run(&self, params: &SessionRunParams) -> Result<Session, ProtocolError> {
        self.call("session.run", params).await
    }

    /// Mengambil daftar sesi.
    pub async fn session_list(
        &self,
        params: &SessionListParams,
    ) -> Result<Vec<Session>, ProtocolError> {
        self.call("session.list", params).await
    }

    /// Mengambil detail satu sesi.
    pub async fn session_get(&self, session_id: &str) -> Result<Session, ProtocolError> {
        self.call(
            "session.get",
            &SessionIdParams {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    /// Menjeda sesi (`SIGSTOP`).
    pub async fn session_pause(&self, session_id: &str) -> Result<Session, ProtocolError> {
        self.call(
            "session.pause",
            &SessionIdParams {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    /// Melanjutkan sesi (`SIGCONT`).
    pub async fn session_resume(&self, session_id: &str) -> Result<Session, ProtocolError> {
        self.call(
            "session.resume",
            &SessionIdParams {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    /// Menghentikan paksa sesi.
    pub async fn session_kill(&self, session_id: &str) -> Result<Session, ProtocolError> {
        self.call(
            "session.kill",
            &SessionIdParams {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    /// Mengambil diff worktree sesi.
    pub async fn session_diff(
        &self,
        params: &SessionDiffParams,
    ) -> Result<SessionDiff, ProtocolError> {
        self.call("session.diff", params).await
    }

    /// Memeriksa potensi konflik merge sesi.
    pub async fn session_merge_check(
        &self,
        session_id: &str,
    ) -> Result<MergeCheckResult, ProtocolError> {
        self.call(
            "session.merge_check",
            &SessionIdParams {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    /// Menggabungkan hasil sesi ke branch target.
    pub async fn session_merge(
        &self,
        params: &SessionMergeParams,
    ) -> Result<Session, ProtocolError> {
        self.call("session.merge", params).await
    }

    /// Membuang sesi dan membersihkan worktree serta branch-nya.
    pub async fn session_discard(&self, session_id: &str) -> Result<(), ProtocolError> {
        let _: serde_json::Value = self
            .call(
                "session.discard",
                &SessionIdParams {
                    session_id: session_id.to_string(),
                },
            )
            .await?;
        Ok(())
    }

    /// Mengambil snapshot layar `vt100` sesi.
    pub async fn session_screen(&self, session_id: &str) -> Result<ScreenSnapshot, ProtocolError> {
        self.call(
            "session.screen",
            &SessionIdParams {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    /// Memulai eksekusi pipeline DAG baru.
    pub async fn pipeline_run(
        &self,
        params: &PipelineRunParams,
    ) -> Result<PipelineRun, ProtocolError> {
        self.call("pipeline.run", params).await
    }

    /// Mengambil daftar pipeline.
    pub async fn pipeline_list(
        &self,
        params: &PipelineListParams,
    ) -> Result<Vec<PipelineRun>, ProtocolError> {
        self.call("pipeline.list", params).await
    }

    /// Mengambil detail satu pipeline.
    pub async fn pipeline_get(&self, pipeline_id: &str) -> Result<PipelineRun, ProtocolError> {
        self.call(
            "pipeline.get",
            &PipelineIdParams {
                pipeline_id: pipeline_id.to_string(),
            },
        )
        .await
    }

    /// Membatalkan pipeline yang sedang berjalan.
    pub async fn pipeline_cancel(&self, pipeline_id: &str) -> Result<PipelineRun, ProtocolError> {
        self.call(
            "pipeline.cancel",
            &PipelineIdParams {
                pipeline_id: pipeline_id.to_string(),
            },
        )
        .await
    }

    /// Memilih pemenang arena pada langkah `fanout` secara manual.
    pub async fn pipeline_arena_select(
        &self,
        params: &PipelineArenaSelectParams,
    ) -> Result<PipelineRun, ProtocolError> {
        self.call("pipeline.arena_select", params).await
    }

    /// Menghibernasi sesi aktif (`T-410`).
    pub async fn session_hibernate(&self, session_id: &str) -> Result<Session, ProtocolError> {
        self.call(
            "session.hibernate",
            &SessionIdParams {
                session_id: session_id.to_string(),
            },
        )
        .await
    }

    /// Menjalankan pembersihan retensi & sampah disk (`latch gc`, `T-411`).
    pub async fn gc(&self, dry_run: bool) -> Result<GcReport, ProtocolError> {
        self.call("daemon.gc", &GcParams { dry_run }).await
    }

    /// Membuat bearer token ber-scope baru (`token.create`, `IT-102`).
    pub async fn token_create(
        &self,
        params: &TokenCreateParams,
    ) -> Result<CreatedToken, ProtocolError> {
        self.call("token.create", params).await
    }

    /// Mengambil daftar metadata token ber-scope (`token.list`, `IT-102`).
    pub async fn token_list(&self) -> Result<Vec<ScopedToken>, ProtocolError> {
        self.call("token.list", &serde_json::json!({})).await
    }

    /// Mencabut bearer token ber-scope berdasarkan ID atau prefiks (`token.revoke`, `IT-102`).
    pub async fn token_revoke(&self, id: &str) -> Result<ScopedToken, ProtocolError> {
        self.call("token.revoke", &TokenRevokeParams { id: id.to_string() })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::{read_raw_frame, write_raw_frame, AttachFrame};

    #[tokio::test]
    async fn roundtrips_length_prefixed_frames_and_attach_codec() {
        let (mut tx, mut rx) = tokio::io::duplex(1024);
        let frames = [
            AttachFrame::Snapshot(b"hello".to_vec()),
            AttachFrame::Output(b"\x1b[32mok\x1b[0m".to_vec()),
            AttachFrame::Dropped(4096),
            AttachFrame::Input(b"ls\n".to_vec()),
            AttachFrame::Resize {
                cols: 120,
                rows: 40,
            },
            AttachFrame::Detach,
        ];

        for f in &frames {
            write_raw_frame(&mut tx, &f.encode()).await.unwrap();
        }
        drop(tx);

        for expected in &frames {
            let raw = read_raw_frame(&mut rx).await.unwrap().unwrap();
            let decoded = AttachFrame::decode(&raw).unwrap();
            assert_eq!(&decoded, expected);
        }
        assert!(read_raw_frame(&mut rx).await.unwrap().is_none());
    }
}
