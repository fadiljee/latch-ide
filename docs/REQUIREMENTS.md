# Requirements

Prioritas: **P0** = wajib MVP · **P1** = pembeda inti · **P2** = otomasi penuh.
Status kolom: `Todo` / `WIP` / `Done` (perbarui saat pengerjaan). Fase merujuk ke [ROADMAP.md](ROADMAP.md).

## Persyaratan Fungsional

### A. Daemon Core, State, & Hibernation

| ID | Requirement | Deskripsi | Prioritas | Fase | Status |
| --- | --- | --- | --- | --- | --- |
| FR-01 | Single Static Binary | Satu executable Rust melayani mode daemon (`latchd`) dan klien CLI/TUI (`latch`) | P0 | v0.1 | Todo |
| FR-02 | Persistent State (SQLite) | Menyimpan proyek, profil, sesi, metrik, status pipeline; pulih setelah restart | P0 | v0.1 | Todo |
| FR-03 | Remote Daemon Mode | Klien terhubung ke daemon jauh via SSH forwarding atau Tailscale/WireGuard tanpa membuka port publik | P1 | v0.2 | Todo |
| FR-04 | Idle Agent Hibernation | Membekukan/menghentikan PTY idle melewati ambang; konteks dan status worktree disimpan di SQLite untuk dilanjutkan | P2 | v1.0 | Todo |

**Kriteria terima ringkas**
- FR-01: `latch daemon start` dan `latch tui` dari binary yang sama; `latchd` (symlink/argv0) setara `latch daemon`.
- FR-02: setelah `kill -9` daemon lalu start ulang, sesi `running` direkonsiliasi (lihat NFR-06) dan riwayat tetap ada.
- FR-03: koneksi hanya lewat UDS yang di-forward atau `127.0.0.1` yang di-tunnel; tidak ada listener publik bawaan.
- FR-04: sesi idle > `idle_hibernate_minutes` berstatus `hibernated`; `resume` memulihkan worktree dan konteks (batasan: lihat ARCHITECTURE §Hibernasi).

### B. Git Worktree & PTY Session Manager

| ID | Requirement | Deskripsi | Prioritas | Fase | Status |
| --- | --- | --- | --- | --- | --- |
| FR-05 | Manajemen Worktree Otomatis | Buat direktori `git worktree` dan branch terisolasi per tugas; bersihkan setelah merge/batal | P0 | v0.1 | Todo |
| FR-06 | Multi-Session PTY Pool | Banyak CLI agen paralel di PTY asli (`portable-pty`): warna ANSI, input interaktif, sinkron resize | P0 | v0.1 | Todo |
| FR-07 | Agent Profile Registry | Profil CLI agen (perintah, argumen, env, dukungan proxy) di TOML | P0 | v0.1 | Todo |
| FR-08 | Diff & Merge Workflow | Daftar file berubah + diff per worktree; aksi satu tombol squash/merge atau buang | P0 | v0.1 | Todo |

**Kriteria terima ringkas**
- FR-05: nama branch unik `latch/<project>/<short-id>`; worktree di `<state_dir>/worktrees/<project>/<session-id>`; cleanup idempoten.
- FR-06: resize terminal klien → `SIGWINCH`/`pty.resize` ke sesi; ANSI utuh ke TUI.
- FR-07: validasi profil saat load; error menunjuk baris/field.
- FR-08: sebelum merge dilakukan simulasi `git merge-tree`; konflik memblokir auto-merge.

### C. Local LLM Proxy & Circuit Breaker

| ID | Requirement | Deskripsi | Prioritas | Fase | Status |
| --- | --- | --- | --- | --- | --- |
| FR-09 | Auto Proxy Injection | HTTP/SSE reverse proxy lokal di daemon; otomatis menyuntik `OPENAI_BASE_URL`, `ANTHROPIC_BASE_URL`, dsb. ke sesi PTY | P1 | v0.2 | Todo |
| FR-10 | Telemetry Token & Biaya | Membedah respons streaming SSE dan JSON untuk token input/output, latensi, estimasi biaya real-time per sesi dan pipeline | P1 | v0.2 | Todo |
| FR-11 | Loop & Error Detector | Mendeteksi error/tool-call identik berturut-turut dan pemanggilan file berulang tak produktif | P1 | v0.2 | Todo |
| FR-12 | Automated Circuit Breaker | Pause/stop agen saat budget, batas token, atau ambang retry terlampaui; kirim peringatan | P1 | v0.2 | Todo |
| FR-13 | Proxy Bypass Mode | Nonaktifkan proxy per profil untuk CLI OAuth tertutup / certificate pinning; PTY dan sandbox tetap aktif | P1 | v0.2 | Todo |

**Kriteria terima ringkas**
- FR-09: proxy mengidentifikasi sesi lewat path prefix atau header kustom per sesi (`/s/<session-token>/...`), bukan lewat kredensial.
- FR-10: akurasi token dibanding `usage` dari penyedia ≤ 1% selisih bila `usage` tersedia; bila tidak, ditandai `estimated`.
- FR-11: pola berulang dikonfigurasi (`max_consecutive_errors`).
- FR-12: waktu dari ambang terlampaui sampai proses berhenti < 1 s; status `tripped`.
- FR-13: pada mode bypass, deteksi loop memakai heuristik output PTY (lihat RISKS R-01).

### D. OS Sandbox & Permission Firewall

