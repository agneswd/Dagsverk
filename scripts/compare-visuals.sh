#!/usr/bin/env bash
set -euo pipefail

electron_dir="${1:-reference/screenshots/electron}"
gpui_dir="${2:-reference/screenshots/gpui}"
output_dir="${3:-reference/screenshots/comparison}"

for command in compare composite identify montage; do
  command -v "$command" >/dev/null || {
    echo "Missing ImageMagick command: $command" >&2
    exit 1
  }
done

mkdir -p "$output_dir"
report="$output_dir/geometry.tsv"
printf 'capture\telectron_width\telectron_height\tgpui_width\tgpui_height\tabsolute_error_pixels\n' >"$report"

matched=0
for electron_image in "$electron_dir"/*.png; do
  name="$(basename "$electron_image")"
  gpui_image="$gpui_dir/$name"
  [[ -f "$gpui_image" ]] || continue
  matched=$((matched + 1))

  read -r electron_width electron_height < <(identify -format '%w %h\n' "$electron_image")
  read -r gpui_width gpui_height < <(identify -format '%w %h\n' "$gpui_image")
  if [[ "$electron_width" != "$gpui_width" || "$electron_height" != "$gpui_height" ]]; then
    error='dimension-mismatch'
  else
    error="$(compare -metric AE "$electron_image" "$gpui_image" null: 2>&1 || true)"
    montage "$electron_image" "$gpui_image" -tile 2x1 -geometry +0+0 "$output_dir/${name%.png}_side_by_side.png"
    composite -dissolve 50 "$gpui_image" "$electron_image" "$output_dir/${name%.png}_overlay.png"
    compare "$electron_image" "$gpui_image" "$output_dir/${name%.png}_diff.png" 2>/dev/null || true
  fi
  printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
    "$name" "$electron_width" "$electron_height" "$gpui_width" "$gpui_height" "$error" >>"$report"
done

if ((matched == 0)); then
  echo "No matching screenshot names in $electron_dir and $gpui_dir." >&2
  exit 1
fi

echo "Wrote $report and $matched comparison set(s)."
