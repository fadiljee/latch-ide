//! Generator PATH shim per sesi untuk pencegatan aksi berisiko seperti `git push` (`T-309`, ADR-0009).

use std::path::{Path, PathBuf};

/// Membuat direktori shim berisi skrip `git` yang mencegat `git push` dan meminta approval daemon.
pub fn write_git_push_shim(
    shim_dir: &Path,
    real_git_path: &str,
    daemon_http_url: &str,
    session_id: &str,
) -> std::io::Result<PathBuf> {
    std::fs::create_dir_all(shim_dir)?;
    let shim_path = shim_dir.join("git");
    let script = format!(
        r#"#!/usr/bin/env sh
set -eu
if [ "${{1:-}}" = "push" ]; then
  if [ -n "${{LATCH_APPROVAL_TOKEN:-}}" ]; then
    RESP=$(curl -sS -X POST "{daemon_http_url}/api/v1/approvals" \
      -H "Authorization: Bearer ${{LATCH_APPROVAL_TOKEN}}" \
      -H "Content-Type: application/json" \
      -d "{{\"session_id\":\"{session_id}\",\"action_type\":\"shell_exec\",\"payload_summary\":\"git $*\",\"wait\":true,\"timeout_secs\":300}}" || true)
    case "$RESP" in
      *'"decision":"approved"'*) ;;
      *) echo "latch: git push denied by approval gate" >&2; exit 126 ;;
    esac
  else
    echo "latch: git push blocked (no approval token)" >&2
    exit 126
  fi
fi
exec "{real_git_path}" -c core.hooksPath=/dev/null "$@"
"#
    );
    std::fs::write(&shim_path, script)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&shim_path, std::fs::Permissions::from_mode(0o755))?;
    }
    Ok(shim_path)
}
