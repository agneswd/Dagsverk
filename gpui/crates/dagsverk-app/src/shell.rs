use chrono::Duration;
use dagsverk_core::{
    calculations::{calculate_daily_pay, normalize_time},
    models::{
        CurrencyPreference, Minutes, MonthViewPreference, Project, ProjectId, WorkEntryStatus,
    },
};
use dagsverk_ui::{
    m3::{M3ColorScheme, ResolvedTheme as UiTheme, m3_icon},
    text_input::TextInput,
    views::timesheet::{MonthView, MonthViewData, MonthViewEvent, summary_banner},
};
use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, KeyBinding, Render, Window, actions,
    div, prelude::*, px,
};

use crate::state::{AppModel, ResolvedTheme, Route};

actions!(
    dagsverk,
    [
        ShowLedger,
        ShowCalendar,
        ShowSettings,
        PreviousMonth,
        NextMonth,
        StartCatchUp,
        SaveActive,
        CloseSurface
    ]
);

pub struct AppShell {
    model: AppModel,
    month_view: Entity<MonthView>,
    start_input: Entity<TextInput>,
    end_input: Entity<TextInput>,
    notes_input: Entity<TextInput>,
    scheduled_input: Entity<TextInput>,
    project_name_input: Entity<TextInput>,
    project_color_input: Entity<TextInput>,
    focus: FocusHandle,
    sidebar_collapsed: bool,
    confirm_reset: bool,
    confirm_project_delete: Option<ProjectId>,
    notice: Option<String>,
}

impl AppShell {
    pub fn register_key_bindings(cx: &mut App) {
        cx.bind_keys([
            KeyBinding::new("ctrl-1", ShowLedger, None),
            KeyBinding::new("ctrl-2", ShowCalendar, None),
            KeyBinding::new("ctrl-,", ShowSettings, None),
            KeyBinding::new("pageup", PreviousMonth, None),
            KeyBinding::new("pagedown", NextMonth, None),
            KeyBinding::new("ctrl-m", StartCatchUp, None),
            KeyBinding::new("ctrl-s", SaveActive, None),
            KeyBinding::new("escape", CloseSurface, None),
        ]);
    }

    pub fn new(model: AppModel, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.set_window_title("Dagsverk GPUI Preview");
        let month_view = cx.new(|_| MonthView::new(month_view_data(&model)));
        let start_input = cx.new(|cx| TextInput::new(cx, "Start time"));
        let end_input = cx.new(|cx| TextInput::new(cx, "End time"));
        let notes_input = cx.new(|cx| TextInput::new(cx, "Notes"));
        let scheduled_input = cx.new(|cx| TextInput::new(cx, "Scheduled hours"));
        let project_name_input = cx.new(|cx| TextInput::new(cx, "Project name"));
        let project_color_input = cx.new(|cx| TextInput::new(cx, "#5F875F"));
        project_color_input.update(cx, |input, cx| input.set_text("#5F875F", cx));
        let focus = cx.focus_handle();
        window.focus(&focus);
        cx.subscribe(&month_view, |shell, _, event: &MonthViewEvent, cx| {
            shell.model.open_editor(event.0);
            shell.sync_editor_inputs(cx);
            shell.refresh_month_view(cx);
        })
        .detach();
        Self {
            model,
            month_view,
            start_input,
            end_input,
            notes_input,
            scheduled_input,
            project_name_input,
            project_color_input,
            focus,
            sidebar_collapsed: false,
            confirm_reset: false,
            confirm_project_delete: None,
            notice: None,
        }
    }

    fn colors(&self) -> M3ColorScheme {
        M3ColorScheme::resolve(match self.model.resolved_theme {
            ResolvedTheme::Light => UiTheme::Light,
            ResolvedTheme::Dark => UiTheme::Dark,
        })
    }

    fn set_route(&mut self, route: Route, cx: &mut Context<Self>) {
        self.model.route = route;
        self.model.close_catch_up();
        cx.notify();
    }

