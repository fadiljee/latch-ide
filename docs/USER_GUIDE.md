# Panduan Pengguna

> Panduan ini menggambarkan perilaku **target** v1.0. Beberapa fitur baru tersedia di rilis tertentu (ditandai v0.x). Perbarui setelah MVP berjalan.

## 1. Persyaratan

- Linux x86_64/aarch64 (kernel ≥ 5.13 untuk sandbox penuh). macOS/Windows: berjalan tanpa sandbox.
- `git` ≥ 2.38.
- Satu atau lebih CLI agen yang sudah terpasang dan terautentikasi (Claude Code, Codex CLI, OpenCode, Gemini CLI, atau skrip kustom).
- Opsional: Tailscale atau SSH untuk akses dari HP/mesin lain.

## 2. Instalasi

```bash
# Unduh binary statis dari halaman rilis, lalu:
chmod +x latch
sudo mv latch /usr/local/bin/latch
sudo ln -s /usr/local/bin/latch /usr/local/bin/latchd   # opsional alias daemon

latch --version
```

Build dari sumber:

```bash
rustup target add x86_64-unknown-linux-musl
(cd web && npm ci && npm run build)
cargo build --release --target x86_64-unknown-linux-musl
```

## 3. Menjalankan Daemon

```bash
latch daemon start            # latar belakang
latch daemon start --foreground
latch daemon status
latch daemon stop
```

Contoh unit `systemd --user` (`~/.config/systemd/user/latchd.service`):

```ini
[Unit]
Description=Latch (`latch-ide`) daemon

[Service]
ExecStart=%h/.local/bin/latch daemon start --foreground
Restart=on-failure

[Install]
WantedBy=default.target
```

```bash
systemctl --user enable --now agentd
loginctl enable-linger "$USER"   # agar tetap hidup tanpa login (VPS)
```

## 4. Konfigurasi

Lokasi: `~/.config/latch/`.

### 4.1 `config.toml`

```toml
[daemon]
http_bind = "127.0.0.1:7878"   # Web/REST/WS; jangan ubah ke 0.0.0.0 tanpa memahami risikonya
worker_threads = 2
state_dir = "~/.local/state/latch"

[limits]
max_sessions = 8
pty_ring_buffer_kib = 1024
pty_log_max_mib = 50

[sandbox]
require_sandbox = false          # true = tolak start sesi bila Landlock tidak tersedia
presets = ["system", "rust", "node", "python", "git"]
extra_ro = []
extra_rw = []

[circuit_breaker]
max_consecutive_errors = 3
max_session_budget_usd = 2.0
idle_hibernate_minutes = 15

[notifications]
desktop = true
webhook_url = ""
webhook_secret_env = "LATCH_WEBHOOK_SECRET"

[retention]
telemetry_days = 30
audit_days = 90
```

### 4.2 Profil agen (`agents.toml`)
<a id="profil-agen"></a>

```toml
[profiles.coder-claude]
command = "claude"
args = []
proxy_mode = "injected"              # "injected" | "bypass"
proxy_provider = "anthropic"         # menentukan env BASE_URL yang disuntik
pass_env = ["ANTHROPIC_API_KEY"]     # hanya variabel yang diteruskan dari shell Anda
extra_rw = ["~/.claude"]             # config/auth milik CLI ini
resume_args = ["--continue"]         # dipakai saat resume dari hibernasi (bila CLI mendukung)

[profiles.coder-codex]
command = "codex"
args = []
proxy_mode = "injected"
proxy_provider = "openai"
pass_env = ["OPENAI_API_KEY"]
extra_rw = ["~/.codex"]

[profiles.oauth-cli]
command = "some-agent"
proxy_mode = "bypass"                # CLI dengan OAuth tertutup / certificate pinning
extra_rw = ["~/.config/some-agent"]

[profiles.sleeper]                   # profil uji (benchmark)
command = "sh"
args = ["-c", "sleep 3600"]
proxy_mode = "bypass"
```

> Nama perintah, argumen, dan variabel env di atas hanya contoh; sesuaikan dengan CLI yang Anda pakai. Jangan menaruh nilai API key di berkas ini: gunakan `pass_env`.

### 4.3 Harga model (`pricing.toml`)

```toml
# USD per 1 juta token. Perbarui sesuai harga penyedia.
[models."contoh-model-a"]
input = 3.00
output = 15.00
[models."contoh-model-b"]
input = 0.50
output = 1.50
```

Model yang tidak ada di tabel dicatat dengan biaya `unknown` dan peringatan.

### 4.4 Memeriksa konfigurasi

```bash
latch config check
```

## 5. Alur Kerja Dasar

```bash
# 1. daftarkan proyek
latch project add ~/code/my-repo --name my-repo --default-branch main

# 2. jalankan agen
latch run --project my-repo --profile coder-claude --task "Perbaiki bug login"

# 3. pantau
latch session ls
latch tui

# 4. tinjau & gabungkan
latch session diff <id>
latch session merge-check <id>
latch session merge <id> --squash
# atau buang
latch session discard <id>
```

Pintasan TUI (default, dapat berubah):

