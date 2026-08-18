# Port status

| Feature | Electron source | Rust target | Domain tests | UI implemented | Visual checked | Status | Notes |
|---|---|---|---|---|---|---|---|
| GPUI platform spike | `electron/main.ts` | `dagsverk-app` | N/A | Yes | No | In progress | Linux debug and release builds pass. Niri Wayland launch and clean close pass. Windows CI pending. |
| Models | `src/app/core/models.ts` | `dagsverk-core` | No | N/A | N/A | Not started | Exact persisted enum values required. |
| Monthly calculations | `src/app/core/monthly-calculations.ts` | `dagsverk-core` | No | No | No | Not started | TypeScript fixtures will provide the oracle. |
| SQLite | `electron/database.service.ts` | `dagsverk-data` | No | N/A | N/A | Not started | No schema changes before bidirectional tests. |
