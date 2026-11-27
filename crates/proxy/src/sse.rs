//! Parser tee inkremental untuk respons SSE dan JSON dari OpenAI, Anthropic, dan Google (T-202, T-203).

use latch_core::model::UsageSource;
use serde_json::Value;
use sha2::{Digest, Sha256};

const MAX_NON_SSE_BUFFER_BYTES: usize = 512 * 1024;
const MAX_SSE_LINE_BYTES: usize = 256 * 1024;

/// Ringkasan metrik yang diekstrak dari satu respons LLM tanpa menyimpan isi prompt/teks mentah.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedResponseUsage {
    /// Nama model yang dilaporkan respons (jika ada).
    pub model: Option<String>,
    /// Jumlah token prompt/input.
    pub prompt_tokens: i64,
    /// Jumlah token completion/output.
    pub completion_tokens: i64,
    /// Apakah usage dilaporkan upstream atau diestimasi lokal.
    pub usage_source: UsageSource,
    /// Jenis error terstruktur (tanpa isi rahasia) jika respons berupa error.
    pub error_kind: Option<String>,
    /// Hash SHA-256 pendek untuk deteksi loop error/tool-call (tanpa isi mentah).
    pub loop_fingerprint: Option<String>,
}

/// Parser tee non-blocking yang menerima potongan byte respons secara inkremental.
#[derive(Debug, Default)]
pub struct ResponseTeeParser {
    is_sse: bool,
    model: Option<String>,
    prompt_tokens: Option<i64>,
    completion_tokens: Option<i64>,
    error_kind: Option<String>,
    error_signature: Option<String>,
    tool_signature: Option<String>,
    request_bytes: usize,
    response_bytes: usize,
    sse_line_buf: Vec<u8>,
    json_buf: Vec<u8>,
}

impl ResponseTeeParser {
    /// Membuat parser baru berdasarkan apakah respons berjenis `text/event-stream`.
    pub fn new(is_sse: bool, request_bytes: usize, request_model_hint: Option<String>) -> Self {
        Self {
            is_sse,
            model: request_model_hint,
            request_bytes,
            ..Self::default()
        }
    }

    /// Menerima satu potongan byte dari stream upstream tanpa menunda forwarding.
    pub fn push_chunk(&mut self, chunk: &[u8]) {
        self.response_bytes = self.response_bytes.saturating_add(chunk.len());
        if self.is_sse {
            for &b in chunk {
                if b == b'\n' {
                    let line = std::mem::take(&mut self.sse_line_buf);
                    self.process_sse_line(&line);
                } else if self.sse_line_buf.len() < MAX_SSE_LINE_BYTES {
                    self.sse_line_buf.push(b);
                }
            }
        } else if self.json_buf.len() < MAX_NON_SSE_BUFFER_BYTES {
            let remaining = MAX_NON_SSE_BUFFER_BYTES - self.json_buf.len();
            let take = remaining.min(chunk.len());
            self.json_buf.extend_from_slice(&chunk[..take]);
        }
    }