| ID | Requirement | Deskripsi | Prioritas | Fase | Status |
| --- | --- | --- | --- | --- | --- |
| FR-14 | Filesystem Jail (Linux-first) | Landlock sebelum proses agen berjalan: tulis hanya di worktree miliknya dan `/tmp` terisolasi | P1 | v0.3 | Todo |
| FR-15 | Sensitive Path Denylist | Blokir baca ke path sensitif di luar worktree (`~/.ssh`, `~/.aws`, `~/.gnupg`, keyring, `.env` proyek lain) | P1 | v0.3 | Todo |
| FR-16 | Sandbox Abstraction Trait | `trait SandboxProvider` agar macOS (Seatbelt)/Windows (AppContainer) bisa ditambah tanpa mengubah inti | P1 | v0.3 | Todo |
| FR-17 | Interactive Approval Gate | Jeda aksi berisiko tinggi (perintah destruktif, `git push`, instalasi paket global, domain asing) dan kirim Approve/Deny ke TUI dan Web PWA | P2 | v0.3 | Todo |

**Kriteria terima ringkas**
- FR-14/15: tes otomatis membuktikan `open("~/.ssh/id_rsa")` → `EACCES` dari dalam sandbox.
- FR-16: crate `sandbox` menyediakan `LandlockProvider` dan `NoopProvider`; daemon hanya bergantung pada trait.
- FR-17: permintaan memiliki `decision` `pending/approved/denied`, timeout bawaan, dan tercatat di `approval_requests`.

> Catatan: FR-17 diberi P2 di blueprint tetapi dijadwalkan di Fase 3 (v0.3) bersama sandbox karena DoD Fase 3 mensyaratkan alur Approve/Deny.

### E. Deterministic DAG Pipeline & Fan-Out

| ID | Requirement | Deskripsi | Prioritas | Fase | Status |
| --- | --- | --- | --- | --- | --- |
| FR-18 | Declarative `workflow.toml` | Parse DAG berisi node agen AI dan node deterministik (shell/test/linter) | P2 | v1.0 | Todo |
| FR-19 | Estafet & Auto-Retry Gate | Teruskan artefak antar tahap; bila node validasi gagal, stdout/stderr diumpan balik ke agen sebelumnya hingga `max_retries` | P2 | v1.0 | Todo |
| FR-20 | Parallel Fan-Out Arena | Satu prompt ke ≥ 2 profil/model di worktree terpisah; matriks perbandingan test, waktu, jumlah diff, biaya | P2 | v1.0 | Todo |

### F. Thin Clients

| ID | Requirement | Deskripsi | Prioritas | Fase | Status |
| --- | --- | --- | --- | --- | --- |
| FR-21 | Terminal UI (TUI) | `ratatui`: daftar sesi, panel output PTY, indikator biaya/status, pintasan attach/pause/kill/approve | P0 | v0.1 | Todo |
| FR-22 | Embedded Web UI / PWA | Web ringan tersemat (`rust-embed`): side-by-side diff di desktop, status + Approve/Reject dari HP | P1 | v0.2 | Todo |
| FR-23 | Event Notifications | Notifikasi desktop atau web push/webhook saat agen selesai, butuh izin, atau di-trip | P1 | v0.2 | Todo |

### G. Workbench (Agent IDE)

FR-22 (Web UI) diperluas menjadi workbench penuh. Lihat spesifikasi lengkap `FR-I01..` dan `NFR-I01..` di [ide/IDE_REQUIREMENTS.md](ide/IDE_REQUIREMENTS.md).

## Persyaratan Non-Fungsional

| Kategori | ID | Target terukur | Cara verifikasi |
| --- | --- | --- | --- |
| Efisiensi RAM | NFR-01 | Daemon idle ≤ **25 MB**; overhead per sesi PTY aktif ≤ **5 MB** (di luar proses CLI agen) | Benchmark RSS/PSS di CI ([NFR_BUDGET](NFR_BUDGET.md)) |
| Latensi Proxy | NFR-02 | Tambahan latensi forwarding ≤ **2 ms** per request | Benchmark proxy dengan upstream mock |
| Responsivitas UI | NFR-03 | Output PTY di-*batch* tiap **8-16 ms** | Tes pembangkit log 10k baris/detik |
| Ukuran & Distribusi | NFR-04 | Static binary musl < **30 MB** termasuk aset Web, tanpa dependensi runtime eksternal | Cek ukuran di CI + `ldd` |
| Keamanan Data | NFR-05 | API Web/WS dilindungi bearer token acak lokal; proxy **tidak** menyimpan API key/`Authorization` di SQLite maupun log | Tes otomatis memindai DB & log; review kode |
| Reliabilitas | NFR-06 | Crash/OOM satu sesi tidak menjatuhkan daemon atau sesi lain; worktree yatim terdeteksi dan dibersihkan saat startup | Tes kill sesi; tes orphan |
| Kompatibilitas OS | NFR-07 | Target v1: Linux kernel ≥ 5.13; fitur non-sandbox tetap build dan jalan di macOS/Windows | CI matrix build |

## Matriks Keterlacakan (Requirement → Crate → Fase)

| Requirement | Crate utama | Fase |
| --- | --- | --- |
| FR-01 | `cli`, `daemon` | v0.1 |
| FR-02 | `store` | v0.1 |
| FR-03 | `daemon`, `cli` | v0.2 |
| FR-04 | `pty-pool`, `store`, `daemon` | v1.0 |
| FR-05, FR-08 | `git-engine` | v0.1 |
| FR-06 | `pty-pool` | v0.1 |
| FR-07 | `core` | v0.1 |
| FR-09 s.d. FR-13 | `proxy`, `core` | v0.2 |
| FR-14 s.d. FR-16 | `sandbox`, `core` | v0.3 |
| FR-17 | `daemon`, `sandbox`, `proxy`, `tui`, `web` | v0.3 |
| FR-18 s.d. FR-20 | `dag`, `core` | v1.0 |
| FR-21 | `tui` | v0.1 |
| FR-22 | `web`, `daemon` | v0.2 |
| FR-23 | `daemon` | v0.2 |
