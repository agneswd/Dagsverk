# Architecture

## Crate boundaries

- `dagsverk-core` owns pure domain behavior.
- `dagsverk-data` owns SQLite compatibility and destructive data workflows.
- `dagsverk-export` owns report validation and generation.
- `dagsverk-ui` owns GPUI controls and views. It does not call SQL.
- `dagsverk-app` constructs services and runs background work.

## GPUI baseline

The workspace pins the published GPUI 0.2.2 release. The official `create-gpui-app` 0.1.5 scaffold does not compile against GPUI main commit `fdad9186` because it still calls `Application::new()`. The same scaffold compiles with GPUI 0.2.2.

The default GPUI features enable Wayland, X11, the font backend, and the Windows manifest. The preview adapts the official GPUI 0.2.2 input example instead of adding a component library.
