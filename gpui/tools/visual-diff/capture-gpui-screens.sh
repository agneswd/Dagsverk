#!/usr/bin/env bash
set -euo pipefail

tool_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
gpui_dir="$(cd "$tool_dir/../.." && pwd)"
output_dir="$(realpath -m "${1:-$gpui_dir/../reference/screenshots/gpui}")"
capture="$tool_dir/capture-gpui-window.sh"

mkdir -p "$output_dir"
cargo build --release -p dagsverk-app --manifest-path "$gpui_dir/Cargo.toml"

while IFS='|' read -r state name size scale; do
  "$capture" "$state" "$output_dir/$name" "$size" "$scale"
done <<'CAPTURES'
ledger|01_ledger_light.png|1366x820|100
editor|02_day_editor_light.png|1366x820|100
calendar|03_calendar_light.png|1366x820|100
projects|04_projects_light.png|1366x820|100
settings-general|05_settings_general_light.png|1366x820|100
settings-overtime|06_settings_overtime_light.png|1366x820|100
backups|07_backups_light.png|1366x820|100
workspaces|08_workspace_dialog_light.png|1366x820|100
month-menu|09_month_menu_light.png|1366x820|100
color-picker|10_color_picker_light.png|1366x820|100
ledger-dark|11_ledger_dark.png|1366x820|100
calendar-dark|12_calendar_dark.png|1366x820|100
editor-dark|13_day_editor_dark.png|1366x820|100
settings-dark|14_settings_dark.png|1366x820|100
workspaces-dark|15_workspace_dialog_dark.png|1366x820|100
ledger|21_ledger_960x640.png|960x640|100
ledger|22_ledger_1200x760.png|1200x760|100
editor|23_day_editor_1600x900.png|1600x900|100
ledger|24_ledger_scale_80.png|1366x820|80
ledger|25_ledger_scale_125.png|1366x820|125
ledger|26_ledger_scale_150.png|1366x820|150
CAPTURES
