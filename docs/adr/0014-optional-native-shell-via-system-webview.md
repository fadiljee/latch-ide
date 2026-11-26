# ADR-0014: Shell native opsional lewat webview sistem

- Status: Proposed
- Tanggal: (isi saat disetujui)

## Konteks
Sebagian pengguna lebih suka jendela aplikasi sendiri (pintasan tidak bentrok dengan browser, ikon dock) dibanding tab browser. ADR-0006 menolak Electron/Tauri karena RAM dan ukuran.

## Keputusan
Sediakan **shell native opsional** (`latch ide --native`) memakai **webview sistem** via `wry`+`tao`, di balik fitur Cargo `native-shell`. Tidak membundel Chromium. Browser/PWA tetap jalur utama dan satu-satunya yang wajib didukung. Bila webview tidak tersedia (mis. WebKitGTK tidak terpasang), perintah jatuh ke browser default.

## Alternatif yang dipertimbangkan
- **Tidak ada shell native**: paling sederhana, tapi pengalaman desktop kurang.
- **Tauri penuh**: menambah runtime/plugin; ditolak di ADR-0006.
- **Electron**: ditolak.

## Konsekuensi
### Positif
Pengalaman jendela mandiri dengan biaya ukuran kecil; tetap tanpa Chromium.
### Negatif / risiko
Dependensi sistem (WebKitGTK/WebView2); fragmentasi versi webview; beban pengujian lintas OS. Mitigasi: opsional, ditandai P2 (FR-I103), diuji di Linux dahulu.

## Referensi
ADR-0006, FR-I103, IR-13.
