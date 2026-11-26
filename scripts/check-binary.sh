#!/usr/bin/env bash
# Gerbang ukuran & build statis musl untuk binary latch (T-004, NFR-04).
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
TARGET="x86_64-unknown-linux-musl"
MAX_BYTES=$((30 * 1024 * 1024))
REQUIRE_MUSL=0

if [ "${1:-}" = "--require-musl" ]; then
  REQUIRE_MUSL=1
fi

cd "$REPO_ROOT"

if command -v musl-gcc >/dev/null 2>&1 || command -v x86_64-linux-musl-gcc >/dev/null 2>&1; then
  BIN_PATH="$REPO_ROOT/target/$TARGET/release/latch"
  echo "==> Membangun binary rilis statis ($TARGET)"
  cargo build --release --target "$TARGET" -p latch-cli

  if [ ! -x "$BIN_PATH" ]; then
    echo "Galat: binary tidak ditemukan di $BIN_PATH" >&2
    exit 1
  fi

  SIZE=$(stat -c %s "$BIN_PATH")
  SIZE_MB=$(awk "BEGIN {printf \"%.2f\", $SIZE / (1024*1024)}")
  echo "Ukuran binary ($TARGET): $SIZE byte ($SIZE_MB MiB) [batas < 30 MiB]"

  if [ "$SIZE" -ge "$MAX_BYTES" ]; then
    echo "Galat: ukuran binary ($SIZE byte) melewati batas 30 MB" >&2
    exit 1
  fi

  LDD_OUT=$(ldd "$BIN_PATH" 2>&1 || true)
  echo "ldd output: $LDD_OUT"
  if ! printf '%s' "$LDD_OUT" | grep -qE "(not a dynamic executable|statically linked)"; then
    echo "Galat: binary bukan executable statis murni" >&2
    exit 1
  fi

  echo "OK: binary musl statis lolos gerbang ukuran dan keterhubungan."
else
  if [ "$REQUIRE_MUSL" -eq 1 ]; then
    echo "Galat: musl-gcc tidak ditemukan; pasang paket musl-tools terlebih dulu." >&2
    exit 1
  fi
  echo "==> Catatan: musl-gcc belum terpasang (pasang 'sudo apt install musl-tools' untuk build musl statis)."
  echo "==> Menjalankan build rilis host dan memeriksa gerbang ukuran (< 30 MiB)..."
  BIN_PATH="$REPO_ROOT/target/release/latch"
  cargo build --release -p latch-cli

  SIZE=$(stat -c %s "$BIN_PATH")
  SIZE_MB=$(awk "BEGIN {printf \"%.2f\", $SIZE / (1024*1024)}")
  echo "Ukuran binary (host release): $SIZE byte ($SIZE_MB MiB) [batas < 30 MiB]"
  if [ "$SIZE" -ge "$MAX_BYTES" ]; then
    echo "Galat: ukuran binary ($SIZE byte) melewati batas 30 MB" >&2
    exit 1
  fi
  echo "OK: binary rilis host lolos gerbang ukuran."
fi
