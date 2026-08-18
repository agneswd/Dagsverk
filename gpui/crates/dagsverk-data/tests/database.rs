use std::path::Path;

use chrono::{TimeZone, Utc};
use dagsverk_core::{
    clock::FixedClock,
    models::{
        CompensationRateType, CompensationRuleType, Minutes, MonthRecord, OvertimeDayCategory,
        OvertimeRateBand, OvertimeThresholdMode, Project, ProjectId, WorkEntry, WorkEntryStatus,
        Workspace, WorkspaceId, WorkspaceType, YearMonth, default_settings,
    },
};
use dagsverk_data::{
    DataError, Database,
    paths::{DataPathOptions, Platform, database_path},
};
use rust_decimal::Decimal;
use tempfile::tempdir;

fn database() -> (tempfile::TempDir, Database<FixedClock>) {
    let directory = tempdir().expect("temporary directory");
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 8, 18, 10, 0, 0).unwrap());
    let database = Database::open(directory.path().join("dagsverk.db"), clock).expect("database");
    (directory, database)
}

fn workspace(id: &str, name: &str) -> Workspace {
    let timestamp = Utc.with_ymd_and_hms(2026, 8, 18, 10, 0, 0).unwrap();
    Workspace {
        id: WorkspaceId::new(id).expect("workspace id"),
        name: name.to_owned(),
        color: "#123456".to_owned(),
        workspace_type: WorkspaceType::Contract,
        organization_name: Some("Client".to_owned()),
        worker_name: Some("Worker".to_owned()),
        created_at: timestamp,
        updated_at: timestamp,
    }
}

fn entry(date: &str, minutes_override: Option<i64>) -> WorkEntry {
    WorkEntry {
        workspace_id: None,
        date: date.parse().expect("date"),
        status: WorkEntryStatus::Worked,
        start_time: Some("08:00".parse().expect("time")),
        end_time: Some("17:00".parse().expect("time")),
        lunch_minutes: Minutes::new(30),
        project_name: Some("General".to_owned()),
        notes: Some("fixture".to_owned()),
        scheduled_minutes_override: minutes_override.map(Minutes::new),
        created_at: None,
        updated_at: None,
    }
}

fn electron_database_defaults() -> dagsverk_core::models::AppSettings {
    let mut settings = default_settings();
    settings.overtime_compensation.threshold_mode = OvertimeThresholdMode::FixedDailyHours;
    settings
}

#[test]
fn data_paths_match_electron_and_override_precedence() {
    let database = Path::new("/override/file.db");
    let cli_directory = Path::new("/cli");
    let environment_directory = Path::new("/environment");
    assert_eq!(
        database_path(
            Platform::Linux,
            &DataPathOptions {
                database: Some(database),
                data_dir: Some(cli_directory),
                environment_data_dir: Some(environment_directory),
                ..DataPathOptions::default()
            }
        ),
        Some(database.to_owned())
    );
    assert_eq!(
        database_path(
            Platform::Windows,
            &DataPathOptions {
                app_data: Some(Path::new("C:/Users/test/AppData/Roaming")),
                ..DataPathOptions::default()
            }
        ),
        Some(Path::new("C:/Users/test/AppData/Roaming/Dagsverk/dagsverk.db").to_owned())
    );
    assert_eq!(
        database_path(
            Platform::Linux,
            &DataPathOptions {
                home: Some(Path::new("/home/test")),
                ..DataPathOptions::default()
            }
        ),
        Some(Path::new("/home/test/.config/Dagsverk/dagsverk.db").to_owned())
    );
}

#[test]
fn new_database_has_exact_defaults_and_pragmas() {
    let (_directory, database) = database();
    database.validate().expect("valid Dagsverk database");
    let connection = database.connection().expect("connection");
    assert_eq!(
        connection
            .query_row("PRAGMA journal_mode", [], |row| row.get::<_, String>(0))
            .expect("journal mode")
            .to_ascii_lowercase(),
        "wal"
    );
    assert_eq!(
        connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .expect("foreign keys"),
        1
    );
    let workspaces = database.list_workspaces().expect("workspaces");
    assert_eq!(workspaces.len(), 1);
    assert_eq!(workspaces[0].id.as_str(), "ws-default");
    assert_eq!(workspaces[0].name, "Main Workspace");
    let preferences = database.load_preferences().expect("preferences");
    assert_eq!(preferences.active_workspace_id.as_str(), "ws-default");
    assert!(!preferences.has_completed_setup);
    let settings = database
        .load_settings(&preferences.active_workspace_id)
        .expect("settings");
    assert_eq!(settings, electron_database_defaults());
    let projects = database
        .list_projects(&preferences.active_workspace_id)
        .expect("projects");
    assert_eq!(projects.len(), 1);
    assert!(projects[0].is_default);
}

