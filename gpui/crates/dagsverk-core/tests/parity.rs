use std::{fs, path::PathBuf, str::FromStr};

use dagsverk_core::{
    calculations::{
        calculate_daily_pay, calculate_monthly_summary, elapsed_minutes, matches_rate_band,
        matching_weekday_occurrence, normalize_time, split_overtime, threshold_for_entry,
        worked_minutes,
    },
    holidays::SwedishHolidayCalendar,
    models::{
        ExpectedHoursSettings, IsoDate, MonthRecord, OvertimeCompensationSettings,
        OvertimeRateBand, WorkEntry,
    },
};
use rust_decimal::Decimal;
use serde::de::DeserializeOwned;
use serde_json::{Value, json};

fn cases(name: &str) -> Vec<Value> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/parity")
        .join(name);
    serde_json::from_str::<Value>(&fs::read_to_string(path).expect("read parity fixture"))
        .expect("parse parity fixture")["cases"]
        .as_array()
        .expect("fixture cases")
        .clone()
}

fn parse<T: DeserializeOwned>(mut value: Value) -> T {
    stringify_decimal_inputs(&mut value, None);
    serde_json::from_value(value).expect("deserialize fixture input")
}

fn stringify_decimal_inputs(value: &mut Value, key: Option<&str>) {
    const DECIMAL_KEYS: &[&str] = &[
        "hoursPerWorkday",
        "hourlyRate",
        "monthlySalary",
        "employmentPercent",
        "defaultRateValue",
        "dailyThresholdHours",
        "rateValue",
        "manualMonthlyDeduction",
    ];
    match value {
        Value::Number(number) if key.is_some_and(|key| DECIMAL_KEYS.contains(&key)) => {
            *value = Value::String(number.to_string());
        }
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| stringify_decimal_inputs(value, None)),
        Value::Object(values) => values
            .iter_mut()
            .for_each(|(key, value)| stringify_decimal_inputs(value, Some(key))),
        _ => {}
    }
}

fn assert_decimal_json_eq(mut actual: Value, mut expected: Value) {
    normalize_decimal_outputs(&mut actual, None);
    normalize_decimal_outputs(&mut expected, None);
    assert_eq!(actual, expected);
}

fn normalize_decimal_outputs(value: &mut Value, key: Option<&str>) {
    const DECIMAL_KEYS: &[&str] = &[
        "regularPay",
        "overtimePay",
        "obPay",
        "total",
        "grossSalary",
        "baseSalary",
        "overtimeCompensation",
        "obCompensation",
        "workedHours",
        "regularHours",
        "overtimeHours",
        "ordinaryPaidHours",
        "obHours",
        "expectedHours",
        "grossPay",
        "preliminaryTax",
        "estimatedNetPay",
    ];
    match value {
        Value::Number(number) if key.is_some_and(|key| DECIMAL_KEYS.contains(&key)) => {
            *value = Value::String(
                Decimal::from_str(&number.to_string())
                    .expect("fixture decimal")
                    .normalize()
                    .to_string(),
            );
        }
        Value::String(text) if key.is_some_and(|key| DECIMAL_KEYS.contains(&key)) => {
            *text = Decimal::from_str(text)
                .expect("Rust decimal")
                .normalize()
                .to_string();
        }
        Value::Array(values) => values
            .iter_mut()
            .for_each(|value| normalize_decimal_outputs(value, None)),
        Value::Object(values) => values
            .iter_mut()
            .for_each(|(key, value)| normalize_decimal_outputs(value, Some(key))),
        _ => {}
    }
}

#[test]
fn time_and_minute_fixtures_match_typescript() {
    for case in cases("time.json") {
        let input = case["input"].as_str().expect("time input");
        assert_eq!(
            normalize_time(input).map(|time| time.to_string()),
            case["output"].as_str().map(str::to_owned)
        );
    }

    for case in cases("minutes.json") {
        let input = &case["input"];
        let entry = WorkEntry {
            workspace_id: None,
            date: "2026-01-01".parse().expect("fixture date"),
            status: dagsverk_core::models::WorkEntryStatus::Worked,
            start_time: Some(
                input["startTime"]
                    .as_str()
                    .expect("start")
                    .parse()
                    .expect("time"),
            ),
            end_time: Some(
                input["endTime"]
                    .as_str()
                    .expect("end")
                    .parse()
                    .expect("time"),
            ),
            lunch_minutes: dagsverk_core::models::Minutes::new(
                input["lunchMinutes"].as_i64().expect("lunch"),
            ),
            project_name: None,
            notes: None,
            scheduled_minutes_override: None,
            created_at: None,
            updated_at: None,
        };
        assert_eq!(
            elapsed_minutes(
                entry.start_time.expect("start"),
                entry.end_time.expect("end")
            )
            .value(),
            case["output"]["elapsed"].as_i64().expect("elapsed")
        );
        assert_eq!(
            worked_minutes(&entry).value(),
            case["output"]["worked"].as_i64().expect("worked")
        );
    }
}

