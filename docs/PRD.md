# Product Requirements Document (PRD)

**Produk:** Autonomous Rust Agent Runtime (`Latch (`latch-ide`)`)
**Status:** Draft v1
**Dokumen terkait:** [REQUIREMENTS](REQUIREMENTS.md) · [ARCHITECTURE](ARCHITECTURE.md) · [ROADMAP](ROADMAP.md) · [SECURITY](SECURITY.md)

---

## 1. Ringkasan Eksekutif

`Latch (`latch-ide`)` adalah *runtime* dan orkestrator agen *coding* CLI dalam **satu binary Rust statis**. Produk ini menggabungkan:

- isolasi kode ala Orca (satu tugas = satu `git worktree` + branch),
- efisiensi *daemon/fan-out* ala enowX (banyak sesi paralel, memori kecil),

lalu menambahkan **OS sandbox**, **local LLM proxy** (telemetri + *circuit breaker*), dan **pipeline estafet deterministik** (DAG). Antarmuka berupa TUI dan Web PWA tersemat sehingga bisa dikendalikan dari terminal maupun HP.

## 2. Latar Belakang & Masalah

| # | Masalah | Dampak |
| --- | --- | --- |
| P1 | Orkestrator berbasis Electron menyedot RAM tinggi saat banyak sesi agen berjalan | Laptop berspesifikasi terbatas melambat/hang, jumlah sesi paralel dibatasi hardware |
| P2 | Agen otonom berjalan tanpa isolasi sistem | Risiko merusak OS, menghapus file di luar proyek, atau membocorkan kredensial (`~/.ssh`, `~/.aws`, `.env`) |
| P3 | Tidak ada visibilitas *real-time* atas token, biaya, dan *infinite loop* | Kuota/uang API terbakar saat pengguna meninggalkan agen |
| P4 | Koordinasi antar-agen (perencana, pembuat kode, penguji) manual | Lambat, rawan salah, tidak dapat diulang |

## 3. Visi & Tujuan

**Visi.** Menjadi lapisan runtime paling ringan dan paling aman untuk menjalankan agen *coding* secara paralel dan otonom.

**Tujuan v1 (terukur):**

1. Menjalankan ≥ 3 sesi agen paralel dengan RSS daemon idle ≤ 25 MB dan overhead ≤ 5 MB per sesi.
2. Menghentikan agen yang *loop*/melewati anggaran dalam < 1 detik sejak ambang terlampaui.
3. Memblokir akses agen ke path sensitif di tingkat kernel (`EACCES`) pada Linux.
4. Mengeksekusi pipeline Planner → Fan-Out Coder → Test Gate → Retry secara deklaratif.
5. Memungkinkan *review* diff dan Approve/Reject dari HP dengan respons < 500 ms.

**Metrik keberhasilan:**

| Metrik | Target |
| --- | --- |
| RSS daemon idle | ≤ 25 MB |
| Waktu *trip* circuit breaker | < 1 s |
| Latensi tambahan proxy | ≤ 2 ms |
| Ukuran binary musl | < 30 MB |
| Latensi Approve dari PWA ke sesi lanjut | < 500 ms |
| Kebocoran kredensial ke SQLite/log | 0 kasus (diuji otomatis) |

## 4. Target Pengguna & Persona

| Persona | Deskripsi | Kebutuhan utama |
| --- | --- | --- |
| **Dina - Developer individu** | Memakai 2-3 CLI agen di laptop 8 GB | RAM kecil, isolasi, diff terpisah |
| **Rafi - Power user Linux** | Hidup di terminal, VPS remote | TUI cepat, daemon remote lewat SSH/Tailscale, skrip |
| **Tim Engineering** | 3-10 orang, tiap orang menjalankan agen | Kebijakan sandbox seragam, biaya terpantau, pipeline yang dapat diulang |

Agen yang didukung: Claude Code, Codex CLI, OpenCode, Gemini CLI, dan skrip agen kustom (apa pun yang dapat dijalankan sebagai proses CLI interaktif di PTY).

## 5. Ruang Lingkup

### 5.1 In-Scope

- *Headless daemon* Rust dengan persistensi SQLite.
- Isolasi `git worktree`, manajemen PTY paralel, *idle hibernation*.
- *Local LLM reverse proxy* dengan pelacakan token dan *circuit breaker*.
- *OS sandbox* (Linux-first via Landlock) dan *approval gate*.
- Eksekutor pipeline DAG dan mode *fan-out* berbasis `workflow.toml`.
- TUI (`ratatui`) dan Web UI/PWA tersemat (`rust-embed`).

