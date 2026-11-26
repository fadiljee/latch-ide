# Kontrak API Klien-Daemon

Status: **Draft**. Versi API: `v1`. Perubahan breaking selama `0.x` dicatat di `CHANGELOG.md`.

## 1. Transport

| Transport | Pengguna | Alamat | Autentikasi |
| --- | --- | --- | --- |
| Unix Domain Socket | CLI, TUI lokal | `$XDG_RUNTIME_DIR/latch.sock` atau `~/.local/state/latch/daemon.sock` | Izin berkas (`0600`, pemilik = pengguna); verifikasi `SO_PEERCRED` UID |
| HTTP REST + WebSocket | Web PWA, klien remote (via tunnel) | `127.0.0.1:<port>` (bawaan `7878`, dapat diubah) | `Authorization: Bearer <token>` |

Daemon **tidak** membuka listener non-loopback secara bawaan. Akses jarak jauh lewat SSH port forwarding atau Tailscale (lihat SECURITY §Remote).

## 2. Autentikasi (Web)

- Token acak ≥ 256 bit (base64url) dibuat saat inisialisasi pertama; disimpan di `~/.local/state/latch/token` (mode `0600`). `latch token show` menampilkan, `latch token rotate` mengganti (memutus WebSocket yang ada).
- REST: header `Authorization: Bearer <token>`.
- WebSocket: **jangan** menaruh token di query string (bocor ke log/riwayat). Gunakan subprotokol `Sec-WebSocket-Protocol: agentd.v1, bearer.<token>` atau pesan pertama `{"type":"auth","token":"..."}` dalam 5 detik; selain itu koneksi ditutup.
- Perbandingan token memakai *constant-time compare*. Kegagalan beruntun dibatasi laju (mis. 5/menit per IP) dan dicatat di `audit_log` (`auth_failure`).
- Pemeriksaan header `Origin`/`Host` untuk menahan DNS rebinding dan akses lintas-origin: hanya `Origin` yang sama dengan host daemon (atau daftar eksplisit) yang diterima.
- Header keamanan untuk aset PWA: `Content-Security-Policy` ketat, `X-Content-Type-Options: nosniff`, `Referrer-Policy: no-referrer`.

## 3. Protokol UDS (CLI/TUI)

Bingkai berprefix panjang (4 byte big-endian) berisi JSON-RPC 2.0.

```json
// request
{"jsonrpc":"2.0","id":1,"method":"session.list","params":{"project":"my-repo"}}
// response
{"jsonrpc":"2.0","id":1,"result":[{"id":"...","status":"running"}]}
// notifikasi dari server (tanpa id)
{"jsonrpc":"2.0","method":"event","params":{"type":"session.status","session_id":"...","status":"tripped"}}
```

Metode mencerminkan endpoint REST (tabel §4). Untuk aliran PTY pada UDS, klien memanggil `session.attach` lalu menerima frame biner terpisah (`type: 0x01` output, `0x02` snapshot, `0x03` dropped marker) pada koneksi yang sama.

## 4. REST API

Basis: `/api/v1`. Respons `application/json`. Waktu RFC 3339 UTC.

### 4.1 Sistem

| Metode | Path | Deskripsi |
| --- | --- | --- |
| GET | `/health` | `{ "status":"ok", "version":"...", "sandbox":"landlock" }` (tanpa auth boleh, tanpa data sensitif) |
| GET | `/metrics` | RSS, jumlah sesi, panjang kanal, versi |
| GET | `/config` | Config efektif yang disaring (tanpa rahasia) |

### 4.2 Proyek

| Metode | Path | Deskripsi |
| --- | --- | --- |
| GET | `/projects` | Daftar proyek |
| POST | `/projects` | `{ "name","repo_path","default_branch?" }` |
| DELETE | `/projects/{id}` | Hapus (gagal bila ada sesi aktif) |

### 4.3 Sesi

| Metode | Path | Deskripsi |
| --- | --- | --- |
| GET | `/sessions?project=&status=` | Daftar sesi |
| POST | `/sessions` | Buat dan jalankan: `{ "project","profile","task","base_ref?","budget_usd?" }` |
| GET | `/sessions/{id}` | Detail sesi + agregat biaya/token |
| POST | `/sessions/{id}/pause` | `SIGSTOP` |
| POST | `/sessions/{id}/resume` | `SIGCONT` / keluar dari hibernasi |
| POST | `/sessions/{id}/kill` | Hentikan sesi |
| POST | `/sessions/{id}/input` | Kirim input ke PTY: `{ "data_base64" }` (dibatasi untuk klien tepercaya) |
| POST | `/sessions/{id}/resize` | `{ "cols","rows" }` |
| GET | `/sessions/{id}/diff` | Daftar file + patch ringkas; `?file=` untuk satu file; `?format=unified` |
| GET | `/sessions/{id}/merge-check` | Hasil simulasi `merge-tree`: `{ "clean":bool, "conflicts":[...] }` |
| POST | `/sessions/{id}/merge` | `{ "strategy":"squash"|"merge", "message?" }` - ditolak bila konflik |
| POST | `/sessions/{id}/discard` | Buang worktree + branch |
| GET | `/sessions/{id}/telemetry?since=` | Telemetri proxy |
| GET | `/sessions/{id}/screen` | Snapshot layar teks (untuk HP) |

