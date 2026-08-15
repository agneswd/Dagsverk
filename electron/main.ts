import { app, BrowserWindow, ipcMain, dialog, shell } from 'electron';
import { autoUpdater } from 'electron-updater';
import * as path from 'path';
import * as fs from 'fs';
import { DatabaseService } from './database.service';
import { ExcelExportService } from './excel-export.service';
import { OdsExportService } from './ods-export.service';

let mainWindow: BrowserWindow | null = null;
let dbService: DatabaseService | null = null;
let updateState: Record<string, unknown> = { status: 'idle' };

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

function configureUpdates(): void {
  if (!app.isPackaged) {
    sendUpdateState({
      status: 'unavailable',
      message: 'Updates are available in packaged builds.',
    });
    return;
  }
  autoUpdater.autoDownload = true;
  autoUpdater.on('checking-for-update', () => sendUpdateState({ status: 'checking' }));
  autoUpdater.on('update-not-available', () => sendUpdateState({ status: 'current' }));
  autoUpdater.on('update-available', (info) =>
    sendUpdateState({ status: 'available', availableVersion: info.version }),
  );
  autoUpdater.on('download-progress', (progress) =>
    sendUpdateState({ status: 'downloading', progress: Math.round(progress.percent) }),
  );
  autoUpdater.on('update-downloaded', (info) =>
    sendUpdateState({ status: 'ready', availableVersion: info.version, progress: 100 }),
  );
  autoUpdater.on('error', (error) => {
    log('Update error.', error);
    sendUpdateState({ status: 'error', message: error.message });
  });
  setTimeout(
    () =>
      void autoUpdater
        .checkForUpdates()
        .catch((error) => log('Automatic update check failed.', error)),
    3000,
  );
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
    if (!app.isPackaged)
      return sendUpdateState({
        status: 'unavailable',
        message: 'Updates are available in packaged builds.',
      });
    await autoUpdater.checkForUpdates();
  });
  ipcMain.on('update:restart', () => autoUpdater.quitAndInstall());

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
