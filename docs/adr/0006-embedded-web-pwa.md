# ADR-0006: Web PWA tersemat, bukan Electron/Tauri

- Status: Accepted
- Tanggal: (isi saat disetujui)

## Konteks
Pengguna butuh melihat diff side-by-side di desktop dan meninjau/approve dari HP. UI desktop berbasis Electron/Tauri bertentangan dengan tujuan RAM dan ukuran, dan tidak membantu akses dari HP.

## Keputusan
Sediakan **Web PWA** (React + TypeScript + Vite + Tailwind) yang dibangun menjadi aset statis dan ditanam ke binary dengan `rust-embed`, dilayani oleh daemon (`axum`). Pustaka frontend dipilih ringan (`xterm.js`, `diff2html`/`react-diff-viewer-continued`, Zustand); Monaco dilarang. Node.js hanya dibutuhkan saat **build**, bukan runtime. TUI tetap menjadi klien utama untuk terminal.

## Alternatif yang dipertimbangkan
- **Electron/Tauri**: RAM/ukuran besar, tidak untuk HP.
- **UI Rust murni (Leptos/Yew/Dioxus)**: menarik, tetapi ekosistem diff viewer/xterm/PWA lebih matang di JS dan merekrut kontributor lebih mudah dengan React.
- **Hanya TUI**: tidak memenuhi kebutuhan HP.

## Konsekuensi
### Positif
Satu UI untuk desktop dan HP; ukuran bundle kecil; tanpa runtime tambahan.
### Negatif / risiko
Toolchain JS pada pipeline build; anggaran bundle harus dijaga (≤ ~600 KB gzip); permukaan keamanan web (CSP, Origin, token); Web Push butuh HTTPS.

## Referensi
FR-22, NFR-04, [TECH_STACK.md §2](../TECH_STACK.md).
