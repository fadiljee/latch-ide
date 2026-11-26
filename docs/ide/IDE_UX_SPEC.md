# Spesifikasi UX Agent IDE

Kerangka tata letak, panel, interaksi, dan perilaku mobile. Semua ASCII hanyalah sketsa; desain visual final dibuat terpisah.

## 1. Prinsip
1. **Status dulu:** apa yang butuh perhatian harus terlihat tanpa mencari.
2. **Review adalah pekerjaan utama.**
3. **Satu pintasan untuk satu aksi, semua di palet.**
4. **Tidak memblokir:** dialog tidak menutup pantauan agen lain.
5. **Padat tetapi terbaca:** densitas tinggi, font monospace untuk kode, sans untuk UI; target sentuh ≥ 44 px di mobile.

## 2. Tata Letak Desktop

```text
+-------------------------------------------------------------------------------+
| ▣ Proyek ▾   🔍 Ctrl+P / Ctrl+Shift+P            Biaya hari ini $3.21  ⚙  🔔(2)|
+------+---------------------------------------------------------+--------------+
|      | [Papan] [Review] [Editor] [Terminal] [Pipeline]  (tab)   | Panel Agen   |
| Rail |                                                         |  Sesi: s-42  |
|      |                       AREA UTAMA                        |  ┌─────────┐ |
| ▤ Mis|                                                         |  │ chat /  │ |
| ▤ Ses|                                                         |  │ aktivitas│|
| ▤ Fil|                                                         |  └─────────┘ |
| ▤ Cri|                                                         |  [Kirim ▸]   |
| ▤ Git|                                                         |  Konteks: @… |
|      |                                                         |  Biaya $0.42 |
+------+---------------------------------------------------------+--------------+
| Status: 3 berjalan · 1 butuh tindakan · RAM daemon 21 MB · sandbox: landlock  |
+-------------------------------------------------------------------------------+
```

- **Rail kiri:** Misi (Mission Control), Sesi, File, Cari, Git, Pipeline.
- **Area utama:** tab; tata letak terpisah (split) horizontal/vertikal.
- **Panel kanan:** konteks sesi terpilih (chat/aktivitas, konteks, biaya). Dapat dilipat.
- **Bilah status:** ringkasan global, indikator sandbox (peringatan merah bila `noop`).

## 3. Mission Control (Papan)

```text
 Antre (1)        Berjalan (3)         Butuh Tindakan (2)     Siap Review (2)   Selesai
┌──────────┐    ┌───────────────┐    ┌──────────────────┐   ┌──────────────┐
│ s-45     │    │ s-42 claude   │    │ s-40 ⚠ approval  │   │ s-39 codex   │
│ tugas…   │    │ "Perbaiki …"  │    │ git push origin… │   │ +120 −34 · 5f│
│          │    │ ▓▓▓▓░ 12m $0.4│    │ [Approve][Deny]  │   │ tes ✔ $0.88  │
└──────────┘    │ ● bekerja     │    └──────────────────┘   │ [Buka Review]│
                └───────────────┘    ┌──────────────────┐   └──────────────┘
                                     │ s-38 ⛔ tripped   │
                                     │ budget $2.03     │
                                     │ [Naikkan][Buang] │
                                     └──────────────────┘
```

Kartu: agen/profil, tugas (dipotong), durasi, biaya, token, file berubah, aktivitas (`working`/`waiting_input`/`idle`), aksi cepat. Seret kartu antar kolom hanya untuk aksi yang valid (mis. Antre → Berjalan = mulai). Inbox Tindakan = filter kolom "Butuh Tindakan" dengan pintasan `Alt+A`.

## 4. Grid Terminal
Mode: **Fokus + strip** (satu besar, lainnya mini dengan pratinjau teks), **2×2**, **daftar**. Setiap sel menampilkan terminal xterm.js; yang tidak terlihat dihentikan render-nya (hanya buffer). Input hanya masuk ke sel fokus. Indikator aktivitas dan biaya di header sel.

## 5. Review (layar paling penting)

```text
┌ Review · s-39 · latch/my-repo/a1b2c3 → main ─────────────────────────────────┐
│ File (5)                    │ src/page.rs                          [Side|Inline]│
│ ▣ src/page.rs      +20 −4   │ @@ -38,7 +38,9 @@                                │
│ ▢ src/api.rs       +80 −10  │  40  let n = items.len();     │  40  let n = …  │
│ ⚠ Cargo.lock       +12 −3   │ -44  if i <= n {              │+ 44  if i < n { │
│ ▢ tests/page.rs    +8  −0   │      [✔ Terima] [✖ Tolak] [💬 Komentar]          │
│                             │                                                  │
│ Komentar tertunda: 2  [Kirim ke agen]                                         │
│ Merge: ✔ tidak ada konflik · tes ✔     [Squash & Merge ▾] [Buang]             │
└──────────────────────────────────────────────────────────────────────────────┘
```

