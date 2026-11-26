# Keamanan

Dokumen ini mendefinisikan model ancaman, kebijakan sandbox, autentikasi, dan batasan yang jujur dari desain. Kebutuhan terkait: FR-13 s.d. FR-17, NFR-05, NFR-06.

## 1. Tujuan Keamanan

1. Agen otonom tidak dapat **membaca kredensial** di luar worktree-nya (SSH, AWS, GPG, keyring, `.env` proyek lain).
2. Agen tidak dapat **menulis/menghapus** di luar worktree dan `/tmp` terisolasi.
3. **Rahasia tidak bocor** lewat penyimpanan sendiri: SQLite dan log tidak memuat API key atau header `Authorization`.
4. Antarmuka kontrol (Web/WS) hanya dapat dipakai pemegang **token** lokal.
5. Aksi berisiko dapat **dijeda dan disetujui manusia**.

## 2. Aset & Batas Kepercayaan

| Aset | Pemilik | Risiko |
| --- | --- | --- |
| Kunci SSH/cloud/GPG pengguna | Pengguna | Dicuri oleh agen/prompt injection |
| Kode proyek lain di mesin yang sama | Pengguna | Dibaca/dimodifikasi agen |
| API key penyedia LLM (di env/konfigurasi CLI agen) | Pengguna | Bocor ke log/DB/jaringan |
| Token Web daemon | Daemon | Dicuri → kontrol penuh |
| Anggaran/kuota API | Pengguna | Dibakar oleh loop |
| Integritas repo utama (`.git`) | Pengguna | Dirusak lewat worktree |

**Batas kepercayaan:** (1) Pengguna ↔ daemon (tepercaya); (2) daemon ↔ proses agen (**tidak tepercaya**); (3) agen ↔ internet/LLM (tidak tepercaya; dapat disusupi prompt injection dari konten repo/web); (4) klien remote ↔ daemon (diautentikasi token + tunnel).

## 3. Model Ancaman (STRIDE ringkas)

| # | Ancaman | Vektor | Mitigasi | Residual |
| --- | --- | --- | --- | --- |
| T1 | Agen membaca `~/.ssh`, `~/.aws` | Prompt injection / agen nakal | Landlock default-deny + denylist (FR-14/15) | Tidak berlaku di macOS/Windows (Noop) |
| T2 | Agen menulis/menghapus di luar worktree | Perintah destruktif | Landlock RW hanya worktree + tmp | Akses `.git` utama (lihat §4.3) |
| T3 | Agen mengirim data ke domain asing | Exfiltrasi | Egress via proxy + allowlist domain; approval gate `net_egress` | Landlock sendiri tidak memfilter domain (lihat §6) |
| T4 | Token Web dicuri | Malware lokal, log, riwayat browser | Token acak, `0600`, tidak di URL, rotasi, bind loopback | Malware dengan akses pengguna sama dapat membaca berkas token |
| T5 | Akses tak sah ke WS dari browser lain (CSRF/DNS rebinding) | Halaman web jahat memanggil `127.0.0.1` | Bearer token wajib, cek `Origin`/`Host`, tanpa cookie auth | - |
| T6 | Kebocoran API key ke SQLite/log | Proxy mencatat header | Tidak pernah menyimpan header sensitif; tes pemindaian otomatis (NFR-05) | Rahasia di dalam *body* prompt (lihat §8) |
| T7 | Proxy disalahgunakan proses lain di mesin | Port lokal terbuka | Token sesi di path proxy; bind loopback/UDS; token kedaluwarsa saat sesi berakhir | Proses sesama pengguna yang mengetahui token sesi |
| T8 | Banjir output/DoS memori | Agen mencetak log masif | Ring buffer, kanal berbatas, batching (NFR-03) | - |
| T9 | Workflow/profil berbahaya dari repo | `workflow.toml` di repo menjalankan `shell` | Workflow dari repo = tak tepercaya, konfirmasi pengguna, hanya boleh memperketat sandbox | Pengguna menyetujui tanpa membaca |
| T10 | Eskalasi lewat symlink/hardlink/`..` | Jalur worktree menunjuk keluar | Landlock beroperasi pada inode; validasi path kanonik; larang symlink keluar worktree saat membuat | Perlu tes eksplisit |
| T11 | Merge hasil agen membawa kode berbahaya | Agen menulis backdoor | Review diff wajib sebelum merge; tidak ada auto-merge tanpa kebijakan eksplisit | Bergantung pada review manusia |
| T12 | Race kondisi saat *trip* | Agen memproses saat dihentikan | Hentikan process group, tunggu exit, `SIGKILL` setelah grace | Efek samping yang sudah terjadi tidak otomatis di-rollback |

