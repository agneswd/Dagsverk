import Database from 'better-sqlite3';
import * as path from 'path';
import * as fs from 'fs';
import { app } from 'electron';

export class DatabaseService {
  private db: Database.Database;
  private dbPath: string;

  constructor(customPath?: string) {
    if (customPath) {
      this.dbPath = customPath;
    } else {
      const userData = app ? app.getPath('userData') : path.join(process.cwd(), 'data');
      if (!fs.existsSync(userData)) {
        fs.mkdirSync(userData, { recursive: true });
      }
      this.dbPath = path.join(userData, 'dagsverk.db');
    }

    this.db = new Database(this.dbPath);
    this.db.pragma('journal_mode = WAL');
    this.db.pragma('foreign_keys = ON');
    this.initSchema();
  }

  public getDatabasePath(): string {
    return this.dbPath;
  }

  private initSchema(): void {
    const tableExists = this.db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='Workspaces'").get();

    if (!tableExists) {
      // Check if legacy table exists
      const legacyWorkEntries = this.db.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='WorkEntries'").get();

      if (legacyWorkEntries) {
        this.migrateFromLegacySchema();
        return;
      }

      this.db.exec(`
        CREATE TABLE Workspaces (
          Id TEXT PRIMARY KEY,
          Name TEXT NOT NULL,
          Color TEXT NOT NULL,
          EmployerName TEXT NOT NULL DEFAULT '',
          CreatedAt TEXT NOT NULL,
          UpdatedAt TEXT NOT NULL
        );

        CREATE TABLE AppPreferences (
          Id INTEGER PRIMARY KEY CHECK (Id = 1),
          ActiveWorkspaceId TEXT NOT NULL,
          ThemePreference INTEGER NOT NULL DEFAULT 0,
          LanguagePreference INTEGER NOT NULL DEFAULT 0,
          InterfaceScalePercent INTEGER NOT NULL DEFAULT 100,
          MonthViewPreference INTEGER NOT NULL DEFAULT 0,
          FOREIGN KEY (ActiveWorkspaceId) REFERENCES Workspaces(Id) ON DELETE RESTRICT
        );

        CREATE TABLE WorkspaceSettings (
          WorkspaceId TEXT PRIMARY KEY,
          EmployeeName TEXT NOT NULL DEFAULT 'Agnes Larsson',
          EmployerName TEXT NOT NULL DEFAULT 'Acme AB',
          DefaultProject TEXT NOT NULL DEFAULT 'General',
          HourlyRate DECIMAL NOT NULL DEFAULT 250,
          SalaryType INTEGER NOT NULL DEFAULT 0,
          MonthlySalary DECIMAL NOT NULL DEFAULT 40000,
          EmploymentPercent DECIMAL NOT NULL DEFAULT 100,
          ExpectedHoursPerWorkday DECIMAL NOT NULL DEFAULT 8,
          ExpectedWorkingWeekdays TEXT NOT NULL DEFAULT '1,2,3,4,5',
          ExcludePublicHolidays INTEGER NOT NULL DEFAULT 1,
          DefaultStartTime TEXT NOT NULL DEFAULT '08:00',
          DefaultEndTime TEXT NOT NULL DEFAULT '16:30',
          DefaultLunchMinutes INTEGER NOT NULL DEFAULT 30,
          TaxMode INTEGER NOT NULL DEFAULT 1,
          TaxYear INTEGER NOT NULL DEFAULT 2026,
          TaxTableNumber INTEGER NOT NULL DEFAULT 30,
          TaxColumn INTEGER NOT NULL DEFAULT 1,
          ManualTaxValue DECIMAL,
          OpeningBalanceMinutes INTEGER NOT NULL DEFAULT 0,
          CurrencyPreference INTEGER NOT NULL DEFAULT 0,
          ExportLanguagePreference INTEGER NOT NULL DEFAULT 2,
          OvertimeCompensationMode INTEGER NOT NULL DEFAULT 0,
          OvertimePremiumPercent DECIMAL NOT NULL DEFAULT 50,
          OvertimeDailyThresholdHours DECIMAL NOT NULL DEFAULT 8,
          OvertimeThresholdMode INTEGER NOT NULL DEFAULT 0,
          OvertimeDefaultRateType INTEGER NOT NULL DEFAULT 0,
          OvertimeRateBandsJson TEXT NOT NULL DEFAULT '[]',
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );

        CREATE TABLE WorkEntries (
          WorkspaceId TEXT NOT NULL,
          Date TEXT NOT NULL,
          Status INTEGER NOT NULL,
          StartTime TEXT,
          EndTime TEXT,
          LunchMinutes INTEGER NOT NULL DEFAULT 0,
          ProjectName TEXT,
          Notes TEXT,
          ScheduledMinutesOverride INTEGER,
          CreatedAt TEXT NOT NULL,
          UpdatedAt TEXT NOT NULL,
          PRIMARY KEY (WorkspaceId, Date),
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );

        CREATE TABLE MonthRecords (
          WorkspaceId TEXT NOT NULL,
          Year INTEGER NOT NULL,
          Month INTEGER NOT NULL,
          OpeningBalanceMinutes INTEGER NOT NULL DEFAULT 0,
          ExpectedMinutesOverride INTEGER,
          OpeningBalanceWasEdited INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (WorkspaceId, Year, Month),
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );

        CREATE TABLE Projects (
          WorkspaceId TEXT NOT NULL,
          Id TEXT NOT NULL,
          Name TEXT NOT NULL,
          Color TEXT,
          IsActive INTEGER NOT NULL DEFAULT 1,
          IsDefault INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (WorkspaceId, Id),
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );
      `);

      // Seed initial default workspace & preferences
      const now = new Date().toISOString();
      this.db.prepare(`
        INSERT INTO Workspaces (Id, Name, Color, EmployerName, CreatedAt, UpdatedAt)
        VALUES ('ws-default', 'Main Workspace', '#5F875F', 'Acme AB', ?, ?)
      `).run(now, now);

      this.db.prepare(`
        INSERT INTO AppPreferences (Id, ActiveWorkspaceId, ThemePreference, LanguagePreference, InterfaceScalePercent, MonthViewPreference)
        VALUES (1, 'ws-default', 0, 0, 100, 0)
      `).run();

      this.db.prepare(`
        INSERT INTO WorkspaceSettings (WorkspaceId)
        VALUES ('ws-default')
      `).run();

      this.db.prepare(`
        INSERT INTO Projects (WorkspaceId, Id, Name, Color, IsActive, IsDefault)
        VALUES ('ws-default', 'proj-default', 'General', '#5F875F', 1, 1)
      `).run();
    }
  }

