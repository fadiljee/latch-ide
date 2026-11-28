# Changelog

Format mengikuti [Keep a Changelog](https://keepachangelog.com/id-ID/1.1.0/) dan versi mengikuti [Semantic Versioning](https://semver.org/lang/id/). Selama `0.x`, perubahan kontrak boleh terjadi antar minor.

## [Unreleased]

### Added
- **Pustaka Profil Agen & Konfigurasi Contoh (`T-904`, `FR-07`):**
  - Menambahkan contoh siap pakai yang tervalidasi oleh tes unit: `examples/agents.toml` (OAuth default + API Key proxy), `examples/config.toml`, `examples/mcp.toml`, dan `examples/workflow.toml` (DAG + Fan-Out Arena + retry gate).
- **Katalog Tarif Model LLM (`T-903`, `R-10`):**
  - Memperbarui tabel harga bawaan `PricingTable` di `latch-core` (24 model termasuk `gpt-5`, `o3`/`o4-mini`, `claude-opus-4-1`/`claude-sonnet-4-5`, `gemini-2.5-*`, `deepseek-*`, `kimi-k2`, `qwen3-coder`), validasi angka finit, dan contoh `examples/pricing.toml` tervalidasi oleh tes unit.
- **Kompatibilitas OS macOS & Windows di CI (`T-901`, `NFR-07`):**
  - Memindahkan dependensi `nix` di `latch-pty-pool` dan `latch-daemon` ke `[target.'cfg(unix)'.dependencies]`, menyediakan abstraksi `IpcStream` lintas OS (`UnixStream` di Unix, loopback `TcpStream` di Windows) di `latch-core`/`latch-daemon`/`latch-tui`, serta menambahkan matrix job CI `os-compat` (`macos-latest` & `windows-latest`).
- **Keamanan Rantai Pasok CI (`T-900`, `SECURITY §11`):**
  - Menambahkan `.github/dependabot.yml` (pembaruan mingguan untuk Cargo workspace, `desktop/src-tauri`, `web` npm, dan GitHub Actions) serta job CI `security-audit` (`cargo deny check`, `cargo audit`, `npm audit --omit=dev --audit-level=high`) dengan jadwal mingguan.
  - Menambal `RUSTSEC-2026-0459` (CVE-2026-119375) lewat upgrade `rusqlite` `0.32 -> 0.41` (`libsqlite3-sys 0.39.1`, MSRV 1.89), menghapus dependensi `chrono` yang sudah tidak dirawat (`RUSTSEC-2026-0543`) dengan pemformat UTC RFC 3339 tanpa dependensi tambahan, dan mengaktifkan `allow-wildcard-paths = true` di `deny.toml`.
- **Universal Agent Profiles (OAuth Default & Optional API Key), GitHub CLI, MCP Servers, dan Plugins:**
  - `latch-core`: model & parser `mcp.toml` (`McpServerConfig`, `McpTransport`), `plugins.toml` (`PluginConfig`, `PluginKind`), parser status `gh auth status` tanpa token (`GitHubStatus`, `GitHubRepoItem`), `AgentAuthMode` (`oauth`/`api_key`/`none`), `ApiKeySource` (`env`/`runtime_memory`/`none`), serta profil agen bawaan komprehensif (`antigravity`, `claude`, `codex`, `gemini`, `opencode`, `aider`, `goose`, `amp`, `cline`, `cursor`, `copilot`, `kimi`, `qwen`, `shell`).
  - `latch-daemon`, `latch-pty-pool`, `latch-sandbox`: endpoint REST `/api/v1/agents*` (CRUD profil, runtime API key khusus RAM daemon tanpa persisten ke disk/SQLite, login OAuth interaktif PTY), `/api/v1/integrations/github*` (status `gh`, daftar repo, login OAuth, clone & tambah proyek), `/api/v1/mcp*` (CRUD + manifest runtime `mcp-enabled.json`), `/api/v1/plugins*`, serta tes integrasi keamanan `crates/daemon/tests/integrations_api.rs`.
  - `web/`: halaman **Agen (`BotsPage`, `AgentCard`, `AddAgentDialog`)** dengan filter/pencarian, pengalihan mode OAuth ↔ API Key, input kunci RAM sementara, dan tombol login OAuth; halaman **Integrasi (`PluginsPage`)** bertab GitHub, MCP Servers (preset 1-klik + custom), dan Plugins; lengkap dengan i18n `id`/`en`, Command Palette, dan tes unit Vitest.

## [1.2.0] - 2026-11-27

### Added
- **Arsitektur UI Modular `UI-SPEC-latch-ide-tauri-react.md` (`web/` & `desktop/`):**
  - **CSS-First Design System Tailwind v4 (`web/src/styles/` & `web/src/theme/`)**: 4 gaya tata letak (`tonal`, `ledger`, `bento`, `glass` dengan fallback WebKitGTK) × 2 mode (`dark`, `light`, plus `system`), aksen dinamis (`--accent`, `--accent-soft`, `--on-accent`), zero inline `<script>` (boot via `public/theme-boot.js`), dan 100% token semantik di seluruh komponen `.tsx`.
  - **Domain Zustand Stores & Transport (`web/src/state/` & `web/src/transport/`)**: store terpisah (`hosts`, `sessions`, `messages`, `layout`, `ui`, `usage`, `notifications`, `selectors`), auto-reconnect WebSocket dengan backoff eksponensial + resync penuh, dan pembungkus native Tauri (`transport/native.ts`).
  - **Terminal `@xterm/xterm` & Shell Desktop Lengkap (`web/src/layout/` & `web/src/features/`)**: `TitleBar` dengan `data-tauri-drag-region` & window controls Tauri, `Sidebar` toggle Status / Pohon Proyek, `SplitLayout` multi-pane (`TerminalView` lazy via `pty-stream.ts` + `@xterm/addon-fit`, `ChatView` + `Composer`, `DiffView` lazy), `SidePanel` (`Git` conflict radar, `Approvals`, `Traffic` proxy lazy), `CommandPalette` (`Ctrl+K`/`Cmd+K`), halaman `Automations` (pipeline DAG & `CompareView` arena), `Bots`, `Hosts`, `Usage` (`latch gc`), dan `Settings`.
  - **Floating Control & Upgrade Tauri v2 (`desktop/` & `web/floating.html`)**: multi-page Vite build (`index.html` + `floating.html`), perintah native `latch_window_action` & `latch_toggle_floating`, serta window `main` (`1440x900`, frameless) + `floating` (`360x400`, always-on-top).

## [1.1.0] - 2026-11-27

### Added
- **Redesign UI Desktop (`web/`) & Shell Native Tauri v2 (`desktop/`):**
  - Tata letak desktop 3-panel modern: top bar (pemilih proyek, tab `Sessions`/`Pipelines & Arena`/`Approvals`, pencarian cepat `Ctrl+K`, tombol `+ New session`), sidebar kiri dikelompokkan per status (*Waiting on you*, *Working*, *Failed*, *Done*, *Idle*) dengan toggle *Sessions only / By project*, stage tengah **Live Interactive PTY Terminal** (`/api/v1/sessions/{id}/pty` + input bar `>`) & tab **Diff** inline, serta floating action rail di kanan. Total bundel gzip tetap **86,2 KiB** (batas 600 KiB).
  - Paket terpisah `desktop/` berbasis **Tauri v2** (WebView OS, tanpa Electron/Chromium) yang otomatis menyalakan `latch-daemon` dan menyuntikkan token lokal (`window.__LATCH_TOKEN__`) langsung ke WebView untuk pengalaman desktop app zero-setup.

## [1.0.1] - 2026-11-27

### Added
- **Zero-Setup UX**:
  - `latch` tanpa argumen otomatis menyalakan daemon, mendaftarkan repositori Git di direktori saat ini, dan langsung membuka TUI.
  - Subcommand `latch web` (alias `latch open`) otomatis menyalakan daemon, menampilkan URL & bearer token, dan membuka Web Workbench di browser.
  - Seluruh perintah klien (`latch run`, `session`, `pipeline`, `project`, `gc`, `tui`) otomatis menyalakan daemon di latar belakang bila belum berjalan.
  - Profil agen bawaan (`claude`, `codex`, `gemini`, `opencode`, `shell`, `sleeper`) aktif langsung tanpa perlu membuat `agents.toml` manual.
  - `scripts/install.sh` otomatis menambahkan `~/.local/bin` ke `~/.bashrc`/`~/.zshrc` (idempoten) dan menyalakan daemon saat instalasi selesai.

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
