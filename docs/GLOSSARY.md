# Glosarium

Istilah baku agar manusia dan agen AI konsisten. Urut abjad.

| Istilah | Definisi |
| --- | --- |
| **Agen (agent)** | Program CLI AI yang menulis/mengubah kode (mis. Claude Code, Codex CLI, OpenCode, Gemini CLI, skrip kustom) dan dijalankan di dalam PTY. |
| **`latch`** | Nama binary tunggal (klien CLI/TUI dan daemon via subcommand). Nama kerja sementara. |
| **`latchd`** | Alias mode daemon (symlink/argv0 dari binary yang sama). |
| **Approval gate** | Mekanisme menjeda aksi berisiko dan meminta keputusan Approve/Deny dari manusia (FR-17). |
| **Arena (fan-out arena)** | Tampilan perbandingan hasil beberapa kandidat agen pada prompt yang sama (FR-20). |
| **Artefak** | Keluaran node pipeline (teks, diff, berkas) yang diteruskan ke node berikutnya. |
| **AppContainer** | Mekanisme sandbox Windows; target masa depan di balik `SandboxProvider`. |
| **Bearer token** | Token acak lokal pada header `Authorization: Bearer` untuk Web/WS. |
| **Bypass (proxy bypass)** | Mode per profil yang menonaktifkan injeksi proxy; PTY dan sandbox tetap aktif (FR-13). |
| **Circuit breaker** | Pemutus otomatis yang menjeda/menghentikan agen saat batas biaya/token/retry terlampaui (FR-12). |
| **Daemon** | Proses latar belakang yang memiliki seluruh state dan logika; klien hanya berkomunikasi dengannya. |
| **DAG** | *Directed Acyclic Graph*; graf dependensi node pipeline tanpa siklus. |
| **Denylist path sensitif** | Daftar path yang tidak boleh diakses agen (`~/.ssh`, `~/.aws`, ...). Di Landlock diwujudkan dengan tidak memberi akses. |
| **Estafet (relay)** | Penerusan artefak dari satu tahap ke tahap berikutnya dalam pipeline. |
| **Fan-out** | Menjalankan satu prompt yang sama secara paralel pada ≥ 2 profil/model di worktree terpisah. |
| **Hibernasi (hibernation)** | Menghentikan/membekukan sesi idle sambil menyimpan konteks dan status worktree untuk dilanjutkan (FR-04). |
| **Landlock** | Modul keamanan Linux (kernel ≥ 5.13) untuk membatasi akses sumber daya oleh proses tanpa privilese. |
| **Loop detector** | Komponen yang mendeteksi error/tool-call identik berulang (FR-11). |
| **Merge-tree** | `git merge-tree --write-tree`: simulasi merge tanpa mengubah working tree, dipakai untuk mendeteksi konflik. |
| **NFR** | *Non-Functional Requirement*; persyaratan kinerja/keamanan/reliabilitas. |
| **Node (pipeline)** | Satu langkah di `workflow.toml`: `latch`, `fanout`, atau `shell`. |
| **Orphan worktree (yatim)** | Worktree yang tidak lagi terkait sesi aktif; dibersihkan saat startup (NFR-06). |
| **P0 / P1 / P2** | Prioritas: wajib MVP / pembeda inti / otomasi penuh. |
| **Pipeline** | Eksekusi workflow DAG; tercatat sebagai `pipeline_runs`. |
| **Profil agen (agent profile)** | Konfigurasi CLI agen di `agents.toml`: perintah, argumen, env, dukungan proxy. |
| **Proxy (local LLM proxy)** | Reverse proxy HTTP/SSE lokal di daemon untuk telemetri token/biaya dan circuit breaker (FR-09-FR-13). |
| **PTY** | *Pseudo-terminal*; antarmuka terminal semu tempat CLI agen berjalan. |
| **PWA** | *Progressive Web App*; Web UI yang dapat dipasang di layar utama HP. |
| **Retry gate** | Node validasi yang, bila gagal, mengumpan balik stdout/stderr ke node sebelumnya hingga `max_retries` (FR-19). |
| **Ring buffer** | Buffer melingkar berkapasitas tetap untuk output PTY terbaru. |
| **Sandbox** | Pembatasan tingkat OS yang membatasi apa yang bisa diakses proses agen. |
| **`SandboxProvider`** | Trait Rust yang mengabstraksi implementasi sandbox per OS (FR-16). |
| **Seatbelt** | Mekanisme sandbox macOS; target masa depan. |
| **Sesi (session)** | Satu eksekusi agen: worktree + branch + proses PTY + telemetri + status. |
| **Snapshot layar** | Salinan keadaan layar virtual (`vt100`) untuk hibernasi/reattach/HP. |
| **Squash merge** | Menggabungkan seluruh perubahan branch sebagai satu commit ke branch tujuan. |
| **Thin client** | Klien tanpa logika bisnis (TUI, Web) yang hanya menyajikan dan mengirim perintah. |
| **Token sesi (proxy)** | Token acak per sesi pada path proxy untuk mengidentifikasi sesi; bukan API key. |
| **TUI** | *Terminal UI*; antarmuka terminal interaktif berbasis `ratatui`. |
| **`tripped`** | Status sesi yang dihentikan/dijeda circuit breaker. |
| **UDS** | *Unix Domain Socket*; jalur IPC lokal CLI/TUI ↔ daemon. |
| **Worktree** | Direktori kerja tambahan dari `git worktree` dengan branch sendiri untuk isolasi per sesi. |
| **`workflow.toml`** | Berkas deklaratif definisi pipeline DAG (FR-18). |


Untuk istilah khusus workbench (Agent IDE, CM6, ETag, `fs-service`, hunk, LSP, scope), lihat [ide/IDE_GLOSSARY.md](ide/IDE_GLOSSARY.md).
