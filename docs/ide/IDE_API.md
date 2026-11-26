# API Agent IDE (ekstensi)

Melengkapi [API.md](../API.md). Semua rute di bawah `/api/v1`, memerlukan `Authorization: Bearer`. Jalur `{sid}` = id sesi (worktree). Konvensi galat, pagination, dan idempotensi mengikuti API.md.

## 1. Token & Scope

Token Web kini membuka kemampuan tulis. Perkenalkan **scope** (satu token dapat memiliki beberapa):

| Scope | Izin |
| --- | --- |
| `read` | GET sesi, diff, file, search, event, layar terminal (baca) |
| `control` | pause/resume/kill, approve/deny, kirim pesan, buat sesi, merge/discard |
| `write` | Tulis/ubah/hapus file di worktree, aksi hunk, operasi git lokal |
| `shell-sandbox` | Terminal shell tersandbox |
| `shell-host` | Terminal tanpa sandbox (nonaktif default; aktivasi eksplisit di config) |
| `admin` | Rotasi token, ubah konfigurasi, kelola proyek |

Perintah: `latch token create --scopes read,control [--label hp] [--expires 7d]`, `latch token ls`, `latch token revoke <id>`. Token disimpan sebagai hash di sisi daemon; token mentah hanya ditampilkan sekali. Token bawaan `latch token show` = `read,control,write,shell-sandbox,admin` untuk pemilik lokal. Token untuk HP disarankan `read,control` (tanpa `write`/shell).

## 2. Workspace & Proyek

| Metode | Path | Deskripsi |
| --- | --- | --- |
| GET | `/workspaces` | Proyek + ringkasan sesi aktif |
| GET | `/prefs` | Preferensi pengguna (layout, tema, keybindings) |
| PUT | `/prefs` | Simpan preferensi |

## 3. File Service

| Metode | Path | Deskripsi | Scope |
| --- | --- | --- | --- |
| GET | `/sessions/{sid}/fs/tree?path=&depth=1` | Daftar direktori (dipaginasi), flag git/agen | read |
| GET | `/sessions/{sid}/fs/file?path=` | Isi + `etag`, `encoding`, `size`, `is_binary` | read |
| PUT | `/sessions/{sid}/fs/file?path=` | Tulis; header `If-Match: <etag>`; body teks (atau `content_base64`) | write |
| POST | `/sessions/{sid}/fs/ops` | `{ "op":"create|mkdir|rename|delete", "path","to?" }` | write |
| GET | `/sessions/{sid}/fs/stat?path=` | Metadata ringan | read |
| GET | `/sessions/{sid}/fs/raw?path=` | Unduh biner (dibatasi, `Content-Disposition`, `X-Content-Type-Options: nosniff`) | read |

Respons `PUT` sukses: `{ "etag":"...", "mtime":"..." }`. Konflik: `409 { "error":{"code":"etag_mismatch","details":{"current_etag":"...","current_content":"..."}}}`.

Aturan: jalur relatif terhadap akar worktree; `..`, jalur absolut, dan symlink keluar akar ditolak (`400 path_outside_root`); tulis ke `.git/` ditolak (`403 git_dir_protected`).

## 4. Pencarian

| Metode | Path | Deskripsi |
| --- | --- | --- |
| GET | `/sessions/{sid}/search/files?q=&limit=50` | Fuzzy nama file |
| POST | `/sessions/{sid}/search/text` | `{ "query","regex?","case?","globs?","max_results?" }` → mengembalikan `search_id`; hasil mengalir lewat WS `search.result` |
| DELETE | `/search/{search_id}` | Batalkan |

Batas: `max_results` bawaan 2000, waktu maks 10 s, ukuran berkas maks 2 MiB per berkas yang dipindai.

## 5. Git (lokal pada worktree)

| Metode | Path | Deskripsi | Scope |
| --- | --- | --- | --- |
| GET | `/sessions/{sid}/git/status` | File berubah, staged/unstaged, ahead/behind base | read |
| POST | `/sessions/{sid}/git/stage` | `{ "paths":[...], "hunks?":[...] }` | write |
| POST | `/sessions/{sid}/git/unstage` | idem | write |
| POST | `/sessions/{sid}/git/commit` | `{ "message","paths?" }` (hook dinonaktifkan) | write |
| GET | `/sessions/{sid}/git/log?path=&limit=` | Riwayat ringkas | read |
| GET | `/sessions/{sid}/git/blame?path=` | Blame ringan | read |
| POST | `/sessions/{sid}/git/sync-base` | Rebase/merge base terbaru; hasil `clean|conflicts[]` | write |
| Larang | push/pull/remote | Tidak ada endpoint; `git push` hanya melalui approval gate/terminal | - |

