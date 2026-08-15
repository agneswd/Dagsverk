#!/usr/bin/env bash
set -euo pipefail

rm -rf -- "$HOME/.local/opt/dagsverk"
rm -f -- "$HOME/.local/share/applications/dagsverk.desktop"
rm -f -- "$HOME/.local/share/icons/hicolor/256x256/apps/dagsverk.png"
gtk-update-icon-cache -f -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true
update-desktop-database "$HOME/.local/share/applications" 2>/dev/null || true
echo "Dagsverk application files removed. Local data was kept."
