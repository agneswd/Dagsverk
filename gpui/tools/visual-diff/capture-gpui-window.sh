#!/usr/bin/env bash
set -euo pipefail

if (($# < 2 || $# > 4)); then
  echo "Usage: $0 <visual-state> <output.png> [window-size] [interface-scale]" >&2
  exit 2
fi

for command in jq niri; do
  command -v "$command" >/dev/null || {
    echo "Missing command: $command" >&2
    exit 1
  }
done

tool_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
gpui_dir="$(cd "$tool_dir/../.." && pwd)"
state="$1"
output="$(realpath -m "$2")"
window_size="${3:-1366x820}"
interface_scale="${4:-100}"
binary="$gpui_dir/target/release/dagsverk-gpui"
fixture="$gpui_dir/fixtures/databases/visual-parity.db"

[[ -x "$binary" ]] || {
  echo "Build the preview first: cd gpui && cargo build --release -p dagsverk-app" >&2
  exit 1
}
[[ -f "$fixture" ]] || {
  echo "Missing visual fixture: $fixture" >&2
  exit 1
}

temporary_dir="$(mktemp -d)"
preview_pid=''
cleanup() {
  if [[ -n "$preview_pid" ]]; then
    kill "$preview_pid" 2>/dev/null || true
    wait "$preview_pid" 2>/dev/null || true
  fi
  rm -r -- "$temporary_dir"
}
trap cleanup EXIT

cp "$fixture" "$temporary_dir/visual-parity.db"
mkdir -p "$(dirname "$output")"
rm -f "$output"

before_focus="$(niri msg --json focused-window 2>/dev/null | jq -r '.id // empty')"
"$binary" \
  --database "$temporary_dir/visual-parity.db" \
  --today 2026-08-18 \
  --visual-state "$state" \
  --window-size "$window_size" \
  --interface-scale "$interface_scale" \
  >"$temporary_dir/preview.log" 2>&1 &
preview_pid=$!

window_id=''
for _ in {1..100}; do
  window_id="$(
    niri msg --json windows \
      | jq -r --argjson pid "$preview_pid" '.[] | select(.pid == $pid) | .id' \
      | head -n 1
  )"
  [[ -n "$window_id" ]] && break
  sleep 0.1
done

if [[ -z "$window_id" ]]; then
  cat "$temporary_dir/preview.log" >&2
  echo "The GPUI preview window did not appear." >&2
  exit 1
fi

for _ in {1..100}; do
  niri msg action screenshot-window \
    --id "$window_id" \
    --write-to-disk true \
    --show-pointer false \
    --path "$output" >/dev/null
  [[ -s "$output" ]] && break
  sleep 0.1
done

if [[ ! -s "$output" ]]; then
  cat "$temporary_dir/preview.log" >&2
  echo "Niri did not produce a screenshot for window $window_id." >&2
  exit 1
fi

after_focus="$(niri msg --json focused-window 2>/dev/null | jq -r '.id // empty')"
if [[ "$before_focus" != "$after_focus" ]]; then
  echo "Capture changed desktop focus from ${before_focus:-none} to ${after_focus:-none}." >&2
  exit 1
fi

echo "Captured $state to $output without changing focus."
