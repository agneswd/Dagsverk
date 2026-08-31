import Database from 'better-sqlite3';
import * as path from 'path';
import * as fs from 'fs';
import { randomUUID } from 'crypto';
import { app } from 'electron';

const RETAINED_BACKUP_COUNT = 5;
const REQUIRED_TABLES = [
  'Workspaces',
  'AppPreferences',
  'WorkspaceSettings',
  'WorkEntries',
  'MonthRecords',
  'Projects',
];

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

    this.db = this.openDatabase();
    this.initSchema();
    this.ensureWorkspaceIdentitySchema();
    this.ensurePayrollParitySchema();
  }

  public getDatabasePath(): string {
    return this.dbPath;
  }

  public close(): void {
    if (this.db.open) this.db.close();
  }

  private initSchema(): void {
    const tableExists = this.db
      .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='Workspaces'")
      .get();

    if (!tableExists) {
      // Check if legacy table exists
      const legacyWorkEntries = this.db
        .prepare("SELECT name FROM sqlite_master WHERE type='table' AND name='WorkEntries'")
        .get();

      if (legacyWorkEntries) {
        this.createMigrationSafetyBackup();
        this.migrateFromLegacySchema();
        return;
      }

      this.db.exec(`
        CREATE TABLE Workspaces (
          Id TEXT PRIMARY KEY,
          Name TEXT NOT NULL,
          Color TEXT NOT NULL,
          WorkspaceType INTEGER NOT NULL DEFAULT 0,
          WorkerName TEXT NOT NULL DEFAULT '',
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
          HasCompletedSetup INTEGER NOT NULL DEFAULT 0,
          FOREIGN KEY (ActiveWorkspaceId) REFERENCES Workspaces(Id) ON DELETE RESTRICT
        );

        CREATE TABLE WorkspaceSettings (
          WorkspaceId TEXT PRIMARY KEY,
          EmployeeName TEXT NOT NULL DEFAULT '',
          EmployerName TEXT NOT NULL DEFAULT '',
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
          DayOffReason TEXT,
          Notes TEXT,
          ScheduledMinutesOverride INTEGER,
          CompTimeMinutes INTEGER NOT NULL DEFAULT 0 CHECK (CompTimeMinutes >= 0),
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
      this.db
        .prepare(
          `
        INSERT INTO Workspaces (Id, Name, Color, WorkspaceType, WorkerName, EmployerName, CreatedAt, UpdatedAt)
        VALUES ('ws-default', 'Main Workspace', '#5F875F', 0, '', '', ?, ?)
      `,
        )
        .run(now, now);

      this.db
        .prepare(
          `
        INSERT INTO AppPreferences (Id, ActiveWorkspaceId, ThemePreference, LanguagePreference, InterfaceScalePercent, MonthViewPreference, HasCompletedSetup)
        VALUES (1, 'ws-default', 0, 0, 100, 0, 0)
      `,
        )
        .run();

      this.db
        .prepare(
          `
        INSERT INTO WorkspaceSettings (WorkspaceId)
        VALUES ('ws-default')
      `,
        )
        .run();

      this.db
        .prepare(
          `
        INSERT INTO Projects (WorkspaceId, Id, Name, Color, IsActive, IsDefault)
        VALUES ('ws-default', 'proj-default', 'General', '#5F875F', 1, 1)
      `,
        )
        .run();
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
          WorkspaceType INTEGER NOT NULL DEFAULT 0,
          WorkerName TEXT NOT NULL DEFAULT '',
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
          HasCompletedSetup INTEGER NOT NULL DEFAULT 0,
          FOREIGN KEY (ActiveWorkspaceId) REFERENCES Workspaces(Id) ON DELETE RESTRICT
        );

        CREATE TABLE WorkspaceSettings (
          WorkspaceId TEXT PRIMARY KEY,
          EmployeeName TEXT NOT NULL DEFAULT '',
          EmployerName TEXT NOT NULL DEFAULT '',
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
          DayOffReason TEXT,
          Notes TEXT,
          ScheduledMinutesOverride INTEGER,
          CompTimeMinutes INTEGER NOT NULL DEFAULT 0 CHECK (CompTimeMinutes >= 0),
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
      this.db
        .prepare(
          `
        INSERT INTO Workspaces (Id, Name, Color, WorkspaceType, WorkerName, EmployerName, CreatedAt, UpdatedAt)
        VALUES ('ws-default', 'Main Workspace', '#5F875F', 0, '', '', ?, ?)
      `,
        )
        .run(now, now);

      // 4. Copy old settings into AppPreferences & WorkspaceSettings
      const oldSettings = this.db.prepare('SELECT * FROM Old_Settings WHERE Id = 1').get() as any;
      if (oldSettings) {
        this.db
          .prepare(
            `
          INSERT INTO AppPreferences (Id, ActiveWorkspaceId, ThemePreference, LanguagePreference, InterfaceScalePercent, MonthViewPreference, HasCompletedSetup)
          VALUES (1, 'ws-default', ?, ?, ?, ?, 1)
        `,
          )
          .run(
            oldSettings.ThemePreference || 0,
            oldSettings.LanguagePreference || 0,
            oldSettings.InterfaceScalePercent || 100,
            oldSettings.MonthViewPreference || 0,
          );

        this.db
          .prepare(
            `
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
        `,
          )
          .run(
            oldSettings.EmployeeName || '',
            oldSettings.EmployerName || '',
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
            oldSettings.OvertimeRateBandsJson || '[]',
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

  private ensureWorkspaceIdentitySchema(): void {
    const columns = this.db.pragma('table_info(Workspaces)') as Array<{ name: string }>;
    const names = new Set(columns.map((column) => column.name));

    if (!names.has('WorkspaceType')) {
      this.db.exec('ALTER TABLE Workspaces ADD COLUMN WorkspaceType INTEGER NOT NULL DEFAULT 0');
    }
    if (!names.has('WorkerName')) {
      this.db.exec("ALTER TABLE Workspaces ADD COLUMN WorkerName TEXT NOT NULL DEFAULT ''");
      this.db.exec(`
        UPDATE Workspaces
        SET WorkerName = COALESCE(
          (SELECT EmployeeName FROM WorkspaceSettings WHERE WorkspaceId = Workspaces.Id),
          ''
        )
      `);
    }

    const preferenceColumns = this.db.pragma('table_info(AppPreferences)') as Array<{
      name: string;
    }>;
    if (!preferenceColumns.some((column) => column.name === 'HasCompletedSetup')) {
      this.db.exec(
        'ALTER TABLE AppPreferences ADD COLUMN HasCompletedSetup INTEGER NOT NULL DEFAULT 1',
      );
    }
  }

  private ensurePayrollParitySchema(): void {
    const columns = this.db.pragma('table_info(WorkspaceSettings)') as Array<{ name: string }>;
    const names = new Set(columns.map((column) => column.name));
    if (!names.has('HourlyPayBasis')) {
      this.db.exec(
        'ALTER TABLE WorkspaceSettings ADD COLUMN HourlyPayBasis INTEGER NOT NULL DEFAULT 0',
      );
    }
    if (!names.has('OvertimeObCombination')) {
      this.db.exec(
        'ALTER TABLE WorkspaceSettings ADD COLUMN OvertimeObCombination INTEGER NOT NULL DEFAULT 0',
      );
    }

    const entryColumns = this.db.pragma('table_info(WorkEntries)') as Array<{ name: string }>;
    if (!entryColumns.some((column) => column.name === 'CompTimeMinutes')) {
      this.db.exec('ALTER TABLE WorkEntries ADD COLUMN CompTimeMinutes INTEGER NOT NULL DEFAULT 0');
    }
    if (!entryColumns.some((column) => column.name === 'DayOffReason')) {
      this.db.exec('ALTER TABLE WorkEntries ADD COLUMN DayOffReason TEXT');
    }
  }

  // --- Workspaces ---
  public getWorkspaces(): any[] {
    const rows = this.db.prepare('SELECT * FROM Workspaces ORDER BY CreatedAt ASC').all() as any[];
    return rows.map((r) => ({
      id: r.Id,
      name: r.Name,
      color: r.Color,
      type: r.WorkspaceType,
      workerName: r.WorkerName || undefined,
      organizationName: r.EmployerName || undefined,
      createdAt: r.CreatedAt,
      updatedAt: r.UpdatedAt,
    }));
  }

  public saveWorkspace(ws: any): void {
    const now = new Date().toISOString();
    this.db
      .prepare(
        `
      INSERT INTO Workspaces (Id, Name, Color, WorkspaceType, WorkerName, EmployerName, CreatedAt, UpdatedAt)
      VALUES (?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(Id) DO UPDATE SET
        Name = excluded.Name,
        Color = excluded.Color,
        WorkspaceType = excluded.WorkspaceType,
        WorkerName = excluded.WorkerName,
        EmployerName = excluded.EmployerName,
        UpdatedAt = excluded.UpdatedAt
    `,
      )
      .run(
        ws.id,
        ws.name,
        ws.color || '#5F875F',
        ws.type ?? 0,
        ws.workerName || '',
        ws.organizationName || '',
        ws.createdAt || now,
        now,
      );

    // Ensure WorkspaceSettings exist
    const settingsExist = this.db
      .prepare('SELECT WorkspaceId FROM WorkspaceSettings WHERE WorkspaceId = ?')
      .get(ws.id);
    if (!settingsExist) {
      this.db
        .prepare(
          `
        INSERT INTO WorkspaceSettings (WorkspaceId, EmployeeName, EmployerName)
        VALUES (?, ?, ?)
      `,
        )
        .run(ws.id, ws.workerName || '', ws.organizationName || '');
    }
  }

  public deleteWorkspace(id: string): void {
    const count = (
      this.db.prepare('SELECT COUNT(*) as cnt FROM Workspaces').get() as { cnt: number }
    ).cnt;
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
        monthViewPreference: 0,
        hasCompletedSetup: false,
      };
    }

    return {
      activeWorkspaceId: row.ActiveWorkspaceId,
      themePreference: row.ThemePreference,
      languagePreference: row.LanguagePreference,
      interfaceScalePercent: row.InterfaceScalePercent,
      monthViewPreference: row.MonthViewPreference,
      hasCompletedSetup: Boolean(row.HasCompletedSetup),
    };
  }

  public saveAppPreferences(prefs: any): void {
    this.db
      .prepare(
        `
      INSERT INTO AppPreferences (Id, ActiveWorkspaceId, ThemePreference, LanguagePreference, InterfaceScalePercent, MonthViewPreference, HasCompletedSetup)
      VALUES (1, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(Id) DO UPDATE SET
        ActiveWorkspaceId = excluded.ActiveWorkspaceId,
        ThemePreference = excluded.ThemePreference,
        LanguagePreference = excluded.LanguagePreference,
        InterfaceScalePercent = excluded.InterfaceScalePercent,
        MonthViewPreference = excluded.MonthViewPreference,
        HasCompletedSetup = excluded.HasCompletedSetup
    `,
      )
      .run(
        prefs.activeWorkspaceId || 'ws-default',
        prefs.themePreference || 0,
        prefs.languagePreference || 0,
        prefs.interfaceScalePercent || 100,
        prefs.monthViewPreference || 0,
        prefs.hasCompletedSetup ? 1 : 0,
      );
  }

  // --- WorkspaceSettings ---
  public getSettings(workspaceId: string = 'ws-default'): any {
    let row = this.db
      .prepare('SELECT * FROM WorkspaceSettings WHERE WorkspaceId = ?')
      .get(workspaceId) as any;
    if (!row) {
      this.db.prepare('INSERT INTO WorkspaceSettings (WorkspaceId) VALUES (?)').run(workspaceId);
      row = this.db
        .prepare('SELECT * FROM WorkspaceSettings WHERE WorkspaceId = ?')
        .get(workspaceId) as any;
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
        employmentPercent: Number(row.EmploymentPercent),
        hourlyPayBasis: row.HourlyPayBasis ?? 0,
      },
      expectedHours: {
        hoursPerWorkday: Number(row.ExpectedHoursPerWorkday),
        workingWeekdays: row.ExpectedWorkingWeekdays.split(',').map(Number),
        excludePublicHolidays: Boolean(row.ExcludePublicHolidays),
      },
      defaultStartTime: row.DefaultStartTime,
      defaultEndTime: row.DefaultEndTime,
      defaultLunchMinutes: row.DefaultLunchMinutes,
      taxSettings: {
        mode: row.TaxMode,
        taxYear: row.TaxYear,
        tableNumber: row.TaxTableNumber,
        column: row.TaxColumn,
        manualMonthlyDeduction: row.ManualTaxValue !== null ? Number(row.ManualTaxValue) : null,
      },
      openingBalanceMinutes: row.OpeningBalanceMinutes,
      currencyPreference:
        ['SEK', 'EUR', 'USD', 'GBP', 'NOK', 'DKK'][row.CurrencyPreference] || 'SEK',
      exportLanguagePreference: row.ExportLanguagePreference,
      overtimeCompensation: {
        mode: row.OvertimeCompensationMode,
        defaultRateType: row.OvertimeDefaultRateType,
        defaultRateValue: Number(row.OvertimePremiumPercent),
        dailyThresholdHours: Number(row.OvertimeDailyThresholdHours),
        thresholdMode: row.OvertimeThresholdMode,
        rateBands: JSON.parse(row.OvertimeRateBandsJson || '[]'),
        obOvertimeCombination: row.OvertimeObCombination ?? 0,
      },
    };
  }

  public saveSettings(settings: any, workspaceId: string = 'ws-default'): void {
    const currencyIdx = ['SEK', 'EUR', 'USD', 'GBP', 'NOK', 'DKK'].indexOf(
      settings.currencyPreference,
    );
    const curr = currencyIdx >= 0 ? currencyIdx : 0;

    this.db
      .prepare(
        `
      INSERT INTO WorkspaceSettings (
        WorkspaceId, EmployeeName, EmployerName, DefaultProject, HourlyRate, SalaryType,
        MonthlySalary, EmploymentPercent, HourlyPayBasis, ExpectedHoursPerWorkday, ExpectedWorkingWeekdays,
        ExcludePublicHolidays, DefaultStartTime, DefaultEndTime, DefaultLunchMinutes,
        TaxMode, TaxYear, TaxTableNumber, TaxColumn, ManualTaxValue,
        OpeningBalanceMinutes, CurrencyPreference, ExportLanguagePreference,
        OvertimeCompensationMode, OvertimePremiumPercent, OvertimeDailyThresholdHours,
        OvertimeThresholdMode, OvertimeDefaultRateType, OvertimeRateBandsJson,
        OvertimeObCombination
      ) VALUES (
        ?, ?, ?, ?, ?, ?,
        ?, ?, ?, ?,
        ?, ?, ?, ?,
        ?, ?, ?, ?, ?, ?,
        ?, ?, ?,
        ?, ?, ?,
        ?, ?, ?, ?
      )
      ON CONFLICT(WorkspaceId) DO UPDATE SET
        EmployeeName = excluded.EmployeeName,
        EmployerName = excluded.EmployerName,
        DefaultProject = excluded.DefaultProject,
        HourlyRate = excluded.HourlyRate,
        SalaryType = excluded.SalaryType,
        MonthlySalary = excluded.MonthlySalary,
        EmploymentPercent = excluded.EmploymentPercent,
        HourlyPayBasis = excluded.HourlyPayBasis,
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
        OvertimeRateBandsJson = excluded.OvertimeRateBandsJson,
        OvertimeObCombination = excluded.OvertimeObCombination
    `,
      )
      .run(
        workspaceId,
        settings.employeeName || '',
        settings.employerName || '',
        settings.defaultProject || '',
        settings.salary?.hourlyRate || 0,
        settings.salary?.type || 0,
        settings.salary?.monthlySalary || 0,
        settings.salary?.employmentPercent || 100,
        settings.salary?.hourlyPayBasis ?? 0,
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
        JSON.stringify(settings.overtimeCompensation?.rateBands || []),
        settings.overtimeCompensation?.obOvertimeCombination ?? 0,
      );
  }

  // --- WorkEntries ---
  public getWorkEntries(year: number, month: number, workspaceId: string = 'ws-default'): any[] {
    const monthPrefix = `${year}-${String(month).padStart(2, '0')}%`;
    const rows = this.db
      .prepare('SELECT * FROM WorkEntries WHERE WorkspaceId = ? AND Date LIKE ? ORDER BY Date ASC')
      .all(workspaceId, monthPrefix) as any[];

    return rows.map((r) => ({
      workspaceId: r.WorkspaceId,
      date: r.Date,
      status: r.Status,
      startTime: r.StartTime,
      endTime: r.EndTime,
      lunchMinutes: r.LunchMinutes,
      projectName: r.ProjectName,
      dayOffReason: r.DayOffReason || null,
      notes: r.Notes,
      scheduledMinutesOverride: r.ScheduledMinutesOverride,
      compTimeMinutes: r.CompTimeMinutes || 0,
      createdAt: r.CreatedAt,
      updatedAt: r.UpdatedAt,
    }));
  }

  public saveWorkEntry(entry: any, workspaceId: string = 'ws-default'): void {
    const now = new Date().toISOString();
    const compTimeMinutes = entry.compTimeMinutes ?? 0;
    if (!Number.isInteger(compTimeMinutes) || compTimeMinutes < 0) {
      throw new Error('Comp time used must be a non-negative whole number of minutes.');
    }
    this.db
      .prepare(
        `
      INSERT INTO WorkEntries (
        WorkspaceId, Date, Status, StartTime, EndTime, LunchMinutes, ProjectName,
        DayOffReason, Notes, ScheduledMinutesOverride, CompTimeMinutes, CreatedAt, UpdatedAt
      ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
      ON CONFLICT(WorkspaceId, Date) DO UPDATE SET
        Status = excluded.Status,
        StartTime = excluded.StartTime,
        EndTime = excluded.EndTime,
        LunchMinutes = excluded.LunchMinutes,
        ProjectName = excluded.ProjectName,
        DayOffReason = excluded.DayOffReason,
        Notes = excluded.Notes,
        ScheduledMinutesOverride = excluded.ScheduledMinutesOverride,
        CompTimeMinutes = excluded.CompTimeMinutes,
        UpdatedAt = excluded.UpdatedAt
    `,
      )
      .run(
        workspaceId,
        entry.date,
        entry.status,
        entry.startTime,
        entry.endTime,
        entry.lunchMinutes || 0,
        entry.projectName,
        entry.dayOffReason ?? null,
        entry.notes,
        entry.scheduledMinutesOverride,
        compTimeMinutes,
        entry.createdAt || now,
        entry.updatedAt || now,
      );
  }

  public saveWorkEntries(entries: any[], workspaceId: string = 'ws-default'): void {
    this.db.transaction(() => {
      for (const entry of entries) this.saveWorkEntry(entry, workspaceId);
    })();
  }

  public deleteWorkEntry(date: string, workspaceId: string = 'ws-default'): void {
    this.db
      .prepare('DELETE FROM WorkEntries WHERE WorkspaceId = ? AND Date = ?')
      .run(workspaceId, date);
  }

  // --- MonthRecords ---
  public getMonthRecord(
    year: number,
    month: number,
    defaultOpeningBalance = 0,
    workspaceId: string = 'ws-default',
  ): any {
    const row = this.db
      .prepare('SELECT * FROM MonthRecords WHERE WorkspaceId = ? AND Year = ? AND Month = ?')
      .get(workspaceId, year, month) as any;
    if (!row) {
      return {
        workspaceId,
        year,
        month,
        openingBalanceMinutes: defaultOpeningBalance,
        expectedMinutesOverride: null,
        openingBalanceWasEdited: false,
      };
    }

    return {
      workspaceId: row.WorkspaceId,
      year: row.Year,
      month: row.Month,
      openingBalanceMinutes: row.OpeningBalanceMinutes,
      expectedMinutesOverride: row.ExpectedMinutesOverride,
      openingBalanceWasEdited: Boolean(row.OpeningBalanceWasEdited),
    };
  }

  public saveMonthRecord(record: any, workspaceId: string = 'ws-default'): void {
    this.db
      .prepare(
        `
      INSERT INTO MonthRecords (WorkspaceId, Year, Month, OpeningBalanceMinutes, ExpectedMinutesOverride, OpeningBalanceWasEdited)
      VALUES (?, ?, ?, ?, ?, ?)
      ON CONFLICT(WorkspaceId, Year, Month) DO UPDATE SET
        OpeningBalanceMinutes = excluded.OpeningBalanceMinutes,
        ExpectedMinutesOverride = excluded.ExpectedMinutesOverride,
        OpeningBalanceWasEdited = excluded.OpeningBalanceWasEdited
    `,
      )
      .run(
        workspaceId,
        record.year,
        record.month,
        record.openingBalanceMinutes || 0,
        record.expectedMinutesOverride,
        record.openingBalanceWasEdited ? 1 : 0,
      );
  }

  public resetMonth(year: number, month: number, workspaceId: string = 'ws-default'): void {
    const monthPrefix = `${year}-${String(month).padStart(2, '0')}%`;
    this.db.transaction(() => {
      this.db
        .prepare('DELETE FROM WorkEntries WHERE WorkspaceId = ? AND Date LIKE ?')
        .run(workspaceId, monthPrefix);
      this.db
        .prepare('DELETE FROM MonthRecords WHERE WorkspaceId = ? AND Year = ? AND Month = ?')
        .run(workspaceId, year, month);
    })();
  }

  public getBalanceHistory(
    beforeYear: number,
    beforeMonth: number,
    workspaceId: string = 'ws-default',
  ): any[] {
    const before = `${beforeYear}-${String(beforeMonth).padStart(2, '0')}`;
    const records = this.db
      .prepare(
        `
      SELECT * FROM MonthRecords
      WHERE WorkspaceId = ? AND printf('%04d-%02d', Year, Month) < ?
    `,
      )
      .all(workspaceId, before) as any[];
    const entries = this.db
      .prepare(
        `
      SELECT * FROM WorkEntries
      WHERE WorkspaceId = ? AND substr(Date, 1, 7) < ? AND Status <> 0
      ORDER BY Date
    `,
      )
      .all(workspaceId, before) as any[];

    const months = new Map<string, any>();
    for (const row of records) {
      const key = `${row.Year}-${String(row.Month).padStart(2, '0')}`;
      months.set(key, {
        year: row.Year,
        month: row.Month,
        record: this.getMonthRecord(row.Year, row.Month, 0, workspaceId),
        entries: [],
      });
    }
    for (const row of entries) {
      const key = row.Date.slice(0, 7);
      const [year, month] = key.split('-').map(Number);
      const item = months.get(key) || { year, month, record: null, entries: [] };
      item.entries.push({
        workspaceId: row.WorkspaceId,
        date: row.Date,
        status: row.Status,
        startTime: row.StartTime,
        endTime: row.EndTime,
        lunchMinutes: row.LunchMinutes,
        projectName: row.ProjectName,
        dayOffReason: row.DayOffReason || null,
        notes: row.Notes,
        scheduledMinutesOverride: row.ScheduledMinutesOverride,
        compTimeMinutes: row.CompTimeMinutes || 0,
      });
      months.set(key, item);
    }
    return [...months.entries()]
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([, value]) => value)
      .slice(-120);
  }

  // --- Projects ---
  public getProjects(workspaceId: string = 'ws-default'): any[] {
    const rows = this.db
      .prepare('SELECT * FROM Projects WHERE WorkspaceId = ? ORDER BY Name ASC')
      .all(workspaceId) as any[];
    return rows.map((r) => ({
      workspaceId: r.WorkspaceId,
      id: r.Id,
      name: r.Name,
      color: r.Color || '#5F875F',
      isActive: Boolean(r.IsActive),
      isDefault: Boolean(r.IsDefault),
    }));
  }

  public saveProject(project: any, workspaceId: string = 'ws-default'): void {
    this.db
      .prepare(
        `
      INSERT INTO Projects (WorkspaceId, Id, Name, Color, IsActive, IsDefault)
      VALUES (?, ?, ?, ?, ?, ?)
      ON CONFLICT(WorkspaceId, Id) DO UPDATE SET
        Name = excluded.Name,
        Color = excluded.Color,
        IsActive = excluded.IsActive,
        IsDefault = excluded.IsDefault
    `,
      )
      .run(
        workspaceId,
        project.id,
        project.name,
        project.color || '#5F875F',
        project.isActive ? 1 : 0,
        project.isDefault ? 1 : 0,
      );
  }

  public deleteProject(id: string, workspaceId: string = 'ws-default'): void {
    this.db.prepare('DELETE FROM Projects WHERE WorkspaceId = ? AND Id = ?').run(workspaceId, id);
  }

  // --- Backups ---
  public async importTidverkDatabase(sourcePath: string): Promise<{
    workspaceId: string;
    workspaceName: string;
    entryCount: number;
    monthCount: number;
    projectCount: number;
    sourceBackupPath: string;
    safetyBackupPath: string;
  }> {
    if (!fs.existsSync(sourcePath)) {
      throw new Error(`Tidverk database does not exist: ${sourcePath}`);
    }
    if (path.resolve(sourcePath) === path.resolve(this.dbPath)) {
      throw new Error('Select the Tidverk database, not the current Dagsverk database.');
    }

    const backupFolder = path.join(path.dirname(this.dbPath), 'backups');
    fs.mkdirSync(backupFolder, { recursive: true });
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const sourceBackupPath = path.join(backupFolder, `tidverk-import-${timestamp}.db`);
    const sourceDatabase = new Database(sourcePath, { readonly: true, fileMustExist: true });

    try {
      const integrity = sourceDatabase.pragma('quick_check') as Array<Record<string, string>>;
      if (Object.values(integrity[0] || {})[0] !== 'ok') {
        throw new Error('The selected Tidverk database failed SQLite integrity validation.');
      }
      const tables = sourceDatabase
        .prepare(
          "SELECT name FROM sqlite_master WHERE type = 'table' AND name IN ('Settings', 'WorkEntries', 'Months', 'Projects')",
        )
        .all() as Array<{ name: string }>;
      if (tables.length !== 4) {
        throw new Error('The selected file is not a supported Tidverk database.');
      }
      await sourceDatabase.backup(sourceBackupPath);
    } finally {
      sourceDatabase.close();
    }

    const snapshot = new Database(sourceBackupPath, { readonly: true, fileMustExist: true });
    try {
      const settings = snapshot.prepare('SELECT * FROM Settings WHERE Id = 1').get() as any;
      if (!settings) throw new Error('The Tidverk database does not contain settings.');
      const entries = snapshot.prepare('SELECT * FROM WorkEntries ORDER BY Date').all() as any[];
      const months = snapshot.prepare('SELECT * FROM Months ORDER BY Year, Month').all() as any[];
      const projects = snapshot.prepare('SELECT * FROM Projects ORDER BY Name').all() as any[];
      const safetyBackupPath = await this.createBackup(undefined, 'before-tidverk-import');
      const pristine =
        (this.db.prepare('SELECT COUNT(*) AS Count FROM Workspaces').get() as any).Count === 1 &&
        (this.db.prepare('SELECT COUNT(*) AS Count FROM WorkEntries').get() as any).Count === 0 &&
        !(this.db.prepare('SELECT HasCompletedSetup FROM AppPreferences WHERE Id = 1').get() as any)
          ?.HasCompletedSetup;
      const workspaceId = pristine ? 'ws-default' : `ws-${randomUUID()}`;
      const workspaceName = String(settings.EmployerName || 'Imported workspace').trim();
      const now = new Date().toISOString();
      const time = (value: unknown): string | null =>
        value === null || value === undefined ? null : String(value).slice(0, 5);
      const number = (value: unknown, fallback: number): number => {
        const parsed = Number(value);
        return Number.isFinite(parsed) ? parsed : fallback;
      };

      this.db.transaction(() => {
        if (pristine) {
          this.db.prepare('DELETE FROM Projects WHERE WorkspaceId = ?').run(workspaceId);
          this.db.prepare('DELETE FROM MonthRecords WHERE WorkspaceId = ?').run(workspaceId);
          this.db.prepare('DELETE FROM WorkspaceSettings WHERE WorkspaceId = ?').run(workspaceId);
        }

        this.saveWorkspace({
          id: workspaceId,
          name: workspaceName,
          color: '#5F875F',
          type: 0,
          workerName: settings.EmployeeName || '',
          organizationName: settings.EmployerName || '',
          createdAt: now,
        });
        this.saveSettings(
          {
            employeeName: settings.EmployeeName || '',
            employerName: settings.EmployerName || '',
            defaultProject: settings.DefaultProject || 'General',
            salary: {
              type: number(settings.SalaryType, 0),
              hourlyRate: number(settings.HourlyRate, 0),
              monthlySalary: number(settings.MonthlySalary, 0),
              employmentPercent: number(settings.EmploymentPercent, 100),
              hourlyPayBasis: number(settings.HourlyPayBasis, 0),
            },
            expectedHours: {
              hoursPerWorkday: number(settings.ExpectedHoursPerWorkday, 8),
              workingWeekdays: String(settings.ExpectedWorkingWeekdays || '1,2,3,4,5')
                .split(',')
                .map(Number),
              excludePublicHolidays: Boolean(settings.ExcludePublicHolidays),
            },
            defaultStartTime: time(settings.DefaultStartTime) || '08:00',
            defaultEndTime: time(settings.DefaultEndTime) || '16:30',
            defaultLunchMinutes: number(settings.DefaultLunchMinutes, 30),
            taxSettings: {
              mode: number(settings.TaxMode, 0),
              taxYear: number(settings.TaxYear, 2026),
              tableNumber: number(settings.TaxTableNumber, 30),
              column: number(settings.TaxColumn, 1),
              manualMonthlyDeduction:
                settings.ManualTaxValue === null ? null : number(settings.ManualTaxValue, 0),
            },
            openingBalanceMinutes: number(settings.OpeningBalanceMinutes, 0),
            currencyPreference:
              ['SEK', 'EUR', 'USD', 'GBP', 'NOK', 'DKK'][
                number(settings.CurrencyPreference, 0)
              ] || 'SEK',
            exportLanguagePreference: number(settings.ExportLanguagePreference, 2),
            overtimeCompensation: {
              mode: number(settings.OvertimeCompensationMode, 0),
              defaultRateType: number(settings.OvertimeDefaultRateType, 0),
              defaultRateValue: number(settings.OvertimePremiumPercent, 50),
              dailyThresholdHours: number(settings.OvertimeDailyThresholdHours, 8),
              thresholdMode: number(settings.OvertimeThresholdMode, 0),
              rateBands: JSON.parse(settings.OvertimeRateBandsJson || '[]'),
              obOvertimeCombination: number(settings.ObOvertimeCombination, 0),
            },
          },
          workspaceId,
        );

        for (const entry of entries) {
          this.saveWorkEntry(
            {
              date: entry.Date,
              status: entry.Status,
              startTime: time(entry.StartTime),
              endTime: time(entry.EndTime),
              lunchMinutes: entry.LunchMinutes,
              projectName: entry.ProjectName,
              dayOffReason: null,
              notes: entry.Notes,
              scheduledMinutesOverride: entry.ScheduledMinutesOverride,
              compTimeMinutes: 0,
              createdAt: entry.CreatedAt,
              updatedAt: entry.UpdatedAt,
            },
            workspaceId,
          );
        }
        for (const month of months) {
          this.saveMonthRecord(
            {
              year: month.Year,
              month: month.Month,
              openingBalanceMinutes: month.OpeningBalanceMinutes,
              expectedMinutesOverride: month.ExpectedMinutesOverride,
              openingBalanceWasEdited: Boolean(month.OpeningBalanceWasEdited),
            },
            workspaceId,
          );
        }
        for (const project of projects) {
          this.saveProject(
            {
              id: project.Id,
              name: project.Name,
              color: '#5F875F',
              isActive: Boolean(project.IsActive),
              isDefault: Boolean(project.IsDefault),
            },
            workspaceId,
          );
        }
        if (projects.length === 0) {
          this.saveProject(
            {
              id: `proj-${randomUUID()}`,
              name: settings.DefaultProject || 'General',
              color: '#5F875F',
              isActive: true,
              isDefault: true,
            },
            workspaceId,
          );
        }
        this.saveAppPreferences({
          activeWorkspaceId: workspaceId,
          themePreference: number(settings.ThemePreference, 0),
          languagePreference: number(settings.LanguagePreference, 0),
          interfaceScalePercent: number(settings.InterfaceScalePercent, 100),
          monthViewPreference: number(settings.MonthViewPreference, 0),
          hasCompletedSetup: true,
        });
      })();

      return {
        workspaceId,
        workspaceName,
        entryCount: entries.length,
        monthCount: months.length,
        projectCount: projects.length || 1,
        sourceBackupPath,
        safetyBackupPath,
      };
    } finally {
      snapshot.close();
    }
  }

  public async createBackup(destinationFolder?: string, reason = 'manual'): Promise<string> {
    const folder = destinationFolder || path.join(path.dirname(this.dbPath), 'backups');
    fs.mkdirSync(folder, { recursive: true });
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    const safeReason = reason.replace(/[^a-z0-9-]/gi, '') || 'backup';
    const backupPath = path.join(folder, `dagsverk-backup-${timestamp}-${safeReason}.db`);
    await this.db.backup(backupPath);
    this.pruneBackups(folder);
    return backupPath;
  }

  public async restoreBackup(backupFilePath: string): Promise<void> {
    if (!fs.existsSync(backupFilePath)) {
      throw new Error(`Backup file does not exist: ${backupFilePath}`);
    }

    const backupFolder = path.join(path.dirname(this.dbPath), 'backups');
    fs.mkdirSync(backupFolder, { recursive: true });
    const candidatePath = path.join(backupFolder, `.restore-${randomUUID()}.db`);
    let safetyBackupPath: string | null = null;

    try {
      const source = new Database(backupFilePath, { readonly: true, fileMustExist: true });
      try {
        await source.backup(candidatePath);
      } finally {
        source.close();
      }

      this.validateDatabase(candidatePath);
      safetyBackupPath = await this.createBackup(undefined, 'before-restore');

      this.db.close();
      this.removeDatabaseSidecars();

      try {
        fs.copyFileSync(candidatePath, this.dbPath);
        this.db = this.openDatabase();
        this.ensureWorkspaceIdentitySchema();
        this.ensurePayrollParitySchema();
        this.validateDatabase(this.dbPath);
      } catch (error) {
        this.removeDatabaseSidecars();
        if (safetyBackupPath) {
          fs.copyFileSync(safetyBackupPath, this.dbPath);
        }
        this.db = this.openDatabase();
        this.ensureWorkspaceIdentitySchema();
        this.ensurePayrollParitySchema();
        throw error;
      }
    } finally {
      fs.rmSync(candidatePath, { force: true });
    }
  }

  private openDatabase(): Database.Database {
    const database = new Database(this.dbPath);
    database.pragma('journal_mode = WAL');
    database.pragma('foreign_keys = ON');
    return database;
  }

  private validateDatabase(databasePath: string): void {
    const database = new Database(databasePath, { readonly: true, fileMustExist: true });
    try {
      const integrityRows = database.pragma('quick_check') as Array<Record<string, string>>;
      const integrity = integrityRows[0] ? Object.values(integrityRows[0])[0] : null;
      if (integrity !== 'ok') {
        throw new Error('The selected database failed SQLite integrity validation.');
      }

      const placeholders = REQUIRED_TABLES.map(() => '?').join(', ');
      const rows = database
        .prepare(
          `
        SELECT name
        FROM sqlite_master
        WHERE type = 'table' AND name IN (${placeholders})
      `,
        )
        .all(...REQUIRED_TABLES) as Array<{ name: string }>;

      if (rows.length !== REQUIRED_TABLES.length) {
        throw new Error('The selected file is not a Dagsverk database.');
      }
    } finally {
      database.close();
    }
  }

  private removeDatabaseSidecars(): void {
    fs.rmSync(`${this.dbPath}-wal`, { force: true });
    fs.rmSync(`${this.dbPath}-shm`, { force: true });
  }

  private pruneBackups(folder: string): void {
    const backups = fs
      .readdirSync(folder)
      .filter((file) => file.startsWith('dagsverk-backup-') && file.endsWith('.db'))
      .sort((left, right) => right.localeCompare(left));

    for (const expired of backups.slice(RETAINED_BACKUP_COUNT)) {
      fs.rmSync(path.join(folder, expired), { force: true });
    }
  }

  private createMigrationSafetyBackup(): void {
    const folder = path.join(path.dirname(this.dbPath), 'backups');
    fs.mkdirSync(folder, { recursive: true });
    this.db.pragma('wal_checkpoint(TRUNCATE)');
    const timestamp = new Date().toISOString().replace(/[:.]/g, '-');
    fs.copyFileSync(
      this.dbPath,
      path.join(folder, `dagsverk-backup-${timestamp}-before-migration.db`),
    );
    this.pruneBackups(folder);
  }
}
