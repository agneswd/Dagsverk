import { Injectable, computed, inject, signal } from '@angular/core';
import {
  AppPreferences,
  AppSettings,
  DEFAULT_PREFERENCES,
  DEFAULT_SETTINGS,
  DEFAULT_WORKSPACE,
  MonthlySummary,
  MonthRecord,
  MonthViewPreference,
  Project,
  ReportExportRequest,
  TaxEstimate,
  ThemePreference,
  WorkEntry,
  WorkEntryStatus,
  Workspace
} from './models';
import { ElectronBridgeService } from './electron-bridge.service';
import { SwedishHolidayService } from './swedish-holiday.service';
import { TaxCalculatorService } from './tax-calculator.service';
import { MonthlyCalculations } from './monthly-calculations';

@Injectable({
  providedIn: 'root'
})
export class AppStateService {
  private bridge = inject(ElectronBridgeService);
  private holidays = inject(SwedishHolidayService);
  private taxCalculator = inject(TaxCalculatorService);

  // Global Multi-Tenancy & Preferences Signals
  public workspaces = signal<Workspace[]>([DEFAULT_WORKSPACE]);
  public activeWorkspaceId = signal<string>('ws-default');
  public preferences = signal<AppPreferences>(DEFAULT_PREFERENCES);

  // Scoped State Signals
  public settings = signal<AppSettings>(DEFAULT_SETTINGS);
  public projects = signal<Project[]>([]);
  public currentYear = signal<number>(new Date().getFullYear());
  public currentMonth = signal<number>(new Date().getMonth() + 1); // 1-12
  public monthRecord = signal<MonthRecord>({
    workspaceId: 'ws-default',
    year: new Date().getFullYear(),
    month: new Date().getMonth() + 1,
    openingBalanceMinutes: 0,
    expectedMinutesOverride: null,
    openingBalanceWasEdited: false
  });
  public entries = signal<WorkEntry[]>([]);
  public selectedDate = signal<string | null>(null);
  public isEditorOpen = signal<boolean>(false);
  public activeView = signal<MonthViewPreference>(MonthViewPreference.Ledger);
  public isDarkTheme = signal<boolean>(false);
  public isInitialized = signal<boolean>(false);

  // Computed Signals
  public activeWorkspace = computed<Workspace>(() => {
    const list = this.workspaces();
    const active = list.find(w => w.id === this.activeWorkspaceId());
    return active || list[0] || DEFAULT_WORKSPACE;
  });

  public todayString = computed<string>(() => {
    const d = new Date();
    const y = d.getFullYear();
    const m = String(d.getMonth() + 1).padStart(2, '0');
    const day = String(d.getDate()).padStart(2, '0');
    return `${y}-${m}-${day}`;
  });

  public formattedMonthTitle = computed<string>(() => {
    const d = new Date(Date.UTC(this.currentYear(), this.currentMonth() - 1, 1));
    const lang = this.preferences().languagePreference === 2 ? 'sv-SE' : 'en-US';
    return d.toLocaleDateString(lang, { month: 'long', year: 'numeric' });
  });

  public selectedEntry = computed<WorkEntry | null>(() => {
    const date = this.selectedDate();
    if (!date) return null;
    const found = this.entries().find(e => e.date === date);
    if (found) return found;

    return {
      workspaceId: this.activeWorkspaceId(),
      date,
      status: WorkEntryStatus.Incomplete,
      startTime: this.settings().defaultStartTime || '08:00',
      endTime: this.settings().defaultEndTime || '16:30',
      lunchMinutes: this.settings().defaultLunchMinutes ?? 30,
      projectName: this.settings().defaultProject || 'General',
      notes: null,
      scheduledMinutesOverride: null
    };
  });

  public summary = computed<MonthlySummary>(() => {
    return MonthlyCalculations.calculateMonthlySummary(
      this.monthRecord(),
      this.entries(),
      this.settings().expectedHours,
      this.settings().salary,
      this.settings().overtimeCompensation,
      this.holidays,
      this.todayString()
    );
  });

  public taxEstimate = computed<TaxEstimate>(() => {
    const gross = this.summary().grossSalary;
    return this.taxCalculator.calculate(gross, this.settings().taxSettings);
  });

  public missingDaysCount = computed<number>(() => {
    return this.summary().missingPastDays.length;
  });

  public constructor() {
    this.init();
  }

