# Breakdown Task

Task kecil yang dapat langsung diberikan ke agen/engineer. Satu task = satu PR (lihat [AGENTS.md](../AGENTS.md)). Format: **ID · Judul · Requirement · Crate · Acceptance Criteria (AC)**. Urutan dalam fase menunjukkan dependensi kasar; `Dep:` menyebut prasyarat.

Status: `[ ]` belum · `[~]` berjalan · `[x]` selesai.

---

## Fase 0 - Spike & Bootstrap (sebelum v0.1)

- [ ] **T-000 · Spike anggaran RAM** · R-08, NFR-01 · `spikes/`
  - Prototipe minimal: tokio(2 worker) + axum + rusqlite(bundled) + portable-pty.
  - AC: laporan RSS idle dan per sesi PTY di `docs/perf/spike-ram.md`; rekomendasi pengaturan.
- [ ] **T-001 · Spike proxy SSE** · R-10/R-11/R-12 · `spikes/`
  - AC: forwarding SSE OpenAI & Anthropic dengan parse `usage`; benchmark selisih latensi p99.
- [ ] **T-002 · Spike Landlock + toolchain** · R-03/R-05 · `spikes/`
  - AC: `cargo build`, `npm test`, `pytest`, `git commit` berjalan di worktree dalam sandbox; daftar jalur baca minimal terdokumentasi.
- [x] **T-003 · Bootstrap workspace** · FR-01 · root
  - Buat Cargo workspace (14 crate + `cli/`), `rust-version`, `[workspace.dependencies]`, `.cargo/config.toml`, `rustfmt.toml`, `clippy.toml`, `deny.toml`.
  - AC: `cargo build --workspace` dan `cargo nextest run` hijau; CI GitHub Actions menjalankan fmt/clippy/test.
- [x] **T-004 · Skrip CI ukuran & statis** · NFR-04 · CI
  - AC: job musl membangun binary, memeriksa `ldd` statis dan ukuran < 30 MB (`scripts/check-binary.sh`).

---

## Fase 1 - Fondasi (v0.1)

### Core & Store
- [x] **T-101 · Tipe domain di `core`** · FR-02 · `core` · Dep: T-003
  - `Project`, `Session`, `SessionStatus`, `ApprovalRequest`, `PipelineRun`, `TelemetryRecord`, ID UUID v7.
  - AC: serde snake_case; `can_transition(from,to)` + tes properti untuk semua transisi di DATA_MODEL §5.
- [x] **T-102 · Parser config & profil agen** · FR-07 · `core` · Dep: T-101
  - `config.toml`, `agents.toml` (command, args, env, `pass_env`, `proxy_mode`, `resume_args`, `extra_rw/ro`).
  - AC: galat validasi menyertakan field/baris; tes dengan fixture valid/invalid; `latch config check` memakai ini.
- [x] **T-103 · Lapisan SQLite & migrasi 0001** · FR-02 · `store` · Dep: T-101
  - Aktor penulis tunggal, WAL, pragma, migrasi dari DATA_MODEL §4, repositori projects/sessions.
  - AC: tes migrasi dari DB kosong; CRUD sesi; `integrity_check` saat startup; tidak ada kolom rahasia.
- [x] **T-104 · Rekonsiliasi startup** · FR-02, NFR-06 · `store`,`daemon` · Dep: T-103, T-111
  - AC: setelah `kill -9` daemon, sesi dengan PID mati ditandai `failed`; worktree yatim terdeteksi.

### Git Engine
- [x] **T-110 · Deteksi repo & validasi `git` ≥ 2.38** · FR-05 · `git-engine`
  - AC: error jelas bila versi kurang/bukan repo.
- [x] **T-111 · Buat/hapus worktree + branch** · FR-05 · `git-engine` · Dep: T-110
  - Branch `latch/<project>/<short-id>`; path `<state>/worktrees/<project>/<session>`.
  - AC: tiga sesi paralel mendapat worktree unik; cleanup idempoten; tes pada repo sementara.
- [x] **T-112 · Status & diff per worktree** · FR-08 · `git-engine` · Dep: T-111
  - AC: daftar file berubah (A/M/D/R) + patch unified; diff dipaginasi dan dibatasi ukuran.
- [x] **T-113 · Merge-check (`merge-tree`)** · FR-08, R-02 · `git-engine` · Dep: T-112
  - AC: konflik terdeteksi dan mengembalikan daftar file; tidak mengubah working tree.
- [x] **T-114 · Squash/merge & discard** · FR-08 · `git-engine` · Dep: T-113
  - AC: merge bersih berhasil; konflik memblokir; discard menghapus worktree+branch.

