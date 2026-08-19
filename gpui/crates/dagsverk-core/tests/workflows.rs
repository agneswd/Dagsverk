use dagsverk_core::{
    calculations::{PasteMonthError, estimate_opening_balance, paste_month_entries},
    holidays::SwedishHolidayCalendar,
    models::{
        BalanceHistoryMonth, IsoDate, Minutes, MonthRecord, WorkEntry, WorkEntryStatus,
        WorkspaceId, YearMonth, default_settings,
    },
};

fn entry(date: &str, status: WorkEntryStatus, start: &str, end: &str) -> WorkEntry {
    WorkEntry {
        workspace_id: None,
        date: date.parse().expect("date"),
        status,
        start_time: Some(start.parse().expect("start")),
        end_time: Some(end.parse().expect("end")),
        lunch_minutes: Minutes::ZERO,
        project_name: Some("General".to_owned()),
        notes: None,
        scheduled_minutes_override: None,
        created_at: None,
        updated_at: None,
    }
}

#[test]
fn paste_month_enforces_scope_and_skips_occupied_or_missing_occurrences() {
    let workspace = WorkspaceId::new("one").expect("workspace");
    let other = WorkspaceId::new("two").expect("workspace");
    let june = YearMonth::new(2026, 6).expect("month");
    let july = YearMonth::new(2026, 7).expect("month");
    let sources = [
        entry("2026-06-01", WorkEntryStatus::Worked, "08:00", "16:00"),
        entry("2026-06-08", WorkEntryStatus::Worked, "22:00", "06:00"),
        entry("2026-06-29", WorkEntryStatus::Worked, "22:00", "06:00"),
        entry("2026-06-02", WorkEntryStatus::Off, "08:00", "16:00"),
        entry("2026-06-03", WorkEntryStatus::Incomplete, "08:00", "16:00"),
    ];
    let occupied = [entry(
        "2026-07-06",
        WorkEntryStatus::Worked,
        "09:00",
        "17:00",
    )];

    assert_eq!(
        paste_month_entries(&workspace, june, &sources, &workspace, june, &[]),
        Err(PasteMonthError::SameMonth)
    );
    assert_eq!(
        paste_month_entries(&workspace, june, &sources, &other, july, &[]),
        Err(PasteMonthError::DifferentWorkspace)
    );
    let pasted = paste_month_entries(&workspace, june, &sources, &workspace, july, &occupied)
        .expect("paste");
    assert_eq!(pasted.len(), 2);
    assert_eq!(
        pasted[0].date,
        "2026-07-13".parse::<IsoDate>().expect("date")
    );
    assert_eq!(pasted[0].start_time.expect("start").to_string(), "22:00");
    assert_eq!(
        pasted[1].date,
        "2026-07-07".parse::<IsoDate>().expect("date")
    );
    assert_eq!(pasted[1].status, WorkEntryStatus::Off);
}

#[test]
fn opening_balance_starts_at_the_latest_explicit_edit_and_ignores_incomplete_months() {
    let settings = default_settings();
    let record = |month, opening, edited| MonthRecord {
        workspace_id: None,
        year: 2026,
        month,
        opening_balance_minutes: Minutes::new(opening),
        expected_minutes_override: Some(Minutes::new(480)),
        opening_balance_was_edited: edited,
    };
    let history = vec![
        BalanceHistoryMonth {
            year: 2026,
            month: 1,
            record: Some(record(1, 120, true)),
            entries: vec![entry(
                "2026-01-05",
                WorkEntryStatus::Worked,
                "08:00",
                "16:00",
            )],
        },
        BalanceHistoryMonth {
            year: 2026,
            month: 2,
            record: None,
            entries: vec![entry(
                "2026-02-02",
                WorkEntryStatus::Incomplete,
                "08:00",
                "16:00",
            )],
        },
        BalanceHistoryMonth {
            year: 2026,
            month: 3,
            record: Some(record(3, 300, true)),
            entries: vec![entry(
                "2026-03-02",
                WorkEntryStatus::Worked,
                "08:00",
                "17:00",
            )],
        },
    ];

    let opening = estimate_opening_balance(
        &history,
        Minutes::new(999),
        &settings.expected_hours,
        &settings.salary,
        &settings.overtime_compensation,
        SwedishHolidayCalendar,
        "2026-04-01".parse().expect("today"),
    );
    assert_eq!(opening, Minutes::new(360));
    assert_eq!(
        estimate_opening_balance(
            &[],
            Minutes::new(75),
            &settings.expected_hours,
            &settings.salary,
            &settings.overtime_compensation,
            SwedishHolidayCalendar,
            "2026-04-01".parse().expect("today"),
        ),
        Minutes::new(75)
    );
}
