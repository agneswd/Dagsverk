import { Injectable, computed, inject, signal } from '@angular/core';
import {
  AppPreferences,
  AppSettings,
  BalanceHistoryMonth,
  DEFAULT_PREFERENCES,
  DEFAULT_SETTINGS,
  DEFAULT_WORKSPACE,
  MonthlySummary,
  MonthRecord,
  MonthViewPreference,
  Project,
  ReportExportRequest,
  ExportLanguagePreference,
  TaxEstimate,
  ThemePreference,
  WorkEntry,
  WorkEntryStatus,
  Workspace,
} from './models';
import { ElectronBridgeService } from './electron-bridge.service';
import { SwedishHolidayService } from './swedish-holiday.service';
import { TaxCalculatorService } from './tax-calculator.service';
import { MonthlyCalculations } from './monthly-calculations';
import { LocalizationService } from './localization.service';

@Injectable({
  providedIn: 'root',
})
export class AppStateService {
  private bridge = inject(ElectronBridgeService);
  private holidays = inject(SwedishHolidayService);
  private taxCalculator = inject(TaxCalculatorService);
  private localization = inject(LocalizationService);

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
    openingBalanceWasEdited: false,
  });
  public entries = signal<WorkEntry[]>([]);
  public selectedDate = signal<string | null>(null);
  public isEditorOpen = signal<boolean>(false);
  public activeView = signal<MonthViewPreference>(MonthViewPreference.Ledger);
  public isDarkTheme = signal<boolean>(false);
  public isInitialized = signal<boolean>(false);
  public catchUpDates = signal<string[]>([]);
  public catchUpIndex = signal<number>(0);

  // Computed Signals
  public activeWorkspace = computed<Workspace>(() => {
    const list = this.workspaces();
    const active = list.find((w) => w.id === this.activeWorkspaceId());
    return active || list[0] || DEFAULT_WORKSPACE;
  });

  public isCatchUpOpen = computed(() => this.catchUpDates().length > 0);
  public isMonthUnstarted = computed(
    () => !this.entries().some((entry) => entry.status !== WorkEntryStatus.Incomplete),
  );
  public catchUpProgress = computed(() => {
    const count = this.catchUpDates().length;
    return count ? `${this.catchUpIndex() + 1} of ${count}` : '';
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
    const lang = this.localization.language() === 'sv' ? 'sv-SE' : 'en-US';
    return d.toLocaleDateString(lang, { month: 'long', year: 'numeric' });
  });

  public selectedEntry = computed<WorkEntry | null>(() => {
    const date = this.selectedDate();
    if (!date) return null;
    const found = this.entries().find((e) => e.date === date);
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
      scheduledMinutesOverride: null,
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
      this.todayString(),
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
    if (typeof window !== 'undefined') {
      window.matchMedia?.('(prefers-color-scheme: dark)').addEventListener('change', (event) => {
        if (this.preferences().themePreference === ThemePreference.System)
          this.setTheme(event.matches);
      });
    }
    this.init();
  }

  public async init(): Promise<void> {
    try {
      // 1. Load workspaces and global preferences
      const [wsList, prefs] = await Promise.all([
        this.bridge.getWorkspaces(),
        this.bridge.getAppPreferences(),
      ]);

      this.workspaces.set(wsList);
      this.preferences.set(prefs);
      const activeWsId = prefs.activeWorkspaceId || wsList[0]?.id || 'ws-default';
      this.activeWorkspaceId.set(activeWsId);
      this.activeView.set(prefs.monthViewPreference ?? MonthViewPreference.Ledger);
      this.applyPreferences(prefs);

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
      activeWorkspaceId: workspaceId,
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
    const defaultOpening = await this.estimateOpeningBalance(y, m);

    const [monthRec, monthEntries] = await Promise.all([
      this.bridge.getMonthRecord(y, m, defaultOpening, wsId),
      this.bridge.getWorkEntries(y, m, wsId),
    ]);

    this.monthRecord.set(monthRec);
    this.entries.set(monthEntries);
  }

  private async estimateOpeningBalance(year: number, month: number): Promise<number> {
    const history = await this.bridge.getBalanceHistory(year, month, this.activeWorkspaceId());
    let opening = this.settings().openingBalanceMinutes || 0;
    let lastEdited = -1;
    for (let index = history.length - 1; index >= 0; index--) {
      if (history[index].record?.openingBalanceWasEdited) {
        lastEdited = index;
        break;
      }
    }
    const relevant = lastEdited >= 0 ? history.slice(lastEdited) : history;

    for (const item of relevant) {
      if (item.record?.openingBalanceWasEdited) opening = item.record.openingBalanceMinutes;
      if (!item.entries.some((entry) => entry.status !== WorkEntryStatus.Incomplete)) continue;
      const record = item.record || this.newHistoryRecord(item, opening);
      const summary = MonthlyCalculations.calculateMonthlySummary(
        {
          ...record,
          openingBalanceMinutes: record.openingBalanceWasEdited
            ? record.openingBalanceMinutes
            : opening,
        },
        item.entries,
        this.settings().expectedHours,
        this.settings().salary,
        this.settings().overtimeCompensation,
        this.holidays,
        this.todayString(),
      );
      opening = summary.closingBalanceMinutes;
    }
    return opening;
  }

  private newHistoryRecord(item: BalanceHistoryMonth, openingBalanceMinutes: number): MonthRecord {
    return {
      workspaceId: this.activeWorkspaceId(),
      year: item.year,
      month: item.month,
      openingBalanceMinutes,
      expectedMinutesOverride: null,
      openingBalanceWasEdited: false,
    };
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

  public startMonth(): void {
    const today = new Date();
    const isCurrentMonth =
      today.getFullYear() === this.currentYear() && today.getMonth() + 1 === this.currentMonth();
    const target = isCurrentMonth
      ? this.todayString()
      : MonthlyCalculations.getExpectedWorkdays(
          this.currentYear(),
          this.currentMonth(),
          this.settings().expectedHours,
          this.holidays,
        )[0] || `${this.currentYear()}-${String(this.currentMonth()).padStart(2, '0')}-01`;
    this.openEditor(target);
  }

  public closeEditor(): void {
    this.isEditorOpen.set(false);
    this.selectedDate.set(null);
  }

  public async saveEntry(entry: WorkEntry): Promise<void> {
    const wsId = this.activeWorkspaceId();
    const scopedEntry = { ...entry, workspaceId: wsId };
    await this.bridge.saveWorkEntry(scopedEntry, wsId);
    const updated = this.entries().filter((e) => e.date !== entry.date);
    updated.push(scopedEntry);
    this.entries.set([...updated]);
  }

  public async deleteEntry(dateStr: string): Promise<void> {
    const wsId = this.activeWorkspaceId();
    await this.bridge.deleteWorkEntry(dateStr, wsId);
    const updated = this.entries().filter((e) => e.date !== dateStr);
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
    const list = this.projects().filter((p) => p.id !== project.id);
    list.push(scopedProj);
    this.projects.set([...list]);
  }

  public async deleteProject(id: string): Promise<void> {
    const wsId = this.activeWorkspaceId();
    await this.bridge.deleteProject(id, wsId);
    const list = this.projects().filter((p) => p.id !== id);
    this.projects.set([...list]);
  }

  public async saveWorkspace(ws: Workspace): Promise<void> {
    await this.bridge.saveWorkspace(ws);
    const list = this.workspaces().filter((item) => item.id !== ws.id);
    list.push(ws);
    this.workspaces.set([...list]);
  }

  public async deleteWorkspace(id: string): Promise<void> {
    if (this.workspaces().length <= 1) {
      throw new Error('Cannot delete the only remaining workspace');
    }
    await this.bridge.deleteWorkspace(id);
    const list = this.workspaces().filter((item) => item.id !== id);
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
      themePreference: next ? ThemePreference.Dark : ThemePreference.Light,
    };
    this.preferences.set(updated);
    this.bridge.saveAppPreferences(updated);
  }

  public async updatePreferences(preferences: AppPreferences): Promise<void> {
    this.preferences.set(preferences);
    this.activeView.set(preferences.monthViewPreference);
    this.applyPreferences(preferences);
    await this.bridge.saveAppPreferences(preferences);
  }

  private applyPreferences(preferences: AppPreferences): void {
    const prefersDark =
      typeof window !== 'undefined' && window.matchMedia?.('(prefers-color-scheme: dark)').matches;
    this.setTheme(
      preferences.themePreference === ThemePreference.Dark ||
        (preferences.themePreference === ThemePreference.System && Boolean(prefersDark)),
    );
    this.localization.setPreference(preferences.languagePreference);
    if (typeof document !== 'undefined') {
      document.body.style.zoom = `${preferences.interfaceScalePercent || 100}%`;
    }
  }

  public setView(view: MonthViewPreference): void {
    this.activeView.set(view);
    const updated: AppPreferences = {
      ...this.preferences(),
      monthViewPreference: view,
    };
    this.preferences.set(updated);
    this.bridge.saveAppPreferences(updated);
  }

  public startCatchUp(): void {
    const dates = [...this.summary().missingPastDays];
    if (!dates.length) return;
    this.catchUpDates.set(dates);
    this.catchUpIndex.set(0);
    this.openEditor(dates[0]);
  }

  public moveCatchUp(delta: number): void {
    const next = this.catchUpIndex() + delta;
    if (next < 0) return;
    if (next >= this.catchUpDates().length) {
      this.closeCatchUp();
      return;
    }
    this.catchUpIndex.set(next);
    this.openEditor(this.catchUpDates()[next]);
  }

  public closeCatchUp(): void {
    this.catchUpDates.set([]);
    this.catchUpIndex.set(0);
    this.closeEditor();
  }

  public async exportExcel(): Promise<void> {
    const workspace = this.activeWorkspace();
    const req: ReportExportRequest = {
      year: this.currentYear(),
      month: this.currentMonth(),
      employeeName: workspace.workerName || 'Worker',
      employerName: workspace.organizationName || '',
      entries: this.entries(),
      summary: this.summary(),
      language:
        this.settings().exportLanguagePreference === ExportLanguagePreference.System
          ? this.localization.language() === 'en'
            ? ExportLanguagePreference.English
            : ExportLanguagePreference.Swedish
          : this.settings().exportLanguagePreference,
      overtimeMode: this.settings().overtimeCompensation.mode,
      dailyOvertimeThresholdHours: this.settings().overtimeCompensation.dailyThresholdHours,
    };

    const monthStr = String(this.currentMonth()).padStart(2, '0');
    const safeName = (workspace.workerName || workspace.name || 'report').replace(
      /[^a-zA-Z0-9_-]/g,
      '_',
    );
    const defaultFilename = `Dagsverk_${safeName}_${this.currentYear()}-${monthStr}.xlsx`;

    const res = await this.bridge.showSaveDialog({
      title: 'Export Timesheet Report',
      defaultPath: defaultFilename,
      filters: [{ name: 'Excel Workbook', extensions: ['xlsx'] }],
    });

    if (!res.canceled && res.filePath) {
      await this.bridge.exportExcel(req, res.filePath);
    }
  }
}
