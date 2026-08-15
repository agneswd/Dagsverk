#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
rid="${1:-}"
version="$(node -p "require('$repo_root/package.json').version")"

case "$rid" in
  linux-x64)
    channel="linux"
    directive="[linux]"
    unpacked_dir="$repo_root/dist/linux-unpacked"
    executable="Dagsverk"
    icon="$repo_root/assets/brand/generated/dagsverk-app-icon.png"
    builder_target="--linux"
    ;;
  win-x64)
    channel="win"
    directive="[win]"
    unpacked_dir="$repo_root/dist/win-unpacked"
    executable="Dagsverk.exe"
    icon="$repo_root/assets/brand/generated/dagsverk-app-icon.ico"
    builder_target="--win"
    ;;
  *)
    echo "Usage: $0 <linux-x64|win-x64>" >&2
    exit 2
    ;;
esac

cd "$repo_root"
npm ci
npm run build:all
npx electron-builder --dir "$builder_target"

release_dir="$repo_root/artifacts/releases/$channel"
release_notes="$repo_root/docs/release-notes/$version.md"
rm -rf -- "$release_dir"
mkdir -p "$release_dir"

if [[ "${DAGSVERK_SKIP_RELEASE_DOWNLOAD:-0}" != "1" ]]; then
  dotnet dnx vpk --version 1.2.0 --yes -- --legacyConsole download github \
    --repoUrl https://github.com/agneswd/Dagsverk \
    --channel "$channel" \
    --outputDir "$release_dir"
fi

pack_arguments=(
  --packId Dagsverk
  --packVersion "$version"
  --packDir "$unpacked_dir"
  --mainExe "$executable"
  --packTitle Dagsverk
  --packAuthors agneswd
  --runtime "$rid"
  --channel "$channel"
  --icon "$icon"
  --outputDir "$release_dir"
)
[[ -f "$release_notes" ]] && pack_arguments+=(--releaseNotes "$release_notes")
[[ "$rid" == "linux-x64" ]] && pack_arguments+=(--categories "Office;Utility")

dotnet dnx vpk --version 1.2.0 --yes -- "$directive" --legacyConsole pack "${pack_arguments[@]}"

if [[ "$rid" == "linux-x64" ]]; then
  extras_dir="$repo_root/artifacts/extras/linux"
  bundle_name="Dagsverk-$version-linux-x64"
  bundle_dir="$extras_dir/$bundle_name"
  appimage="$(find "$release_dir" -maxdepth 1 -type f -name '*.AppImage' -print -quit)"
  rm -rf -- "$extras_dir"
  mkdir -p "$bundle_dir"
  install -m 0755 "$appimage" "$bundle_dir/Dagsverk.AppImage"
  install -m 0755 "$repo_root/packaging/linux/install-user.sh" "$bundle_dir/install.sh"
  install -m 0755 "$repo_root/packaging/linux/uninstall-user.sh" "$bundle_dir/uninstall.sh"
  install -m 0644 "$repo_root/packaging/linux/dagsverk.desktop" "$bundle_dir/dagsverk.desktop"
  install -m 0644 "$repo_root/packaging/linux/dagsverk.png" "$bundle_dir/dagsverk.png"
  tar -C "$extras_dir" -czf "$extras_dir/$bundle_name.tar.gz" "$bundle_name"
  rm -rf -- "$bundle_dir"
fi

echo "Packaged Dagsverk $version for $rid in $release_dir"
