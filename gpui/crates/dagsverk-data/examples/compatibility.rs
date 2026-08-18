use std::{env, path::Path};

use chrono::{TimeZone, Utc};
use dagsverk_core::{
    clock::FixedClock,
    models::{
        Minutes, MonthRecord, Project, ProjectId, WorkEntry, WorkEntryStatus, Workspace,
        WorkspaceId, WorkspaceType, YearMonth,
    },
};
use dagsverk_data::Database;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 3 {
        return Err("usage: compatibility <mode> <electron-db> <rust-db>".into());
    }
    let clock = FixedClock::new(Utc.with_ymd_and_hms(2026, 8, 18, 12, 0, 0).unwrap());
    match arguments[0].as_str() {
        "exchange" => exchange(Path::new(&arguments[1]), Path::new(&arguments[2]), clock),
        "assert-round-trip" => assert_round_trip(Path::new(&arguments[2]), clock),
        mode => Err(format!("unknown mode: {mode}").into()),
    }
}

fn exchange(
    electron_path: &Path,
    rust_path: &Path,
    clock: FixedClock,
) -> Result<(), Box<dyn std::error::Error>> {
    let electron = Database::open(electron_path, clock)?;
    let electron_workspace = WorkspaceId::new("electron-workspace")?;
    let workspaces = electron.list_workspaces()?;
    assert!(workspaces.iter().any(|workspace| {
        workspace.id == electron_workspace && workspace.name == "Electron Workspace"
    }));
    let settings = electron.load_settings(&electron_workspace)?;
    assert_eq!(settings.employee_name, "Electron Worker");
    assert_eq!(settings.salary.hourly_rate.decimal().to_string(), "321.5");
    assert_eq!(
        settings.overtime_compensation.rate_bands[0]
            .rate_value
            .to_string(),
        "55.5"
    );
    assert_eq!(
        electron.load_entries(&electron_workspace, YearMonth::new(2026, 8)?)?[0]
            .scheduled_minutes_override,
        Some(Minutes::ZERO)
    );
    assert_eq!(electron.list_projects(&electron_workspace)?.len(), 1);
    assert!(
        electron
            .load_month_record(&electron_workspace, YearMonth::new(2026, 8)?, Minutes::ZERO,)?
            .opening_balance_was_edited
    );
    electron.save_entry(
        &electron_workspace,
        &entry("2026-08-18", WorkEntryStatus::Worked, "rust-created"),
    )?;

    let rust = Database::open(rust_path, clock)?;
    let timestamp = Utc.with_ymd_and_hms(2026, 8, 18, 12, 0, 0).unwrap();
    let workspace = Workspace {
        id: WorkspaceId::new("rust-workspace")?,
        name: "Rust Workspace".to_owned(),
        color: "#654321".to_owned(),
        workspace_type: WorkspaceType::Personal,
        organization_name: None,
        worker_name: Some("Rust Worker".to_owned()),
        created_at: timestamp,
        updated_at: timestamp,
    };
    rust.save_workspace(&workspace)?;
    rust.save_entry(
        &workspace.id,
        &entry("2026-08-17", WorkEntryStatus::Worked, "rust-created"),
    )?;
    rust.save_month_record(
        &workspace.id,
        &MonthRecord {
            workspace_id: Some(workspace.id.clone()),
            year: 2026,
            month: 8,
            opening_balance_minutes: Minutes::new(90),
            expected_minutes_override: Some(Minutes::new(6000)),
            opening_balance_was_edited: true,
        },
    )?;
    rust.save_project(
        &workspace.id,
        &Project {
            workspace_id: Some(workspace.id.clone()),
            id: ProjectId::new("rust-project")?,
            name: "Rust Project".to_owned(),
            color: Some("#fedcba".to_owned()),
            is_active: true,
            is_default: true,
        },
    )?;
    Ok(())
}

fn assert_round_trip(
    rust_path: &Path,
    clock: FixedClock,
) -> Result<(), Box<dyn std::error::Error>> {
    let database = Database::open(rust_path, clock)?;
    let workspace = WorkspaceId::new("rust-workspace")?;
    let entries = database.load_entries(&workspace, YearMonth::new(2026, 8)?)?;
    assert!(entries.iter().any(|entry| {
        entry.date.to_string() == "2026-08-19"
            && entry.status == WorkEntryStatus::Off
            && entry.notes.as_deref() == Some("electron-round-trip")
    }));
    Ok(())
}

fn entry(date: &str, status: WorkEntryStatus, notes: &str) -> WorkEntry {
    WorkEntry {
        workspace_id: None,
        date: date.parse().expect("fixture date"),
        status,
        start_time: Some("08:00".parse().expect("fixture time")),
        end_time: Some("16:30".parse().expect("fixture time")),
        lunch_minutes: Minutes::new(30),
        project_name: Some("General".to_owned()),
        notes: Some(notes.to_owned()),
        scheduled_minutes_override: None,
        created_at: None,
        updated_at: None,
    }
}