#[test]
fn settings_and_repositories_round_trip_with_workspace_isolation() {
    let (_directory, database) = database();
    let first = WorkspaceId::new("ws-default").expect("workspace");
    let second_workspace = workspace("ws-second", "Second");
    let second = second_workspace.id.clone();
    database
        .save_workspace(&second_workspace)
        .expect("save workspace");

    let mut settings = database.load_settings(&second).expect("settings");
    settings.employee_name = "Employee".to_owned();
    settings.salary.hourly_rate = dagsverk_core::models::Money::new(Decimal::new(12345, 2));
    settings.expected_hours.working_weekdays = vec![0, 2, 4, 6];
    settings.overtime_compensation.rate_bands = vec![OvertimeRateBand {
        name: "Night".to_owned(),
        day_category: OvertimeDayCategory::MajorHolidays,
        start_time: "22:00".parse().expect("time"),
        end_time: "06:00".parse().expect("time"),
        compensation_type: CompensationRuleType::Ob,
        rate_type: CompensationRateType::FixedHourlyAmount,
        rate_value: Decimal::new(505, 1),
    }];
    database
        .save_settings(&second, &settings)
        .expect("save settings");
    assert_eq!(
        database.load_settings(&second).expect("load settings"),
        settings
    );
    assert_eq!(
        database.load_settings(&first).expect("first settings"),
        electron_database_defaults()
    );

    database
        .save_entries(
            &second,
            &[entry("2026-08-17", Some(0)), entry("2026-08-18", None)],
        )
        .expect("save entries");
    database
        .save_entry(&first, &entry("2026-08-17", None))
        .expect("save first entry");
    assert_eq!(
        database
            .load_entries(&second, YearMonth::new(2026, 8).expect("month"))
            .expect("entries")
            .len(),
        2
    );
    assert_eq!(
        database
            .load_entries(&first, YearMonth::new(2026, 8).expect("month"))
            .expect("entries")
            .len(),
        1
    );
    assert_eq!(
        database
            .load_entries(&second, YearMonth::new(2026, 8).expect("month"))
            .expect("entries")[0]
            .scheduled_minutes_override,
        Some(Minutes::ZERO)
    );

    let record = MonthRecord {
        workspace_id: Some(second.clone()),
        year: 2026,
        month: 8,
        opening_balance_minutes: Minutes::new(90),
        expected_minutes_override: Some(Minutes::new(6000)),
        opening_balance_was_edited: true,
    };
    database
        .save_month_record(&second, &record)
        .expect("month record");
    assert_eq!(
        database
            .load_month_record(
                &second,
                YearMonth::new(2026, 8).expect("month"),
                Minutes::ZERO,
            )
            .expect("month record"),
        record
    );
    assert_eq!(
        database
            .load_balance_history(&second, YearMonth::new(2026, 9).expect("month"))
            .expect("history")
            .len(),
        1
    );

    let project = Project {
        workspace_id: Some(second.clone()),
        id: ProjectId::new("project-two").expect("project"),
        name: "Project Two".to_owned(),
        color: Some("#abcdef".to_owned()),
        is_active: true,
        is_default: false,
    };
    database
        .save_project(&second, &project)
        .expect("save project");
    assert!(
        database
            .list_projects(&second)
            .expect("projects")
            .contains(&project)
    );
    database
        .delete_project(&second, &project.id)
        .expect("delete project");
    assert!(
        !database
            .list_projects(&second)
            .expect("projects")
            .contains(&project)
    );

    database
        .reset_month(&second, YearMonth::new(2026, 8).expect("month"))
        .expect("reset month");
    assert!(
        database
            .load_entries(&second, YearMonth::new(2026, 8).expect("month"))
            .expect("entries")
            .is_empty()
    );
    assert!(
        !database
            .load_entries(&first, YearMonth::new(2026, 8).expect("month"))
            .expect("entries")
            .is_empty()
    );

    database
        .delete_workspace(&second)
        .expect("delete workspace");
    assert!(matches!(
        database.delete_workspace(&first),
        Err(DataError::LastWorkspace)
    ));
}
