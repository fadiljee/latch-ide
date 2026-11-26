#!/usr/bin/env bash
# Benchmark RSS daemon idle (<= 25 MB) dan overhead per sesi PTY (<= 5 MB) (T-151, NFR-01).
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"

if [ -n "${LATCH_BIN:-}" ] && [ -x "$LATCH_BIN" ]; then
  BIN_PATH="$LATCH_BIN"
elif [ -x "$REPO_ROOT/target/x86_64-unknown-linux-musl/release/latch" ]; then
  BIN_PATH="$REPO_ROOT/target/x86_64-unknown-linux-musl/release/latch"
elif [ -x "$REPO_ROOT/target/release/latch" ]; then
  BIN_PATH="$REPO_ROOT/target/release/latch"
else
  echo "==> Binary rilis belum ada; menjalankan scripts/check-binary.sh"
  "$REPO_ROOT/scripts/check-binary.sh"
  if [ -x "$REPO_ROOT/target/x86_64-unknown-linux-musl/release/latch" ]; then
    BIN_PATH="$REPO_ROOT/target/x86_64-unknown-linux-musl/release/latch"
  else
    BIN_PATH="$REPO_ROOT/target/release/latch"
  fi
fi

echo "==> Mengukur RSS menggunakan binary: $BIN_PATH"

TMP_ROOT="$(mktemp -d)"
DAEMON_PID=""
cleanup() {
  if [ -n "$DAEMON_PID" ] && kill -0 "$DAEMON_PID" 2>/dev/null; then
    "$BIN_PATH" daemon stop >/dev/null 2>&1 || true
    sleep 1
    kill -9 "$DAEMON_PID" 2>/dev/null || true
  fi
  rm -rf "$TMP_ROOT"
}
trap cleanup EXIT

export LATCH_CONFIG_DIR="$TMP_ROOT/config"
export LATCH_STATE_DIR="$TMP_ROOT/state"
export LATCH_SOCKET="$TMP_ROOT/state/latch.sock"
mkdir -p "$LATCH_CONFIG_DIR" "$LATCH_STATE_DIR"

REPO_DIR="$TMP_ROOT/bench-repo"
mkdir -p "$REPO_DIR"
git -C "$REPO_DIR" init -q -b main
git -C "$REPO_DIR" config user.name "Latch Bench"
git -C "$REPO_DIR" config user.email "bench@localhost"
echo "bench" > "$REPO_DIR/README.md"
git -C "$REPO_DIR" add .
git -C "$REPO_DIR" commit -q -m "initial"

"$BIN_PATH" daemon start --foreground >/dev/null 2>&1 &
DAEMON_PID=$!

for _ in $(seq 1 50); do
  if [ -S "$LATCH_SOCKET" ] && "$BIN_PATH" daemon status >/dev/null 2>&1; then
    break
  fi
  sleep 0.1
done

sleep 1
IDLE_KB=$(awk '/VmRSS/ {print $2}' "/proc/$DAEMON_PID/status")
echo "idle_rss_kb=$IDLE_KB (batas <= 25600 kB)"
if [ "$IDLE_KB" -gt $((25 * 1024)) ]; then
  echo "Galat: RSS idle ($IDLE_KB kB) melebihi 25 MB" >&2
  exit 1
fi

"$BIN_PATH" project add "$REPO_DIR" --name bench --default-branch main >/dev/null
for i in 1 2 3; do
  "$BIN_PATH" run --project bench --profile sleeper --task "bench-$i" >/dev/null
done

sleep 2
WITH3_KB=$(awk '/VmRSS/ {print $2}' "/proc/$DAEMON_PID/status")
PER_SESSION_KB=$(( (WITH3_KB - IDLE_KB) / 3 ))
echo "with_3_sessions_rss_kb=$WITH3_KB"
echo "per_session_overhead_kb=$PER_SESSION_KB (batas <= 5120 kB)"

if [ "$PER_SESSION_KB" -gt $((5 * 1024)) ]; then
  echo "Galat: overhead per sesi ($PER_SESSION_KB kB) melebihi 5 MB" >&2
  exit 1
fi

echo "OK: anggaran RAM idle dan per sesi PTY terpenuhi."
