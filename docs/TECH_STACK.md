# Tech Stack

Seluruh sistem dibangun di atas **100% Rust untuk daemon, CLI, dan TUI**, dipadukan dengan **React + TypeScript + Vite** yang dibundel ke dalam binary Rust untuk Web PWA. Tujuannya: binary statis < 30 MB, RAM idle < 25 MB, tanpa dependensi runtime eksternal (Node.js, Electron, Docker).

## 1. Ringkasan per Lapisan

| Lapisan | Teknologi / Crate | Peran | Alasan |
| --- | --- | --- | --- |
| Bahasa & runtime | Rust + `tokio` | Inti daemon, konkurensi async, I/O | Tanpa GC, aman memori, hemat RAM untuk banyak proses |
| CLI & konfigurasi | `clap` + `serde` + `toml` | Subcommand `daemon`, `tui`, `run`; parsing config & workflow | Standar ekosistem; parsing type-safe |
| IPC & server web | `axum` + `tower-http` + `rust-embed` | UDS (lokal), REST, WebSocket, aset PWA | Ringan di atas tokio/hyper; aset web tertanam di executable |
| PTY & terminal core | `portable-pty` + `vt100` | Menjalankan CLI agen interaktif; layar virtual | ANSI, resize, snapshot layar untuk hibernasi/HP |
| Git & worktree | `git` CLI (`tokio::process`) + `gix` (atau `git2`) | Worktree, `merge-tree`, status/diff | CLI paling stabil untuk worktree; `gix` cepat untuk inspeksi diff |
| Local LLM proxy | `hyper` + `reqwest` (`rustls`) + `eventsource-stream` | Reverse proxy HTTP/SSE, token, biaya, circuit breaker | `rustls` murni Rust → mudah dikompilasi statis |
| OS sandbox & proses | `landlock` + `nix` | Penguncian filesystem Linux; sinyal & process group | Isolasi tanpa overhead kontainer |
| DAG engine | `petgraph` + `minijinja` | Toposort, validasi siklus, templating prompt | Mencegah dependensi sirkular; render `{{steps.x.output}}` |
| Database & logging | `rusqlite` (`bundled`) + `tracing` | Persistensi, metrik, rolling logs | `bundled` menyatukan SQLite ke binary |
| TUI | `ratatui` + `crossterm` + `tui-term` | UI terminal; render output `vt100` di panel | Render cepat lintas terminal |
| Web PWA | React + TypeScript + Vite + Tailwind CSS | Diff viewer, grafik biaya, Approve/Reject dari HP | Bundle ringkas, mobile-first, mudah dipasang sebagai PWA |

## 2. Frontend (Embedded Web PWA)

Seluruh hasil build ditanam lewat `rust-embed`, jadi setiap pustaka dipilih berdasarkan ukuran bundle.

| Kebutuhan | Pilihan | Catatan |
| --- | --- | --- |
| Terminal di browser | `@xterm/xterm` + `@xterm/addon-fit` | Attach ke sesi via WebSocket (desktop) |
| Diff viewer | `diff2html` atau `react-diff-viewer-continued` | Hindari Monaco (menambah > 10 MB) |
| State & streaming | WebSocket native + auto-reconnect + Zustand | Sesi, antrean approval, metrik biaya |
| PWA & notifikasi | `vite-plugin-pwa` | Home screen HP; akses via Tailscale/SSH tunnel |
| Styling | Tailwind CSS (purge saat build) | CSS akhir kecil |
| Kompresi aset | Brotli/gzip saat build, `rust-embed` + `compression` | Binary lebih kecil; layani `Content-Encoding` |

**Anggaran bundle web:** bundel awal ≤ 600 KB gzip, total seluruh chunk (termasuk CodeMirror 6 lazy, `xterm.js`, `diff`) ≤ 3 MB terkompresi (lihat [ide/IDE_TECH_STACK.md](ide/IDE_TECH_STACK.md) & [ide/IDE_NFR_BUDGET.md](ide/IDE_NFR_BUDGET.md)); tanpa font eksternal.

## 3. Struktur Cargo Workspace

