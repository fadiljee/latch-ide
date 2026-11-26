# Glosarium Tambahan - Agent IDE

Melengkapi [GLOSSARY.md](../GLOSSARY.md).

| Istilah | Definisi |
| --- | --- |
| **ACP (Agent Client Protocol)** | Kandidat protokol JSON-RPC untuk menghubungkan klien/editor dengan agen coding secara terstruktur. Perlu diverifikasi sebelum diadopsi (ADR-0016). |
| **Agent IDE / Workbench** | Antarmuka kerja web berpusat pada sesi agen di atas runtime. |
| **Aktivitas (activity)** | Perkiraan kondisi agen: `working`, `waiting_input`, `idle`, `blocked`. Heuristik. |
| **Arena** | Tampilan perbandingan kandidat hasil fan-out. |
| **Bracketed paste** | Mode terminal yang membungkus teks tempel agar tidak ditafsirkan sebagai ketikan satuan. |
| **CodeMirror 6 (CM6)** | Pustaka editor modular yang dipakai workbench. |
| **Edit policy** | Kebijakan penyuntingan manusia saat agen berjalan: `pause_agent`, `side_by_side`, `readonly_while_running`. |
| **ETag** | Penanda versi isi berkas untuk deteksi konflik tulis. |
| **fs-service** | Layanan akses file aman di akar worktree. |
| **Gutter perubahan** | Penanda di tepi editor untuk baris yang diubah agen. |
| **Hunk** | Potongan perubahan kontigu dalam diff. |
| **`hunk_id`** | Pengenal stabil-berbasis-konteks sebuah hunk. |
| **Inbox Tindakan** | Daftar item yang butuh keputusan/tindakan manusia. |
| **LSP** | Language Server Protocol; sumber diagnostik/definisi/hover. |
| **MCP** | Model Context Protocol; cara menyediakan alat/sumber ke agen. |
| **Mission Control** | Tampilan papan sesi dan grid terminal. |
| **Mode terstruktur** | Integrasi agen lewat protokol JSON-RPC (lawan: mode PTY). |
| **Palet perintah** | Kotak pencarian untuk semua aksi. |
| **Quick open** | Pencarian fuzzy nama berkas (`Mod+P`). |
| **Review feedback** | Pesan terstruktur berisi komentar review untuk agen. |
| **Scope (token)** | Batas izin token: `read`, `control`, `write`, `shell-sandbox`, `shell-host`, `admin`. |
| **Shell host** | Terminal tanpa sandbox; nonaktif default. |
| **Shell native** | Jendela webview sistem (`wry`) untuk membuka workbench. |
| **Sentuhan agen** | Penandaan berkas yang baru/sedang ditulis agen. |
| **Undo token** | Penanda untuk membatalkan aksi hunk. |
| **Workspace** | Konteks proyek yang dibuka di workbench (satu atau lebih sesi/worktree). |
