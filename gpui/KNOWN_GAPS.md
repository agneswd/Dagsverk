# Known gaps

## Interactive state parity

- Current behavior: Every direct pointer control in the production shell now has a hover state. Buttons, menus, chips, dialog actions, and editor controls use Material 3 8% hover and 12% pressed state layers.
- Expected parity: Buttons, icon buttons, rows, chips, and navigation items must show consistent hover, pressed, focus, and disabled states.
- Reason incomplete: Some production shell controls still use direct GPUI elements and do not yet share the complete keyboard focus implementation.
- Files involved: `gpui/crates/dagsverk-ui/src/m3/`, `gpui/crates/dagsverk-app/src/shell.rs`.
- Proposed solution: Apply the shared Material state-layer helper to all remaining clickable surfaces.
- Test needed: Component interaction tests and light/dark visual review.

## Background GPUI capture

- Current behavior: Tests, builds, and GPUI captures run without taking desktop focus. An isolated headless Sway compositor presents through the real NVIDIA Vulkan driver and records all typed visual states at exact window sizes and scales.
- Expected parity: Automated captures must not use the active desktop.
- Reason incomplete: The pinned Linux backend still cannot present through Xvfb or seatless Weston, so capture depends on a machine with a real Vulkan driver.
- Files involved: `gpui/tools/visual-diff/`, `gpui/VISUAL_PARITY.md`.
- Proposed solution: Keep the non-focusing Niri fallback and document the isolated headless Sway path for GPU-capable hosts.
- Test needed: Add the isolated capture path to a GPU-capable CI runner when one is available.

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
