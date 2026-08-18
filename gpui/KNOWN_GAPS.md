# Known gaps

## Interactive state parity

- Current behavior: Every direct pointer control in the production shell now has a hover state. Buttons, menus, chips, dialog actions, and editor controls use Material 3 8% hover and 12% pressed state layers.
- Expected parity: Buttons, icon buttons, rows, chips, and navigation items must show consistent hover, pressed, focus, and disabled states.
- Reason incomplete: Some production shell controls still use direct GPUI elements and do not yet share the complete keyboard focus implementation.
- Files involved: `gpui/crates/dagsverk-ui/src/m3/`, `gpui/crates/dagsverk-app/src/shell.rs`.
- Proposed solution: Apply the shared Material state-layer helper to all remaining clickable surfaces.
- Test needed: Component interaction tests and light/dark visual review.

## Current GPUI comparison captures

- Current behavior: The complete deterministic Electron set is current. The committed GPUI images predate the latest shell, ledger, editor, and palette changes.
- Expected parity: Every required Electron image has a current GPUI image at the same viewport, theme, fixture, and scale.
- Reason incomplete: Niri did not write an image for the unfocused GPUI window on another workspace during the latest capture attempt.
- Files involved: `reference/screenshots/gpui/`, `gpui/VISUAL_PARITY.md`.
- Proposed solution: Capture through an isolated GPU-backed compositor or a Niri session where the preview workspace is visible without changing keyboard focus.
- Test needed: Run `npm run visual:compare` for every required pair and inspect geometry and interaction states.

## Background GPUI capture

- Current behavior: Tests, builds, and GPUI launches run without taking desktop focus. GPUI 0.2.2 fails to present through Xvfb and panics on a seatless headless Weston compositor. The latest Niri window-ID capture created no file while the window was on another workspace.
- Expected parity: Automated captures must not use the active desktop.
- Reason incomplete: The pinned Linux backend needs a real Vulkan presentation surface and assumes a Wayland seat. The Niri method still uses the active compositor, but an app-specific rule prevents focus changes.
- Files involved: `gpui/tools/visual-diff/`, `gpui/VISUAL_PARITY.md`.
- Proposed solution: Use unfocused Niri captures and GPUI test-platform structural checks now. Add an isolated GPU-backed virtual compositor when available.
- Test needed: Repeat the capture in an isolated GPU-backed compositor when one is available in CI.

## Day editor parity

- Current behavior: The editor uses a 416px tonal side sheet, multiline notes, floating field labels, anchored project and day-off selects, a connected status control, a shared switch, live draft/pay updates, and a 64px footer.
- Expected parity: Selects must close on all outside clicks and show leading icons. Fields must show supporting error text within the Material field anatomy.
- Reason incomplete: The first shared select pass covers trigger anchoring and keyboard navigation. The common overlay host does not yet own outside-click dismissal.
- Files involved: `gpui/crates/dagsverk-app/src/shell.rs`, `gpui/crates/dagsverk-ui/src/text_input.rs`, `gpui/crates/dagsverk-ui/src/m3/select.rs`.
- Proposed solution: Route select panels through the shared overlay host and add field-level error content.
- Test needed: GPUI editor focus, outside dismissal, validation, save, reset, and catch-up tests.

## Phase 0 platform proof

- Current behavior: The pinned preview builds, launches, sets its native title, resizes under Niri, and exits cleanly on Linux.
- Expected parity: Input, clipboard, focus, resize, and native launch work on Windows and Linux.
- Reason incomplete: Windows CI and launch checks remain. Automated clipboard interaction did not produce reliable evidence, so the clipboard check remains open.
- Files involved: `gpui/crates/dagsverk-app`, `.github/workflows/gpui.yml`.
- Proposed solution: Complete local Wayland checks and Windows CI.
- Test needed: Manual platform checklist and CI release builds.

## Filled Material Symbols

- Current behavior: Selected icons use the bundled Material Symbols font with stronger weight. The bundled variable font exposes optical size and weight axes, but no fill axis.
- Expected parity: Selected navigation icons use the filled Material Symbols appearance from Electron.
- Reason incomplete: GPUI 0.2.2 does not expose a fill variation for the current font asset.
- Files involved: `gpui/assets/fonts/`, `gpui/crates/dagsverk-ui/src/m3/icon.rs`.
- Proposed solution: Bundle the required static filled Material SVGs or a compatible filled font asset with its license notice.
- Test needed: Compare selected navigation icons on Windows and Linux.
