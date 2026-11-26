# Roadmap - Agent IDE

Jalur IDE berjalan setelah runtime menyediakan server web, WebSocket, dan token (v0.2). Hubungan milestone:

| Milestone IDE | Bergantung pada runtime | Target rilis | Tema |
| --- | --- | --- | --- |
| **I0** | v0.1 selesai | paralel dengan v0.2 | Spike & fondasi workbench |
| **I1** | v0.2 (FR-22, FR-23) | `v0.4`* | Mission Control + Review dasar + terminal |
| **I2** | v0.3 (sandbox) disarankan | `v0.5` | Editor, explorer, hunk review, konkurensi |
| **I3** | - | `v0.6` | Pencarian, git, konteks agen, dashboard biaya |
| **I4** | v1.0 (pipeline) | `v1.1` | Pipeline/arena UI, mode agen terstruktur |
| **I5** | - | `v1.2` | LSP, resolusi konflik, shell native, ekstensibilitas |

\* Penomoran rilis bersifat usulan; sesuaikan dengan jalur versi runtime. Alternatif: rilis workbench sebagai `ide-0.x` terpisah.

> Urutan prioritas adalah **kontrol dan review agen → penyuntingan → navigasi → LSP**. LSP dan editor tingkat lanjut adalah yang pertama dikorbankan bila jadwal ketat.

## I0 - Spike & Fondasi
**Isi:** spike ukuran bundel (CM6 + xterm + diff), spike review per hunk (opsi A vs B), spike `fs-service` aman (`openat2`), spike memori indeks 50k berkas, rancangan desain visual dan token, bootstrap `web/` + generasi tipe.
**DoD:** laporan angka ukuran/RAM; keputusan ADR-0012..0013 final; kerangka `web/` di CI dengan gerbang ukuran.

## I1 - Mission Control & Review Dasar
**Cakupan:** FR-I01, I02, I10-I14, I20, I30, I33, I60, I100-I102 (+ I101).
**DoD**
- `latch ide` membuka workbench dengan token otomatis.
- Papan sesi live; inbox tindakan; approve/deny dari UI dan HP.
- Grid terminal dengan input ke sesi fokus.
- Diff multi-file; merge-check dan merge/discard dari UI.
- Kirim pesan teks ke agen (mode PTY).
- Layout mobile fungsional untuk papan, approve, dan diff.
- Gerbang ukuran dan a11y dasar hijau.

## I2 - Editor, Explorer, Hunk Review
**Cakupan:** FR-I03-I06, I31, I34, I40-I43, I50, I53.
**DoD**
- Explorer + quick open; editor CM6 simpan dengan ETag; konflik 3-arah.
- Watcher menyinkronkan perubahan agen; gutter perubahan.
- Terima/tolak hunk dengan undo; kebijakan `edit_policy`.
- Keybindings dan tema; preferensi tersimpan di daemon.
- Tes keamanan fs lulus (IT1, IT2).

## I3 - Navigasi, Git, Konteks
**Cakupan:** FR-I15, I16, I21, I22, I32, I35, I51, I61, I63, I70, I104.
**DoD**
- Pencarian teks streaming; panel git (stage/commit).
- Lampiran konteks (`@file`, seleksi, diff, test output) dan templat tugas.
- Komentar review terkumpul dikirim ke agen.
- Shell tersandbox; tautan `file:line` dari terminal.
- Dashboard biaya; riwayat sesi; multi-workspace.

## I4 - Pipeline, Arena, Mode Terstruktur
**Cakupan:** FR-I23-I25, I36, I72, I90, I92.
**DoD**
- Viewer DAG live dan arena dengan "Pakai kandidat ini".
- Fan-out dari panel agen.
- Adaptor protokol terstruktur (ACP atau pilihan ADR-0016) berfungsi dengan minimal satu agen nyata; permintaan izin terstruktur tampil di UI/HP.
- Sinkron dengan base + deteksi konflik.

## I5 - LSP, Konflik, Native, Ekstensibilitas
**Cakupan:** FR-I44, I52, I62, I71, I73, I80-I82, I91, I103, I110-I111.
**DoD**
- LSP tersandbox lazy untuk ≥ 3 bahasa (Rust, TypeScript, Python) dengan batas memori.
- Resolusi konflik 3-arah + "minta agen".
- `latch ide --native` berfungsi di Linux (WebKitGTK) dan didokumentasikan.
- Perintah kustom, impor templat/profil.
- Dokumentasi pengguna final; rilis.

## Pasca-I5 (di luar komitmen)
- Plugin UI tersandbox (FR-I112).
- Mode Vim/Emacs.
- Kolaborasi multi-user.
- Pratinjau aplikasi (port forwarding browser terintegrasi).
- Sandbox macOS/Windows untuk terminal/LSP.

## Gerbang Rilis IDE
1. DoD milestone lulus di CI + e2e.
2. Anggaran NFR-I01..I06 tidak dilanggar.
3. Tes keamanan IDE_SECURITY §7 hijau.
4. A11y AA komponen inti.
5. Dokumen dan CHANGELOG diperbarui.