### PTY Pool
- [x] **T-120 · Spawn PTY & siklus proses** · FR-06 · `pty-pool`
  - AC: jalankan perintah di PTY, deteksi exit code, `kill` dengan grace period; crash tidak mempengaruhi sesi lain (NFR-06).
- [x] **T-121 · Ring buffer + log rolling** · FR-06, R-04 · `pty-pool` · Dep: T-120
  - AC: kapasitas default 1 MiB; sisa ke file log rolling; tes memori tidak tumbuh pada 10k baris/detik.
- [x] **T-122 · Batching frame 8-16 ms** · NFR-03 · `pty-pool` · Dep: T-120
  - AC: interval frame dalam rentang; klien lambat tidak memblokir (drop + marker `dropped`).
- [x] **T-123 · Layar `vt100` & snapshot** · FR-06 · `pty-pool` · Dep: T-120
  - AC: snapshot layar dapat diminta; resize menyinkronkan PTY dan parser.
- [x] **T-124 · Pause/Resume (SIGSTOP/SIGCONT)** · FR-06 · `pty-pool`
  - AC: kontrol process group; status `paused` ↔ `running`.

### Daemon & CLI
- [x] **T-130 · Server UDS + JSON-RPC** · FR-01 · `daemon` · Dep: T-103
  - AC: izin socket `0600`, `SO_PEERCRED`; metode `project.*`, `session.*` dasar.
- [x] **T-131 · Session Manager** · FR-05..FR-08 · `daemon` · Dep: T-111, T-120, T-130
  - Orkestrasi: buat sesi → worktree → spawn → pantau → status.
  - AC: 3 sesi paralel stabil; status tersimpan; event status dipublikasikan.
- [x] **T-132 · CLI `clap` & mode daemon** · FR-01 · `cli` · Dep: T-130
  - Subcommand `daemon`, `run`, `session`, `project`, `config`; alias argv0 `latchd`.
  - AC: satu binary menjalankan daemon dan klien; `--foreground` untuk systemd.
- [x] **T-133 · `latch run` end-to-end** · FR-05..FR-07 · `cli`,`daemon` · Dep: T-131, T-132
  - AC: `latch run --profile X --task Y` membuat worktree dan menjalankan profil.
- [x] **T-134 · Logging & konfigurasi tracing** · NFR-05 · `daemon`
  - AC: rolling file; lapisan redaksi pola kunci; tes bahwa string kanari disamarkan.

### TUI
- [x] **T-140 · Kerangka TUI & daftar sesi** · FR-21 · `tui` · Dep: T-130
  - AC: daftar sesi dengan status, biaya (placeholder), navigasi keyboard.
- [x] **T-141 · Panel output PTY (`tui-term`)** · FR-21 · `tui` · Dep: T-123, T-140
  - AC: warna ANSI benar; resize sinkron; scrollback.
- [x] **T-142 · Aksi attach/pause/kill + panel diff** · FR-21, FR-08 · `tui` · Dep: T-141, T-114
  - AC: pintasan attach/pause/kill; tampilan diff; aksi merge/discard dengan konfirmasi.

### Gerbang v0.1
- [x] **T-150 · Tes integrasi 3 sesi paralel** · DoD v0.1
  - AC: 3 sesi pada satu repo tanpa konflik; diff terpisah; restart daemon memulihkan state.
- [x] **T-151 · Benchmark RSS di CI** · NFR-01
  - AC: skrip `scripts/bench-rss.sh` gagal bila idle > 25 MB atau per sesi > 5 MB.
- [ ] **T-152 · Rilis v0.1** · CHANGELOG, tag, artefak musl.

---

## Fase 2 - Observabilitas & Web (v0.2)

### Proxy
- [ ] **T-201 · Reverse proxy HTTP dasar** · FR-09 · `proxy`
  - Rute `/s/<session-token>/<provider>/...`, forwarding ke upstream via `reqwest`+`rustls`.
  - AC: header `Authorization` diteruskan dan **tidak** dicatat; token sesi kedaluwarsa saat sesi selesai.
- [ ] **T-202 · Penerusan SSE & tee parser** · FR-09, FR-10 · `proxy` · Dep: T-201
  - AC: streaming tidak menunggu parse; tes dengan fixture OpenAI dan Anthropic.
- [ ] **T-203 · Ekstraksi usage & kalkulator biaya** · FR-10 · `proxy`,`core`
  - `pricing.toml`; `usage_source` reported/estimated.
  - AC: akurasi ≤ 1% pada fixture; model tak dikenal ditandai.
