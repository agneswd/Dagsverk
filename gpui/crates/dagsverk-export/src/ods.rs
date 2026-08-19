use std::{collections::BTreeMap, fs::File, io::Write, path::Path};

use chrono::NaiveDate;
use dagsverk_core::models::{LanguagePreference, ReportExportRequest, WorkEntryStatus};
use rust_decimal::Decimal;
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    ExportError, Result,
    localization::{is_english, month_title, text},
    request::{entry_worked_hours, has_ob, overtime_or_comp_hours, paid_hours, uses_monthly_basis},
    validation::validate_request,
};

const MIME: &str = "application/vnd.oasis.opendocument.spreadsheet";

pub fn export_ods(
    request: &ReportExportRequest,
    output: &Path,
    system_language: LanguagePreference,
) -> Result<()> {
    validate_request(request, output, "ods")?;
    let file = File::create(output).map_err(|source| ExportError::Io {
        path: output.to_owned(),
        source,
    })?;
    let mut zip = ZipWriter::new(file);
    zip.start_file(
        "mimetype",
        SimpleFileOptions::default().compression_method(CompressionMethod::Stored),
    )?;
    zip.write_all(MIME.as_bytes())
        .map_err(|source| ExportError::Io {
            path: output.to_owned(),
            source,
        })?;
    let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    for (name, contents) in [
        ("content.xml", content(request, system_language)),
        ("styles.xml", STYLES.to_owned()),
        ("meta.xml", META.to_owned()),
        ("META-INF/manifest.xml", MANIFEST.to_owned()),
    ] {
        zip.start_file(name, options)?;
        zip.write_all(contents.as_bytes())
            .map_err(|source| ExportError::Io {
                path: output.to_owned(),
                source,
            })?;
    }
    zip.finish()?;
    Ok(())
}

fn content(request: &ReportExportRequest, system: LanguagePreference) -> String {
    let english = is_english(request, system);
    format!(
        r##"<?xml version="1.0" encoding="UTF-8"?>
<office:document-content xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:table="urn:oasis:names:tc:opendocument:xmlns:table:1.0" xmlns:text="urn:oasis:names:tc:opendocument:xmlns:text:1.0" xmlns:style="urn:oasis:names:tc:opendocument:xmlns:style:1.0" xmlns:fo="urn:oasis:names:tc:opendocument:xmlns:xsl-fo-compatible:1.0" office:version="1.3">
<office:automatic-styles><style:style style:name="title" style:family="table-cell"><style:text-properties fo:font-size="16pt" fo:font-weight="bold"/></style:style><style:style style:name="heading" style:family="table-cell"><style:table-cell-properties fo:background-color="#E3ECE7"/><style:text-properties fo:font-weight="bold"/></style:style><style:style style:name="bold" style:family="table-cell"><style:text-properties fo:font-weight="bold"/></style:style></office:automatic-styles>
<office:body><office:spreadsheet>{}{}</office:spreadsheet></office:body></office:document-content>"##,
        report_table(request, english),
        balance_table(request, english)
    )
}

