#!/usr/bin/env sh
# Installer satu baris untuk Latch (`latch` & `latchd`) dari GitHub Releases.
# Penggunaan:
#   curl -fsSL https://raw.githubusercontent.com/fadiljee/latch-ide/main/scripts/install.sh | sh
# Variabel opsional:
#   LATCH_VERSION     - Tag rilis tertentu (mis. v0.3.0); bawaan: rilis terbaru.
#   LATCH_INSTALL_DIR - Direktori tujuan binary (bawaan: $HOME/.local/bin).
#   LATCH_REPO        - Repositori GitHub (bawaan: fadiljee/latch-ide).

set -eu

REPO="${LATCH_REPO:-fadiljee/latch-ide}"
INSTALL_DIR="${LATCH_INSTALL_DIR:-$HOME/.local/bin}"

log() {
  printf '[latch-install] %s\n' "$*"
}

fail() {
  printf '[latch-install] ERROR: %s\n' "$*" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || fail "perintah '$1' tidak ditemukan; pasang terlebih dahulu."
}

need_cmd curl
need_cmd tar
need_cmd mktemp

OS="$(uname -s)"
ARCH="$(uname -m)"

case "$OS" in
  Linux) ;;
  *) fail "OS '$OS' belum didukung oleh binary rilis statis (saat ini: Linux x86_64)." ;;
esac

case "$ARCH" in
  x86_64 | amd64) TARGET="x86_64-unknown-linux-musl" ;;
  *) fail "Arsitektur '$ARCH' belum didukung oleh binary rilis statis (saat ini: x86_64)." ;;
esac

if [ -n "${LATCH_VERSION:-}" ]; then
  TAG="$LATCH_VERSION"
else
  log "Mencari rilis terbaru dari github.com/${REPO} ..."
  TAG="$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
    | sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' \
    | head -n 1)"
  [ -n "$TAG" ] || fail "gagal mendeteksi tag rilis terbaru. Set LATCH_VERSION=v0.3.0 secara manual."
fi

ARCHIVE="latch-${TAG}-${TARGET}.tar.gz"
BASE_URL="https://github.com/${REPO}/releases/download/${TAG}"
TMP_DIR="$(mktemp -d)"

cleanup() {
  rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

log "Mengunduh ${ARCHIVE} (${TAG}) ..."
curl -fsSL "${BASE_URL}/${ARCHIVE}" -o "${TMP_DIR}/${ARCHIVE}" \
  || fail "gagal mengunduh ${BASE_URL}/${ARCHIVE}"

log "Memverifikasi checksum SHA-256 ..."
if curl -fsSL "${BASE_URL}/SHA256SUMS.txt" -o "${TMP_DIR}/SHA256SUMS.txt"; then
  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$TMP_DIR" && sha256sum -c SHA256SUMS.txt) || fail "verifikasi SHA-256 gagal!"
  elif command -v shasum >/dev/null 2>&1; then
    (cd "$TMP_DIR" && shasum -a 256 -c SHA256SUMS.txt) || fail "verifikasi SHA-256 gagal!"
  else
    log "PERINGATAN: sha256sum/shasum tidak tersedia; melewati verifikasi checksum."
  fi
else
  log "PERINGATAN: SHA256SUMS.txt tidak ditemukan pada rilis ${TAG}; melewati verifikasi checksum."
fi

tar -xzf "${TMP_DIR}/${ARCHIVE}" -C "$TMP_DIR"
BIN_SRC="${TMP_DIR}/latch-${TAG}-${TARGET}/latch"
[ -f "$BIN_SRC" ] || fail "binary 'latch' tidak ditemukan di dalam arsip."

mkdir -p "$INSTALL_DIR"
install -m 0755 "$BIN_SRC" "${INSTALL_DIR}/latch" 2>/dev/null || {
  cp "$BIN_SRC" "${INSTALL_DIR}/latch"
  chmod 0755 "${INSTALL_DIR}/latch"
}
ln -sf "${INSTALL_DIR}/latch" "${INSTALL_DIR}/latchd"

log "Berhasil dipasang: ${INSTALL_DIR}/latch (dan symlink latchd)"
"${INSTALL_DIR}/latch" --version || true

# Tambahkan INSTALL_DIR ke PATH shell secara idempoten bila belum terdaftar
case ":${PATH}:" in
  *":${INSTALL_DIR}:"*) ;;
  *)
    PATH_LINE="export PATH=\"${INSTALL_DIR}:\$PATH\""
    UPDATED_RC=""
    for rc in "$HOME/.bashrc" "$HOME/.zshrc" "$HOME/.profile"; do
      if [ -f "$rc" ]; then
        if ! grep -F "$INSTALL_DIR" "$rc" >/dev/null 2>&1; then
          printf '\n# Latch CLI\n%s\n' "$PATH_LINE" >> "$rc"
          UPDATED_RC="${UPDATED_RC} ${rc}"
        fi
      fi
    done
    if [ -n "$UPDATED_RC" ]; then
      log "PATH ditambahkan otomatis ke:${UPDATED_RC}"
      log "Jalankan 'export PATH=\"${INSTALL_DIR}:\$PATH\"' atau buka ulang terminal."
    else
      log "Tambahkan ke PATH Anda: ${PATH_LINE}"
    fi
    ;;
esac

# Auto-start daemon agar langsung siap dipakai tanpa setup manual (lewati dengan LATCH_NO_START=1)
if [ "${LATCH_NO_START:-0}" != "1" ]; then
  "${INSTALL_DIR}/latch" daemon start >/dev/null 2>&1 || true
fi

log "Siap dipakai! Jalankan 'latch' (TUI) atau 'latch web' (Web Workbench di http://127.0.0.1:7878)."
