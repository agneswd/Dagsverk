use std::str::FromStr;

use chrono::{DateTime, Utc};
use dagsverk_core::{
    clock::{Clock, FixedClock},
    models::{
        ClockTime, CompensationRateType, CompensationRuleType, CurrencyPreference,
        ExportLanguagePreference, HourlyPayBasis, IsoDate, LanguagePreference, Minutes,
        MonthViewPreference, ObOvertimeCombinationMode, OvertimeCompensationMode,
        OvertimeDayCategory, OvertimeThresholdMode, SalaryType, TaxMode, ThemePreference,
        WorkEntry, WorkEntryStatus, WorkspaceType, default_preferences, default_workspace,
    },
};
use serde::{Serialize, de::DeserializeOwned};

fn assert_enum<T>(values: &[(i64, T)])
where
    T: Copy + std::fmt::Debug + PartialEq + TryFrom<i64> + Serialize + DeserializeOwned,
    <T as TryFrom<i64>>::Error: std::fmt::Debug,
{
    for (persisted, expected) in values {
        let Ok(actual) = T::try_from(*persisted) else {
            panic!("valid persisted enum value was rejected: {persisted}");
        };
        assert_eq!(actual, *expected);

        let Ok(json) = serde_json::to_string(expected) else {
            panic!("enum serialization failed: {expected:?}");
        };
        assert_eq!(json, persisted.to_string());

        let Ok(round_trip) = serde_json::from_str::<T>(&json) else {
            panic!("enum deserialization failed: {expected:?}");
        };
        assert_eq!(round_trip, *expected);
    }
    assert!(T::try_from(i64::MAX).is_err());
}

#[test]
fn every_persisted_enum_round_trips() {
    assert_enum(&[
        (0, WorkEntryStatus::Incomplete),
        (1, WorkEntryStatus::Worked),
        (2, WorkEntryStatus::Off),
    ]);
    assert_enum(&[
        (0, ThemePreference::System),
        (1, ThemePreference::Light),
        (2, ThemePreference::Dark),
    ]);
    assert_enum(&[
        (0, MonthViewPreference::Ledger),
        (1, MonthViewPreference::Calendar),
    ]);
    assert_enum(&[
        (0, LanguagePreference::System),
        (1, LanguagePreference::English),
        (2, LanguagePreference::Swedish),
    ]);
    assert_enum(&[
        (0, WorkspaceType::Employment),
        (1, WorkspaceType::Contract),
        (2, WorkspaceType::Personal),
    ]);
    assert_enum(&[
        (0, ExportLanguagePreference::Swedish),
        (1, ExportLanguagePreference::English),
        (2, ExportLanguagePreference::System),
    ]);
    assert_enum(&[
        (0, OvertimeCompensationMode::CompTime),
        (1, OvertimeCompensationMode::Paid),
    ]);
    assert_enum(&[
        (0, OvertimeThresholdMode::FixedDailyHours),
        (1, OvertimeThresholdMode::ScheduledHours),
    ]);
    assert_enum(&[
        (0, CompensationRuleType::Overtime),
        (1, CompensationRuleType::Ob),
    ]);
    assert_enum(&[
        (0, CompensationRateType::HourlyPremiumPercent),
        (1, CompensationRateType::FixedHourlyAmount),
        (2, CompensationRateType::FullTimeMonthlySalaryDivisor),
    ]);
    assert_enum(&[
        (0, OvertimeDayCategory::AllDays),
        (1, OvertimeDayCategory::ScheduledWorkdays),
        (2, OvertimeDayCategory::NonWorkdays),
        (3, OvertimeDayCategory::Monday),
        (4, OvertimeDayCategory::Tuesday),
        (5, OvertimeDayCategory::Wednesday),
        (6, OvertimeDayCategory::Thursday),
        (7, OvertimeDayCategory::Friday),
        (8, OvertimeDayCategory::Saturday),
        (9, OvertimeDayCategory::Sunday),
        (10, OvertimeDayCategory::PublicHolidays),
        (11, OvertimeDayCategory::ScheduledWeekdays),
        (12, OvertimeDayCategory::Weekends),
        (13, OvertimeDayCategory::MajorHolidays),
    ]);
    assert_enum(&[(0, SalaryType::Hourly), (1, SalaryType::Monthly)]);
    assert_enum(&[
        (0, HourlyPayBasis::DailyRegularHours),
        (1, HourlyPayBasis::MonthlyExpectedHours),
    ]);
    assert_enum(&[
        (0, ObOvertimeCombinationMode::ExcludeOb),
        (1, ObOvertimeCombinationMode::IncludeOb),
    ]);
    assert_enum(&[
        (0, TaxMode::Disabled),
        (1, TaxMode::PrimaryIncomeTaxTable),
        (2, TaxMode::SecondaryIncomeThirtyPercent),
        (3, TaxMode::ManualMonthlyDeduction),
    ]);
}