  private migrateFromLegacySchema(): void {
    const now = new Date().toISOString();
    this.db.pragma('foreign_keys = OFF');

    const migrateTx = this.db.transaction(() => {
      // 1. Rename existing legacy tables
      this.db.exec(`
        ALTER TABLE WorkEntries RENAME TO Old_WorkEntries;
        ALTER TABLE MonthRecords RENAME TO Old_MonthRecords;
        ALTER TABLE Projects RENAME TO Old_Projects;
        ALTER TABLE Settings RENAME TO Old_Settings;
      `);

      // 2. Create new schema
      this.db.exec(`
        CREATE TABLE Workspaces (
          Id TEXT PRIMARY KEY,
          Name TEXT NOT NULL,
          Color TEXT NOT NULL,
          EmployerName TEXT NOT NULL DEFAULT '',
          CreatedAt TEXT NOT NULL,
          UpdatedAt TEXT NOT NULL
        );

        CREATE TABLE AppPreferences (
          Id INTEGER PRIMARY KEY CHECK (Id = 1),
          ActiveWorkspaceId TEXT NOT NULL,
          ThemePreference INTEGER NOT NULL DEFAULT 0,
          LanguagePreference INTEGER NOT NULL DEFAULT 0,
          InterfaceScalePercent INTEGER NOT NULL DEFAULT 100,
          MonthViewPreference INTEGER NOT NULL DEFAULT 0,
          FOREIGN KEY (ActiveWorkspaceId) REFERENCES Workspaces(Id) ON DELETE RESTRICT
        );

        CREATE TABLE WorkspaceSettings (
          WorkspaceId TEXT PRIMARY KEY,
          EmployeeName TEXT NOT NULL DEFAULT 'Agnes Larsson',
          EmployerName TEXT NOT NULL DEFAULT 'Acme AB',
          DefaultProject TEXT NOT NULL DEFAULT 'General',
          HourlyRate DECIMAL NOT NULL DEFAULT 250,
          SalaryType INTEGER NOT NULL DEFAULT 0,
          MonthlySalary DECIMAL NOT NULL DEFAULT 40000,
          EmploymentPercent DECIMAL NOT NULL DEFAULT 100,
          ExpectedHoursPerWorkday DECIMAL NOT NULL DEFAULT 8,
          ExpectedWorkingWeekdays TEXT NOT NULL DEFAULT '1,2,3,4,5',
          ExcludePublicHolidays INTEGER NOT NULL DEFAULT 1,
          DefaultStartTime TEXT NOT NULL DEFAULT '08:00',
          DefaultEndTime TEXT NOT NULL DEFAULT '16:30',
          DefaultLunchMinutes INTEGER NOT NULL DEFAULT 30,
          TaxMode INTEGER NOT NULL DEFAULT 1,
          TaxYear INTEGER NOT NULL DEFAULT 2026,
          TaxTableNumber INTEGER NOT NULL DEFAULT 30,
          TaxColumn INTEGER NOT NULL DEFAULT 1,
          ManualTaxValue DECIMAL,
          OpeningBalanceMinutes INTEGER NOT NULL DEFAULT 0,
          CurrencyPreference INTEGER NOT NULL DEFAULT 0,
          ExportLanguagePreference INTEGER NOT NULL DEFAULT 2,
          OvertimeCompensationMode INTEGER NOT NULL DEFAULT 0,
          OvertimePremiumPercent DECIMAL NOT NULL DEFAULT 50,
          OvertimeDailyThresholdHours DECIMAL NOT NULL DEFAULT 8,
          OvertimeThresholdMode INTEGER NOT NULL DEFAULT 0,
          OvertimeDefaultRateType INTEGER NOT NULL DEFAULT 0,
          OvertimeRateBandsJson TEXT NOT NULL DEFAULT '[]',
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );

        CREATE TABLE WorkEntries (
          WorkspaceId TEXT NOT NULL,
          Date TEXT NOT NULL,
          Status INTEGER NOT NULL,
          StartTime TEXT,
          EndTime TEXT,
          LunchMinutes INTEGER NOT NULL DEFAULT 0,
          ProjectName TEXT,
          Notes TEXT,
          ScheduledMinutesOverride INTEGER,
          CreatedAt TEXT NOT NULL,
          UpdatedAt TEXT NOT NULL,
          PRIMARY KEY (WorkspaceId, Date),
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );

        CREATE TABLE MonthRecords (
          WorkspaceId TEXT NOT NULL,
          Year INTEGER NOT NULL,
          Month INTEGER NOT NULL,
          OpeningBalanceMinutes INTEGER NOT NULL DEFAULT 0,
          ExpectedMinutesOverride INTEGER,
          OpeningBalanceWasEdited INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (WorkspaceId, Year, Month),
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );

        CREATE TABLE Projects (
          WorkspaceId TEXT NOT NULL,
          Id TEXT NOT NULL,
          Name TEXT NOT NULL,
          Color TEXT,
          IsActive INTEGER NOT NULL DEFAULT 1,
          IsDefault INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (WorkspaceId, Id),
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );
      `);

      // 3. Create default workspace
      this.db.prepare(`
        INSERT INTO Workspaces (Id, Name, Color, EmployerName, CreatedAt, UpdatedAt)
        VALUES ('ws-default', 'Main Workspace', '#5F875F', 'Acme AB', ?, ?)
      `).run(now, now);

      // 4. Copy old settings into AppPreferences & WorkspaceSettings
      const oldSettings = this.db.prepare('SELECT * FROM Old_Settings WHERE Id = 1').get() as any;
      if (oldSettings) {
        this.db.prepare(`
          INSERT INTO AppPreferences (Id, ActiveWorkspaceId, ThemePreference, LanguagePreference, InterfaceScalePercent, MonthViewPreference)
          VALUES (1, 'ws-default', ?, ?, ?, ?)
        `).run(
          oldSettings.ThemePreference || 0,
          oldSettings.LanguagePreference || 0,
          oldSettings.InterfaceScalePercent || 100,
          oldSettings.MonthViewPreference || 0
        );

        this.db.prepare(`
          INSERT INTO WorkspaceSettings (
            WorkspaceId, EmployeeName, EmployerName, DefaultProject, HourlyRate,
            SalaryType, MonthlySalary, EmploymentPercent, ExpectedHoursPerWorkday,
            ExpectedWorkingWeekdays, ExcludePublicHolidays, DefaultStartTime, DefaultEndTime,
            DefaultLunchMinutes, TaxMode, TaxYear, TaxTableNumber, TaxColumn, ManualTaxValue,
            OpeningBalanceMinutes, CurrencyPreference, ExportLanguagePreference,
            OvertimeCompensationMode, OvertimePremiumPercent, OvertimeDailyThresholdHours,
            OvertimeThresholdMode, OvertimeDefaultRateType, OvertimeRateBandsJson
          ) VALUES (
            'ws-default', ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?,
            ?, ?, ?, ?, ?, ?,
            ?, ?, ?,
            ?, ?, ?,
            ?, ?, ?
          )
        `).run(
          oldSettings.EmployeeName || 'Agnes Larsson',
          oldSettings.EmployerName || 'Acme AB',
          oldSettings.DefaultProject || 'General',
          oldSettings.HourlyRate || 250,
          oldSettings.SalaryType || 0,
          oldSettings.MonthlySalary || 40000,
          oldSettings.EmploymentPercent || 100,
          oldSettings.ExpectedHoursPerWorkday || 8,
          oldSettings.ExpectedWorkingWeekdays || '1,2,3,4,5',
          oldSettings.ExcludePublicHolidays !== undefined ? oldSettings.ExcludePublicHolidays : 1,
          oldSettings.DefaultStartTime || '08:00',
          oldSettings.DefaultEndTime || '16:30',
          oldSettings.DefaultLunchMinutes || 30,
          oldSettings.TaxMode || 1,
          oldSettings.TaxYear || 2026,
          oldSettings.TaxTableNumber || 30,
          oldSettings.TaxColumn || 1,
          oldSettings.ManualTaxValue,
          oldSettings.OpeningBalanceMinutes || 0,
          oldSettings.CurrencyPreference || 0,
          oldSettings.ExportLanguagePreference || 2,
          oldSettings.OvertimeCompensationMode || 0,
          oldSettings.OvertimePremiumPercent || 50,
          oldSettings.OvertimeDailyThresholdHours || 8,
          oldSettings.OvertimeThresholdMode || 0,
          oldSettings.OvertimeDefaultRateType || 0,
          oldSettings.OvertimeRateBandsJson || '[]'
        );
      }

      // 5. Copy WorkEntries
      this.db.exec(`
        INSERT INTO WorkEntries (WorkspaceId, Date, Status, StartTime, EndTime, LunchMinutes, ProjectName, Notes, ScheduledMinutesOverride, CreatedAt, UpdatedAt)
        SELECT 'ws-default', Date, Status, StartTime, EndTime, LunchMinutes, ProjectName, Notes, ScheduledMinutesOverride, CreatedAt, UpdatedAt
        FROM Old_WorkEntries;
      `);

      // 6. Copy MonthRecords
      this.db.exec(`
        INSERT INTO MonthRecords (WorkspaceId, Year, Month, OpeningBalanceMinutes, ExpectedMinutesOverride, OpeningBalanceWasEdited)
        SELECT 'ws-default', Year, Month, OpeningBalanceMinutes, ExpectedMinutesOverride, OpeningBalanceWasEdited
        FROM Old_MonthRecords;
      `);

      // 7. Copy Projects
      this.db.exec(`
        INSERT INTO Projects (WorkspaceId, Id, Name, Color, IsActive, IsDefault)
        SELECT 'ws-default', Id, Name, '#5F875F', IsActive, IsDefault
        FROM Old_Projects;
      `);

      // 8. Drop old tables
      this.db.exec(`
        DROP TABLE Old_WorkEntries;
        DROP TABLE Old_MonthRecords;
        DROP TABLE Old_Projects;
        DROP TABLE Old_Settings;
      `);
    });

    migrateTx();
    this.db.pragma('foreign_keys = ON');
  }

