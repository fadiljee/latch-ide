#![forbid(unsafe_code)]
//! Adaptor protokol agen terstruktur (ACP / JSON-RPC 2.0 stdio), antrean pesan,
//! dan pembatas lampiran konteks aman (`IT-303`, `IT-404`, `IT-405`, `IT-407`).

use std::collections::VecDeque;

use latch_core::id::{new_id, now_ms};
use latch_core::model::{ApprovalActionType, SessionStatus};
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Batas maksimum ukuran satu pesan ke agen (`IDE_API.md` §11: 64 KiB).
pub const MAX_AGENT_MESSAGE_BYTES: usize = 64 * 1024;
/// Batas maksimum item antrean pesan per sesi (`IT-407`).
pub const MAX_QUEUE_LEN: usize = 32;

/// Kesalahan adaptor protokol agen terstruktur.
#[derive(Debug, Error)]
pub enum AgentProtoError {
    /// Pesan kosong atau melebihi batas 64 KiB.
    #[error("invalid agent message: {0}")]
    InvalidMessage(String),
    /// Frame JSON-RPC ACP tidak valid.
    #[error("invalid ACP JSON-RPC frame: {0}")]
    InvalidFrame(String),
    /// Antrean pesan sesi penuh.
    #[error("session message queue is full (max {0})")]
    QueueFull(usize),
}

/// Jenis aktivitas terstruktur atau pemanggilan alat agen (`IT-404`, `IT-406`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    /// Pesan pengguna terkirim ke agen.
    UserMessage,
    /// Fragmen teks balasan agen.
    AgentDelta,
    /// Pemanggilan alat dimulai.
    ToolStarted,
    /// Pemanggilan alat selesai.
    ToolFinished,
    /// Permintaan izin terstruktur dipetakan ke approval gate (`IT-405`).
    PermissionRequested,
    /// Sinyal interupsi dikirim ke agen.
    Interrupted,
}

/// Satu entri aliran aktivitas / tool-call sesi (`GET /sessions/{sid}/activity`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActivityEvent {
    /// ID unik event.
    pub id: String,
    /// ID sesi terkait.
    pub session_id: String,
    /// Jenis aktivitas.
    pub kind: ActivityKind,
    /// Nama alat atau sumber (mis. `edit_file`, `run_command`, `user`, `acp`).
    pub source: String,
    /// Ringkasan singkat ramah UI.
    pub summary: String,
    /// Status sukses opsional (untuk `tool_finished`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ok: Option<bool>,
    /// Waktu UTC epoch ms.
    pub timestamp: i64,
}

/// Permintaan izin terstruktur dari agen ACP (`IT-405`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AcpPermissionRequest {
    /// ID permintaan JSON-RPC ACP.
    pub request_id: String,
    /// Nama alat yang meminta izin (mis. `fs_write`, `shell_exec`, `net_egress`).
    pub tool: String,
    /// Ringkasan argumen/payload.
    pub summary: String,
}

impl AcpPermissionRequest {
    /// Memetakan jenis alat ACP ke `ApprovalActionType` pada approval gate Latch (`FR-17`).
    pub fn to_approval_action_type(&self) -> ApprovalActionType {
        let lower = self.tool.to_ascii_lowercase();
        if lower.contains("net") || lower.contains("http") || lower.contains("fetch") {
            ApprovalActionType::NetEgress
        } else if lower.contains("write") || lower.contains("edit") || lower.contains("fs") {
            ApprovalActionType::FsWrite
        } else {
            ApprovalActionType::ShellExec
        }
    }
}

/// Hasil parsing satu baris NDJSON/JSON-RPC ACP dari proses agen (seperti `agy acp`) (`IT-404`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AcpIncomingEvent {
    /// Delta teks respons agen.
    MessageDelta { text: String },
    /// Alat mulai dieksekusi.
    ToolStarted { tool: String, summary: String },
    /// Alat selesai dieksekusi.
    ToolFinished { tool: String, ok: bool },
    /// Permintaan izin yang wajib dijembatani ke approval gate Latch.
    Permission(AcpPermissionRequest),
    /// Frame lain yang diketahui valid tetapi diabaikan.
    Ignored,
}

