use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::clock::Clock;

use super::{
    ClockTime, CompensationRateType, CompensationRuleType, CurrencyPreference,
    ExportLanguagePreference, HourlyPayBasis, IsoDate, LanguagePreference, Minutes, Money,
    MonthViewPreference, ObOvertimeCombinationMode, OvertimeCompensationMode, OvertimeDayCategory,
    OvertimeThresholdMode, ProjectId, SalaryType, TaxMode, TaxUnavailableReason, ThemePreference,
    UpdateStatus, WorkEntryStatus, WorkspaceId, WorkspaceType, YearMonth,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub color: String,
    #[serde(rename = "type")]
    pub workspace_type: WorkspaceType,
    pub organization_name: Option<String>,
    pub worker_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppPreferences {
    pub id: Option<i64>,
    pub active_workspace_id: WorkspaceId,
    pub theme_preference: ThemePreference,
    pub language_preference: LanguagePreference,
    pub interface_scale_percent: i32,
    pub month_view_preference: MonthViewPreference,
    pub has_completed_setup: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UpdateState {
    pub status: UpdateStatus,
    pub current_version: String,
    pub available_version: Option<String>,
    pub progress: Option<i32>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkEntry {
    pub workspace_id: Option<WorkspaceId>,
    pub date: IsoDate,
    pub status: WorkEntryStatus,
    pub start_time: Option<ClockTime>,
    pub end_time: Option<ClockTime>,
    pub lunch_minutes: Minutes,
    pub project_name: Option<String>,
    pub notes: Option<String>,
    pub scheduled_minutes_override: Option<Minutes>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OvertimeRateBand {
    pub name: String,
    pub day_category: OvertimeDayCategory,
    pub start_time: ClockTime,
    pub end_time: ClockTime,
    pub compensation_type: CompensationRuleType,
    pub rate_type: CompensationRateType,
    pub rate_value: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SalarySettings {
    #[serde(rename = "type")]
    pub salary_type: SalaryType,
    pub hourly_rate: Money,
    pub monthly_salary: Money,
    pub employment_percent: Decimal,
    pub hourly_pay_basis: HourlyPayBasis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExpectedHoursSettings {
    pub hours_per_workday: Decimal,
    pub working_weekdays: Vec<u32>,
    pub exclude_public_holidays: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxSettings {
    pub mode: TaxMode,
    pub tax_year: i32,
    pub table_number: i32,
    pub column: i32,
    pub manual_monthly_deduction: Option<Money>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OvertimeCompensationSettings {
    pub mode: OvertimeCompensationMode,
    pub default_rate_type: CompensationRateType,
    pub default_rate_value: Decimal,
    pub daily_threshold_hours: Decimal,
    pub threshold_mode: OvertimeThresholdMode,
    pub rate_bands: Vec<OvertimeRateBand>,
    pub ob_overtime_combination: ObOvertimeCombinationMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub id: Option<i64>,
    pub workspace_id: Option<WorkspaceId>,
    pub employee_name: String,
    pub employer_name: String,
    pub default_project: String,
    pub salary: SalarySettings,
    pub expected_hours: ExpectedHoursSettings,
    pub default_start_time: ClockTime,
    pub default_end_time: ClockTime,
    pub default_lunch_minutes: Minutes,
    pub tax_settings: TaxSettings,
    pub theme_preference: ThemePreference,
    pub opening_balance_minutes: Minutes,
    pub month_view_preference: MonthViewPreference,
    pub language_preference: LanguagePreference,
    pub currency_preference: CurrencyPreference,
    pub interface_scale_percent: i32,
    pub export_language_preference: ExportLanguagePreference,
    pub overtime_compensation: OvertimeCompensationSettings,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthRecord {
    pub workspace_id: Option<WorkspaceId>,
    pub year: i32,
    pub month: u32,
    pub opening_balance_minutes: Minutes,
    pub expected_minutes_override: Option<Minutes>,
    pub opening_balance_was_edited: bool,
}

impl MonthRecord {
    pub fn year_month(&self) -> crate::Result<YearMonth> {
        YearMonth::new(self.year, self.month)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BalanceHistoryMonth {
    pub year: i32,
    pub month: u32,
    pub record: Option<MonthRecord>,
    pub entries: Vec<WorkEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub workspace_id: Option<WorkspaceId>,
    pub id: ProjectId,
    pub name: String,
    pub color: Option<String>,
    pub is_active: bool,
    pub is_default: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonthlySummary {
    pub year: i32,
    pub month: u32,
    pub worked_minutes: Minutes,
    pub regular_minutes: Minutes,
    pub overtime_minutes: Minutes,
    pub ordinary_paid_minutes: Minutes,
    pub balance_eligible_minutes: Minutes,
    pub expected_minutes: Minutes,
    pub monthly_difference_minutes: Minutes,
    pub opening_balance_minutes: Minutes,
    pub closing_balance_minutes: Minutes,
    pub gross_salary: Money,
    pub base_salary: Money,
    pub overtime_compensation: Money,
    pub ob_compensation: Money,
    pub ob_minutes: Minutes,
    pub completed_day_count: usize,
    pub missing_past_days: Vec<IsoDate>,
    pub worked_hours: Decimal,
    pub regular_hours: Decimal,
    pub overtime_hours: Decimal,
    pub ordinary_paid_hours: Decimal,
    pub ob_hours: Decimal,
    pub expected_hours: Decimal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DailyPayBreakdown {
    pub regular_pay: Money,
    pub overtime_pay: Money,
    pub ob_pay: Money,
    pub ob_minutes: Minutes,
    pub total: Money,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaxEstimate {
    pub gross_pay: Money,
    pub preliminary_tax: Option<Money>,
    pub estimated_net_pay: Option<Money>,
    pub unavailable_reason: TaxUnavailableReason,
    pub is_available: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportExportRequest {
    pub year: i32,
    pub month: u32,
    pub employee_name: String,
    pub employer_name: String,
    pub entries: Vec<WorkEntry>,
    pub summary: MonthlySummary,
    pub language: ExportLanguagePreference,
    pub expected_hours: Option<ExpectedHoursSettings>,
    pub overtime_settings: Option<OvertimeCompensationSettings>,
    pub overtime_mode: OvertimeCompensationMode,
    pub daily_overtime_threshold_hours: Decimal,
    pub hourly_pay_basis: HourlyPayBasis,
    pub threshold_minutes_by_date: BTreeMap<IsoDate, Minutes>,
}

pub fn default_workspace(clock: &(impl Clock + ?Sized)) -> Workspace {
    let now = clock.now_utc();
    Workspace {
        id: WorkspaceId::new("ws-default").unwrap_or_else(|_| unreachable!()),
        name: "Main Workspace".to_owned(),
        color: "#5F875F".to_owned(),
        workspace_type: WorkspaceType::Employment,
        organization_name: Some(String::new()),
        worker_name: Some(String::new()),
        created_at: now,
        updated_at: now,
    }
}

pub fn default_preferences() -> AppPreferences {
    AppPreferences {
        id: None,
        active_workspace_id: WorkspaceId::new("ws-default").unwrap_or_else(|_| unreachable!()),
        theme_preference: ThemePreference::System,
        language_preference: LanguagePreference::System,
        interface_scale_percent: 100,
        month_view_preference: MonthViewPreference::Ledger,
        has_completed_setup: false,
    }
}

pub fn default_settings() -> AppSettings {
    AppSettings {
        id: None,
        workspace_id: Some(WorkspaceId::new("ws-default").unwrap_or_else(|_| unreachable!())),
        employee_name: String::new(),
        employer_name: String::new(),
        default_project: "General".to_owned(),
        salary: SalarySettings {
            salary_type: SalaryType::Hourly,
            hourly_rate: Money::new(Decimal::new(250, 0)),
            monthly_salary: Money::new(Decimal::new(40_000, 0)),
            employment_percent: Decimal::ONE_HUNDRED,
            hourly_pay_basis: HourlyPayBasis::DailyRegularHours,
        },
        expected_hours: ExpectedHoursSettings {
            hours_per_workday: Decimal::new(8, 0),
            working_weekdays: vec![1, 2, 3, 4, 5],
            exclude_public_holidays: true,
        },
        default_start_time: "08:00".parse().unwrap_or_else(|_| unreachable!()),
        default_end_time: "16:30".parse().unwrap_or_else(|_| unreachable!()),
        default_lunch_minutes: Minutes::new(30),
        tax_settings: TaxSettings {
            mode: TaxMode::PrimaryIncomeTaxTable,
            tax_year: 2026,
            table_number: 30,
            column: 1,
            manual_monthly_deduction: None,
        },
        theme_preference: ThemePreference::System,
        opening_balance_minutes: Minutes::ZERO,
        month_view_preference: MonthViewPreference::Ledger,
        language_preference: LanguagePreference::System,
        currency_preference: CurrencyPreference::Sek,
        interface_scale_percent: 100,
        export_language_preference: ExportLanguagePreference::System,
        overtime_compensation: OvertimeCompensationSettings {
            mode: OvertimeCompensationMode::CompTime,
            default_rate_type: CompensationRateType::HourlyPremiumPercent,
            default_rate_value: Decimal::new(50, 0),
            daily_threshold_hours: Decimal::new(8, 0),
            threshold_mode: OvertimeThresholdMode::ScheduledHours,
            rate_bands: Vec::new(),
            ob_overtime_combination: ObOvertimeCombinationMode::ExcludeOb,
        },
    }
}
