use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate, Timelike};
use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, RoundingStrategy};

use crate::{
    holidays::SwedishHolidayCalendar,
    models::{
        BalanceHistoryMonth, ClockTime, CompensationRateType, CompensationRuleType,
        DailyPayBreakdown, ExpectedHoursSettings, HourlyPayBasis, IsoDate, Minutes, Money,
        MonthRecord, MonthlySummary, ObOvertimeCombinationMode, OvertimeCompensationMode,
        OvertimeCompensationSettings, OvertimeDayCategory, OvertimeRateBand, OvertimeThresholdMode,
        SalarySettings, SalaryType, WorkEntry, WorkEntryStatus, WorkspaceId, YearMonth,
    },
};

pub fn normalize_time(input: &str) -> Option<ClockTime> {
    let mut candidate = input.trim().replace('.', ":");
    if candidate.is_empty() {
        return None;
    }
    if candidate.bytes().all(|byte| byte.is_ascii_digit()) {
        candidate = match candidate.len() {
            1 | 2 => format!("{candidate}:00"),
            3 => format!("{}:{}", &candidate[..1], &candidate[1..]),
            4 => format!("{}:{}", &candidate[..2], &candidate[2..]),
            _ => candidate,
        };
    }
    let (hours, minutes) = candidate.split_once(':')?;
    if hours.is_empty()
        || minutes.is_empty()
        || hours.len() > 2
        || minutes.len() > 2
        || !hours.bytes().all(|byte| byte.is_ascii_digit())
        || !minutes.bytes().all(|byte| byte.is_ascii_digit())
    {
        return None;
    }
    format!(
        "{:02}:{:02}",
        hours.parse::<u32>().ok()?,
        minutes.parse::<u32>().ok()?
    )
    .parse()
    .ok()
}

pub fn time_to_minutes(time: ClockTime) -> i64 {
    i64::from(time.as_naive_time().hour() * 60 + time.as_naive_time().minute())
}

pub fn time_from_minutes(total_minutes: i64) -> ClockTime {
    let normalized = total_minutes.rem_euclid(24 * 60);
    let value = format!("{:02}:{:02}", normalized / 60, normalized % 60);
    value.parse().unwrap_or_else(|_| unreachable!())
}

pub fn elapsed_minutes(start: ClockTime, end: ClockTime) -> Minutes {
    let elapsed = time_to_minutes(end) - time_to_minutes(start);
    Minutes::new(if elapsed > 0 {
        elapsed
    } else {
        elapsed + 24 * 60
    })
}

pub fn worked_minutes(entry: &WorkEntry) -> Minutes {
    match (entry.start_time, entry.end_time) {
        (Some(start), Some(end)) if start != end => {
            Minutes::new((elapsed_minutes(start, end).value() - entry.lunch_minutes.value()).max(0))
        }
        _ => Minutes::ZERO,
    }
}

pub fn matches_rate_band(
    band: &OvertimeRateBand,
    compensation_type: CompensationRuleType,
    date: IsoDate,
    time: ClockTime,
    is_scheduled_workday: bool,
    is_public_holiday: bool,
    is_major_holiday: bool,
) -> bool {
    band.compensation_type == compensation_type
        && matches_day_category(
            band.day_category,
            date,
            is_scheduled_workday,
            is_public_holiday,
            is_major_holiday,
        )
        && matches_time(band.start_time, band.end_time, time)
}

pub fn matches_day_category(
    category: OvertimeDayCategory,
    date: IsoDate,
    is_scheduled_workday: bool,
    is_public_holiday: bool,
    is_major_holiday: bool,
) -> bool {
    use OvertimeDayCategory::*;
    let weekday = date.as_naive_date().weekday().num_days_from_sunday();
    match category {
        AllDays => true,
        ScheduledWorkdays => is_scheduled_workday,
        NonWorkdays => !is_scheduled_workday,
        Monday => weekday == 1,
        Tuesday => weekday == 2,
        Wednesday => weekday == 3,
        Thursday => weekday == 4,
        Friday => weekday == 5,
        Saturday => weekday == 6,
        Sunday => weekday == 0,
        PublicHolidays => is_public_holiday,
        ScheduledWeekdays => is_scheduled_workday && (1..=5).contains(&weekday),
        Weekends => weekday == 0 || weekday == 6,
        MajorHolidays => is_major_holiday,
    }
}

pub fn matches_time(start: ClockTime, end: ClockTime, target: ClockTime) -> bool {
    let start = time_to_minutes(start);
    let end = time_to_minutes(end);
    let target = time_to_minutes(target);
    start == end
        || if start < end {
            target >= start && target < end
        } else {
            target >= start || target < end
        }
}

