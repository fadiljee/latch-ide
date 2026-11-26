# Contributing to Latch (`latch-ide`)

Thank you for your interest in contributing to **Latch**!

## Prerequisites

- Latest stable Rust toolchain (`rustup`, MSRV `1.85`), plus `x86_64-unknown-linux-musl` and `musl-tools` for static musl release builds
- `git >= 2.38` (required for `git merge-tree --write-tree`)
- Node.js `>= 22` and `npm` (only needed when working on `web/`; not a runtime dependency)
- Optional tools: `clang` + `lld` (or `mold`), `cargo-nextest`, `cargo-bloat`, `cargo-deny`, `cargo-audit`
- Linux kernel `>= 5.13` to test full Landlock sandboxing (Phase 3)

## Local Setup

```bash
git clone https://github.com/fadiljee/latch-ide.git && cd latch-ide

# Build and run Rust tests
cargo build --workspace
cargo nextest run --workspace

# Optional: Workbench frontend
cd web && npm ci && npm run dev
```

## Development Workflow

1. Open or pick an issue to discuss larger changes before starting.
2. Create a focused branch (`feat/<crate>-<summary>` or `fix/<crate>-<summary>`). Keep each PR scoped to a single task when possible.
3. Write tests alongside (or before) implementation changes.
4. Run all required checks before opening a Pull Request:

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace

# Binary size and RAM budget checks
./scripts/check-binary.sh
./scripts/bench-rss.sh

# If touching web/
(cd web && npm ci && npm run lint && npm run typecheck && npm run test && npm run build)
```

5. Update `README.md` if user-facing behavior changed and add an entry under `[Unreleased]` in [`CHANGELOG.md`](CHANGELOG.md).
6. Open a Pull Request using the template below.

## Coding Standards

### Rust
- **Safety:** `#![forbid(unsafe_code)]` across all crates unless strictly required in `sandbox` or `pty-pool` (every `unsafe` block must include a `// SAFETY:` justification).
- **Error handling:** Use `thiserror` in library crates; reserve `anyhow` for application boundaries (`cli/` and `daemon/`). No `.unwrap()`, `.expect()`, or `panic!()` in production paths (enforced via workspace Clippy lints).
- **Async hygiene:** Never block Tokio worker threads. Synchronous SQLite (`rusqlite`) operations run on the dedicated single-writer actor thread in `latch-store`. All channels must be bounded.
- **Logging & privacy:** Use `tracing` instead of `println!` in libraries/daemon. Never log `Authorization` headers, API keys, or full prompt payloads (`NFR-05`).
- **One-way crate dependencies:** `latch-core` must not depend on any other workspace crate; `latch-tui` must only communicate with `latch-daemon` via the UDS client protocol in `latch-core`.
- **Lean dependencies:** Disable unused `default-features`, avoid pulling `openssl` or `native-tls`, and verify binary size (`< 30 MiB`) and idle RSS (`<= 25 MiB`) impact before adding crates.

### Frontend (`web/`)
- Strict TypeScript (`strict: true`, no unjustified `any`).
- Feature-sliced layout (`src/features/*`), lazy-loaded heavy panels, no Electron/Monaco/heavy UI suites.
- Accessibility (`eslint-plugin-jsx-a11y`), keyboard navigability, and i18n catalog strings (`id`/`en`).
- Keep initial gzipped bundle `<= 600 KiB` (enforced by `web/scripts/check-bundle-size.mjs`).

## Commit Messages

Follow [Conventional Commits](https://www.conventionalcommits.org/): `feat`, `fix`, `docs`, `refactor`, `test`, `perf`, `build`, `ci`, `chore`.
Use the crate or subsystem name as the scope, e.g. `feat(pty-pool): add slow-subscriber drop markers`.

## Pull Request Template

```markdown
## Summary
<what changed and why>

## Related Requirements / Tasks
FR-xx / NFR-xx / T-xx

## Testing
<commands run and key test cases verified>

## Performance & Size Impact
Idle RSS: ... MiB | Per-session RSS: ... MiB | Binary size: ... MiB | Notes: ...

## Checklist
- [ ] `cargo fmt`, `cargo clippy -D warnings`, and tests pass
- [ ] Affected documentation updated
- [ ] `CHANGELOG.md` updated under `[Unreleased]`
- [ ] No secrets, API keys, or sensitive headers logged or stored
```

## Reporting Security Vulnerabilities

Do **not** open a public GitHub issue for security vulnerabilities. Follow the private disclosure process in [`SECURITY.md`](SECURITY.md).

## Code of Conduct

Be respectful, direct, and constructive. Critique code and design trade-offs, never people.