  // --- Workspaces ---
  public getWorkspaces(): any[] {
    const rows = this.db.prepare('SELECT * FROM Workspaces ORDER BY CreatedAt ASC').all() as any[];
    return rows.map(r => ({
      id: r.Id,
      name: r.Name,
      color: r.Color,
      employerName: r.EmployerName,
      createdAt: r.CreatedAt,
      updatedAt: r.UpdatedAt
    }));
  }

  public saveWorkspace(ws: any): void {
    const now = new Date().toISOString();
    this.db.prepare(`
      INSERT INTO Workspaces (Id, Name, Color, EmployerName, CreatedAt, UpdatedAt)
      VALUES (?, ?, ?, ?, ?, ?)
      ON CONFLICT(Id) DO UPDATE SET
        Name = excluded.Name,
        Color = excluded.Color,
        EmployerName = excluded.EmployerName,
        UpdatedAt = excluded.UpdatedAt
    `).run(
      ws.id,
      ws.name,
      ws.color || '#5F875F',
      ws.employerName || '',
      ws.createdAt || now,
      now
    );

    // Ensure WorkspaceSettings exist
    const settingsExist = this.db.prepare('SELECT WorkspaceId FROM WorkspaceSettings WHERE WorkspaceId = ?').get(ws.id);
    if (!settingsExist) {
      this.db.prepare(`
        INSERT INTO WorkspaceSettings (WorkspaceId, EmployerName)
        VALUES (?, ?)
      `).run(ws.id, ws.employerName || '');
    }
  }

