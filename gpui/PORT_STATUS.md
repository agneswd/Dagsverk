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
| Application state | `src/app/core/app-state.service.ts` | `dagsverk-app/state` | Yes | N/A | N/A | In progress | Initialization, persistence, derived summaries, editor state, month workflows, workspace changes, preferences, catch-up, and stale-load rejection are tested. |
| Startup and shell | `src/app/app.*`, `src/app/layout/` | `dagsverk-app/startup.rs`, `dagsverk-app/shell.rs` | Yes | In progress | Wayland launch | In progress | The preview loads a real database. It has explicit safe paths, route navigation, month actions, theme actions, and initial shell geometry. |
| Timesheet views | `src/app/features/month-workspace/` | `dagsverk-ui/views/timesheet` | Yes | In progress | Wayland launch | In progress | Summary, ledger, calendar, day editor, catch-up controls, and confirmed month actions are connected to real state. Responsive editor parity remains. |
| Projects | `src/app/features/projects/` | `dagsverk-app/shell.rs` | Yes | Yes | No | Behavior complete | Add, archive, unarchive, transactional default selection, and confirmed non-default deletion use UUID identifiers and real persistence. |
| Workspaces | `src/app/features/workspaces/` | `dagsverk-app/shell.rs` | Yes | Yes | No | Behavior complete | The sidebar dialog creates UUID workspaces, switches active data, changes accents, blocks final deletion, and confirms destructive deletion. |
| Settings | `src/app/features/settings/` | `dagsverk-app/shell.rs` | Yes | In progress | No | In progress | Five typed tabs, structured dirty state, validation, Save, Discard, Ctrl+S, currency confirmation, schedules, payroll, tax, preferences, and rate-band add/remove are connected. Full rate-band field editing remains. |
| Resource comparison | Electron and GPUI builds | `gpui/PERFORMANCE.md` | N/A | N/A | N/A | Not started | Same-machine startup, CPU, memory, database, editor, navigation, and export measurements are required in M8. |
