# Data compatibility

The GPUI preview uses the current six-table SQLite schema without changes. Tests use copied fixture databases or temporary directories. No test opens the production data path.

The compatible production paths are:

- Windows: `%APPDATA%\Dagsverk\dagsverk.db`
- Linux: `${XDG_CONFIG_HOME:-$HOME/.config}/Dagsverk/dagsverk.db`

`--database` has highest precedence. `--data-dir` follows it. `DAGSVERK_DATA_DIR` follows both.

The Rust schema keeps all current table and column names. It does not add a schema-version table.

Legacy migration creates an online SQLite safety backup before it changes tables. Restore validates a temporary candidate, creates a current-data safety backup, removes WAL sidecars, and rolls back after replacement failures.

Tidverk import validates and snapshots the source first. It creates a Dagsverk safety backup before one import transaction. A pristine target uses `ws-default`. A populated target gets a new workspace. The source file remains unchanged.

Do not run Electron Dagsverk and the GPUI preview against the same database during backup, restore, or import.
