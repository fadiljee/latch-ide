# Ekstensibilitas

Tujuan: memungkinkan kustomisasi **tanpa** membangun ekosistem ekstensi setara VS Code (non-goal). Prinsip: deklaratif dulu, kode pihak ketiga terakhir dan tersandbox.

## 1. Tingkat Ekstensibilitas

| Tingkat | Mekanisme | Kode pihak ketiga? | Milestone |
| --- | --- | --- | --- |
| 1 | Konfigurasi: `keybindings.json`, tema, preferensi | Tidak | I2 |
| 2 | Templat tugas (`templates/*.md`), profil agen (`agents.toml`), workflow (`workflow.toml`) | Tidak | I3 |
| 3 | Perintah kustom (`commands.toml`) menjalankan shell/pesan/pipeline | Hanya shell tersandbox milik pengguna | I5 |
| 4 | Impor paket (templat+profil+workflow+perintah) dari berkas/arsip bertanda | Tidak (data) | I5 |
| 5 | Plugin UI (panel iframe) | Ya, tersandbox | pasca-I5 (eksperimental) |
| 6 | Server MCP kustom untuk agen | Ya (proses terpisah) | I5 (didukung via profil) |

## 2. Tema

`themes/nama.json`:

```json
{
  "name": "Malam Biru",
  "extends": "dark",
  "tokens": {
    "--bg": "#0b1020",
    "--fg": "#d6deeb",
    "--accent": "#7aa2f7",
    "--diff-add": "#1f3a2a",
    "--diff-del": "#3a1f25"
  },
  "editor": { "highlight": "default-dark" }
}
```

Hanya token CSS yang dikenali dan allowlist properti; nilai dikontrol (tanpa `url()` eksternal).

## 3. Perintah Kustom (`commands.toml`)

Jenis aksi:

| `kind` | Efek | Batasan |
| --- | --- | --- |
| `shell` | Menjalankan perintah di worktree sesi aktif | Selalu tersandbox kecuali `sandbox=false` + scope `shell-host` aktif |
| `message` | Mengirim pesan templat ke agen | Variabel terbatas |
| `pipeline` | Menjalankan workflow tertentu | Mengikuti validasi workflow |
| `open` | Membuka berkas/tautan di workbench | Hanya dalam workspace |

Variabel templat: `{{session.id}}`, `{{session.branch}}`, `{{selection}}`, `{{file}}`, `{{line}}`, `{{project.name}}`. Nilai di-*escape* saat masuk ke `command`: kirim sebagai env var (`AGENT_SELECTION`), bukan interpolasi string.

## 4. Paket (Bundle)

Arsip `.agentpack` (tar+zstd) berisi manifest dan data:

```text
manifest.toml        # nama, versi, penulis, deskripsi, hash isi
templates/*.md
profiles.toml        # profil agen (tanpa rahasia)
workflows/*.toml
commands.toml
```

Impor menampilkan ringkasan (perintah shell, jaringan, jalur tulis) dan meminta konfirmasi. Verifikasi tanda tangan opsional (`minisign`). Paket **tidak** dapat menimpa kebijakan sandbox global.

## 5. Plugin UI (eksperimental, pasca-I5)

- Panel di `iframe` dengan `sandbox="allow-scripts"` (tanpa `allow-same-origin`), origin terpisah.
- Komunikasi `postMessage` berskema; API minimal: baca daftar sesi, baca diff ringkas, daftarkan perintah palet, tampilkan panel.
- Tanpa akses ke token, `fetch` ke daemon, atau DOM host. Daemon memproksi permintaan data melalui kanal berizin per plugin.
- Manifest meminta izin (`read:sessions`, `read:diff`); pengguna menyetujui.
- Ditunda sampai model keamanan teruji; bukan komitmen v1.

## 6. Integrasi MCP untuk Agen

Profil agen dapat menyertakan server MCP (perintah + env, tanpa rahasia). Daemon menjalankannya di sandbox sesi dan menghubungkan ke agen yang mendukung MCP. Alat internal workbench (opsional): `workbench.open_file`, `workbench.request_review`, `workbench.ask_user` (menampilkan pertanyaan di UI). Semua dibatasi per sesi.

## 7. Kompatibilitas & Versi
Skema berkas konfigurasi memiliki `schema_version`; `latch config check` memigrasi/menolak versi tak dikenal. Perubahan breaking dicatat di CHANGELOG.

## 8. Yang Tidak Didukung
Ekstensi VS Code, tema VS Code mentah (dapat dikonversi manual), marketplace terpusat, eksekusi kode plugin di proses daemon.
