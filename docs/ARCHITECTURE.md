# Arsitektur

Dokumen ini menjelaskan struktur sistem, tanggung jawab crate, alur data, dan keputusan desain utama. Pilihan teknologi per lapisan ada di [TECH_STACK.md](TECH_STACK.md); kontrak API di [API.md](API.md).

## 1. Gambaran Umum

Sistem terdiri dari **satu daemon** yang memiliki semua state dan logika, dan **klien tipis** (CLI, TUI, Web PWA) yang berkomunikasi dengannya.

```text
 +------------------+   +------------------+   +--------------------+
 |  CLI (agent ...) |   |   TUI (ratatui)  |   |  Web PWA (browser/ |
 |                  |   |                  |   |  HP via tunnel)    |
 +---------+--------+   +---------+--------+   +----------+---------+
           | UDS (JSON frames)    | UDS                   | HTTP+WS (127.0.0.1, bearer)
           +----------------------+-----------------------+
                                  |
                      +-----------v------------+
                      |         daemon         |
                      |  (axum, tokio, state)  |
                      +--+----+-----+-----+----+
        +-----------------+    |     |     +------------------+
        |                      |     |                        |
 +------v------+      +--------v--+  |  +---------+     +------v------+
 |    store    |      | git-engine|  |  |   dag   |     |   proxy     |
 |  (SQLite)   |      | worktree  |  |  | pipeline|     | LLM reverse |
 +-------------+      +-----------+  |  +----+----+     |   proxy     |
                                     |       |          +------+------+
                              +------v-------v--+              |
                              |    pty-pool     |              | HTTPS (rustls)
                              | portable-pty,   |              v
                              | vt100, ring buf |       API LLM (OpenAI,
                              +------+----------+       Anthropic, Google)
                                     | spawn (setelah sandbox diterapkan)
                              +------v----------+
                              |    sandbox      |
                              | Landlock / noop |
                              +------+----------+
                                     v
                         Proses CLI agen (di worktree)
                         env: *_BASE_URL -> proxy lokal
```

## 2. Single Binary, Banyak Mode

Satu executable `latch` dengan subcommand (`clap`):

| Perintah | Peran |
| --- | --- |
| `latch daemon start/stop/status` | Menjalankan daemon (alias `latchd` via argv0/symlink) |
| `latch tui` | TUI interaktif (klien) |
| `latch run`, `latch session ...`, `latch project ...`, `latch approvals ...`, `latch pipeline ...` | Klien CLI satu kali jalan |
| `latch token show/rotate` | Mengelola token Web |
| `latch config check` | Memvalidasi config, profil, workflow |

Daemon dapat dijalankan di *foreground* (untuk systemd) atau *daemonize*. Rekomendasi: unit `systemd --user` agar daemon hidup tanpa sesi login interaktif.

## 3. Tanggung Jawab Crate

| Crate | Tanggung jawab | Bergantung pada |
| --- | --- | --- |
| `core` | Model data (`Session`, `Project`, `ApprovalRequest`, ...), enum status, `trait SandboxProvider`, parser config dan `workflow.toml`, tipe event/protokol bersama | - |
| `store` | Akses SQLite (`rusqlite`), migrasi, repositori per entitas, thread/aktor penyimpanan | `core` |
| `git-engine` | Membuat/menghapus worktree & branch, `status`, `diff`, simulasi `merge-tree`, squash/merge, deteksi worktree yatim | `core` |
| `pty-pool` | Spawn proses di PTY, ring buffer, `vt100` screen state, resize, batching output, SIGSTOP/SIGCONT, hibernasi | `core` |
| `sandbox` | `LandlockProvider` (Linux), `NoopProvider` (lainnya); menerapkan aturan di proses anak sebelum `exec` | `core` |
| `proxy` | Reverse proxy HTTP/SSE, parser usage, kalkulator biaya, detektor loop, circuit breaker | `core` |
| `dag` | Validasi graf, eksekusi node (agent/fanout/shell), retry gate, arena perbandingan | `core` |
| `daemon` | Orkestrasi semua komponen, server UDS + REST + WebSocket, penyajian aset PWA, notifikasi, rekonsiliasi startup | semua crate di atas |
| `tui` | UI `ratatui`; klien protokol daemon | `core` (tipe protokol) |
| `cli` | Entrypoint `main.rs`, parsing `clap`, memilih mode | `daemon`, `tui`, `core` |
| `fs-service` | Operasi berkas aman di akar worktree (`openat2`/resolusi path, ETag, watcher) | `core` |
| `search` | Indeks nama berkas fuzzy (`nucleo`) + pencarian teks (`grep-*`) | `core` |
| `lsp-host` | Server bahasa tersandbox + jembatan JSON-RPC WS | `core`, `sandbox`, `pty-pool` |
| `agent-proto` | Adaptor agen terstruktur (`AGENT_PROTOCOL`) | `core` |
| `workbench-api` | Rute REST/WS Agent IDE, scope token, paginasi | `core`, `fs-service`, `search`, `agent-proto` |
| `web` | Frontend Vite/React; output `web/dist` ditanam oleh `daemon` | - |

