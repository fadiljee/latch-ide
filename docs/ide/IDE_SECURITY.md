# Keamanan - Agent IDE

Melengkapi [SECURITY.md](../SECURITY.md). Workbench memperluas permukaan serangan: klien web kini dapat **menulis file, membuka terminal, dan menjalankan LSP**. Kompromi token menjadi jauh lebih berbahaya daripada pada versi kontrol-saja.

## 1. Aset Baru & Batas Kepercayaan

| Aset | Risiko |
| --- | --- |
| Isi worktree (kode pengguna) | Ditulis/dihapus lewat UI atau token curian |
| Terminal host (jika diaktifkan) | Eksekusi kode arbitrer sebagai pengguna |
| Server LSP | Proses tambahan yang membaca kode dan dapat dijalankan oleh konfigurasi proyek |
| Konten dari agen/repo ditampilkan di UI | XSS/injeksi UI |
| Preferensi/keybindings/perintah kustom | Perintah kustom dapat menjalankan shell |

**Batas kepercayaan:** isi repo, keluaran agen, pesan LLM, nama file, dan hasil pencarian = **tak tepercaya**. Hanya pengguna ber-token dan UI dari aset tertanam yang tepercaya.

## 2. Model Ancaman Tambahan

| # | Ancaman | Vektor | Mitigasi |
| --- | --- | --- | --- |
| IT1 | Path traversal / symlink escape lewat API fs | `../`, symlink di repo ke `~/.ssh` | `openat2 RESOLVE_BENEATH`, tolak jalur absolut dan symlink keluar akar, uji fuzz; tulis hanya di worktree sesi |
| IT2 | Penulisan ke `.git/` (hook, config) lewat UI | Berkas `.git/hooks/*` atau `.git/config` | Larang tulis `.git/*` pada fs-service; commit lewat git-engine dengan hook dimatikan |
| IT3 | XSS dari konten agen/repo | Markdown/HTML berbahaya, nama file berisi skrip, diff HTML | Render teks sebagai teks; markdown via sanitizer (DOMPurify) dengan allowlist; tanpa `dangerouslySetInnerHTML` kecuali sanitized; CSP ketat tanpa `unsafe-eval`/inline script |
| IT4 | Pencurian token dari browser | XSS, ekstensi browser jahat, riwayat | Token di memori (bukan URL); `HttpOnly`-setara tidak tersedia untuk bearer → minimalkan umur, scope sempit, rotasi, `Referrer-Policy: no-referrer` |
| IT5 | CSRF/DNS rebinding terhadap daemon lokal | Halaman jahat memanggil `127.0.0.1` | Bearer wajib (bukan cookie), validasi `Origin`/`Host`, tolak `Host` tak dikenal |
| IT6 | Eskalasi dari token HP | Token HP bocor | Token ber-scope `read,control` untuk HP; `write`/shell tidak diberikan; masa berlaku; revoke cepat |
| IT7 | Terminal host menjadi RCE | Token dengan `shell-host` | Nonaktif default; butuh flag config + konfirmasi di klien lokal (bukan remote); log audit; tidak tersedia lewat token remote kecuali dikonfigurasi eksplisit |
| IT8 | LSP berbahaya / konfigurasi proyek menjalankan perintah | `.vscode`/config proyek menentukan server | `lsp.toml` hanya dari config pengguna (bukan repo); server dijalankan di sandbox; metode berbahaya difilter |
| IT9 | Edit manual menimpa kerja agen atau sebaliknya | Race | ETag, deteksi sentuhan agen, jeda opsional (IDE_ARCHITECTURE §6) |
| IT10 | Perintah kustom berbahaya | `commands.toml` dari sumber tak tepercaya | Hanya dibaca dari config pengguna; impor berbagi menampilkan ringkasan dan minta konfirmasi |
| IT11 | Penolakan layanan lewat pencarian/tree besar | Regex catastrophic, repo raksasa | Batas waktu/ukuran/hasil; mesin regex linear (ripgrep `regex` tanpa backtracking); batalkan |
| IT12 | Kebocoran rahasia lewat UI | Menampilkan `.env`, token di terminal | Peringatan saat membuka berkas yang cocok pola rahasia; opsi sembunyikan; log tidak menyimpan isi file |
| IT13 | Komentar review/pesan memuat prompt injection ke agen | Kontributor jahat menulis komentar | Pesan hanya dari pengguna ber-token; tandai pesan otomatis; konten berkas lampiran diberi pembatas jelas |
| IT14 | Plugin UI pihak ketiga | Panel iframe | Eksperimental; iframe `sandbox` tanpa `allow-same-origin`, kanal `postMessage` berskema, tanpa akses token |
| IT15 | Aset web dimodifikasi | Binary dipalsukan | Aset tertanam di binary bertanda tangan/checksum rilis; tanpa CDN |

