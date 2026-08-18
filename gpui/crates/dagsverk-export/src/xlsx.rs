use std::{collections::BTreeMap, path::Path};

use chrono::NaiveDate;
use dagsverk_core::{
    calculations::{time_to_minutes, worked_minutes},
    models::{LanguagePreference, ReportExportRequest, WorkEntryStatus},
};
use rust_decimal::{Decimal, prelude::ToPrimitive};
use rust_xlsxwriter::{Color, Format, Formula, Workbook};

use crate::{
    Result,
    localization::{is_english, month_title, text},
    request::{has_ob, overtime_or_comp_hours, paid_hours, uses_monthly_basis},
    validation::validate_request,
};

pub fn export_xlsx(
    request: &ReportExportRequest,
    output: &Path,
    system_language: LanguagePreference,
) -> Result<()> {
    validate_request(request, output, "xlsx")?;
    let english = is_english(request, system_language);
    let title = month_title(request, english);
    let mut workbook = Workbook::new();
    let title_format = Format::new().set_bold().set_font_size(16);
    let heading_format = Format::new()
        .set_bold()
        .set_background_color(Color::RGB(0xE3ECE7));
    let bold = Format::new().set_bold();
    let time_format = Format::new().set_num_format("hh:mm");
    let hours_format = Format::new().set_num_format("0.00");
    let day_count = days_in_month(request.year, request.month);
    let totals_excel_row = day_count + 6;
    {
        let sheet = workbook.add_worksheet();
        sheet.set_name(&title)?;
        sheet.set_freeze_panes(4, 0)?;
        sheet.merge_range(
            0,
            0,
            0,
            6,
            text(english, "Dagsverk - Time report", "Dagsverk - Tidrapport"),
            &title_format,
        )?;
        sheet.write_string(1, 0, &request.employee_name)?;
        sheet.write_string(1, 3, &request.employer_name)?;
        for (column, heading) in [
            text(english, "Day", "Dag"),
            "Start",
            text(english, "Stop", "Slut"),
            "Lunch",
            text(english, "Hours", "Timmar"),
            "Status",
            text(english, "Project", "Projekt"),
        ]
        .into_iter()
        .enumerate()
        {
            sheet.write_string_with_format(3, column as u16, heading, &heading_format)?;
        }
        let entries: BTreeMap<_, _> = request
            .entries
            .iter()
            .map(|entry| (entry.date, entry))
            .collect();
        for day in 1..=day_count {
            let row = day + 3;
            let date = dagsverk_core::models::IsoDate::new(
                NaiveDate::from_ymd_opt(request.year, request.month, day)
                    .unwrap_or_else(|| unreachable!()),
            );
            sheet.write_number(row, 0, day)?;
            if let Some(entry) = entries.get(&date) {
                if entry.status == WorkEntryStatus::Worked
                    && let (Some(start), Some(end)) = (entry.start_time, entry.end_time)
                {
                    sheet.write_number_with_format(
                        row,
                        1,
                        time_to_minutes(start) as f64 / 1440.0,
                        &time_format,
                    )?;
                    sheet.write_number_with_format(
                        row,
                        2,
                        time_to_minutes(end) as f64 / 1440.0,
                        &time_format,
                    )?;
                    sheet.write_number_with_format(
                        row,
                        3,
                        entry.lunch_minutes.value() as f64 / 1440.0,
                        &time_format,
                    )?;
                    let worked = Decimal::from(worked_minutes(entry).value()) / Decimal::from(60);
                    let excel_row = row + 1;
                    sheet.write_formula_with_format(
                        row, 4,
                        Formula::new(format!("IF(OR(B{excel_row}=\"\",C{excel_row}=\"\"),\"\",MAX(0,(MOD(C{excel_row}-B{excel_row},1)-D{excel_row})*24))")).set_result(worked.normalize().to_string()),
                        &hours_format,
                    )?;
                    let threshold = request
                        .threshold_minutes_by_date
                        .get(&date)
                        .copied()
                        .or(entry.scheduled_minutes_override)
                        .map_or(
                            request.daily_overtime_threshold_hours * Decimal::from(60),
                            |value| Decimal::from(value.value()),
                        )
                        / Decimal::from(60);
                    let overtime = (worked - threshold).max(Decimal::ZERO);
                    sheet.write_formula_with_format(
                        row, 7,
                        Formula::new(format!("IF(OR(B{excel_row}=\"\",C{excel_row}=\"\"),\"\",MAX(0,(MOD(C{excel_row}-B{excel_row},1)-D{excel_row})*24-{}))", threshold.normalize())).set_result(overtime.normalize().to_string()),
                        &hours_format,
                    )?;
                    sheet.write_string(row, 6, entry.project_name.as_deref().unwrap_or(""))?;
                } else if entry.status == WorkEntryStatus::Off {
                    sheet.write_string(row, 5, text(english, "Day off", "Ledig"))?;
                }
            }
        }
        let totals = totals_excel_row - 1;
        let first = 5;
        let last = day_count + 4;
        if uses_monthly_basis(request) {
            sheet.write_string_with_format(
                totals,
                3,
                text(english, "Total paid hours", "Totalt betalda timmar"),
                &bold,
            )?;
            sheet.write_number_with_format(
                totals,
                4,
                decimal(paid_hours(request)),
                &hours_format,
            )?;
            sheet.write_string_with_format(
                totals + 1,
                3,
                text(english, "Comp time earned", "Intjänad komptid"),
                &bold,
            )?;
            sheet.write_number_with_format(
                totals + 1,
                4,
                decimal(overtime_or_comp_hours(request)),
                &hours_format,
            )?;
        } else {
            sheet.write_string_with_format(
                totals,
                3,
                text(english, "Total regular hours", "Totalt ordinarie timmar"),
                &bold,
            )?;
            sheet.write_formula_with_format(
                totals,
                4,
                Formula::new(format!("SUM(E{first}:E{last})-SUM(H{first}:H{last})"))
                    .set_result(request.summary.regular_hours.normalize().to_string()),
                &hours_format,
            )?;
            sheet.write_string_with_format(
                totals + 1,
                3,
                text(english, "Total overtime", "Total övertid"),
                &bold,
            )?;
            sheet.write_formula_with_format(
                totals + 1,
                4,
                Formula::new(format!("SUM(H{first}:H{last})"))
                    .set_result(request.summary.overtime_hours.normalize().to_string()),
                &hours_format,
            )?;
        }
        if has_ob(request) {
            sheet.write_string_with_format(
                totals + 2,
                3,
                text(english, "Total OB hours", "Totala OB-timmar"),
                &bold,
            )?;
            sheet.write_number_with_format(
                totals + 2,
                4,
                decimal(request.summary.ob_hours),
                &hours_format,
            )?;
        }
        for (column, width) in [8.0, 12.0, 12.0, 25.0, 18.0, 14.0, 24.0]
            .into_iter()
            .enumerate()
        {
            sheet.set_column_width(column as u16, width)?;
        }
        sheet.set_column_hidden(7)?;
    }
    {
        let sheet = workbook.add_worksheet();
        sheet.set_name(text(english, "Time balance", "Tidsbalans"))?;
        sheet.merge_range(
            0,
            0,
            0,
            1,
            text(
                english,
                "Time balance - personal tracking",
                "Tidsbalans - personlig uppföljning",
            ),
            &title_format,
        )?;
        sheet.write_string(1, 0, text(english, "Month", "Månad"))?;
        sheet.write_string(1, 1, &title)?;
        let report_ref = format!("'{}'", title.replace('\'', "''"));
        let labels = [
            text(
                english,
                if uses_monthly_basis(request) {
                    "Paid hours"
                } else {
                    "Regular hours"
                },
                if uses_monthly_basis(request) {
                    "Betalda timmar"
                } else {
                    "Ordinarie timmar"
                },
            ),
            text(
                english,
                if uses_monthly_basis(request) {
                    "Comp time earned"
                } else {
                    "Overtime"
                },
                if uses_monthly_basis(request) {
                    "Intjänad komptid"
                } else {
                    "Övertid"
                },
            ),
            text(english, "Worked hours", "Arbetade timmar"),
        ];
        for (index, label) in labels.into_iter().enumerate() {
            sheet.write_string_with_format(index as u32 + 3, 0, label, &bold)?;
        }
        sheet.write_formula_with_format(
            3,
            1,
            Formula::new(format!("{report_ref}!E{totals_excel_row}"))
                .set_result(paid_hours(request).normalize().to_string()),
            &hours_format,
        )?;
        sheet.write_formula_with_format(
            4,
            1,
            Formula::new(format!("{report_ref}!E{}", totals_excel_row + 1))
                .set_result(overtime_or_comp_hours(request).normalize().to_string()),
            &hours_format,
        )?;
        sheet.write_formula_with_format(
            5,
            1,
            Formula::new("B4+B5").set_result(request.summary.worked_hours.normalize().to_string()),
            &hours_format,
        )?;
        if has_ob(request) {
            sheet.write_string_with_format(6, 0, text(english, "OB hours", "OB-timmar"), &bold)?;
            sheet.write_number_with_format(
                6,
                1,
                decimal(request.summary.ob_hours),
                &hours_format,
            )?;
        }
        for (row, label, value) in [
            (
                7,
                text(english, "Expected hours", "Förväntade timmar"),
                request.summary.expected_hours,
            ),
            (
                8,
                text(english, "Monthly time balance", "Månadens tidsbalans"),
                Decimal::from(request.summary.monthly_difference_minutes.value())
                    / Decimal::from(60),
            ),
            (
                9,
                text(english, "Opening time balance", "Ingående tidsbalans"),
                Decimal::from(request.summary.opening_balance_minutes.value()) / Decimal::from(60),
            ),
        ] {
            sheet.write_string_with_format(row, 0, label, &bold)?;
            sheet.write_number_with_format(row, 1, decimal(value), &hours_format)?;
        }
        sheet.write_string_with_format(
            10,
            0,
            text(english, "Closing time balance", "Utgående tidsbalans"),
            &bold,
        )?;
        sheet.write_formula_with_format(
            10,
            1,
            Formula::new("B9+B10").set_result(
                (Decimal::from(request.summary.closing_balance_minutes.value())
                    / Decimal::from(60))
                .normalize()
                .to_string(),
            ),
            &hours_format,
        )?;
        sheet.set_column_width(0, 34)?;
        sheet.set_column_width(1, 18)?;
    }
    workbook.save(output)?;
    Ok(())
}

fn decimal(value: Decimal) -> f64 {
    value.to_f64().unwrap_or(0.0)
}
fn days_in_month(year: i32, month: u32) -> u32 {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(|| unreachable!());
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap_or_else(|| unreachable!());
    (next - first).num_days() as u32
}
