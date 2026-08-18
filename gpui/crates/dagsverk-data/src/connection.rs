use std::{
    path::{Path, PathBuf},
    str::FromStr,
    time::Duration,
};

use chrono::{DateTime, Datelike, Utc};
use dagsverk_core::{
    clock::Clock,
    models::{
        AppPreferences, AppSettings, BalanceHistoryMonth, ClockTime, CompensationRateType,
        CompensationRuleType, CurrencyPreference, ExpectedHoursSettings, ExportLanguagePreference,
        HourlyPayBasis, IsoDate, LanguagePreference, Minutes, Money, MonthRecord,
        MonthViewPreference, ObOvertimeCombinationMode, OvertimeCompensationMode,
        OvertimeCompensationSettings, OvertimeDayCategory, OvertimeRateBand, OvertimeThresholdMode,
        Project, ProjectId, SalarySettings, SalaryType, TaxMode, TaxSettings, ThemePreference,
        WorkEntry, WorkEntryStatus, Workspace, WorkspaceId, WorkspaceType, YearMonth,
    },
};
use rusqlite::{Connection, OptionalExtension, Row, params, types::ValueRef};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{DataError, Result, migration, schema};

pub struct Database<C> {
    pub(crate) path: PathBuf,
    pub(crate) clock: C,
}

