import { contextBridge, ipcRenderer } from 'electron';

export interface ElectronAPI {
  // Workspaces & Preferences
  getWorkspaces: () => Promise<any[]>;
  saveWorkspace: (workspace: any) => Promise<void>;
  deleteWorkspace: (id: string) => Promise<void>;
  getAppPreferences: () => Promise<any>;
  saveAppPreferences: (prefs: any) => Promise<void>;

  // Scoped Data
  getSettings: (workspaceId?: string) => Promise<any>;
  saveSettings: (settings: any, workspaceId?: string) => Promise<void>;
  getWorkEntries: (year: number, month: number, workspaceId?: string) => Promise<any[]>;
  saveWorkEntry: (entry: any, workspaceId?: string) => Promise<void>;
  deleteWorkEntry: (date: string, workspaceId?: string) => Promise<void>;
  getMonthRecord: (year: number, month: number, defaultOpening?: number, workspaceId?: string) => Promise<any>;
  saveMonthRecord: (record: any, workspaceId?: string) => Promise<void>;
  getProjects: (workspaceId?: string) => Promise<any[]>;
  saveProject: (project: any, workspaceId?: string) => Promise<void>;
  deleteProject: (id: string, workspaceId?: string) => Promise<void>;

  // Utilities
  createBackup: (destinationFolder?: string) => Promise<string>;
  restoreBackup: (filePath: string) => Promise<void>;
  exportExcel: (request: any, outputPath: string) => Promise<void>;
  showSaveDialog: (options: any) => Promise<any>;
  showOpenDialog: (options: any) => Promise<any>;
  minimizeWindow: () => void;
  maximizeWindow: () => void;
  closeWindow: () => void;
  isMaximized: () => Promise<boolean>;
}

const electronAPI: ElectronAPI = {
  getWorkspaces: () => ipcRenderer.invoke('db:get-workspaces'),
  saveWorkspace: (workspace) => ipcRenderer.invoke('db:save-workspace', workspace),
  deleteWorkspace: (id) => ipcRenderer.invoke('db:delete-workspace', id),
  getAppPreferences: () => ipcRenderer.invoke('db:get-preferences'),
  saveAppPreferences: (prefs) => ipcRenderer.invoke('db:save-preferences', prefs),

  getSettings: (workspaceId) => ipcRenderer.invoke('db:get-settings', workspaceId),
  saveSettings: (settings, workspaceId) => ipcRenderer.invoke('db:save-settings', settings, workspaceId),
  getWorkEntries: (year, month, workspaceId) => ipcRenderer.invoke('db:get-entries', year, month, workspaceId),
  saveWorkEntry: (entry, workspaceId) => ipcRenderer.invoke('db:save-entry', entry, workspaceId),
  deleteWorkEntry: (date, workspaceId) => ipcRenderer.invoke('db:delete-entry', date, workspaceId),
  getMonthRecord: (year, month, defaultOpening, workspaceId) => ipcRenderer.invoke('db:get-month', year, month, defaultOpening, workspaceId),
  saveMonthRecord: (record, workspaceId) => ipcRenderer.invoke('db:save-month', record, workspaceId),
  getProjects: (workspaceId) => ipcRenderer.invoke('db:get-projects', workspaceId),
  saveProject: (project, workspaceId) => ipcRenderer.invoke('db:save-project', project, workspaceId),
  deleteProject: (id, workspaceId) => ipcRenderer.invoke('db:delete-project', id, workspaceId),

  createBackup: (folder) => ipcRenderer.invoke('db:backup', folder),
  restoreBackup: (filePath) => ipcRenderer.invoke('db:restore', filePath),
  exportExcel: (request, outputPath) => ipcRenderer.invoke('export:excel', request, outputPath),
  showSaveDialog: (options) => ipcRenderer.invoke('dialog:save-file', options),
  showOpenDialog: (options) => ipcRenderer.invoke('dialog:open-file', options),
  minimizeWindow: () => ipcRenderer.send('window:minimize'),
  maximizeWindow: () => ipcRenderer.send('window:maximize'),
  closeWindow: () => ipcRenderer.send('window:close'),
  isMaximized: () => ipcRenderer.invoke('window:is-maximized')
};

contextBridge.exposeInMainWorld('electronAPI', electronAPI);
