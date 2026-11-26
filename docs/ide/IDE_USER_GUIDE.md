# Panduan Pengguna - Agent IDE

> Menggambarkan perilaku target. Fitur muncul bertahap sesuai [IDE_ROADMAP](IDE_ROADMAP.md).

## 1. Membuka Workbench

```bash
latch daemon start       # jika belum berjalan
latch ide                # membuka workbench di browser default
latch ide --native       # jendela native (butuh fitur native-shell)
latch ide --no-open      # hanya cetak URL
```

`latch ide` meneruskan token secara otomatis; Anda tidak perlu menyalinnya. Untuk perangkat lain, buat token ber-scope:

```bash
latch token create --scopes read,control --label hp --expires 7d
```

## 2. Konsep Singkat
- **Proyek** = repositori Git.
- **Sesi** = satu agen yang bekerja di **worktree** + branch sendiri.
- **Papan** = tampilan semua sesi menurut status.
- **Review** = meninjau diff sesi, menerima/menolak hunk, mengirim komentar, lalu merge.

## 3. Alur Kerja Harian

### 3.1 Mulai
1. `Alt+N` (Sesi baru) → pilih proyek, profil agen, tulis tugas, atur anggaran → **Mulai**.
2. Ulangi untuk beberapa sesi paralel (batas `max_sessions`).

### 3.2 Pantau
- **Papan:** kolom Antre / Berjalan / Butuh Tindakan / Siap Review / Selesai.
- **Butuh Tindakan** (`Alt+A`): approval menunggu, sesi `tripped`, sesi gagal.
- Indikator aktivitas: *bekerja*, *menunggu input*, *idle* (perkiraan heuristik).
- Biaya dan token tampil per kartu dan di bilah atas.

### 3.3 Berinteraksi dengan agen
- Panel kanan → tab **Kirim**: ketik pesan, tekan `Ctrl+Enter`.
- `@` melampirkan file/folder/diff/hasil tes; `/` memanggil templat tugas.
- Untuk agen mode PTY, Anda melihat pratinjau teks yang akan diketik.
- Pilih kode di editor → **Kirim ke agen** (`Mod+Shift+Enter`) untuk menyertakan path dan rentang baris.

### 3.4 Review
1. Kartu **Siap Review** → **Buka Review**.
2. Telusuri file (berisiko tampil dahulu), baca diff (`s` ganti side-by-side/inline).
3. Per hunk: `a` terima, `r` tolak (dapat di-undo), `c` komentar.
4. **Kirim ke agen** untuk meneruskan komentar terkumpul; tunggu agen memperbaiki.
5. Bila puas: periksa **cek merge**, lalu **Squash & Merge** atau **Buang**.

> Menolak hunk pada sesi yang masih berjalan meminta Anda **menjeda agen** dulu agar perubahan tidak tertimpa.

### 3.5 Mengedit sendiri
- Buka file dari Explorer atau `Mod+P`; edit; `Mod+S` menyimpan ke worktree.
- Tab 🤖 berarti agen sedang menyentuh file itu. Pilih **Jeda agen**, **Berdampingan**, atau **Batal**.
- Konflik simpan → dialog 3-arah (milik Anda vs terbaru).
- Untuk perubahan besar, pertimbangkan membuka worktree di editor favorit Anda: **Buka di editor eksternal**.

### 3.6 Fan-out & Pipeline
- Panel agen → **Kirim ke beberapa agen** untuk mengadu model; hasil di **Arena** (tes, durasi, diff, biaya) → **Pakai ini**.
- Tab **Pipeline** menampilkan DAG `workflow.toml` dan status tiap node.

## 4. Terminal
- Grid terminal: **Fokus+strip**, **2×2**, atau **daftar**. Input hanya ke sel terfokus.
- `Mod+Shift+\`` membuka shell tersandbox pada worktree sesi.
- Shell host (tanpa sandbox) nonaktif secara default; lihat §9.

