# Dependency baseline

- Source baseline: `37c28d7d7e368d3f647205b3489cfa7bf3b07b6b`
- Rust: `1.96.0`
- Scaffold: `create-gpui-app 0.1.5`, repository commit `63fbe214da83c5409f6845147d793d595b12e2c5`
- GPUI: exact crates.io release `0.2.2`
- GPUI default features: `font-kit`, `wayland`, `x11`, `windows-manifest`
- Local platform: Niri Wayland session on AMD Radeon RX 9070 XT with `amdgpu`

## Upstream risks

- GPUI remains pre-1.0 and changes its API often.
- GPUI main commit `fdad9186` breaks the current scaffold API.
- Upstream issue 56294 reports a Wayland resize shift when elements register `on_next_frame` during paint.
- Public distribution requires a full resolved-license review.

## Initial audit state

`cargo metadata --locked` resolved 703 packages across all target platforms. Every package declares a license expression. The graph includes MPL-2.0 tools or target dependencies and permissive alternatives in several compound expressions. A public artifact still requires `cargo deny`, generated notices, and a review of which target-specific packages ship in each artifact.

Packages that need explicit review include `cbindgen`, `dwrote`, `option-ext`, `r-efi`, `self_cell`, and the NCSA-licensed portion of `libfuzzer-sys`. This list records review work. It does not state that distribution is approved or prohibited.
