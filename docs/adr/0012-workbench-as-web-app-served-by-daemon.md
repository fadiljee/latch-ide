# ADR-0012: Workbench sebagai web app yang disajikan daemon

- Status: Proposed
- Tanggal: (isi saat disetujui)

## Konteks
Agent IDE membutuhkan UI kaya (papan sesi, review diff, editor, terminal). Tujuan produk: RAM rendah, tanpa Electron, bekerja untuk remote/VPS dan HP. ADR-0006 sudah memilih PWA tersemat untuk kontrol dan diff.

## Keputusan
Workbench dibangun sebagai **aplikasi web (React+TS+Vite)** yang disajikan daemon dan dapat dibuka di browser/PWA. Logika bisnis, akses file, pencarian, dan LSP berada di daemon; klien hanya menyajikan dan mengirim perintah. Satu codebase untuk desktop, remote, dan mobile.

## Alternatif yang dipertimbangkan
- **Aplikasi desktop Electron/Tauri**: RAM/ukuran, tidak untuk HP/remote.
- **UI Rust murni (Dioxus/Leptos/egui)**: menarik untuk satu bahasa, namun diff viewer/xterm/editor/PWA lebih matang di web; dapat ditinjau ulang pasca-v1.
- **code-server/Theia**: IDE penuh, berat, di luar scope.

## Konsekuensi
### Positif
Remote-native; satu UI; ukuran bundel terkendali; tidak ada runtime tambahan.
### Negatif / risiko
Permukaan keamanan web (token, XSS, CSP); keterbatasan browser (pintasan bentrok); toolchain JS di build; kebutuhan layanan file/terminal di daemon (scope token).

## Referensi
ADR-0006, IDE_ARCHITECTURE, IDE_SECURITY, FR-I01.