## 4. Sandbox Filesystem (Landlock)

### 4.1 Prinsip

- Landlock berbasis **allowlist**: apa pun yang tidak diberi izin eksplisit ditolak. Karena itu `~/.ssh` diblokir hanya dengan **tidak memberikan** akses ke sana atau induknya.
- **Tidak bisa mengecualikan sub-path dari induk yang sudah diizinkan.** Jangan pernah memberi akses baca ke `$HOME` secara keseluruhan; beri akses per-direktori spesifik (mis. `~/.cargo/registry`, `~/.rustup`, `~/.npm`, `~/.cache/pip`).
- Aturan diterapkan di proses anak sebelum `exec` (`pre_exec`), bersama `prctl(PR_SET_NO_NEW_PRIVS)`. Semua anak turunan mewarisi kebijakan dan tidak dapat melonggarkannya.
- Deteksi ABI saat runtime memakai pendekatan *best-effort* (`landlock` crate `CompatLevel::BestEffort`), dengan opsi `require_sandbox = true` untuk menolak menjalankan sesi bila Landlock tidak tersedia penuh.

### 4.2 Kebijakan bawaan

| Jalur | Akses | Alasan |
| --- | --- | --- |
| `<worktree>` | RW (+ eksekusi) | Ruang kerja agen |
| `<tmp_sesi>` (mis. `<state>/tmp/<session>`; diatur `TMPDIR`) | RW | `/tmp` terisolasi, bukan `/tmp` global |
| `/usr`, `/lib`, `/lib64`, `/bin`, `/sbin`, `/opt` | RO + eksekusi | Toolchain sistem |
| `/etc/ssl`, `/etc/ca-certificates`, `/etc/resolv.conf`, `/etc/hosts`, `/etc/nsswitch.conf`, `/etc/passwd`, `/etc/group`, `/etc/localtime`, `/etc/ld.so.cache` | RO | TLS/DNS/resolusi |
| `/proc` (terbatas), `/dev/null`, `/dev/zero`, `/dev/urandom`, `/dev/pts`, `/dev/tty` | RO/RW minimal | Runtime & PTY |
| `~/.cargo`, `~/.rustup` | RO (+ RW untuk `~/.cargo/registry` & cache bila `cache_rw=true`) | Rust toolchain |
| `~/.npm`, `~/.nvm`, `~/.local/share/pnpm`, `~/.cache/pip`, `~/.pyenv` | RO / RW terbatas | Node/Python |
| Direktori konfigurasi CLI agen (mis. `~/.claude`, `~/.codex`) | Sesuai profil (`extra_rw`/`extra_ro`) | CLI agen perlu config/auth miliknya |
| Direktori `.git` repo utama (bagian tertentu) | RW terbatas | Lihat §4.3 |

Denylist bawaan (diverifikasi oleh tes dan dihindari oleh aturan allow): `~/.ssh`, `~/.aws`, `~/.gnupg`, `~/.config/gcloud`, `~/.azure`, `~/.kube`, `~/.docker`, `~/.netrc`, `~/.git-credentials`, keyring (`~/.local/share/keyrings`), manajer kata sandi, profil browser, `.env*` di luar worktree.

> Peringatan: direktori config CLI agen (mis. `~/.claude`) sering menyimpan **token OAuth**. Izin ke direktori itu tak terhindarkan untuk agen tersebut, tetapi berarti agen dapat membacanya; ini bagian dari kepercayaan terhadap CLI tersebut, bukan terhadap kode yang dijalankan di dalamnya. Beri akses hanya pada profil yang membutuhkannya.

