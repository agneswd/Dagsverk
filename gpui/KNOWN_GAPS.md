# Known gaps

## Production shell controls

- Current behavior: The preview loads a real database and renders the first shell, summary, and saved-entry list.
- Expected parity: Header, navigation, editor, menu, dialog, and keyboard behavior must match Electron.
- Reason incomplete: The shell is the current M5 vertical slice. Timesheet and administrative views follow in M6 and M7.
- Files involved: `gpui/crates/dagsverk-app/src/shell.rs`, `gpui/crates/dagsverk-ui/src/`.
- Proposed solution: Replace each temporary route body with its tested Material 3 view.
- Test needed: GPUI focus, action, view navigation, and visual comparison tests.

## Day editor parity

- Current behavior: Worked, off, and incomplete status changes can save. Start, end, lunch, presets, notes, reset, and Ctrl+S are connected.
- Expected parity: Copy actions, scheduled override, projects, multiline notes, day-off reasons, pay details, and catch-up footer controls must match Electron.
- Reason incomplete: This is the first M6 editor slice.
- Files involved: `gpui/crates/dagsverk-app/src/shell.rs`, `gpui/crates/dagsverk-ui/src/text_input.rs`.
- Proposed solution: Add the remaining typed controls to the same draft and save path.
- Test needed: GPUI editor focus, validation, save, reset, and catch-up tests.

## Phase 0 platform proof

- Current behavior: The pinned preview builds, launches, sets its native title, resizes under Niri, and exits cleanly on Linux.
- Expected parity: Input, clipboard, focus, resize, and native launch work on Windows and Linux.
- Reason incomplete: Windows CI and launch checks remain. Automated clipboard interaction did not produce reliable evidence, so the clipboard check remains open.
- Files involved: `gpui/crates/dagsverk-app`, `.github/workflows/gpui.yml`.
- Proposed solution: Complete local Wayland checks and Windows CI.
- Test needed: Manual platform checklist and CI release builds.