fn hourly_amount(
    rate_type: CompensationRateType,
    rate_value: Decimal,
    salary: &SalarySettings,
    include_hourly_base: bool,
) -> Decimal {
    match rate_type {
        CompensationRateType::HourlyPremiumPercent => {
            salary.hourly_rate.decimal()
                * (rate_value / Decimal::ONE_HUNDRED
                    + if include_hourly_base {
                        Decimal::ONE
                    } else {
                        Decimal::ZERO
                    })
        }
        CompensationRateType::FixedHourlyAmount => rate_value,
        CompensationRateType::FullTimeMonthlySalaryDivisor
            if salary.salary_type == SalaryType::Monthly && rate_value > Decimal::ZERO =>
        {
            salary.monthly_salary.decimal() * Decimal::ONE_HUNDRED
                / salary.employment_percent
                / rate_value
        }
        CompensationRateType::FullTimeMonthlySalaryDivisor => Decimal::ZERO,
    }
}

fn hourly_amount_at(
    compensation_type: CompensationRuleType,
    salary: &SalarySettings,
    overtime: &OvertimeCompensationSettings,
    clock: CompensationClock,
) -> Decimal {
    let highest = overtime
        .rate_bands
        .iter()
        .filter(|band| {
            matches_rate_band(
                band,
                compensation_type,
                clock.date,
                clock.time,
                clock.scheduled,
                clock.public_holiday,
                clock.major_holiday,
            )
        })
        .map(|band| {
            hourly_amount(
                band.rate_type,
                band.rate_value,
                salary,
                compensation_type == CompensationRuleType::Overtime,
            )
        })
        .max();
    highest.unwrap_or_else(|| {
        if compensation_type == CompensationRuleType::Ob {
            Decimal::ZERO
        } else {
            hourly_amount(
                overtime.default_rate_type,
                overtime.default_rate_value,
                salary,
                true,
            )
        }
    })
}

#[derive(Clone, Copy)]
struct CompensationClock {
    date: IsoDate,
    time: ClockTime,
    scheduled: bool,
    public_holiday: bool,
    major_holiday: bool,
}

pub fn dates_in_month(year: i32, month: u32) -> Vec<IsoDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(|| unreachable!());
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1).unwrap_or_else(|| unreachable!())
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1).unwrap_or_else(|| unreachable!())
    };
    (0..(next - first).num_days())
        .map(|offset| IsoDate::new(first + Duration::days(offset)))
        .collect()
}

pub fn is_scheduled_workday(
    date: IsoDate,
    expected: &ExpectedHoursSettings,
    holidays: SwedishHolidayCalendar,
) -> bool {
    expected
        .working_weekdays
        .contains(&date.as_naive_date().weekday().num_days_from_sunday())
        && (!expected.exclude_public_holidays || !holidays.is_public_holiday(date))
}

pub fn expected_workdays(
    year: i32,
    month: u32,
    expected: &ExpectedHoursSettings,
    holidays: SwedishHolidayCalendar,
) -> Vec<IsoDate> {
    dates_in_month(year, month)
        .into_iter()
        .filter(|date| is_scheduled_workday(*date, expected, holidays))
        .collect()
}

