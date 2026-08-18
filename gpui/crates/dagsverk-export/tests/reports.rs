use std::{collections::BTreeMap, fs::File, io::Read};

use dagsverk_core::models::{
    ExportLanguagePreference, HourlyPayBasis, LanguagePreference, Minutes, Money, MonthlySummary,
    OvertimeCompensationMode, ReportExportRequest, WorkEntry, WorkEntryStatus,
};
use dagsverk_export::{ExportError, export_ods, export_xlsx};
use quick_xml::{Reader, events::Event};
use rust_decimal::Decimal;
use tempfile::tempdir;
use zip::{CompressionMethod, ZipArchive};

fn request(language: ExportLanguagePreference) -> ReportExportRequest {
    let worked = WorkEntry {
        workspace_id: None,
        date: "2026-08-17".parse().expect("date"),
        status: WorkEntryStatus::Worked,
        start_time: Some("22:00".parse().expect("time")),
        end_time: Some("06:00".parse().expect("time")),
        lunch_minutes: Minutes::ZERO,
        project_name: Some("A&B <Night>".to_owned()),
        notes: None,
        scheduled_minutes_override: None,
        created_at: None,
        updated_at: None,
    };
    let off = WorkEntry {
        date: "2026-08-18".parse().expect("date"),
        status: WorkEntryStatus::Off,
        start_time: None,
        end_time: None,
        lunch_minutes: Minutes::ZERO,
        project_name: None,
        ..worked.clone()
    };
    ReportExportRequest {
        year: 2026,
        month: 8,
        employee_name: "Employee & Co".to_owned(),
        employer_name: "Employer <AB>".to_owned(),
        entries: vec![worked, off],
        summary: MonthlySummary {
            year: 2026,
            month: 8,
            worked_minutes: Minutes::new(480),
            regular_minutes: Minutes::new(420),
            overtime_minutes: Minutes::new(60),
            ordinary_paid_minutes: Minutes::new(420),
            balance_eligible_minutes: Minutes::new(480),
            expected_minutes: Minutes::new(480),
            monthly_difference_minutes: Minutes::ZERO,
            opening_balance_minutes: Minutes::new(60),
            closing_balance_minutes: Minutes::new(60),
            gross_salary: Money::ZERO,
            base_salary: Money::ZERO,
            overtime_compensation: Money::ZERO,
            ob_compensation: Money::ZERO,
            ob_minutes: Minutes::new(480),
            completed_day_count: 2,
            missing_past_days: vec![],
            worked_hours: Decimal::new(8, 0),
            regular_hours: Decimal::new(7, 0),
            overtime_hours: Decimal::new(1, 0),
            ordinary_paid_hours: Decimal::new(7, 0),
            ob_hours: Decimal::new(8, 0),
            expected_hours: Decimal::new(8, 0),
        },
        language,
        expected_hours: None,
        overtime_settings: None,
        overtime_mode: OvertimeCompensationMode::CompTime,
        daily_overtime_threshold_hours: Decimal::new(7, 0),
        hourly_pay_basis: HourlyPayBasis::DailyRegularHours,
        threshold_minutes_by_date: BTreeMap::from([(
            "2026-08-17".parse().expect("date"),
            Minutes::new(420),
        )]),
    }
}

#[test]
fn request_validation_rejects_extension_duplicate_outside_month_and_lunch() {
    let directory = tempdir().expect("directory");
    let mut report = request(ExportLanguagePreference::English);
    assert!(matches!(
        export_xlsx(
            &report,
            &directory.path().join("report.ods"),
            LanguagePreference::English
        ),
        Err(ExportError::InvalidExtension { .. })
    ));
    report.entries.push(report.entries[0].clone());
    assert!(matches!(
        export_xlsx(
            &report,
            &directory.path().join("report.xlsx"),
            LanguagePreference::English
        ),
        Err(ExportError::DuplicateDate(_))
    ));
    report.entries.pop();
    report.entries[0].date = "2026-09-01".parse().expect("date");
    assert!(matches!(
        export_ods(
            &report,
            &directory.path().join("report.ods"),
            LanguagePreference::English
        ),
        Err(ExportError::EntryOutsideMonth(_))
    ));
    report.entries[0].date = "2026-08-17".parse().expect("date");
    report.entries[0].lunch_minutes = Minutes::new(-1);
    assert!(matches!(
        export_ods(
            &report,
            &directory.path().join("report.ods"),
            LanguagePreference::English
        ),
        Err(ExportError::InvalidLunch(_))
    ));
    report.entries[0].lunch_minutes = Minutes::ZERO;
    report.entries[0].start_time = None;
    assert!(matches!(
        export_xlsx(
            &report,
            &directory.path().join("report.xlsx"),
            LanguagePreference::English
        ),
        Err(ExportError::InvalidTime(_))
    ));
}