| Tombol | Aksi |
| --- | --- |
| `↑/↓` `j/k` | Pilih sesi |
| `Enter` / `a` | Attach ke PTY sesi |
| `Ctrl-b d` | Detach dari sesi |
| `p` | Pause/Resume |
| `x` | Kill (konfirmasi) |
| `d` | Lihat diff |
| `m` | Merge (konfirmasi) |
| `y` / `n` | Approve / Deny permintaan |
| `?` | Bantuan |
| `q` | Keluar |

## 6. Circuit Breaker

Sesi dihentikan otomatis bila:
- biaya sesi ≥ `max_session_budget_usd`, atau
- error/tool-call identik berulang `max_consecutive_errors` kali, atau
- batas token terlampaui.

Status menjadi `tripped`, Anda menerima notifikasi. Anda dapat meninjau penyebab di `latch session show <id>`, lalu `resume` (menaikkan batas) atau `discard`.

Untuk profil `bypass`, pemantauan token tidak tersedia; deteksi hanya berbasis pola output PTY dan idle.

## 7. Sandbox

Pada Linux dengan Landlock, agen hanya dapat menulis di worktree-nya dan direktori tmp terisolasi, dan tidak dapat membaca `~/.ssh`, `~/.aws`, dst.

Jika toolchain gagal di dalam sandbox (mis. "permission denied" pada cache), tambahkan jalur baca di config:

```toml
[sandbox]
extra_ro = ["~/.cache/mytool"]
```

Cek mode uji coba untuk melihat akses yang akan ditolak:

```bash
latch run --sandbox-dry-run ...
```

Jika `NoopProvider` aktif (macOS/Windows atau kernel lama), Anda akan melihat peringatan "sesi tidak terisolasi".

## 8. Approval

Saat aksi berisiko tercegat, sesi masuk `waiting_approval`:

```bash
latch approvals ls
latch approvals approve <id>
latch approvals deny <id>
```

atau tekan `y`/`n` di TUI, atau tombol Approve/Reject di PWA. Permintaan tanpa keputusan akan **ditolak** setelah timeout. Ingat: cakupan approval gate terbatas pada aksi yang tercegat (lihat SECURITY §7).

## 9. Pipeline

```bash
latch pipeline run workflow.toml --project my-repo --task "Tambahkan paginasi"
latch pipeline ls
latch pipeline show <id>
latch pipeline arena <id>             # matriks perbandingan fan-out
latch pipeline select <id> --step implement --session <session-id>
```

Lihat [WORKFLOW_SPEC.md](WORKFLOW_SPEC.md) untuk format `workflow.toml`.

## 10. Akses dari HP / Mesin Lain

**Aturan utama: jangan membuka port daemon ke internet.** Gunakan salah satu:

### SSH tunnel

```bash
# dari laptop/HP (mis. Termux) ke server tempat daemon berjalan
ssh -N -L 7878:127.0.0.1:7878 user@server
# lalu buka http://127.0.0.1:7878 di browser
```

### Tailscale

```bash
# di server
tailscale serve --bg 7878      # HTTPS di dalam tailnet, meneruskan ke loopback
# di HP: buka URL tailnet yang ditampilkan, lalu "Tambahkan ke layar utama"
```

Ambil token:

```bash
latch token show
latch token rotate      # memutus semua koneksi lama
```

Masukkan token saat pertama membuka PWA. Jangan menempelkan token di URL atau membagikannya.

## 11. Pemecahan Masalah

| Gejala | Kemungkinan penyebab | Solusi |
| --- | --- | --- |
| `Permission denied` saat build di dalam sandbox | Jalur cache toolchain belum diizinkan | Tambah `extra_ro`/`extra_rw` atau preset; gunakan `--sandbox-dry-run` |
| Biaya selalu 0 / tidak ada telemetri | Profil `bypass` atau CLI mengabaikan `*_BASE_URL` | Cek `proxy_mode`; lihat matriks kompatibilitas |
| CLI gagal TLS/OAuth saat proxy aktif | Certificate pinning / OAuth tertutup | Set `proxy_mode = "bypass"` |
| `merge_conflict` | Dua worktree mengubah baris yang sama | Lihat daftar konflik; selesaikan manual atau pakai agen resolver; merge bergantian |
| Sesi `failed` tiba-tiba | Proses keluar/OOM | Lihat `latch session logs <id>` |
| Worktree sisa | Crash sebelum cleanup | `latch gc` (atau otomatis saat startup) |
| PWA tidak bisa menerima push | Tidak ada HTTPS | Gunakan `tailscale serve` (HTTPS) |
| Daemon memakai RAM besar | Terlalu banyak sesi/ring buffer besar | Turunkan `pty_ring_buffer_kib`, `max_sessions` |

## 12. Praktik Terbaik

1. Selalu tinjau diff sebelum merge; jangan aktifkan auto-merge pada repo penting.
2. Atur `max_session_budget_usd` realistis sebelum meninggalkan agen.
3. Pakai `pass_env` seminimal mungkin; jangan menaruh kunci di berkas konfigurasi.
4. Pakai profil terpisah untuk agen yang butuh akses direktori auth.
5. Jangan menjalankan `workflow.toml` dari repo yang tidak Anda percaya tanpa membacanya.