  public deleteWorkspace(id: string): void {
    const count = (this.db.prepare('SELECT COUNT(*) as cnt FROM Workspaces').get() as { cnt: number }).cnt;
    if (count <= 1) {
      throw new Error('Cannot delete the last remaining workspace');
    }
    this.db.prepare('DELETE FROM Workspaces WHERE Id = ?').run(id);
  }

  // --- AppPreferences ---
  public getAppPreferences(): any {
    const row = this.db.prepare('SELECT * FROM AppPreferences WHERE Id = 1').get() as any;
    if (!row) {
      return {
        activeWorkspaceId: 'ws-default',
        themePreference: 0,
        languagePreference: 0,
        interfaceScalePercent: 100,
        monthViewPreference: 0
      };
    }

    return {
      activeWorkspaceId: row.ActiveWorkspaceId,
      themePreference: row.ThemePreference,
      languagePreference: row.LanguagePreference,
      interfaceScalePercent: row.InterfaceScalePercent,
      monthViewPreference: row.MonthViewPreference
    };
  }

  public saveAppPreferences(prefs: any): void {
    this.db.prepare(`
      INSERT INTO AppPreferences (Id, ActiveWorkspaceId, ThemePreference, LanguagePreference, InterfaceScalePercent, MonthViewPreference)
      VALUES (1, ?, ?, ?, ?, ?)
      ON CONFLICT(Id) DO UPDATE SET
        ActiveWorkspaceId = excluded.ActiveWorkspaceId,
        ThemePreference = excluded.ThemePreference,
        LanguagePreference = excluded.LanguagePreference,
        InterfaceScalePercent = excluded.InterfaceScalePercent,
        MonthViewPreference = excluded.MonthViewPreference
    `).run(
      prefs.activeWorkspaceId || 'ws-default',
      prefs.themePreference || 0,
      prefs.languagePreference || 0,
      prefs.interfaceScalePercent || 100,
      prefs.monthViewPreference || 0
    );
  }