impl<C: Clock> Database<C> {
    pub fn open(path: impl Into<PathBuf>, clock: C) -> Result<Self> {
        let path = path.into();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|source| DataError::CreateDirectory {
                path: parent.to_owned(),
                source,
            })?;
        }
        let database = Self { path, clock };
        let connection = database.connection()?;
        if migration::is_legacy(&connection)? {
            migration::migrate(
                &connection,
                &database.path,
                &database.clock.now_utc().to_rfc3339(),
            )?;
        } else {
            schema::initialize(&connection, &database.clock.now_utc().to_rfc3339())?;
        }
        Ok(database)
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn connection(&self) -> Result<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.busy_timeout(Duration::from_secs(5))?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        connection.pragma_update(None, "foreign_keys", true)?;
        Ok(connection)
    }

    pub fn validate(&self) -> Result<()> {
        schema::validate_path(&self.path)
    }

    pub fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        let connection = self.connection()?;
        let mut statement =
            connection.prepare("SELECT * FROM Workspaces ORDER BY CreatedAt ASC")?;
        statement
            .query_and_then([], workspace_from_row)?
            .collect::<Result<Vec<_>>>()
    }

    pub fn save_workspace(&self, workspace: &Workspace) -> Result<()> {
        let connection = self.connection()?;
        let transaction = connection.unchecked_transaction()?;
        let now = self.clock.now_utc().to_rfc3339();
        transaction.execute(
            r#"INSERT INTO Workspaces (Id, Name, Color, WorkspaceType, WorkerName, EmployerName, CreatedAt, UpdatedAt)
               VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
               ON CONFLICT(Id) DO UPDATE SET Name=excluded.Name, Color=excluded.Color,
               WorkspaceType=excluded.WorkspaceType, WorkerName=excluded.WorkerName,
               EmployerName=excluded.EmployerName, UpdatedAt=excluded.UpdatedAt"#,
            params![
                workspace.id.as_str(),
                workspace.name,
                workspace.color,
                i32::from(workspace.workspace_type),
                workspace.worker_name.as_deref().unwrap_or(""),
                workspace.organization_name.as_deref().unwrap_or(""),
                workspace.created_at.to_rfc3339(),
                now,
            ],
        )?;
        transaction.execute(
            "INSERT OR IGNORE INTO WorkspaceSettings (WorkspaceId, EmployeeName, EmployerName) VALUES (?1, ?2, ?3)",
            params![workspace.id.as_str(), workspace.worker_name.as_deref().unwrap_or(""), workspace.organization_name.as_deref().unwrap_or("")],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_workspace(&self, id: &WorkspaceId) -> Result<()> {
        let connection = self.connection()?;
        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM Workspaces", [], |row| row.get(0))?;
        if count <= 1 {
            return Err(DataError::LastWorkspace);
        }
        connection.execute("DELETE FROM Workspaces WHERE Id = ?1", [id.as_str()])?;
        Ok(())
    }

    pub fn load_preferences(&self) -> Result<AppPreferences> {
        let connection = self.connection()?;
        connection
            .query_row("SELECT * FROM AppPreferences WHERE Id = 1", [], |row| {
                Ok((
                    row.get::<_, String>("ActiveWorkspaceId")?,
                    row.get::<_, i64>("ThemePreference")?,
                    row.get::<_, i64>("LanguagePreference")?,
                    row.get::<_, i32>("InterfaceScalePercent")?,
                    row.get::<_, i64>("MonthViewPreference")?,
                    row.get::<_, bool>("HasCompletedSetup")?,
                ))
            })
            .map_err(DataError::from)
            .and_then(|row| {
                Ok(AppPreferences {
                    id: Some(1),
                    active_workspace_id: WorkspaceId::new(row.0)?,
                    theme_preference: ThemePreference::try_from(row.1)?,
                    language_preference: LanguagePreference::try_from(row.2)?,
                    interface_scale_percent: row.3,
                    month_view_preference: MonthViewPreference::try_from(row.4)?,
                    has_completed_setup: row.5,
                })
            })
    }

    pub fn save_preferences(&self, preferences: &AppPreferences) -> Result<()> {
        self.connection()?.execute(
            r#"INSERT INTO AppPreferences (Id, ActiveWorkspaceId, ThemePreference, LanguagePreference, InterfaceScalePercent, MonthViewPreference, HasCompletedSetup)
               VALUES (1, ?1, ?2, ?3, ?4, ?5, ?6)
               ON CONFLICT(Id) DO UPDATE SET ActiveWorkspaceId=excluded.ActiveWorkspaceId,
               ThemePreference=excluded.ThemePreference, LanguagePreference=excluded.LanguagePreference,
               InterfaceScalePercent=excluded.InterfaceScalePercent,
               MonthViewPreference=excluded.MonthViewPreference, HasCompletedSetup=excluded.HasCompletedSetup"#,
            params![
                preferences.active_workspace_id.as_str(),
                i32::from(preferences.theme_preference),
                i32::from(preferences.language_preference),
                preferences.interface_scale_percent,
                i32::from(preferences.month_view_preference),
                preferences.has_completed_setup,
            ],
        )?;
        Ok(())
    }

    pub fn load_settings(&self, workspace: &WorkspaceId) -> Result<AppSettings> {
        let connection = self.connection()?;
        connection.execute(
            "INSERT OR IGNORE INTO WorkspaceSettings (WorkspaceId) VALUES (?1)",
            [workspace.as_str()],
        )?;
        connection.query_row(
            "SELECT * FROM WorkspaceSettings WHERE WorkspaceId = ?1",
            [workspace.as_str()],
            |row| Ok(settings_raw(row)),
        )?
    }

    pub fn save_settings(&self, workspace: &WorkspaceId, settings: &AppSettings) -> Result<()> {
        let bands = serialize_rate_bands(&settings.overtime_compensation.rate_bands)?;
        self.connection()?.execute(
            r#"INSERT INTO WorkspaceSettings (
                WorkspaceId, EmployeeName, EmployerName, DefaultProject, HourlyRate, SalaryType,
                MonthlySalary, EmploymentPercent, HourlyPayBasis, ExpectedHoursPerWorkday,
                ExpectedWorkingWeekdays, ExcludePublicHolidays, DefaultStartTime, DefaultEndTime,
                DefaultLunchMinutes, TaxMode, TaxYear, TaxTableNumber, TaxColumn, ManualTaxValue,
                OpeningBalanceMinutes, CurrencyPreference, ExportLanguagePreference,
                OvertimeCompensationMode, OvertimePremiumPercent, OvertimeDailyThresholdHours,
                OvertimeThresholdMode, OvertimeDefaultRateType, OvertimeRateBandsJson,
                OvertimeObCombination
              ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17,?18,?19,?20,?21,?22,?23,?24,?25,?26,?27,?28,?29,?30)
              ON CONFLICT(WorkspaceId) DO UPDATE SET
                EmployeeName=excluded.EmployeeName, EmployerName=excluded.EmployerName,
                DefaultProject=excluded.DefaultProject, HourlyRate=excluded.HourlyRate,
                SalaryType=excluded.SalaryType, MonthlySalary=excluded.MonthlySalary,
                EmploymentPercent=excluded.EmploymentPercent, HourlyPayBasis=excluded.HourlyPayBasis,
                ExpectedHoursPerWorkday=excluded.ExpectedHoursPerWorkday,
                ExpectedWorkingWeekdays=excluded.ExpectedWorkingWeekdays,
                ExcludePublicHolidays=excluded.ExcludePublicHolidays,
                DefaultStartTime=excluded.DefaultStartTime, DefaultEndTime=excluded.DefaultEndTime,
                DefaultLunchMinutes=excluded.DefaultLunchMinutes, TaxMode=excluded.TaxMode,
                TaxYear=excluded.TaxYear, TaxTableNumber=excluded.TaxTableNumber,
                TaxColumn=excluded.TaxColumn, ManualTaxValue=excluded.ManualTaxValue,
                OpeningBalanceMinutes=excluded.OpeningBalanceMinutes,
                CurrencyPreference=excluded.CurrencyPreference,
                ExportLanguagePreference=excluded.ExportLanguagePreference,
                OvertimeCompensationMode=excluded.OvertimeCompensationMode,
                OvertimePremiumPercent=excluded.OvertimePremiumPercent,
                OvertimeDailyThresholdHours=excluded.OvertimeDailyThresholdHours,
                OvertimeThresholdMode=excluded.OvertimeThresholdMode,
                OvertimeDefaultRateType=excluded.OvertimeDefaultRateType,
                OvertimeRateBandsJson=excluded.OvertimeRateBandsJson,
                OvertimeObCombination=excluded.OvertimeObCombination"#,
            params![
                workspace.as_str(), settings.employee_name, settings.employer_name,
                settings.default_project, settings.salary.hourly_rate.decimal().to_string(),
                i32::from(settings.salary.salary_type), settings.salary.monthly_salary.decimal().to_string(),
                settings.salary.employment_percent.to_string(), i32::from(settings.salary.hourly_pay_basis),
                settings.expected_hours.hours_per_workday.to_string(),
                settings.expected_hours.working_weekdays.iter().map(u32::to_string).collect::<Vec<_>>().join(","),
                settings.expected_hours.exclude_public_holidays, settings.default_start_time.to_string(),
                settings.default_end_time.to_string(), settings.default_lunch_minutes.value(),
                i32::from(settings.tax_settings.mode), settings.tax_settings.tax_year,
                settings.tax_settings.table_number, settings.tax_settings.column,
                settings.tax_settings.manual_monthly_deduction.map(|value| value.decimal().to_string()),
                settings.opening_balance_minutes.value(), i32::from(settings.currency_preference),
                i32::from(settings.export_language_preference), i32::from(settings.overtime_compensation.mode),
                settings.overtime_compensation.default_rate_value.to_string(),
                settings.overtime_compensation.daily_threshold_hours.to_string(),
                i32::from(settings.overtime_compensation.threshold_mode),
                i32::from(settings.overtime_compensation.default_rate_type), bands,
                i32::from(settings.overtime_compensation.ob_overtime_combination),
            ],
        )?;
        Ok(())
    }

    pub fn load_entries(
        &self,
        workspace: &WorkspaceId,
        month: YearMonth,
    ) -> Result<Vec<WorkEntry>> {
        let connection = self.connection()?;
        let pattern = format!("{}-{:02}%", month.year, month.month);
        let mut statement = connection.prepare(
            "SELECT * FROM WorkEntries WHERE WorkspaceId = ?1 AND Date LIKE ?2 ORDER BY Date ASC",
        )?;
        statement
            .query_and_then(params![workspace.as_str(), pattern], work_entry_from_row)?
            .collect::<Result<Vec<_>>>()
    }

    pub fn save_entry(&self, workspace: &WorkspaceId, entry: &WorkEntry) -> Result<()> {
        let connection = self.connection()?;
        save_entry_on(
            &connection,
            workspace,
            entry,
            &self.clock.now_utc().to_rfc3339(),
        )
    }

    pub fn save_entries(&self, workspace: &WorkspaceId, entries: &[WorkEntry]) -> Result<()> {
        let connection = self.connection()?;
        let transaction = connection.unchecked_transaction()?;
        let now = self.clock.now_utc().to_rfc3339();
        for entry in entries {
            save_entry_on(&transaction, workspace, entry, &now)?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn delete_entry(&self, workspace: &WorkspaceId, date: IsoDate) -> Result<()> {
        self.connection()?.execute(
            "DELETE FROM WorkEntries WHERE WorkspaceId = ?1 AND Date = ?2",
            params![workspace.as_str(), date.to_string()],
        )?;
        Ok(())
    }

    pub fn load_month_record(
        &self,
        workspace: &WorkspaceId,
        month: YearMonth,
        default_opening: Minutes,
    ) -> Result<MonthRecord> {
        let connection = self.connection()?;
        connection
            .query_row(
                "SELECT * FROM MonthRecords WHERE WorkspaceId=?1 AND Year=?2 AND Month=?3",
                params![workspace.as_str(), month.year, month.month],
                |row| {
                    Ok(MonthRecord {
                        workspace_id: Some(workspace.clone()),
                        year: row.get("Year")?,
                        month: row.get("Month")?,
                        opening_balance_minutes: Minutes::new(row.get("OpeningBalanceMinutes")?),
                        expected_minutes_override: row
                            .get::<_, Option<i64>>("ExpectedMinutesOverride")?
                            .map(Minutes::new),
                        opening_balance_was_edited: row.get("OpeningBalanceWasEdited")?,
                    })
                },
            )
            .optional()?
            .map_or_else(
                || {
                    Ok(MonthRecord {
                        workspace_id: Some(workspace.clone()),
                        year: month.year,
                        month: month.month,
                        opening_balance_minutes: default_opening,
                        expected_minutes_override: None,
                        opening_balance_was_edited: false,
                    })
                },
                Ok,
            )
    }

    pub fn save_month_record(&self, workspace: &WorkspaceId, record: &MonthRecord) -> Result<()> {
        self.connection()?.execute(
            r#"INSERT INTO MonthRecords (WorkspaceId, Year, Month, OpeningBalanceMinutes, ExpectedMinutesOverride, OpeningBalanceWasEdited)
               VALUES (?1,?2,?3,?4,?5,?6) ON CONFLICT(WorkspaceId,Year,Month) DO UPDATE SET
               OpeningBalanceMinutes=excluded.OpeningBalanceMinutes,
               ExpectedMinutesOverride=excluded.ExpectedMinutesOverride,
               OpeningBalanceWasEdited=excluded.OpeningBalanceWasEdited"#,
            params![workspace.as_str(), record.year, record.month, record.opening_balance_minutes.value(), record.expected_minutes_override.map(Minutes::value), record.opening_balance_was_edited],
        )?;
        Ok(())
    }

    pub fn reset_month(&self, workspace: &WorkspaceId, month: YearMonth) -> Result<()> {
        let connection = self.connection()?;
        let transaction = connection.unchecked_transaction()?;
        transaction.execute(
            "DELETE FROM WorkEntries WHERE WorkspaceId=?1 AND Date LIKE ?2",
            params![
                workspace.as_str(),
                format!("{}-{:02}%", month.year, month.month)
            ],
        )?;
        transaction.execute(
            "DELETE FROM MonthRecords WHERE WorkspaceId=?1 AND Year=?2 AND Month=?3",
            params![workspace.as_str(), month.year, month.month],
        )?;
        transaction.commit()?;
        Ok(())
    }

    pub fn load_balance_history(
        &self,
        workspace: &WorkspaceId,
        before: YearMonth,
    ) -> Result<Vec<BalanceHistoryMonth>> {
        let connection = self.connection()?;
        let before_text = format!("{}-{:02}", before.year, before.month);
        let mut months = std::collections::BTreeMap::new();
        let mut records = connection.prepare(
            "SELECT * FROM MonthRecords WHERE WorkspaceId=?1 AND printf('%04d-%02d',Year,Month) < ?2",
        )?;
        for record in records.query_map(params![workspace.as_str(), before_text], |row| {
            Ok(MonthRecord {
                workspace_id: Some(workspace.clone()),
                year: row.get("Year")?,
                month: row.get("Month")?,
                opening_balance_minutes: Minutes::new(row.get("OpeningBalanceMinutes")?),
                expected_minutes_override: row
                    .get::<_, Option<i64>>("ExpectedMinutesOverride")?
                    .map(Minutes::new),
                opening_balance_was_edited: row.get("OpeningBalanceWasEdited")?,
            })
        })? {
            let record = record?;
            months.insert(
                (record.year, record.month),
                BalanceHistoryMonth {
                    year: record.year,
                    month: record.month,
                    record: Some(record),
                    entries: Vec::new(),
                },
            );
        }
        let mut entries = connection.prepare(
            "SELECT * FROM WorkEntries WHERE WorkspaceId=?1 AND substr(Date,1,7) < ?2 AND Status <> 0 ORDER BY Date",
        )?;
        for entry in entries.query_and_then(
            params![workspace.as_str(), before_text],
            work_entry_from_row,
        )? {
            let entry = entry?;
            let key = (
                entry.date.as_naive_date().year(),
                entry.date.as_naive_date().month(),
            );
            months
                .entry(key)
                .or_insert_with(|| BalanceHistoryMonth {
                    year: key.0,
                    month: key.1,
                    record: None,
                    entries: Vec::new(),
                })
                .entries
                .push(entry);
        }
        let mut result: Vec<_> = months.into_values().collect();
        if result.len() > 120 {
            result.drain(..result.len() - 120);
        }
        Ok(result)
    }

    pub fn list_projects(&self, workspace: &WorkspaceId) -> Result<Vec<Project>> {
        let connection = self.connection()?;
        let mut statement =
            connection.prepare("SELECT * FROM Projects WHERE WorkspaceId=?1 ORDER BY Name ASC")?;
        statement
            .query_and_then([workspace.as_str()], |row| {
                Ok(Project {
                    workspace_id: Some(WorkspaceId::new(row.get::<_, String>("WorkspaceId")?)?),
                    id: ProjectId::new(row.get::<_, String>("Id")?)?,
                    name: row.get("Name")?,
                    color: row
                        .get::<_, Option<String>>("Color")?
                        .or_else(|| Some("#5F875F".to_owned())),
                    is_active: row.get("IsActive")?,
                    is_default: row.get("IsDefault")?,
                })
            })?
            .collect::<Result<Vec<_>>>()
    }

    pub fn save_project(&self, workspace: &WorkspaceId, project: &Project) -> Result<()> {
        self.connection()?.execute(
            r#"INSERT INTO Projects (WorkspaceId,Id,Name,Color,IsActive,IsDefault) VALUES (?1,?2,?3,?4,?5,?6)
               ON CONFLICT(WorkspaceId,Id) DO UPDATE SET Name=excluded.Name,Color=excluded.Color,IsActive=excluded.IsActive,IsDefault=excluded.IsDefault"#,
            params![workspace.as_str(), project.id.as_str(), project.name, project.color.as_deref().unwrap_or("#5F875F"), project.is_active, project.is_default],
        )?;
        Ok(())
    }

    pub fn delete_project(&self, workspace: &WorkspaceId, id: &ProjectId) -> Result<()> {
        self.connection()?.execute(
            "DELETE FROM Projects WHERE WorkspaceId=?1 AND Id=?2",
            params![workspace.as_str(), id.as_str()],
        )?;
        Ok(())
    }
}

fn workspace_from_row(row: &Row<'_>) -> Result<Workspace> {
    Ok(Workspace {
        id: WorkspaceId::new(row.get::<_, String>("Id")?)?,
        name: row.get("Name")?,
        color: row.get("Color")?,
        workspace_type: WorkspaceType::try_from(row.get::<_, i64>("WorkspaceType")?)?,
        worker_name: nonempty(row.get("WorkerName")?),
        organization_name: nonempty(row.get("EmployerName")?),
        created_at: timestamp(row, "CreatedAt")?,
        updated_at: timestamp(row, "UpdatedAt")?,
    })
}

fn work_entry_from_row(row: &Row<'_>) -> Result<WorkEntry> {
    Ok(WorkEntry {
        workspace_id: Some(WorkspaceId::new(row.get::<_, String>("WorkspaceId")?)?),
        date: row.get::<_, String>("Date")?.parse()?,
        status: WorkEntryStatus::try_from(row.get::<_, i64>("Status")?)?,
        start_time: optional_time(row.get("StartTime")?)?,
        end_time: optional_time(row.get("EndTime")?)?,
        lunch_minutes: Minutes::new(row.get("LunchMinutes")?),
        project_name: row.get("ProjectName")?,
        notes: row.get("Notes")?,
        scheduled_minutes_override: row
            .get::<_, Option<i64>>("ScheduledMinutesOverride")?
            .map(Minutes::new),
        created_at: optional_timestamp(row, "CreatedAt")?,
        updated_at: optional_timestamp(row, "UpdatedAt")?,
    })
}

fn save_entry_on(
    connection: &Connection,
    workspace: &WorkspaceId,
    entry: &WorkEntry,
    now: &str,
) -> Result<()> {
    connection.execute(
        r#"INSERT INTO WorkEntries (WorkspaceId,Date,Status,StartTime,EndTime,LunchMinutes,ProjectName,Notes,ScheduledMinutesOverride,CreatedAt,UpdatedAt)
           VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11)
           ON CONFLICT(WorkspaceId,Date) DO UPDATE SET Status=excluded.Status,
           StartTime=excluded.StartTime,EndTime=excluded.EndTime,LunchMinutes=excluded.LunchMinutes,
           ProjectName=excluded.ProjectName,Notes=excluded.Notes,
           ScheduledMinutesOverride=excluded.ScheduledMinutesOverride,UpdatedAt=excluded.UpdatedAt"#,
        params![
            workspace.as_str(), entry.date.to_string(), i32::from(entry.status),
            entry.start_time.map(|value| value.to_string()), entry.end_time.map(|value| value.to_string()),
            entry.lunch_minutes.value(), entry.project_name, entry.notes,
            entry.scheduled_minutes_override.map(Minutes::value),
            entry.created_at.map_or_else(|| now.to_owned(), |value| value.to_rfc3339()),
            entry.updated_at.map_or_else(|| now.to_owned(), |value| value.to_rfc3339()),
        ],
    )?;
    Ok(())
}

fn settings_raw(row: &Row<'_>) -> Result<AppSettings> {
    let bands_json: String = row.get("OvertimeRateBandsJson")?;
    Ok(AppSettings {
        id: None,
        workspace_id: Some(WorkspaceId::new(row.get::<_, String>("WorkspaceId")?)?),
        employee_name: row.get("EmployeeName")?,
        employer_name: row.get("EmployerName")?,
        default_project: row.get("DefaultProject")?,
        salary: SalarySettings {
            salary_type: SalaryType::try_from(row.get::<_, i64>("SalaryType")?)?,
            hourly_rate: Money::new(decimal(row, "HourlyRate")?),
            monthly_salary: Money::new(decimal(row, "MonthlySalary")?),
            employment_percent: decimal(row, "EmploymentPercent")?,
            hourly_pay_basis: HourlyPayBasis::try_from(row.get::<_, i64>("HourlyPayBasis")?)?,
        },
        expected_hours: ExpectedHoursSettings {
            hours_per_workday: decimal(row, "ExpectedHoursPerWorkday")?,
            working_weekdays: row
                .get::<_, String>("ExpectedWorkingWeekdays")?
                .split(',')
                .map(|value| {
                    value.parse::<u32>().map_err(|_| DataError::InvalidValue {
                        column: "ExpectedWorkingWeekdays",
                        value: value.to_owned(),
                    })
                })
                .collect::<Result<Vec<_>>>()?,
            exclude_public_holidays: row.get("ExcludePublicHolidays")?,
        },
        default_start_time: row.get::<_, String>("DefaultStartTime")?.parse()?,
        default_end_time: row.get::<_, String>("DefaultEndTime")?.parse()?,
        default_lunch_minutes: Minutes::new(row.get("DefaultLunchMinutes")?),
        tax_settings: TaxSettings {
            mode: TaxMode::try_from(row.get::<_, i64>("TaxMode")?)?,
            tax_year: row.get("TaxYear")?,
            table_number: row.get("TaxTableNumber")?,
            column: row.get("TaxColumn")?,
            manual_monthly_deduction: optional_decimal(row, "ManualTaxValue")?.map(Money::new),
        },
        theme_preference: ThemePreference::System,
        opening_balance_minutes: Minutes::new(row.get("OpeningBalanceMinutes")?),
        month_view_preference: MonthViewPreference::Ledger,
        language_preference: LanguagePreference::System,
        currency_preference: CurrencyPreference::try_from(
            row.get::<_, i64>("CurrencyPreference")?,
        )?,
        interface_scale_percent: 100,
        export_language_preference: ExportLanguagePreference::try_from(
            row.get::<_, i64>("ExportLanguagePreference")?,
        )?,
        overtime_compensation: OvertimeCompensationSettings {
            mode: OvertimeCompensationMode::try_from(
                row.get::<_, i64>("OvertimeCompensationMode")?,
            )?,
            default_rate_type: CompensationRateType::try_from(
                row.get::<_, i64>("OvertimeDefaultRateType")?,
            )?,
            default_rate_value: decimal(row, "OvertimePremiumPercent")?,
            daily_threshold_hours: decimal(row, "OvertimeDailyThresholdHours")?,
            threshold_mode: OvertimeThresholdMode::try_from(
                row.get::<_, i64>("OvertimeThresholdMode")?,
            )?,
            rate_bands: deserialize_rate_bands(&bands_json)?,
            ob_overtime_combination: ObOvertimeCombinationMode::try_from(
                row.get::<_, i64>("OvertimeObCombination")?,
            )?,
        },
    })
}

fn decimal(row: &Row<'_>, column: &'static str) -> Result<Decimal> {
    let value = match row.get_ref(column)? {
        ValueRef::Integer(value) => value.to_string(),
        ValueRef::Real(value) => value.to_string(),
        ValueRef::Text(value) => String::from_utf8_lossy(value).into_owned(),
        other => {
            return Err(DataError::Decimal {
                column,
                value: format!("{other:?}"),
            });
        }
    };
    Decimal::from_str(&value).map_err(|_| DataError::Decimal { column, value })
}

fn optional_decimal(row: &Row<'_>, column: &'static str) -> Result<Option<Decimal>> {
    if matches!(row.get_ref(column)?, ValueRef::Null) {
        Ok(None)
    } else {
        decimal(row, column).map(Some)
    }
}

fn timestamp(row: &Row<'_>, column: &'static str) -> Result<DateTime<Utc>> {
    let value: String = row.get(column)?;
    DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|_| DataError::Timestamp { column, value })
}

