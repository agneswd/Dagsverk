use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate};
use dagsverk_core::{
    calculations::{dates_in_month, is_scheduled_workday, split_overtime, worked_minutes},
    holidays::SwedishHolidayCalendar,
    i18n::translate,
    models::{
        AppSettings, IsoDate, LanguagePreference, MonthViewPreference, MonthlySummary, Project,
        TaxEstimate, WorkEntry, WorkEntryStatus, YearMonth,
    },
};
use gpui::{
    BoxShadow, Context, EventEmitter, Hsla, KeyDownEvent, Render, Window, div, point, prelude::*,
    px, relative, rgb,
};

use crate::m3::{
    FOCUS_OPACITY, HOVER_OPACITY, M3ColorScheme, M3TypographyExt, TypographyRole, UiScale, m3_card,
    m3_icon_colored, m3_state_layer,
};

const LEDGER_COLUMN_RATIOS: [f32; 8] = [
    0.110_615, 0.143_567, 0.152_247, 0.088_208, 0.090_673, 0.113_213, 0.131_126, 0.170_351,
];

#[derive(Clone)]
pub struct MonthViewData {
    pub month: YearMonth,
    pub entries: Vec<WorkEntry>,
    pub settings: AppSettings,
    pub projects: Vec<Project>,
    pub today: IsoDate,
    pub selected_date: Option<IsoDate>,
    pub month_started: bool,
    pub mode: MonthViewPreference,
    pub language: LanguagePreference,
    pub colors: M3ColorScheme,
    pub scale: UiScale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthViewEvent(pub IsoDate);

pub struct MonthView {
    data: MonthViewData,
}

impl MonthView {
    pub fn new(data: MonthViewData) -> Self {
        Self { data }
    }

    pub fn set_data(&mut self, data: MonthViewData, cx: &mut Context<Self>) {
        self.data = data;
        cx.notify();
    }

    fn activate(&mut self, date: IsoDate, event: &KeyDownEvent, cx: &mut Context<Self>) {
        if matches!(event.keystroke.key.as_str(), "enter" | "space" | " ") {
            cx.stop_propagation();
            cx.emit(MonthViewEvent(date));
        }
    }

    fn project_color(&self, name: Option<&str>) -> Hsla {
        name.and_then(|name| {
            self.data
                .projects
                .iter()
                .find(|project| project.name.eq_ignore_ascii_case(name))
                .and_then(|project| project.color.as_deref())
        })
        .and_then(parse_hex)
        .unwrap_or(self.data.colors.primary)
    }