#[test]
fn currency_preserves_json_and_sqlite_representations() {
    let values = [
        (0, CurrencyPreference::Sek, "\"SEK\""),
        (1, CurrencyPreference::Eur, "\"EUR\""),
        (2, CurrencyPreference::Usd, "\"USD\""),
        (3, CurrencyPreference::Gbp, "\"GBP\""),
        (4, CurrencyPreference::Nok, "\"NOK\""),
        (5, CurrencyPreference::Dkk, "\"DKK\""),
    ];
    for (persisted, expected, json) in values {
        assert_eq!(CurrencyPreference::try_from(persisted), Ok(expected));
        assert_eq!(i32::from(expected), persisted as i32);
        let Ok(serialized) = serde_json::to_string(&expected) else {
            panic!("currency serialization failed: {expected:?}");
        };
        assert_eq!(serialized, json);
    }
}

#[test]
fn validated_values_keep_database_formats() {
    let Ok(date) = IsoDate::from_str("2026-08-18") else {
        panic!("valid date rejected");
    };
    let Ok(time) = ClockTime::from_str("08:30") else {
        panic!("valid time rejected");
    };
    assert_eq!(date.to_string(), "2026-08-18");
    assert_eq!(time.to_string(), "08:30");
    assert!(IsoDate::from_str("2026-02-30").is_err());
    assert!(ClockTime::from_str("24:00").is_err());
}

#[test]
fn defaults_use_the_injected_clock() {
    let Ok(parsed) = DateTime::parse_from_rfc3339("2026-08-18T10:00:00Z") else {
        panic!("fixed timestamp rejected");
    };
    let clock = FixedClock::new(parsed.with_timezone(&Utc));
    let workspace = default_workspace(&clock);
    let preferences = default_preferences();
    assert_eq!(workspace.id.as_str(), "ws-default");
    assert_eq!(workspace.created_at, clock.now_utc());
    assert_eq!(preferences.active_workspace_id.as_str(), "ws-default");
}

#[test]
fn work_entry_serializes_enum_and_validated_values_for_fixtures() {
    let Ok(date) = "2026-08-18".parse() else {
        panic!("valid date rejected");
    };
    let Ok(start_time) = "08:00".parse() else {
        panic!("valid start time rejected");
    };
    let Ok(end_time) = "16:30".parse() else {
        panic!("valid end time rejected");
    };
    let entry = WorkEntry {
        workspace_id: None,
        date,
        status: WorkEntryStatus::Worked,
        start_time: Some(start_time),
        end_time: Some(end_time),
        lunch_minutes: Minutes::new(30),
        project_name: Some("General".to_owned()),
        notes: None,
        scheduled_minutes_override: None,
        created_at: None,
        updated_at: None,
    };
    let Ok(value) = serde_json::to_value(entry) else {
        panic!("work entry serialization failed");
    };
    assert_eq!(value["date"], "2026-08-18");
    assert_eq!(value["status"], 1);
    assert_eq!(value["startTime"], "08:00");
    assert_eq!(value["lunchMinutes"], 30);
}
