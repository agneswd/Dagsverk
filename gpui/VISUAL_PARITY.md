# Visual parity

The Angular and Electron application remains the visual reference. The GPUI implementation uses extracted Dagsverk Material 3 tokens and a deterministic fixture database.

## Deterministic fixture

```bash
npm run gpui:visual-fixture
cd gpui
cargo run --release -- --database fixtures/databases/visual-parity.db --today 2026-06-25
```

The fixture contains two workspaces, multiple projects, worked and missing days, leave, overtime, OB, overnight work, notes, holidays, balance, and tax data. Its SHA-256 is `26e3fc04ad0445fe7a244deb14dfce88b2d14fa301d08aac95f0215d3068335a`.

## Linux baseline

The current Niri Wayland captures use a 1366 x 820 window at device scale 1:

- `03_timesheet_fixture_wayland.png`
- `04_calendar_fixture_wayland.png`
- `05_timesheet_fixture_dark_wayland.png`

Visual review confirmed card clipping, 52 px ledger rows, the six-week calendar grid, the 400 px editor breakpoint, Material colors, and light/dark rendering.

## Comparison tolerances

- Color tokens must match their extracted hex values exactly.
- Major layout boundaries can differ by at most 2 logical pixels.
- Text baselines can differ by at most 2 logical pixels across platforms.
- Font rasterization and subpixel antialiasing are excluded from pixel thresholds.
- Controls must not leak outside rounded corners.
- Focus, hover, pressed, selected, disabled, and error states require manual interaction review.
