# Known gaps

## Interactive state parity

- Current behavior: Shared buttons, header menus, chips, and maintenance buttons use Material 3 8% hover and 12% pressed state layers. Some shell-specific rows still use direct colors.
- Expected parity: Buttons, icon buttons, rows, chips, and navigation items must show consistent hover, pressed, focus, and disabled states.
- Reason incomplete: Some production shell controls still use direct GPUI elements instead of the shared Material control.
- Files involved: `gpui/crates/dagsverk-ui/src/m3/`, `gpui/crates/dagsverk-app/src/shell.rs`.
- Proposed solution: Apply the shared Material state-layer helper to all remaining clickable surfaces.
- Test needed: Component interaction tests and light/dark visual review.

## Production visual polish

- Current behavior: Month actions and report formats now use Electron-style top-bar menus. Menus use 12px containers and 8px items. Dialogs use 28px corners.
- Expected parity: Every control group, gap, radius, icon, menu position, and state must match the Electron reference.
- Reason incomplete: The first behavior-complete UI pass used direct GPUI layout values. Screen-by-screen metric comparison is still in progress.
- Files involved: `gpui/crates/dagsverk-app/src/shell.rs`, `gpui/crates/dagsverk-ui/src/m3/`, `reference/screenshots/`.
- Proposed solution: Compare each deterministic screen with the Electron capture and apply measured values only.
- Test needed: Comparable light/dark screenshots and keyboard interaction review for every route.

## Background GPUI capture

- Current behavior: Tests and builds run without desktop focus. GPUI 0.2.2 fails to present through Xvfb and panics on a seatless headless Weston compositor.
- Expected parity: Automated captures must not use the active desktop.
- Reason incomplete: The pinned Linux backend needs a real Vulkan presentation surface and assumes a Wayland seat.
- Files involved: `gpui/tools/visual-diff/`, `gpui/VISUAL_PARITY.md`.
- Proposed solution: Use GPUI test-platform structural checks now. Add an isolated GPU-backed virtual compositor when available.
- Test needed: A background capture must produce the same 1366 x 820 fixture geometry without a visible host window.

## Day editor parity

- Current behavior: Status, time, lunch, presets, copy actions, scheduled override, project, day-off reason, notes, reset, pay details, catch-up controls, and Ctrl+S are connected.
- Expected parity: Multiline notes, dynamic pay updates during typing, and holiday details must match Electron.
- Reason incomplete: This is the first M6 editor slice.
- Files involved: `gpui/crates/dagsverk-app/src/shell.rs`, `gpui/crates/dagsverk-ui/src/text_input.rs`.
- Proposed solution: Add multiline input behavior and live draft calculation on the same save path.
- Test needed: GPUI editor focus, validation, save, reset, and catch-up tests.

## Phase 0 platform proof

- Current behavior: The pinned preview builds, launches, sets its native title, resizes under Niri, and exits cleanly on Linux.
- Expected parity: Input, clipboard, focus, resize, and native launch work on Windows and Linux.
- Reason incomplete: Windows CI and launch checks remain. Automated clipboard interaction did not produce reliable evidence, so the clipboard check remains open.
- Files involved: `gpui/crates/dagsverk-app`, `.github/workflows/gpui.yml`.
- Proposed solution: Complete local Wayland checks and Windows CI.
- Test needed: Manual platform checklist and CI release builds.
