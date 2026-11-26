# Protokol Integrasi Agen

Workbench mendukung dua jalur ke agen. Tujuannya: PTY tetap **universal** (semua CLI), sementara mode **terstruktur** memberi UI kaya dan menutup celah approval gate pada agen yang mendukungnya.

## 1. Dua Mode

| Aspek | Mode PTY (universal) | Mode Terstruktur (opsional) |
| --- | --- | --- |
| Cara | Proses CLI di pseudo-terminal | Proses agen berbicara protokol JSON-RPC via stdio/socket |
| UI | Terminal xterm.js | Aliran pesan, tool call, diff inline, izin |
| Kirim pesan | Mengetik ke stdin PTY | Pesan pengguna terstruktur |
| Approval | PATH shim / hook / deteksi (SECURITY §7) | **Permintaan izin native dari protokol** |
| Konteks | Teks yang diketik/ditempel | Lampiran terstruktur (berkas, rentang, diff) |
| Dukungan agen | Semua | Hanya agen yang mengimplementasikan protokol |

Satu profil agen menentukan mode lewat `mode = "pty" | "structured"`; agen terstruktur tetap dapat menyediakan terminal untuk output mentah.

## 2. Kandidat Protokol (verifikasi sebelum implementasi)

| Protokol | Catatan |
| --- | --- |
| **Agent Client Protocol (ACP)** | Standar yang diusulkan untuk menghubungkan editor/klien dengan agen coding lewat JSON-RPC stdio. Diperkirakan relevan; **cek spesifikasi dan agen yang mendukung saat ini** sebelum memutuskan (ADR-0016). |
| **MCP (Model Context Protocol)** | Untuk menyediakan *tools/resources* ke agen (mis. alat workbench), bukan pengganti protokol sesi agen. |
| **Protokol kustom `agentd-session`** | Cadangan: skema minimal sendiri bila ACP tidak sesuai. |

Keputusan awal (Proposed): adaptasikan **ACP** sebagai jalur terstruktur utama di crate `agent-proto`, dan opsional menyediakan **server MCP internal** yang mengekspos alat workbench (mis. `workbench.open_file`, `workbench.request_review`) ke agen.

> Jangan menyalin klaim kompatibilitas agen tertentu tanpa pengujian; tabel kompatibilitas dipelihara di `docs/compat/agents.md` (dibuat saat uji).

## 3. Pemetaan Event (mode terstruktur → event daemon)

| Dari agen | Event daemon/WS | UI |
| --- | --- | --- |
| Pesan asisten (stream) | `agent.message.delta` | Aliran teks |
| Tool call dimulai/selesai | `agent.tool.started/finished` | Kartu tool (perintah, berkas, hasil) |
| Permintaan izin | `approval.requested` (action_type sesuai) | Dialog Approve/Deny, push ke HP |
| Edit berkas | `fs.changed` + `agent.edit` | Gutter, daftar "file disentuh agen" |
| Rencana/todo | `agent.plan.updated` | Panel rencana |
| Selesai/kesalahan | `session.status` | Papan sesi |

Telemetri biaya tetap berasal dari proxy bila `proxy_mode=injected`; agen terstruktur juga dapat melaporkan penggunaan (ditandai sumber `agent_reported`).

## 4. Pesan dari Workbench ke Agen

Jenis pesan terstruktur (`POST /sessions/{id}/messages`):

```json
{
  "kind": "user_message | review_feedback | context_attach | interrupt | resume_hint",
  "text": "Perbaiki off-by-one pada paginasi",
  "attachments": [
    {"type":"file","path":"src/page.rs"},
    {"type":"range","path":"src/page.rs","start_line":40,"end_line":58},
    {"type":"diff","hunk_id":"h_91ab"},
    {"type":"test_output","step_id":"verify","truncated":true}
  ],
  "items": [
    {"path":"src/page.rs","line":44,"comment":"Seharusnya < bukan <="}
  ]
}
```

### 4.1 Pemetaan ke mode PTY
Mode PTY tidak memahami pesan terstruktur. Workbench **merender pesan menjadi teks** terformat (templat per profil) lalu mengetikkannya ke stdin, dengan:
- batas panjang dan penandaan jelas ("[Komentar review 1/3] ..."),
- opsi *bracketed paste* agar CLI tidak menafsirkan baris baru sebagai submit prematur,
- konfirmasi sebelum mengirim bila agen terdeteksi sedang menunggu input lain.

Templat per profil di `agents.toml`: `send_template`, `paste_mode = "bracketed" | "plain"`, `submit_key = "enter"`.

## 5. Status Aktivitas Agen (heuristik PTY)

Untuk kartu sesi, daemon menyimpulkan `activity`: `working` (ada output/lalu lintas proxy), `waiting_input` (pola prompt dikenali, konfigurasi `prompt_regex` per profil), `idle` (tidak ada aktivitas > N detik), `blocked` (menunggu approval). Heuristik dapat salah; ditampilkan sebagai petunjuk, bukan status resmi.

## 6. Keamanan Protokol

- Proses agen terstruktur berjalan di sandbox yang sama; kanal stdio tidak membuka jaringan.
- Permintaan izin dari agen **tidak** menurunkan sandbox: persetujuan hanya mengizinkan aksi yang tetap berada dalam kebijakan sandbox; aksi di luar kebijakan butuh perubahan kebijakan eksplisit.
- Konten pesan dari agen diperlakukan tak tepercaya (sanitasi markdown/HTML di UI; lihat IDE_SECURITY).
- Alat MCP internal bersifat *least privilege* dan dibatasi per sesi (token sesi).

## 7. Peta Jalan Integrasi
1. I1: hanya PTY + pesan teks.
2. I3: templat konteks PTY, `review_feedback` ter-render.
3. I4: spike dan implementasi ACP (adaptor `agent-proto`), permintaan izin terstruktur.
4. I5: server MCP internal, tabel kompatibilitas agen.
