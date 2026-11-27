# Changelog

Format mengikuti [Keep a Changelog](https://keepachangelog.com/id-ID/1.1.0/) dan versi mengikuti [Semantic Versioning](https://semver.org/lang/id/). Selama `0.x`, perubahan kontrak boleh terjadi antar minor.

## [Unreleased]

## [1.0.0] - 2026-11-27

### Added
- **Fase 4 — Otomasi Penuh, Pipeline DAG, Arena, Hibernasi & GC (`T-401`–`T-412`):**
  - `latch-dag`: parser & validator `workflow.toml` dengan deteksi siklus Kahn + validasi leluhur `on_fail_retry_step`, templating strict (`{{var}}`) + pemotongan output UTF-8 aman (`[...dipotong...]`), eksekutor DAG (`agent`, `shell`, `fanout`), retry gate dengan umpan balik stdout/stderr, matriks Fan-Out Arena dengan strategi pemilihan pemenang (`manual`, `cheapest_passing`, `fastest_passing`, `smallest_diff_passing`), dan circuit breaker anggaran pipeline (`max_budget_usd`).
  - `latch-store`, `latch-core`, `latch-daemon`, `latch-cli`: persistensi `pipeline_runs`, eksekutor `DaemonStepRunner`, rekonsiliasi pemulihan pipeline pasca-restart (`T-408`), endpoint UDS `pipeline.*`, REST `/api/v1/pipelines*` + `/arena` + `/arena/select`, event WebSocket `pipeline.status`, dan CLI `latch pipeline run|ls|show|cancel|select`.
  - `latch-tui` & `web/`: tampilan **Pipeline DAG & Fan-Out Arena** di TUI (tombol `P`) dan komponen lazy-loaded `PipelineArenaPanel` (1,38 kB gzip, aksesibel, i18n `id`/`en`) di Web PWA (`T-409`).
  - `latch-pty-pool`, `latch-store`, `latch-daemon`, `latch-cli`: pelacakan aktivitas idle PTY, hibernasi idle otomatis & manual (`session.hibernate`, `POST /api/v1/sessions/{id}/hibernate`, `latch session hibernate`) dengan penyimpanan `session_snapshots` dan pemulihan konteks (`resume_args` + `LATCH_RESUME_CONTEXT`) (`T-410`).
  - `latch-daemon` & `latch-cli`: `latch gc [--dry-run]` (`daemon.gc`, `POST /api/v1/gc`) untuk pemangkasan retensi `proxy_telemetry`/`audit_log` serta pembersihan direktori `tmp/` dan log PTY yatim (`T-411`).
  - Installer satu baris `scripts/install.sh` untuk Linux `x86_64` static musl binary dengan verifikasi `SHA256SUMS.txt`.

## [0.3.0] - 2026-11-27

### Added
- **Fase 3 — Keamanan & Approval Gate (`T-301`–`T-312`):**
  - `latch-sandbox`: implementasi nyata `LandlockProvider` (Linux `>= 5.13`, probing ABI v1–v5, `NO_NEW_PRIVS`, default-deny allowlist), preset toolchain (`system`, `rust`, `node`, `python`, `git`), pemfilteran denylist sensitif (`~/.ssh`, `~/.aws`, `~/.gnupg`, dll.), dan laporan `dry_run_policy`.
  - `latch-proxy`: allowlist domain egress kooperatif (`403 Forbidden` di luar allowlist).
  - `latch-store` & `latch-daemon`: tabel & API approval gate (`/api/v1/approvals`, `/api/v1/approvals/{id}/decision`, event WebSocket `approval.requested`/`approval.decided`, wakeup `< 500 ms`, timeout default-deny, audit log) serta generator PATH shim `git push` (`crates/daemon/src/shim.rs`).
  - `web/`: panel **Approvals Inbox** (Approve / Deny) terhubung ke REST/WS daemon.
  - Suite tes keamanan otomatis (`crates/sandbox/tests/landlock_security.rs` dan `crates/daemon/tests/approval_and_security.rs`) serta ADR-0008 & ADR-0009.

## [0.2.0] - 2026-11-27

### Added
- **Fase 2 — Jalur Proxy LLM & Circuit Breaker (`T-201`–`T-210`, `T-250`):**
  - `latch-core`: parser & tabel harga `pricing.toml` (`PricingTable`) dengan pencocokan prefiks model serta model ringkasan `SessionTelemetrySummary`.
  - `latch-store`: penulisan telemetri batch transaksional (`insert_telemetry_batch`), `list_telemetry`, dan agregat biaya/token/error per sesi dengan redaksi rahasia otomatis.
  - `latch-proxy`: server reverse proxy HTTP/SSE loopback (`axum` + `reqwest`/`rustls`) pada rute `/s/<token>/<provider>/...` dengan token 256-bit yang dicabut otomatis saat sesi berakhir, penerusan header otorisasi tanpa pencatatan, tee parser OpenAI/Anthropic/Google, detektor loop error/tool-call (`SHA-256`), heuristik mode bypass, dan circuit breaker anggaran + error beruntun.
  - `latch-daemon` & `latch-pty-pool`: injeksi `OPENAI_BASE_URL`/`ANTHROPIC_BASE_URL`/`GEMINI_API_BASE_URL` pada mode `injected`, dukungan penuh mode `bypass`, batch writer telemetri non-blocking, dan `trip_session` (< 1 detik) beserta audit log.
  - Pengujian integrasi end-to-end, benchmark overhead latensi proxy (`≤ 2 ms`), dan tes kanari rahasia yang memindai berkas SQLite mentah serta log daemon (`NFR-05`).
- **Fase 2 — Jalur Web, REST/WS, Notifikasi & Remote (`T-220`–`T-227`):**
  - `latch-daemon`: server HTTP REST (`/api/v1/*`) + WebSocket (`/api/v1/ws` & `/api/v1/sessions/{id}/pty`) loopback-only dengan bearer token acak 256-bit (`0600`, constant-time compare), pemeriksaan `Origin`, rate-limit gagal auth (`429` + `audit_log`), penyajian aset Web PWA tersemat (`rust-embed`) + header CSP ketat, serta dispatcher notifikasi desktop (`notify-send`) dan webhook bertanda tangan HMAC-SHA256.
  - `web/`: klien REST/WS dengan autentikasi token non-query, `DiffViewer` side-by-side + unified responsif mobile, `manifest.webmanifest`, dan service worker PWA (`sw.js`).
  - `latch-cli`: subcommand `latch token show|rotate` dan `latch remote <user@host>` (SSH port forwarding lokal tanpa listener publik).

## [0.1.0] - 2026-11-27

### Added
- Lisensi open-source `Apache-2.0` (`LICENSE`), sinkronisasi metadata lisensi di `Cargo.toml` dan `web/package.json`, serta pembaruan logo adaptif gelap/terang dan informasi open-source di `README.md`.
- Workflow rilis otomatis `.github/workflows/release.yml` untuk membangun dan memublikasikan binary statis `x86_64-unknown-linux-musl` beserta `SHA256SUMS.txt` saat tag `v*` didorong (`T-152`).
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
