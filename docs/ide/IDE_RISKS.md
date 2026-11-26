# Risiko & Mitigasi - Agent IDE

Melengkapi [RISKS.md](../RISKS.md). K = kemungkinan, D = dampak (R/S/T).

| ID | Risiko | Dampak | Mitigasi | K | D | Milestone |
| --- | --- | --- | --- | --- | --- | --- |
| IR-01 | **Scope creep menjadi editor/IDE penuh** | Jadwal molor, kehilangan fokus agent-first | Daftar "tidak akan dibangun" mengikat (IDE_PRD §5.2); urutan prioritas; ADR untuk tambahan | T | T | semua |
| IR-02 | **Ukuran bundel melebihi 3 MB** | Gagal NFR-I01/NFR-04, UI lambat di HP | Lazy per fitur, CM6 bukan Monaco, size-limit di CI, audit dependensi | S | S | I0+ |
| IR-03 | **Latensi ketik/jank dengan banyak terminal aktif** | UX buruk | Hentikan render terminal tak terlihat, virtualisasi, worker, batching | S | S | I1-I2 |
| IR-04 | **Race pengguna vs agen pada berkas sama** | Kehilangan kerja | ETag, deteksi sentuhan agen, jeda opsional, 3-arah, undo | T | T | I2 |
| IR-05 | **Hunk bergeser/tidak stabil saat agen menulis** | Aksi terima/tolak mengenai bagian salah | `hunk_id` berbasis konteks, verifikasi sebelum apply, minta jeda untuk sesi berjalan, respons memuat hunk terbaru | S | S | I2 |
| IR-06 | **Token web dengan scope tulis dicuri** | Penulisan file/terminal oleh penyerang | Scope sempit untuk HP, masa berlaku, revoke, shell-host terkunci, header keamanan | R | T | I1-I2 |
| IR-07 | **XSS dari konten agen/repo** | Pencurian token, kontrol UI | Sanitizer, CSP ketat, tidak ada inline script, tes keamanan | S | T | I1+ |
| IR-08 | **RAM daemon bengkak oleh indeks/watcher/LSP** | Gagal NFR-I05/NFR-01 | Lazy + unload idle, indeks arena, batas watch, LSP proses terpisah dengan limit | S | S | I2-I5 |
| IR-09 | **Repo besar/monorepo** (50k+ berkas) | Quick open/pencarian lambat, inotify habis | Sparse worktree, batas indeks, peringatan, watch bertahap, dokumentasi sysctl | S | S | I2-I3 |
| IR-10 | **Mode terstruktur (ACP) bergeser/tidak didukung agen** | Investasi sia-sia | Jalur PTY tetap universal; adaptor terisolasi di `agent-proto`; spike awal; ADR dapat ditinjau ulang | S | R | I4 |
| IR-11 | **Pesan terformat ke PTY merusak input agen** | Agen salah tafsir/terkirim prematur | Bracketed paste, templat per profil, pratinjau, konfirmasi bila agen menunggu input | S | S | I1 |
| IR-12 | **Heuristik aktivitas salah** | Kartu menampilkan status keliru | Label "perkiraan", `prompt_regex` per profil, jangan dipakai untuk keputusan otomatis | S | R | I1 |
| IR-13 | **Shell native bergantung webview sistem** | Fragmentasi (WebKitGTK versi, WebView2) | Opsional, browser tetap jalur utama, dokumentasi dependensi, uji matriks | T | R | I5 |
| IR-14 | **Kompatibilitas browser/mobile (Safari, iOS push)** | Fitur tak jalan di HP | Matriks e2e, fallback, dokumentasi batas iOS | S | S | I1 |
| IR-15 | **LSP tidak aman/berat** | RAM/CPU, eksekusi kode proyek | Sandbox, batas memori, hanya dari config pengguna, filter metode, nonaktif default | S | S | I5 |
| IR-16 | **Duplikasi fungsi dengan editor lain** | Pengguna tetap pindah ke VS Code | Fokus pada review & kontrol agen; tombol "Buka di editor eksternal" (`$EDITOR`/`code`/`zed` URL handler) | T | R | I2 |
| IR-17 | **Kelelahan review (terlalu banyak diff)** | Review dangkal | Penanda berisiko, ringkasan, urutan file, statistik test, batas ukuran diff, ringkasan oleh agen (opsional, ditandai) | S | S | I3 |
| IR-18 | **Aksesibilitas terabaikan** | Pengguna tertentu terhambat, risiko hukum | Linter a11y, audit berkala, gerbang CI | S | R | semua |
| IR-19 | **Drift kontrak Rust↔TS** | Bug runtime | Pembangkit tipe + CI memeriksa sinkron | S | R | I0+ |
| IR-20 | **Penulisan dari UI melewati sandbox agen** | Daemon (tidak tersandbox) menjadi vektor | Validasi jalur ketat, daftar tolak tulis, audit, fuzz, scope `write` | R | T | I2 |
| IR-21 | **Klaim kompetitif tidak akurat** | Reputasi | Verifikasi fitur Orca/enowX sebelum publikasi (COMPETITIVE_POSITIONING) | S | R | rilis |
| IR-22 | **Beban pemeliharaan frontend + backend oleh tim kecil** | Kecepatan turun | Batas fitur, tooling otomatis, komponen kecil, dokumentasi AGENTS_IDE | T | S | semua |

## Spike untuk Menurunkan Risiko
IT-001 (IR-02), IT-002 (IR-05), IT-003 (IR-20), IT-004 (IR-08/09), IT-005 (IR-10).

## Pelacakan
Tinjau tabel pada akhir tiap milestone; risiko K×D tinggi wajib punya pemilik dan rencana. Status: Terbuka / Dimitigasi / Diterima / Ditutup.