- [ ] **T-204 · Penulisan telemetri ber-batch** · FR-10 · `store`,`proxy`
  - AC: tidak memblokir jalur proxy; agregat per sesi/pipeline tersedia.
- [ ] **T-205 · Injeksi env proxy ke sesi** · FR-09 · `daemon`,`pty-pool`
  - AC: `OPENAI_BASE_URL`, `ANTHROPIC_BASE_URL`, dll. terpasang sesuai profil.
- [ ] **T-206 · Mode bypass per profil** · FR-13 · `core`,`daemon`
  - AC: `proxy_mode = "bypass"` tidak menyuntik env; PTY tetap berjalan; heuristik idle/loop dari output PTY.
- [ ] **T-207 · Detektor loop & error** · FR-11 · `proxy`
  - AC: error/tool-call identik ≥ `max_consecutive_errors` terdeteksi; fingerprint tanpa menyimpan isi.
- [ ] **T-208 · Circuit breaker** · FR-12 · `proxy`,`daemon`
  - AC: status `tripped` dan proses berhenti < 1 s sejak ambang; event + audit; uji dengan upstream mock yang mengulang error.
- [ ] **T-209 · Benchmark latensi proxy** · NFR-02
  - AC: p99 selisih ≤ 2 ms di CI.
- [ ] **T-210 · Tes kanari rahasia** · NFR-05
  - AC: pindai SQLite + log; gagal bila kanari ditemukan.

### Web & Remote
- [ ] **T-220 · Server axum REST + WS + auth token** · FR-22 · `daemon`
  - AC: token acak 256 bit `0600`; cek Origin; rate-limit gagal auth; endpoint API §4 untuk sesi/proyek.
- [ ] **T-221 · Aliran PTY via WebSocket** · FR-22 · `daemon` · Dep: T-122
  - AC: frame pertama snapshot; batching; resync.
- [ ] **T-222 · Kerangka PWA (Vite+React+TS+Tailwind)** · FR-22 · `web`
  - AC: daftar sesi, status, biaya; build menghasilkan `web/dist`; anggaran bundle ≤ 600 KB gzip.
- [ ] **T-223 · Diff viewer side-by-side + tampilan mobile** · FR-22 · `web`
  - AC: `diff2html`/setara; nyaman di layar ≤ 400 px.
- [ ] **T-224 · Embed aset (`rust-embed`) + CSP** · FR-22, NFR-04 · `daemon`
  - AC: aset terkompresi dilayani dengan `Content-Encoding`; CSP ketat.
- [ ] **T-225 · PWA manifest, service worker, notifikasi** · FR-22, FR-23 · `web`
  - AC: dapat dipasang di HP; menerima web push (HTTPS via Tailscale).
- [ ] **T-226 · Notifikasi desktop & webhook** · FR-23 · `daemon`
  - AC: event selesai/butuh izin/tripped; webhook HMAC.
- [ ] **T-227 · Mode remote (dokumentasi + klien)** · FR-03 · `cli`,`daemon`
  - `latch --remote ssh://host` / forward socket; panduan Tailscale.
  - AC: tanpa listener publik; uji manual dengan SSH forward.

### Gerbang v0.2
- [ ] **T-250 · Skenario infinite loop & budget** · DoD v0.2
- [ ] **T-251 · Uji PWA dari HP via tunnel; Approve latensi < 500 ms** (siapkan dulu untuk Fase 3)
- [ ] **T-252 · Rilis v0.2**

---

## Fase 3 - Keamanan (v0.3)

- [ ] **T-301 · ADR: opsi akses `.git` & approval gate** · R-05/R-06 · `docs/adr`
  - AC: ADR 0008 (akses `.git`) dan 0009 (mekanisme approval) disetujui.
- [ ] **T-302 · `trait SandboxProvider` & `NoopProvider`** · FR-16 · `core`,`sandbox`
  - AC: daemon hanya bergantung pada trait; Noop memunculkan peringatan dan `audit_log`.
- [ ] **T-303 · `LandlockProvider` (filesystem)** · FR-14 · `sandbox`
  - Deteksi ABI, `NO_NEW_PRIVS`, aturan RW worktree + tmp terisolasi, preset RO.
  - AC: tes: `cat ~/.ssh/id_rsa` → EACCES; tulis ke induk worktree ditolak; `require_sandbox` menolak start bila tak tersedia.
- [ ] **T-304 · Denylist & preset toolchain** · FR-15, R-03 · `sandbox`,`core`
  - AC: preset cargo/node/python/git; config global dapat disesuaikan; mode `dry-run` melaporkan akses.