#[test]
fn xlsx_contains_semantic_sheets_formulas_formats_and_hidden_helper() {
    let directory = tempdir().expect("directory");
    let path = directory.path().join("report.xlsx");
    export_xlsx(
        &request(ExportLanguagePreference::English),
        &path,
        LanguagePreference::Swedish,
    )
    .expect("XLSX export");
    let mut archive = ZipArchive::new(File::open(path).expect("workbook")).expect("XLSX zip");
    let workbook = read(&mut archive, "xl/workbook.xml");
    assert!(workbook.contains("August 2026"));
    assert!(workbook.contains("Time balance"));
    let report = read(&mut archive, "xl/worksheets/sheet1.xml");
    assert!(report.contains("hidden=\"1\""));
    assert!(report.contains("MOD(C21-B21,1)"));
    assert!(report.contains("SUM(E5:E35)-SUM(H5:H35)"));
    assert!(report.contains("<v>8</v>"));
    let balance = read(&mut archive, "xl/worksheets/sheet2.xml");
    assert!(balance.contains("B4+B5"));
    assert!(balance.contains("B9+B10"));
    assert!(read(&mut archive, "xl/styles.xml").contains("numFmt"));
}

#[test]
fn ods_has_required_package_order_valid_xml_localization_and_escaping() {
    let directory = tempdir().expect("directory");
    let english_path = directory.path().join("english.ods");
    export_ods(
        &request(ExportLanguagePreference::System),
        &english_path,
        LanguagePreference::English,
    )
    .expect("ODS export");
    let mut archive = ZipArchive::new(File::open(english_path).expect("ODS")).expect("ODS zip");
    assert_eq!(archive.by_index(0).expect("first entry").name(), "mimetype");
    assert_eq!(
        archive.by_index(0).expect("first entry").compression(),
        CompressionMethod::Stored
    );
    let content = read(&mut archive, "content.xml");
    assert!(content.contains("August 2026"));
    assert!(content.contains("Time balance"));
    assert!(content.contains("A&amp;B &lt;Night&gt;"));
    assert_well_formed(&content);
    for name in ["styles.xml", "meta.xml", "META-INF/manifest.xml"] {
        assert_well_formed(&read(&mut archive, name));
    }

    let swedish_path = directory.path().join("swedish.ods");
    export_ods(
        &request(ExportLanguagePreference::Swedish),
        &swedish_path,
        LanguagePreference::English,
    )
    .expect("Swedish ODS");
    let mut swedish = ZipArchive::new(File::open(swedish_path).expect("ODS")).expect("ODS zip");
    let content = read(&mut swedish, "content.xml");
    assert!(content.contains("augusti 2026"));
    assert!(content.contains("Tidsbalans"));
    assert!(content.contains("Ledig"));
}

#[test]
fn swedish_xlsx_and_monthly_pay_basis_use_parity_labels_and_values() {
    let directory = tempdir().expect("directory");
    let path = directory.path().join("swedish.xlsx");
    let mut report = request(ExportLanguagePreference::Swedish);
    report.hourly_pay_basis = HourlyPayBasis::MonthlyExpectedHours;
    report.summary.ordinary_paid_hours = Decimal::new(75, 1);
    export_xlsx(&report, &path, LanguagePreference::English).expect("XLSX export");
    let mut archive = ZipArchive::new(File::open(path).expect("workbook")).expect("XLSX zip");
    let workbook = read(&mut archive, "xl/workbook.xml");
    assert!(workbook.contains("augusti 2026"));
    assert!(workbook.contains("Tidsbalans"));
    let strings = read(&mut archive, "xl/sharedStrings.xml");
    assert!(strings.contains("Totalt betalda timmar"));
    assert!(strings.contains("Intjänad komptid"));
    assert!(read(&mut archive, "xl/worksheets/sheet1.xml").contains("<v>7.5</v>"));
}

fn read(archive: &mut ZipArchive<File>, name: &str) -> String {
    let mut output = String::new();
    archive
        .by_name(name)
        .expect("archive entry")
        .read_to_string(&mut output)
        .expect("read entry");
    output
}

fn assert_well_formed(xml: &str) {
    let mut reader = Reader::from_str(xml);
    loop {
        if matches!(reader.read_event().expect("valid XML"), Event::Eof) {
            break;
        }
    }
}