  // --- WorkspaceSettings ---
  public getSettings(workspaceId: string = 'ws-default'): any {
    let row = this.db.prepare('SELECT * FROM WorkspaceSettings WHERE WorkspaceId = ?').get(workspaceId) as any;
    if (!row) {
      this.db.prepare('INSERT INTO WorkspaceSettings (WorkspaceId) VALUES (?)').run(workspaceId);
      row = this.db.prepare('SELECT * FROM WorkspaceSettings WHERE WorkspaceId = ?').get(workspaceId) as any;
    }

    return {
      workspaceId: row.WorkspaceId,
      employeeName: row.EmployeeName,
      employerName: row.EmployerName,
      defaultProject: row.DefaultProject,
      salary: {
        type: row.SalaryType,
        hourlyRate: Number(row.HourlyRate),
        monthlySalary: Number(row.MonthlySalary),
        employmentPercent: Number(row.EmploymentPercent)
      },
      expectedHours: {
        hoursPerWorkday: Number(row.ExpectedHoursPerWorkday),
        workingWeekdays: row.ExpectedWorkingWeekdays.split(',').map(Number),
        excludePublicHolidays: Boolean(row.ExcludePublicHolidays)
      },
      defaultStartTime: row.DefaultStartTime,
      defaultEndTime: row.DefaultEndTime,
      defaultLunchMinutes: row.DefaultLunchMinutes,
      taxSettings: {
        mode: row.TaxMode,
        taxYear: row.TaxYear,
        tableNumber: row.TaxTableNumber,
        column: row.TaxColumn,
        manualMonthlyDeduction: row.ManualTaxValue !== null ? Number(row.ManualTaxValue) : null
      },
      openingBalanceMinutes: row.OpeningBalanceMinutes,
      currencyPreference: ['SEK', 'EUR', 'USD', 'GBP', 'NOK', 'DKK'][row.CurrencyPreference] || 'SEK',
      exportLanguagePreference: row.ExportLanguagePreference,
      overtimeCompensation: {
        mode: row.OvertimeCompensationMode,
        defaultRateType: row.OvertimeDefaultRateType,
        defaultRateValue: Number(row.OvertimePremiumPercent),
        dailyThresholdHours: Number(row.OvertimeDailyThresholdHours),
        thresholdMode: row.OvertimeThresholdMode,
        rateBands: JSON.parse(row.OvertimeRateBandsJson || '[]')
      }
    };
  }