### 5.2 Out-of-Scope (Non-Goals)

- Membangun model LLM atau menjadi IDE serbaguna sekelas VS Code/JetBrains (tanpa ekosistem ekstensi VS Code, debugger, refactoring mendalam, atau notebook; untuk *Agent IDE / Workbench* lihat [ide/IDE_PRD.md](ide/IDE_PRD.md)).
- Membundel Electron/Chromium.
- Cloud SaaS multi-tenant pada v1 (fokus *local-first* dan *self-hosted remote daemon*).
- Server *relay* milik sendiri (gunakan SSH forwarding/Tailscale).
- Sandbox macOS/Windows pada v1 (hanya disiapkan abstraksinya).

## 6. Prinsip Desain

1. **Hemat sumber daya dulu.** Setiap dependensi harus membenarkan biayanya terhadap RAM dan ukuran binary.
2. **Default aman.** Sandbox, bind `127.0.0.1`, token acak, dan penolakan secara bawaan.
3. **Tidak mengganggu agen.** Bila proxy/sandbox tidak kompatibel dengan suatu CLI, ada mode bypass yang jelas, bukan kegagalan diam-diam.
4. **Pulih dan transparan.** Semua state di SQLite; crash satu sesi tidak menjatuhkan daemon.
5. **Deterministik bila bisa.** Pipeline memakai node shell/test untuk keputusan objektif, agen hanya untuk pekerjaan generatif.
6. **Klien tipis.** Logika ada di daemon; TUI dan Web hanya menyajikan dan mengirim perintah.

## 7. Persyaratan Produk

Rincian lengkap FR-01 s.d. FR-23 dan NFR-01 s.d. NFR-07 ada di [REQUIREMENTS.md](REQUIREMENTS.md). Ringkasan per kelompok:

| Kelompok | FR | Prioritas dominan |
| --- | --- | --- |
| A. Daemon Core, State, Hibernation | FR-01 s.d. FR-04 | P0 (FR-04: P2) |
| B. Git Worktree & PTY Manager | FR-05 s.d. FR-08 | P0 |
| C. Local LLM Proxy & Circuit Breaker | FR-09 s.d. FR-13 | P1 |
| D. OS Sandbox & Permission Firewall | FR-14 s.d. FR-17 | P1 (FR-17: P2) |
| E. DAG Pipeline & Fan-Out | FR-18 s.d. FR-20 | P2 |
| F. Thin Clients | FR-21 s.d. FR-23 | P0 (TUI), P1 (Web, notifikasi) |

## 8. User Stories & Kriteria Penerimaan

### US-1 Eksekusi Paralel Terisolasi
*Sebagai developer, saya ingin menjalankan 3 agen CLI berbeda pada satu repositori secara bersamaan tanpa risiko konflik file agar pekerjaan fitur paralel selesai lebih cepat.*

**Kriteria penerimaan**
- Setiap sesi otomatis mendapat `git worktree` dan branch unik.
- Mengedit file yang sama di dua sesi tidak saling menimpa.
- Seluruh diff dapat dilihat terpisah sebelum di-*merge*.

**Requirement:** FR-05, FR-06, FR-07, FR-08

### US-2 Proteksi Kuota via Circuit Breaker
*Sebagai developer, saya ingin sistem otomatis menghentikan agen yang terjebak error loop atau melewati batas biaya agar kuota API tidak habis saat saya tinggal.*

**Kriteria penerimaan**
- Saat sesi melewati `max_budget_usd` atau mengulang error yang sama `max_consecutive_errors` kali, status sesi menjadi `tripped`.
- Proses agen berhenti dalam < 1 detik.
- Notifikasi muncul di klien.

**Requirement:** FR-10, FR-11, FR-12, FR-23

### US-3 Eksekusi Aman dengan Sandbox
*Sebagai developer, saya ingin menjalankan agen dalam mode otonom tanpa takut agen membaca kunci SSH atau menghapus file di luar proyek.*

**Kriteria penerimaan**
- Percobaan membaca `~/.ssh/id_rsa` atau menulis ke direktori induk di luar worktree ditolak kernel (`EACCES`).
- Percobaan tercatat di log audit.
- File asli tidak terganggu.

**Requirement:** FR-14, FR-15, FR-16