## 5. Pencarian & Git
- `Mod+P` cari file; `Mod+Shift+F` cari teks (hasil bertahap).
- Panel Git: stage/unstage, commit, riwayat ringkas. `push` tidak tersedia dari UI (lewat terminal/approval).

## 6. Dari HP
1. Pastikan daemon dapat dijangkau lewat SSH tunnel/Tailscale (lihat USER_GUIDE runtime §10).
2. Buka URL, masukkan token `read,control`, pasang sebagai PWA.
3. Layar utama: **Butuh Tindakan** dan **Siap Review**. Review memakai diff vertikal; **Approve/Deny** langsung.
4. **Kirim** untuk pesan cepat ("lanjut", "jalankan tes", "berhenti").
Token HP sebaiknya **tanpa** `write` dan shell.

## 6.1 Notifikasi
Aktifkan notifikasi di Pengaturan → Notifikasi. Jenis: selesai, butuh izin, tripped, gagal. Di iOS, push PWA membutuhkan iOS ≥ 16.4 dan PWA terpasang.

## 7. Kustomisasi

| Berkas (`~/.config/latch/`) | Fungsi |
| --- | --- |
| `keybindings.json` | Pintasan |
| `themes/*.json` | Tema (token warna) |
| `commands.toml` | Perintah palet kustom |
| `templates/*.md` | Templat tugas |
| `lsp.toml` | Server bahasa |

Contoh `commands.toml`:

```toml
[[commands]]
id = "run-tests"
title = "Jalankan tes di sesi aktif"
run = { kind = "shell", sandbox = true, command = "cargo test" }

[[commands]]
id = "ask-review"
title = "Minta agen meninjau diff"
run = { kind = "message", text = "Tinjau perubahanmu sendiri dan daftar risiko." }
```

Contoh templat tugas (`templates/fix-failing-test.md`):

```markdown
---
title: Perbaiki test gagal
variables: [test_name]
---
Test `{{test_name}}` gagal. Cari penyebabnya, perbaiki kodenya (bukan tes), lalu jalankan ulang tes.
```

## 8. LSP (opsional)
Aktifkan di `config.toml`:

```toml
[ide.lsp]
enabled = true
idle_shutdown_minutes = 10
memory_limit_mb = 512
```

`lsp.toml`:

```toml
[servers.rust]
command = "rust-analyzer"
[servers.typescript]
command = "typescript-language-server"
args = ["--stdio"]
```

Server tidak dibundel; pasang sendiri. Server berjalan di sandbox dan dimatikan saat idle.

## 9. Keamanan Praktis
- Gunakan token ber-scope sesempit mungkin; revoke token yang tidak dipakai (`latch token revoke`).
- Jangan menyalin token ke pesan/URL.
- Shell host: `ide.allow_host_shell = true` hanya jika benar-benar perlu, dan hanya dari mesin lokal.
- Berkas yang cocok pola rahasia (mis. `.env`) menampilkan peringatan saat dibuka.
- Tinjau perubahan pada `.github/workflows`, lockfile, skrip build dengan ekstra hati-hati (ditandai ⚠).

## 10. Pemecahan Masalah

| Gejala | Penyebab | Solusi |
| --- | --- | --- |
| UI "Terputus" | Daemon mati/tunnel putus | Periksa `latch daemon status`, sambung ulang tunnel; draf edit aman |
| `403 insufficient_scope` | Token tanpa scope | Buat token dengan scope yang diperlukan |
| Tidak bisa tolak hunk | Sesi masih berjalan | Jeda agen dahulu |
| Simpan ditolak `etag_mismatch` | Agen/proses lain mengubah berkas | Gunakan dialog 3-arah |
| Quick open lambat | Repo sangat besar | Atur `ide.index_ignore` / sparse worktree |
| Watcher berhenti | Batas inotify | Naikkan `fs.inotify.max_user_watches` |
| Banner merah "sandbox: noop" | OS tanpa Landlock/kernel lama | Sesi tidak terisolasi; pakai Linux ≥ 5.13 atau terima risikonya |
| UI lambat dengan banyak terminal | Banyak sel aktif | Pakai mode Fokus+strip |