  public saveSettings(settings: any, workspaceId: string = 'ws-default'): void {
    const currencyIdx = ['SEK', 'EUR', 'USD', 'GBP', 'NOK', 'DKK'].indexOf(settings.currencyPreference);
    const curr = currencyIdx >= 0 ? currencyIdx : 0;

    this.db.prepare(`
      INSERT INTO WorkspaceSettings (
        WorkspaceId, EmployeeName, EmployerName, DefaultProject, HourlyRate, SalaryType,
        MonthlySalary, EmploymentPercent, ExpectedHoursPerWorkday, ExpectedWorkingWeekdays,
        ExcludePublicHolidays, DefaultStartTime, DefaultEndTime, DefaultLunchMinutes,
        TaxMode, TaxYear, TaxTableNumber, TaxColumn, ManualTaxValue,
        OpeningBalanceMinutes, CurrencyPreference, ExportLanguagePreference,
        OvertimeCompensationMode, OvertimePremiumPercent, OvertimeDailyThresholdHours,
        OvertimeThresholdMode, OvertimeDefaultRateType, OvertimeRateBandsJson
      ) VALUES (
        ?, ?, ?, ?, ?, ?,
        ?, ?, ?, ?,
        ?, ?, ?, ?,
        ?, ?, ?, ?, ?,
        ?, ?, ?,
        ?, ?, ?,
        ?, ?, ?
      )
      ON CONFLICT(WorkspaceId) DO UPDATE SET
        EmployeeName = excluded.EmployeeName,
        EmployerName = excluded.EmployerName,
        DefaultProject = excluded.DefaultProject,
        HourlyRate = excluded.HourlyRate,
        SalaryType = excluded.SalaryType,
        MonthlySalary = excluded.MonthlySalary,
        EmploymentPercent = excluded.EmploymentPercent,
        ExpectedHoursPerWorkday = excluded.ExpectedHoursPerWorkday,
        ExpectedWorkingWeekdays = excluded.ExpectedWorkingWeekdays,
        ExcludePublicHolidays = excluded.ExcludePublicHolidays,
        DefaultStartTime = excluded.DefaultStartTime,
        DefaultEndTime = excluded.DefaultEndTime,
        DefaultLunchMinutes = excluded.DefaultLunchMinutes,
        TaxMode = excluded.TaxMode,
        TaxYear = excluded.TaxYear,
        TaxTableNumber = excluded.TaxTableNumber,
        TaxColumn = excluded.TaxColumn,
        ManualTaxValue = excluded.ManualTaxValue,
        OpeningBalanceMinutes = excluded.OpeningBalanceMinutes,
        CurrencyPreference = excluded.CurrencyPreference,
        ExportLanguagePreference = excluded.ExportLanguagePreference,
        OvertimeCompensationMode = excluded.OvertimeCompensationMode,
        OvertimePremiumPercent = excluded.OvertimePremiumPercent,
        OvertimeDailyThresholdHours = excluded.OvertimeDailyThresholdHours,
        OvertimeThresholdMode = excluded.OvertimeThresholdMode,
        OvertimeDefaultRateType = excluded.OvertimeDefaultRateType,
        OvertimeRateBandsJson = excluded.OvertimeRateBandsJson
    `).run(
      workspaceId,
      settings.employeeName || '',
      settings.employerName || '',
      settings.defaultProject || '',
      settings.salary?.hourlyRate || 0,
      settings.salary?.type || 0,
      settings.salary?.monthlySalary || 0,
      settings.salary?.employmentPercent || 100,
      settings.expectedHours?.hoursPerWorkday || 8,
      (settings.expectedHours?.workingWeekdays || [1, 2, 3, 4, 5]).join(','),
      settings.expectedHours?.excludePublicHolidays ? 1 : 0,
      settings.defaultStartTime || '08:00',
      settings.defaultEndTime || '16:30',
      settings.defaultLunchMinutes ?? 30,
      settings.taxSettings?.mode || 0,
      settings.taxSettings?.taxYear || 2026,
      settings.taxSettings?.tableNumber || 30,
      settings.taxSettings?.column || 1,
      settings.taxSettings?.manualMonthlyDeduction,
      settings.openingBalanceMinutes || 0,
      curr,
      settings.exportLanguagePreference ?? 2,
      settings.overtimeCompensation?.mode || 0,
      settings.overtimeCompensation?.defaultRateValue || 50,
      settings.overtimeCompensation?.dailyThresholdHours || 8,
      settings.overtimeCompensation?.thresholdMode || 0,
      settings.overtimeCompensation?.defaultRateType || 0,
      JSON.stringify(settings.overtimeCompensation?.rateBands || [])
    );
  }

