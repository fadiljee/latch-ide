# Security Policy

## Supported Versions

Latch (`latch-ide`) is currently in pre-release (`0.1.x`). Security fixes are applied to the latest commit on the default branch (`master`).

| Version | Supported |
| --- | --- |
| `0.1.x` / `master` | Yes |

## Reporting a Vulnerability

**Please do not report security vulnerabilities through public GitHub issues, discussions, or pull requests.**

Instead, report vulnerabilities privately through GitHub Security Advisories:

- **Private advisory:** https://github.com/fadiljee/latch-ide/security/advisories/new

Please include:
- A clear description of the vulnerability and its potential impact.
- Step-by-step reproduction instructions or a minimal proof of concept.
- Affected components (crate, CLI command, UDS/HTTP/WS endpoint, sandbox rule, or web UI) and environment details (OS, kernel version, Rust/Git versions).

### Response Targets

- **Initial acknowledgment:** within **72 hours**.
- **Triage & remediation timeline:** coordinated privately with the reporter before public disclosure.

## Core Security Invariants

Latch is designed around defense-in-depth for running autonomous AI coding agents locally:

1. **Local-only control plane by default:** The UDS socket uses `0600` permissions (in a `0700` directory) with `SO_PEERCRED` UID verification. HTTP/WebSocket listeners bind only to `127.0.0.1` (remote access is via SSH port forwarding or Tailscale/WireGuard, never a public relay).
2. **Zero secret persistence (`NFR-05`):** Daemon logs and SQLite tables redact secret patterns (`sk-...`, `AKIA...`, `ghp_...`, `Bearer ...`, `Authorization`, `x-api-key`) and never store provider API keys or full prompts.
3. **Process & crash isolation (`NFR-06`):** Each agent runs in its own isolated `git worktree` and PTY process group; crashing or killing an agent never brings down the daemon or sibling sessions.
4. **Sanitized child environments:** Agent processes start from an allowlisted environment (`PATH`, `HOME`, `TERM`, `LANG`, isolated `TMPDIR`) and only inherit sensitive variables explicitly listed in `pass_env`.
5. **Default-deny filesystem sandboxing (scheduled for `v0.3`):** Linux Landlock allowlisting (`<worktree>` + isolated `TMPDIR` RW, toolchain presets RO) with explicit sensitive-path denylist verification (`~/.ssh`, `~/.aws`, `~/.gnupg`, etc.).

## Scope & Acknowledged Limitations

- Kernel filesystem sandboxing (Landlock) targets Linux (`>= 5.13`) in v1; macOS and Windows run with `NoopProvider` and emit prominent warnings.
- Domain-level outbound filtering via proxy env vars is cooperative unless kernel TCP rules are enabled.
- Prompt injection cannot be prevented completely; isolation and approval gates are designed to limit blast radius.