## 6. Review (hunk)

| Metode | Path | Deskripsi | Scope |
| --- | --- | --- | --- |
| GET | `/sessions/{sid}/review` | Daftar file + hunk dengan `hunk_id` stabil, penanda berisiko, statistik | read |
| POST | `/sessions/{sid}/review/hunks/{hunk_id}/accept` | Tandai diterima (stage) | write |
| POST | `/sessions/{sid}/review/hunks/{hunk_id}/reject` | Revert hunk; mengembalikan `undo_token` | write |
| POST | `/sessions/{sid}/review/undo` | `{ "undo_token" }` | write |
| POST | `/sessions/{sid}/review/comments` | Tambah komentar `{ "path","line","text" }` | write |
| GET | `/sessions/{sid}/review/comments` | Daftar komentar tertunda | read |
| POST | `/sessions/{sid}/review/send` | Kirim komentar terkumpul ke agen (`review_feedback`) | control |

`hunk_id` = hash (path + konteks hunk); dapat berubah setelah perubahan lain; respons aksi menyertakan daftar hunk terbaru. Menolak hunk pada sesi `running` mengembalikan `409 session_running` kecuali parameter `pause_agent=true`.

## 7. Pesan ke Agen

| Metode | Path | Deskripsi | Scope |
| --- | --- | --- | --- |
| POST | `/sessions/{sid}/messages` | Pesan terstruktur (lihat AGENT_PROTOCOL §4) | control |
| GET | `/sessions/{sid}/activity?since=` | Aliran aktivitas/tool call (mode terstruktur) atau ringkasan (PTY) | read |
| POST | `/sessions/{sid}/interrupt` | Interupsi (Ctrl+C / `interrupt` protokol) | control |

## 8. Terminal Tambahan

| Metode | Path | Deskripsi | Scope |
| --- | --- | --- | --- |
| POST | `/sessions/{sid}/terminals` | `{ "kind":"sandbox|host","cols","rows" }` → `terminal_id` | shell-sandbox / shell-host |
| WS | `/terminals/{terminal_id}` | Aliran biner (sama dengan PTY sesi) | sesuai |
| DELETE | `/terminals/{terminal_id}` | Tutup | sesuai |

## 9. LSP

| Metode | Path | Deskripsi | Scope |
| --- | --- | --- | --- |
| GET | `/lsp/servers` | Server terdaftar dan status | read |
| WS | `/sessions/{sid}/lsp/{language}` | Jembatan JSON-RPC LSP (frame teks) | write |

Daemon memfilter metode berbahaya (mis. `workspace/executeCommand` kecuali diizinkan), membatasi ukuran pesan, dan menutup server saat idle.

## 10. Event Tambahan (WS `/ws`)

```json
{"type":"fs.changed","session_id":"...","paths":["src/page.rs"],"by":"agent|user|unknown"}
{"type":"session.diff_updated","session_id":"...","files":5,"additions":120,"deletions":34}
{"type":"session.activity","session_id":"...","activity":"working|waiting_input|idle|blocked"}
{"type":"agent.message.delta","session_id":"...","text":"..."}
{"type":"agent.tool.started","session_id":"...","tool":"edit_file","summary":"src/page.rs"}
{"type":"agent.tool.finished","session_id":"...","tool":"edit_file","ok":true}
{"type":"search.result","search_id":"...","path":"src/x.rs","line":12,"text":"...","done":false}
{"type":"review.comment_added","session_id":"...","comment":{...}}
{"type":"prefs.changed"}
```

Klien berlangganan per topik: `sessions`, `fs:<sid>`, `search:<id>`, `agent:<sid>`, `approvals`, `pipelines`.

## 11. Batas, Keamanan, dan Kinerja

- `Content-Security-Policy` workbench: `default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'(dibatasi, preferensi nonce); img-src 'self' data: blob:; connect-src 'self' ws: wss:; frame-ancestors 'none'; object-src 'none'`.
- Semua respons berkas teks dikirim sebagai JSON, bukan HTML; render di UI melewati sanitizer.
- Batas ukuran body: PUT file 5 MiB; pesan agen 64 KiB; komentar 4 KiB.
- Batas laju per token untuk endpoint mahal (search teks, tree rekursif).
- Semua operasi tulis dicatat di `audit_log` (`kind = "fs_write"`, jalur dan ukuran; **tanpa isi**).
- Pagination: `?cursor=&limit=` (maks 500) untuk daftar besar.

## 12. Kontrak Tipe

Tipe request/respons didefinisikan di `core` (Rust) dan dibangkitkan ke TypeScript (`specta`/`ts-rs`) atau OpenAPI; CI gagal bila artefak tipe tidak sinkron dengan kode. Rilis breaking → `/api/v2`.