### 4.3 Worktree dan `.git` utama (keputusan terbuka)

`git worktree` menyimpan metadata di `<repo>/.git/worktrees/<name>` dan objek/ref di `<repo>/.git`. Agar agen dapat `git add/commit` dari worktree, proses perlu **akses tulis** ke bagian `.git` repo utama. Ini melemahkan isolasi: agen bisa memodifikasi ref branch lain atau menanam hook.

Opsi (putuskan lewat ADR sebelum v0.3):

| Opsi | Deskripsi | Kelebihan | Kekurangan |
| --- | --- | --- | --- |
| A | RW ke `.git/worktrees/<id>`, `.git/objects`, `.git/refs/heads/agent/<...>` saja | Isolasi cukup baik | Aturan Landlock lebih rumit; `objects` tetap tulis-bebas |
| B | Agen tidak boleh commit; daemon yang commit dari luar sandbox | Isolasi kuat | Mengubah perilaku CLI agen yang suka commit |
| C | Worktree memakai *clone* terpisah dengan `objects` alternates (RO ke repo utama) | Isolasi sangat kuat | Lebih berat; berbeda dari semantik `worktree` |
| D | RW penuh ke `.git` | Sederhana | Isolasi lemah |

Bawaan sementara: **A**, plus `core.hooksPath` diarahkan ke direktori kosong dan `GIT_CONFIG_*` untuk menonaktifkan hook, serta pemeriksaan integritas ref sebelum merge.

### 4.4 Cakupan Landlock dan keterbatasannya

- Landlock ABI v1 (kernel 5.13) hanya membatasi akses **filesystem**. Pembatasan jaringan TCP (bind/connect per port) baru tersedia mulai ABI v4 (kernel 6.7). Tidak ada pembatasan **domain/IP** di Landlock.
- Landlock tidak mencegat `ptrace`, sinyal ke proses lain, atau akses socket UNIX abstrak pada ABI lama; ABI lebih baru (v6, kernel 6.12) menambah scoping untuk socket abstrak dan sinyal. Kombinasikan dengan `PR_SET_NO_NEW_PRIVS`, `PR_SET_DUMPABLE=0`, dan (opsional) filter `seccomp` dasar bila diperlukan.
- Landlock **tidak** memberi notifikasi penolakan ke induk; audit penolakan bergantung pada galat `EACCES` yang terlihat di output proses atau mode uji.
- Akses ke socket UNIX di filesystem (mis. `docker.sock`, agen SSH `$SSH_AUTH_SOCK`) dikendalikan oleh hak filesystem; pastikan tidak diberikan dan **bersihkan env** (`SSH_AUTH_SOCK`, `DBUS_SESSION_BUS_ADDRESS`, dsb.) pada proses agen.

## 5. Lingkungan Proses Agen

- Env **bersih secara bawaan** (allowlist): `PATH` (dengan shim bila ada), `HOME` (disarankan HOME virtual), `TERM`, `LANG`, `TMPDIR`, variabel dari profil, dan `*_BASE_URL` proxy.
- Variabel sensitif dari shell pengguna (`AWS_*`, `GITHUB_TOKEN`, `SSH_AUTH_SOCK`, dst.) **tidak** diwariskan kecuali dicantumkan eksplisit di profil (`pass_env = [...]`).
- API key penyedia: diteruskan hanya jika profil menetapkannya (`pass_env`), dan tidak pernah dicatat.
- Pertimbangkan `HOME` terpisah per sesi dengan salinan/tautan config minimal agar tidak membuka `$HOME` asli.

## 6. Jaringan (`allow_network`)

Pembatasan domain **tidak** dapat ditegakkan oleh Landlock. Strategi berlapis:

