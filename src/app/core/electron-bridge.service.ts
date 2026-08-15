import { Injectable } from '@angular/core';
import {
  AppPreferences,
  AppSettings,
  DEFAULT_PREFERENCES,
  DEFAULT_SETTINGS,
  DEFAULT_WORKSPACE,
  MonthRecord,
  Project,
  ReportExportRequest,
  WorkEntry,
  Workspace,
  WorkspaceType
} from './models';

declare global {
  interface Window {
    electronAPI?: {
      getWorkspaces: () => Promise<Workspace[]>;
      saveWorkspace: (workspace: Workspace) => Promise<void>;
      deleteWorkspace: (id: string) => Promise<void>;
      getAppPreferences: () => Promise<AppPreferences>;
      saveAppPreferences: (prefs: AppPreferences) => Promise<void>;

      getSettings: (workspaceId?: string) => Promise<AppSettings>;
      saveSettings: (settings: AppSettings, workspaceId?: string) => Promise<void>;
      getWorkEntries: (year: number, month: number, workspaceId?: string) => Promise<WorkEntry[]>;
      saveWorkEntry: (entry: WorkEntry, workspaceId?: string) => Promise<void>;
      deleteWorkEntry: (date: string, workspaceId?: string) => Promise<void>;
      getMonthRecord: (year: number, month: number, defaultOpening?: number, workspaceId?: string) => Promise<MonthRecord>;
      saveMonthRecord: (record: MonthRecord, workspaceId?: string) => Promise<void>;
      getProjects: (workspaceId?: string) => Promise<Project[]>;
      saveProject: (project: Project, workspaceId?: string) => Promise<void>;
      deleteProject: (id: string, workspaceId?: string) => Promise<void>;

      createBackup: (folder?: string) => Promise<string>;
      restoreBackup: (filePath: string) => Promise<void>;
      exportExcel: (request: ReportExportRequest, outputPath: string) => Promise<void>;
      showSaveDialog: (options: any) => Promise<{ canceled: boolean; filePath?: string }>;
      showOpenDialog: (options: any) => Promise<{ canceled: boolean; filePaths: string[] }>;
      minimizeWindow: () => void;
      maximizeWindow: () => void;
      closeWindow: () => void;
      isMaximized: () => Promise<boolean>;
    };
  }
}

@Injectable({
  providedIn: 'root'
})
export class ElectronBridgeService {
  private isElectron = typeof window !== 'undefined' && Boolean(window.electronAPI);
  private memoryStorage = new Map<string, string>();

  public get isRunningInElectron(): boolean {
    return this.isElectron;
  }

  private getItem(key: string): string | null {
    if (typeof localStorage !== 'undefined') {
      try {
        return localStorage.getItem(key);
      } catch {
        return this.memoryStorage.get(key) || null;
      }
    }
    return this.memoryStorage.get(key) || null;
  }

  private setItem(key: string, value: string): void {
    if (typeof localStorage !== 'undefined') {
      try {
        localStorage.setItem(key, value);
        return;
      } catch {
        // fallback
      }
    }
    this.memoryStorage.set(key, value);
  }

  // --- Workspaces ---
  public async getWorkspaces(): Promise<Workspace[]> {
    if (this.isElectron) {
      const list = await window.electronAPI!.getWorkspaces();
      return list && list.length > 0 ? list : [DEFAULT_WORKSPACE];
    }
    const local = this.getItem('dagsverk_workspaces');
    if (!local) return [DEFAULT_WORKSPACE];
    return (JSON.parse(local) as Array<Workspace & { employerName?: string }>).map(workspace => ({
      ...workspace,
      type: workspace.type ?? WorkspaceType.Employment,
      organizationName: workspace.organizationName ?? workspace.employerName,
      workerName: workspace.workerName ?? ''
    }));
  }