## 4. Komponen Inti

### 4.1 Session Manager (di `daemon`)

Mesin status per sesi:

```text
            start
              |
              v
          +--------+  pause   +--------+
          | running|--------->| paused |
          +---+----+<---------+--------+
              |       resume
  need approval|                          trip
              v                    +-----------------+
   +------------------+            | (dari running/  |
   | waiting_approval |            |  paused/waiting)|
   +--------+---------+            +--------+--------+
     approve|  deny                         v
            v     v                     +---------+
        running  (kill/failed)          | tripped |
                                        +---------+
   idle > threshold          resume
 running --------> hibernated -------> running
   proses selesai OK -> completed ; proses gagal/crash -> failed
```

Transisi yang diizinkan didefinisikan sekali di `core` (fungsi `can_transition(from, to)`) dan dipakai `daemon` serta divalidasi tes properti.

### 4.2 Worktree Lifecycle (`git-engine`)

1. `create(project, task)` → `git worktree add -b latch/<project>/<short-id> <path> <base>`.
2. Sesi berjalan; perubahan terjadi di `<path>`.
3. `diff(session)` → daftar file + patch (via `gix` bila memungkinkan, fallback `git diff`).
4. `merge(session, strategy)`:
   - jalankan `git merge-tree --write-tree` untuk simulasi; bila konflik → blokir dan laporkan file konflik;
   - bila bersih → squash/merge ke branch tujuan.
5. `cleanup(session)` → `git worktree remove --force` + `git branch -D` setelah merge/discard; idempoten.
6. Startup: bandingkan `git worktree list` dengan tabel `sessions`; worktree tanpa sesi aktif ditandai yatim dan dibersihkan (NFR-06).

### 4.3 PTY Pool (`pty-pool`)

- Satu `PtySession` per sesi: `portable-pty` master/slave, thread pembaca blocking → kanal `tokio::mpsc` terbatas.
- **Ring buffer** per sesi (bawaan 1 MiB) menyimpan output mentah terbaru; sisanya digulirkan ke file log di disk (rolling).
- **`vt100::Parser`** menjaga layar virtual untuk *snapshot* (hibernasi, kirim ke HP, atau reattach TUI).
- **Batching 8-16 ms:** pembaca mengumpulkan byte, lalu *flush* sebagai satu frame ke pelanggan (TUI/WS) per tick.
- **Backpressure:** pelanggan lambat tidak memblokir PTY; frame lama dibuang/di-*coalesce* dan klien diberi tahu lewat penanda `dropped`.
- **Kontrol proses:** pause = `SIGSTOP` ke *process group*, resume = `SIGCONT`, kill = `SIGTERM` lalu `SIGKILL` setelah *grace period*.
- **Isolasi crash:** setiap sesi diawasi tugas terpisah; `waitpid` mendeteksi exit/OOM-kill, memperbarui status, dan tidak mempengaruhi sesi lain.

### 4.4 Sandbox (`sandbox`)

`trait SandboxProvider` (didefinisikan di `core`):

```rust
pub trait SandboxProvider: Send + Sync {
    /// Nama provider, mis. "landlock", "noop".
    fn name(&self) -> &'static str;
    /// Kemampuan yang tersedia di host saat ini (mis. ABI Landlock).
    fn capabilities(&self) -> SandboxCapabilities;
    /// Menyiapkan `Command` agar kebijakan diterapkan di proses anak
    /// sebelum exec (mis. lewat pre_exec).
    fn prepare(&self, policy: &SandboxPolicy, cmd: &mut std::process::Command)
        -> Result<(), SandboxError>;
}
```

