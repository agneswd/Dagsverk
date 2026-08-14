import { app, BrowserWindow, ipcMain, dialog } from 'electron';
import * as path from 'path';
import { DatabaseService } from './database.service';
import { ExcelExportService } from './excel-export.service';

let mainWindow: BrowserWindow | null = null;
let dbService: DatabaseService | null = null;

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
      contextIsolation: true
    }
  });

  const isDev = process.env['NODE_ENV'] === 'development' || !app.isPackaged;
  if (isDev) {
    mainWindow.loadURL('http://localhost:4200');
  } else {
    mainWindow.loadFile(path.join(__dirname, '../dist/dagsverk/browser/index.html'));
  }

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
  ipcMain.handle('db:save-settings', async (_, settings, wsId) => dbService?.saveSettings(wsId, settings));

  ipcMain.handle('db:get-entries', async (_, year, month, wsId) => dbService?.getWorkEntries(wsId, year, month));
  ipcMain.handle('db:save-entry', async (_, entry, wsId) => dbService?.saveWorkEntry(wsId, entry));
  ipcMain.handle('db:delete-entry', async (_, date, wsId) => dbService?.deleteWorkEntry(wsId, date));

  ipcMain.handle('db:get-month', async (_, year, month, defaultOpening, wsId) => dbService?.getMonthRecord(wsId, year, month, defaultOpening));
  ipcMain.handle('db:save-month', async (_, record, wsId) => dbService?.saveMonthRecord(wsId, record));

  ipcMain.handle('db:get-projects', async (_, wsId) => dbService?.getProjects(wsId));
  ipcMain.handle('db:save-project', async (_, project, wsId) => dbService?.saveProject(wsId, project));
  ipcMain.handle('db:delete-project', async (_, id, wsId) => dbService?.deleteProject(wsId, id));

  ipcMain.handle('db:backup', async (_, folder) => dbService?.createBackup(folder));
  ipcMain.handle('db:restore', async (_, filePath) => dbService?.restoreBackup(filePath));

  ipcMain.handle('export:excel', async (_, request, outputPath) => {
    await ExcelExportService.exportToFile(request, outputPath);
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

  app.on('activate', () => {
    if (BrowserWindow.getAllWindows().length === 0) {
      createWindow();
    }
  });
});

app.on('window-all-closed', () => {
  if (process.platform !== 'darwin') {
    app.quit();
  }
});