    fn ledger(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let colors = self.data.colors;
        let scale = self.data.scale;
        let rows = self.rows();
        let last_row = rows.len().saturating_sub(1);
        m3_card(colors)
            .border_0()
            .rounded(scale.px(16.0))
            .bg(colors.surface_container_low)
            .overflow_hidden()
            .child(
                div()
                    .h(scale.px(52.0))
                    .rounded_t(scale.px(16.0))
                    .flex()
                    .items_center()
                    .bg(colors.surface_container)
                    .border_b_1()
                    .border_color(colors.grid_line)
                    .m3_typography(TypographyRole::LabelMedium, scale)
                    .text_color(colors.on_surface_variant)
                    .children(
                        [
                            "Date",
                            "Status",
                            "Logged hours",
                            "Lunch",
                            "Hours",
                            "Overtime",
                            "Project",
                            "Notes",
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(index, key)| {
                            ledger_cell(index, scale)
                                .child(localized(self.data.language, key).to_uppercase())
                        }),
                    ),
            )
            .children(rows.into_iter().enumerate().map(|(index, row)| {
                let date = row.date;
                let date_for_key = date;
                let date_for_click = date;
                let status = row.status_label(self.data.language);
                let status_color = if row.holiday.is_some() {
                    colors.warning_container
                } else {
                    match row.status {
                        WorkEntryStatus::Worked => colors.success_container,
                        WorkEntryStatus::Off => colors.surface_container_high,
                        WorkEntryStatus::Incomplete if row.is_missing => colors.warning_container,
                        WorkEntryStatus::Incomplete => colors.surface_container_low,
                    }
                };
                let status_foreground = if row.holiday.is_some() {
                    colors.on_warning_container
                } else if row.status == WorkEntryStatus::Worked {
                    colors.on_primary_container
                } else {
                    colors.on_surface_variant
                };
                let project_color = self.project_color(row.project_name.as_deref());
                let row_background = colors.surface_container_low;
                div()
                    .id(("ledger-row", index))
                    .tab_index(0)
                    .h(scale.px(52.0))
                    .when(index == last_row, |row| row.rounded_b(scale.px(16.0)))
                    .flex()
                    .items_center()
                    .border_b_1()
                    .border_color(colors.grid_line)
                    .bg(row_background)
                    .m3_typography(TypographyRole::BodyMedium, scale)
                    .cursor_pointer()
                    .focus(move |style| style.shadow(focus_shadow(colors.primary, scale)))
                    .hover(move |style| {
                        style.bg(m3_state_layer(
                            row_background,
                            colors.on_surface,
                            HOVER_OPACITY,
                        ))
                    })
                    .on_click(
                        cx.listener(move |_, _, _, cx| cx.emit(MonthViewEvent(date_for_click))),
                    )
                    .on_key_down(
                        cx.listener(move |view, event, _, cx| {
                            view.activate(date_for_key, event, cx)
                        }),
                    )
                    .child(
                        ledger_cell(0, scale).child(
                            div()
                                .w(scale.px(62.0))
                                .h(scale.px(26.0))
                                .px(scale.px(6.0))
                                .grid()
                                .grid_cols(2)
                                .gap(scale.px(4.0))
                                .items_center()
                                .rounded(scale.px(8.0))
                                .bg(if row.is_today {
                                    colors.primary
                                } else if row.date.as_naive_date().weekday().number_from_monday()
                                    > 5
                                {
                                    colors.surface_container
                                } else {
                                    gpui::transparent_black()
                                })
                                .text_color(if row.is_today {
                                    colors.on_primary
                                } else {
                                    colors.on_surface
                                })
                                .child(
                                    div()
                                        .text_align(gpui::TextAlign::Right)
                                        .font_weight(gpui::FontWeight::BOLD)
                                        .child(format!("{:02}", row.day)),
                                )
                                .child(
                                    div()
                                        .text_size(scale.px(11.0))
                                        .font_weight(gpui::FontWeight::SEMIBOLD)
                                        .text_color(if row.is_today {
                                            colors.on_primary
                                        } else {
                                            colors.on_surface_variant
                                        })
                                        .child(row.weekday.to_uppercase()),
                                ),
                        ),
                    )
                    .child(
                        ledger_cell(1, scale).child(
                            div()
                                .w(scale.px(90.0))
                                .h(scale.px(26.0))
                                .px(scale.px(10.0))
                                .rounded(scale.px(13.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .gap(scale.px(8.0))
                                .bg(status_color)
                                .text_color(status_foreground)
                                .text_size(scale.px(12.0))
                                .when_some(row.status_icon(), |chip, icon| {
                                    chip.child(m3_icon_colored(
                                        icon,
                                        14.0 * scale.factor(),
                                        status_foreground,
                                    ))
                                })
                                .when(row.is_missing && row.holiday.is_none(), |chip| {
                                    chip.child(
                                        div().size(scale.px(6.0)).rounded_full().bg(colors.warning),
                                    )
                                })
                                .child(status),
                        ),
                    )
                    .child(ledger_cell(2, scale).child(numeric_value(row.interval(), colors)))
                    .child(ledger_cell(3, scale).child(numeric_value(row.lunch(), colors)))
                    .child(ledger_cell(4, scale).child(numeric_value(row.hours(), colors)))
                    .child(
                        ledger_cell(5, scale).child(
                            div()
                                .font_weight(gpui::FontWeight::SEMIBOLD)
                                .text_color(if row.overtime_minutes > 0 {
                                    colors.warning
                                } else {
                                    colors.on_surface_variant.opacity(0.6)
                                })
                                .child(row.overtime()),
                        ),
                    )
                    .child(
                        ledger_cell(6, scale).child(match row.project_name {
                            Some(project) => div()
                                .max_w(scale.px(180.0))
                                .px(scale.px(10.0))
                                .py(scale.px(4.0))
                                .flex()
                                .items_center()
                                .gap(scale.px(8.0))
                                .rounded(scale.px(12.0))
                                .bg(colors.surface_container)
                                .child(div().size(scale.px(8.0)).rounded_full().bg(project_color))
                                .child(div().min_w_0().truncate().child(project)),
                            None => empty_value(colors),
                        }),
                    )
                    .child(
                        ledger_cell(7, scale).child(match row.notes {
                            Some(notes) => div()
                                .min_w_0()
                                .truncate()
                                .text_color(colors.on_surface_variant)
                                .child(notes),
                            None => empty_value(colors),
                        }),
                    )
            }))
    }

    fn calendar(&mut self, cx: &mut Context<Self>) -> gpui::Div {
        let colors = self.data.colors;
        let scale = self.data.scale;
        let cells = self.calendar_cells();
        let last_cell = cells.len().saturating_sub(1);
        let first_bottom_cell = last_cell.saturating_sub(6);
        m3_card(colors)
            .border_0()
            .rounded(scale.px(16.0))
            .bg(colors.surface_container_low)
            .overflow_hidden()
            .child(
                div()
                    .h(scale.px(40.0))
                    .rounded_t(scale.px(16.0))
                    .grid()
                    .grid_cols(7)
                    .items_center()
                    .bg(colors.surface_container)
                    .m3_typography(TypographyRole::LabelMedium, scale)
                    .text_color(colors.on_surface_variant)
                    .children(
                        [
                            "Monday",
                            "Tuesday",
                            "Wednesday",
                            "Thursday",
                            "Friday",
                            "Saturday",
                            "Sunday",
                        ]
                        .map(|key| {
                            localized(self.data.language, key)
                                .chars()
                                .take(3)
                                .collect::<String>()
                                .to_uppercase()
                        }),
                    ),
            )
            .child(
                div()
                    .grid()
                    .grid_cols(7)
                    .gap(scale.px(1.0))
                    .bg(colors.grid_line)
                    .children(cells.into_iter().enumerate().map(|(index, cell)| {
                        let date = cell.date;
                        let date_for_key = date;
                        let date_for_click = date;
                        let selected = self.data.selected_date == Some(date);
                        div()
                            .id(("calendar-cell", index))
                            .tab_index(if cell.current_month { 0 } else { -1 })
                            .min_h(scale.px(110.0))
                            .when(index == first_bottom_cell, |cell| {
                                cell.rounded_bl(scale.px(16.0))
                            })
                            .when(index == last_cell, |cell| cell.rounded_br(scale.px(16.0)))
                            .p(scale.px(12.0))
                            .flex()
                            .flex_col()
                            .gap(scale.px(6.0))
                            .m3_typography(TypographyRole::BodyMedium, scale)
                            .bg(if selected {
                                colors
                                    .surface_container_low
                                    .blend(colors.primary_container.opacity(0.54))
                            } else if cell.current_month {
                                colors.surface_container_low
                            } else {
                                colors.surface_container
                            })
                            .when(cell.current_month, |item| {
                                item.cursor_pointer()
                                    .focus(move |style| {
                                        style.shadow(focus_shadow(colors.primary, scale))
                                    })
                                    .hover(move |style| {
                                        style.bg(m3_state_layer(
                                            colors.surface_container_low,
                                            colors.on_surface,
                                            HOVER_OPACITY,
                                        ))
                                    })
                                    .on_click(cx.listener(move |_, _, _, cx| {
                                        cx.emit(MonthViewEvent(date_for_click))
                                    }))
                                    .on_key_down(cx.listener(move |view, event, _, cx| {
                                        view.activate(date_for_key, event, cx)
                                    }))
                            })
                            .child(
                                div()
                                    .size(scale.px(24.0))
                                    .rounded_full()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .bg(if cell.is_today {
                                        colors.primary
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .text_color(if cell.is_today {
                                        colors.on_primary
                                    } else {
                                        colors.on_surface
                                    })
                                    .child(cell.day.to_string()),
                            )
                            .when(cell.is_missing, |item| {
                                item.child(
                                    div()
                                        .m3_typography(TypographyRole::LabelSmall, scale)
                                        .text_color(colors.warning)
                                        .child(localized(self.data.language, "Unlogged")),
                                )
                            })
                            .when_some(cell.holiday, |item, holiday| {
                                item.child(
                                    div()
                                        .rounded(scale.px(6.0))
                                        .p(scale.px(6.0))
                                        .bg(colors.warning_container)
                                        .text_color(colors.on_warning_container)
                                        .m3_typography(TypographyRole::LabelSmall, scale)
                                        .child(holiday),
                                )
                            })
                            .when_some(cell.entry, |item, entry| {
                                item.child(calendar_entry(entry, colors, self.data.language, scale))
                            })
                    })),
            )
    }

    fn rows(&self) -> Vec<LedgerRow> {
        let entries: BTreeMap<_, _> = self
            .data
            .entries
            .iter()
            .map(|entry| (entry.date, entry))
            .collect();
        dates_in_month(self.data.month.year, self.data.month.month)
            .into_iter()
            .map(|date| {
                let entry = entries.get(&date).copied();
                let status = entry.map_or(WorkEntryStatus::Incomplete, |entry| entry.status);
                let overtime = entry.map_or(0, |entry| {
                    split_overtime(
                        entry,
                        &self.data.settings.expected_hours,
                        &self.data.settings.overtime_compensation,
                        SwedishHolidayCalendar,
                    )
                    .1
                    .value()
                });
                let naive = date.as_naive_date();
                LedgerRow {
                    date,
                    day: naive.day(),
                    weekday: naive.format("%a").to_string(),
                    is_today: date == self.data.today,
                    is_missing: self.data.month_started
                        && date < self.data.today
                        && status == WorkEntryStatus::Incomplete
                        && is_scheduled_workday(
                            date,
                            &self.data.settings.expected_hours,
                            SwedishHolidayCalendar,
                        ),
                    holiday: SwedishHolidayCalendar.holiday_name(date),
                    status,
                    start_time: entry.and_then(|entry| entry.start_time),
                    end_time: entry.and_then(|entry| entry.end_time),
                    lunch_minutes: entry.map_or(0, |entry| entry.lunch_minutes.value()),
                    worked_minutes: entry.map_or(0, |entry| worked_minutes(entry).value()),
                    overtime_minutes: overtime,
                    project_name: entry.and_then(|entry| entry.project_name.clone()),
                    notes: entry.and_then(|entry| entry.notes.clone()),
                }
            })
            .collect()
    }

    fn calendar_cells(&self) -> Vec<CalendarCell> {
        let first = NaiveDate::from_ymd_opt(self.data.month.year, self.data.month.month, 1)
            .unwrap_or_else(|| unreachable!());
        let leading = i64::from(first.weekday().num_days_from_monday());
        let mut date = first - Duration::days(leading);
        let month_days = dates_in_month(self.data.month.year, self.data.month.month).len();
        let cell_count = (leading as usize + month_days).div_ceil(7) * 7;
        let entries: BTreeMap<_, _> = self
            .data
            .entries
            .iter()
            .map(|entry| (entry.date, entry.clone()))
            .collect();
        (0..cell_count)
            .map(|_| {
                let iso = IsoDate::new(date);
                let current_month = self.data.month.contains(iso);
                let entry = entries.get(&iso).cloned();
                let status = entry
                    .as_ref()
                    .map_or(WorkEntryStatus::Incomplete, |entry| entry.status);
                let cell = CalendarCell {
                    date: iso,
                    day: date.day(),
                    current_month,
                    is_today: iso == self.data.today,
                    is_missing: current_month
                        && self.data.month_started
                        && iso < self.data.today
                        && status == WorkEntryStatus::Incomplete
                        && is_scheduled_workday(
                            iso,
                            &self.data.settings.expected_hours,
                            SwedishHolidayCalendar,
                        ),
                    holiday: current_month
                        .then(|| SwedishHolidayCalendar.holiday_name(iso))
                        .flatten()
                        .map(str::to_owned),
                    entry,
                };
                date += Duration::days(1);
                cell
            })
            .collect()
    }
}

impl EventEmitter<MonthViewEvent> for MonthView {}

impl Render for MonthView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        match self.data.mode {
            MonthViewPreference::Ledger => self.ledger(cx),
            MonthViewPreference::Calendar => self.calendar(cx),
        }
    }
}

pub fn summary_banner(
    summary: &MonthlySummary,
    tax: &TaxEstimate,
    currency: &str,
    unstarted: bool,
    language: LanguagePreference,
    colors: M3ColorScheme,
    scale: UiScale,
) -> gpui::Div {
    let net = tax
        .estimated_net_pay
        .unwrap_or(summary.gross_salary)
        .decimal()
        .round_dp(0);
    let metrics = [
        (
            "schedule",
            localized(language, "Worked Time"),
            format!("{} / {}h", summary.worked_hours, summary.expected_hours),
            String::new(),
            colors.on_surface,
        ),
        (
            "more_time",
            localized(language, "Overtime & OB"),
            format!("{}h", summary.overtime_hours),
            format!("{}h OB", summary.ob_hours),
            colors.on_surface,
        ),
        (
            "balance",
            localized(language, "Time Balance"),
            format_minutes(summary.closing_balance_minutes.value()),
            if unstarted {
                localized(language, "Opening Balance")
            } else {
                format!(
                    "Δ {}",
                    format_minutes(summary.monthly_difference_minutes.value())
                )
            },
            if summary.closing_balance_minutes.value() > 0 {
                colors.success
            } else if summary.closing_balance_minutes.value() < 0 {
                colors.error
            } else {
                colors.on_surface
            },
        ),
        (
            "payments",
            localized(language, "Estimated Net Pay"),
            format!("{net} {currency}"),
            if tax.is_available {
                format!("Gross: {}", summary.gross_salary.decimal().round_dp(0))
            } else {
                "Gross".to_owned()
            },
            colors.on_surface,
        ),
    ];
    div()
        .grid()
        .grid_cols(4)
        .gap(scale.px(24.0))
        .py(scale.px(16.0))
        .px(scale.px(20.0))
        .rounded(scale.px(16.0))
        .bg(colors.surface_container_low)
        .children(
            metrics
                .into_iter()
                .map(|(icon, label, value, qualifier, value_color)| {
                    div()
                        .flex()
                        .items_center()
                        .gap(scale.px(12.0))
                        .child(
                            div()
                                .size(scale.px(36.0))
                                .rounded(scale.px(12.0))
                                .flex()
                                .items_center()
                                .justify_center()
                                .bg(colors.primary_container)
                                .child(m3_icon_colored(
                                    icon,
                                    20.0 * scale.factor(),
                                    colors.on_primary_container,
                                )),
                        )
                        .child(
                            div()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .child(
                                    div()
                                        .m3_typography(TypographyRole::LabelSmall, scale)
                                        .text_color(colors.on_surface_variant)
                                        .child(label),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .items_baseline()
                                        .gap(scale.px(8.0))
                                        .child(
                                            div()
                                                .m3_typography(TypographyRole::TitleMedium, scale)
                                                .text_color(value_color)
                                                .child(value),
                                        )
                                        .child(
                                            div()
                                                .m3_typography(TypographyRole::BodySmall, scale)
                                                .text_color(colors.on_surface_variant)
                                                .child(qualifier),
                                        ),
                                ),
                        )
                }),
        )
}

struct LedgerRow {
    date: IsoDate,
    day: u32,
    weekday: String,
    is_today: bool,
    is_missing: bool,
    holiday: Option<&'static str>,
    status: WorkEntryStatus,
    start_time: Option<dagsverk_core::models::ClockTime>,
    end_time: Option<dagsverk_core::models::ClockTime>,
    lunch_minutes: i64,
    worked_minutes: i64,
    overtime_minutes: i64,
    project_name: Option<String>,
    notes: Option<String>,
}

impl LedgerRow {
    fn status_label(&self, language: LanguagePreference) -> String {
        if self.holiday.is_some() {
            localized(language, "Public Holiday")
        } else {
            match self.status {
                WorkEntryStatus::Worked => localized(language, "Worked"),
                WorkEntryStatus::Off => localized(language, "Day Off"),
                WorkEntryStatus::Incomplete if self.is_missing => localized(language, "Unlogged"),
                WorkEntryStatus::Incomplete => "-".to_owned(),
            }
        }
    }

    fn interval(&self) -> String {
        match (self.start_time, self.end_time, self.status) {
            (Some(start), Some(end), WorkEntryStatus::Worked) => format!("{start}-{end}"),
            _ => "-".to_owned(),
        }
    }

    fn lunch(&self) -> String {
        if self.lunch_minutes > 0 {
            format!("{}m", self.lunch_minutes)
        } else {
            "-".to_owned()
        }
    }

    fn hours(&self) -> String {
        format_hours(self.worked_minutes, false)
    }

    fn overtime(&self) -> String {
        format_hours(self.overtime_minutes, true)
    }

    fn status_icon(&self) -> Option<&'static str> {
        if self.holiday.is_some() {
            Some("celebration")
        } else {
            match self.status {
                WorkEntryStatus::Worked => Some("check"),
                WorkEntryStatus::Off => Some("beach_access"),
                WorkEntryStatus::Incomplete => None,
            }
        }
    }
}

struct CalendarCell {
    date: IsoDate,
    day: u32,
    current_month: bool,
    is_today: bool,
    is_missing: bool,
    holiday: Option<String>,
    entry: Option<WorkEntry>,
}

fn calendar_entry(
    entry: WorkEntry,
    colors: M3ColorScheme,
    language: LanguagePreference,
    scale: UiScale,
) -> gpui::Div {
    let (background, text) = match entry.status {
        WorkEntryStatus::Worked => (colors.primary_container, colors.on_primary_container),
        WorkEntryStatus::Off => (colors.surface_container_high, colors.on_surface_variant),
        WorkEntryStatus::Incomplete => (colors.surface_container, colors.on_surface_variant),
    };
    let label = match entry.status {
        WorkEntryStatus::Worked => match (entry.start_time, entry.end_time) {
            (Some(start), Some(end)) => format!("{start}-{end}"),
            _ => localized(language, "Worked"),
        },
        WorkEntryStatus::Off => entry
            .notes
            .unwrap_or_else(|| localized(language, "Day Off")),
        WorkEntryStatus::Incomplete => localized(language, "Unlogged"),
    };
    div()
        .min_h(scale.px(32.0))
        .p(scale.px(6.0))
        .rounded(scale.px(6.0))
        .bg(background)
        .text_color(text)
        .m3_typography(TypographyRole::LabelSmall, scale)
        .child(label)
}

fn localized(language: LanguagePreference, key: &str) -> String {
    translate(language, key).into_owned()
}

fn format_hours(minutes: i64, plus: bool) -> String {
    if minutes <= 0 {
        "-".to_owned()
    } else {
        let hundredths = (minutes * 100 + 30) / 60;
        let value = format!("{}.{:02}h", hundredths / 100, hundredths % 100);
        if plus { format!("+{value}") } else { value }
    }
}

fn format_minutes(minutes: i64) -> String {
    let sign = if minutes < 0 { "-" } else { "" };
    let absolute = minutes.abs();
    format!("{sign}{}:{:02}", absolute / 60, absolute % 60)
}

fn parse_hex(value: &str) -> Option<Hsla> {
    let value = value.strip_prefix('#').unwrap_or(value);
    (value.len() == 6)
        .then(|| u32::from_str_radix(value, 16).ok())
        .flatten()
        .map(|value| rgb(value).into())
}

fn ledger_cell(index: usize, scale: UiScale) -> gpui::Div {
    div()
        .w(relative(LEDGER_COLUMN_RATIOS[index]))
        .min_w_0()
        .px(scale.px(16.0))
}

fn empty_value(colors: M3ColorScheme) -> gpui::Div {
    div()
        .text_color(colors.on_surface_variant.opacity(0.6))
        .child("-")
}

fn numeric_value(value: String, colors: M3ColorScheme) -> gpui::Div {
    if value == "-" {
        empty_value(colors)
    } else {
        div().font_weight(gpui::FontWeight::MEDIUM).child(value)
    }
}

fn focus_shadow(color: Hsla, scale: UiScale) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: color.opacity(FOCUS_OPACITY),
        offset: point(px(0.0), px(0.0)),
        blur_radius: px(0.0),
        spread_radius: scale.px(3.0),
    }]
}

