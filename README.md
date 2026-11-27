<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="assets/icon.png" />
    <source media="(prefers-color-scheme: light)" srcset="assets/icon-light.png" />
    <img src="assets/icon-light.png" alt="Latch Logo" width="280" />
  </picture>
</p>

<h1 align="center">Latch (<code>latch-ide</code>)</h1>

<p align="center">
  An <strong>open-source, 100% Rust</strong> AI coding-agent runtime, orchestrator, and Agent IDE foundation in a single binary (<code>latch</code> / <code>latchd</code>).<br />
  Low memory footprint, <code>git worktree</code> isolation, observable by design, and built without Electron.
</p>

<p align="center">
  <a href="https://github.com/fadiljee/latch-ide/actions/workflows/ci.yml"><img src="https://github.com/fadiljee/latch-ide/actions/workflows/ci.yml/badge.svg" alt="CI" /></a>
  <a href="https://github.com/fadiljee/latch-ide/releases/tag/v0.3.0"><img src="https://img.shields.io/badge/release-v0.3.0-blue" alt="Release: v0.3.0" /></a>
  <img src="https://img.shields.io/badge/rust-1.85%2B-orange" alt="Rust 1.85+" />
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-Apache--2.0-green" alt="License: Apache-2.0" /></a>
  <a href="CONTRIBUTING.md"><img src="https://img.shields.io/badge/PRs-welcome-brightgreen" alt="PRs Welcome" /></a>
</p>

## Why Latch Exists

Running multiple AI coding CLIs (`claude`, `codex`, `opencode`, `gemini`, or custom agent scripts) in parallel creates four recurring problems:

1. **High RAM consumption.** Electron/Chromium-based orchestrators consume hundreds of megabytes to gigabytes as concurrent sessions scale.
2. **Weak isolation.** Autonomous agents without isolated worktrees and OS sandboxing can overwrite each other's changes or access local credentials.
3. **Lack of visibility.** Real-time token usage, cost tracking, and runaway loop protection are rarely built into local workflows.
4. **Manual handoffs.** Coordinating multi-agent workflows (plan → implement → verify → review diff → merge) still relies on copy-paste.

## Implementation Status & Roadmap

| Area | Scope | Phase | Status |
| --- | --- | --- | --- |
| **Daemon & State** | Single binary (`latch` / `latchd`), UDS JSON-RPC 2.0 (`0600` + `SO_PEERCRED`), single-writer SQLite WAL actor, startup reconciliation & orphan worktree cleanup | `v0.1` | Done |
| **Git Engine** | `git >= 2.38` check, isolated `git worktree` + `latch/<project>/<short-id>` branches, paginated diffs (including untracked files), `git merge-tree` conflict simulation, idempotent squash/merge & discard | `v0.1` | Done |
| **PTY Pool** | Parallel `portable-pty` sessions, 1 MiB ring buffer + `0600` rolling logs, 8–16 ms output batching with slow-client backpressure, `vt100` virtual screen + resize, `SIGSTOP`/`SIGCONT`/`kill` | `v0.1` | Done |
| **TUI & CLI** | Full `clap` CLI (`daemon`, `project`, `run`, `session`, `config`, `tui`) & interactive `ratatui` TUI with ANSI terminal pane, diff view, and merge/kill/discard confirmation modals | `v0.1` | Done |
| **Workbench Scaffold** | `web/` foundation (Vite + React + strict TypeScript + Tailwind CSS + Zustand + `id`/`en` i18n + ESLint `jsx-a11y` + Vitest + gzipped bundle-size gate) | `I0` | Done |
| **LLM Proxy & Web PWA** | Local HTTP/SSE reverse proxy (`rustls`), token/cost telemetry (`pricing.toml`), automated circuit breaker (`< 1 s`), REST/WS + embedded Web PWA (`rust-embed`, CSP), HMAC webhooks & SSH remote tunnel | `v0.2` | Done |
| **OS Sandbox & Approvals** | Linux Landlock filesystem jail (`ABI v1–v5`), toolchain presets, sensitive path denylist, egress domain allowlist, PATH shim, interactive Approval Gate (REST/WS + Web Inbox) | `v0.3` | Done |
| **DAG Pipelines & Full IDE** | Declarative `workflow.toml` DAG, retry gates, fan-out arena, idle hibernation, CodeMirror 6 editor, fuzzy/regex search, sandboxed LSP | `v1.0`–`v1.2` | Planned |

## Benchmark Results (`v0.2`)

Measured on Linux x86_64 using [`scripts/check-binary.sh`](scripts/check-binary.sh), [`scripts/bench-rss.sh`](scripts/bench-rss.sh), and [`web/scripts/check-bundle-size.mjs`](web/scripts/check-bundle-size.mjs):

| Metric | Measured | Budget |
| --- | --- | --- |
| Release binary size (`latch`, incl. embedded Web PWA) | **8.18 MiB** | `< 30 MiB` |
| Idle daemon RSS (`VmRSS`) | **13.59 MiB** (`13,916 kB`) | `≤ 25 MiB` |
| Per-session PTY RAM overhead | **0.25 MiB** (`261 kB` / session) | `≤ 5 MiB` |
| Initial Web Workbench bundle (gzip) | **75.81 KiB** (`77,630 B`) | `≤ 600 KiB` |