### US-4 Kontrol & Approval dari HP
*Sebagai developer yang tidak di depan laptop, saya ingin menerima notifikasi saat agen selesai atau meminta izin, lalu melihat diff singkat dari HP.*

**Kriteria penerimaan**
- PWA dapat diakses lewat Tailscale/SSH tunnel dengan token valid.
- Daftar file berubah tampil nyaman di layar ponsel.
- Menekan **Approve** melanjutkan sesi di daemon dalam < 500 ms.

**Requirement:** FR-03, FR-17, FR-22, FR-23

### US-5 Estafet Otomatis & Fan-Out
*Sebagai developer, saya ingin satu perintah pipeline yang mengadu dua model pada tugas yang sama lalu memilih kandidat yang lolos unit test dengan biaya terendah.*

**Kriteria penerimaan**
- Dua worktree paralel berjalan, perintah test otomatis dieksekusi pada keduanya.
- *Retry* terpicu bila gagal (maksimal sesuai batas).
- Tabel skor perbandingan ditampilkan di akhir.

**Requirement:** FR-18, FR-19, FR-20

## 9. Alur Pengguna Utama

1. **Satu agen, satu tugas.** `latch run` → daemon membuat worktree + branch → PTY dijalankan dengan env proxy + sandbox → pengguna memantau di TUI → melihat diff → merge atau buang.
2. **Pipeline.** `latch pipeline run workflow.toml` → DAG divalidasi → Planner → Fan-Out → Test Gate (retry bila gagal) → tabel perbandingan → pengguna memilih pemenang → merge.
3. **Remote.** Daemon di VPS → klien tersambung via SSH tunnel/Tailscale → PWA di HP menerima notifikasi → Approve/Reject.

## 10. Batasan & Asumsi

- Linux kernel ≥ 5.13 diperlukan untuk Landlock penuh (ABI v1); fitur non-sandbox tetap berjalan di macOS/Windows.
- Repo target adalah repositori Git valid dengan `git` ≥ 2.38 terpasang.
- CLI agen dipasang dan diautentikasi sendiri oleh pengguna; runtime tidak mengelola kredensial penyedia LLM.
- Kompatibilitas proxy bergantung pada kemampuan CLI membaca `*_BASE_URL`; sisanya memakai mode bypass (FR-13).

## 11. Dependensi & Integrasi

| Dependensi | Alasan |
| --- | --- |
| `git` CLI | Manajemen worktree yang paling stabil |
| Kernel Landlock | Sandbox filesystem |
| Tailscale / SSH | Transport remote terenkripsi tanpa relay sendiri |
| API penyedia LLM (OpenAI, Anthropic, Google) | Lalu lintas yang diproksi |

## 12. Risiko Utama

Ringkasan; rincian di [RISKS.md](RISKS.md): CLI menolak proxy lokal, konflik merge pasca-paralel, kompleksitas aturan Landlock, banjir output terminal, serta kesenjangan intersepsi aksi untuk *approval gate* (lihat [SECURITY.md](SECURITY.md)).

## 13. Rencana Rilis

| Fase | Versi | Fokus |
| --- | --- | --- |
| 1 | v0.1 | Fondasi: daemon, SQLite, worktree, PTY, diff/merge, TUI |
| 2 | v0.2 | Proxy, telemetri, circuit breaker, Web PWA, notifikasi, remote |
| 3 | v0.3 | Landlock, denylist, trait sandbox, approval gate |
| 4 | v1.0 | Workflow DAG, estafet, fan-out, hibernasi |

Detail di [ROADMAP.md](ROADMAP.md).

## 14. Pertanyaan Terbuka

1. Bagaimana tepatnya *approval gate* mencegat aksi yang dieksekusi di dalam CLI agen (PATH shim, hook bawaan agen, atau inspeksi tool-call di proxy)? Lihat ADR terkait di [SECURITY.md](SECURITY.md#7-approval-gate-fr-17).
2. Apakah agen boleh melakukan `git commit` sendiri (butuh akses tulis ke `.git` repo utama) atau daemon yang commit? Lihat [ADR-0004](adr/0004-git-cli-plus-gix.md) dan SECURITY.
3. Strategi kebijakan harga model (`pricing.toml`) yang diperbarui bagaimana?
4. Lisensi proyek.
5. Nama final produk dan binary.

## 15. Riwayat Dokumen

| Versi | Tanggal | Catatan |
| --- | --- | --- |
| Draft v1 | - | Disusun dari Blueprint Requirement dan Tech Stack |