/// Mem-parse satu baris pesan JSON-RPC 2.0 ACP dari stdout agen (`IT-404`).
pub fn parse_acp_line(raw_line: &str) -> Result<AcpIncomingEvent, AgentProtoError> {
    let trimmed = raw_line.trim();
    if trimmed.is_empty() {
        return Ok(AcpIncomingEvent::Ignored);
    }
    let val: serde_json::Value =
        serde_json::from_str(trimmed).map_err(|e| AgentProtoError::InvalidFrame(e.to_string()))?;
    let method = val
        .get("method")
        .and_then(|m| m.as_str())
        .unwrap_or_default();
    let params = val.get("params").unwrap_or(&serde_json::Value::Null);

    match method {
        "session/update" | "agent/messageDelta" => {
            let text = params
                .get("text")
                .or_else(|| params.get("delta"))
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            if text.is_empty() {
                Ok(AcpIncomingEvent::Ignored)
            } else {
                Ok(AcpIncomingEvent::MessageDelta { text })
            }
        }
        "tool/started" | "session/toolStarted" => Ok(AcpIncomingEvent::ToolStarted {
            tool: params
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string(),
            summary: params
                .get("summary")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        }),
        "tool/finished" | "session/toolFinished" => Ok(AcpIncomingEvent::ToolFinished {
            tool: params
                .get("tool")
                .and_then(|v| v.as_str())
                .unwrap_or("tool")
                .to_string(),
            ok: params.get("ok").and_then(|v| v.as_bool()).unwrap_or(true),
        }),
        "session/requestPermission" | "permission/request" => {
            let request_id = val
                .get("id")
                .map(|id| match id {
                    serde_json::Value::String(s) => s.clone(),
                    other => other.to_string(),
                })
                .unwrap_or_else(new_id);
            Ok(AcpIncomingEvent::Permission(AcpPermissionRequest {
                request_id,
                tool: params
                    .get("tool")
                    .or_else(|| params.get("kind"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("shell_exec")
                    .to_string(),
                summary: params
                    .get("summary")
                    .or_else(|| params.get("command"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("ACP permission request")
                    .to_string(),
            }))
        }
        _ => Ok(AcpIncomingEvent::Ignored),
    }
}

/// Membangun balasan JSON-RPC 2.0 ACP untuk keputusan izin (`approved`/`denied`) (`IT-405`).
pub fn encode_acp_permission_response(request_id: &str, approved: bool) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": request_id,
        "result": {
            "outcome": if approved { "approved" } else { "denied" }
        }
    })
    .to_string()
}

/// Membangun permintaan JSON-RPC 2.0 ACP `session/prompt` untuk mengirim pesan ke agen ACP.
pub fn encode_acp_prompt_request(id: u64, prompt: &str) -> String {
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": id,
        "method": "session/prompt",
        "params": { "prompt": prompt }
    })
    .to_string()
}

/// Lampiran berkas konteks (`@file`) dengan pembatas keamanan eksplisit (`IT-303`, `IDE_SECURITY.md` IT13).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextAttachment {
    /// Jalur relatif berkas di worktree.
    pub path: String,
    /// Isi teks berkas (dibatasi agar total pesan <= 64 KiB).
    pub content: String,
}

/// Menggabungkan teks pesan pengguna dengan lampiran `@file` memakai pembatas jelas agar tahan prompt injection (`IT13`).
pub fn format_message_with_attachments(
    text: &str,
    attachments: &[ContextAttachment],
) -> Result<String, AgentProtoError> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AgentProtoError::InvalidMessage(
            "message text must not be empty".to_string(),
        ));
    }
    let mut out = String::from(trimmed);
    for att in attachments {
        out.push_str("\n\n--- BEGIN ATTACHED FILE (UNTRUSTED REPO CONTENT): ");
        out.push_str(&att.path);
        out.push_str(" ---\n");
        out.push_str(&att.content);
        if !att.content.ends_with('\n') {
            out.push('\n');
        }
        out.push_str("--- END ATTACHED FILE ---");
    }
    if out.len() > MAX_AGENT_MESSAGE_BYTES {
        return Err(AgentProtoError::InvalidMessage(format!(
            "message payload ({} bytes) exceeds 64 KiB limit",
            out.len()
        )));
    }
    Ok(out)
}

