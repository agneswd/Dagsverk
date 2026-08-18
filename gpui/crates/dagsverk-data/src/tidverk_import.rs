use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use dagsverk_core::clock::Clock;
use rusqlite::{Connection, OpenFlags, params};
use uuid::Uuid;

use crate::{DataError, Database, Result, backup::backup_connection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TidverkImportResult {
    pub workspace_id: String,
    pub workspace_name: String,
    pub entry_count: usize,
    pub month_count: usize,
    pub project_count: usize,
    pub source_backup_path: PathBuf,
    pub safety_backup_path: PathBuf,
}

impl<C: Clock> Database<C> {
    pub fn import_tidverk(&self, source_path: &Path) -> Result<TidverkImportResult> {
        if !source_path.exists() {
            return Err(DataError::MissingFile(source_path.to_owned()));
        }
        if fs::canonicalize(source_path).ok() == fs::canonicalize(&self.path).ok() {
            return Err(DataError::SameDatabase);
        }
        let source = Connection::open_with_flags(source_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        validate_tidverk(&source)?;
        let backup_folder = self
            .path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join("backups");
        fs::create_dir_all(&backup_folder).map_err(|source| DataError::CreateDirectory {
            path: backup_folder.clone(),
            source,
        })?;
        let timestamp = self.clock.now_utc().format("%Y-%m-%dT%H-%M-%S-%3fZ");
        let source_backup_path =
            backup_folder.join(format!("tidverk-import-{timestamp}-{}.db", Uuid::new_v4()));
        backup_connection(&source, &source_backup_path)?;
        drop(source);

        let snapshot =
            Connection::open_with_flags(&source_backup_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let settings_columns = columns(&snapshot, "Settings")?;
        let workspace_name = snapshot
            .query_row("SELECT EmployerName FROM Settings WHERE Id=1", [], |row| {
                row.get::<_, Option<String>>(0)
            })?
            .filter(|name| !name.trim().is_empty())
            .unwrap_or_else(|| "Imported workspace".to_owned())
            .trim()
            .to_owned();
        let entry_count = count(&snapshot, "WorkEntries")?;
        let month_count = count(&snapshot, "Months")?;
        let source_project_count = count(&snapshot, "Projects")?;
        drop(snapshot);

        let safety_backup_path = self.create_backup(None, "before-tidverk-import")?;
        let connection = self.connection()?;
        let pristine = connection.query_row(
            r#"SELECT
                (SELECT COUNT(*) FROM Workspaces)=1 AND
                (SELECT COUNT(*) FROM WorkEntries)=0 AND
                COALESCE((SELECT HasCompletedSetup FROM AppPreferences WHERE Id=1),0)=0"#,
            [],
            |row| row.get::<_, bool>(0),
        )?;
        let workspace_id = if pristine {
            "ws-default".to_owned()
        } else {
            format!("ws-{}", Uuid::new_v4())
        };
        connection.execute(
            "ATTACH DATABASE ?1 AS tidverk",
            [source_backup_path.to_string_lossy().as_ref()],
        )?;
        let import_result: Result<()> = (|| {
            let transaction = connection.unchecked_transaction()?;
            if pristine {
                transaction
                    .execute("DELETE FROM Projects WHERE WorkspaceId=?1", [&workspace_id])?;
                transaction.execute(
                    "DELETE FROM MonthRecords WHERE WorkspaceId=?1",
                    [&workspace_id],
                )?;
                transaction.execute(
                    "DELETE FROM WorkspaceSettings WHERE WorkspaceId=?1",
                    [&workspace_id],
                )?;
            }
            let now = self.clock.now_utc().to_rfc3339();
            let worker = expression(&settings_columns, "EmployeeName", "''");
            let employer = expression(&settings_columns, "EmployerName", "''");
            transaction.execute(
                &format!(
                    r#"INSERT INTO Workspaces (Id,Name,Color,WorkspaceType,WorkerName,EmployerName,CreatedAt,UpdatedAt)
                       SELECT ?1,?2,'#5F875F',0,{worker},{employer},?3,?3 FROM tidverk.Settings s WHERE s.Id=1
                       ON CONFLICT(Id) DO UPDATE SET Name=excluded.Name,Color=excluded.Color,
                       WorkspaceType=excluded.WorkspaceType,WorkerName=excluded.WorkerName,
                       EmployerName=excluded.EmployerName,UpdatedAt=excluded.UpdatedAt"#
                ),
                params![workspace_id, workspace_name, now],
            )?;
            insert_settings(&transaction, &settings_columns, &workspace_id)?;
            transaction.execute(
                r#"INSERT INTO WorkEntries (WorkspaceId,Date,Status,StartTime,EndTime,LunchMinutes,ProjectName,Notes,ScheduledMinutesOverride,CreatedAt,UpdatedAt)
                   SELECT ?1,Date,Status,CASE WHEN StartTime IS NULL THEN NULL ELSE substr(StartTime,1,5) END,
                   CASE WHEN EndTime IS NULL THEN NULL ELSE substr(EndTime,1,5) END,
                   COALESCE(LunchMinutes,0),ProjectName,Notes,ScheduledMinutesOverride,
                   COALESCE(CreatedAt,?2),COALESCE(UpdatedAt,?2) FROM tidverk.WorkEntries"#,
                params![workspace_id, now],
            )?;
            transaction.execute(
                r#"INSERT INTO MonthRecords (WorkspaceId,Year,Month,OpeningBalanceMinutes,ExpectedMinutesOverride,OpeningBalanceWasEdited)
                   SELECT ?1,Year,Month,COALESCE(OpeningBalanceMinutes,0),ExpectedMinutesOverride,COALESCE(OpeningBalanceWasEdited,0) FROM tidverk.Months"#,
                [&workspace_id],
            )?;
            transaction.execute(
                r#"INSERT INTO Projects (WorkspaceId,Id,Name,Color,IsActive,IsDefault)
                   SELECT ?1,Id,Name,'#5F875F',COALESCE(IsActive,1),COALESCE(IsDefault,0) FROM tidverk.Projects"#,
                [&workspace_id],
            )?;
            if source_project_count == 0 {
                transaction.execute(
                    r#"INSERT INTO Projects (WorkspaceId,Id,Name,Color,IsActive,IsDefault)
                       SELECT ?1,?2,COALESCE(DefaultProject,'General'),'#5F875F',1,1 FROM tidverk.Settings WHERE Id=1"#,
                    params![workspace_id, format!("proj-{}", Uuid::new_v4())],
                )?;
            }
            update_preferences(&transaction, &settings_columns, &workspace_id)?;
            transaction.commit()?;
            Ok(())
        })();
        let _ = connection.execute("DETACH DATABASE tidverk", []);
        import_result?;
        Ok(TidverkImportResult {
            workspace_id,
            workspace_name,
            entry_count,
            month_count,
            project_count: source_project_count.max(1),
            source_backup_path,
            safety_backup_path,
        })
    }
}

fn validate_tidverk(connection: &Connection) -> Result<()> {
    let integrity: String = connection.query_row("PRAGMA quick_check", [], |row| row.get(0))?;
    if integrity != "ok" {
        return Err(DataError::Integrity(integrity));
    }
    for table in ["Settings", "WorkEntries", "Months", "Projects"] {
        let exists: bool = connection.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
            [table],
            |row| row.get(0),
        )?;
        if !exists {
            return Err(DataError::NotTidverkDatabase);
        }
    }
    let has_settings: bool = connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM Settings WHERE Id=1)",
        [],
        |row| row.get(0),
    )?;
    if !has_settings {
        return Err(DataError::MissingTidverkSettings);
    }
    Ok(())
}