#[test]
fn overtime_fixtures_match_typescript() {
    let holidays = SwedishHolidayCalendar;
    for case in cases("overtime-threshold.json") {
        let entry: WorkEntry = parse(case["input"]["entry"].clone());
        let expected: ExpectedHoursSettings = parse(case["input"]["expectedHours"].clone());
        let overtime: OvertimeCompensationSettings = parse(case["input"]["overtime"].clone());
        assert_eq!(
            threshold_for_entry(&entry, &expected, &overtime, holidays).value(),
            case["output"]["thresholdMinutes"]
                .as_i64()
                .expect("threshold")
        );
        let (regular, overtime_minutes) = split_overtime(&entry, &expected, &overtime, holidays);
        assert_eq!(
            json!({ "regularMinutes": regular.value(), "overtimeMinutes": overtime_minutes.value() }),
            case["output"]["split"]
        );
    }

    for case in cases("rate-bands.json") {
        let input = &case["input"];
        let band: OvertimeRateBand = parse(input["band"].clone());
        let output = matches_rate_band(
            &band,
            parse(input["compensationType"].clone()),
            input["date"].as_str().expect("date").parse().expect("date"),
            input["time"].as_str().expect("time").parse().expect("time"),
            input["isScheduledWorkday"].as_bool().expect("scheduled"),
            input["isPublicHoliday"].as_bool().expect("holiday"),
            input["isMajorHoliday"].as_bool().expect("major holiday"),
        );
        assert_eq!(output, case["output"].as_bool().expect("match result"));
    }
}

#[test]
fn pay_and_monthly_fixtures_match_typescript() {
    let holidays = SwedishHolidayCalendar;
    let default_expected: ExpectedHoursSettings = parse(json!({
        "hoursPerWorkday": 8,
        "workingWeekdays": [1, 2, 3, 4, 5],
        "excludePublicHolidays": true
    }));
    for case in cases("daily-pay.json") {
        let input = &case["input"];
        let output = calculate_daily_pay(
            &parse(input["entry"].clone()),
            &default_expected,
            &parse(input["salary"].clone()),
            &parse(input["overtime"].clone()),
            holidays,
        );
        assert_decimal_json_eq(
            serde_json::to_value(output).expect("serialize pay"),
            case["output"].clone(),
        );
    }

    for case in cases("monthly-summary.json") {
        let input = &case["input"];
        let output = calculate_monthly_summary(
            &parse::<MonthRecord>(input["record"].clone()),
            &parse::<Vec<WorkEntry>>(input["entries"].clone()),
            &parse(input["expectedHours"].clone()),
            &parse(input["salary"].clone()),
            &parse(input["overtime"].clone()),
            holidays,
            input["today"]
                .as_str()
                .expect("today")
                .parse()
                .expect("date"),
        );
        assert_decimal_json_eq(
            serde_json::to_value(output).expect("serialize summary"),
            case["output"].clone(),
        );
    }
}

#[test]
fn holiday_fixtures_match_typescript() {
    let calendar = SwedishHolidayCalendar;
    for case in cases("holidays.json") {
        let year = case["year"].as_i64().expect("year") as i32;
        let named: Vec<Value> = calendar
            .holidays(year)
            .into_iter()
            .map(|holiday| json!({ "date": holiday.date.to_string(), "name": holiday.name }))
            .collect();
        assert_eq!(Value::Array(named), case["named"]);
        for sunday in case["sundays"].as_array().expect("sundays") {
            let date: IsoDate = sunday["date"]
                .as_str()
                .expect("date")
                .parse()
                .expect("date");
            assert_eq!(calendar.is_public_holiday(date), sunday["isPublicHoliday"]);
            assert_eq!(calendar.holiday_name(date), sunday["name"].as_str());
        }
        for boundary in case["majorBoundaries"].as_array().expect("boundaries") {
            let result = calendar.is_major_holiday_period(
                boundary["date"]
                    .as_str()
                    .expect("date")
                    .parse()
                    .expect("date"),
                boundary["time"]
                    .as_str()
                    .expect("time")
                    .parse()
                    .expect("time"),
            );
            assert_eq!(result, boundary["output"].as_bool().expect("result"));
        }
    }
}

#[test]
fn tax_fixtures_match_typescript_and_canonical_source() {
    use dagsverk_core::{models::TaxSettings, tax::TaxEngine};
    use sha2::{Digest, Sha256};

    let tax_path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../public/tax-data/tax-2026.json");
    let tax_json = fs::read_to_string(tax_path).expect("read canonical tax data");
    assert_eq!(
        format!("{:x}", Sha256::digest(tax_json.as_bytes())),
        "f660a261b4f4abb44b3595f69d1e93bd2895faad19847ff45b50865919ebc0b6"
    );

    let mut engine = TaxEngine::default();
    engine.register_json(&tax_json).expect("parse tax data");
    for case in cases("tax.json") {
        let gross = Decimal::from_str(
            &case["input"]["grossPay"]
                .as_number()
                .expect("gross pay")
                .to_string(),
        )
        .expect("gross pay decimal");
        let settings: TaxSettings = parse(case["input"]["settings"].clone());
        assert_decimal_json_eq(
            serde_json::to_value(
                engine.calculate(dagsverk_core::models::Money::new(gross), &settings),
            )
            .expect("serialize tax estimate"),
            case["output"].clone(),
        );
    }
}

#[test]
fn copy_month_date_mapping_matches_typescript() {
    for case in cases("copy-paste-month.json") {
        let input = &case["input"];
        let output = matching_weekday_occurrence(
            input["sourceDate"]
                .as_str()
                .expect("source date")
                .parse()
                .expect("date"),
            dagsverk_core::models::YearMonth::new(
                input["targetYear"].as_i64().expect("year") as i32,
                input["targetMonth"].as_u64().expect("month") as u32,
            )
            .expect("year month"),
        )
        .map(|date| date.to_string());
        assert_eq!(output.as_deref(), case["output"].as_str());
    }
}