### 4.4 Approval

| Metode | Path | Deskripsi |
| --- | --- | --- |
| GET | `/approvals?status=pending` | Daftar permintaan |
| POST | `/approvals/{id}/decision` | `{ "decision":"approved"|"denied" }` |

### 4.5 Pipeline

| Metode | Path | Deskripsi |
| --- | --- | --- |
| GET | `/pipelines` | Daftar run |
| POST | `/pipelines` | `{ "project","workflow_toml" | "workflow_path","task" }` |
| GET | `/pipelines/{id}` | Status DAG, biaya, artefak |
| POST | `/pipelines/{id}/cancel` | Batalkan |
| GET | `/pipelines/{id}/arena` | Matriks perbandingan fan-out |
| POST | `/pipelines/{id}/arena/select` | `{ "step_id","session_id" }` pilih pemenang untuk merge |

### 4.6 Format Galat

```json
{
  "error": {
    "code": "session_not_found",
    "message": "Sesi tidak ditemukan",
    "details": {}
  }
}
```

| HTTP | Kode contoh |
| --- | --- |
| 400 | `invalid_request`, `invalid_workflow` |
| 401 | `unauthorized` |
| 403 | `forbidden_origin` |
| 404 | `session_not_found`, `project_not_found` |
| 409 | `invalid_state_transition`, `merge_conflict` |
| 422 | `profile_invalid` |
| 429 | `rate_limited` |
| 500 | `internal` |

## 5. WebSocket

Endpoint: `GET /api/v1/ws` (event umum) dan `GET /api/v1/sessions/{id}/pty` (aliran PTY).

### 5.1 Event umum (`/ws`)

Pesan JSON, satu objek per frame:

```json
{"type":"hello","version":"0.2.0","server_time":"2026-01-01T00:00:00Z"}
{"type":"session.created","session":{...}}
{"type":"session.status","session_id":"...","from":"running","to":"tripped","reason":"error_loop"}
{"type":"session.cost","session_id":"...","cost_usd":0.42,"prompt_tokens":12000,"completion_tokens":3400}
{"type":"approval.requested","approval":{"id":"...","session_id":"...","action_type":"shell_exec","payload_summary":"git push origin agent/x"}}
{"type":"approval.decided","id":"...","decision":"approved","decided_by":"web:iphone"}
{"type":"pipeline.step","pipeline_id":"...","step_id":"verify","status":"failed","retry":1}
{"type":"notification","level":"info","title":"Sesi selesai","session_id":"..."}
```

Klien dapat berlangganan selektif: `{"type":"subscribe","topics":["sessions","approvals","pipelines"]}`.

### 5.2 Aliran PTY (`/sessions/{id}/pty`)

- Server → klien: frame **biner** berisi byte terminal yang sudah di-*batch* tiap 8-16 ms. Frame pertama adalah snapshot layar (`vt100`) agar klien baru langsung sinkron.
- Klien → server: frame teks JSON `{"type":"input","data":"..."}`, `{"type":"resize","cols":120,"rows":40}`.
- Jika klien lambat, server membuang frame lama dan mengirim `{"type":"dropped","bytes":N}`; klien dapat meminta `{"type":"resync"}` untuk snapshot baru.
- Mode baca-saja untuk token/klien tertentu (usulan: token terpisah `read-only` untuk berbagi tampilan).

### 5.3 Keep-alive & Reconnect

- Ping/pong tiap 20 s; tutup koneksi bila tidak ada pong 60 s.
- Klien mengimplementasikan *auto-reconnect* dengan backoff eksponensial dan `resync`.
- Event membawa `seq` monoton; klien dapat meminta `since_seq` untuk menyusul event terlewat (buffer pendek di memori).

## 6. Notifikasi (FR-23)

| Kanal | Mekanisme |
| --- | --- |
| Desktop | Notifikasi OS (`notify-rust` atau `notify-send`) dari proses daemon/TUI |
| Web push | Web Push (VAPID) via service worker PWA; butuh HTTPS (Tailscale Serve/`tailscale cert`) |
| Webhook | `POST` JSON ke URL di config; tanda tangan HMAC `X-Latch-Signature` |

Contoh payload webhook:

```json
{
  "event": "session.tripped",
  "session_id": "...",
  "project": "my-repo",
  "reason": "budget",
  "cost_usd": 2.03,
  "timestamp": "2026-01-01T00:00:00Z"
}
```

## 7. Idempotensi & Versioning

- `POST` pembuatan menerima header `Idempotency-Key` opsional.
- Penambahan field bersifat kompatibel; penghapusan/penggantian nama field menaikkan `/api/v2`.
- `GET /health` mengembalikan `api_versions: ["v1"]`.

## 8. Batas & Keamanan Tambahan

- Ukuran body maksimum 1 MiB (kecuali upload workflow 256 KiB).
- Batas koneksi WebSocket per token (mis. 8).
- Respons diff dibatasi ukuran dan dipaginasi (`?offset=&limit=`) untuk melindungi RAM.
- Tidak ada endpoint yang mengembalikan nilai env var sesi atau header `Authorization`.
