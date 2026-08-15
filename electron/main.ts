import { app, BrowserWindow, ipcMain, dialog, shell } from 'electron';
import { GithubSource, UpdateInfo, UpdateManager, VelopackApp } from 'velopack';
import * as path from 'path';
import * as fs from 'fs';
import { DatabaseService } from './database.service';
import { ExcelExportService } from './excel-export.service';
import { OdsExportService } from './ods-export.service';

VelopackApp.build().run();

let mainWindow: BrowserWindow | null = null;
let dbService: DatabaseService | null = null;
let updateState: Record<string, unknown> = { status: 'idle' };
let updateManager: UpdateManager | null = null;
let pendingUpdate: UpdateInfo | null = null;

function log(message: string, error?: unknown): void {
  try {
    const logsDirectory = app.getPath('logs');
    fs.mkdirSync(logsDirectory, { recursive: true });
    const retentionCutoff = Date.now() - 7 * 24 * 60 * 60 * 1000;
    for (const fileName of fs.readdirSync(logsDirectory)) {
      if (!/^dagsverk-\d{4}-\d{2}-\d{2}\.log(?:\.old)?$/.test(fileName)) continue;
      const retainedLog = path.join(logsDirectory, fileName);
      if (fs.statSync(retainedLog).mtimeMs < retentionCutoff) fs.unlinkSync(retainedLog);
    }

    const logPath = path.join(
      logsDirectory,
      `dagsverk-${new Date().toISOString().slice(0, 10)}.log`,
    );
    if (fs.existsSync(logPath) && fs.statSync(logPath).size > 1_000_000) {
      const oldLogPath = `${logPath}.old`;
      if (fs.existsSync(oldLogPath)) fs.unlinkSync(oldLogPath);
      fs.renameSync(logPath, oldLogPath);
    }
    const detail =
      error instanceof Error
        ? ` ${error.stack || error.message}`
        : error
          ? ` ${String(error)}`
          : '';
    fs.appendFileSync(logPath, `${new Date().toISOString()} ${message}${detail}\n`);
  } catch {
    // Logging must not interrupt the application.
  }
}

function sendUpdateState(state: Record<string, unknown>): void {
  updateState = { ...updateState, ...state, currentVersion: app.getVersion() };
  mainWindow?.webContents.send('update:state', updateState);
}

async function checkForUpdates(manual: boolean): Promise<void> {
  if (!app.isPackaged) {
    sendUpdateState({
      status: 'unavailable',
      message: 'Updates are available in packaged builds.',
    });
    return;
  }

  try {
    updateManager ??= new UpdateManager(
      new GithubSource('https://github.com/agneswd/Dagsverk'),
    );
    sendUpdateState({ status: 'checking', message: undefined, progress: undefined });
    pendingUpdate = await updateManager.checkForUpdatesAsync();
    if (!pendingUpdate) {
      sendUpdateState({ status: 'current', availableVersion: undefined });
      return;
    }

    sendUpdateState({
      status: 'available',
      availableVersion: pendingUpdate.TargetFullRelease.Version,
    });
    await updateManager.downloadUpdateAsync(pendingUpdate, (progress) =>
      sendUpdateState({ status: 'downloading', progress: Math.round(progress) }),
    );
    sendUpdateState({ status: 'ready', progress: 100 });
  } catch (error) {
    log('Update error.', error);
    pendingUpdate = null;
    sendUpdateState(
      manual
        ? { status: 'error', message: error instanceof Error ? error.message : String(error) }
        : { status: 'idle', message: undefined },
    );
  }
}

function configureUpdates(): void {
  setTimeout(() => void checkForUpdates(false), 3000);
}

function createWindow() {
  dbService = new DatabaseService();

  mainWindow = new BrowserWindow({
    width: 1366,
    height: 850,
    minWidth: 960,
    minHeight: 640,
    frame: false,
    titleBarStyle: 'hidden',
    backgroundColor: '#131314',
    webPreferences: {
      preload: path.join(__dirname, 'preload.js'),
      nodeIntegration: false,
      contextIsolation: true,
    },
  });

  const isDev = process.env['NODE_ENV'] === 'development' || !app.isPackaged;
  if (isDev) {
    mainWindow.loadURL('http://localhost:4200');
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/dagsverk/browser/index.html'));
  }

  mainWindow.webContents.setWindowOpenHandler(({ url }) => {
    if (url.startsWith('https://')) void shell.openExternal(url);
    return { action: 'deny' };
  });

  mainWindow.on('closed', () => {
    mainWindow = null;
  });
}