1. **Proxy sebagai jalur wajib untuk LLM:** lalu lintas API LLM lewat proxy lokal; proxy hanya meneruskan ke host yang ada di allowlist.
2. **HTTP(S) proxy untuk lalu lintas lain:** suntik `HTTP_PROXY`/`HTTPS_PROXY`/`ALL_PROXY` ke proxy egress lokal (dengan CONNECT filter domain). Ini **kooperatif**: proses yang mengabaikan env proxy dapat menghindar.
3. **Penegakan kernel (opsional, v0.3+):** Landlock ABI ≥ 4 untuk membatasi `connect` TCP hanya ke port proxy lokal; atau network namespace + nftables bila tersedia (membutuhkan privilese; ditangguhkan).
4. **Approval gate `net_egress`** untuk domain di luar allowlist (bila tercegat).

Dokumentasikan ke pengguna bahwa pada v1 pembatasan jaringan bersifat *best-effort* kecuali opsi kernel diaktifkan.

## 7. Approval Gate (FR-17)

Tantangan: aksi (perintah shell, `git push`, instal paket) dijalankan **di dalam CLI agen**, sehingga daemon tidak otomatis melihatnya.

Pendekatan yang dapat digabungkan:

| Jalur | Cara kerja | Cakupan | Catatan |
| --- | --- | --- | --- |
| PATH shim | Direktori shim berisi `git`, `npm`, `pnpm`, `pip`, `cargo`, `curl`, `rm`... yang memanggil `latch approvals request` lalu `exec` biner asli bila disetujui | Perintah yang dipanggil lewat PATH | Dapat dilewati dengan path absolut (`/usr/bin/git`); dimitigasi karena Landlock dapat melarang eksekusi biner sistem tertentu di luar shim (mahal) atau melalui pengamatan |
| Hook bawaan agen | Konfigurasi hook izin pada CLI yang mendukungnya diarahkan ke daemon | Akurat untuk agen tersebut | Spesifik per agen; dikelola di profil |
| Inspeksi tool-call di proxy | Mendeteksi tool-call berisiko di respons LLM | Deteksi dan peringatan | Tidak bisa memblokir eksekusi yang sudah diputuskan CLI |
| Penegakan sandbox | Aksi yang melanggar Landlock langsung gagal | Penolakan keras | Tidak interaktif |

Keputusan arsitektur disusun dalam ADR terpisah sebelum v0.3. Sampai itu, dokumentasi harus jelas menyatakan cakupan: *approval gate melindungi aksi yang tercegat oleh mekanisme di atas*, bukan semua aksi.

Aturan keputusan:
- Default **deny** saat timeout (bawaan 5 menit; dapat diubah).
- Setiap keputusan dicatat di `approval_requests` dan `audit_log`.
- Tampilkan ringkasan yang **disaring** (tanpa rahasia) dan konteks (sesi, worktree, perintah).
- Approve dari HP memerlukan token Web valid; opsi "konfirmasi ulang" untuk aksi paling berisiko.

## 8. Rahasia, Log, dan Telemetri (NFR-05)

- Proxy **tidak** menyimpan header `Authorization`, `x-api-key`, `api-key`, `cookie`, atau query yang berisi kunci. Daftar header sensitif diterapkan sebagai *denylist* di satu fungsi pusat dan dites.
- Isi prompt/respons **tidak** disimpan secara bawaan; hanya metrik (token, biaya, model, status, latensi) dan *fingerprint* hash untuk deteksi loop.
- `tracing` menggunakan *redaction layer*: pola kunci umum (`sk-...`, `AKIA...`, `ghp_...`, `Bearer ...`) disamarkan.
- Log PTY (output agen) dapat memuat rahasia yang dicetak agen; dokumentasikan, simpan dengan izin `0600`, dan sediakan opsi `log_pty = false`.
- **Tes otomatis** (wajib CI): jalankan sesi dengan kunci uji `sk-test-CANARY...`, lalu pindai SQLite, log daemon, dan log proxy: tidak boleh ditemukan.

## 9. Autentikasi & Akses Jarak Jauh

