//! Redaksi pola rahasia (API key, bearer token, header sensitif) sebelum pencatatan log/DB.

/// Pola awalan token sensitif yang nilainya harus disamarkan.
const TOKEN_PREFIXES: &[&str] = &["github_pat_", "ghp_", "gho_", "ghu_", "ghs_", "ghr_", "sk-"];

/// Menyamarkan rahasia umum (`sk-...`, `AKIA...`, `ghp_...`, `Bearer ...`, header auth) pada teks.
pub fn redact_secrets(input: &str) -> String {
    let bytes = input.as_bytes();
    let mut out = String::with_capacity(input.len());
    let mut i = 0usize;

    while i < bytes.len() {
        if let Some(next_i) = try_redact_token_prefix(input, i, &mut out) {
            i = next_i;
            continue;
        }
        if let Some(next_i) = try_redact_akia(input, i, &mut out) {
            i = next_i;
            continue;
        }
        if let Some(next_i) = try_redact_bearer(input, i, &mut out) {
            i = next_i;
            continue;
        }
        if let Some(next_i) = try_redact_header_kv(input, i, &mut out) {
            i = next_i;
            continue;
        }

        if let Some(ch) = input[i..].chars().next() {
            out.push(ch);
            i += ch.len_utf8();
        } else {
            break;
        }
    }

    out
}

fn is_boundary_before(input: &str, idx: usize) -> bool {
    if idx == 0 {
        return true;
    }
    input[..idx]
        .chars()
        .next_back()
        .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_')
}

fn is_token_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.' | b'+' | b'/' | b'=')
}

fn try_redact_token_prefix(input: &str, idx: usize, out: &mut String) -> Option<usize> {
    if !is_boundary_before(input, idx) {
        return None;
    }
    let rest = &input[idx..];
    for prefix in TOKEN_PREFIXES {
        if rest.starts_with(prefix) {
            let mut end = idx + prefix.len();
            let bytes = input.as_bytes();
            while end < bytes.len()
                && (bytes[end].is_ascii_alphanumeric() || matches!(bytes[end], b'_' | b'-'))
            {
                end += 1;
            }
            if end - (idx + prefix.len()) >= 8 {
                out.push_str("[REDACTED]");
                return Some(end);
            }
        }
    }
    None
}

fn try_redact_akia(input: &str, idx: usize, out: &mut String) -> Option<usize> {
    if !is_boundary_before(input, idx) || !input[idx..].starts_with("AKIA") {
        return None;
    }
    let bytes = input.as_bytes();
    let end = idx + 20;
    if end <= bytes.len()
        && bytes[idx + 4..end]
            .iter()
            .all(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
    {
        out.push_str("[REDACTED]");
        return Some(end);
    }
    None
}

fn try_redact_bearer(input: &str, idx: usize, out: &mut String) -> Option<usize> {
    if !is_boundary_before(input, idx) {
        return None;
    }
    let rest = &input[idx..];
    if rest.len() <= 7 || !rest[..7].eq_ignore_ascii_case("bearer ") {
        return None;
    }
    let bytes = input.as_bytes();
    let mut token_start = idx + 7;
    while token_start < bytes.len() && bytes[token_start] == b' ' {
        token_start += 1;
    }
    let mut token_end = token_start;
    while token_end < bytes.len() && is_token_char(bytes[token_end]) {
        token_end += 1;
    }
    if token_end > token_start + 3 {
        out.push_str(&input[idx..token_start]);
        out.push_str("[REDACTED]");
        return Some(token_end);
    }
    None
}

fn try_redact_header_kv(input: &str, idx: usize, out: &mut String) -> Option<usize> {
    if !is_boundary_before(input, idx) {
        return None;
    }
    let rest = &input[idx..];
    for key in ["authorization", "x-api-key", "api-key", "api_key"] {
        if rest.len() > key.len() && rest[..key.len()].eq_ignore_ascii_case(key) {
            let bytes = input.as_bytes();
            let mut sep = idx + key.len();
            while sep < bytes.len() && bytes[sep] == b' ' {
                sep += 1;
            }
            if sep < bytes.len() && matches!(bytes[sep], b':' | b'=') {
                sep += 1;
                while sep < bytes.len() && matches!(bytes[sep], b' ' | b'"' | b'\'') {
                    sep += 1;
                }
                let mut val_end = sep;
                while val_end < bytes.len()
                    && !matches!(bytes[val_end], b'\n' | b'\r' | b'"' | b'\'' | b',' | b';')
                {
                    val_end += 1;
                }
                if val_end > sep {
                    out.push_str(&input[idx..sep]);
                    out.push_str("[REDACTED]");
                    return Some(val_end);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::redact_secrets;

    #[test]
    fn redacts_api_keys_and_auth_headers() {
        let line = "Authorization: Bearer sk-test-CANARY1234567890 and x-api-key=secret_abc123";
        let redacted = redact_secrets(line);
        assert!(!redacted.contains("sk-test-CANARY1234567890"));
        assert!(!redacted.contains("secret_abc123"));
        assert!(redacted.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_aws_and_github_tokens() {
        let sample = "aws=AKIAIOSFODNN7EXAMPLE gh=ghp_1234567890abcdef123456";
        let redacted = redact_secrets(sample);
        assert!(!redacted.contains("AKIAIOSFODNN7EXAMPLE"));
        assert!(!redacted.contains("ghp_1234567890abcdef123456"));
    }

    #[test]
    fn preserves_normal_text() {
        let plain = "session started for task: implement store migration";
        assert_eq!(redact_secrets(plain), plain);
    }
}