  // --- WorkEntries ---
  public getWorkEntries(year: number, month: number, workspaceId: string = 'ws-default'): any[] {
    const monthPrefix = `${year}-${String(month).padStart(2, '0')}%`;
    const rows = this.db.prepare('SELECT * FROM WorkEntries WHERE WorkspaceId = ? AND Date LIKE ? ORDER BY Date ASC').all(workspaceId, monthPrefix) as any[];

    return rows.map(r => ({
      workspaceId: r.WorkspaceId,
      date: r.Date,
      status: r.Status,
      startTime: r.StartTime,
      endTime: r.EndTime,
      lunchMinutes: r.LunchMinutes,
      projectName: r.ProjectName,
      notes: r.Notes,
      scheduledMinutesOverride: r.ScheduledMinutesOverride,
      createdAt: r.CreatedAt,
      updatedAt: r.UpdatedAt
    }));
  }

  public saveWorkEntry(entry: any, workspaceId: string = 'ws-default'): void {
    const now = new Date().toISOString();
    this.db.prepare(`
      INSERT INTO WorkEntries (
        WorkspaceId, Date, Status, StartTime, EndTime, LunchMinutes, ProjectName,
        Notes, ScheduledMinutesOverride, CreatedAt, UpdatedAt
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(WorkspaceId, Date) DO UPDATE SET
        Status = excluded.Status,
        StartTime = excluded.StartTime,
        EndTime = excluded.EndTime,
        LunchMinutes = excluded.LunchMinutes,
        ProjectName = excluded.ProjectName,
        Notes = excluded.Notes,
        ScheduledMinutesOverride = excluded.ScheduledMinutesOverride,
        UpdatedAt = excluded.UpdatedAt
    `).run(
      workspaceId,
      entry.date,
      entry.status,
      entry.startTime,
      entry.endTime,
      entry.lunchMinutes || 0,
      entry.projectName,
      entry.notes,
      entry.scheduledMinutesOverride,
      now,
      now
    );
  }

