#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
source_svg="$repo_root/assets/brand/dagsverk-app-icon.svg"
target_dir="$repo_root/assets/brand/generated"

command -v magick >/dev/null
mkdir -p "$target_dir" "$repo_root/packaging/linux"
magick -background none "$source_svg" -resize 1024x1024 "$target_dir/dagsverk-app-icon.png"
magick "$target_dir/dagsverk-app-icon.png" -define icon:auto-resize=256,128,64,48,32,16 "$target_dir/dagsverk-app-icon.ico"
magick -background none "$source_svg" -resize 256x256 "$repo_root/packaging/linux/dagsverk.png"
magick -background none "$source_svg" -resize 64x64 "$repo_root/public/favicon.ico"

echo "Generated Dagsverk application icons."
