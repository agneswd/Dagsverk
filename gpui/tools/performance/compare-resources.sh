#!/usr/bin/env bash
set -euo pipefail

tool_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
gpui_dir="$(cd "$tool_dir/../.." && pwd)"
repo_dir="$(cd "$gpui_dir/.." && pwd)"
fixture="$gpui_dir/fixtures/databases/visual-parity.db"
gpui_binary="$gpui_dir/target/release/dagsverk-gpui"
electron_binary="${DAGSVERK_ELECTRON_BINARY:-$repo_dir/dist/linux-unpacked/Dagsverk}"
output="$(realpath -m "${1:-$gpui_dir/resource-comparison.csv}")"
runs="${2:-5}"
duration="${3:-60}"
warmup="${4:-5}"
clock_ticks="$(getconf CLK_TCK)"

for value in "$runs" "$duration" "$warmup"; do
  [[ "$value" =~ ^[0-9]+$ ]] || {
    echo "Runs, duration, and warmup must be non-negative integers." >&2
    exit 2
  }
done
((runs > 0 && duration > 0)) || {
  echo "Runs and duration must be greater than zero." >&2
  exit 2
}

for path in "$fixture" "$gpui_binary" "$electron_binary"; do
  [[ -e "$path" ]] || {
    echo "Missing required file: $path" >&2
    exit 1
  }
done

declare -a cleanup_pids=()
temporary_root="$(mktemp -d)"
cleanup() {
  local pid
  for pid in "${cleanup_pids[@]}"; do
    kill -- "-$pid" 2>/dev/null || kill "$pid" 2>/dev/null || true
  done
  wait "${cleanup_pids[@]}" 2>/dev/null || true
  rm -r -- "$temporary_root"
}
trap cleanup EXIT

