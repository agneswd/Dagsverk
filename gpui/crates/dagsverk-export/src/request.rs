use dagsverk_core::{
    calculations::worked_minutes,
    models::{HourlyPayBasis, OvertimeCompensationMode, ReportExportRequest},
};
use rust_decimal::Decimal;

pub fn uses_monthly_basis(request: &ReportExportRequest) -> bool {
    request.overtime_mode == OvertimeCompensationMode::CompTime
        && request.hourly_pay_basis == HourlyPayBasis::MonthlyExpectedHours
}

pub fn paid_hours(request: &ReportExportRequest) -> Decimal {
    if uses_monthly_basis(request) {
        request.summary.ordinary_paid_hours
    } else {
        request.summary.regular_hours
    }
}

pub fn overtime_or_comp_hours(request: &ReportExportRequest) -> Decimal {
    if uses_monthly_basis(request) {
        (request.summary.worked_hours - paid_hours(request)).max(Decimal::ZERO)
    } else {
        request.summary.overtime_hours
    }
}

pub fn has_ob(request: &ReportExportRequest) -> bool {
    request.summary.ob_hours != Decimal::ZERO
        || request.overtime_settings.as_ref().is_some_and(|settings| {
            settings.rate_bands.iter().any(|band| {
                band.compensation_type == dagsverk_core::models::CompensationRuleType::Ob
            })
        })
}

pub fn entry_worked_hours(entry: &dagsverk_core::models::WorkEntry) -> Decimal {
    Decimal::from(worked_minutes(entry).value()) / Decimal::from(60)
}