Detail:
- Daftar file diurutkan: berisiko dulu (lockfile, CI, skrip, `.env`, biner) lalu menurut perubahan; penanda ⚠ dengan alasan.
- Navigasi: `j/k` hunk, `]`/`[` file, `a` terima, `r` tolak, `c` komentar, `Shift+Enter` kirim komentar.
- Terima = tandai/stage; Tolak = revert hunk (dengan undo `Ctrl+Z`).
- Panel hasil tes dan biaya sesi di footer.
- Banner jika sesi masih berjalan: "Sesi sedang menulis. Jeda untuk menolak hunk."

## 6. Editor
- Tab dengan penanda: ● belum disimpan, 🤖 sedang disentuh agen.
- Gutter: garis perubahan agen (tambah/ubah/hapus) relatif basis worktree; klik membuka hunk inline.
- Seleksi → bilah mengambang: **Kirim ke agen**, **Jelaskan**, **Perbaiki**.
- Minimap opsional (default mati demi RAM/ruang).
- Pencarian dalam file, ganti, goto line, lipat, multi-kursor dasar, indentasi otomatis, bracket matching.
- Peringatan konflik/ETag: bilah di atas editor dengan aksi **Muat ulang**, **Bandingkan**, **Timpa**.

## 7. Panel Agen
Dua tab: **Aktivitas** (aliran pesan/tool call untuk mode terstruktur; ringkasan aktivitas PTY) dan **Kirim** (kotak prompt).
- Kotak prompt: `@` untuk lampiran (file/folder/diff/test), `/` untuk templat tugas, `Ctrl+Enter` kirim.
- Opsi kirim: ke sesi ini · ke sesi baru · ke beberapa agen (fan-out) · antre.
- Menampilkan pratinjau teks yang akan diketik untuk mode PTY (agar tidak mengejutkan).

## 8. Explorer, Cari, Git
- **Explorer:** pohon dengan ikon, penanda status git/agen; konteks menu: buka, kirim ke agen sebagai konteks, salin path.
- **Cari:** satu kotak dua mode (file `Ctrl+P`, teks `Ctrl+Shift+F`); hasil streaming; pratinjau baris.
- **Git:** status dikelompokkan (staged/unstaged), pesan commit dengan saran dari agen (opsional), riwayat ringkas.

## 9. Pipeline & Arena
Graf node dengan warna status; klik node membuka sesi terkait; retry ditampilkan sebagai panah balik dengan hitungan. Arena = tabel kandidat × metrik (tes, durasi, +/− baris, token, biaya) dengan tombol **Pakai ini** dan perbandingan diff antar kandidat.

## 10. Mobile (≤ 640 px)

```text
┌──────────────────────┐
│ ☰  Agent IDE   🔔(2) │
├──────────────────────┤
│ Butuh Tindakan (2)   │
│ ┌──────────────────┐ │
│ │ s-40 ⚠ approval  │ │
│ │ git push …       │ │
│ │ [Approve] [Deny] │ │
│ └──────────────────┘ │
│ Siap Review (2)      │
│ ┌──────────────────┐ │
│ │ s-39 +120 −34    │ │
│ │ [Review ▸]       │ │
│ └──────────────────┘ │
├──────────────────────┤
│ Sesi │ Review │ Kirim│  ← bottom nav
└──────────────────────┘
```

- Review: diff **inline vertikal**, hunk dapat dilipat, swipe untuk terima/tolak (dengan konfirmasi/undo).
- "Kirim": kotak pesan singkat + templat cepat ("lanjut", "jalankan tes", "berhenti").
- Terminal: layar teks snapshot (read-mostly) dengan input singkat.
- Tidak ada editor multi-tab penuh; edit satu berkas sederhana opsional (FR-I44).

## 11. Notifikasi & Indikator
Bel menampilkan hitungan Butuh Tindakan. Judul tab browser menampilkan `(2)`. Push: selesai, butuh izin, tripped, gagal. Preferensi per jenis event.

## 12. Keadaan Kosong, Galat, Offline
- **Kosong:** onboarding "Tambah proyek → Jalankan agen pertama".
- **Galat daemon/koneksi putus:** banner "Terputus, mencoba menyambung…"; draf edit dipertahankan.
- **Sandbox tidak aktif (`noop`):** banner persisten merah.
- **Token salah/kedaluwarsa:** layar masukkan token; tidak menyimpan di URL.

## 13. Aksesibilitas
Navigasi keyboard penuh, fokus terlihat, peran ARIA pada papan/tab/daftar, kontras AA, `prefers-reduced-motion`, ukuran font dapat diatur, pembaca layar mengumumkan perubahan status sesi (live region sopan).

## 14. Internasionalisasi
Katalog pesan (`id`, `en`), format tanggal/angka lokal, tata letak tidak bergantung pada panjang teks.

## 15. Tema & Desain
Token desain (warna, spasi, radius, tipografi) dalam CSS variables; tema bawaan: Terang, Gelap, Kontras Tinggi. Font: system font stack (tanpa font eksternal); font monospace sistem untuk kode (opsional bundel satu font kecil bila perlu, ukur).