#[cfg(test)]
mod tests {
    use dagsverk_core::models::{MonthViewPreference, YearMonth, default_settings};

    use crate::m3::M3ColorScheme;

    use super::{
        LEDGER_COLUMN_RATIOS, MonthView, MonthViewData, format_hours, format_minutes, parse_hex,
    };

    #[test]
    fn display_helpers_cover_empty_negative_and_project_colors() {
        assert_eq!(format_hours(0, false), "-");
        assert_eq!(format_hours(90, true), "+1.50h");
        assert_eq!(format_minutes(-75), "-1:15");
        assert!(parse_hex("#5F875F").is_some());
        assert!(parse_hex("bad").is_none());
    }

    #[test]
    fn ledger_and_calendar_cover_the_complete_month() {
        let view = MonthView::new(MonthViewData {
            month: YearMonth::new(2026, 8).expect("month"),
            entries: Vec::new(),
            settings: default_settings(),
            projects: Vec::new(),
            today: "2026-08-18".parse().expect("date"),
            selected_date: None,
            month_started: true,
            mode: MonthViewPreference::Ledger,
            language: dagsverk_core::models::LanguagePreference::English,
            colors: M3ColorScheme::light(),
            scale: crate::m3::UiScale::default(),
        });
        assert_eq!(view.rows().len(), 31);
        let cells = view.calendar_cells();
        assert_eq!(cells.len() % 7, 0);
        assert_eq!(cells.iter().filter(|cell| cell.current_month).count(), 31);
    }

    #[test]
    fn ledger_columns_preserve_the_measured_electron_proportions() {
        let total: f32 = LEDGER_COLUMN_RATIOS.iter().sum();
        assert!((total - 1.0).abs() < f32::EPSILON);
        assert!(
            LEDGER_COLUMN_RATIOS
                .windows(2)
                .any(|pair| pair[0] != pair[1])
        );
    }
}