Kebijakan (`SandboxPolicy`): `rw_paths` (worktree, tmp terisolasi), `ro_paths` (preset toolchain dan sistem), `deny_hints` (untuk audit/validasi), `allow_network` (domain; ditegakkan di proxy, lihat SECURITY §Jaringan).

### 4.5 Local LLM Proxy (`proxy`)

- Listener `127.0.0.1:<port>` (atau UDS) di dalam daemon.
- Rute per sesi: `/s/<session-token>/<provider>/...`; token sesi dibuat daemon dan hanya valid selama sesi hidup. Env yang disuntik, contoh: `ANTHROPIC_BASE_URL=http://127.0.0.1:<port>/s/<tok>/anthropic`.
- Meneruskan request ke upstream dengan `reqwest` + `rustls`; header `Authorization`/`x-api-key` dari klien diteruskan apa adanya dan **tidak** dicatat.
- Respons streaming di-*tee*: byte diteruskan ke klien tanpa menunggu, salinan diparse oleh `eventsource-stream` di tugas terpisah untuk menghitung token dan mengekstrak `usage`.
- Hasil telemetri dikirim ke `store` lewat kanal; kegagalan penulisan tidak boleh memblokir jalur forwarding.
- **Detektor loop:** jendela geser atas (hash error/tool-call) per sesi; ambang `max_consecutive_errors`.
- **Circuit breaker:** dievaluasi per event telemetri; saat terlampaui, memanggil `SessionManager::trip(session)`.

### 4.6 DAG Pipeline (`dag`)

- Parse `workflow.toml` → `petgraph::DiGraph`; validasi: id unik, `depends_on` valid, tanpa siklus (toposort).
- Node: `latch`, `fanout`, `shell`. Eksekusi berjenjang; node tanpa dependensi tak terpenuhi berjalan paralel (dibatasi `max_parallel`).
- Template `minijinja` untuk `{{task}}`, `{{steps.<id>.output}}`, dll.
- **Retry gate:** edge balik `on_fail_retry_step` bukan edge dependensi; bila `shell` gagal, stdout/stderr (dipotong) ditambahkan ke prompt retry node target hingga `max_retries`.
- **Fan-out arena:** membuat N worktree, menjalankan profil paralel, menjalankan perintah verifikasi pada masing-masing, mengumpulkan metrik (lolos/gagal, durasi, jumlah baris diff, biaya) → tabel perbandingan.
- State DAG diserialisasi ke `pipeline_runs.dag_state_json` setiap transisi agar pulih setelah restart.

### 4.7 Hibernasi (FR-04)

Alur: idle terdeteksi (tidak ada output PTY dan tidak ada lalu lintas proxy > ambang) → ambil snapshot layar `vt100` + status worktree (HEAD, `git status`) → simpan di SQLite → hentikan proses (atau `SIGSTOP` bila tidak perlu membebaskan RAM penuh).

**Batasan jujur:** proses CLI agen **tidak** bisa dipulihkan persis (memori internal hilang bila dihentikan). *Resume* berarti menjalankan ulang profil di worktree yang sama dengan prompt "lanjutkan" yang berisi ringkasan konteks, atau memakai fitur *resume/continue* bawaan CLI bila ada (diatur di profil: `resume_args`). Mode `SIGSTOP` mempertahankan proses tetapi tidak mengurangi RSS CLI.

### 4.8 Approval Gate (FR-17)

