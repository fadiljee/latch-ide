# Breakdown Task - Agent IDE

Format sama dengan [TASKS.md](../TASKS.md). Prefiks `IT-` (IDE Task). Satu task = satu PR.

## I0 - Spike & Fondasi
- [ ] **IT-001 · Spike ukuran bundel** · NFR-I01 · `web`
  - Bangun prototipe: CM6 (6 bahasa) + xterm + diff; ukur per chunk. AC: laporan di `docs/perf/ide-spike-bundle.md`, rekomendasi pustaka diff.
- [ ] **IT-002 · Spike review per hunk (opsi A vs B)** · FR-I31 · `git-engine`
  - AC: prototipe revert hunk via `git apply -R`; uji hunk bergeser; keputusan didokumentasikan.
- [ ] **IT-003 · Spike `fs-service` aman** · NFR-I07 · `fs-service`
  - AC: `openat2` RESOLVE_BENEATH; suite fuzz jalur; perbandingan `rustix` vs `cap-std` (ukuran).
- [ ] **IT-004 · Spike memori indeks & watcher** · NFR-I05 · `search`,`fs-service`
  - AC: RSS pada repo sintetis 50k berkas; rekomendasi struktur data.
- [ ] **IT-005 · Spike mode terstruktur (ACP)** · FR-I24 · `agent-proto`
  - AC: jalankan satu agen yang mendukung protokol; catat kemampuan dan celah; input ADR-0016.
- [ ] **IT-006 · Desain visual & token** · UX · desain
  - AC: token warna/tipografi/spasi, komponen dasar, tema terang/gelap/kontras tinggi.
- [x] **IT-007 · Bootstrap `web/` + generasi tipe + gerbang CI** · `web`
  - AC: Vite+TS+Tailwind+Vitest+ESLint a11y; tipe disinkronkan dari `core`; gerbang ukuran bundel di CI.

## I1 - Mission Control & Review Dasar
- [ ] **IT-101 · Tipe & event API IDE di `core`** · `core` · Dep: IT-007
- [ ] **IT-102 · Token ber-scope** · `daemon`,`store` · `latch token create/ls/revoke`; hash tersimpan. AC: scope ditegakkan di semua rute; tes 403.
- [ ] **IT-103 · `latch ide` & penyajian workbench** · FR-I102 · `cli`,`daemon`
  - AC: membuka browser dengan token (fragment/IPC), tidak menaruh token di log.
- [ ] **IT-104 · Shell aplikasi & layout panel** · FR-I01 · `web`
- [ ] **IT-105 · Palet perintah & registry perintah** · FR-I02 · `web`
- [ ] **IT-106 · Klien WS + store + reconnect** · NFR-I09 · `web`
  - AC: `seq`/resync; indikator koneksi.
- [ ] **IT-107 · Papan sesi & kartu** · FR-I10, I12 · `web`
  - AC: kolom sesuai status; update live < 500 ms.
- [ ] **IT-108 · Dialog sesi baru** · FR-I13 · `web`
- [ ] **IT-109 · Inbox Tindakan + approve/deny** · FR-I14 · `web`,`daemon`
- [ ] **IT-110 · Grid terminal (xterm.js)** · FR-I11, I60 · `web`
  - AC: mode fokus+strip/2×2; render terminal tak terlihat dihentikan; input hanya sel fokus.
- [ ] **IT-111 · Diff multi-file (read-only)** · FR-I30 · `web`,`daemon`
- [ ] **IT-112 · Merge-check & merge/discard UI** · FR-I33 · `web`
- [ ] **IT-113 · Kirim pesan teks (PTY)** · FR-I20 · `daemon`,`web`
  - AC: `bracketed paste`, templat per profil, pratinjau teks.
- [ ] **IT-114 · Layout mobile (papan, approve, diff)** · FR-I100 · `web`
- [ ] **IT-115 · PWA & push** · FR-I101 · `web`,`daemon`
- [ ] **IT-116 · Header keamanan & CSP workbench** · IDE_SECURITY §5 · `daemon`
- [ ] **IT-117 · E2E I1 & gerbang ukuran** · DoD I1