collect_pids() {
  local root="$1" current child
  local -a queue=("$root") result=()
  declare -A seen=()

  while ((${#queue[@]})); do
    current="${queue[0]}"
    queue=("${queue[@]:1}")
    [[ -r "/proc/$current/stat" && -z "${seen[$current]:-}" ]] || continue
    seen[$current]=1
    result+=("$current")
    while read -r child; do
      [[ -n "$child" ]] && queue+=("$child")
    done < <(pgrep -P "$current" 2>/dev/null || true)
  done

  ((${#result[@]})) && printf '%s\n' "${result[@]}"
}

process_ticks() {
  local stat suffix
  stat="$(<"/proc/$1/stat")"
  suffix="${stat##*) }"
  set -- $suffix
  printf '%s\n' "$(( ${12} + ${13} ))"
}

sample_tree() {
  local root="$1" app="$2" run="$3" startup_ms="$4"
  local elapsed=0 samples=0 pss_total=0 rss_total=0
  local peak_pss=0 peak_rss=0 max_processes=0 max_threads=0 max_fds=0
  local pid pss rss threads fds ticks
  declare -A first_ticks=() last_ticks=()

  while ((elapsed < duration)); do
    local sample_pss=0 sample_rss=0 sample_threads=0 sample_fds=0 process_count=0
    while read -r pid; do
      [[ -r "/proc/$pid/status" ]] || continue
      ((process_count += 1))
      pss="$(awk '/^Pss:/ { print $2 }' "/proc/$pid/smaps_rollup" 2>/dev/null || true)"
      rss="$(awk '/^VmRSS:/ { print $2 }' "/proc/$pid/status" 2>/dev/null || true)"
      threads="$(awk '/^Threads:/ { print $2 }' "/proc/$pid/status" 2>/dev/null || true)"
      fds="$(find "/proc/$pid/fd" -mindepth 1 -maxdepth 1 -printf . 2>/dev/null | wc -c)"
      ticks="$(process_ticks "$pid" 2>/dev/null || true)"
      ((sample_pss += ${pss:-0}, sample_rss += ${rss:-0}))
      ((sample_threads += ${threads:-0}, sample_fds += fds))
      if [[ -n "$ticks" ]]; then
        [[ -n "${first_ticks[$pid]:-}" ]] || first_ticks[$pid]="$ticks"
        last_ticks[$pid]="$ticks"
      fi
    done < <(collect_pids "$root")

    ((samples += 1, pss_total += sample_pss, rss_total += sample_rss))
    ((sample_pss > peak_pss)) && peak_pss="$sample_pss"
    ((sample_rss > peak_rss)) && peak_rss="$sample_rss"
    ((process_count > max_processes)) && max_processes="$process_count"
    ((sample_threads > max_threads)) && max_threads="$sample_threads"
    ((sample_fds > max_fds)) && max_fds="$sample_fds"
    sleep 1
    ((elapsed += 1))
  done

  local total_tick_delta=0
  for pid in "${!last_ticks[@]}"; do
    total_tick_delta=$((total_tick_delta + last_ticks[$pid] - first_ticks[$pid]))
  done

  awk -v app="$app" -v run="$run" -v startup="$startup_ms" \
    -v ticks="$total_tick_delta" -v hz="$clock_ticks" -v seconds="$duration" \
    -v pss="$pss_total" -v rss="$rss_total" -v samples="$samples" \
    -v peak_pss="$peak_pss" -v peak_rss="$peak_rss" \
    -v processes="$max_processes" -v threads="$max_threads" -v fds="$max_fds" \
    'BEGIN { printf "%s,%d,%d,%.3f,%.1f,%.1f,%.1f,%.1f,%d,%d,%d\n", app, run, startup, ticks / hz / seconds * 100, pss / samples, peak_pss, rss / samples, peak_rss, processes, threads, fds }' \
    >>"$output"
}

wait_for_process_tree() {
  local root="$1" minimum="$2" deadline=$((SECONDS + 20)) count
  while ((SECONDS < deadline)); do
    kill -0 "$root" 2>/dev/null || return 1
    count="$(collect_pids "$root" | wc -l)"
    ((count >= minimum)) && return 0
    sleep 0.1
  done
  return 1
}

run_gpui() {
  local run="$1" data_dir="$temporary_root/gpui-$run" start_ns startup_ms pid
  mkdir -p "$data_dir"
  cp "$fixture" "$data_dir/dagsverk.db"
  start_ns="$(date +%s%N)"
  setsid "$gpui_binary" --database "$data_dir/dagsverk.db" --today 2026-08-18 \
    --visual-state ledger --window-size 1366x820 --interface-scale 100 \
    >"$data_dir/app.log" 2>&1 &
  pid=$!
  cleanup_pids+=("$pid")
  wait_for_process_tree "$pid" 1 || {
    cat "$data_dir/app.log" >&2
    return 1
  }
  startup_ms=$(( ($(date +%s%N) - start_ns) / 1000000 ))
  sleep "$warmup"
  sample_tree "$pid" GPUI "$run" "$startup_ms"
  kill -- "-$pid" 2>/dev/null || kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
}

run_electron() {
  local run="$1" data_dir="$temporary_root/electron-$run" display=$((90 + run))
  local start_ns startup_ms xvfb_pid pid
  mkdir -p "$data_dir/config/Dagsverk"
  cp "$fixture" "$data_dir/config/Dagsverk/dagsverk.db"
  Xvfb ":$display" -screen 0 1366x820x24 -nolisten tcp >"$data_dir/xvfb.log" 2>&1 &
  xvfb_pid=$!
  cleanup_pids+=("$xvfb_pid")
  for _ in {1..100}; do
    [[ -S "/tmp/.X11-unix/X$display" ]] && break
    sleep 0.05
  done
  start_ns="$(date +%s%N)"
  setsid env -u ELECTRON_RUN_AS_NODE -u WAYLAND_DISPLAY DISPLAY=":$display" \
    XDG_CONFIG_HOME="$data_dir/config" ELECTRON_OZONE_PLATFORM_HINT=x11 \
    DAGSVERK_BENCHMARK=1 "$electron_binary" --ozone-platform=x11 --disable-gpu \
    >"$data_dir/app.log" 2>&1 &
  pid=$!
  cleanup_pids+=("$pid")
  wait_for_process_tree "$pid" 4 || {
    cat "$data_dir/app.log" >&2
    return 1
  }
  startup_ms=$(( ($(date +%s%N) - start_ns) / 1000000 ))
  sleep "$warmup"
  sample_tree "$pid" Electron "$run" "$startup_ms"
  kill -- "-$pid" 2>/dev/null || kill "$pid" 2>/dev/null || true
  wait "$pid" 2>/dev/null || true
  kill "$xvfb_pid" 2>/dev/null || true
  wait "$xvfb_pid" 2>/dev/null || true
}

mkdir -p "$(dirname "$output")"
printf 'application,run,process_tree_ready_ms,cpu_percent,mean_pss_kib,peak_pss_kib,mean_rss_kib,peak_rss_kib,max_processes,max_threads,max_fds\n' >"$output"

for ((run = 1; run <= runs; run++)); do
  echo "Run $run/$runs: Electron" >&2
  run_electron "$run"
  echo "Run $run/$runs: GPUI" >&2
  run_gpui "$run"
done

echo "Wrote $output"
