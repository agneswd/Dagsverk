# Visual parity

The Angular and Electron application remains the visual reference. The GPUI implementation uses extracted Dagsverk Material 3 tokens and a deterministic fixture database.

## Deterministic fixture

```bash
npm run gpui:visual-fixture
cd gpui
cargo run --release -- --database fixtures/databases/visual-parity.db --today 2026-08-18
```

The preview accepts deterministic visual states and exact window sizes without changing fixture data:

```bash
cd gpui
cargo run --release -- \
  --database fixtures/databases/visual-parity.db \
  --today 2026-08-18 \
  --visual-state editor-dark \
  --window-size 1366x820 \
  --interface-scale 100
```

On Niri, capture one state or the complete set without focusing the preview:

```bash
gpui/tools/visual-diff/capture-gpui-window.sh ledger /tmp/ledger.png
gpui/tools/visual-diff/capture-gpui-screens.sh
```

The fixture contains two workspaces, multiple projects, worked and missing days, leave, overtime, OB, overnight work, notes, holidays, balance, and tax data. Its SHA-256 is `65edb70a9e981ab5403bc97cde0fc3706f5deae180df237566859f109a570502`.

## Linux baseline

The current Niri Wayland captures use a 1366 x 820 window at device scale 1:

- `03_timesheet_fixture_wayland.png`
- `04_calendar_fixture_wayland.png`
- `05_timesheet_fixture_dark_wayland.png`

These images predate the latest shell and editor polish. They remain historical evidence only. New matching captures are required before visual-complete status.

The local Niri rule matches only `dev.agneswd.dagsverk-gpui-preview` and sets `open-focused false`. Launch verification on 2026-08-18 confirmed that the GPUI window did not take focus. The capture tool also compares focus before and after each image. Niri created no file during the latest attempt because the DMS fade-to-lock overlay held exclusive input and Niri reported no focused window. No desktop focus or lock state was changed to force a capture.

## Current measured implementation

- Electron metrics contain no unexpected `null` values.
- Sidebar widths are 256 px and 80 px.
- Header height is 64 px.
- Ledger header and rows are 52 px.
- The ledger uses the measured eight-column proportions.
- The day editor is 416 px wide.
- Header menus use trigger-relative GPUI anchors with an 8 px viewport margin.
- Projects and workspaces use the 12 Electron color presets.
- Text edits update the editor draft and pay estimate while typing.
- Notes preserve typed and pasted line breaks.
- Project and day-off fields use outlined, trigger-anchored selects with keyboard navigation.
- Focused shared controls use a 3 px shadow outline that does not change layout. No production Rust control uses a permanent 2 px border.
- The component gallery applies `--interface-scale` to its layout and every reusable control, including overlays.
- Production notices use the shared snackbar host and avoid the sidebar and open day editor at every interface scale.
- Projects stack below an 860 px content pane. Route pages reduce horizontal padding below 720 px.
- The timesheet header hides labels and secondary actions from the measured available pane width, including the docked editor width.
- Settings tabs share one tonal surface with the first settings card and use the selected text and indicator colors.

## Comparison tolerances

- Color tokens must match their extracted hex values exactly.
- Major layout boundaries can differ by at most 2 logical pixels.
- Text baselines can differ by at most 2 logical pixels across platforms.
- Font rasterization and subpixel antialiasing are excluded from pixel thresholds.
- Controls must not leak outside rounded corners.
- Focus, hover, pressed, selected, disabled, and error states require manual interaction review.