    /// Menyelesaikan parsing setelah stream berakhir dan menghasilkan ringkasan usage.
    pub fn finish(mut self, http_status: u16) -> ParsedResponseUsage {
        if self.is_sse {
            if !self.sse_line_buf.is_empty() {
                let line = std::mem::take(&mut self.sse_line_buf);
                self.process_sse_line(&line);
            }
        } else if !self.json_buf.is_empty() {
            if let Ok(val) = serde_json::from_slice::<Value>(&self.json_buf) {
                self.inspect_json_value(&val);
            }
        }

        if http_status >= 400 && self.error_kind.is_none() {
            self.error_kind = Some(format!("http_{http_status}"));
        }

        let (prompt_tokens, completion_tokens, usage_source) =
            match (self.prompt_tokens, self.completion_tokens) {
                (Some(p), Some(c)) => (p, c, UsageSource::Reported),
                (Some(p), None) => (
                    p,
                    estimate_tokens(self.response_bytes),
                    UsageSource::Reported,
                ),
                (None, Some(c)) => (
                    estimate_tokens(self.request_bytes),
                    c,
                    UsageSource::Reported,
                ),
                (None, None) => (
                    estimate_tokens(self.request_bytes),
                    estimate_tokens(self.response_bytes),
                    UsageSource::Estimated,
                ),
            };

        let loop_fingerprint = if let Some(ref sig) = self.error_signature {
            Some(hash_fingerprint("err", sig))
        } else if http_status >= 400 {
            let fallback = format!(
                "{http_status}:{}",
                self.error_kind.as_deref().unwrap_or("error")
            );
            Some(hash_fingerprint("err", &fallback))
        } else {
            self.tool_signature
                .as_deref()
                .map(|sig| hash_fingerprint("tool", sig))
        };

        ParsedResponseUsage {
            model: self.model,
            prompt_tokens,
            completion_tokens,
            usage_source,
            error_kind: self.error_kind,
            loop_fingerprint,
        }
    }

    fn process_sse_line(&mut self, raw_line: &[u8]) {
        let Ok(line_str) = std::str::from_utf8(raw_line) else {
            return;
        };
        let trimmed = line_str.trim();
        let Some(data) = trimmed.strip_prefix("data:") else {
            return;
        };
        let payload = data.trim();
        if payload.is_empty() || payload == "[DONE]" {
            return;
        }
        if let Ok(val) = serde_json::from_str::<Value>(payload) {
            self.inspect_json_value(&val);
        }
    }

    fn inspect_json_value(&mut self, val: &Value) {
        // Model name: top-level `model` (OpenAI / Anthropic non-stream) atau `message.model` (Anthropic SSE)
        if let Some(m) = val
            .get("model")
            .or_else(|| val.get("message").and_then(|msg| msg.get("model")))
            .and_then(Value::as_str)
        {
            if !m.is_empty() {
                self.model = Some(m.to_string());
            }
        }

        // OpenAI / Anthropic `usage` langsung di root
        if let Some(usage) = val.get("usage") {
            self.extract_usage_object(usage);
        }

        // Anthropic SSE `message_start`: `message.usage.input_tokens`
        if let Some(usage) = val.get("message").and_then(|msg| msg.get("usage")) {
            self.extract_usage_object(usage);
        }

        // Google Gemini `usageMetadata`
        if let Some(meta) = val.get("usageMetadata") {
            if let Some(p) = meta.get("promptTokenCount").and_then(Value::as_i64) {
                self.prompt_tokens = Some(p);
            }
            if let Some(c) = meta.get("candidatesTokenCount").and_then(Value::as_i64) {
                self.completion_tokens = Some(c);
            }
        }

        // Deteksi objek `error` (OpenAI / Anthropic / Google)
        if let Some(err) = val.get("error") {
            let err_type = err
                .get("type")
                .or_else(|| err.get("code"))
                .or_else(|| err.get("status"))
                .and_then(Value::as_str)
                .unwrap_or("upstream_error");
            self.error_kind = Some(err_type.to_string());
            let err_code = err.get("code").and_then(Value::as_str).unwrap_or("");
            self.error_signature = Some(format!("{err_type}:{err_code}"));
        }

        // Deteksi signature tool call (hanya nama tool + panjang argumen, bukan isi rahasia)
        if let Some(choices) = val.get("choices").and_then(Value::as_array) {
            for choice in choices {
                if let Some(tcs) = choice
                    .get("message")
                    .or_else(|| choice.get("delta"))
                    .and_then(|m| m.get("tool_calls"))
                    .and_then(Value::as_array)
                {
                    for tc in tcs {
                        if let Some(name) = tc
                            .get("function")
                            .and_then(|f| f.get("name"))
                            .and_then(Value::as_str)
                        {
                            self.tool_signature = Some(format!("openai:{name}"));
                        }
                    }
                }
            }
        }
        if let Some(block) = val.get("content_block") {
            if block.get("type").and_then(Value::as_str) == Some("tool_use") {
                if let Some(name) = block.get("name").and_then(Value::as_str) {
                    self.tool_signature = Some(format!("anthropic:{name}"));
                }
            }
        }
    }