## 3. Kebijakan Terminal

| Jenis | Sandbox | Scope | Default |
| --- | --- | --- | --- |
| Terminal agen (PTY sesi) | Ya (kebijakan sesi) | `read` baca; `control` tulis | Aktif |
| Shell tersandbox | Ya (kebijakan worktree, tanpa preset jaringan ekstra) | `shell-sandbox` | Aktif untuk token lokal |
| Shell host | Tidak | `shell-host` | **Nonaktif**; aktifkan `ide.allow_host_shell = true` dan hanya untuk koneksi UDS/loopback lokal; tunneling remote menolak kecuali `ide.allow_host_shell_remote = true` |

Peringatan UI menonjol saat shell host dibuka. Semua pembukaan dicatat di `audit_log`.

## 4. Keamanan Tulis File
1. Setiap `PUT/ops` memverifikasi sesi ada, status memungkinkan, scope `write`, ukuran, dan jalur.
2. Daftar `deny_write_globs` bawaan: `.git/**`, `**/.env*` bila di luar worktree (tidak relevan di dalam), serta konfigurasi sensitif (opsional: `.github/workflows/**` memerlukan konfirmasi UI).
3. Penghapusan massal (> N berkas) memerlukan konfirmasi.
4. Cadangan satu-langkah (undo) untuk penimpaan: salinan di `state_dir/undo/` (retensi singkat, `0600`).

## 5. Header & Kebijakan Web

- `Content-Security-Policy` (lihat IDE_API §11), `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`, `Cross-Origin-Opener-Policy: same-origin`, `Cross-Origin-Resource-Policy: same-origin`, `X-Frame-Options: DENY`.
- Tidak ada cookie otentikasi. WebSocket diautentikasi sebagaimana di API.md §2.
- Subresource dari origin sendiri saja; tanpa font/skrip CDN.

## 6. Privasi & Telemetri
- Tanpa telemetri keluar (*phone-home*) secara default. Bila ada telemetri anonim di masa depan: opt-in, terdokumentasi, tanpa isi kode.
- Preferensi/perintah kustom tidak disinkronkan ke cloud.

## 7. Tes Keamanan Wajib (tambahan)

| Tes | Hasil |
| --- | --- |
| Fuzz jalur fs (`..`, `//`, `%2e`, symlink, unicode normalization) | Tidak ada akses di luar akar |
| PUT ke `.git/config` / `.git/hooks/x` | `403` |
| Membaca symlink yang menunjuk `~/.ssh/id_rsa` | Ditolak |
| Render markdown/diff berisi `<script>`/`onerror` | Tidak dieksekusi |
| Panggilan dari `Origin` asing | Ditolak |
| Token `read,control` memanggil PUT file / buka terminal | `403 insufficient_scope` |
| `shell-host` tanpa aktivasi | Ditolak |
| LSP dengan `executeCommand` | Diblokir |
| Pencarian regex patologis | Dihentikan oleh batas waktu |
| Kanari kunci di isi file | Tidak muncul di log/DB |

## 8. Batasan yang Diakui
- Browser bukan batas kepercayaan sempurna: ekstensi browser berbahaya dapat membaca token di memori halaman.
- Shell native mengurangi paparan ekstensi, tetapi bergantung pada webview sistem.
- Penulisan manual oleh pengguna dari UI dilakukan oleh daemon (di luar sandbox agen) dan **tidak** dibatasi Landlock agen; perlindungan bergantung pada validasi jalur daemon, bukan kernel.