- Satu-satunya cara remote yang didukung v1: **SSH port forwarding** atau **Tailscale/WireGuard**. Tidak ada server relay.
- Bind bawaan `127.0.0.1`. Untuk Tailscale, bind ke antarmuka `tailscale0` atau gunakan `tailscale serve` yang meneruskan ke loopback dan menyediakan HTTPS.
- **Jangan** mengekspos port daemon ke `0.0.0.0` / internet publik. Opsi bind non-loopback membutuhkan flag eksplisit `--i-understand-the-risk` dan ADR.
- Token: lihat API §2. Rotasi memutus semua koneksi. Pertimbangkan token terpisah `read-only` vs `control`.
- UDS: izin `0600`, direktori `0700`, verifikasi `SO_PEERCRED`.
- Web Push memerlukan HTTPS: gunakan sertifikat Tailscale; jangan melemahkan CSP demi push.

## 10. Keamanan Pipeline & Workflow dari Repo

- `workflow.toml` dari dalam repo yang tidak tepercaya dapat memuat `shell` berbahaya. Perlakukan sebagai kode tak tepercaya: tampilkan ringkasan dan minta konfirmasi sebelum run pertama (hash disimpan setelah disetujui).
- Node `shell` berjalan dalam sandbox yang sama dan tidak boleh melonggarkannya.
- Keluaran agen dimasukkan ke template sebagai data; jangan diinterpolasi ke `command` shell (cegah injeksi perintah).

## 11. Rantai Pasok & Build

- `cargo deny` (lisensi, advisories, sumber) dan `cargo audit` di CI.
- Kunci versi (`Cargo.lock` di-commit), `npm ci` dengan `package-lock.json`.
- Aset web ditanam saat build; tidak ada CDN eksternal saat runtime (CSP `default-src 'self'`).
- Build rilis dapat direproduksi sejauh memungkinkan; sertakan checksum SHA-256 dan (opsional) tanda tangan `minisign`/`cosign` pada rilis.

## 12. Audit

Peristiwa yang dicatat di `audit_log`: percobaan auth gagal, keputusan approval, *trip*, merge/discard, perubahan konfigurasi sandbox, sesi dijalankan tanpa sandbox (Noop), serta penolakan sandbox yang terdeteksi. Penolakan Landlock tidak dilaporkan kernel secara langsung: tes keamanan memverifikasi penolakan, sedangkan log runtime hanya menangkap indikasi tidak langsung (mis. `EACCES` pada output).

## 13. Pengujian Keamanan (Wajib)

| Tes | Hasil yang diharapkan |
| --- | --- |
| Dalam sandbox: `cat ~/.ssh/id_rsa` | `EACCES`/`Permission denied` |
| Dalam sandbox: `touch ../outside` (induk worktree) | Ditolak |
| Dalam sandbox: baca `.env` proyek lain | Ditolak |
| Dalam sandbox: symlink dari worktree ke `~/.ssh` lalu baca | Ditolak |
| Tulis ke `/tmp` global | Ditolak; `TMPDIR` terisolasi berfungsi |
| `cargo build`, `npm test`, `python -m pytest`, `git commit` di worktree | Berhasil dengan preset baca |
| Kanari kunci API di header | Tidak ada di DB/log |
| Akses WS tanpa/ dengan token salah | 401, tercatat |
| Request dengan `Origin` asing | Ditolak |
| *Trip* saat anggaran terlampaui | Proses berhenti < 1 s |
| Daemon tetap hidup saat sesi di-`kill -9` | Ya (NFR-06) |

## 14. Pelaporan Kerentanan

Kirim laporan privat ke kontak keamanan proyek (isi alamat/URL *security advisory* sebelum rilis publik: `TODO`). Jangan membuka issue publik untuk celah keamanan. Target respons awal: 72 jam.

## 15. Batasan yang Diakui

- Sandbox hanya Linux pada v1; macOS/Windows berjalan tanpa isolasi (dengan peringatan).
- Pembatasan jaringan per domain bersifat kooperatif kecuali penegakan kernel diaktifkan.
- Approval gate tidak mencakup semua aksi di dalam CLI agen.
- Agen yang diberi akses ke direktori konfigurasi/otentikasinya dapat membaca token OAuth miliknya sendiri.
- Prompt injection tidak dapat dicegah sepenuhnya; sandbox membatasi **dampaknya**, bukan kemungkinannya.
