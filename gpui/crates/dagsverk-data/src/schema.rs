use rusqlite::Connection;

use crate::Result;

pub const REQUIRED_TABLES: [&str; 6] = [
    "Workspaces",
    "AppPreferences",
    "WorkspaceSettings",
    "WorkEntries",
    "MonthRecords",
    "Projects",
];

pub fn initialize(connection: &Connection, now: &str) -> Result<()> {
    connection.execute_batch(
        r#"
        CREATE TABLE IF NOT EXISTS Workspaces (
          Id TEXT PRIMARY KEY,
          Name TEXT NOT NULL,
          Color TEXT NOT NULL,
          WorkspaceType INTEGER NOT NULL DEFAULT 0,
          WorkerName TEXT NOT NULL DEFAULT '',
          EmployerName TEXT NOT NULL DEFAULT '',
          CreatedAt TEXT NOT NULL,
          UpdatedAt TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS AppPreferences (
          Id INTEGER PRIMARY KEY CHECK (Id = 1),
          ActiveWorkspaceId TEXT NOT NULL,
          ThemePreference INTEGER NOT NULL DEFAULT 0,
          LanguagePreference INTEGER NOT NULL DEFAULT 0,
          InterfaceScalePercent INTEGER NOT NULL DEFAULT 100,
          MonthViewPreference INTEGER NOT NULL DEFAULT 0,
          HasCompletedSetup INTEGER NOT NULL DEFAULT 0,
          FOREIGN KEY (ActiveWorkspaceId) REFERENCES Workspaces(Id) ON DELETE RESTRICT
        );
        CREATE TABLE IF NOT EXISTS WorkspaceSettings (
          WorkspaceId TEXT PRIMARY KEY,
          EmployeeName TEXT NOT NULL DEFAULT '',
          EmployerName TEXT NOT NULL DEFAULT '',
          DefaultProject TEXT NOT NULL DEFAULT 'General',
          HourlyRate DECIMAL NOT NULL DEFAULT 250,
          SalaryType INTEGER NOT NULL DEFAULT 0,
          MonthlySalary DECIMAL NOT NULL DEFAULT 40000,
          EmploymentPercent DECIMAL NOT NULL DEFAULT 100,
          HourlyPayBasis INTEGER NOT NULL DEFAULT 0,
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
          OvertimeObCombination INTEGER NOT NULL DEFAULT 0,
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS WorkEntries (
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
        CREATE TABLE IF NOT EXISTS MonthRecords (
          WorkspaceId TEXT NOT NULL,
          Year INTEGER NOT NULL,
          Month INTEGER NOT NULL,
          OpeningBalanceMinutes INTEGER NOT NULL DEFAULT 0,
          ExpectedMinutesOverride INTEGER,
          OpeningBalanceWasEdited INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (WorkspaceId, Year, Month),
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );
        CREATE TABLE IF NOT EXISTS Projects (
          WorkspaceId TEXT NOT NULL,
          Id TEXT NOT NULL,
          Name TEXT NOT NULL,
          Color TEXT,
          IsActive INTEGER NOT NULL DEFAULT 1,
          IsDefault INTEGER NOT NULL DEFAULT 0,
          PRIMARY KEY (WorkspaceId, Id),
          FOREIGN KEY (WorkspaceId) REFERENCES Workspaces(Id) ON DELETE CASCADE
        );
        "#,
    )?;
    ensure_column(
        connection,
        "WorkspaceSettings",
        "HourlyPayBasis",
        "ALTER TABLE WorkspaceSettings ADD COLUMN HourlyPayBasis INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_column(
        connection,
        "WorkspaceSettings",
        "OvertimeObCombination",
        "ALTER TABLE WorkspaceSettings ADD COLUMN OvertimeObCombination INTEGER NOT NULL DEFAULT 0",
    )?;
    seed(connection, now)
}

fn seed(connection: &Connection, now: &str) -> Result<()> {
    connection.execute(
        "INSERT OR IGNORE INTO Workspaces (Id, Name, Color, WorkspaceType, WorkerName, EmployerName, CreatedAt, UpdatedAt) VALUES ('ws-default', 'Main Workspace', '#5F875F', 0, '', '', ?1, ?1)",
        [now],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO AppPreferences (Id, ActiveWorkspaceId, ThemePreference, LanguagePreference, InterfaceScalePercent, MonthViewPreference, HasCompletedSetup) VALUES (1, 'ws-default', 0, 0, 100, 0, 0)",
        [],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO WorkspaceSettings (WorkspaceId) VALUES ('ws-default')",
        [],
    )?;
    connection.execute(
        "INSERT OR IGNORE INTO Projects (WorkspaceId, Id, Name, Color, IsActive, IsDefault) VALUES ('ws-default', 'proj-default', 'General', '#5F875F', 1, 1)",
        [],
    )?;
    Ok(())
}

fn ensure_column(
    connection: &Connection,
    table: &str,
    column: &str,
    statement: &str,
) -> Result<()> {
    let mut query = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let names = query
        .query_map([], |row| row.get::<_, String>(1))?
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if !names.iter().any(|name| name == column) {
        connection.execute_batch(statement)?;
    }
    Ok(())
}