/// Satu item dalam antrean pesan sesi (`IT-407`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedMessage {
    /// ID unik item antrean.
    pub id: String,
    /// Isi pesan terformat siap kirim.
    pub text: String,
    /// Waktu masuk antrean (UTC epoch ms).
    pub queued_at: i64,
}

/// Antrean pesan FIFO berbatas per sesi (`IT-407`).
#[derive(Debug, Default)]
pub struct SessionMessageQueue {
    items: VecDeque<QueuedMessage>,
}

impl SessionMessageQueue {
    /// Menambahkan pesan ke belakang antrean.
    pub fn push(&mut self, text: String) -> Result<QueuedMessage, AgentProtoError> {
        if self.items.len() >= MAX_QUEUE_LEN {
            return Err(AgentProtoError::QueueFull(MAX_QUEUE_LEN));
        }
        let item = QueuedMessage {
            id: new_id(),
            text,
            queued_at: now_ms(),
        };
        self.items.push_back(item.clone());
        Ok(item)
    }

    /// Mengambil pesan terdepan untuk segera dikirim.
    pub fn pop_front(&mut self) -> Option<QueuedMessage> {
        self.items.pop_front()
    }

    /// Daftar snapshot seluruh pesan yang sedang mengantre.
    pub fn list(&self) -> Vec<QueuedMessage> {
        self.items.iter().cloned().collect()
    }
}

/// Memetakan permintaan izin protokol agen ke status sesi Latch.
pub const fn permission_wait_status() -> SessionStatus {
    SessionStatus::WaitingApproval
}

#[cfg(test)]
mod tests {
    use latch_core::model::{ApprovalActionType, SessionStatus};

    use super::{
        encode_acp_permission_response, encode_acp_prompt_request, format_message_with_attachments,
        parse_acp_line, permission_wait_status, AcpIncomingEvent, ContextAttachment,
        SessionMessageQueue,
    };

    #[test]
    fn parses_acp_events_and_maps_permissions() {
        assert_eq!(permission_wait_status(), SessionStatus::WaitingApproval);

        let delta = parse_acp_line(
            r#"{"jsonrpc":"2.0","method":"session/update","params":{"text":"hello"}}"#,
        )
        .unwrap();
        assert_eq!(
            delta,
            AcpIncomingEvent::MessageDelta {
                text: "hello".to_string()
            }
        );

        let tool = parse_acp_line(r#"{"jsonrpc":"2.0","method":"tool/started","params":{"tool":"edit_file","summary":"src/main.rs"}}"#).unwrap();
        assert_eq!(
            tool,
            AcpIncomingEvent::ToolStarted {
                tool: "edit_file".to_string(),
                summary: "src/main.rs".to_string(),
            }
        );

        let perm = parse_acp_line(r#"{"jsonrpc":"2.0","id":"req-9","method":"session/requestPermission","params":{"tool":"shell_exec","command":"git push"}}"#).unwrap();
        match perm {
            AcpIncomingEvent::Permission(req) => {
                assert_eq!(req.request_id, "req-9");
                assert_eq!(req.to_approval_action_type(), ApprovalActionType::ShellExec);
            }
            other => assert!(matches!(other, AcpIncomingEvent::Permission(_))),
        }

        assert!(encode_acp_permission_response("req-9", true).contains("\"approved\""));
        assert!(encode_acp_prompt_request(1, "fix tests").contains("session/prompt"));
    }

    #[test]
    fn formats_context_attachments_and_manages_fifo_queue() {
        let msg = format_message_with_attachments(
            "Review this file",
            &[ContextAttachment {
                path: "src/lib.rs".to_string(),
                content: "pub fn ok() {}".to_string(),
            }],
        )
        .unwrap();
        assert!(msg.contains("BEGIN ATTACHED FILE (UNTRUSTED REPO CONTENT): src/lib.rs"));
        assert!(msg.contains("END ATTACHED FILE"));

        let mut q = SessionMessageQueue::default();
        let first = q.push("step 1".to_string()).unwrap();
        let _second = q.push("step 2".to_string()).unwrap();
        assert_eq!(q.list().len(), 2);
        assert_eq!(q.pop_front().unwrap().id, first.id);
        assert_eq!(q.list().len(), 1);
    }
}