- [ ] **T-305 · Env bersih & `HOME` per sesi** · SECURITY §5 · `daemon`
  - AC: variabel sensitif tidak diwariskan kecuali `pass_env`.
- [ ] **T-306 · Akses `.git` terbatas untuk worktree** · R-05 · `sandbox`,`git-engine`
  - AC: commit dari worktree berhasil; hook dinonaktifkan; cek integritas ref sebelum merge.
- [ ] **T-307 · Egress proxy domain-allowlist** · SECURITY §6 · `proxy`
  - AC: domain di luar allowlist ditolak/diminta approval; dokumentasi sifat kooperatif.
- [ ] **T-308 · Approval request/decision (store+API)** · FR-17 · `store`,`daemon`
  - AC: `approval_requests` pending/approved/denied; timeout default deny; audit.
- [ ] **T-309 · Mekanisme pencegat (PATH shim / hook agen)** · FR-17 · `daemon`,`sandbox`
  - AC: `git push` lewat shim memicu approval; keputusan melanjutkan/menolak.
- [ ] **T-310 · Approval di TUI & Web PWA** · FR-17 · `tui`,`web`
  - AC: daftar pending, Approve/Deny; < 500 ms dari klik ke sesi lanjut.
- [ ] **T-311 · Suite tes keamanan otomatis** · SECURITY §13
  - AC: semua tabel tes lolos di runner Linux ≥ 5.13.
- [ ] **T-312 · Rilis v0.3**

---

## Fase 4 - Otomasi Penuh (v1.0)

- [ ] **T-401 · Parser `workflow.toml` & validasi** · FR-18 · `core`,`dag`
  - AC: aturan WORKFLOW_SPEC §5; galat jelas; siklus ditolak.
- [ ] **T-402 · Graf & eksekutor dasar (agent+shell)** · FR-18 · `dag` · Dep: T-401
  - AC: toposort, paralelisme ≤ `max_parallel`, state di `dag_state_json`.
- [ ] **T-403 · Templating minijinja strict + pemotongan output** · FR-19 · `dag`
  - AC: variabel tak dikenal = galat; keluaran dipotong dengan penanda.
- [ ] **T-404 · Retry gate** · FR-19 · `dag`
  - AC: `shell` gagal → stdout/stderr diumpankan ke node target hingga `max_retries`; saat habis pipeline `failed`.
- [ ] **T-405 · Fan-out arena** · FR-20 · `dag`,`git-engine`
  - AC: N worktree paralel; verifikasi per kandidat; matriks metrik (lolos, durasi, diff, biaya).
- [ ] **T-406 · Aturan pemilihan pemenang** · FR-20 · `dag`
  - AC: `manual`, `cheapest_passing`, `fastest_passing`, `smallest_diff_passing`.
- [ ] **T-407 · Anggaran pipeline & trip** · FR-12/FR-18 · `dag`,`proxy`
  - AC: melampaui `max_budget_usd` menghentikan semua sesi turunan; status `tripped`.
- [ ] **T-408 · Pemulihan pipeline setelah restart** · FR-18 · `dag`,`daemon`
- [ ] **T-409 · UI pipeline & arena (TUI+Web)** · FR-20 · `tui`,`web`
- [ ] **T-410 · Hibernasi idle** · FR-04 · `pty-pool`,`store`,`daemon`
  - AC: idle > ambang → `hibernated` + snapshot; resume via `resume_args`/prompt konteks; tes pulih.
- [ ] **T-411 · `latch gc` & kebijakan retensi** · R-16 · `daemon`
- [ ] **T-412 · Dokumentasi pengguna final & rilis v1.0** · USER_GUIDE, CHANGELOG, artefak x86_64/aarch64 musl, checksum.

---

## Task Lintas-Fase

- [ ] **T-900 · Dependabot/`cargo deny`/`cargo audit` di CI** · SECURITY §11
- [ ] **T-901 · Tes kompatibilitas OS (matrix macOS/Windows build)** · NFR-07
- [ ] **T-902 · Matriks kompatibilitas CLI agen** · R-01 (Claude Code, Codex CLI, OpenCode, Gemini CLI)
- [ ] **T-903 · Pembaruan `pricing.toml` berkala** · R-10
- [ ] **T-904 · Pustaka profil agen contoh** · FR-07

## Templat Task

```markdown
### T-xxx · Judul
- Requirement: FR-xx / NFR-xx
- Crate: ...
- Dependensi: T-...
- Deskripsi:
- Acceptance Criteria:
  - [ ] ...
- Uji:
- Dampak RAM/ukuran:
- Dokumen yang diperbarui:
```
