use std::{fs, path::Path};

use rusqlite::Connection;
use uuid::Uuid;

use crate::{
    DataError, Result,
    backup::{backup_connection, prune_backups},
    schema,
};

pub fn is_legacy(connection: &Connection) -> Result<bool> {
    Ok(!table_exists(connection, "Workspaces")? && table_exists(connection, "WorkEntries")?)
}

pub fn migrate(connection: &Connection, database_path: &Path, now: &str) -> Result<()> {
    for table in ["Settings", "WorkEntries", "MonthRecords", "Projects"] {
        if !table_exists(connection, table)? {
            return Err(DataError::InvalidValue {
                column: "legacy schema",
                value: format!("missing table {table}"),
            });
        }
    }
    create_safety_backup(connection, database_path, now)?;
    connection.pragma_update(None, "foreign_keys", false)?;
    let transaction = connection.unchecked_transaction()?;
    transaction.execute_batch(
        r#"
        ALTER TABLE WorkEntries RENAME TO Old_WorkEntries;
        ALTER TABLE MonthRecords RENAME TO Old_MonthRecords;
        ALTER TABLE Projects RENAME TO Old_Projects;
        ALTER TABLE Settings RENAME TO Old_Settings;
        "#,
    )?;
    schema::initialize(&transaction, now)?;
    transaction.execute("DELETE FROM Projects WHERE WorkspaceId='ws-default'", [])?;
    transaction.execute_batch(
        r#"
        UPDATE AppPreferences SET
          ThemePreference=COALESCE((SELECT ThemePreference FROM Old_Settings WHERE Id=1),0),
          LanguagePreference=COALESCE((SELECT LanguagePreference FROM Old_Settings WHERE Id=1),0),
          InterfaceScalePercent=COALESCE((SELECT InterfaceScalePercent FROM Old_Settings WHERE Id=1),100),
          MonthViewPreference=COALESCE((SELECT MonthViewPreference FROM Old_Settings WHERE Id=1),0),
          HasCompletedSetup=1
        WHERE Id=1;

        UPDATE WorkspaceSettings SET
          EmployeeName=COALESCE((SELECT EmployeeName FROM Old_Settings WHERE Id=1),''),
          EmployerName=COALESCE((SELECT EmployerName FROM Old_Settings WHERE Id=1),''),
          DefaultProject=COALESCE((SELECT DefaultProject FROM Old_Settings WHERE Id=1),'General'),
          HourlyRate=COALESCE((SELECT HourlyRate FROM Old_Settings WHERE Id=1),250),
          SalaryType=COALESCE((SELECT SalaryType FROM Old_Settings WHERE Id=1),0),
          MonthlySalary=COALESCE((SELECT MonthlySalary FROM Old_Settings WHERE Id=1),40000),
          EmploymentPercent=COALESCE((SELECT EmploymentPercent FROM Old_Settings WHERE Id=1),100),
          ExpectedHoursPerWorkday=COALESCE((SELECT ExpectedHoursPerWorkday FROM Old_Settings WHERE Id=1),8),
          ExpectedWorkingWeekdays=COALESCE((SELECT ExpectedWorkingWeekdays FROM Old_Settings WHERE Id=1),'1,2,3,4,5'),
          ExcludePublicHolidays=COALESCE((SELECT ExcludePublicHolidays FROM Old_Settings WHERE Id=1),1),
          DefaultStartTime=COALESCE((SELECT DefaultStartTime FROM Old_Settings WHERE Id=1),'08:00'),
          DefaultEndTime=COALESCE((SELECT DefaultEndTime FROM Old_Settings WHERE Id=1),'16:30'),
          DefaultLunchMinutes=COALESCE((SELECT DefaultLunchMinutes FROM Old_Settings WHERE Id=1),30),
          TaxMode=COALESCE((SELECT TaxMode FROM Old_Settings WHERE Id=1),1),
          TaxYear=COALESCE((SELECT TaxYear FROM Old_Settings WHERE Id=1),2026),
          TaxTableNumber=COALESCE((SELECT TaxTableNumber FROM Old_Settings WHERE Id=1),30),
          TaxColumn=COALESCE((SELECT TaxColumn FROM Old_Settings WHERE Id=1),1),
          ManualTaxValue=(SELECT ManualTaxValue FROM Old_Settings WHERE Id=1),
          OpeningBalanceMinutes=COALESCE((SELECT OpeningBalanceMinutes FROM Old_Settings WHERE Id=1),0),
          CurrencyPreference=COALESCE((SELECT CurrencyPreference FROM Old_Settings WHERE Id=1),0),
          ExportLanguagePreference=COALESCE((SELECT ExportLanguagePreference FROM Old_Settings WHERE Id=1),2),
          OvertimeCompensationMode=COALESCE((SELECT OvertimeCompensationMode FROM Old_Settings WHERE Id=1),0),
          OvertimePremiumPercent=COALESCE((SELECT OvertimePremiumPercent FROM Old_Settings WHERE Id=1),50),
          OvertimeDailyThresholdHours=COALESCE((SELECT OvertimeDailyThresholdHours FROM Old_Settings WHERE Id=1),8),
          OvertimeThresholdMode=COALESCE((SELECT OvertimeThresholdMode FROM Old_Settings WHERE Id=1),0),
          OvertimeDefaultRateType=COALESCE((SELECT OvertimeDefaultRateType FROM Old_Settings WHERE Id=1),0),
          OvertimeRateBandsJson=COALESCE((SELECT OvertimeRateBandsJson FROM Old_Settings WHERE Id=1),'[]')
        WHERE WorkspaceId='ws-default';

        INSERT INTO WorkEntries (WorkspaceId,Date,Status,StartTime,EndTime,LunchMinutes,ProjectName,Notes,ScheduledMinutesOverride,CreatedAt,UpdatedAt)
        SELECT 'ws-default',Date,Status,StartTime,EndTime,LunchMinutes,ProjectName,Notes,ScheduledMinutesOverride,CreatedAt,UpdatedAt FROM Old_WorkEntries;
        INSERT INTO MonthRecords (WorkspaceId,Year,Month,OpeningBalanceMinutes,ExpectedMinutesOverride,OpeningBalanceWasEdited)
        SELECT 'ws-default',Year,Month,OpeningBalanceMinutes,ExpectedMinutesOverride,OpeningBalanceWasEdited FROM Old_MonthRecords;
        INSERT INTO Projects (WorkspaceId,Id,Name,Color,IsActive,IsDefault)
        SELECT 'ws-default',Id,Name,'#5F875F',IsActive,IsDefault FROM Old_Projects;

        DROP TABLE Old_WorkEntries;
        DROP TABLE Old_MonthRecords;
        DROP TABLE Old_Projects;
        DROP TABLE Old_Settings;
        "#,
    )?;
    transaction.commit()?;
    connection.pragma_update(None, "foreign_keys", true)?;
    schema::validate_path(database_path)
}

fn table_exists(connection: &Connection, table: &str) -> Result<bool> {
    Ok(connection.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name=?1)",
        [table],
        |row| row.get(0),
    )?)
}

fn create_safety_backup(connection: &Connection, database_path: &Path, now: &str) -> Result<()> {
    let folder = database_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join("backups");
    fs::create_dir_all(&folder).map_err(|source| DataError::CreateDirectory {
        path: folder.clone(),
        source,
    })?;
    connection.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
    let timestamp = now.replace([':', '.'], "-");
    let path = folder.join(format!(
        "dagsverk-backup-{timestamp}-{}-before-migration.db",
        Uuid::new_v4()
    ));
    backup_connection(connection, &path)?;
    prune_backups(&folder)
}
