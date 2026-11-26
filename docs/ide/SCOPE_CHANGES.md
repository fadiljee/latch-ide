# Perubahan Scope Akibat Agent IDE

Blueprint awal secara eksplisit menyatakan non-goal: *"menggantikan fungsi text editor / IDE penuh"* dan sempat mengatur anggaran web 1 MB. Keputusan membuat Agent IDE mengubah beberapa hal. Dokumen ini mencatatnya agar tidak ada kontradiksi diam-diam.

## 1. Perubahan Non-Goal

| Sebelumnya | Sekarang |
| --- | --- |
| Bukan pengganti text editor / IDE penuh | **Bukan IDE serbaguna.** Agent IDE adalah workbench agen dengan editor ringan (lihat/edit/review). Tidak menargetkan paritas dengan VS Code/JetBrains: tanpa ekosistem ekstensi VS Code, debugger, refactoring mendalam, atau notebook. |
| Tidak membundel Electron/Chromium | **Tetap berlaku.** Shell native opsional memakai webview sistem (ADR-0014), bukan Chromium bawaan. |
| Tanpa cloud SaaS multi-tenant di v1 | **Tetap berlaku.** |

## 2. Dampak pada Dokumen Runtime

| Dokumen | Perubahan yang perlu dibuat |
| --- | --- |
| `PRD.md` §5.2 | Revisi baris non-goal editor/IDE sesuai tabel di atas; tambah rujukan ke IDE_PRD |
| `REQUIREMENTS.md` | Tambah bagian "G. Workbench (IDE)" yang merujuk `IDE_REQUIREMENTS.md`; FR-22 (Web UI) menjadi subset dari workbench |
| `ARCHITECTURE.md` §3 | Tambah crate baru: `fs-service`, `search`, `lsp-host`, `workbench-api` (lihat IDE_ARCHITECTURE §3) |
| `TECH_STACK.md` §2 | Tambah CodeMirror 6, `nucleo`, `ignore`/`grep-*`, `notify`; anggaran bundle web diperbarui |
| `NFR_BUDGET.md` | Anggaran web dari ≤ 1 MB menjadi ≤ 3 MB terkompresi; tambah NFR-I* (lihat IDE_NFR_BUDGET). Anggaran inti 25 MB idle **tidak berubah**; layanan IDE terpisah dan lazy |
| `API.md` | Perluas dengan IDE_API; token ber-*scope* (read/control/write/shell-host) |
| `SECURITY.md` | Tambah rujukan IDE_SECURITY; token Web kini dapat menulis file, sehingga model ancaman T4 naik tingkat |
| `ROADMAP.md` | Tambah jalur IDE (IDE_ROADMAP) setelah v0.2 |
| `docs/adr/README.md` | Tambah ADR-0012..0017 |
| `AGENTS.md` | Tambah aturan workbench (lihat AGENTS_IDE) |
| `GLOSSARY.md` | Tambah istilah di IDE_GLOSSARY |

## 3. Keputusan yang Dibalik atau Diperluas

| ADR | Status baru |
| --- | --- |
| ADR-0006 (Web PWA tersemat) | **Diperluas**: PWA menjadi workbench penuh. Larangan Monaco tetap (ADR-0013 memilih CodeMirror 6). Larangan Electron/Tauri tetap, dengan pengecualian shell webview opsional (ADR-0014). |
| ADR-0001 (Rust murni) | Tetap. Frontend TypeScript hanya aset build, bukan runtime. |

## 4. Risiko Scope

Menambah IDE adalah perluasan besar. Risiko utamanya: *scope creep* menjadi "editor lengkap". Pengendali:

1. **Daftar "tidak akan dibangun"** di IDE_PRD §5.2 bersifat mengikat; menambah item butuh ADR.
2. **Gerbang anggaran** (IDE_NFR_BUDGET) dijalankan di CI sejak milestone pertama.
3. **Urutan prioritas:** review & kontrol agen dulu, penyuntingan dasar kedua, navigasi kode ketiga, LSP terakhir.

## 5. Yang Tidak Berubah

Tujuan inti runtime (hemat RAM, sandbox, proxy/circuit breaker, pipeline DAG), fase v0.1-v1.0, dan semua FR/NFR sebelumnya tetap berlaku. Agent IDE adalah klien tambahan dan sekumpulan layanan opsional, bukan pengganti.