// Register IPC handlers
function registerIpcHandlers() {
  ipcMain.handle('db:get-workspaces', async () => dbService?.getWorkspaces());
  ipcMain.handle('db:save-workspace', async (_, ws) => dbService?.saveWorkspace(ws));
  ipcMain.handle('db:delete-workspace', async (_, id) => dbService?.deleteWorkspace(id));

  ipcMain.handle('db:get-preferences', async () => dbService?.getAppPreferences());
  ipcMain.handle('db:save-preferences', async (_, prefs) => dbService?.saveAppPreferences(prefs));

  ipcMain.handle('db:get-settings', async (_, wsId) => dbService?.getSettings(wsId));
  ipcMain.handle('db:save-settings', async (_, settings, wsId) =>
    dbService?.saveSettings(settings, wsId),
  );

  ipcMain.handle('db:get-entries', async (_, year, month, wsId) =>
    dbService?.getWorkEntries(year, month, wsId),
  );
  ipcMain.handle('db:save-entry', async (_, entry, wsId) => dbService?.saveWorkEntry(entry, wsId));
  ipcMain.handle('db:delete-entry', async (_, date, wsId) =>
    dbService?.deleteWorkEntry(date, wsId),
  );

  ipcMain.handle('db:get-month', async (_, year, month, defaultOpening, wsId) =>
    dbService?.getMonthRecord(year, month, defaultOpening, wsId),
  );
  ipcMain.handle('db:save-month', async (_, record, wsId) =>
    dbService?.saveMonthRecord(record, wsId),
  );
  ipcMain.handle('db:get-balance-history', async (_, year, month, wsId) =>
    dbService?.getBalanceHistory(year, month, wsId),
  );

  ipcMain.handle('db:get-projects', async (_, wsId) => dbService?.getProjects(wsId));
  ipcMain.handle('db:save-project', async (_, project, wsId) =>
    dbService?.saveProject(project, wsId),
  );
  ipcMain.handle('db:delete-project', async (_, id, wsId) => dbService?.deleteProject(id, wsId));

  ipcMain.handle('db:backup', async (_, folder) => dbService?.createBackup(folder));
  ipcMain.handle('db:restore', async (_, filePath) => dbService?.restoreBackup(filePath));
  ipcMain.handle('db:get-path', async () => dbService?.getDatabasePath());
  ipcMain.handle('db:open-folder', async () => {
    const databasePath = dbService?.getDatabasePath();
    if (databasePath) await shell.openPath(path.dirname(databasePath));
  });

  ipcMain.handle('update:get-state', async () => ({
    ...updateState,
    status: app.isPackaged ? updateState['status'] : 'unavailable',
    currentVersion: app.getVersion(),
  }));
  ipcMain.handle('update:check', async () => {
    await checkForUpdates(true);
  });
  ipcMain.on('update:restart', () => {
    if (!updateManager || !pendingUpdate) return;
    updateManager.waitExitThenApplyUpdate(pendingUpdate);
    app.quit();
  });

  ipcMain.handle('export:excel', async (_, request, outputPath) => {
    if (path.extname(outputPath).toLowerCase() === '.ods') {
      await OdsExportService.exportToFile(request, outputPath);
    } else {
      await ExcelExportService.exportToFile(request, outputPath);
    }
  });

  ipcMain.handle('dialog:save-file', async (_, options) => {
    if (!mainWindow) return { canceled: true, filePath: undefined };
    return await dialog.showSaveDialog(mainWindow, options);
  });

  ipcMain.handle('dialog:open-file', async (_, options) => {
    if (!mainWindow) return { canceled: true, filePaths: [] };
    return await dialog.showOpenDialog(mainWindow, options);
  });

  ipcMain.on('window:minimize', () => {
    mainWindow?.minimize();
  });

  ipcMain.on('window:maximize', () => {
    if (mainWindow?.isMaximized()) {
      mainWindow.unmaximize();
    } else {
      mainWindow?.maximize();
    }
  });

  ipcMain.on('window:close', () => {
    mainWindow?.close();
  });

  ipcMain.handle('window:is-maximized', () => mainWindow?.isMaximized() ?? false);
}

app.whenReady().then(() => {
  registerIpcHandlers();
  createWindow();
  configureUpdates();

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

process.on('uncaughtException', (error) => log('Uncaught exception.', error));
process.on('unhandledRejection', (error) => log('Unhandled rejection.', error));

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
