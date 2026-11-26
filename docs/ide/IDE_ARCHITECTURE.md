# Arsitektur Agent IDE

Melengkapi [ARCHITECTURE.md](../ARCHITECTURE.md). Fokus: layanan IDE di daemon dan struktur workbench web.

## 1. Gambaran

```text
 Browser / PWA / Shell native (webview)
 +--------------------------------------------------------------+
 |  Workbench (React + TS)                                      |
 |  Mission Control | Review | Editor | Terminal | Git | Search |
 +---------------------------+----------------------------------+
                             | HTTPS/HTTP loopback (tunnel)
                             | REST + WS (token ber-scope)
 +---------------------------v----------------------------------+
 |  daemon                                                      |
 |   workbench-api (axum routes IDE)                            |
 |   +-----------+ +-----------+ +---------+ +-----------+      |
 |   | fs-service| |  search   | | lsp-host| | git-engine|      |
 |   +-----+-----+ +-----+-----+ +----+----+ +-----+-----+      |
 |         |             |            |            |            |
 |   +-----v-------------v------------v------------v-----+      |
 |   |   Session Manager + pty-pool + proxy + dag + store |      |
 |   +----------------------------------------------------+      |
 +--------------------------------------------------------------+
        |                          |
   Worktree (FS)         Proses agen / LSP / shell (tersandbox)
```

Prinsip: **semua logika di daemon**, workbench adalah klien tipis berstatus UI saja.

## 2. Crate Baru

| Crate | Tanggung jawab | Bergantung pada |
| --- | --- | --- |
| `fs-service` | Operasi file aman di akar worktree: tree, baca, tulis (ETag), rename, hapus, watcher, deteksi biner/ukuran | `core` |
| `search` | Indeks nama file (fuzzy), pencarian teks (regex) bertahap, menghormati `.gitignore` | `core` |
| `lsp-host` | Menjalankan server bahasa tersandbox, proxy JSON-RPC lewat WS, siklus hidup/lazy/batas memori | `core`, `sandbox`, `pty-pool` (utilitas proses) |
| `agent-proto` | Adapter agen terstruktur (AGENT_PROTOCOL): stdio JSON-RPC, pemetaan ke event sesi | `core` |
| `workbench-api` | Rute REST/WS IDE, otorisasi scope, pagination, batas ukuran | semua di atas |
| `web/` | Workbench frontend (struktur §8) | - |

Aturan dependensi tetap satu arah; `daemon` merangkai. Setiap crate IDE **dapat dimatikan lewat fitur Cargo** (`ide`, `lsp`) untuk membangun binary runtime-minimal.

## 3. Fitur Cargo & Pengiriman

| Fitur | Efek |
| --- | --- |
| `ide` (bawaan) | Menyertakan `fs-service`, `search`, `workbench-api`, aset workbench |
| `lsp` | Menyertakan `lsp-host` |
| `native-shell` | Menyertakan wrapper webview (`wry`) untuk `latch ide --native` |
| (tanpa `ide`) | Hanya aset PWA ringkas FR-22 (kontrol dan diff) |

Layanan dimuat **lazy**: indeks `search` dan watcher `fs-service` baru dibuat saat workspace pertama dibuka dan dibongkar setelah idle (`ide.idle_unload_minutes`).

## 4. File Service (`fs-service`)

### 4.1 Akses aman
- Operasi dibatasi pada **akar sesi** (worktree). Jalur dinormalisasi; pada Linux gunakan `openat2` dengan `RESOLVE_BENEATH | RESOLVE_NO_MAGICLINKS` (fallback: kanonisasi + verifikasi awalan + tolak symlink keluar).
- Tolak jalur ke `.git/` untuk penulisan oleh UI (git hanya lewat `git-engine`).
- Batas: baca ≤ 5 MiB per berkas (lebih besar: mode potongan/read-only), tulis ≤ 5 MiB, pohon dipaginasi.
- Deteksi biner (byte NUL) dan encoding (UTF-8; deteksi sederhana lain, simpan encoding aslinya).

### 4.2 Konkurensi & versi
Setiap `GET /fs/file` mengembalikan `etag` (hash isi + mtime). `PUT` wajib `If-Match`. Jika tidak cocok → `409 conflict` dengan isi terbaru, UI menawarkan resolusi 3-arah (basis = isi saat dibuka, kiri = punya pengguna, kanan = terbaru).

### 4.3 Watcher
`notify` (inotify) pada akar worktree dengan debounce 50-100 ms, mengabaikan `.git/` dan jalur ter-ignore. Event `fs.changed {paths[]}` dikirim ke klien yang membuka workspace. Batas watch dikelola (rekursif bertahap) untuk menjaga RAM/inotify.

## 5. Pencarian (`search`)

- Indeks nama file: daftar jalur dari `ignore::WalkBuilder` (menghormati `.gitignore`), fuzzy dengan `nucleo`. Disimpan di memori sebagai arena string terkompresi (≈ 50-100 B/berkas → 5 MB untuk 50k berkas; ukur).
- Pencarian teks: `grep-searcher`/`grep-regex` (komponen ripgrep) paralel terbatas (≤ 2 thread), hasil di-*stream* per berkas, dapat dibatalkan.
- Pembaruan indeks inkremental dari watcher.
- Tanpa indeks isi permanen pada v1 (hemat RAM/disk).

## 6. Konkurensi Pengguna vs Agen pada Worktree yang Sama

Masalah: pengguna menyunting file yang sedang ditulis agen.

Kebijakan bawaan:

