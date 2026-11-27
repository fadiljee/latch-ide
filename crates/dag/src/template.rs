//! Templating strict (`{{var}}`) dan pemotongan output dengan penanda (T-403).

use std::collections::HashMap;

use crate::DagError;

/// Batas bawaan pemotongan output template (32 KiB sesuai WORKFLOW_SPEC §4).
pub const DEFAULT_OUTPUT_MAX_BYTES: usize = 32 * 1024;
/// Penanda standar saat output dipotong.
pub const TRUNCATED_MARKER: &str = "\n[...dipotong...]";

/// Memotong teks UTF-8 dengan aman pada batas byte dan menambahkan penanda `[...dipotong...]`.
pub fn truncate_output(raw: &str, max_bytes: usize) -> String {
    if raw.len() <= max_bytes {
        return raw.to_string();
    }
    let mut end = max_bytes;
    while end > 0 && !raw.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{}", &raw[..end], TRUNCATED_MARKER)
}

/// Merender template `{{path.to.var}}` dalam mode **strict** (variabel tak dikenal -> galat).
pub fn render_strict_template(
    template: &str,
    vars: &HashMap<String, String>,
) -> Result<String, DagError> {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after_open = &rest[start + 2..];
        let Some(end) = after_open.find("}}") else {
            return Err(DagError::Template(
                "unclosed '{{' placeholder in template".to_string(),
            ));
        };
        let key = after_open[..end].trim();
        let Some(val) = vars.get(key) else {
            return Err(DagError::UnknownTemplateVar(key.to_string()));
        };
        out.push_str(val);
        rest = &after_open[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{render_strict_template, truncate_output, TRUNCATED_MARKER};
    use crate::DagError;

    #[test]
    fn renders_strict_and_rejects_unknown_variable() {
        let mut vars = HashMap::new();
        vars.insert("task".to_string(), "Add metrics".to_string());
        vars.insert("steps.plan.output".to_string(), "1. Step A".to_string());

        let rendered =
            render_strict_template("Task: {{task}} | Plan: {{ steps.plan.output }}", &vars)
                .unwrap();
        assert_eq!(rendered, "Task: Add metrics | Plan: 1. Step A");

        let err = render_strict_template("Unknown {{steps.missing.output}}", &vars).unwrap_err();
        assert!(matches!(err, DagError::UnknownTemplateVar(ref v) if v == "steps.missing.output"));
    }

    #[test]
    fn truncates_output_with_marker() {
        let s = "abcdef".repeat(10);
        let cut = truncate_output(&s, 10);
        assert!(cut.ends_with(TRUNCATED_MARKER));
        assert!(cut.starts_with("abcdefabcd"));
    }
}