pub fn matching_weekday_occurrence(source: IsoDate, target: YearMonth) -> Option<IsoDate> {
    let source_date = source.as_naive_date();
    let occurrence = (source_date.day() - 1) / 7;
    let first = NaiveDate::from_ymd_opt(target.year, target.month, 1)?;
    let offset = (source_date.weekday().num_days_from_monday() + 7
        - first.weekday().num_days_from_monday())
        % 7;
    let result = first + Duration::days(i64::from(offset + occurrence * 7));
    (result.month() == target.month).then(|| IsoDate::new(result))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasteMonthError {
    SameMonth,
    DifferentWorkspace,
}

pub fn paste_month_entries(
    source_workspace: &WorkspaceId,
    source_month: YearMonth,
    source_entries: &[WorkEntry],
    target_workspace: &WorkspaceId,
    target_month: YearMonth,
    target_entries: &[WorkEntry],
) -> Result<Vec<WorkEntry>, PasteMonthError> {
    if source_workspace != target_workspace {
        return Err(PasteMonthError::DifferentWorkspace);
    }
    if source_month == target_month {
        return Err(PasteMonthError::SameMonth);
    }
    let occupied: std::collections::BTreeSet<_> = target_entries
        .iter()
        .filter(|entry| entry.status != WorkEntryStatus::Incomplete)
        .map(|entry| entry.date)
        .collect();
    Ok(source_entries
        .iter()
        .filter(|entry| entry.status != WorkEntryStatus::Incomplete)
        .filter_map(|entry| {
            let date = matching_weekday_occurrence(entry.date, target_month)?;
            (!occupied.contains(&date)).then(|| {
                let mut pasted = entry.clone();
                pasted.workspace_id = Some(target_workspace.clone());
                pasted.date = date;
                pasted
            })
        })
        .collect())
}

pub fn estimate_opening_balance(
    history: &[BalanceHistoryMonth],
    initial: Minutes,
    expected: &ExpectedHoursSettings,
    salary: &SalarySettings,
    overtime: &OvertimeCompensationSettings,
    holidays: SwedishHolidayCalendar,
    today: IsoDate,
) -> Minutes {
    let first_relevant = history
        .iter()
        .rposition(|item| {
            item.record
                .as_ref()
                .is_some_and(|record| record.opening_balance_was_edited)
        })
        .unwrap_or(0);
    let mut opening = initial;
    for item in &history[first_relevant..] {
        if let Some(record) = &item.record
            && record.opening_balance_was_edited
        {
            opening = record.opening_balance_minutes;
        }
        if !item
            .entries
            .iter()
            .any(|entry| entry.status != WorkEntryStatus::Incomplete)
        {
            continue;
        }
        let mut record = item.record.clone().unwrap_or(MonthRecord {
            workspace_id: None,
            year: item.year,
            month: item.month,
            opening_balance_minutes: opening,
            expected_minutes_override: None,
            opening_balance_was_edited: false,
        });
        if !record.opening_balance_was_edited {
            record.opening_balance_minutes = opening;
        }
        opening = calculate_monthly_summary(
            &record,
            &item.entries,
            expected,
            salary,
            overtime,
            holidays,
            today,
        )
        .closing_balance_minutes;
    }
    opening
}

pub fn threshold_for_entry(
    entry: &WorkEntry,
    expected: &ExpectedHoursSettings,
    overtime: &OvertimeCompensationSettings,
    holidays: SwedishHolidayCalendar,
) -> Minutes {
    if let Some(value) = entry.scheduled_minutes_override {
        return value;
    }
    let hours = if overtime.threshold_mode == OvertimeThresholdMode::ScheduledHours {
        if is_scheduled_workday(entry.date, expected, holidays) {
            expected.hours_per_workday
        } else {
            Decimal::ZERO
        }
    } else {
        overtime.daily_threshold_hours
    };
    Minutes::new(round_integer(hours * Decimal::from(60)))
}

pub fn split_overtime(
    entry: &WorkEntry,
    expected: &ExpectedHoursSettings,
    overtime: &OvertimeCompensationSettings,
    holidays: SwedishHolidayCalendar,
) -> (Minutes, Minutes) {
    let worked = worked_minutes(entry).value();
    let threshold = threshold_for_entry(entry, expected, overtime, holidays).value();
    let regular = worked.min(threshold);
    (
        Minutes::new(regular),
        Minutes::new((worked - regular).max(0)),
    )
}

pub fn calculate_daily_pay(
    entry: &WorkEntry,
    expected: &ExpectedHoursSettings,
    salary: &SalarySettings,
    overtime: &OvertimeCompensationSettings,
    holidays: SwedishHolidayCalendar,
) -> DailyPayBreakdown {
    if entry.status != WorkEntryStatus::Worked
        || entry.start_time.is_none()
        || entry.end_time.is_none()
    {
        return empty_pay();
    }
    let (regular, overtime_minutes) = split_overtime(entry, expected, overtime, holidays);
    let regular_pay = if salary.salary_type == SalaryType::Hourly {
        Decimal::from(regular.value()) * salary.hourly_rate.decimal() / Decimal::from(60)
    } else {
        Decimal::ZERO
    };
    let pays_overtime =
        overtime.mode == OvertimeCompensationMode::Paid && overtime_minutes.value() > 0;
    let pays_ob = overtime
        .rate_bands
        .iter()
        .any(|band| band.compensation_type == CompensationRuleType::Ob);
    if !pays_overtime && !pays_ob {
        let regular_pay = round_money(regular_pay);
        return DailyPayBreakdown {
            regular_pay,
            overtime_pay: Money::ZERO,
            ob_pay: Money::ZERO,
            ob_minutes: Minutes::ZERO,
            total: regular_pay,
        };
    }

    let mut overtime_pay = Decimal::ZERO;
    let mut ob_pay = Decimal::ZERO;
    let mut ob_minutes = 0;
    for minute in 0..regular.value() + overtime_minutes.value() {
        let is_overtime = minute >= regular.value();
        let offset = if is_overtime {
            minute + entry.lunch_minutes.value()
        } else {
            minute
        };
        let (date, time) = clock_at(entry, offset);
        let scheduled = is_scheduled_workday(date, expected, holidays);
        let public_holiday = holidays.is_public_holiday(date);
        let major_holiday = holidays.is_major_holiday_period(date, time);
        let clock = CompensationClock {
            date,
            time,
            scheduled,
            public_holiday,
            major_holiday,
        };
        if pays_ob
            && (!is_overtime
                || overtime.ob_overtime_combination == ObOvertimeCombinationMode::IncludeOb)
        {
            let amount = hourly_amount_at(CompensationRuleType::Ob, salary, overtime, clock);
            if amount > Decimal::ZERO {
                ob_pay += amount / Decimal::from(60);
                ob_minutes += 1;
            }
        }
        if is_overtime && pays_overtime {
            overtime_pay +=
                hourly_amount_at(CompensationRuleType::Overtime, salary, overtime, clock)
                    / Decimal::from(60);
        }
    }
    let regular_pay = round_money(regular_pay);
    let overtime_pay = round_money(overtime_pay);
    let ob_pay = round_money(ob_pay);
    DailyPayBreakdown {
        regular_pay,
        overtime_pay,
        ob_pay,
        ob_minutes: Minutes::new(ob_minutes),
        total: Money::new(regular_pay.decimal() + overtime_pay.decimal() + ob_pay.decimal()),
    }
}

pub fn calculate_monthly_summary(
    record: &MonthRecord,
    entries: &[WorkEntry],
    expected: &ExpectedHoursSettings,
    salary: &SalarySettings,
    overtime: &OvertimeCompensationSettings,
    holidays: SwedishHolidayCalendar,
    today: IsoDate,
) -> MonthlySummary {
    let entries_by_date: BTreeMap<_, _> = entries
        .iter()
        .filter(|entry| {
            entry.date.as_naive_date().year() == record.year
                && entry.date.as_naive_date().month() == record.month
        })
        .map(|entry| (entry.date, entry))
        .collect();
    let month_entries: Vec<_> = entries_by_date.values().copied().collect();
    let expected_minutes =
        calculate_expected_minutes(record, &month_entries, expected, holidays, today);
    let mut worked = 0;
    let mut regular = 0;
    let mut overtime_total = 0;
    let mut overtime_pay = Decimal::ZERO;
    let mut ob_pay = Decimal::ZERO;
    let mut ob_minutes = 0;
    let mut gross = if salary.salary_type == SalaryType::Monthly {
        salary.monthly_salary.decimal()
    } else {
        Decimal::ZERO
    };
    let mut completed = 0;
    for entry in entries_by_date.values() {
        if entry.status == WorkEntryStatus::Worked
            && entry.start_time.is_some()
            && entry.end_time.is_some()
        {
            completed += 1;
            let day_worked = worked_minutes(entry).value();
            let (day_regular, day_overtime) = split_overtime(entry, expected, overtime, holidays);
            let pay = calculate_daily_pay(entry, expected, salary, overtime, holidays);
            worked += day_worked;
            regular += day_regular.value();
            overtime_total += day_overtime.value();
            gross += pay.total.decimal();
            overtime_pay += pay.overtime_pay.decimal();
            ob_pay += pay.ob_pay.decimal();
            ob_minutes += pay.ob_minutes.value();
        } else if entry.status == WorkEntryStatus::Off {
            completed += 1;
        }
    }
    let balance_eligible = if overtime.mode == OvertimeCompensationMode::CompTime {
        worked
    } else {
        regular
    };
    let mut ordinary_paid = if salary.salary_type == SalaryType::Hourly {
        regular
    } else {
        0
    };
    if salary.salary_type == SalaryType::Hourly
        && overtime.mode == OvertimeCompensationMode::CompTime
        && salary.hourly_pay_basis == HourlyPayBasis::MonthlyExpectedHours
    {
        ordinary_paid = worked.min(expected_minutes);
        gross = Decimal::from(ordinary_paid) * salary.hourly_rate.decimal() / Decimal::from(60)
            + ob_pay;
    }
    let difference = balance_eligible - expected_minutes;
    let missing_past_days = expected_workdays(record.year, record.month, expected, holidays)
        .into_iter()
        .filter(|date| {
            *date < today
                && entries_by_date
                    .get(date)
                    .is_none_or(|entry| entry.status == WorkEntryStatus::Incomplete)
        })
        .collect();
    MonthlySummary {
        year: record.year,
        month: record.month,
        worked_minutes: Minutes::new(worked),
        regular_minutes: Minutes::new(regular),
        overtime_minutes: Minutes::new(overtime_total),
        ordinary_paid_minutes: Minutes::new(ordinary_paid),
        balance_eligible_minutes: Minutes::new(balance_eligible),
        expected_minutes: Minutes::new(expected_minutes),
        monthly_difference_minutes: Minutes::new(difference),
        opening_balance_minutes: record.opening_balance_minutes,
        closing_balance_minutes: Minutes::new(record.opening_balance_minutes.value() + difference),
        gross_salary: round_money(gross),
        base_salary: if salary.salary_type == SalaryType::Monthly {
            salary.monthly_salary
        } else {
            Money::ZERO
        },
        overtime_compensation: round_money(overtime_pay),
        ob_compensation: round_money(ob_pay),
        ob_minutes: Minutes::new(ob_minutes),
        completed_day_count: completed,
        missing_past_days,
        worked_hours: round_hours(worked),
        regular_hours: round_hours(regular),
        overtime_hours: round_hours(overtime_total),
        ordinary_paid_hours: Decimal::from(ordinary_paid) / Decimal::from(60),
        ob_hours: round_hours(ob_minutes),
        expected_hours: round_hours(expected_minutes),
    }
}

fn calculate_expected_minutes(
    record: &MonthRecord,
    entries: &[&WorkEntry],
    expected: &ExpectedHoursSettings,
    holidays: SwedishHolidayCalendar,
    through: IsoDate,
) -> i64 {
    let workdays = expected_workdays(record.year, record.month, expected, holidays);
    let daily = round_integer(expected.hours_per_workday * Decimal::from(60));
    let full_expected = record
        .expected_minutes_override
        .map_or(workdays.len() as i64 * daily, Minutes::value);
    let month_dates = dates_in_month(record.year, record.month);
    let month_start = month_dates[0];
    let month_end = *month_dates.last().unwrap_or_else(|| unreachable!());
    let full_month = through >= month_end;
    if !full_month && through < month_start {
        return 0;
    }
    let elapsed = if full_month {
        workdays.len()
    } else {
        workdays.iter().filter(|date| **date <= through).count()
    } as i64;
    if record.expected_minutes_override.is_some() {
        return if full_month {
            full_expected
        } else if workdays.is_empty() {
            0
        } else {
            round_integer(
                Decimal::from(full_expected) * Decimal::from(elapsed)
                    / Decimal::from(workdays.len()),
            )
        };
    }
    let last_included = if full_month { month_end } else { through };
    let mut adjusted = if full_month {
        full_expected
    } else {
        elapsed * daily
    };
    for entry in entries {
        if entry.date > last_included || entry.scheduled_minutes_override.is_none() {
            continue;
        }
        if is_scheduled_workday(entry.date, expected, holidays) {
            adjusted -= daily;
        }
        adjusted += entry
            .scheduled_minutes_override
            .unwrap_or(Minutes::ZERO)
            .value();
    }
    adjusted.max(0)
}

fn clock_at(entry: &WorkEntry, offset: i64) -> (IsoDate, ClockTime) {
    let absolute = time_to_minutes(entry.start_time.unwrap_or_else(|| unreachable!())) + offset;
    (
        IsoDate::new(entry.date.as_naive_date() + Duration::days(absolute.div_euclid(24 * 60))),
        time_from_minutes(absolute),
    )
}

fn round_integer(value: Decimal) -> i64 {
    value
        .round_dp_with_strategy(0, RoundingStrategy::MidpointAwayFromZero)
        .to_i64()
        .unwrap_or_else(|| unreachable!())
}

fn round_money(value: Decimal) -> Money {
    Money::new(value.round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero))
}

fn round_hours(minutes: i64) -> Decimal {
    (Decimal::from(minutes) / Decimal::from(60))
        .round_dp_with_strategy(2, RoundingStrategy::MidpointAwayFromZero)
}

fn empty_pay() -> DailyPayBreakdown {
    DailyPayBreakdown {
        regular_pay: Money::ZERO,
        overtime_pay: Money::ZERO,
        ob_pay: Money::ZERO,
        ob_minutes: Minutes::ZERO,
        total: Money::ZERO,
    }
}