1. **Deteksi:** `fs-service` menandai berkas "disentuh agen dalam N detik terakhir" (dari watcher + pid proses; bila ambigu, dari waktu).
2. **Peringatan** saat pengguna mulai mengedit berkas ber-tanda; opsi: *Jeda agen* (SIGSTOP), *Lanjut berdampingan*, *Batal*.
3. **Tulis dengan ETag**; konflik → 3-arah.
4. Setelah pengguna menyimpan, UI dapat otomatis mengirim catatan ke agen: *"Pengguna mengubah <file>; baca ulang sebelum mengedit"* (opsi bawaan aktif untuk mode terstruktur; untuk PTY menjadi saran yang dapat diklik).
5. Opsi mode ketat per sesi: `edit_policy = "pause_agent" | "side_by_side" | "readonly_while_running"`.

## 7. Review per Hunk

Dua desain, putuskan di spike IDE-T-002 (lihat IDE_TASKS):

| Opsi | Cara | Kelebihan | Kekurangan |
| --- | --- | --- | --- |
| **A. Revert langsung** | "Tolak hunk" menerapkan patch terbalik ke worktree (`git apply -R` pada hunk) | Sederhana, WYSIWYG | Agen yang sedang berjalan dapat menulis ulang; perlu jeda agen |
| **B. Lapisan review (shadow)** | Daemon menyimpan keputusan hunk; menerapkan saat "Finalisasi review" | Tidak mengganggu agen berjalan | Lebih kompleks; hunk bergeser |

Bawaan: **A** untuk sesi berstatus selesai/jeda; untuk sesi berjalan UI meminta jeda dulu. Semua aksi hunk dapat di-*undo* (tumpukan patch di memori + `git stash`-like cadangan di store).

## 8. Struktur Workbench (`web/`)

```text
web/
├── src/
│   ├── app/                 # shell, router, layout manager (panel resizable)
│   ├── features/
│   │   ├── mission-control/ # papan sesi, inbox tindakan, kartu
│   │   ├── review/          # diff multi-file, hunk actions, komentar
│   │   ├── editor/          # CodeMirror 6, tab, gutter perubahan agen
│   │   ├── terminal/        # xterm.js attach
│   │   ├── explorer/        # pohon file
│   │   ├── search/          # quick open, pencarian global
│   │   ├── git/             # panel git
│   │   ├── agent-panel/     # prompt, konteks, templat, fan-out
│   │   ├── pipelines/       # viewer DAG, arena
│   │   └── settings/        # tema, keybindings, preferensi
│   ├── lib/
│   │   ├── api/             # klien REST/WS dengan tipe dari kontrak
│   │   ├── store/           # Zustand slices
│   │   ├── commands/        # registry perintah + palet
│   │   └── a11y, i18n
│   └── workers/             # diff worker, search-highlight worker
├── public/
└── vite.config.ts
```

Prinsip: pemisahan per fitur, *code splitting* per fitur (editor, terminal, diff dimuat malas), tipe API dihasilkan dari skema (OpenAPI atau `ts-rs`/`specta` dari tipe Rust `core`) agar kontrak sinkron.

## 9. State & Sinkronisasi

- **Sumber kebenaran: daemon.** Workbench menyimpan cache UI; event WS memperbarui store (`seq` untuk menyusul).
- Draf edit disimpan lokal (IndexedDB) agar tahan reconnect (NFR-I09); dibersihkan setelah tersimpan.
- Preferensi (layout, tema, keybindings) disimpan di daemon (`GET/PUT /api/v1/prefs`) dan di-*cache* lokal.

## 10. Alur Utama

### 10.1 Review → umpan balik → merge
```text
Sesi selesai -> event 'session.status completed' -> kartu pindah ke "Siap Review"
Pengguna buka Review -> GET diff -> pilih hunk -> komentar/terima/tolak
"Kirim ke agen" -> POST /sessions/{id}/messages {kind:"review_feedback", items:[...]}
Agen memperbaiki -> diff diperbarui via WS (fs.changed / session.diff_updated)
Puas -> merge-check -> merge squash -> sesi Selesai, worktree dibersihkan
```

### 10.2 Edit manual
```text
Buka file -> GET /fs/file (etag) -> edit -> Ctrl+S -> PUT If-Match
 sukses: etag baru, gutter diperbarui
 409: dialog 3-arah -> gabung -> PUT ulang
```

### 10.3 Fan-out dari UI
```text
Panel agen: pilih profil A,B + prompt -> POST /pipelines (ad-hoc fanout)
Arena menampilkan kandidat live -> pilih pemenang -> merge
```

## 11. Shell Native Opsional

`latch ide --native` membuka jendela webview sistem (`wry`+`tao`) yang memuat `http://127.0.0.1:<port>/#token` lewat IPC aman (token dikirim sebagai header awal, bukan URL bila memungkinkan). Hanya menambah ukuran bila fitur `native-shell` diaktifkan; Linux membutuhkan WebKitGTK terpasang (dependensi sistem, bukan dibundel). Bila tidak tersedia, jatuh ke browser default.

## 12. Observabilitas IDE

Metrik internal: jumlah workspace terbuka, ukuran indeks, jumlah watcher, klien WS aktif, latensi operasi fs/search (p50/p99). Ditampilkan di `GET /metrics` dan halaman "Diagnostik" workbench.

## 13. Kompatibilitas
Layanan `fs-service`, `search`, `workbench-api` berjalan lintas OS. `lsp-host` dan sandbox ketat hanya Linux pada v1; di OS lain terminal/LSP berjalan tanpa isolasi dengan peringatan menonjol atau dinonaktifkan secara default.
