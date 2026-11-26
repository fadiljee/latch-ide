<p align="center">
  <img src="assets/icon.png" alt="Latch Logo" width="112" />
</p>

# Latch (`latch-ide`)

> Runtime, orkestrator, dan fondasi Agent IDE berbasis **100% Rust** dalam satu binary tunggal (`latch` / `latchd`). Hemat RAM, terisolasi di `git worktree`, terukur, dan dibangun tanpa Electron.

![status](https://img.shields.io/badge/status-v0.1--alpha-blue)
![rust](https://img.shields.io/badge/rust-1.85%2B-orange)
![license](https://img.shields.io/badge/license-TBD-lightgrey)

## Kenapa Latch Dibangun

Menjalankan banyak CLI agen AI (`claude`, `codex`, `opencode`, `gemini`, atau skrip agen kustom) secara paralel sering memunculkan empat masalah:

1. **Boros RAM.** Orkestrator berbasis Electron/Chromium memakan ratusan MB hingga GB saat beberapa sesi aktif.
2. **Isolasi lemah.** Agen otonom tanpa worktree dan batas sandbox berisiko menimpa berkas antar-tugas atau menyentuh kredensial lokal.
3. **Kurang teramati.** Sulit memantau token, biaya, dan *infinite loop* secara *real-time*.
4. **Alur kerja manual.** Koordinasi antar-agen (perencana → pembuat kode → verifikasi → review diff → merge) masih serba *copy-paste*.

## Status Implementasi & Roadmap

| Area | Cakupan | Fase | Status |
| --- | --- | --- | --- |
| **Daemon & State** | Single binary (`latch` / `latchd`), UDS JSON-RPC 2.0 (`0600` + `SO_PEERCRED`), SQLite WAL aktor tunggal, rekonsiliasi startup & pembersihan worktree yatim | `v0.1` | Selesai |
| **Git Engine** | Validasi `git >= 2.38`, `git worktree` + branch `latch/<project>/<short-id>`, diff terpaginasi, simulasi konflik `git merge-tree`, squash/merge & discard idempoten | `v0.1` | Selesai |
| **PTY Pool** | `portable-pty` paralel, ring buffer 1 MiB + rolling log `0600`, batching 8–16 ms + backpressure, layar `vt100` + resize, `SIGSTOP`/`SIGCONT`/`kill` | `v0.1` | Selesai |
| **TUI & CLI** | CLI `clap` lengkap (`daemon`, `project`, `run`, `session`, `config`, `tui`) & TUI `ratatui` interaktif dengan panel ANSI, diff, dan konfirmasi merge/kill/discard | `v0.1` | Selesai |
| **Workbench Scaffold** | Kerangka `web/` (Vite + React + TS strict + Tailwind + Zustand + i18n `id`/`en` + ESLint `jsx-a11y` + Vitest + gerbang ukuran bundel) | `I0` | Selesai |
| **LLM Proxy & Web PWA** | Reverse proxy HTTP/SSE, telemetri biaya, circuit breaker, REST/WS + Web PWA tersemat (`rust-embed`) | `v0.2` / `I1` | Terjadwal |
| **OS Sandbox & Approvals** | Landlock filesystem jail (Linux), denylist sensitif, approval gate interaktif | `v0.3` / `I2` | Terjadwal |
| **DAG Pipeline & IDE Penuh** | `workflow.toml` DAG, retry gate, fan-out arena, hibernasi, editor CM6, pencarian, LSP tersandbox | `v1.0`–`v1.2` | Terjadwal |

## Hasil Benchmark Awal (`v0.1`)

Lihat rincian lengkap di [`docs/perf/v0.1-fondasi.md`](docs/perf/v0.1-fondasi.md):

| Metrik | Hasil Terukur | Anggaran |
| --- | --- | --- |
| Ukuran binary rilis (`latch`) | **4,21 MiB** | `< 30 MiB` |
| RSS daemon idle (`VmRSS`) | **10,40 MiB** (`10.652 kB`) | `≤ 25 MiB` |
| Overhead RAM per sesi PTY aktif | **0,25 MiB** (`253 kB` / sesi) | `≤ 5 MiB` |
| Bundel awal Web Workbench (gzip) | **72,70 KiB** (`74.447 B`) | `≤ 600 KiB` |

## Struktur Workspace

```text
latch-ide/
├── Cargo.toml                 # Cargo workspace root + lints ketat (#![forbid(unsafe_code)], no unwrap/expect)
├── cli/                       # Entrypoint binary tunggal `latch` (alias `latchd`)
├── crates/
│   ├── core/                  # Model domain UUID v7, config/profil validator, redaksi rahasia, protokol UDS
│   ├── store/                 # SQLite bundled (aktor penulis tunggal + migrasi 0001)
│   ├── git-engine/            # Manajemen worktree, diff, merge-tree, squash/merge & discard
│   ├── pty-pool/              # Kolam PTY, ring buffer, rolling log, batching 8–16 ms, vt100
│   ├── sandbox/               # Trait SandboxProvider & NoopProvider (Landlock pada v0.3)
│   ├── daemon/                # SessionManager, server UDS, rekonsiliasi startup, logging ter-redaksi
│   ├── tui/                   # Antarmuka terminal ratatui (klien tipis via UDS)
│   ├── proxy/                 # Stub Fase 2: reverse proxy LLM & circuit breaker
│   ├── dag/                   # Stub Fase 4: eksekutor DAG workflow.toml
│   ├── fs-service/            # Stub IDE: akses berkas aman di akar worktree
│   ├── search/                # Stub IDE: indeks fuzzy & pencarian teks
│   ├── lsp-host/              # Stub IDE: server LSP tersandbox
│   ├── agent-proto/           # Stub IDE: adaptor protokol agen terstruktur
│   └── workbench-api/         # Stub IDE: rute REST/WS & token ber-scope
├── web/                       # Kerangka Agent IDE Workbench (React + TypeScript + Vite + Tailwind)
├── scripts/                   # Gerbang build/ukuran (check-binary.sh) dan benchmark RAM (bench-rss.sh)
└── docs/                      # Spesifikasi lengkap: PRD, arsitektur, API, keamanan, ADR, dan Agent IDE
```

## Quick Start (`v0.1`)

### Prasyarat

- Rust stable `>= 1.85` (dan `musl-tools` + target `x86_64-unknown-linux-musl` bila membangun binary statis musl)
- `git >= 2.38`
- Node.js `>= 22` (opsional, hanya untuk membangun frontend `web/`)

### Build & Jalankan

```bash
# 1. Build binary rilis
cargo build --release -p latch-cli
# atau build statis musl (butuh `musl-tools` & `rustup target add x86_64-unknown-linux-musl`):
./scripts/check-binary.sh

# 2. Validasi konfigurasi (~/.config/latch/config.toml & agents.toml)
./target/release/latch config check

# 3. Mulai daemon latar belakang (atau tambahkan --foreground untuk systemd)
./target/release/latch daemon start
./target/release/latch daemon status

# 4. Daftarkan repositori Git
./target/release/latch project add /path/ke/repo-anda --name my-repo

# 5. Jalankan sesi agen pada worktree + branch terisolasi (profil bawaan: `shell`, `sleeper`, atau definisi di agents.toml)
./target/release/latch run --project my-repo --profile shell --task "Tambahkan endpoint health"

# 6. Pantau lewat CLI atau buka TUI interaktif
./target/release/latch session ls
./target/release/latch tui

# 7. Tinjau diff, cek konflik merge-tree, lalu squash-merge atau buang
./target/release/latch session diff <session-id>
./target/release/latch session merge-check <session-id>
./target/release/latch session merge <session-id> --squash
# atau:
./target/release/latch session discard <session-id>
```

> Variabel lingkungan opsional untuk isolasi lokasi konfigurasi/state: `LATCH_CONFIG_DIR`, `LATCH_STATE_DIR`, `LATCH_SOCKET`, dan `LATCH_LOG`.

## Pengembangan & Verifikasi

```bash
# Format, lint ketat, dan seluruh tes unit + integrasi end-to-end
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo nextest run --workspace

# Verifikasi ukuran binary (< 30 MiB) dan anggaran RAM daemon (idle <= 25 MiB, <= 5 MiB/sesi)
./scripts/check-binary.sh
./scripts/bench-rss.sh

# Verifikasi frontend Workbench (lint + a11y + typecheck + Vitest + batas bundel <= 600 KiB gzip)
(cd web && npm ci && npm run lint && npm run typecheck && npm run test && npm run build)
```

## Dokumentasi

| Dokumen | Isi |
| --- | --- |
| [`docs/PRD.md`](docs/PRD.md) | Product Requirements Document inti |
| [`docs/REQUIREMENTS.md`](docs/REQUIREMENTS.md) | Daftar FR-01..FR-23 dan NFR-01..NFR-07 |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | Arsitektur crate, siklus sesi, dan alur data |
| [`docs/DATA_MODEL.md`](docs/DATA_MODEL.md) | Skema SQLite, migrasi, dan mesin status sesi |
| [`docs/API.md`](docs/API.md) | Kontrak UDS JSON-RPC 2.0, REST, dan WebSocket |
| [`docs/SECURITY.md`](docs/SECURITY.md) | Model ancaman, kebijakan sandbox Landlock, dan redaksi rahasia |
| [`docs/NFR_BUDGET.md`](docs/NFR_BUDGET.md) | Anggaran RAM/ukuran/latensi & metodologi ukur |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) & [`docs/TASKS.md`](docs/TASKS.md) | Roadmap rilis dan breakdown task |
| [`docs/ide/README.md`](docs/ide/README.md) | Paket spesifikasi **Agent IDE (Workbench)** |
| [`docs/adr/`](docs/adr/README.md) | Architecture Decision Records (ADR-0001..0017) |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) & [`CHANGELOG.md`](CHANGELOG.md) | Panduan kontribusi dan catatan perubahan |

## Keamanan & Privasi

- **Default aman:** daemon UDS mengikat izin `0600` + verifikasi `SO_PEERCRED` UID; bind HTTP (v0.2) dibatasi ke `127.0.0.1`.
- **Tanpa kebocoran rahasia (`NFR-05`):** log daemon dan catatan basis data otomatis melalui lapisan redaksi rahasia (`sk-...`, `AKIA...`, `ghp_...`, `Bearer ...`, `Authorization`, `x-api-key`).
- **Isolasi proses (`NFR-06`):** crash atau `kill -9` pada proses agen tidak menjatuhkan daemon atau sesi lain; restart daemon otomatis merekonsiliasi status dan menyapu worktree yatim.

## Lisensi

Belum ditentukan (`TBD`). Akan diputuskan melalui ADR sebelum rilis publik pertama.
