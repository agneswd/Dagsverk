# Known gaps

## Interactive state parity

- Current behavior: Shared buttons have hover and pressed opacity. Several shell-specific clickable surfaces do not yet show a Material 3 state layer.
- Expected parity: Buttons, icon buttons, rows, chips, and navigation items must show consistent hover, pressed, focus, and disabled states.
- Reason incomplete: Some production shell controls still use direct GPUI elements instead of the shared Material control.
- Files involved: `gpui/crates/dagsverk-ui/src/m3/`, `gpui/crates/dagsverk-app/src/shell.rs`.
- Proposed solution: Apply the shared Material state-layer helper to all remaining clickable surfaces.
- Test needed: Component interaction tests and light/dark visual review.

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