#[derive(Clone)]
enum Cell {
    Empty,
    Text(String, Option<&'static str>),
    Number(Decimal, Option<&'static str>),
}

fn report_table(request: &ReportExportRequest, english: bool) -> String {
    let mut rows = vec![
        vec![Cell::Text(
            text(english, "Dagsverk - Time report", "Dagsverk - Tidrapport").to_owned(),
            Some("title"),
        )],
        vec![
            Cell::Text(request.employee_name.clone(), None),
            Cell::Empty,
            Cell::Empty,
            Cell::Text(request.employer_name.clone(), None),
        ],
        vec![],
        vec![
            heading(text(english, "Day", "Dag")),
            heading("Start"),
            heading(text(english, "Stop", "Slut")),
            heading("Lunch"),
            heading(text(english, "Hours", "Timmar")),
            heading("Status"),
            heading(text(english, "Project", "Projekt")),
        ],
    ];
    let entries: BTreeMap<_, _> = request
        .entries
        .iter()
        .map(|entry| (entry.date, entry))
        .collect();
    for day in 1..=days_in_month(request.year, request.month) {
        let date = NaiveDate::from_ymd_opt(request.year, request.month, day)
            .unwrap_or_else(|| unreachable!());
        let entry = entries.get(&dagsverk_core::models::IsoDate::new(date));
        let mut row = vec![Cell::Number(Decimal::from(day), None)];
        match entry {
            Some(entry) if entry.status == WorkEntryStatus::Worked => row.extend([
                Cell::Text(
                    entry
                        .start_time
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                    None,
                ),
                Cell::Text(
                    entry
                        .end_time
                        .map(|value| value.to_string())
                        .unwrap_or_default(),
                    None,
                ),
                Cell::Number(Decimal::from(entry.lunch_minutes.value()), None),
                Cell::Number(entry_worked_hours(entry), None),
                Cell::Empty,
                Cell::Text(entry.project_name.clone().unwrap_or_default(), None),
            ]),
            Some(entry) if entry.status == WorkEntryStatus::Off => row.extend([
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Empty,
                Cell::Text(text(english, "Day off", "Ledig").to_owned(), None),
            ]),
            _ => {}
        }
        rows.push(row);
    }
    rows.push(vec![]);
    rows.push(total_row(
        text(
            english,
            if uses_monthly_basis(request) {
                "Total paid hours"
            } else {
                "Total regular hours"
            },
            if uses_monthly_basis(request) {
                "Totalt betalda timmar"
            } else {
                "Totalt ordinarie timmar"
            },
        ),
        paid_hours(request),
    ));
    rows.push(total_row(
        text(
            english,
            if uses_monthly_basis(request) {
                "Comp time earned"
            } else {
                "Total overtime"
            },
            if uses_monthly_basis(request) {
                "Intjänad komptid"
            } else {
                "Total övertid"
            },
        ),
        overtime_or_comp_hours(request),
    ));
    if has_ob(request) {
        rows.push(total_row(
            text(english, "Total OB hours", "Totala OB-timmar"),
            request.summary.ob_hours,
        ));
    }
    table(&month_title(request, english), &rows)
}

fn balance_table(request: &ReportExportRequest, english: bool) -> String {
    let mut rows = vec![
        vec![Cell::Text(
            text(
                english,
                "Time balance - personal tracking",
                "Tidsbalans - personlig uppföljning",
            )
            .to_owned(),
            Some("title"),
        )],
        vec![
            Cell::Text(text(english, "Month", "Månad").to_owned(), None),
            Cell::Text(month_title(request, english), None),
        ],
        vec![],
        balance_row(
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
            paid_hours(request),
        ),
        balance_row(
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
            overtime_or_comp_hours(request),
        ),
        balance_row(
            text(english, "Worked hours", "Arbetade timmar"),
            request.summary.worked_hours,
        ),
    ];
    if has_ob(request) {
        rows.push(balance_row(
            text(english, "OB hours", "OB-timmar"),
            request.summary.ob_hours,
        ));
    }
    rows.extend([
        balance_row(
            text(english, "Expected hours", "Förväntade timmar"),
            request.summary.expected_hours,
        ),
        balance_row(
            text(english, "Monthly time balance", "Månadens tidsbalans"),
            Decimal::from(request.summary.monthly_difference_minutes.value()) / Decimal::from(60),
        ),
        balance_row(
            text(english, "Opening time balance", "Ingående tidsbalans"),
            Decimal::from(request.summary.opening_balance_minutes.value()) / Decimal::from(60),
        ),
        balance_row(
            text(english, "Closing time balance", "Utgående tidsbalans"),
            Decimal::from(request.summary.closing_balance_minutes.value()) / Decimal::from(60),
        ),
    ]);
    table(text(english, "Time balance", "Tidsbalans"), &rows)
}

fn heading(value: &str) -> Cell {
    Cell::Text(value.to_owned(), Some("heading"))
}
fn total_row(label: &str, value: Decimal) -> Vec<Cell> {
    vec![
        Cell::Empty,
        Cell::Empty,
        Cell::Empty,
        Cell::Text(label.to_owned(), Some("bold")),
        Cell::Number(value, Some("bold")),
    ]
}
fn balance_row(label: &str, value: Decimal) -> Vec<Cell> {
    vec![
        Cell::Text(label.to_owned(), Some("bold")),
        Cell::Number(value, None),
    ]
}

fn table(name: &str, rows: &[Vec<Cell>]) -> String {
    format!(
        "<table:table table:name=\"{}\">{}</table:table>",
        escape(name),
        rows.iter()
            .map(|row| format!(
                "<table:table-row>{}</table:table-row>",
                row.iter().map(cell).collect::<String>()
            ))
            .collect::<String>()
    )
}
fn cell(cell: &Cell) -> String {
    let (style, body) = match cell {
        Cell::Empty => (
            None,
            " office:value-type=\"string\"><text:p></text:p>".to_owned(),
        ),
        Cell::Text(value, style) => (
            *style,
            format!(
                " office:value-type=\"string\"><text:p>{}</text:p>",
                escape(value)
            ),
        ),
        Cell::Number(value, style) => {
            let value = value.normalize();
            (
                *style,
                format!(
                    " office:value-type=\"float\" office:value=\"{value}\"><text:p>{value}</text:p>"
                ),
            )
        }
    };
    format!(
        "<table:table-cell{}{body}</table:table-cell>",
        style
            .map(|style| format!(" table:style-name=\"{style}\""))
            .unwrap_or_default()
    )
}
fn escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn days_in_month(year: i32, month: u32) -> u32 {
    let next = if month == 12 {
        NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        NaiveDate::from_ymd_opt(year, month + 1, 1)
    }
    .unwrap_or_else(|| unreachable!());
    (next - NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(|| unreachable!())).num_days()
        as u32
}

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8"?><manifest:manifest xmlns:manifest="urn:oasis:names:tc:opendocument:xmlns:manifest:1.0" manifest:version="1.3"><manifest:file-entry manifest:full-path="/" manifest:media-type="application/vnd.oasis.opendocument.spreadsheet"/><manifest:file-entry manifest:full-path="content.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="styles.xml" manifest:media-type="text/xml"/><manifest:file-entry manifest:full-path="meta.xml" manifest:media-type="text/xml"/></manifest:manifest>"#;
const STYLES: &str = r#"<?xml version="1.0" encoding="UTF-8"?><office:document-styles xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" office:version="1.3"><office:styles/></office:document-styles>"#;
const META: &str = r#"<?xml version="1.0" encoding="UTF-8"?><office:document-meta xmlns:office="urn:oasis:names:tc:opendocument:xmlns:office:1.0" xmlns:meta="urn:oasis:names:tc:opendocument:xmlns:meta:1.0" office:version="1.3"><office:meta><meta:generator>Dagsverk</meta:generator></office:meta></office:document-meta>"#;