## Workspace Structure

```text
latch-ide/
├── Cargo.toml                 # Cargo workspace root + strict lints (#![forbid(unsafe_code)], no unwrap/expect)
├── cli/                       # Single binary entrypoint `latch` (with `latchd` argv[0] alias)
├── crates/
│   ├── core/                  # UUID v7 domain models, config/profile validation, secret redaction, UDS protocol
│   ├── store/                 # Bundled SQLite (single-writer actor + 0001 migration)
│   ├── git-engine/            # Worktrees, diffs, merge-tree simulation, squash/merge & discard
│   ├── pty-pool/              # PTY pool, ring buffer, rolling logs, 8–16 ms batching, vt100
│   ├── sandbox/               # SandboxProvider trait & NoopProvider (Landlock in v0.3)
│   ├── daemon/                # SessionManager, UDS server, startup reconciliation, redacted tracing logs
│   ├── tui/                   # Thin ratatui terminal UI client over UDS
│   ├── proxy/                 # Phase 2 stub: local LLM reverse proxy & circuit breaker
│   ├── dag/                   # Phase 4 stub: workflow.toml DAG executor
│   ├── fs-service/            # IDE stub: guarded worktree file service
│   ├── search/                # IDE stub: fuzzy file index & streaming text search
│   ├── lsp-host/              # IDE stub: sandboxed LSP host
│   ├── agent-proto/           # IDE stub: structured agent protocol adapter
│   └── workbench-api/         # IDE stub: REST/WS routes & scoped token auth
├── web/                       # Agent IDE Workbench frontend (React + TypeScript + Vite + Tailwind)
└── scripts/                   # Binary size/static gate (check-binary.sh) and RSS benchmark (bench-rss.sh)
```

## Quick Start (`v0.1`)

### Prerequisites

- Rust stable `>= 1.85` (plus `musl-tools` and target `x86_64-unknown-linux-musl` for static musl builds)
- `git >= 2.38`
- Node.js `>= 22` (optional; build-time only for `web/`)

### Build & Run

```bash
# 1. Build the release binary
cargo build --release -p latch-cli
# Or build & check size (uses static musl automatically if musl-gcc is installed):
./scripts/check-binary.sh

# 2. Validate configuration (~/.config/latch/config.toml & agents.toml)
./target/release/latch config check

# 3. Start the daemon (or pass --foreground for systemd / supervisors)
./target/release/latch daemon start
./target/release/latch daemon status

# 4. Register a Git repository
./target/release/latch project add /path/to/your-repo --name my-repo

# 5. Start an isolated agent session (built-in profiles: `shell`, `sleeper`, or custom entries in agents.toml)
./target/release/latch run --project my-repo --profile shell --task "Add health endpoint"

# 6. Monitor from the CLI or open the interactive TUI
./target/release/latch session ls
./target/release/latch tui

# 7. Inspect diff, run merge-tree conflict check, then squash-merge or discard
./target/release/latch session diff <session-id>
./target/release/latch session merge-check <session-id>
./target/release/latch session merge <session-id> --squash
# Or discard:
./target/release/latch session discard <session-id>
```

> Optional environment overrides for isolated state/testing: `LATCH_CONFIG_DIR`, `LATCH_STATE_DIR`, `LATCH_SOCKET`, and `LATCH_LOG`.

## Development & Verification

```bash
# Formatting, strict lints, and all unit + end-to-end tests
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace

# Binary size gate (< 30 MiB) and daemon RSS budget (<= 25 MiB idle, <= 5 MiB/session)
./scripts/check-binary.sh
./scripts/bench-rss.sh

# Workbench frontend checks (lint + jsx-a11y + typecheck + Vitest + <= 600 KiB gzip gate)
(cd web && npm ci && npm run lint && npm run typecheck && npm run test && npm run build)
```

## Open Source & Contributing

Latch is an open-source project built in the open — bug reports, architectural feedback, tests, documentation improvements, and pull requests are welcome.

- **Issues & Discussions:** Report bugs or propose features on [GitHub Issues](https://github.com/fadiljee/latch-ide/issues).
- **Contributing Guide:** See [`CONTRIBUTING.md`](CONTRIBUTING.md) for local setup, coding standards, PR workflow, and required checks.
- **Security Policy:** See [`SECURITY.md`](SECURITY.md) for private vulnerability disclosure and core security invariants.
- **Changelog:** See [`CHANGELOG.md`](CHANGELOG.md) for release history and upcoming changes.

## Security

- **Secure by default:** the local UDS listener enforces `0600` socket permissions and `SO_PEERCRED` UID verification; future HTTP/WS listeners bind exclusively to `127.0.0.1`.
- **Zero secret leakage (`NFR-05`):** daemon logs and SQLite summaries automatically pass through a secret redaction layer (`sk-...`, `AKIA...`, `ghp_...`, `Bearer ...`, `Authorization`, `x-api-key`).
- **Crash isolation (`NFR-06`):** crashing or `kill -9` on an agent process never brings down the daemon or sibling sessions; daemon restarts reconcile dead PIDs and clean up orphaned worktrees.
- Please report security vulnerabilities privately as described in [`SECURITY.md`](SECURITY.md).

## License

Latch (`latch-ide`) is open-source software licensed under the [Apache License, Version 2.0](LICENSE).
