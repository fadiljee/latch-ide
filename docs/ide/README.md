# Agent IDE (Workbench) - Indeks Dokumen

**Agent IDE** adalah lapisan antarmuka kerja (*workbench*) di atas `Latch (`latch-ide`)`. Tujuannya: mengubah runtime/orkestrator ini menjadi **IDE yang berpusat pada agen AI**, di mana unit kerja utamanya adalah *sesi agen di worktree*, bukan sekadar file atau tab editor.

> Nama kerja: **Agent IDE** / **Workbench**. Binary tetap `latch`; UI dibuka lewat `latch ide`.

## Prinsip

1. **Agent-first, bukan editor-first.** Editor ada untuk meninjau dan mengoreksi hasil agen, bukan bersaing dengan VS Code/JetBrains sebagai editor serbaguna.
2. **Tetap ringan.** Daemon inti tetap mengikuti anggaran RAM/ukuran; layanan IDE dimuat malas (*lazy*) dan dapat dimatikan.
3. **Tanpa Electron.** Workbench adalah web app yang disajikan daemon (browser/PWA), dengan *shell* native opsional berbasis webview sistem (lihat ADR-0014).
4. **Remote-native.** File tinggal di mesin daemon; browser hanya klien. Ini membuat IDE otomatis bekerja untuk VPS dan HP.
5. **Aman secara default.** Seluruh akses file, terminal, dan LSP melewati sandbox dan token ber-*scope*.

## Peta Dokumen

| Dokumen | Isi |
| --- | --- |
| [IDE_PRD.md](IDE_PRD.md) | Visi, persona, masalah, scope, user stories Agent IDE |
| [IDE_REQUIREMENTS.md](IDE_REQUIREMENTS.md) | FR-I01.. dan NFR-I01.. dengan prioritas |
| [IDE_ARCHITECTURE.md](IDE_ARCHITECTURE.md) | Arsitektur layanan IDE di daemon dan workbench web |
| [IDE_UX_SPEC.md](IDE_UX_SPEC.md) | Tata letak, panel, alur interaksi, mobile |
| [AGENT_PROTOCOL.md](AGENT_PROTOCOL.md) | Integrasi agen: mode PTY dan mode terstruktur |
| [IDE_API.md](IDE_API.md) | Ekstensi API: fs, search, git, terminal, LSP, agent chat |
| [IDE_TECH_STACK.md](IDE_TECH_STACK.md) | Pilihan teknologi frontend/backend IDE |
| [IDE_SECURITY.md](IDE_SECURITY.md) | Model ancaman khusus workbench |
| [IDE_NFR_BUDGET.md](IDE_NFR_BUDGET.md) | Anggaran performa/ukuran IDE dan cara mengukur |
| [IDE_ROADMAP.md](IDE_ROADMAP.md) | Milestone IDE dan keterkaitan dengan fase runtime |
| [IDE_TASKS.md](IDE_TASKS.md) | Breakdown task |
| [IDE_RISKS.md](IDE_RISKS.md) | Risiko dan mitigasi |
| [IDE_USER_GUIDE.md](IDE_USER_GUIDE.md) | Panduan pengguna |
| [KEYBINDINGS.md](KEYBINDINGS.md) | Pintasan keyboard bawaan |
| [EXTENSIBILITY.md](EXTENSIBILITY.md) | Tema, perintah, profil, template, plugin |
| [COMPETITIVE_POSITIONING.md](COMPETITIVE_POSITIONING.md) | Posisi terhadap Orca, enowX, dan alternatif lain |
| [SCOPE_CHANGES.md](SCOPE_CHANGES.md) | Perubahan terhadap dokumen/ADR runtime yang sudah ada |
| [IDE_GLOSSARY.md](IDE_GLOSSARY.md) | Istilah tambahan |
| [AGENTS_IDE.md](AGENTS_IDE.md) | Aturan untuk AI coding agent yang mengerjakan workbench |
| [../adr/](../adr/) | ADR-0012 s.d. ADR-0017 (keputusan IDE) |

## Urutan Baca yang Disarankan

`SCOPE_CHANGES` → `IDE_PRD` → `IDE_UX_SPEC` → `IDE_ARCHITECTURE` → `AGENT_PROTOCOL` → `IDE_SECURITY` → `IDE_ROADMAP`.

## Status

Draft v1. Bergantung pada runtime v0.2 (server web + WebSocket + token). Lihat [IDE_ROADMAP.md](IDE_ROADMAP.md).
