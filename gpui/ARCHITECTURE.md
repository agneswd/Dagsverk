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

## Domain values

The core crate uses `rust_decimal::Decimal` for money, rates, percentages, and hours. `Money` serializes as a canonical decimal string. Date and time values validate and serialize as the existing `YYYY-MM-DD` and `HH:mm` formats.

Calculation code receives a `Clock`. Production uses `SystemClock`, while tests and visual fixtures use `FixedClock`.

The TypeScript engine generates committed parity fixtures. Rust tests read those fixtures directly. The canonical tax source remains `public/tax-data/tax-2026.json`. Its test SHA-256 is `f660a261b4f4abb44b3595f69d1e93bd2895faad19847ff45b50865919ebc0b6`.

## Database connections

`dagsverk-data` owns the database path. It opens one SQLite connection per operation. Each connection enables WAL, foreign keys, and a five-second busy timeout. Multi-row writes use transactions. The UI never receives a SQLite connection.

## Report generation

`dagsverk-export` validates typed report requests before it writes files. `rust_xlsxwriter` creates XLSX workbooks. The ODS writer uses ZIP and escaped XML directly. File dialogs remain outside this crate.

## Native services

The application crate owns async file-dialog and update traits plus the shell boundary. Native dialogs use `rfd` behind the private service. Development builds use an explicit unavailable updater.

## Material 3 foundation

`dagsverk-ui::m3` owns the shared color and typography tokens. Values come from `src/styles.scss` and the generated design metrics. Reusable controls own focus and interaction state. The text editor remains based on GPUI 0.2.2's official input example.