```text
latch-ide/
├── Cargo.toml                  # workspace root + shared dependencies
├── crates/
│   ├── core/                   # model data, trait SandboxProvider, parser config & workflow.toml
│   ├── store/                  # SQLite (rusqlite) & migrasi
│   ├── git-engine/             # worktree, merge-tree, diff extractor
│   ├── pty-pool/               # portable-pty, ring buffer, vt100, hibernation
│   ├── sandbox/                # Landlock (Linux) + stub OS lain
│   ├── proxy/                  # reverse proxy LLM, SSE parser, circuit breaker
│   ├── dag/                    # petgraph executor, retry gate, fan-out arena
│   ├── daemon/                 # axum (UDS + WS) + rust-embed PWA
│   └── tui/                    # ratatui
├── cli/                        # entrypoint tunggal (src/main.rs)
└── web/                        # Vite + React + TS + Tailwind (output web/dist)
```

Manfaat: batas modul yang bersih, kontrak OS terisolasi di `core`/`sandbox`, kompilasi inkremental lebih cepat.

## 4. Keputusan Build

### 4.1 Static binary (musl)

- Target `x86_64-unknown-linux-musl` (dan `aarch64-unknown-linux-musl` bila perlu).
- `rustls` (tanpa OpenSSL) dan `rusqlite` dengan fitur `bundled`.
- Verifikasi: `ldd target/.../agent` harus menampilkan `not a dynamic executable`.
- Perhatian: alokator musl bisa lebih lambat; evaluasi `mimalloc`/`jemalloc` hanya bila benchmark membuktikan manfaat dan masih dalam anggaran ukuran/RAM.

### 4.2 Profil release

```toml
[profile.release]
lto = "thin"
strip = true
codegen-units = 1
panic = "abort"   # opsional; pertimbangkan karena memengaruhi isolasi crash (lihat catatan)
opt-level = "z"   # opsional; ukur dulu dibanding "s"/3
```

> Catatan `panic = "abort"`: panic di satu tugas akan mematikan seluruh daemon, bertentangan dengan NFR-06. Gunakan `panic = "unwind"` (bawaan) dan tangkap panic per tugas, atau pastikan seluruh jalur sesi bebas panic. Putuskan lewat benchmark ukuran vs. keandalan.

### 4.3 Pengembangan

`.cargo/config.toml`:

```toml
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]   # atau lld
```

- Tes: `cargo nextest run --workspace`.
- Audit dependensi: `cargo deny check`, `cargo audit`.
- Ukuran: `cargo bloat --release --crates`, `cargo tree -d`.

### 4.4 Rust & MSRV

Rust stable terbaru; tetapkan MSRV di `Cargo.toml` (`rust-version`) saat v0.1 dan uji di CI.

## 5. Daftar Fitur Cargo yang Disarankan

| Crate | Fitur/konfigurasi |
| --- | --- |
| `tokio` | `rt-multi-thread`, `macros`, `net`, `io-util`, `process`, `signal`, `sync`, `time`, `fs` (jangan `full`) |
| `reqwest` | `default-features = false`, `rustls-tls`, `stream`, `json`, `gzip`, `brotli` bila perlu |
| `rusqlite` | `bundled`; tambahkan `backup` hanya bila diperlukan |
| `axum` | `ws`, `macros` opsional; matikan fitur tak terpakai |
| `serde` | `derive` |
| `clap` | `derive`, `std`, `help`, `usage`; tanpa `color` bila ingin lebih kecil |
| `tracing-subscriber` | `fmt`, `env-filter`; `tracing-appender` untuk rolling file |
| `gix` | Batasi fitur (hanya `status`/`diff`/`revision` yang dipakai) |

## 6. Dependensi Sistem Saat Runtime

| Komponen | Wajib? | Catatan |
| --- | --- | --- |
| `git` ≥ 2.38 | Ya | Worktree, `merge-tree --write-tree` |
| Tailscale / SSH | Opsional | Untuk akses remote (FR-03) |
| Node.js | Tidak | Hanya untuk build `web/` |
| Docker | Tidak | Tidak dipakai |

## 7. Alternatif yang Dipertimbangkan

| Keputusan | Ditolak | Alasan |
| --- | --- | --- |
| UI desktop | Electron, Tauri | RAM/ukuran; bertentangan dengan tujuan produk (ADR-0006) |
| TLS | `native-tls`/OpenSSL | Menyulitkan build statis |
| Git | `git2` saja | Dukungan worktree/merge-tree kurang lengkap; dependensi C `libgit2` |
| Sandbox | Docker/bubblewrap/firejail | Dependensi eksternal / overhead; Landlock bawaan kernel (ADR-0002) |
| Relay jarak jauh | Server relay sendiri | Beban infrastruktur dan keamanan (ADR-0003) |
| Editor diff | Monaco | > 10 MB |