    fn set_view(&mut self, view: MonthViewPreference, cx: &mut Context<Self>) {
        if let Err(error) = self.model.set_view(view) {
            self.model.transient_error = Some(error.to_string());
        } else {
            self.model.route = Route::Timesheet;
        }
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn refresh_month_view(&mut self, cx: &mut Context<Self>) {
        let data = month_view_data(&self.model);
        self.month_view
            .update(cx, |month_view, cx| month_view.set_data(data, cx));
    }

    fn sync_editor_inputs(&mut self, cx: &mut Context<Self>) {
        let Some(draft) = self.model.editor.draft.as_ref() else {
            return;
        };
        let colors = self.colors();
        let start = draft
            .start_time
            .unwrap_or(self.model.settings.default_start_time)
            .to_string();
        let end = draft
            .end_time
            .unwrap_or(self.model.settings.default_end_time)
            .to_string();
        self.start_input.update(cx, |input, cx| {
            input.set_text(start, cx);
            input.set_colors(colors, cx);
        });
        self.end_input.update(cx, |input, cx| {
            input.set_text(end, cx);
            input.set_colors(colors, cx);
        });
        let notes = draft.notes.clone().unwrap_or_default();
        self.notes_input.update(cx, |input, cx| {
            input.set_text(notes, cx);
            input.set_colors(colors, cx);
        });
        let scheduled = draft.scheduled_minutes_override.map_or_else(
            || {
                self.model
                    .settings
                    .expected_hours
                    .hours_per_workday
                    .to_string()
            },
            |minutes| format_hours_input(minutes.value()),
        );
        self.scheduled_input
            .update(cx, |input, cx| input.set_text(scheduled, cx));
    }

    fn save_editor(&mut self, cx: &mut Context<Self>) {
        let Some(mut draft) = self.model.editor.draft.clone() else {
            return;
        };
        if draft.status == WorkEntryStatus::Worked {
            let start = normalize_time(self.start_input.read(cx).text());
            let end = normalize_time(self.end_input.read(cx).text());
            let (Some(start), Some(end)) = (start, end) else {
                self.model.editor.validation_error =
                    Some("Enter valid start and end times.".to_owned());
                cx.notify();
                return;
            };
            draft.start_time = Some(start);
            draft.end_time = Some(end);
        } else if draft.status == WorkEntryStatus::Off {
            draft.start_time = None;
            draft.end_time = None;
            draft.lunch_minutes = Minutes::ZERO;
            draft.project_name = None;
        }
        let notes = self.notes_input.read(cx).text().trim().to_owned();
        draft.notes = (!notes.is_empty()).then_some(notes);
        if draft.scheduled_minutes_override.is_some() {
            match parse_scheduled_minutes(self.scheduled_input.read(cx).text()) {
                Ok(minutes) => draft.scheduled_minutes_override = Some(minutes),
                Err(message) => {
                    self.model.editor.validation_error = Some(message.to_owned());
                    cx.notify();
                    return;
                }
            }
        }
        match self.model.save_entry(draft) {
            Ok(()) => {
                if self.model.catch_up.is_some() {
                    self.model.move_catch_up(1);
                    self.sync_editor_inputs(cx);
                } else {
                    self.model.close_editor();
                }
                self.refresh_month_view(cx);
            }
            Err(error) => self.model.editor.validation_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn copy_editor_entry(
        &mut self,
        source: Option<dagsverk_core::models::WorkEntry>,
        cx: &mut Context<Self>,
    ) {
        let Some(source) = source.filter(|entry| entry.status == WorkEntryStatus::Worked) else {
            self.model.editor.validation_error =
                Some("No completed day is available to copy.".to_owned());
            cx.notify();
            return;
        };
        if let Some(draft) = self.model.editor.draft.as_mut() {
            draft.status = WorkEntryStatus::Worked;
            draft.start_time = source.start_time;
            draft.end_time = source.end_time;
            draft.lunch_minutes = source.lunch_minutes;
            draft.project_name = source.project_name;
            draft.scheduled_minutes_override = source.scheduled_minutes_override;
        }
        self.model.editor.validation_error = None;
        self.sync_editor_inputs(cx);
        cx.notify();
    }

    fn copy_previous(&mut self, cx: &mut Context<Self>) {
        let Some(date) = self.model.selected_date else {
            return;
        };
        let source = self
            .model
            .entries
            .iter()
            .filter(|entry| entry.date < date && entry.status == WorkEntryStatus::Worked)
            .max_by_key(|entry| entry.date)
            .cloned();
        self.copy_editor_entry(source, cx);
    }

    fn copy_last_week(&mut self, cx: &mut Context<Self>) {
        let Some(date) = self.model.selected_date else {
            return;
        };
        let previous =
            dagsverk_core::models::IsoDate::new(date.as_naive_date() - Duration::days(7));
        let source = self
            .model
            .entries
            .iter()
            .find(|entry| entry.date == previous)
            .cloned();
        self.copy_editor_entry(source, cx);
    }

    fn load_month(&mut self, key: crate::state::LoadKey, cx: &mut Context<Self>) {
        match self.model.load_for_key(&key) {
            Ok(data) => {
                self.model.apply_load(&key, data);
                self.model.transient_error = None;
                self.refresh_month_view(cx);
            }
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn previous_month(&mut self, _: &PreviousMonth, _: &mut Window, cx: &mut Context<Self>) {
        let key = self.model.previous_month();
        self.load_month(key, cx);
    }

    fn next_month(&mut self, _: &NextMonth, _: &mut Window, cx: &mut Context<Self>) {
        let key = self.model.next_month();
        self.load_month(key, cx);
    }

    fn start_catch_up(&mut self, _: &StartCatchUp, _: &mut Window, cx: &mut Context<Self>) {
        self.model.start_catch_up();
        cx.notify();
    }

    fn save_active(&mut self, _: &SaveActive, _: &mut Window, cx: &mut Context<Self>) {
        self.save_editor(cx);
    }

    fn close_surface(&mut self, _: &CloseSurface, _: &mut Window, cx: &mut Context<Self>) {
        if self.confirm_reset {
            self.confirm_reset = false;
            cx.notify();
            return;
        }
        if self.confirm_project_delete.take().is_some() {
            cx.notify();
            return;
        }
        self.model.close_catch_up();
        cx.notify();
    }

    fn fill_month(&mut self, cx: &mut Context<Self>) {
        match self.model.fill_normal_workdays() {
            Ok(count) => self.notice = Some(format!("Added {count} workdays.")),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn copy_month(&mut self, cx: &mut Context<Self>) {
        let count = self.model.copy_month();
        self.notice = Some(format!("Copied {count} entries."));
        cx.notify();
    }

    fn paste_month(&mut self, cx: &mut Context<Self>) {
        match self.model.paste_month() {
            Ok(count) => self.notice = Some(format!("Pasted {count} entries.")),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn reset_month(&mut self, cx: &mut Context<Self>) {
        match self.model.reset_month() {
            Ok(()) => self.notice = Some("Month reset.".to_owned()),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.confirm_reset = false;
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn add_project(&mut self, cx: &mut Context<Self>) {
        let name = self.project_name_input.read(cx).text().trim().to_owned();
        let color = self.project_color_input.read(cx).text().trim().to_owned();
        if name.is_empty() || !is_hex_color(&color) {
            self.model.transient_error =
                Some("Enter a project name and a six-digit hex color.".to_owned());
            cx.notify();
            return;
        }
        let id = match ProjectId::new(uuid::Uuid::new_v4().to_string()) {
            Ok(id) => id,
            Err(error) => {
                self.model.transient_error = Some(error.to_string());
                cx.notify();
                return;
            }
        };
        let project = Project {
            workspace_id: Some(self.model.active_workspace_id.clone()),
            id,
            name,
            color: Some(color),
            is_active: true,
            is_default: self.model.projects.is_empty(),
        };
        match self.model.save_project(project) {
            Ok(()) => {
                self.project_name_input
                    .update(cx, |input, cx| input.set_text("", cx));
                self.notice = Some("Project added.".to_owned());
            }
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn set_default_project(&mut self, id: &ProjectId, cx: &mut Context<Self>) {
        match self.model.set_default_project(id) {
            Ok(()) => self.notice = Some("Default project changed.".to_owned()),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn toggle_project(&mut self, id: &ProjectId, cx: &mut Context<Self>) {
        let Some(mut project) = self
            .model
            .projects
            .iter()
            .find(|project| &project.id == id)
            .cloned()
        else {
            return;
        };
        project.is_active = !project.is_active;
        match self.model.save_project(project) {
            Ok(()) => self.notice = Some("Project updated.".to_owned()),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn update_project_color(&mut self, id: &ProjectId, cx: &mut Context<Self>) {
        let color = self.project_color_input.read(cx).text().trim().to_owned();
        if !is_hex_color(&color) {
            self.model.transient_error = Some("Enter a six-digit hex color.".to_owned());
            cx.notify();
            return;
        }
        let Some(mut project) = self
            .model
            .projects
            .iter()
            .find(|project| &project.id == id)
            .cloned()
        else {
            return;
        };
        project.color = Some(color);
        match self.model.save_project(project) {
            Ok(()) => self.notice = Some("Project color updated.".to_owned()),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn delete_project(&mut self, id: &ProjectId, cx: &mut Context<Self>) {
        match self.model.delete_project(id) {
            Ok(()) => self.notice = Some("Project deleted.".to_owned()),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.confirm_project_delete = None;
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn navigation_item(
        &self,
        id: &'static str,
        icon: &'static str,
        label: &'static str,
        route: Route,
        colors: M3ColorScheme,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let selected = self.model.route == route;
        div()
            .id(id)
            .h(px(56.0))
            .mx(px(12.0))
            .px(px(16.0))
            .flex()
            .items_center()
            .gap(px(16.0))
            .rounded(px(28.0))
            .cursor_pointer()
            .bg(if selected {
                colors.secondary_container
            } else {
                colors.surface_container_low
            })
            .child(m3_icon(icon, 24.0, colors))
            .when(!self.sidebar_collapsed, |item| item.child(label))
            .on_click(cx.listener(move |shell, _, _, cx| shell.set_route(route, cx)))
    }

    fn route_content(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        match self.model.route {
            Route::Timesheet => self.timesheet(colors),
            Route::Projects => self.projects_page(colors, cx),
            Route::Settings => self.placeholder_page(
                "Settings",
                "Workspace and application settings are connected to the state model.",
                colors,
            ),
            Route::DataBackups => self.placeholder_page(
                "Data & backups",
                "Backup, restore, and import services are available in dagsverk-data.",
                colors,
            ),
        }
    }

    fn placeholder_page(
        &self,
        title: impl Into<gpui::SharedString>,
        detail: impl Into<gpui::SharedString>,
        colors: M3ColorScheme,
    ) -> gpui::Div {
        div()
            .p(px(32.0))
            .flex()
            .flex_col()
            .gap(px(12.0))
            .child(div().text_size(px(24.0)).child(title.into()))
            .child(
                div()
                    .text_color(colors.on_surface_variant)
                    .child(detail.into()),
            )
    }

    fn projects_page(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let projects = self.model.projects.clone();
        div()
            .max_w(px(1088.0))
            .mx_auto()
            .p(px(32.0))
            .flex()
            .gap(px(24.0))
            .child(
                div()
                    .w(px(340.0))
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .rounded(px(16.0))
                    .bg(colors.surface_container_low)
                    .child(div().text_size(px(20.0)).child("Add project"))
                    .child("Name")
                    .child(self.project_name_input.clone())
                    .child("Color")
                    .child(self.project_color_input.clone())
                    .child(
                        div()
                            .id("add-project")
                            .h(px(40.0))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(px(20.0))
                            .cursor_pointer()
                            .bg(colors.primary)
                            .text_color(colors.on_primary)
                            .child("Add project")
                            .on_click(cx.listener(|shell, _, _, cx| shell.add_project(cx))),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(12.0))
                    .rounded(px(16.0))
                    .bg(colors.surface_container_low)
                    .child(
                        div()
                            .text_size(px(20.0))
                            .child(format!("Projects ({})", projects.len())),
                    )
                    .children(projects.into_iter().enumerate().map(|(index, project)| {
                        let default_id = project.id.clone();
                        let toggle_id = project.id.clone();
                        let color_id = project.id.clone();
                        let delete_id = project.id.clone();
                        let color = project
                            .color
                            .as_deref()
                            .and_then(color_from_hex)
                            .unwrap_or(colors.primary);
                        div()
                            .h(px(64.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .rounded(px(12.0))
                            .bg(colors.surface_container)
                            .child(div().size(px(12.0)).rounded_full().bg(color))
                            .child(
                                div().flex_1().flex().flex_col().child(project.name).child(
                                    div()
                                        .text_size(px(12.0))
                                        .text_color(colors.on_surface_variant)
                                        .child(if project.is_active {
                                            "Active"
                                        } else {
                                            "Archived"
                                        }),
                                ),
                            )
                            .when(project.is_default, |row| {
                                row.child(
                                    div()
                                        .px(px(10.0))
                                        .py(px(4.0))
                                        .rounded(px(12.0))
                                        .bg(colors.primary_container)
                                        .child("Default"),
                                )
                            })
                            .when(!project.is_default, |row| {
                                row.child(
                                    div()
                                        .id(("default-project", index))
                                        .cursor_pointer()
                                        .child("Set default")
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            shell.set_default_project(&default_id, cx)
                                        })),
                                )
                            })
                            .child(
                                div()
                                    .id(("color-project", index))
                                    .cursor_pointer()
                                    .child("Apply color")
                                    .on_click(cx.listener(move |shell, _, _, cx| {
                                        shell.update_project_color(&color_id, cx)
                                    })),
                            )
                            .child(
                                div()
                                    .id(("toggle-project", index))
                                    .cursor_pointer()
                                    .child(if project.is_active {
                                        "Archive"
                                    } else {
                                        "Unarchive"
                                    })
                                    .on_click(cx.listener(move |shell, _, _, cx| {
                                        shell.toggle_project(&toggle_id, cx)
                                    })),
                            )
                            .when(!project.is_default, |row| {
                                row.child(
                                    div()
                                        .id(("delete-project", index))
                                        .cursor_pointer()
                                        .text_color(colors.error)
                                        .child("Delete")
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            shell.confirm_project_delete = Some(delete_id.clone());
                                            cx.notify();
                                        })),
                                )
                            })
                    })),
            )
    }

    fn timesheet(&self, colors: M3ColorScheme) -> gpui::Div {
        let summary = self.model.summary();
        let tax = self.model.tax_estimate();
        let currency = match self.model.settings.currency_preference {
            CurrencyPreference::Sek => "SEK",
            CurrencyPreference::Eur => "EUR",
            CurrencyPreference::Usd => "USD",
            CurrencyPreference::Gbp => "GBP",
            CurrencyPreference::Nok => "NOK",
            CurrencyPreference::Dkk => "DKK",
        };
        div()
            .p(px(24.0))
            .flex()
            .flex_col()
            .gap(px(16.0))
            .child(summary_banner(
                &summary,
                &tax,
                currency,
                self.model.is_month_unstarted(),
                colors,
            ))
            .child(self.month_view.clone())
    }

    fn editor_panel(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let Some(draft) = self.model.editor.draft.as_ref() else {
            return div();
        };
        let date = draft.date;
        let status = draft.status;
        let lunch = draft.lunch_minutes.value();
        let scheduled_override = draft.scheduled_minutes_override.is_some();
        let projects = self.model.projects.clone();
        let daily_pay = calculate_daily_pay(
            draft,
            &self.model.settings.expected_hours,
            &self.model.settings.salary,
            &self.model.settings.overtime_compensation,
            dagsverk_core::holidays::SwedishHolidayCalendar,
        );
        let error = self.model.editor.validation_error.clone();
        div()
            .w(px(400.0))
            .h_full()
            .flex_none()
            .flex()
            .flex_col()
            .border_l_1()
            .border_color(colors.outline_variant)
            .bg(colors.surface_container_lowest)
            .child(
                div()
                    .h(px(64.0))
                    .px(px(24.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_b_1()
                    .border_color(colors.grid_line)
                    .child(div().text_size(px(18.0)).child(date.to_string()))
                    .child(
                        div()
                            .id("close-editor")
                            .cursor_pointer()
                            .child(m3_icon("close", 24.0, colors))
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.model.close_catch_up();
                                shell.refresh_month_view(cx);
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .id("editor-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(18.0))
                    .child("Status")
                    .child(
                        div()
                            .flex()
                            .rounded(px(20.0))
                            .border_1()
                            .border_color(colors.outline_variant)
                            .children(
                                [
                                    ("status-worked", "Worked", WorkEntryStatus::Worked),
                                    ("status-off", "Day Off", WorkEntryStatus::Off),
                                    ("status-incomplete", "Unlogged", WorkEntryStatus::Incomplete),
                                ]
                                .into_iter()
                                .map(|(id, label, value)| {
                                    div()
                                        .id(id)
                                        .h(px(40.0))
                                        .px(px(12.0))
                                        .flex_1()
                                        .flex()
                                        .items_center()
                                        .justify_center()
                                        .cursor_pointer()
                                        .rounded(px(18.0))
                                        .bg(if status == value {
                                            colors.secondary_container
                                        } else {
                                            colors.surface_container_lowest
                                        })
                                        .child(label)
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            if let Some(draft) = shell.model.editor.draft.as_mut() {
                                                draft.status = value;
                                            }
                                            cx.notify();
                                        }))
                                }),
                            ),
                    )
                    .child("Reuse")
                    .child(
                        div().flex().gap(px(8.0)).children(
                            ["Normal day", "Copy previous", "Copy last week"]
                                .into_iter()
                                .enumerate()
                                .map(|(index, label)| {
                                    div()
                                        .id(("reuse", index))
                                        .h(px(36.0))
                                        .px(px(10.0))
                                        .flex()
                                        .items_center()
                                        .rounded(px(18.0))
                                        .border_1()
                                        .border_color(colors.outline_variant)
                                        .cursor_pointer()
                                        .text_size(px(12.0))
                                        .child(label)
                                        .on_click(cx.listener(move |shell, _, _, cx| match index {
                                            0 => {
                                                if let Some(draft) =
                                                    shell.model.editor.draft.as_mut()
                                                {
                                                    draft.status = WorkEntryStatus::Worked;
                                                    draft.start_time = Some(
                                                        shell.model.settings.default_start_time,
                                                    );
                                                    draft.end_time =
                                                        Some(shell.model.settings.default_end_time);
                                                    draft.lunch_minutes =
                                                        shell.model.settings.default_lunch_minutes;
                                                    draft.project_name = Some(
                                                        shell
                                                            .model
                                                            .settings
                                                            .default_project
                                                            .clone(),
                                                    );
                                                    draft.scheduled_minutes_override = None;
                                                }
                                                shell.sync_editor_inputs(cx);
                                                cx.notify();
                                            }
                                            1 => shell.copy_previous(cx),
                                            _ => shell.copy_last_week(cx),
                                        }))
                                }),
                        ),
                    )
                    .when(status == WorkEntryStatus::Worked, |panel| {
                        panel
                            .child("Presets")
                            .child(
                                div().flex().gap(px(8.0)).children(
                                    [
                                        ("08:00-16:30", "08:00", "16:30"),
                                        ("08:30-17:00", "08:30", "17:00"),
                                        ("09:00-17:30", "09:00", "17:30"),
                                    ]
                                    .into_iter()
                                    .enumerate()
                                    .map(
                                        |(index, (label, start, end))| {
                                            div()
                                                .id(("preset", index))
                                                .h(px(36.0))
                                                .px(px(10.0))
                                                .flex()
                                                .items_center()
                                                .rounded(px(18.0))
                                                .border_1()
                                                .border_color(colors.outline_variant)
                                                .cursor_pointer()
                                                .text_size(px(12.0))
                                                .child(label)
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    shell.start_input.update(cx, |input, cx| {
                                                        input.set_text(start, cx)
                                                    });
                                                    shell.end_input.update(cx, |input, cx| {
                                                        input.set_text(end, cx)
                                                    });
                                                    if let Some(draft) =
                                                        shell.model.editor.draft.as_mut()
                                                    {
                                                        draft.status = WorkEntryStatus::Worked;
                                                        draft.lunch_minutes = Minutes::new(30);
                                                    }
                                                    cx.notify();
                                                }))
                                        },
                                    ),
                                ),
                            )
                            .child("Start")
                            .child(self.start_input.clone())
                            .child("End")
                            .child(self.end_input.clone())
                            .child("Lunch")
                            .child(div().flex().gap(px(8.0)).children(
                                [0_i64, 30, 45, 60].into_iter().map(|minutes| {
                                    div()
                                        .id(("lunch", minutes as usize))
                                        .h(px(36.0))
                                        .px(px(14.0))
                                        .flex()
                                        .items_center()
                                        .rounded(px(18.0))
                                        .cursor_pointer()
                                        .bg(if lunch == minutes {
                                            colors.secondary_container
                                        } else {
                                            colors.surface_container
                                        })
                                        .child(format!("{minutes}m"))
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            if let Some(draft) = shell.model.editor.draft.as_mut() {
                                                draft.lunch_minutes = Minutes::new(minutes);
                                            }
                                            cx.notify();
                                        }))
                                }),
                            ))
                            .child(
                                div()
                                    .id("scheduled-override")
                                    .h(px(40.0))
                                    .flex()
                                    .items_center()
                                    .justify_between()
                                    .cursor_pointer()
                                    .child("Scheduled-hours override")
                                    .child(if scheduled_override { "On" } else { "Off" })
                                    .on_click(cx.listener(|shell, _, _, cx| {
                                        if let Some(draft) = shell.model.editor.draft.as_mut() {
                                            draft.scheduled_minutes_override =
                                                if draft.scheduled_minutes_override.is_some() {
                                                    None
                                                } else {
                                                    Some(Minutes::ZERO)
                                                };
                                        }
                                        cx.notify();
                                    })),
                            )
                            .when(scheduled_override, |panel| {
                                panel.child(self.scheduled_input.clone())
                            })
                            .child("Project")
                            .child(
                                div().flex().flex_wrap().gap(px(8.0)).children(
                                    projects
                                        .into_iter()
                                        .filter(|project| project.is_active)
                                        .enumerate()
                                        .map(|(index, project)| {
                                            let project_name = project.name.clone();
                                            let selected =
                                                draft.project_name.as_ref() == Some(&project.name);
                                            div()
                                                .id(("project", index))
                                                .h(px(34.0))
                                                .px(px(12.0))
                                                .flex()
                                                .items_center()
                                                .rounded(px(17.0))
                                                .cursor_pointer()
                                                .bg(if selected {
                                                    colors.secondary_container
                                                } else {
                                                    colors.surface_container
                                                })
                                                .child(project.name)
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    if let Some(draft) =
                                                        shell.model.editor.draft.as_mut()
                                                    {
                                                        draft.project_name =
                                                            Some(project_name.clone());
                                                    }
                                                    cx.notify();
                                                }))
                                        }),
                                ),
                            )
                    })
                    .when(status == WorkEntryStatus::Off, |panel| {
                        panel.child("Reason").child(
                            div().flex().flex_wrap().gap(px(8.0)).children(
                                [
                                    "Vacation",
                                    "Sick leave",
                                    "Care of child",
                                    "Leave of absence",
                                    "Parental leave",
                                    "Public holiday",
                                ]
                                .into_iter()
                                .enumerate()
                                .map(|(index, reason)| {
                                    div()
                                        .id(("off-reason", index))
                                        .h(px(34.0))
                                        .px(px(12.0))
                                        .flex()
                                        .items_center()
                                        .rounded(px(17.0))
                                        .cursor_pointer()
                                        .bg(colors.surface_container)
                                        .child(reason)
                                        .on_click(cx.listener(move |shell, _, _, cx| {
                                            shell
                                                .notes_input
                                                .update(cx, |input, cx| input.set_text(reason, cx));
                                            cx.notify();
                                        }))
                                }),
                            ),
                        )
                    })
                    .child(
                        div()
                            .p(px(14.0))
                            .rounded(px(12.0))
                            .bg(colors.surface_container)
                            .child(format!(
                                "Estimated day pay: {}",
                                daily_pay.total.decimal().round_dp(2)
                            )),
                    )
                    .child("Notes")
                    .child(self.notes_input.clone())
                    .when_some(error, |panel, error| {
                        panel.child(div().text_color(colors.error).child(error))
                    }),
            )
            .child(
                div()
                    .h(px(72.0))
                    .px(px(24.0))
                    .flex()
                    .items_center()
                    .justify_between()
                    .border_t_1()
                    .border_color(colors.grid_line)
                    .child(
                        div()
                            .id("reset-entry")
                            .cursor_pointer()
                            .text_color(colors.error)
                            .child("Reset")
                            .on_click(cx.listener(move |shell, _, _, cx| {
                                if let Err(error) = shell.model.delete_entry(date) {
                                    shell.model.editor.validation_error = Some(error.to_string());
                                }
                                shell.refresh_month_view(cx);
                                cx.notify();
                            })),
                    )
                    .when(self.model.catch_up.is_some(), |footer| {
                        footer
                            .child(
                                div()
                                    .id("catch-up-back")
                                    .cursor_pointer()
                                    .child("Back")
                                    .on_click(cx.listener(|shell, _, _, cx| {
                                        shell.model.move_catch_up(-1);
                                        shell.sync_editor_inputs(cx);
                                        shell.refresh_month_view(cx);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id("catch-up-skip")
                                    .cursor_pointer()
                                    .child("Skip")
                                    .on_click(cx.listener(|shell, _, _, cx| {
                                        shell.model.move_catch_up(1);
                                        shell.sync_editor_inputs(cx);
                                        shell.refresh_month_view(cx);
                                        cx.notify();
                                    })),
                            )
                    })
                    .child(
                        div()
                            .id("cancel-editor")
                            .cursor_pointer()
                            .child("Cancel")
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.model.close_catch_up();
                                shell.refresh_month_view(cx);
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .id("save-entry")
                            .h(px(40.0))
                            .px(px(20.0))
                            .flex()
                            .items_center()
                            .rounded(px(20.0))
                            .cursor_pointer()
                            .bg(colors.primary)
                            .text_color(colors.on_primary)
                            .child(if self.model.catch_up.is_some() {
                                "Save and next"
                            } else {
                                "Save entry"
                            })
                            .on_click(cx.listener(|shell, _, _, cx| shell.save_editor(cx))),
                    ),
            )
    }
}

impl Focusable for AppShell {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}

impl Render for AppShell {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = self.colors();
        let scale = self.model.interface_scale.clamp(0.8, 1.5);
        let sidebar_width = if self.sidebar_collapsed { 80.0 } else { 256.0 } * scale;
        let workspace_name = self
            .model
            .active_workspace()
            .map_or_else(|| "Dagsverk".to_owned(), |workspace| workspace.name.clone());
        let month = format!(
            "{} {:04}",
            [
                "January",
                "February",
                "March",
                "April",
                "May",
                "June",
                "July",
                "August",
                "September",
                "October",
                "November",
                "December"
            ][self.model.current_month.month as usize - 1],
            self.model.current_month.year
        );
        let message = self
            .model
            .transient_error
            .clone()
            .or_else(|| self.notice.clone());
        let project_delete = self.confirm_project_delete.clone();

        div()
            .track_focus(&self.focus)
            .key_context("Dagsverk")
            .on_action(cx.listener(|shell, _: &ShowLedger, _, cx| {
                shell.set_view(MonthViewPreference::Ledger, cx)
            }))
            .on_action(cx.listener(|shell, _: &ShowCalendar, _, cx| {
                shell.set_view(MonthViewPreference::Calendar, cx)
            }))
            .on_action(
                cx.listener(|shell, _: &ShowSettings, _, cx| shell.set_route(Route::Settings, cx)),
            )
            .on_action(cx.listener(Self::previous_month))
            .on_action(cx.listener(Self::next_month))
            .on_action(cx.listener(Self::start_catch_up))
            .on_action(cx.listener(Self::save_active))
            .on_action(cx.listener(Self::close_surface))
            .relative()
            .size_full()
            .flex()
            .font_family("Roboto")
            .text_color(colors.on_surface)
            .bg(colors.background)
            .child(
                div()
                    .w(px(sidebar_width))
                    .h_full()
                    .flex_none()
                    .flex()
                    .flex_col()
                    .bg(colors.surface_container_low)
                    .child(
                        div()
                            .id("toggle-sidebar")
                            .h(px(72.0 * scale))
                            .px(px(24.0))
                            .flex()
                            .items_center()
                            .gap(px(18.0))
                            .cursor_pointer()
                            .child(m3_icon("menu", 24.0, colors))
                            .when(!self.sidebar_collapsed, |item| item.child("Dagsverk"))
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.sidebar_collapsed = !shell.sidebar_collapsed;
                                cx.notify();
                            })),
                    )
                    .child(
                        div()
                            .h(px(64.0))
                            .mx(px(12.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .rounded(px(16.0))
                            .bg(colors.surface_container)
                            .when(!self.sidebar_collapsed, |item| item.child(workspace_name)),
                    )
                    .child(self.navigation_item(
                        "nav-timesheet",
                        "schedule",
                        "Timesheet",
                        Route::Timesheet,
                        colors,
                        cx,
                    ))
                    .child(self.navigation_item(
                        "nav-projects",
                        "folder",
                        "Projects",
                        Route::Projects,
                        colors,
                        cx,
                    ))
                    .child(div().flex_1())
                    .child(self.navigation_item(
                        "nav-settings",
                        "settings",
                        "Settings",
                        Route::Settings,
                        colors,
                        cx,
                    )),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_1()
                    .h_full()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .h(px(64.0 * scale))
                            .px(px(24.0))
                            .flex()
                            .items_center()
                            .justify_between()
                            .bg(colors.surface)
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(16.0))
                                    .child(
                                        div()
                                            .id("previous-month")
                                            .cursor_pointer()
                                            .child(m3_icon("chevron_left", 24.0, colors))
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                let key = shell.model.previous_month();
                                                shell.load_month(key, cx);
                                            })),
                                    )
                                    .child(div().text_size(px(18.0)).child(month))
                                    .child(
                                        div()
                                            .id("next-month")
                                            .cursor_pointer()
                                            .child(m3_icon("chevron_right", 24.0, colors))
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                let key = shell.model.next_month();
                                                shell.load_month(key, cx);
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap(px(10.0))
                                    .child(
                                        div()
                                            .id("fill-month")
                                            .cursor_pointer()
                                            .child(m3_icon("playlist_add", 22.0, colors))
                                            .on_click(
                                                cx.listener(|shell, _, _, cx| shell.fill_month(cx)),
                                            ),
                                    )
                                    .child(
                                        div()
                                            .id("copy-month")
                                            .cursor_pointer()
                                            .opacity(if self.model.is_month_unstarted() {
                                                0.38
                                            } else {
                                                1.0
                                            })
                                            .child(m3_icon("content_copy", 22.0, colors))
                                            .when(!self.model.is_month_unstarted(), |button| {
                                                button.on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.copy_month(cx)
                                                }))
                                            }),
                                    )
                                    .child(
                                        div()
                                            .id("paste-month")
                                            .cursor_pointer()
                                            .opacity(if self.model.can_paste_month() {
                                                1.0
                                            } else {
                                                0.38
                                            })
                                            .child(m3_icon("content_paste", 22.0, colors))
                                            .when(self.model.can_paste_month(), |button| {
                                                button.on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.paste_month(cx)
                                                }))
                                            }),
                                    )
                                    .child(
                                        div()
                                            .id("reset-month")
                                            .cursor_pointer()
                                            .opacity(if self.model.can_reset_month() {
                                                1.0
                                            } else {
                                                0.38
                                            })
                                            .child(m3_icon("delete_sweep", 22.0, colors))
                                            .when(self.model.can_reset_month(), |button| {
                                                button.on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.confirm_reset = true;
                                                    cx.notify();
                                                }))
                                            }),
                                    )
                                    .when(self.model.missing_days_count() > 0, |actions| {
                                        actions.child(
                                            div()
                                                .id("catch-up")
                                                .h(px(40.0))
                                                .px(px(14.0))
                                                .flex()
                                                .items_center()
                                                .rounded(px(20.0))
                                                .cursor_pointer()
                                                .bg(colors.primary)
                                                .text_color(colors.on_primary)
                                                .child(format!(
                                                    "Catch Up ({})",
                                                    self.model.missing_days_count()
                                                ))
                                                .on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.model.start_catch_up();
                                                    shell.sync_editor_inputs(cx);
                                                    shell.refresh_month_view(cx);
                                                    cx.notify();
                                                })),
                                        )
                                    }),
                            )
                            .child(
                                div()
                                    .h(px(40.0))
                                    .p(px(2.0))
                                    .flex()
                                    .items_center()
                                    .rounded(px(20.0))
                                    .border_1()
                                    .border_color(colors.outline_variant)
                                    .child(
                                        div()
                                            .id("view-ledger")
                                            .h_full()
                                            .px(px(16.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(18.0))
                                            .cursor_pointer()
                                            .bg(
                                                if self.model.active_view
                                                    == MonthViewPreference::Ledger
                                                {
                                                    colors.secondary_container
                                                } else {
                                                    colors.surface
                                                },
                                            )
                                            .child("Ledger")
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                shell.set_view(MonthViewPreference::Ledger, cx)
                                            })),
                                    )
                                    .child(
                                        div()
                                            .id("view-calendar")
                                            .h_full()
                                            .px(px(16.0))
                                            .flex()
                                            .items_center()
                                            .rounded(px(18.0))
                                            .cursor_pointer()
                                            .bg(
                                                if self.model.active_view
                                                    == MonthViewPreference::Calendar
                                                {
                                                    colors.secondary_container
                                                } else {
                                                    colors.surface
                                                },
                                            )
                                            .child("Calendar")
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                shell.set_view(MonthViewPreference::Calendar, cx)
                                            })),
                                    ),
                            )
                            .child(
                                div()
                                    .id("toggle-theme")
                                    .cursor_pointer()
                                    .child(m3_icon("dark_mode", 24.0, colors))
                                    .on_click(cx.listener(|shell, _, _, cx| {
                                        if let Err(error) = shell.model.toggle_theme() {
                                            shell.model.transient_error = Some(error.to_string());
                                        }
                                        shell.refresh_month_view(cx);
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_h_0()
                            .flex()
                            .child(
                                div()
                                    .id("route-content")
                                    .flex_1()
                                    .min_w_0()
                                    .overflow_y_scroll()
                                    .rounded_tl(px(24.0))
                                    .bg(colors.background)
                                    .child(self.route_content(colors, cx)),
                            )
                            .when(self.model.editor.is_open, |content| {
                                content.child(self.editor_panel(colors, cx))
                            }),
                    ),
            )
            .when_some(message, |root, message| {
                root.child(
                    div()
                        .absolute()
                        .bottom(px(24.0))
                        .left(px(280.0))
                        .px(px(18.0))
                        .h(px(48.0))
                        .flex()
                        .items_center()
                        .rounded(px(12.0))
                        .bg(colors.on_surface)
                        .text_color(colors.surface_container_lowest)
                        .child(message),
                )
            })
            .when(self.confirm_reset, |root| {
                root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::black().opacity(0.45))
                        .child(
                            div()
                                .w(px(420.0))
                                .p(px(24.0))
                                .flex()
                                .flex_col()
                                .gap(px(18.0))
                                .rounded(px(24.0))
                                .bg(colors.surface_container_high)
                                .child(div().text_size(px(20.0)).child("Reset month?"))
                                .child(
                                    div()
                                        .text_color(colors.on_surface_variant)
                                        .child("All entries and the month record will be deleted."),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(16.0))
                                        .child(
                                            div()
                                                .id("cancel-reset")
                                                .cursor_pointer()
                                                .child("Cancel")
                                                .on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.confirm_reset = false;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("confirm-reset")
                                                .cursor_pointer()
                                                .text_color(colors.error)
                                                .child("Reset")
                                                .on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.reset_month(cx)
                                                })),
                                        ),
                                ),
                        ),
                )
            })
            .when_some(project_delete, |root, project_id| {
                root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::black().opacity(0.45))
                        .child(
                            div()
                                .w(px(420.0))
                                .p(px(24.0))
                                .flex()
                                .flex_col()
                                .gap(px(18.0))
                                .rounded(px(24.0))
                                .bg(colors.surface_container_high)
                                .child(div().text_size(px(20.0)).child("Delete project?"))
                                .child("Existing entries keep the stored project name.")
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(16.0))
                                        .child(
                                            div()
                                                .id("cancel-project-delete")
                                                .cursor_pointer()
                                                .child("Cancel")
                                                .on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.confirm_project_delete = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("confirm-project-delete")
                                                .cursor_pointer()
                                                .text_color(colors.error)
                                                .child("Delete")
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    shell.delete_project(&project_id, cx)
                                                })),
                                        ),
                                ),
                        ),
                )
            })
    }
}