  public deleteWorkEntry(date: string, workspaceId: string = 'ws-default'): void {
    this.db.prepare('DELETE FROM WorkEntries WHERE WorkspaceId = ? AND Date = ?').run(workspaceId, date);
  }

  // --- MonthRecords ---
  public getMonthRecord(year: number, month: number, defaultOpeningBalance = 0, workspaceId: string = 'ws-default'): any {
    const row = this.db.prepare('SELECT * FROM MonthRecords WHERE WorkspaceId = ? AND Year = ? AND Month = ?').get(workspaceId, year, month) as any;
    if (!row) {
      return {
        workspaceId,
        year,
        month,
        openingBalanceMinutes: defaultOpeningBalance,
        expectedMinutesOverride: null,
        openingBalanceWasEdited: false
      };
    }

    return {
      workspaceId: row.WorkspaceId,
      year: row.Year,
      month: row.Month,
      openingBalanceMinutes: row.OpeningBalanceMinutes,
      expectedMinutesOverride: row.ExpectedMinutesOverride,
      openingBalanceWasEdited: Boolean(row.OpeningBalanceWasEdited)
    };
  }

  public saveMonthRecord(record: any, workspaceId: string = 'ws-default'): void {
    this.db.prepare(`
      INSERT INTO MonthRecords (WorkspaceId, Year, Month, OpeningBalanceMinutes, ExpectedMinutesOverride, OpeningBalanceWasEdited)
      VALUES (?, ?, ?, ?, ?, ?)
      ON CONFLICT(WorkspaceId, Year, Month) DO UPDATE SET
        OpeningBalanceMinutes = excluded.OpeningBalanceMinutes,
        ExpectedMinutesOverride = excluded.ExpectedMinutesOverride,
        OpeningBalanceWasEdited = excluded.OpeningBalanceWasEdited
    `).run(
      workspaceId,
      record.year,
      record.month,
      record.openingBalanceMinutes || 0,
      record.expectedMinutesOverride,
      record.openingBalanceWasEdited ? 1 : 0
    );
  }

  // --- Projects ---
  public getProjects(workspaceId: string = 'ws-default'): any[] {
    const rows = this.db.prepare('SELECT * FROM Projects WHERE WorkspaceId = ? ORDER BY Name ASC').all(workspaceId) as any[];
    return rows.map(r => ({
      workspaceId: r.WorkspaceId,
      id: r.Id,
      name: r.Name,
      color: r.Color || '#5F875F',
      isActive: Boolean(r.IsActive),
      isDefault: Boolean(r.IsDefault)
    }));
  }

  public saveProject(project: any, workspaceId: string = 'ws-default'): void {
    this.db.prepare(`
      INSERT INTO Projects (WorkspaceId, Id, Name, Color, IsActive, IsDefault)
      VALUES (?, ?, ?, ?, ?, ?)
      ON CONFLICT(WorkspaceId, Id) DO UPDATE SET
        Name = excluded.Name,
        Color = excluded.Color,
        IsActive = excluded.IsActive,
        IsDefault = excluded.IsDefault
    `).run(
      workspaceId,
      project.id,
      project.name,
      project.color || '#5F875F',
      project.isActive ? 1 : 0,
      project.isDefault ? 1 : 0
    );
  }

  public deleteProject(id: string, workspaceId: string = 'ws-default'): void {
    this.db.prepare('DELETE FROM Projects WHERE WorkspaceId = ? AND Id = ?').run(workspaceId, id);
  }

  // --- Backups ---
  public async createBackup(destinationFolder?: string): Promise<string> {
    const folder = destinationFolder || path.dirname(this.dbPath);
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const backupPath = path.join(folder, `dagsverk-backup-${timestamp}.db`);
    await this.db.backup(backupPath);
    return backupPath;
  }

  public restoreBackup(backupFilePath: string): void {
    if (!fs.existsSync(backupFilePath)) {
      throw new Error(`Backup file does not exist: ${backupFilePath}`);
    }
    this.db.close();
    fs.copyFileSync(backupFilePath, this.dbPath);
    this.db = new Database(this.dbPath);
    this.db.pragma('journal_mode = WAL');
    this.db.pragma('foreign_keys = ON');
  }
}
