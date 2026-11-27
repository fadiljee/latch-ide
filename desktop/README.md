# Latch Desktop (Tauri v2)

Aplikasi desktop native untuk **Latch IDE** berbasis Tauri v2 (WebView OS — tanpa Electron/Chromium). Saat dibuka, aplikasi otomatis menyalakan `latch-daemon` di latar belakang dan menyuntikkan token autentikasi lokal langsung ke WebView (zero-setup).

## Prasyarat Linux (Ubuntu / Debian)

```bash
sudo apt-get update && sudo apt-get install -y \
  libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev
```

## Build

```bash
# 1. Build frontend web/dist
(cd web && npm ci && npm run build)

# 2. Build binary desktop native (atau paket .deb / .AppImage dengan cargo tauri build)
cargo build --release --manifest-path desktop/Cargo.toml
```