  public async init(): Promise<void> {
    try {
      // 1. Load workspaces and global preferences
      const [wsList, prefs] = await Promise.all([
        this.bridge.getWorkspaces(),
        this.bridge.getAppPreferences()
      ]);

      this.workspaces.set(wsList);
      this.preferences.set(prefs);
      const activeWsId = prefs.activeWorkspaceId || wsList[0]?.id || 'ws-default';
      this.activeWorkspaceId.set(activeWsId);
      this.activeView.set(prefs.monthViewPreference ?? MonthViewPreference.Ledger);

      // Set initial theme from global preferences
      const prefersDark = typeof window !== 'undefined' && window.matchMedia && window.matchMedia('(prefers-color-scheme: dark)').matches;
      const isDark = prefs.themePreference === ThemePreference.Dark || (prefs.themePreference === ThemePreference.System && prefersDark);
      this.setTheme(isDark);

      // 2. Load workspace-scoped data
      await this.loadWorkspaceData(activeWsId);
      this.isInitialized.set(true);
    } catch (err) {
      console.error('Failed to initialize AppState:', err);
    }
  }

  public async switchWorkspace(workspaceId: string): Promise<void> {
    if (this.activeWorkspaceId() === workspaceId) return;
    this.activeWorkspaceId.set(workspaceId);

    const updatedPrefs: AppPreferences = {
      ...this.preferences(),
      activeWorkspaceId: workspaceId
    };
    this.preferences.set(updatedPrefs);
    await this.bridge.saveAppPreferences(updatedPrefs);

    await this.loadWorkspaceData(workspaceId);
  }

  private async loadWorkspaceData(workspaceId: string): Promise<void> {
    const s = await this.bridge.getSettings(workspaceId);
    this.settings.set(s);

    const projs = await this.bridge.getProjects(workspaceId);
    this.projects.set(projs);

    const taxYear = s.taxSettings?.taxYear || 2026;
    await this.taxCalculator.loadTaxYear(taxYear);

    await this.loadCurrentMonth();
  }

  public async loadCurrentMonth(): Promise<void> {
    const wsId = this.activeWorkspaceId();
    const y = this.currentYear();
    const m = this.currentMonth();
    const defaultOpening = this.settings().openingBalanceMinutes || 0;

    const [monthRec, monthEntries] = await Promise.all([
      this.bridge.getMonthRecord(y, m, defaultOpening, wsId),
      this.bridge.getWorkEntries(y, m, wsId)
    ]);

    this.monthRecord.set(monthRec);
    this.entries.set(monthEntries);
  }

  public async selectMonth(year: number, month: number): Promise<void> {
    this.currentYear.set(year);
    this.currentMonth.set(month);
    this.closeEditor();
    await this.loadCurrentMonth();
  }

  public async nextMonth(): Promise<void> {
    let y = this.currentYear();
    let m = this.currentMonth() + 1;
    if (m > 12) {
      m = 1;
      y++;
    }
    await this.selectMonth(y, m);
  }

  public async previousMonth(): Promise<void> {
    let y = this.currentYear();
    let m = this.currentMonth() - 1;
    if (m < 1) {
      m = 12;
      y--;
    }
    await this.selectMonth(y, m);
  }

  public async goToToday(): Promise<void> {
    const d = new Date();
    await this.selectMonth(d.getFullYear(), d.getMonth() + 1);
    this.openEditor(this.todayString());
  }

  public openEditor(dateStr: string): void {
    this.selectedDate.set(dateStr);
    this.isEditorOpen.set(true);
  }

  public closeEditor(): void {
    this.isEditorOpen.set(false);
    this.selectedDate.set(null);
  }

  public async saveEntry(entry: WorkEntry): Promise<void> {
    const wsId = this.activeWorkspaceId();
    const scopedEntry = { ...entry, workspaceId: wsId };
    await this.bridge.saveWorkEntry(scopedEntry, wsId);
    const updated = this.entries().filter(e => e.date !== entry.date);
    updated.push(scopedEntry);
    this.entries.set([...updated]);
  }

  public async deleteEntry(dateStr: string): Promise<void> {
    const wsId = this.activeWorkspaceId();
    await this.bridge.deleteWorkEntry(dateStr, wsId);
    const updated = this.entries().filter(e => e.date !== dateStr);
    this.entries.set([...updated]);
    this.closeEditor();
  }

  public async saveMonthRecord(record: MonthRecord): Promise<void> {
    const wsId = this.activeWorkspaceId();
    const scopedRec = { ...record, workspaceId: wsId };
    this.monthRecord.set(scopedRec);
    await this.bridge.saveMonthRecord(scopedRec, wsId);
  }