    fn extract_usage_object(&mut self, usage: &Value) {
        if let Some(p) = usage
            .get("prompt_tokens")
            .or_else(|| usage.get("input_tokens"))
            .and_then(Value::as_i64)
        {
            self.prompt_tokens = Some(p);
        }
        if let Some(c) = usage
            .get("completion_tokens")
            .or_else(|| usage.get("output_tokens"))
            .and_then(Value::as_i64)
        {
            self.completion_tokens = Some(c);
        }
    }
}

/// Menghitung hash SHA-256 pendek (`16` karakter hex) untuk fingerprint tanpa menyimpan teks mentah.
pub fn hash_fingerprint(prefix: &str, material: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    hasher.update(b":");
    hasher.update(material.as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(prefix.len() + 1 + 16);
    hex.push_str(prefix);
    hex.push(':');
    for byte in &digest[..8] {
        use std::fmt::Write as _;
        let _ = write!(&mut hex, "{byte:02x}");
    }
    hex
}

fn estimate_tokens(bytes: usize) -> i64 {
    if bytes == 0 {
        return 0;
    }
    i64::try_from(bytes.div_ceil(4)).unwrap_or(i64::MAX)
}

#[cfg(test)]
mod tests {
    use latch_core::model::UsageSource;

    use super::ResponseTeeParser;

    #[test]
    fn parses_openai_sse_usage_stream() {
        let mut parser = ResponseTeeParser::new(true, 400, None);
        parser.push_chunk(
            b"data: {\"id\":\"1\",\"model\":\"gpt-4o\",\"choices\":[{\"delta\":{\"content\":\"hi\"}}]}\n\n",
        );
        parser.push_chunk(
            b"data: {\"id\":\"1\",\"model\":\"gpt-4o\",\"choices\":[],\"usage\":{\"prompt_tokens\":120,\"completion_tokens\":45}}\n\n",
        );
        parser.push_chunk(b"data: [DONE]\n\n");

        let out = parser.finish(200);
        assert_eq!(out.model.as_deref(), Some("gpt-4o"));
        assert_eq!(out.prompt_tokens, 120);
        assert_eq!(out.completion_tokens, 45);
        assert_eq!(out.usage_source, UsageSource::Reported);
        assert!(out.error_kind.is_none());
    }

    #[test]
    fn parses_anthropic_sse_usage_stream() {
        let mut parser = ResponseTeeParser::new(true, 600, None);
        parser.push_chunk(
            b"event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"model\":\"claude-3-5-sonnet-20241022\",\"usage\":{\"input_tokens\":310}}}\n\n",
        );
        parser.push_chunk(
            b"event: message_delta\ndata: {\"type\":\"message_delta\",\"usage\":{\"output_tokens\":88}}\n\n",
        );

        let out = parser.finish(200);
        assert_eq!(out.model.as_deref(), Some("claude-3-5-sonnet-20241022"));
        assert_eq!(out.prompt_tokens, 310);
        assert_eq!(out.completion_tokens, 88);
        assert_eq!(out.usage_source, UsageSource::Reported);
    }

    #[test]
    fn falls_back_to_estimated_usage_and_extracts_error_fingerprint() {
        let mut parser = ResponseTeeParser::new(false, 200, Some("gpt-4o-mini".to_string()));
        parser.push_chunk(
            br#"{"error":{"type":"rate_limit_error","code":"rate_limit_exceeded","message":"secret sk-test-CANARY"}}"#,
        );
        let out = parser.finish(429);
        assert_eq!(out.usage_source, UsageSource::Estimated);
        assert_eq!(out.error_kind.as_deref(), Some("rate_limit_error"));
        let fp = out.loop_fingerprint.as_deref().unwrap();
        assert!(fp.starts_with("err:"));
        assert!(!fp.contains("CANARY"));
    }
}
