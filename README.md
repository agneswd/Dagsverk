# Dagsverk

Dagsverk is an Angular and Electron fork of [Tidverk](https://github.com/agneswd/Tidverk). It keeps Tidverk's offline timesheet and salary workflow while adding Material 3 design and separate workspaces.

The project is under active development. Feature parity with Tidverk is not complete.

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