  public async updateSettings(settings: AppSettings): Promise<void> {
    const wsId = this.activeWorkspaceId();
    const scopedSettings = { ...settings, workspaceId: wsId };
    this.settings.set(scopedSettings);
    await this.bridge.saveSettings(scopedSettings, wsId);

    if (settings.taxSettings?.taxYear) {
      await this.taxCalculator.loadTaxYear(settings.taxSettings.taxYear);
    }
  }

  public async saveProject(project: Project): Promise<void> {
    const wsId = this.activeWorkspaceId();
    const scopedProj = { ...project, workspaceId: wsId };
    await this.bridge.saveProject(scopedProj, wsId);
    const list = this.projects().filter(p => p.id !== project.id);
    list.push(scopedProj);
    this.projects.set([...list]);
  }

  public async deleteProject(id: string): Promise<void> {
    const wsId = this.activeWorkspaceId();
    await this.bridge.deleteProject(id, wsId);
    const list = this.projects().filter(p => p.id !== id);
    this.projects.set([...list]);
  }

  public async saveWorkspace(ws: Workspace): Promise<void> {
    await this.bridge.saveWorkspace(ws);
    const list = this.workspaces().filter(item => item.id !== ws.id);
    list.push(ws);
    this.workspaces.set([...list]);
  }

  public async deleteWorkspace(id: string): Promise<void> {
    if (this.workspaces().length <= 1) {
      throw new Error('Cannot delete the only remaining workspace');
    }
    await this.bridge.deleteWorkspace(id);
    const list = this.workspaces().filter(item => item.id !== id);
    this.workspaces.set([...list]);

    if (this.activeWorkspaceId() === id) {
      await this.switchWorkspace(list[0].id);
    }
  }

  public setTheme(dark: boolean): void {
    this.isDarkTheme.set(dark);
    if (typeof document !== 'undefined') {
      if (dark) {
        document.documentElement.classList.add('dark-theme');
        document.body.classList.add('dark-theme');
      } else {
        document.documentElement.classList.remove('dark-theme');
        document.body.classList.remove('dark-theme');
      }
    }
  }

  public toggleTheme(): void {
    const next = !this.isDarkTheme();
    this.setTheme(next);
    const updated: AppPreferences = {
      ...this.preferences(),
      themePreference: next ? ThemePreference.Dark : ThemePreference.Light
    };
    this.preferences.set(updated);
    this.bridge.saveAppPreferences(updated);
  }

  public setView(view: MonthViewPreference): void {
    this.activeView.set(view);
    const updated: AppPreferences = {
      ...this.preferences(),
      monthViewPreference: view
    };
    this.preferences.set(updated);
    this.bridge.saveAppPreferences(updated);
  }

  public async quickCatchUp(): Promise<void> {
    const missing = this.summary().missingPastDays;
    if (missing.length === 0) return;

    const defaultStart = this.settings().defaultStartTime || '08:00';
    const defaultEnd = this.settings().defaultEndTime || '16:30';
    const defaultLunch = this.settings().defaultLunchMinutes ?? 30;
    const defaultProj = this.settings().defaultProject || 'General';

    for (const date of missing) {
      const entry: WorkEntry = {
        workspaceId: this.activeWorkspaceId(),
        date,
        status: WorkEntryStatus.Worked,
        startTime: defaultStart,
        endTime: defaultEnd,
        lunchMinutes: defaultLunch,
        projectName: defaultProj,
        notes: null,
        scheduledMinutesOverride: null
      };
      await this.saveEntry(entry);
    }
  }

  public async exportExcel(): Promise<void> {
    const req: ReportExportRequest = {
      year: this.currentYear(),
      month: this.currentMonth(),
      employeeName: this.settings().employeeName || 'Employee',
      employerName: this.settings().employerName || 'Employer',
      entries: this.entries(),
      summary: this.summary(),
      language: this.settings().exportLanguagePreference ?? 2,
      overtimeMode: this.settings().overtimeCompensation.mode,
      dailyOvertimeThresholdHours: this.settings().overtimeCompensation.dailyThresholdHours
    };

    const monthStr = String(this.currentMonth()).padStart(2, '0');
    const safeName = (this.settings().employeeName || 'report').replace(/[^a-zA-Z0-9_-]/g, '_');
    const defaultFilename = `Dagsverk_${safeName}_${this.currentYear()}-${monthStr}.xlsx`;

    const res = await this.bridge.showSaveDialog({
      title: 'Export Timesheet Report',
      defaultPath: defaultFilename,
      filters: [{ name: 'Excel Workbook', extensions: ['xlsx'] }]
    });

    if (!res.canceled && res.filePath) {
      await this.bridge.exportExcel(req, res.filePath);
    }
  }
}