  public async saveWorkspace(ws: Workspace): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.saveWorkspace(ws);
      return;
    }
    const list = (await this.getWorkspaces()).filter(item => item.id !== ws.id);
    list.push(ws);
    this.setItem('dagsverk_workspaces', JSON.stringify(list));
  }

  public async deleteWorkspace(id: string): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.deleteWorkspace(id);
      return;
    }
    const list = (await this.getWorkspaces()).filter(item => item.id !== id);
    if (list.length === 0) {
      throw new Error('Cannot delete the last remaining workspace');
    }
    this.setItem('dagsverk_workspaces', JSON.stringify(list));
  }

  // --- AppPreferences ---
  public async getAppPreferences(): Promise<AppPreferences> {
    if (this.isElectron) {
      const prefs = await window.electronAPI!.getAppPreferences();
      return prefs || DEFAULT_PREFERENCES;
    }
    const local = this.getItem('dagsverk_preferences');
    return local ? JSON.parse(local) : DEFAULT_PREFERENCES;
  }

  public async saveAppPreferences(prefs: AppPreferences): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.saveAppPreferences(prefs);
      return;
    }
    this.setItem('dagsverk_preferences', JSON.stringify(prefs));
  }

  // --- Workspace Settings ---
  public async getSettings(workspaceId: string = 'ws-default'): Promise<AppSettings> {
    if (this.isElectron) {
      const s = await window.electronAPI!.getSettings(workspaceId);
      return s || { ...DEFAULT_SETTINGS, workspaceId };
    }
    const local = this.getItem(`dagsverk_settings_${workspaceId}`);
    return local ? JSON.parse(local) : { ...DEFAULT_SETTINGS, workspaceId };
  }

  public async saveSettings(settings: AppSettings, workspaceId: string = 'ws-default'): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.saveSettings(settings, workspaceId);
      return;
    }
    this.setItem(`dagsverk_settings_${workspaceId}`, JSON.stringify(settings));
  }

  // --- WorkEntries ---
  public async getWorkEntries(year: number, month: number, workspaceId: string = 'ws-default'): Promise<WorkEntry[]> {
    if (this.isElectron) {
      return await window.electronAPI!.getWorkEntries(year, month, workspaceId);
    }
    const prefix = `${year}-${String(month).padStart(2, '0')}`;
    const all = this.getLocalEntries(workspaceId);
    return all.filter(e => e.date.startsWith(prefix));
  }

  public async saveWorkEntry(entry: WorkEntry, workspaceId: string = 'ws-default'): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.saveWorkEntry(entry, workspaceId);
      return;
    }
    const all = this.getLocalEntries(workspaceId).filter(e => e.date !== entry.date);
    all.push({ ...entry, workspaceId });
    this.setItem(`dagsverk_entries_${workspaceId}`, JSON.stringify(all));
  }

  public async deleteWorkEntry(date: string, workspaceId: string = 'ws-default'): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.deleteWorkEntry(date, workspaceId);
      return;
    }
    const all = this.getLocalEntries(workspaceId).filter(e => e.date !== date);
    this.setItem(`dagsverk_entries_${workspaceId}`, JSON.stringify(all));
  }

  private getLocalEntries(workspaceId: string): WorkEntry[] {
    const raw = this.getItem(`dagsverk_entries_${workspaceId}`);
    return raw ? JSON.parse(raw) : [];
  }

  // --- MonthRecords ---
  public async getMonthRecord(year: number, month: number, defaultOpening = 0, workspaceId: string = 'ws-default'): Promise<MonthRecord> {
    if (this.isElectron) {
      return await window.electronAPI!.getMonthRecord(year, month, defaultOpening, workspaceId);
    }
    const key = `dagsverk_month_${workspaceId}_${year}_${month}`;
    const raw = this.getItem(key);
    if (raw) return JSON.parse(raw);

    return {
      workspaceId,
      year,
      month,
      openingBalanceMinutes: defaultOpening,
      expectedMinutesOverride: null,
      openingBalanceWasEdited: false
    };
  }

  public async saveMonthRecord(record: MonthRecord, workspaceId: string = 'ws-default'): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.saveMonthRecord(record, workspaceId);
      return;
    }
    const key = `dagsverk_month_${workspaceId}_${record.year}_${record.month}`;
    this.setItem(key, JSON.stringify({ ...record, workspaceId }));
  }

  // --- Projects ---
  public async getProjects(workspaceId: string = 'ws-default'): Promise<Project[]> {
    if (this.isElectron) {
      const list = await window.electronAPI!.getProjects(workspaceId);
      return list && list.length > 0 ? list : [
        { workspaceId, id: 'proj-default', name: 'General', color: '#5F875F', isActive: true, isDefault: true }
      ];
    }
    const raw = this.getItem(`dagsverk_projects_${workspaceId}`);
    if (raw) return JSON.parse(raw);

    return [
      { workspaceId, id: 'proj-default', name: 'General', color: '#5F875F', isActive: true, isDefault: true }
    ];
  }

  public async saveProject(project: Project, workspaceId: string = 'ws-default'): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.saveProject(project, workspaceId);
      return;
    }
    const list = (await this.getProjects(workspaceId)).filter(p => p.id !== project.id);
    list.push({ ...project, workspaceId });
    this.setItem(`dagsverk_projects_${workspaceId}`, JSON.stringify(list));
  }

  public async deleteProject(id: string, workspaceId: string = 'ws-default'): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.deleteProject(id, workspaceId);
      return;
    }
    const list = (await this.getProjects(workspaceId)).filter(p => p.id !== id);
    this.setItem(`dagsverk_projects_${workspaceId}`, JSON.stringify(list));
  }

  // --- Utilities ---
  public async createBackup(destinationFolder?: string): Promise<string> {
    if (this.isElectron) {
      return await window.electronAPI!.createBackup(destinationFolder);
    }
    return 'browser-storage-backup-ok';
  }

  public async restoreBackup(filePath: string): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.restoreBackup(filePath);
    }
  }

  public async exportExcel(request: ReportExportRequest, outputPath: string): Promise<void> {
    if (this.isElectron) {
      await window.electronAPI!.exportExcel(request, outputPath);
    }
  }

  public async showSaveDialog(options: any): Promise<{ canceled: boolean; filePath?: string }> {
    if (this.isElectron) {
      return await window.electronAPI!.showSaveDialog(options);
    }
    return { canceled: true };
  }

  public async showOpenDialog(options: any): Promise<{ canceled: boolean; filePaths: string[] }> {
    if (this.isElectron) {
      return await window.electronAPI!.showOpenDialog(options);
    }
    return { canceled: true, filePaths: [] };
  }

  public minimize(): void {
    if (this.isElectron) window.electronAPI!.minimizeWindow();
  }

  public maximize(): void {
    if (this.isElectron) window.electronAPI!.maximizeWindow();
  }

  public close(): void {
    if (this.isElectron) window.electronAPI!.closeWindow();
  }
}
