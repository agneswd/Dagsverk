# Dagsverk

Dagsverk is an offline-first desktop app for timesheets, time balance, salary estimates, and payroll reports. It runs on Windows and Linux. Your data stays on your computer.

## Features

- Separate workspaces for jobs, clients, contracts, or other work
- Monthly ledger and calendar views
- Workday, day-off, lunch, project, and note tracking
- Expected hours, public holidays, overtime, OB, and comp-time calculations
- Hourly and monthly salary estimates
- Swedish preliminary tax estimates from bundled tax tables
- Excel and OpenDocument report exports
- Local SQLite storage with backup, restore, and Tidverk import
- English and Swedish interfaces
- Light, dark, and system themes
- Automatic updates through GitHub Releases

## Install

Download the latest files from [GitHub Releases](https://github.com/agneswd/Dagsverk/releases/latest).

### Windows 10 or later

Download and run `Dagsverk-<version>-Setup.exe`. Velopack installs Dagsverk for the current user and adds application shortcuts.

Windows can show a SmartScreen warning for an unsigned installer. Confirm that you downloaded the file from this repository before you continue.

### Linux x64

Run the AppImage directly:

```bash
chmod +x Dagsverk-*.AppImage
./Dagsverk-*.AppImage
```

For application-menu integration, extract `Dagsverk-<version>-linux-x64.tar.gz` and run:

```bash
./install.sh
```

The installer uses `~/.local/opt/dagsverk` and does not require root access. `uninstall.sh` removes the app but keeps its data.

## Updates

An installed copy checks GitHub after startup. Dagsverk downloads an available update in the background. Settings shows the download state and lets you restart when the update is ready.

Development builds do not check for updates.

## Privacy and local data

Dagsverk has no accounts, cloud storage, telemetry, timer, or activity monitoring. It only contacts GitHub to check for app updates.

The desktop app stores `dagsverk.db` in Electron's per-user application data directory:

- Windows: `%APPDATA%\Dagsverk`
- Linux: `${XDG_CONFIG_HOME:-$HOME/.config}/Dagsverk`

Use Settings to create backups, restore a Dagsverk backup, or import an existing Tidverk database. Import keeps the source database unchanged.

## Reports and tax

Each spreadsheet contains an employer month sheet and a personal time-balance sheet. Salary and tax values stay out of exported reports. Dagsverk supports `.xlsx` and `.ods` files.

Tax values are estimates of preliminary withholding. They are not a final annual tax calculation.

## Develop

Requirements:

- Node.js 24
- npm 12.0.2
- .NET SDK 10 for Velopack packaging

Install dependencies and run the browser development server:

```bash
npm ci
npm start
```

Run the Electron development app:

```bash
npm run dev
```

Verify the app:

```bash
npm run verify
```

Create native Velopack packages:

```bash
npm run package:linux
npm run package:windows
```

Packages are written to `artifacts/releases`. GitHub Actions verifies Windows and Linux before it publishes a tagged release.

## License

Dagsverk is available under the [MIT License](LICENSE).

<sub>Dagsverk was forked from my earlier project [Tidverk](https://github.com/agneswd/Tidverk).</sub>
