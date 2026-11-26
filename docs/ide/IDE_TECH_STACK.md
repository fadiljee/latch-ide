# Tech Stack - Agent IDE

Tambahan terhadap [TECH_STACK.md](../TECH_STACK.md). Aturan emas: **setiap dependensi harus lolos gerbang ukuran dan RAM**.

## 1. Backend (Rust) Tambahan

| Kebutuhan | Pilihan | Alasan | Catatan |
| --- | --- | --- | --- |
| Walk direktori & ignore | `ignore` | Menghormati `.gitignore`, cepat, dipakai ripgrep | Thread dibatasi |
| Pencarian teks | `grep-searcher`, `grep-regex`, `grep-matcher` | Mesin ripgrep sebagai pustaka | Hindari memanggil biner `rg` eksternal |
| Fuzzy match | `nucleo` | Cepat, hemat memori, dipakai Helix | |
| Pantau file | `notify` (inotify) | Standar | Debounce; batasi watch |
| File aman | `rustix` (`openat2`) atau `cap-std` | Cegah traversal/symlink escape | Pilih berdasarkan dukungan kernel/ukuran |
| Diff/hunk | `similar` (diff teks) + `gix`/`git` untuk patch | Hunk stabil, ringan | `git apply -R` untuk revert hunk |
| Highlight sisi server (opsional) | Tidak dipakai | Highlight di klien | Hemat RAM daemon |
| Simbol ringan | `tree-sitter` (opsional, fitur) | Outline tanpa LSP | Ukuran grammar besar; modul opsional/diunduh |
| LSP | Proses eksternal + jembatan JSON-RPC sendiri | Tidak membundel server bahasa | Dijalankan di sandbox |
| Shell native | `wry` + `tao` (fitur `native-shell`) | Webview sistem, tanpa Chromium | Butuh WebKitGTK/WebView2 |
| Pembangkit tipe | `specta` atau `ts-rs` | Kontrak Rust↔TS sinkron | |
| ID/versi | `blake3` atau `xxhash` untuk ETag | Cepat | |

## 2. Frontend

| Lapisan | Pilihan | Alasan | Anggaran/catatan |
| --- | --- | --- | --- |
| Framework | React 18+ + TypeScript | Ekosistem, rekrutmen | Pertimbangkan Preact via alias bila ukuran perlu |
| Build | Vite | Code splitting, cepat | `manualChunks` per fitur |
| Styling | Tailwind CSS + CSS variables (token tema) | Ringkas setelah purge | |
| State | Zustand (+ slice per fitur) | Kecil, sederhana | |
| Editor | **CodeMirror 6** (`@codemirror/*`) | Modular, ±300-500 KB gzip untuk set inti, dukungan mobile lebih baik, tanpa worker berat | Monaco ditolak (ADR-0013) |
| Bahasa editor | `@codemirror/lang-*` dimuat malas per bahasa | Hindari memuat semua | Lezer grammar |
| Diff | `@codemirror/merge` untuk diff inline editor; `diff2html` atau komponen kustom untuk daftar multi-file | Konsisten dengan editor | Pilih setelah spike |
| Terminal | `@xterm/xterm` + `addon-fit` + `addon-webgl` (opsional, dimuat malas) + `addon-search` | Standar | Render WebGL hanya bila didukung |
| Graf DAG | `@xyflow/react` (React Flow) atau `elkjs`/SVG sederhana | Visual pipeline | Pilih yang terkecil; mungkin SVG kustom |
| Grafik biaya | `uPlot` atau SVG kustom | Sangat kecil | Hindari Chart.js/D3 penuh |
| Virtualisasi | `@tanstack/react-virtual` | Daftar besar (tree, hasil cari, diff) | |
| Palet/perintah | Registry kustom + `cmdk` (opsional) | Ringan | |
| Panel resizable | `react-resizable-panels` atau kustom | Layout | |
| Drag-drop papan | `@dnd-kit/core` | Aksesibel | |
| Markdown | `markdown-it` + `DOMPurify` | Render aman pesan agen | Lazy |
| i18n | `i18next` ringan atau kustom | id/en | |
| Tes | Vitest + Testing Library + Playwright (e2e) | | |
| Lint | ESLint + `typescript-eslint` + `eslint-plugin-jsx-a11y` | A11y | |
| PWA | `vite-plugin-pwa` | Sudah di stack | |

## 3. Anggaran Ukuran (rincian awal, terkompresi)

| Bagian | Anggaran |
| --- | --- |
| Shell + Mission Control + state + API klien | ≤ 400 KB (bagian bundel awal ≤ 600 KB) |
| Editor CM6 inti + 6 bahasa umum | ≤ 450 KB (lazy) |
| Bahasa tambahan | ≤ 20-40 KB per bahasa (lazy) |
| Terminal xterm.js | ≤ 120 KB (lazy) |
| Diff/review | ≤ 150 KB (lazy) |
| Graf pipeline | ≤ 120 KB (lazy) |
| Lainnya (i18n, ikon, markdown) | ≤ 300 KB |
| **Total** | **≤ 3 MB** (NFR-I01) |

Ikon: SVG sprite kecil atau `lucide` dengan *tree-shaking*; hindari paket ikon penuh. Font: system font stack.

## 4. Build & Pengiriman

```text
web/ (Vite) --build--> web/dist --brotli/gzip prekompres--> rust-embed (daemon)
```

- Aset terkompresi Brotli ditanam; daemon melayani `Content-Encoding: br` sesuai `Accept-Encoding` (fallback gzip) tanpa dekompresi ke heap.
- Hash nama berkas + cache `immutable`; `index.html` `no-cache`.
- Peta sumber (source map) tidak ditanam di rilis.
- CI: laporan ukuran bundle per chunk (`rollup-plugin-visualizer`/`size-limit`) dan gagal bila melewati anggaran.

## 5. Dukungan Browser
Chrome/Edge/Firefox (2 versi terakhir), Safari ≥ 16 (iOS ≥ 16.4 untuk push PWA). Gunakan fitur web yang luas didukung; hindari API eksperimental tanpa fallback.

## 6. Keputusan yang Ditolak

| Opsi | Alasan |
| --- | --- |
| Monaco Editor | > 10 MB, worker berat (ADR-0013) |
| Electron / Chromium bundle | Bertentangan dengan tujuan (ADR-0006/0014) |
| Tauri (WebView + runtime sendiri) | Sudah ditolak di ADR-0006; hanya `wry` mentah opsional (ADR-0014) |
| UI Rust murni (Leptos/Dioxus) | Ekosistem editor/diff/terminal lebih matang di JS; tinjau ulang pasca-v1 |
| Theia / code-server | Bobot besar dan scope IDE penuh |
| Highlight sisi server | Menambah RAM daemon |