## I2 - Editor & Explorer
- [ ] **IT-201 · `fs-service`: tree/baca/tulis/ops dengan ETag** · NFR-I07 · `fs-service`
- [ ] **IT-202 · Larang `.git/` & audit tulis** · IT2 · `fs-service`,`store`
- [ ] **IT-203 · Watcher + event `fs.changed`** · FR-I53 · `fs-service`
- [ ] **IT-204 · Explorer UI** · FR-I50 · `web`
- [ ] **IT-205 · Editor CM6 (buka/simpan/tab)** · FR-I40 · `web`
- [ ] **IT-206 · Konkurensi & 3-arah** · FR-I41 · `web`,`fs-service`
- [ ] **IT-207 · Deteksi sentuhan agen & `edit_policy`** · FR-I42 · `daemon`,`web`
- [ ] **IT-208 · File besar/biner/gambar** · FR-I43 · `fs-service`,`web`
- [ ] **IT-209 · Gutter perubahan agen** · FR-I34 · `web`
- [ ] **IT-210 · Aksi hunk (terima/tolak/undo)** · FR-I31 · `git-engine`,`daemon`,`web`
- [ ] **IT-211 · Quick open fuzzy** · FR-I06 · `search`,`web`
- [ ] **IT-212 · Keybindings & tema & prefs di daemon** · FR-I03-I05 · `daemon`,`web`
- [ ] **IT-213 · Suite tes keamanan fs** · IDE_SECURITY §7

## I3 - Navigasi, Git, Konteks
- [ ] **IT-301 · Pencarian teks streaming** · FR-I51 · `search`,`web`
- [ ] **IT-302 · Panel git** · FR-I70 · `git-engine`,`web`
- [ ] **IT-303 · Lampiran konteks (`@`) & templat tugas** · FR-I21, I22 · `web`,`daemon`
- [ ] **IT-304 · Komentar review & kirim ke agen** · FR-I32 · `web`,`daemon`
- [ ] **IT-305 · Ringkasan perubahan & penanda berisiko** · FR-I35 · `daemon`,`web`
- [ ] **IT-306 · Shell tersandbox** · FR-I61 · `daemon`,`sandbox`,`web`
- [ ] **IT-307 · Tautan `file:line` terminal** · FR-I63 · `web`
- [ ] **IT-308 · Dashboard biaya & riwayat** · FR-I15, I16 · `web`,`store`
- [ ] **IT-309 · Multi-workspace** · FR-I104 · `daemon`,`web`

## I4 - Pipeline & Terstruktur
- [ ] **IT-401 · Viewer DAG** · FR-I90 · `web`
- [ ] **IT-402 · Arena UI & pilih pemenang** · FR-I92, I36 · `web`,`dag`
- [ ] **IT-403 · Fan-out dari panel agen** · FR-I23 · `web`,`daemon`
- [ ] **IT-404 · `agent-proto`: adaptor ACP** · FR-I24 · `agent-proto`
- [ ] **IT-405 · Permintaan izin terstruktur → approval gate** · FR-17/FR-I24 · `daemon`,`agent-proto`
- [ ] **IT-406 · UI aktivitas/tool call** · `web`
- [ ] **IT-407 · Antrean pesan** · FR-I25 · `daemon`
- [ ] **IT-408 · Sinkron base & deteksi konflik** · FR-I72 · `git-engine`,`web`

## I5 - LSP, Konflik, Native, Ekstensibilitas
- [ ] **IT-501 · `lsp-host` tersandbox + batas memori** · FR-I80 · `lsp-host`
- [ ] **IT-502 · Jembatan LSP WS + filter metode** · FR-I81 · `daemon`
- [ ] **IT-503 · `lsp.toml` & registri** · FR-I82 · `core`
- [ ] **IT-504 · Integrasi LSP di editor** · FR-I81 · `web`
- [ ] **IT-505 · Resolusi konflik 3-arah + "minta agen"** · FR-I73 · `web`,`daemon`
- [ ] **IT-506 · Shell host (opsional, terkunci)** · FR-I62 · `daemon`
- [ ] **IT-507 · Shell native (`wry`)** · FR-I103 · `cli`
- [ ] **IT-508 · Perintah kustom & impor templat** · FR-I110-I111 · `core`,`web`
- [ ] **IT-509 · Simbol dasar (outline)** · FR-I52 · `web`/`search`
- [ ] **IT-510 · Editor workflow dengan validasi** · FR-I91 · `web`,`dag`
- [ ] **IT-511 · Dokumentasi & rilis** · USER_GUIDE, CHANGELOG

## Lintas Milestone
- [ ] **IT-900 · Audit a11y berkala** · NFR-I08
- [ ] **IT-901 · i18n id/en** · NFR-I12
- [ ] **IT-902 · Matriks browser e2e** · NFR-I10
- [ ] **IT-903 · Laporan perf per rilis** · `docs/perf/`
- [ ] **IT-904 · Tabel kompatibilitas agen** · `docs/compat/agents.md`

## Templat
```markdown
### IT-xxx · Judul
- Requirement: FR-Ixx / NFR-Ixx
- Area: web | fs-service | search | ...
- Dependensi:
- AC:
  - [ ] ...
- Uji: unit / e2e / keamanan
- Dampak ukuran bundel / RAM:
- Dokumen diperbarui:
```