fn columns(connection: &Connection, table: &str) -> Result<BTreeSet<String>> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    Ok(statement
        .query_map([], |row| row.get(1))?
        .collect::<std::result::Result<_, _>>()?)
}

fn count(connection: &Connection, table: &str) -> Result<usize> {
    let count: i64 = connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| {
        row.get(0)
    })?;
    usize::try_from(count).map_err(|_| DataError::InvalidValue {
        column: "COUNT(*)",
        value: count.to_string(),
    })
}

fn expression(columns: &BTreeSet<String>, name: &str, fallback: &str) -> String {
    if columns.contains(name) {
        format!("COALESCE(s.{name},{fallback})")
    } else {
        fallback.to_owned()
    }
}

fn insert_settings(
    transaction: &rusqlite::Transaction<'_>,
    columns: &BTreeSet<String>,
    workspace_id: &str,
) -> Result<()> {
    let fields = [
        ("EmployeeName", "EmployeeName", "''"),
        ("EmployerName", "EmployerName", "''"),
        ("DefaultProject", "DefaultProject", "'General'"),
        ("HourlyRate", "HourlyRate", "0"),
        ("SalaryType", "SalaryType", "0"),
        ("MonthlySalary", "MonthlySalary", "0"),
        ("EmploymentPercent", "EmploymentPercent", "100"),
        ("HourlyPayBasis", "HourlyPayBasis", "0"),
        ("ExpectedHoursPerWorkday", "ExpectedHoursPerWorkday", "8"),
        (
            "ExpectedWorkingWeekdays",
            "ExpectedWorkingWeekdays",
            "'1,2,3,4,5'",
        ),
        ("ExcludePublicHolidays", "ExcludePublicHolidays", "0"),
        ("DefaultStartTime", "DefaultStartTime", "'08:00'"),
        ("DefaultEndTime", "DefaultEndTime", "'16:30'"),
        ("DefaultLunchMinutes", "DefaultLunchMinutes", "30"),
        ("TaxMode", "TaxMode", "0"),
        ("TaxYear", "TaxYear", "2026"),
        ("TaxTableNumber", "TaxTableNumber", "30"),
        ("TaxColumn", "TaxColumn", "1"),
        ("ManualTaxValue", "ManualTaxValue", "NULL"),
        ("OpeningBalanceMinutes", "OpeningBalanceMinutes", "0"),
        ("CurrencyPreference", "CurrencyPreference", "0"),
        ("ExportLanguagePreference", "ExportLanguagePreference", "2"),
        ("OvertimeCompensationMode", "OvertimeCompensationMode", "0"),
        ("OvertimePremiumPercent", "OvertimePremiumPercent", "50"),
        (
            "OvertimeDailyThresholdHours",
            "OvertimeDailyThresholdHours",
            "8",
        ),
        ("OvertimeThresholdMode", "OvertimeThresholdMode", "0"),
        ("OvertimeDefaultRateType", "OvertimeDefaultRateType", "0"),
        ("OvertimeRateBandsJson", "OvertimeRateBandsJson", "'[]'"),
        ("OvertimeObCombination", "ObOvertimeCombination", "0"),
    ];
    let values = fields
        .iter()
        .map(|(_, source, fallback)| expression(columns, source, fallback))
        .collect::<Vec<_>>()
        .join(",");
    let names = fields
        .iter()
        .map(|(target, _, _)| *target)
        .collect::<Vec<_>>()
        .join(",");
    transaction.execute(
        &format!("INSERT INTO WorkspaceSettings (WorkspaceId,{names}) SELECT ?1,{values} FROM tidverk.Settings s WHERE s.Id=1"),
        [workspace_id],
    )?;
    Ok(())
}

fn update_preferences(
    transaction: &rusqlite::Transaction<'_>,
    columns: &BTreeSet<String>,
    workspace_id: &str,
) -> Result<()> {
    let theme = expression(columns, "ThemePreference", "0");
    let language = expression(columns, "LanguagePreference", "0");
    let scale = expression(columns, "InterfaceScalePercent", "100");
    let view = expression(columns, "MonthViewPreference", "0");
    transaction.execute(
        &format!(
            r#"UPDATE AppPreferences SET ActiveWorkspaceId=?1,
            ThemePreference=(SELECT {theme} FROM tidverk.Settings s WHERE s.Id=1),
            LanguagePreference=(SELECT {language} FROM tidverk.Settings s WHERE s.Id=1),
            InterfaceScalePercent=(SELECT {scale} FROM tidverk.Settings s WHERE s.Id=1),
            MonthViewPreference=(SELECT {view} FROM tidverk.Settings s WHERE s.Id=1),
            HasCompletedSetup=1 WHERE Id=1"#
        ),
        [workspace_id],
    )?;
    Ok(())
}
