# Data compatibility

The GPUI preview will use the current six-table SQLite schema without changes. Tests must use copied fixture databases or temporary directories. No test may open the production data path.

Do not run Electron Dagsverk and the GPUI preview against the same database during backup, restore, or import.