fn optional_timestamp(row: &Row<'_>, column: &'static str) -> Result<Option<DateTime<Utc>>> {
    let value: Option<String> = row.get(column)?;
    value
        .map(|value| {
            DateTime::parse_from_rfc3339(&value)
                .map(|date| date.with_timezone(&Utc))
                .map_err(|_| DataError::Timestamp { column, value })
        })
        .transpose()
}

fn optional_time(value: Option<String>) -> Result<Option<ClockTime>> {
    value
        .map(|value| value.parse().map_err(DataError::from))
        .transpose()
}

fn nonempty(value: String) -> Option<String> {
    (!value.is_empty()).then_some(value)
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DbRateBand {
    name: String,
    day_category: OvertimeDayCategory,
    start_time: ClockTime,
    end_time: ClockTime,
    compensation_type: CompensationRuleType,
    rate_type: CompensationRateType,
    rate_value: serde_json::Number,
}

fn serialize_rate_bands(bands: &[OvertimeRateBand]) -> Result<String> {
    let values = bands
        .iter()
        .map(|band| {
            Ok(DbRateBand {
                name: band.name.clone(),
                day_category: band.day_category,
                start_time: band.start_time,
                end_time: band.end_time,
                compensation_type: band.compensation_type,
                rate_type: band.rate_type,
                rate_value: serde_json::Number::from_str(&band.rate_value.to_string())?,
            })
        })
        .collect::<serde_json::Result<Vec<_>>>()?;
    Ok(serde_json::to_string(&values)?)
}

fn deserialize_rate_bands(json: &str) -> Result<Vec<OvertimeRateBand>> {
    serde_json::from_str::<Vec<DbRateBand>>(json)?
        .into_iter()
        .map(|band| {
            Ok(OvertimeRateBand {
                name: band.name,
                day_category: band.day_category,
                start_time: band.start_time,
                end_time: band.end_time,
                compensation_type: band.compensation_type,
                rate_type: band.rate_type,
                rate_value: Decimal::from_str(band.rate_value.as_str()).map_err(|_| {
                    DataError::Decimal {
                        column: "OvertimeRateBandsJson",
                        value: band.rate_value.to_string(),
                    }
                })?,
            })
        })
        .collect()
}
