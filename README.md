# Dagsverk

Dagsverk is an Angular and Electron fork of [Tidverk](https://github.com/agneswd/Tidverk). It keeps Tidverk's offline timesheet, salary, tax, reporting, backup, and update workflows. Material 3 design and separate workspaces extend the original application.

Each workspace owns its identity, schedule, pay settings, projects, month records, and time entries. Application preferences and data tools stay global.

## Development

Install dependencies and start the Angular development server:

```bash
npm ci
npm start
```

Run the Electron application:

```bash
npm run dev
```

## Verification

```bash
npm run build:all
npm test -- --watch=false
```

See [Workspace ownership](docs/workspace-ownership.md) for the application, workspace, month, and entry data boundaries.