Mekanisme pencegatan aksi bergantung pada kemampuan agen; ada tiga jalur yang bisa digabung (lihat [SECURITY.md §7](SECURITY.md#7-approval-gate-fr-17)):

1. **PATH shim**: direktori shim di depan `PATH` untuk `git`, `npm`, `pip`, `curl`, dll. yang meminta keputusan ke daemon sebelum memanggil biner asli.
2. **Hook bawaan agen** (mis. hook izin di CLI yang mendukungnya) yang diarahkan ke `latch approvals request`.
3. **Inspeksi tool-call di proxy** untuk menandai aksi berisiko (hanya deteksi/peringatan, tidak dapat memblokir eksekusi di dalam CLI).

Permintaan menjadi baris `approval_requests` (`pending`), sesi pindah ke `waiting_approval`, event dikirim ke TUI/Web/notifikasi, dan keputusan memulihkan sesi.

## 5. Alur Data Utama

### 5.1 Menjalankan satu tugas

```text
CLI: latch run --profile P --task T
 -> daemon: validasi project/profil
 -> store: insert session(status=starting->running)
 -> git-engine: create worktree + branch
 -> proxy: terbitkan session-token, tentukan env *_BASE_URL (kecuali bypass)
 -> sandbox: SandboxPolicy(worktree, tmp, ro presets)
 -> pty-pool: spawn CLI agen (env + sandbox) -> mulai streaming
 -> klien attach: terima frame PTY (batched) dan event status
 -> proxy -> store: telemetri token/biaya; circuit breaker memantau
 -> proses selesai: status completed/failed; diff siap
 -> pengguna: merge / discard -> git-engine cleanup
```

### 5.2 Trip circuit breaker

```text
proxy menerima response usage -> hitung biaya kumulatif sesi
 -> melampaui max_budget_usd? atau error identik >= max_consecutive_errors?
 -> SessionManager.trip(id): SIGSTOP/SIGTERM group, status=tripped
 -> store: update + audit_log
 -> event 'session.tripped' ke klien + notifikasi
Target: < 1 detik dari event pemicu sampai proses berhenti
```

## 6. Konkurensi & Model Thread

- `tokio` multi-thread runtime dengan jumlah worker kecil (bawaan 2) demi RAM.
- Thread blocking khusus: pembaca PTY per sesi (`portable-pty` sinkron) dan satu thread store SQLite (aktor dengan kanal perintah).
- Seluruh kanal berbatas (*bounded*) untuk mencegah pertumbuhan memori tak terbatas.
- Satu proses = satu pemilik data; klien tidak pernah mengakses SQLite langsung.

## 7. Penyimpanan & Direktori

| Jalur (Linux, XDG) | Isi |
| --- | --- |
| `~/.config/latch/config.toml` | Konfigurasi global (port, direktori, preset sandbox, batas) |
| `~/.config/latch/agents.toml` | Profil agen (FR-07) |
| `~/.config/latch/pricing.toml` | Tabel harga model untuk estimasi biaya |
| `~/.local/state/latch/latch.db` | SQLite |
| `~/.local/state/latch/daemon.sock` (atau `$XDG_RUNTIME_DIR/latch.sock`) | Unix Domain Socket |
| `~/.local/state/latch/token` | Bearer token Web (mode `0600`) |
| `~/.local/state/latch/worktrees/<project>/<session>` | Worktree sesi |
| `~/.local/state/latch/logs/` | Log daemon dan log PTY (rolling) |

## 8. Observabilitas Internal

- `tracing` dengan *structured logs* dan rolling file; level lewat `LATCH_LOG`.
- Metrik internal diekspos lewat endpoint `GET /api/v1/metrics` (JSON sederhana: RSS, jumlah sesi, panjang kanal).
- Log audit sandbox/approval ditulis ke tabel `audit_log`.

## 9. Kompatibilitas Platform

| Fitur | Linux | macOS | Windows |
| --- | --- | --- | --- |
| Daemon, store, git-engine, proxy, dag, tui, web | ✔ | ✔ (build & jalan) | ✔ (build & jalan; PTY via ConPTY) |
| Sandbox Landlock | ✔ (≥ 5.13) | ✘ (`NoopProvider`) | ✘ (`NoopProvider`) |
| `SIGSTOP/SIGCONT` | ✔ | ✔ | ✘ (pause diganti suspend thread/job object, ditunda) |

Saat provider `noop` aktif, daemon **harus** menampilkan peringatan jelas bahwa sesi tidak terisolasi.

## 10. Keputusan Desain Terkait

Lihat [`adr/`](adr/): 0001 Rust murni single binary · 0002 Landlock lebih dulu · 0003 tanpa relay v1 · 0004 `git` CLI + `gix` · 0005 SQLite bundled · 0006 Web PWA tersemat (bukan Tauri/Electron) · 0007 proxy via injeksi BASE_URL.
