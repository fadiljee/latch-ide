# ADR-0013: CodeMirror 6 sebagai editor, bukan Monaco

- Status: Proposed
- Tanggal: (isi saat disetujui)

## Konteks
Workbench perlu editor ringan untuk melihat dan mengubah berkas, dengan penanda perubahan agen dan diff inline. Monaco menambah > 10 MB dan worker berat; bertentangan dengan anggaran ukuran (NFR-04, NFR-I01) dan dukungan mobile kurang baik.

## Keputusan
Gunakan **CodeMirror 6** (modular, bahasa dimuat malas, `@codemirror/merge` untuk diff inline). Fitur lanjutan (LSP) diintegrasikan lewat jembatan ke daemon, bukan lewat worker Monaco. Spike IT-001 memvalidasi anggaran ukuran sebelum final.

## Alternatif yang dipertimbangkan
- **Monaco**: kaya fitur, terlalu besar.
- **Ace**: lebih tua, modularitas dan mobile kurang.
- **Textarea + highlight kustom**: terlalu terbatas.
- **Editor sisi native (mis. Helix/neovim embed)**: kompleks, tidak untuk web/HP.

## Konsekuensi
### Positif
Bundel kecil, modular, bagus di mobile, kontrol penuh atas dekorasi.
### Negatif / risiko
Fitur IDE (refactor, minimap kaya, mode Vim) harus dibangun/ditambah sendiri; pengalaman dasar lebih sederhana dibanding VS Code. Konsisten dengan posisi "bukan IDE serbaguna".

## Referensi
IDE_TECH_STACK, NFR-I01, FR-I40.
