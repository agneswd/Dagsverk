# Port status

| Feature | Electron source | Rust target | Domain tests | UI implemented | Visual checked | Status | Notes |
|---|---|---|---|---|---|---|---|
| GPUI platform spike | `electron/main.ts` | `dagsverk-app` | N/A | Yes | No | In progress | Linux and Windows CI builds pass. Niri Wayland launch and clean close pass. Windows launch remains manual. |
| Models | `src/app/core/models.ts` | `dagsverk-core` | Yes | N/A | N/A | Complete | All fields are ported. Persisted enum and currency values round-trip. |
| Monthly calculations | `src/app/core/monthly-calculations.ts` | `dagsverk-core` | Yes | No | No | Behavior complete | TypeScript fixtures cover time, overtime, OB, pay, monthly accrual, and decimal rounding. |
| Swedish holidays | `src/app/core/swedish-holiday.service.ts` | `dagsverk-core` | Yes | N/A | N/A | Behavior complete | Fixtures cover 2024 through 2035, Sundays, named holidays, and major periods. |
| Tax | `src/app/core/tax-calculator.service.ts` | `dagsverk-core` | Yes | N/A | N/A | Behavior complete | All modes, columns, sampled boundaries, and the canonical tax-data SHA pass. |
| Balance and month copy | `src/app/core/app-state.service.ts` | `dagsverk-core` | Yes | No | No | Behavior complete | Latest explicit balance edits and weekday-occurrence mapping are tested. |
| SQLite | `electron/database.service.ts` | `dagsverk-data` | Yes | N/A | N/A | Behavior complete | Electron-to-Rust, Rust-to-Electron, round-trip, migration, backup, restore, and Tidverk tests pass. |
| XLSX and ODS export | `electron/*-export.service.ts` | `dagsverk-export` | Yes | N/A | N/A | Behavior complete | Semantic ZIP, XML, worksheet, formula, cached value, localization, and validation tests pass. |
| Native platform services | `electron/main.ts`, `preload.ts` | `dagsverk-app/platform` | Yes | N/A | N/A | Behavior complete | Async native dialogs, shell opening, injectable traits, and development updater state are implemented. |
| Material 3 foundation | `src/styles.scss` | `dagsverk-ui/m3` | Yes | In progress | Linux | In progress | Core controls, tabs, segmented choices, dialogs, menus, snackbars, progress, expansion panels, and Material Symbols compile and launch on Wayland. |
| Resource comparison | Electron and GPUI builds | `gpui/PERFORMANCE.md` | N/A | N/A | N/A | Not started | Same-machine startup, CPU, memory, database, editor, navigation, and export measurements are required in M8. |
