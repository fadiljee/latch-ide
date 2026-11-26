# Changelog

Format mengikuti [Keep a Changelog](https://keepachangelog.com/id-ID/1.1.0/) dan versi mengikuti [Semantic Versioning](https://semver.org/lang/id/). Selama `0.x`, perubahan kontrak boleh terjadi antar minor.

## [Unreleased]

### Added
- Bootstrap Cargo workspace **Latch (`latch-ide`)** (14 crate di `crates/` + `cli/` binary tunggal `latch` dengan alias `latchd`), konfigurasi `.cargo/config.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, dan workflow CI `.github/workflows/ci.yml` (`T-003`, `T-004`).
- Implementasi penuh Fase 1 (`v0.1` Fondasi):
  - `latch-core`: model domain UUID v7, mesin status `can_transition` + proptest, parser & validator `config.toml`/`agents.toml` dengan baris/field, redaksi rahasia (`NFR-05`), trait `SandboxProvider`, dan protokol UDS JSON-RPC 2.0 + `AttachFrame` (`T-101`, `T-102`).
  - `latch-store`: SQLite `rusqlite` (`bundled`) beraktor penulis tunggal, migrasi `0001_initial.sql`, pragma WAL + `integrity_check` saat startup (`T-103`).
  - `latch-git-engine`: validasi `git >= 2.38`, worktree & branch paralel `latch/<project>/<short-id>`, diff terpaginasi (termasuk berkas untracked), simulasi konflik `git merge-tree --write-tree`, serta squash/merge & discard idempoten (`T-110`..`T-114`).
  - `latch-pty-pool`: kolam PTY `portable-pty`, ring buffer 1 MiB + rolling log `0600`, batching 8–16 ms dengan backpressure (`Dropped`), layar virtual `vt100` + resize, dan kontrol `SIGSTOP`/`SIGCONT`/`kill` (`T-120`..`T-124`).
  - `latch-daemon`, `latch-cli`, `latch-tui`, `latch-sandbox`: server UDS `0600` + `SO_PEERCRED`, `SessionManager` dengan rekonsiliasi pasca-crash & pembersihan worktree yatim, logging `tracing` ter-redaksi, CLI lengkap `latch`, dan TUI `ratatui` interaktif (`T-104`, `T-130`..`T-134`, `T-140`..`T-142`, `T-150`).
- Bootstrap kerangka frontend Agent IDE di `web/` (Vite + React + TypeScript strict + Tailwind CSS + Zustand + i18n `id`/`en` + ESLint `jsx-a11y` + Vitest + gerbang ukuran bundel `check-bundle-size.mjs`) (`IT-007`).
- Skrip gerbang performa & ukuran `scripts/check-binary.sh`, `scripts/bench-rss.sh`, dan laporan awal `docs/perf/v0.1-fondasi.md` (`T-004`, `T-151`).
- Paket dokumen Agent IDE (`docs/ide/`) dan ADR-0001..0007, ADR-0012..0017 yang dikonsolidasikan di `docs/`.

## Rencana rilis

- **0.1.0** Fondasi: daemon, SQLite, worktree, PTY pool, diff/merge, TUI.
- **0.2.0** Observabilitas & Web: proxy LLM, telemetri, circuit breaker, Web PWA, notifikasi, mode remote.
- **0.3.0** Keamanan: Landlock, denylist, trait `SandboxProvider`, approval gate.
- **1.0.0** Otomasi penuh: `workflow.toml`, estafet, retry gate, fan-out arena, hibernasi.

<!--
Templat entri rilis:

## [0.1.0] - YYYY-MM-DD
### Added
### Changed
### Deprecated
### Removed
### Fixed
### Security
-->