fn month_view_data(model: &AppModel) -> MonthViewData {
    MonthViewData {
        month: model.current_month,
        entries: model.entries.clone(),
        settings: model.settings.clone(),
        projects: model.projects.clone(),
        today: model.today(),
        selected_date: model.selected_date,
        month_started: !model.is_month_unstarted(),
        mode: model.active_view,
        colors: M3ColorScheme::resolve(match model.resolved_theme {
            ResolvedTheme::Light => UiTheme::Light,
            ResolvedTheme::Dark => UiTheme::Dark,
        }),
    }
}

fn format_hours_input(minutes: i64) -> String {
    let whole = minutes / 60;
    let remainder = minutes % 60;
    if remainder == 0 {
        whole.to_string()
    } else {
        format!("{whole}.{:02}", remainder * 100 / 60)
    }
}

fn parse_scheduled_minutes(value: &str) -> Result<Minutes, &'static str> {
    let hours = value
        .trim()
        .parse::<f64>()
        .map_err(|_| "Scheduled hours must be a non-negative number.")?;
    if !hours.is_finite() || hours < 0.0 {
        return Err("Scheduled hours must be a non-negative number.");
    }
    Ok(Minutes::new((hours * 60.0).round() as i64))
}

fn is_hex_color(value: &str) -> bool {
    let value = value.strip_prefix('#').unwrap_or(value);
    value.len() == 6 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn color_from_hex(value: &str) -> Option<gpui::Hsla> {
    let value = value.strip_prefix('#').unwrap_or(value);
    is_hex_color(value)
        .then(|| u32::from_str_radix(value, 16).ok())
        .flatten()
        .map(|value| gpui::rgb(value).into())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{DateTime, Utc};
    use dagsverk_core::{clock::FixedClock, models::MonthViewPreference, tax::TaxEngine};
    use dagsverk_data::Database;
    use gpui::TestAppContext;
    use tempfile::tempdir;

    use super::{AppShell, parse_scheduled_minutes};
    use crate::state::AppModel;

    #[gpui::test]
    fn global_view_shortcuts_persist_from_the_focused_shell(cx: &mut TestAppContext) {
        let directory = tempdir().expect("temporary data directory");
        let now = DateTime::parse_from_rfc3339("2026-08-18T10:00:00Z")
            .expect("fixed time")
            .with_timezone(&Utc);
        let clock = FixedClock::new(now);
        let repository = Arc::new(
            Database::open(directory.path().join("dagsverk.db"), clock)
                .expect("temporary database"),
        );
        let mut model = AppModel::new(repository, Arc::new(clock), TaxEngine::default(), false);
        model.initialize().expect("application state");

        cx.update(AppShell::register_key_bindings);
        let (shell, cx) = cx.add_window_view(|window, cx| AppShell::new(model, window, cx));
        cx.simulate_keystrokes("ctrl-2");
        assert_eq!(
            shell.read_with(cx, |shell, _| shell.model.active_view),
            MonthViewPreference::Calendar
        );
    }

    #[test]
    fn scheduled_override_accepts_zero_and_rejects_invalid_values() {
        assert_eq!(parse_scheduled_minutes("0").expect("zero hours").value(), 0);
        assert_eq!(
            parse_scheduled_minutes("7.5")
                .expect("fractional hours")
                .value(),
            450
        );
        assert!(parse_scheduled_minutes("-1").is_err());
        assert!(parse_scheduled_minutes("NaN").is_err());
    }
}
