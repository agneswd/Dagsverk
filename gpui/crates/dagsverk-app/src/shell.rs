use std::{path::PathBuf, sync::Arc};

use chrono::Duration;
use dagsverk_core::{
    calculations::{calculate_daily_pay, normalize_time},
    models::{
        AppPreferences, AppSettings, CompensationRateType, CompensationRuleType,
        CurrencyPreference, ExportLanguagePreference, HourlyPayBasis, LanguagePreference, Minutes,
        Money, MonthViewPreference, ObOvertimeCombinationMode, OvertimeCompensationMode,
        OvertimeDayCategory, OvertimeRateBand, OvertimeThresholdMode, Project, ProjectId,
        SalaryType, TaxMode, ThemePreference, WorkEntryStatus, WorkspaceId, WorkspaceType,
    },
};
use dagsverk_data::DataMaintenance;
use dagsverk_ui::{
    m3::{M3ColorScheme, ResolvedTheme as UiTheme, m3_icon},
    text_input::TextInput,
    views::timesheet::{MonthView, MonthViewData, MonthViewEvent, summary_banner},
};
use gpui::{
    App, AppContext, Context, ElementId, Entity, FocusHandle, Focusable, KeyBinding, Render,
    SharedString, Stateful, Window, actions, div, prelude::*, px,
};
use rust_decimal::{Decimal, prelude::ToPrimitive};

use crate::{
    platform::{FileDialogService, OpenFileRequest, ShellService},
    state::{AppModel, ResolvedTheme, Route},
};

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

struct SettingsInputs {
    opening_balance: Entity<TextInput>,
    expected_hours: Entity<TextInput>,
    default_start: Entity<TextInput>,
    default_end: Entity<TextInput>,
    default_lunch: Entity<TextInput>,
    overtime_threshold: Entity<TextInput>,
    default_rate_value: Entity<TextInput>,
    hourly_rate: Entity<TextInput>,
    monthly_salary: Entity<TextInput>,
    employment_percent: Entity<TextInput>,
    tax_year: Entity<TextInput>,
    tax_table: Entity<TextInput>,
    tax_column: Entity<TextInput>,
    manual_tax: Entity<TextInput>,
}

impl SettingsInputs {
    fn new(settings: &AppSettings, cx: &mut Context<AppShell>) -> Self {
        let inputs = Self {
            opening_balance: cx.new(|cx| TextInput::new(cx, "Starting balance hours")),
            expected_hours: cx.new(|cx| TextInput::new(cx, "Hours per workday")),
            default_start: cx.new(|cx| TextInput::new(cx, "Default start")),
            default_end: cx.new(|cx| TextInput::new(cx, "Default end")),
            default_lunch: cx.new(|cx| TextInput::new(cx, "Lunch minutes")),
            overtime_threshold: cx.new(|cx| TextInput::new(cx, "Daily threshold hours")),
            default_rate_value: cx.new(|cx| TextInput::new(cx, "Default rate value")),
            hourly_rate: cx.new(|cx| TextInput::new(cx, "Hourly rate")),
            monthly_salary: cx.new(|cx| TextInput::new(cx, "Monthly salary")),
            employment_percent: cx.new(|cx| TextInput::new(cx, "Employment percent")),
            tax_year: cx.new(|cx| TextInput::new(cx, "Tax year")),
            tax_table: cx.new(|cx| TextInput::new(cx, "Tax table")),
            tax_column: cx.new(|cx| TextInput::new(cx, "Tax column")),
            manual_tax: cx.new(|cx| TextInput::new(cx, "Manual monthly deduction")),
        };
        inputs.sync(settings, cx);
        inputs
    }

    fn sync(&self, settings: &AppSettings, cx: &mut Context<AppShell>) {
        let values = [
            (
                &self.opening_balance,
                format_hours_input(settings.opening_balance_minutes.value()),
            ),
            (
                &self.expected_hours,
                settings.expected_hours.hours_per_workday.to_string(),
            ),
            (&self.default_start, settings.default_start_time.to_string()),
            (&self.default_end, settings.default_end_time.to_string()),
            (
                &self.default_lunch,
                settings.default_lunch_minutes.value().to_string(),
            ),
            (
                &self.overtime_threshold,
                settings
                    .overtime_compensation
                    .daily_threshold_hours
                    .to_string(),
            ),
            (
                &self.default_rate_value,
                settings
                    .overtime_compensation
                    .default_rate_value
                    .to_string(),
            ),
            (
                &self.hourly_rate,
                settings.salary.hourly_rate.decimal().to_string(),
            ),
            (
                &self.monthly_salary,
                settings.salary.monthly_salary.decimal().to_string(),
            ),
            (
                &self.employment_percent,
                settings.salary.employment_percent.to_string(),
            ),
            (&self.tax_year, settings.tax_settings.tax_year.to_string()),
            (
                &self.tax_table,
                settings.tax_settings.table_number.to_string(),
            ),
            (&self.tax_column, settings.tax_settings.column.to_string()),
            (
                &self.manual_tax,
                settings
                    .tax_settings
                    .manual_monthly_deduction
                    .map_or_else(String::new, |money| money.decimal().to_string()),
            ),
        ];
        for (input, value) in values {
            input.update(cx, |input, cx| input.set_text(value, cx));
        }
    }
}

pub struct AppShell {
    model: AppModel,
    services: AppShellServices,
    month_view: Entity<MonthView>,
    start_input: Entity<TextInput>,
    end_input: Entity<TextInput>,
    notes_input: Entity<TextInput>,
    scheduled_input: Entity<TextInput>,
    project_name_input: Entity<TextInput>,
    project_color_input: Entity<TextInput>,
    workspace_name_input: Entity<TextInput>,
    workspace_worker_input: Entity<TextInput>,
    workspace_organization_input: Entity<TextInput>,
    workspace_color_input: Entity<TextInput>,
    settings_inputs: SettingsInputs,
    focus: FocusHandle,
    sidebar_collapsed: bool,
    confirm_reset: bool,
    confirm_project_delete: Option<ProjectId>,
    manage_workspaces: bool,
    confirm_workspace_delete: Option<WorkspaceId>,
    new_workspace_type: WorkspaceType,
    settings_tab: usize,
    settings_draft: AppSettings,
    preferences_draft: AppPreferences,
    pending_currency: Option<CurrencyPreference>,
    maintenance_busy: bool,
    confirm_restore: Option<PathBuf>,
    confirm_import: Option<PathBuf>,
    last_backup: Option<PathBuf>,
    notice: Option<String>,
}

pub struct AppShellServices {
    pub data: Arc<dyn DataMaintenance>,
    pub file_dialog: Arc<dyn FileDialogService>,
    pub shell: Arc<dyn ShellService>,
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

    pub fn new(
        model: AppModel,
        services: AppShellServices,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        window.set_window_title("Dagsverk GPUI Preview");
        let month_view = cx.new(|_| MonthView::new(month_view_data(&model)));
        let start_input = cx.new(|cx| TextInput::new(cx, "Start time"));
        let end_input = cx.new(|cx| TextInput::new(cx, "End time"));
        let notes_input = cx.new(|cx| TextInput::new(cx, "Notes"));
        let scheduled_input = cx.new(|cx| TextInput::new(cx, "Scheduled hours"));
        let project_name_input = cx.new(|cx| TextInput::new(cx, "Project name"));
        let project_color_input = cx.new(|cx| TextInput::new(cx, "#5F875F"));
        project_color_input.update(cx, |input, cx| input.set_text("#5F875F", cx));
        let workspace_name_input = cx.new(|cx| TextInput::new(cx, "Workspace name"));
        let workspace_worker_input = cx.new(|cx| TextInput::new(cx, "Worker name"));
        let workspace_organization_input = cx.new(|cx| TextInput::new(cx, "Organization"));
        let workspace_color_input = cx.new(|cx| TextInput::new(cx, "#5F875F"));
        workspace_color_input.update(cx, |input, cx| input.set_text("#5F875F", cx));
        let settings_inputs = SettingsInputs::new(&model.settings, cx);
        let settings_draft = model.settings.clone();
        let preferences_draft = model.preferences.clone();
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
            services,
            month_view,
            start_input,
            end_input,
            notes_input,
            scheduled_input,
            project_name_input,
            project_color_input,
            workspace_name_input,
            workspace_worker_input,
            workspace_organization_input,
            workspace_color_input,
            settings_inputs,
            focus,
            sidebar_collapsed: false,
            confirm_reset: false,
            confirm_project_delete: None,
            manage_workspaces: false,
            confirm_workspace_delete: None,
            new_workspace_type: WorkspaceType::Employment,
            settings_tab: 0,
            settings_draft,
            preferences_draft,
            pending_currency: None,
            maintenance_busy: false,
            confirm_restore: None,
            confirm_import: None,
            last_backup: None,
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
        if route == Route::Settings && self.model.route != Route::Settings {
            self.settings_draft = self.model.settings.clone();
            self.preferences_draft = self.model.preferences.clone();
            self.settings_inputs.sync(&self.settings_draft, cx);
        }
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
        if self.model.route == Route::Settings {
            self.save_settings(cx);
        } else {
            self.save_editor(cx);
        }
    }

    fn close_surface(&mut self, _: &CloseSurface, _: &mut Window, cx: &mut Context<Self>) {
        if self.confirm_restore.take().is_some() {
            cx.notify();
            return;
        }
        if self.confirm_import.take().is_some() {
            cx.notify();
            return;
        }
        if self.confirm_reset {
            self.confirm_reset = false;
            cx.notify();
            return;
        }
        if self.confirm_project_delete.take().is_some() {
            cx.notify();
            return;
        }
        if self.confirm_workspace_delete.take().is_some() {
            cx.notify();
            return;
        }
        if self.pending_currency.take().is_some() {
            cx.notify();
            return;
        }
        if self.manage_workspaces {
            self.manage_workspaces = false;
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

    fn create_workspace(&mut self, cx: &mut Context<Self>) {
        let name = self.workspace_name_input.read(cx).text().trim().to_owned();
        let color = self.workspace_color_input.read(cx).text().trim().to_owned();
        if name.is_empty() || !is_hex_color(&color) {
            self.model.transient_error =
                Some("Enter a workspace name and a six-digit hex color.".to_owned());
            cx.notify();
            return;
        }
        let worker = nonempty(self.workspace_worker_input.read(cx).text());
        let organization = nonempty(self.workspace_organization_input.read(cx).text());
        match self.model.create_workspace(
            name,
            color,
            self.new_workspace_type,
            worker,
            organization,
        ) {
            Ok(_) => {
                self.workspace_name_input
                    .update(cx, |input, cx| input.set_text("", cx));
                self.notice = Some("Workspace created.".to_owned());
            }
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn switch_workspace(&mut self, id: &WorkspaceId, cx: &mut Context<Self>) {
        match self.model.switch_workspace(id) {
            Ok(()) => {
                self.notice = Some("Workspace changed.".to_owned());
                self.manage_workspaces = false;
            }
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn update_workspace_color(&mut self, id: &WorkspaceId, cx: &mut Context<Self>) {
        let color = self.workspace_color_input.read(cx).text().trim().to_owned();
        if !is_hex_color(&color) {
            self.model.transient_error = Some("Enter a six-digit hex color.".to_owned());
            cx.notify();
            return;
        }
        let Some(mut workspace) = self
            .model
            .workspaces
            .iter()
            .find(|workspace| &workspace.id == id)
            .cloned()
        else {
            return;
        };
        workspace.color = color;
        match self.model.save_workspace(workspace) {
            Ok(()) => self.notice = Some("Workspace color updated.".to_owned()),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        cx.notify();
    }

    fn delete_workspace(&mut self, id: &WorkspaceId, cx: &mut Context<Self>) {
        match self.model.delete_workspace(id) {
            Ok(()) => self.notice = Some("Workspace deleted.".to_owned()),
            Err(error) => self.model.transient_error = Some(error.to_string()),
        }
        self.confirm_workspace_delete = None;
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn save_settings(&mut self, cx: &mut Context<Self>) {
        let result = self.parse_settings_inputs(cx);
        let Ok(mut settings) = result else {
            self.model.transient_error = result.err().map(str::to_owned);
            cx.notify();
            return;
        };
        settings.workspace_id = Some(self.model.active_workspace_id.clone());
        if let Err(error) = self.model.update_settings(settings.clone()) {
            self.model.transient_error = Some(error.to_string());
            cx.notify();
            return;
        }
        if let Err(error) = self
            .model
            .update_preferences(self.preferences_draft.clone())
        {
            self.model.transient_error = Some(error.to_string());
            cx.notify();
            return;
        }
        self.settings_draft = settings;
        self.preferences_draft = self.model.preferences.clone();
        self.notice = Some("Settings saved.".to_owned());
        self.refresh_month_view(cx);
        cx.notify();
    }

    fn parse_settings_inputs(&self, cx: &Context<Self>) -> Result<AppSettings, &'static str> {
        let mut settings = self.settings_draft.clone();
        let opening =
            parse_non_negative_decimal(self.settings_inputs.opening_balance.read(cx).text())?;
        settings.opening_balance_minutes = Minutes::new(
            (opening * Decimal::from(60))
                .round()
                .to_i64()
                .ok_or("Starting balance is too large.")?,
        );
        settings.expected_hours.hours_per_workday =
            parse_non_negative_decimal(self.settings_inputs.expected_hours.read(cx).text())?;
        settings.default_start_time =
            normalize_time(self.settings_inputs.default_start.read(cx).text())
                .ok_or("Default start time is invalid.")?;
        settings.default_end_time =
            normalize_time(self.settings_inputs.default_end.read(cx).text())
                .ok_or("Default end time is invalid.")?;
        settings.default_lunch_minutes = Minutes::new(parse_non_negative_i64(
            self.settings_inputs.default_lunch.read(cx).text(),
        )?);
        settings.overtime_compensation.daily_threshold_hours =
            parse_non_negative_decimal(self.settings_inputs.overtime_threshold.read(cx).text())?;
        settings.overtime_compensation.default_rate_value =
            parse_non_negative_decimal(self.settings_inputs.default_rate_value.read(cx).text())?;
        settings.salary.hourly_rate = Money::new(parse_non_negative_decimal(
            self.settings_inputs.hourly_rate.read(cx).text(),
        )?);
        settings.salary.monthly_salary = Money::new(parse_non_negative_decimal(
            self.settings_inputs.monthly_salary.read(cx).text(),
        )?);
        let employment =
            parse_non_negative_decimal(self.settings_inputs.employment_percent.read(cx).text())?;
        if employment < Decimal::ONE || employment > Decimal::ONE_HUNDRED {
            return Err("Employment percent must be from 1 to 100.");
        }
        settings.salary.employment_percent = employment;
        settings.tax_settings.tax_year = parse_i32(self.settings_inputs.tax_year.read(cx).text())?;
        settings.tax_settings.table_number =
            parse_i32(self.settings_inputs.tax_table.read(cx).text())?;
        if settings.tax_settings.table_number <= 0 {
            return Err("Tax table must be positive.");
        }
        settings.tax_settings.column = parse_i32(self.settings_inputs.tax_column.read(cx).text())?;
        if !(1..=6).contains(&settings.tax_settings.column) {
            return Err("Tax column must be from 1 to 6.");
        }
        let manual = self.settings_inputs.manual_tax.read(cx).text().trim();
        settings.tax_settings.manual_monthly_deduction = if manual.is_empty() {
            None
        } else {
            Some(Money::new(parse_non_negative_decimal(manual)?))
        };
        if ![80, 90, 100, 110, 125, 150].contains(&self.preferences_draft.interface_scale_percent) {
            return Err("Interface scale is invalid.");
        }
        Ok(settings)
    }

    fn discard_settings(&mut self, cx: &mut Context<Self>) {
        self.settings_draft = self.model.settings.clone();
        self.preferences_draft = self.model.preferences.clone();
        self.settings_inputs.sync(&self.settings_draft, cx);
        self.model.transient_error = None;
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
            Route::Settings => self.settings_page(colors, cx),
            Route::DataBackups => self.data_backups_page(colors, cx),
        }
    }

    fn data_backups_page(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let database_path = self.services.data.database_path();
        let busy = self.maintenance_busy;
        div()
            .max_w(px(880.0))
            .mx_auto()
            .p(px(32.0))
            .flex()
            .flex_col()
            .gap(px(20.0))
            .child(div().text_size(px(28.0)).child("Data & Backups"))
            .child("Current database")
            .child(
                div()
                    .p(px(16.0))
                    .rounded(px(12.0))
                    .bg(colors.surface_container)
                    .text_color(colors.on_surface_variant)
                    .child(database_path.display().to_string()),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(
                        maintenance_button("open-data-folder", "Open data folder", !busy, colors)
                            .when(!busy, |button| {
                                button.on_click(cx.listener(|shell, _, _, cx| {
                                    shell.open_data_folder();
                                    cx.notify();
                                }))
                            }),
                    )
                    .child(
                        maintenance_button("create-backup", "Create backup", !busy, colors).when(
                            !busy,
                            |button| {
                                button.on_click(
                                    cx.listener(|shell, _, _, cx| shell.create_backup(cx)),
                                )
                            },
                        ),
                    ),
            )
            .when_some(self.last_backup.clone(), |page, path| {
                page.child(format!("Last backup: {}", path.display()))
            })
            .child(
                div()
                    .mt(px(12.0))
                    .text_size(px(20.0))
                    .child("Restore or import"),
            )
            .child(
                div()
                    .text_color(colors.on_surface_variant)
                    .child("Close Electron Dagsverk before restore or import."),
            )
            .child(
                div()
                    .flex()
                    .gap(px(12.0))
                    .child(
                        maintenance_button("restore-database", "Restore database", !busy, colors)
                            .when(!busy, |button| {
                                button.on_click(
                                    cx.listener(|shell, _, _, cx| shell.choose_restore(cx)),
                                )
                            }),
                    )
                    .child(
                        maintenance_button("import-tidverk", "Import Tidverk", !busy, colors).when(
                            !busy,
                            |button| {
                                button.on_click(
                                    cx.listener(|shell, _, _, cx| shell.choose_tidverk_import(cx)),
                                )
                            },
                        ),
                    ),
            )
            .when(busy, |page| {
                page.child(
                    div()
                        .text_color(colors.primary)
                        .child("Database operation in progress..."),
                )
            })
    }

    fn open_data_folder(&mut self) {
        let path = self.services.data.database_path();
        let folder = path.parent().unwrap_or(path.as_path());
        if let Err(error) = self.services.shell.open_folder(folder) {
            self.notice = Some(error.to_string());
        }
    }

    fn create_backup(&mut self, cx: &mut Context<Self>) {
        self.maintenance_busy = true;
        self.notice = None;
        let data = self.services.data.clone();
        let task = cx.background_executor().spawn(async move {
            data.create_manual_backup()
                .map_err(|error| error.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |shell, cx| {
                shell.maintenance_busy = false;
                match result {
                    Ok(path) => {
                        shell.last_backup = Some(path.clone());
                        shell.notice = Some(format!("Backup created: {}", path.display()));
                    }
                    Err(error) => shell.notice = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn choose_restore(&mut self, cx: &mut Context<Self>) {
        self.choose_database_file("Select a Dagsverk backup", false, cx);
    }

    fn choose_tidverk_import(&mut self, cx: &mut Context<Self>) {
        self.choose_database_file("Select a Tidverk database", true, cx);
    }

    fn choose_database_file(&mut self, title: &'static str, tidverk: bool, cx: &mut Context<Self>) {
        self.maintenance_busy = true;
        let dialog = self.services.file_dialog.clone();
        let directory = self
            .services
            .data
            .database_path()
            .parent()
            .map(std::path::Path::to_owned);
        cx.spawn(async move |this, cx| {
            let result = dialog
                .choose_open_file(OpenFileRequest {
                    title: title.to_owned(),
                    filters: vec![("SQLite database".to_owned(), vec!["db".to_owned()])],
                    directory,
                })
                .await;
            let _ = this.update(cx, |shell, cx| {
                shell.maintenance_busy = false;
                match result {
                    Ok(Some(path)) if tidverk => shell.confirm_import = Some(path),
                    Ok(Some(path)) => shell.confirm_restore = Some(path),
                    Ok(None) => {}
                    Err(error) => shell.notice = Some(error.to_string()),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn run_restore(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.confirm_restore = None;
        self.run_database_change(path, false, cx);
    }

    fn run_tidverk_import(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.confirm_import = None;
        self.run_database_change(path, true, cx);
    }

    fn run_database_change(&mut self, path: PathBuf, tidverk: bool, cx: &mut Context<Self>) {
        self.maintenance_busy = true;
        self.notice = None;
        let data = self.services.data.clone();
        let task = cx.background_executor().spawn(async move {
            if tidverk {
                data.import_tidverk_database(&path)
                    .map(|result| format!("Imported {} entries from Tidverk.", result.entry_count))
            } else {
                data.restore(&path)
                    .map(|()| "Database restored.".to_owned())
            }
            .map_err(|error| error.to_string())
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |shell, cx| {
                shell.maintenance_busy = false;
                match result {
                    Ok(message) => match shell.model.initialize() {
                        Ok(()) => {
                            shell.settings_draft = shell.model.settings.clone();
                            shell.preferences_draft = shell.model.preferences.clone();
                            shell.settings_inputs.sync(&shell.settings_draft, cx);
                            shell.refresh_month_view(cx);
                            shell.notice = Some(message);
                        }
                        Err(error) => shell.notice = Some(error.to_string()),
                    },
                    Err(error) => shell.notice = Some(error),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
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

    fn workspace_dialog(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let workspaces = self.model.workspaces.clone();
        let active = self.model.active_workspace_id.clone();
        let can_delete = workspaces.len() > 1;
        div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(gpui::black().opacity(0.45))
            .child(
                div()
                    .w(px(760.0))
                    .max_h(px(700.0))
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(16.0))
                    .rounded(px(28.0))
                    .bg(colors.surface_container_high)
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .child(div().text_size(px(22.0)).child("Manage workspaces"))
                            .child(
                                div()
                                    .id("close-workspaces")
                                    .cursor_pointer()
                                    .child(m3_icon("close", 24.0, colors))
                                    .on_click(cx.listener(|shell, _, _, cx| {
                                        shell.manage_workspaces = false;
                                        cx.notify();
                                    })),
                            ),
                    )
                    .child(
                        div()
                            .grid()
                            .grid_cols(2)
                            .gap(px(16.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(10.0))
                                    .child("Workspace name")
                                    .child(self.workspace_name_input.clone())
                                    .child("Worker name")
                                    .child(self.workspace_worker_input.clone())
                                    .child("Organization or client")
                                    .child(self.workspace_organization_input.clone()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .gap(px(10.0))
                                    .child("Type")
                                    .child(
                                        div().flex().gap(px(6.0)).children(
                                            [
                                                ("Employment", WorkspaceType::Employment),
                                                ("Contract", WorkspaceType::Contract),
                                                ("Personal", WorkspaceType::Personal),
                                            ]
                                            .into_iter()
                                            .enumerate()
                                            .map(
                                                |(index, (label, value))| {
                                                    div()
                                                        .id(("workspace-type", index))
                                                        .h(px(36.0))
                                                        .px(px(10.0))
                                                        .flex()
                                                        .items_center()
                                                        .rounded(px(18.0))
                                                        .cursor_pointer()
                                                        .bg(if self.new_workspace_type == value {
                                                            colors.secondary_container
                                                        } else {
                                                            colors.surface_container
                                                        })
                                                        .child(label)
                                                        .on_click(cx.listener(
                                                            move |shell, _, _, cx| {
                                                                shell.new_workspace_type = value;
                                                                cx.notify();
                                                            },
                                                        ))
                                                },
                                            ),
                                        ),
                                    )
                                    .child("Accent color")
                                    .child(self.workspace_color_input.clone())
                                    .child(
                                        div()
                                            .id("create-workspace")
                                            .h(px(40.0))
                                            .flex()
                                            .items_center()
                                            .justify_center()
                                            .rounded(px(20.0))
                                            .cursor_pointer()
                                            .bg(colors.primary)
                                            .text_color(colors.on_primary)
                                            .child("Create workspace")
                                            .on_click(cx.listener(|shell, _, _, cx| {
                                                shell.create_workspace(cx)
                                            })),
                                    ),
                            ),
                    )
                    .child(
                        div()
                            .id("workspace-list")
                            .max_h(px(300.0))
                            .overflow_y_scroll()
                            .flex()
                            .flex_col()
                            .gap(px(8.0))
                            .children(workspaces.into_iter().enumerate().map(
                                |(index, workspace)| {
                                    let switch_id = workspace.id.clone();
                                    let color_id = workspace.id.clone();
                                    let delete_id = workspace.id.clone();
                                    let is_active = workspace.id == active;
                                    div()
                                        .h(px(60.0))
                                        .px(px(14.0))
                                        .flex()
                                        .items_center()
                                        .gap(px(12.0))
                                        .rounded(px(12.0))
                                        .bg(colors.surface_container)
                                        .child(
                                            div()
                                                .size(px(12.0))
                                                .rounded_full()
                                                .bg(color_from_hex(&workspace.color)
                                                    .unwrap_or(colors.primary)),
                                        )
                                        .child(div().flex_1().child(workspace.name))
                                        .when(is_active, |row| row.child("Active"))
                                        .when(!is_active, |row| {
                                            row.child(
                                                div()
                                                    .id(("switch-workspace", index))
                                                    .cursor_pointer()
                                                    .child("Switch")
                                                    .on_click(cx.listener(
                                                        move |shell, _, _, cx| {
                                                            shell.switch_workspace(&switch_id, cx)
                                                        },
                                                    )),
                                            )
                                        })
                                        .child(
                                            div()
                                                .id(("workspace-color", index))
                                                .cursor_pointer()
                                                .child("Apply color")
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    shell.update_workspace_color(&color_id, cx)
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id(("delete-workspace", index))
                                                .opacity(if can_delete { 1.0 } else { 0.38 })
                                                .text_color(colors.error)
                                                .child("Delete")
                                                .when(can_delete, |button| {
                                                    button.cursor_pointer().on_click(cx.listener(
                                                        move |shell, _, _, cx| {
                                                            shell.confirm_workspace_delete =
                                                                Some(delete_id.clone());
                                                            cx.notify();
                                                        },
                                                    ))
                                                }),
                                        )
                                },
                            )),
                    ),
            )
    }

    fn settings_page(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let dirty = self
            .parse_settings_inputs(cx)
            .map_or(true, |settings| settings != self.model.settings)
            || self.preferences_draft != self.model.preferences;
        let content = match self.settings_tab {
            0 => self.general_settings(colors, cx),
            1 => self.schedule_settings(colors, cx),
            2 => self.overtime_settings(colors, cx),
            3 => self.salary_tax_settings(colors, cx),
            _ => self.application_settings(colors, cx),
        };
        div()
            .max_w(px(1088.0))
            .mx_auto()
            .p(px(32.0))
            .flex()
            .flex_col()
            .gap(px(20.0))
            .child(div().text_size(px(24.0)).child("Settings"))
            .child(
                div()
                    .h(px(44.0))
                    .flex()
                    .gap(px(4.0))
                    .border_b_1()
                    .border_color(colors.outline_variant)
                    .children(
                        [
                            "General",
                            "Schedule",
                            "Overtime & OB",
                            "Salary & Tax",
                            "Application",
                        ]
                        .into_iter()
                        .enumerate()
                        .map(|(index, label)| {
                            div()
                                .id(("settings-tab", index))
                                .h_full()
                                .px(px(16.0))
                                .flex()
                                .items_center()
                                .cursor_pointer()
                                .border_b_2()
                                .border_color(if self.settings_tab == index {
                                    colors.primary
                                } else {
                                    gpui::transparent_black()
                                })
                                .child(label)
                                .on_click(cx.listener(move |shell, _, _, cx| {
                                    shell.settings_tab = index;
                                    cx.notify();
                                }))
                        }),
                    ),
            )
            .child(
                div()
                    .p(px(24.0))
                    .flex()
                    .flex_col()
                    .gap(px(18.0))
                    .rounded(px(16.0))
                    .bg(colors.surface_container_low)
                    .child(content),
            )
            .child(
                div()
                    .flex()
                    .justify_end()
                    .gap(px(16.0))
                    .child(
                        div()
                            .id("discard-settings")
                            .h(px(40.0))
                            .px(px(18.0))
                            .flex()
                            .items_center()
                            .rounded(px(20.0))
                            .opacity(if dirty { 1.0 } else { 0.38 })
                            .child("Discard")
                            .when(dirty, |button| {
                                button.cursor_pointer().on_click(
                                    cx.listener(|shell, _, _, cx| shell.discard_settings(cx)),
                                )
                            }),
                    )
                    .child(
                        div()
                            .id("save-settings")
                            .h(px(40.0))
                            .px(px(20.0))
                            .flex()
                            .items_center()
                            .rounded(px(20.0))
                            .opacity(if dirty { 1.0 } else { 0.38 })
                            .bg(colors.primary)
                            .text_color(colors.on_primary)
                            .child("Save")
                            .when(dirty, |button| {
                                button.cursor_pointer().on_click(
                                    cx.listener(|shell, _, _, cx| shell.save_settings(cx)),
                                )
                            }),
                    ),
            )
    }

    fn general_settings(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let current_currency = self.settings_draft.currency_preference;
        let projects = self.model.projects.clone();
        div()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(div().text_size(px(18.0)).child("General"))
            .child("Default project")
            .child(
                div().flex().flex_wrap().gap(px(8.0)).children(
                    projects
                        .into_iter()
                        .filter(|project| project.is_active)
                        .enumerate()
                        .map(|(index, project)| {
                            let name = project.name.clone();
                            let selected = self.settings_draft.default_project == project.name;
                            setting_chip(
                                ("default-setting-project", index),
                                project.name,
                                selected,
                                colors,
                            )
                            .on_click(cx.listener(
                                move |shell, _, _, cx| {
                                    shell.settings_draft.default_project = name.clone();
                                    cx.notify();
                                },
                            ))
                        }),
                ),
            )
            .child("Currency")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("SEK", CurrencyPreference::Sek),
                        ("EUR", CurrencyPreference::Eur),
                        ("USD", CurrencyPreference::Usd),
                        ("GBP", CurrencyPreference::Gbp),
                        ("NOK", CurrencyPreference::Nok),
                        ("DKK", CurrencyPreference::Dkk),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("currency", index),
                            label,
                            current_currency == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            if shell.settings_draft.currency_preference != value {
                                shell.pending_currency = Some(value);
                            }
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Starting time balance in hours")
            .child(self.settings_inputs.opening_balance.clone())
    }

    fn schedule_settings(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let weekdays = self.settings_draft.expected_hours.working_weekdays.clone();
        let excluded = self.settings_draft.expected_hours.exclude_public_holidays;
        div()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(div().text_size(px(18.0)).child("Schedule"))
            .child("Target hours per workday")
            .child(self.settings_inputs.expected_hours.clone())
            .child("Scheduled weekdays")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("Mon", 1_u32),
                        ("Tue", 2),
                        ("Wed", 3),
                        ("Thu", 4),
                        ("Fri", 5),
                        ("Sat", 6),
                        ("Sun", 0),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, day))| {
                        setting_chip(("weekday", index), label, weekdays.contains(&day), colors)
                            .on_click(cx.listener(move |shell, _, _, cx| {
                                let days =
                                    &mut shell.settings_draft.expected_hours.working_weekdays;
                                if let Some(position) = days.iter().position(|value| *value == day)
                                {
                                    if days.len() > 1 {
                                        days.remove(position);
                                    }
                                } else {
                                    days.push(day);
                                    days.sort_unstable();
                                }
                                cx.notify();
                            }))
                    }),
                ),
            )
            .child(
                setting_chip(
                    "exclude-holidays",
                    "Exclude Swedish public holidays",
                    excluded,
                    colors,
                )
                .on_click(cx.listener(|shell, _, _, cx| {
                    shell.settings_draft.expected_hours.exclude_public_holidays =
                        !shell.settings_draft.expected_hours.exclude_public_holidays;
                    cx.notify();
                })),
            )
            .child("Default start")
            .child(self.settings_inputs.default_start.clone())
            .child("Default end")
            .child(self.settings_inputs.default_end.clone())
            .child("Default lunch minutes")
            .child(self.settings_inputs.default_lunch.clone())
    }

    fn overtime_settings(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let overtime = self.settings_draft.overtime_compensation.clone();
        div()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(div().text_size(px(18.0)).child("Overtime & OB"))
            .child("Compensation mode")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("Comp time", OvertimeCompensationMode::CompTime),
                        ("Direct salary", OvertimeCompensationMode::Paid),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("overtime-mode", index),
                            label,
                            overtime.mode == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.settings_draft.overtime_compensation.mode = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Threshold")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("Fixed daily hours", OvertimeThresholdMode::FixedDailyHours),
                        ("Scheduled hours", OvertimeThresholdMode::ScheduledHours),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("threshold-mode", index),
                            label,
                            overtime.threshold_mode == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.settings_draft.overtime_compensation.threshold_mode = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Fixed daily threshold")
            .child(self.settings_inputs.overtime_threshold.clone())
            .child("OB during overtime")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("Exclude", ObOvertimeCombinationMode::ExcludeOb),
                        ("Include", ObOvertimeCombinationMode::IncludeOb),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("ob-combination", index),
                            label,
                            overtime.ob_overtime_combination == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell
                                .settings_draft
                                .overtime_compensation
                                .ob_overtime_combination = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Default paid-overtime rate")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        (
                            "Premium percent",
                            CompensationRateType::HourlyPremiumPercent,
                        ),
                        ("Fixed amount", CompensationRateType::FixedHourlyAmount),
                        (
                            "Monthly divisor",
                            CompensationRateType::FullTimeMonthlySalaryDivisor,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("default-rate", index),
                            label,
                            overtime.default_rate_type == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.settings_draft.overtime_compensation.default_rate_type = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child(self.settings_inputs.default_rate_value.clone())
            .child(
                div()
                    .id("add-overtime-rule")
                    .h(px(38.0))
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .rounded(px(19.0))
                    .cursor_pointer()
                    .bg(colors.secondary_container)
                    .child("Add overtime rule")
                    .on_click(cx.listener(|shell, _, _, cx| {
                        shell.add_rate_band(CompensationRuleType::Overtime);
                        cx.notify();
                    })),
            )
            .child(
                div()
                    .id("add-ob-rule")
                    .h(px(38.0))
                    .px(px(14.0))
                    .flex()
                    .items_center()
                    .rounded(px(19.0))
                    .cursor_pointer()
                    .bg(colors.secondary_container)
                    .child("Add OB rule")
                    .on_click(cx.listener(|shell, _, _, cx| {
                        shell.add_rate_band(CompensationRuleType::Ob);
                        cx.notify();
                    })),
            )
            .children(
                overtime
                    .rate_bands
                    .into_iter()
                    .enumerate()
                    .map(|(index, band)| {
                        div()
                            .p(px(14.0))
                            .flex()
                            .items_center()
                            .gap(px(12.0))
                            .rounded(px(12.0))
                            .bg(colors.surface_container)
                            .child(div().flex_1().child(format!(
                                "{} - {:?}, {:?}, {}-{}, {:?} {}",
                                band.name,
                                band.compensation_type,
                                band.day_category,
                                band.start_time,
                                band.end_time,
                                band.rate_type,
                                band.rate_value
                            )))
                            .child(
                                div()
                                    .id(("cycle-band-day", index))
                                    .cursor_pointer()
                                    .child("Next day category")
                                    .on_click(cx.listener(move |shell, _, _, cx| {
                                        shell.cycle_band_day(index);
                                        cx.notify();
                                    })),
                            )
                            .child(
                                div()
                                    .id(("remove-band", index))
                                    .cursor_pointer()
                                    .text_color(colors.error)
                                    .child("Remove")
                                    .on_click(cx.listener(move |shell, _, _, cx| {
                                        if index
                                            < shell
                                                .settings_draft
                                                .overtime_compensation
                                                .rate_bands
                                                .len()
                                        {
                                            shell
                                                .settings_draft
                                                .overtime_compensation
                                                .rate_bands
                                                .remove(index);
                                        }
                                        cx.notify();
                                    })),
                            )
                    }),
            )
    }

    fn salary_tax_settings(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let salary = self.settings_draft.salary.clone();
        let tax = self.settings_draft.tax_settings.clone();
        div()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(div().text_size(px(18.0)).child("Salary & Tax"))
            .child("Salary model")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("Hourly", SalaryType::Hourly),
                        ("Monthly", SalaryType::Monthly),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("salary-type", index),
                            label,
                            salary.salary_type == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.settings_draft.salary.salary_type = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Hourly rate")
            .child(self.settings_inputs.hourly_rate.clone())
            .child("Monthly salary")
            .child(self.settings_inputs.monthly_salary.clone())
            .child("Employment percent")
            .child(self.settings_inputs.employment_percent.clone())
            .child("Hourly pay basis")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("Regular hours per day", HourlyPayBasis::DailyRegularHours),
                        (
                            "Monthly expected hours",
                            HourlyPayBasis::MonthlyExpectedHours,
                        ),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("hourly-basis", index),
                            label,
                            salary.hourly_pay_basis == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.settings_draft.salary.hourly_pay_basis = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Tax mode")
            .child(
                div().flex().flex_wrap().gap(px(8.0)).children(
                    [
                        ("Disabled", TaxMode::Disabled),
                        ("Primary table", TaxMode::PrimaryIncomeTaxTable),
                        ("Secondary 30%", TaxMode::SecondaryIncomeThirtyPercent),
                        ("Manual", TaxMode::ManualMonthlyDeduction),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(("tax-mode", index), label, tax.mode == value, colors)
                            .on_click(cx.listener(move |shell, _, _, cx| {
                                shell.settings_draft.tax_settings.mode = value;
                                cx.notify();
                            }))
                    }),
                ),
            )
            .child("Tax year")
            .child(self.settings_inputs.tax_year.clone())
            .child("Tax table")
            .child(self.settings_inputs.tax_table.clone())
            .child("Tax column 1-6")
            .child(self.settings_inputs.tax_column.clone())
            .child("Manual monthly deduction")
            .child(self.settings_inputs.manual_tax.clone())
    }

    fn application_settings(&mut self, colors: M3ColorScheme, cx: &mut Context<Self>) -> gpui::Div {
        let preferences = self.preferences_draft.clone();
        let export_language = self.settings_draft.export_language_preference;
        div()
            .flex()
            .flex_col()
            .gap(px(14.0))
            .child(div().text_size(px(18.0)).child("Application"))
            .child("Theme")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("System", ThemePreference::System),
                        ("Light", ThemePreference::Light),
                        ("Dark", ThemePreference::Dark),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("theme-setting", index),
                            label,
                            preferences.theme_preference == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.preferences_draft.theme_preference = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Language")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("System", LanguagePreference::System),
                        ("English", LanguagePreference::English),
                        ("Swedish", LanguagePreference::Swedish),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("language-setting", index),
                            label,
                            preferences.language_preference == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.preferences_draft.language_preference = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Interface scale")
            .child(
                div().flex().gap(px(8.0)).children(
                    [80, 90, 100, 110, 125, 150]
                        .into_iter()
                        .enumerate()
                        .map(|(index, value)| {
                            setting_chip(
                                ("scale-setting", index),
                                format!("{value}%"),
                                preferences.interface_scale_percent == value,
                                colors,
                            )
                            .on_click(cx.listener(
                                move |shell, _, _, cx| {
                                    shell.preferences_draft.interface_scale_percent = value;
                                    cx.notify();
                                },
                            ))
                        }),
                ),
            )
            .child("Export language")
            .child(
                div().flex().gap(px(8.0)).children(
                    [
                        ("System", ExportLanguagePreference::System),
                        ("English", ExportLanguagePreference::English),
                        ("Swedish", ExportLanguagePreference::Swedish),
                    ]
                    .into_iter()
                    .enumerate()
                    .map(|(index, (label, value))| {
                        setting_chip(
                            ("export-language", index),
                            label,
                            export_language == value,
                            colors,
                        )
                        .on_click(cx.listener(move |shell, _, _, cx| {
                            shell.settings_draft.export_language_preference = value;
                            cx.notify();
                        }))
                    }),
                ),
            )
            .child("Updates are unavailable in development builds.")
            .child(
                div()
                    .id("open-data-backups")
                    .h(px(40.0))
                    .flex()
                    .items_center()
                    .cursor_pointer()
                    .text_color(colors.primary)
                    .child("Data & Backups")
                    .on_click(
                        cx.listener(|shell, _, _, cx| shell.set_route(Route::DataBackups, cx)),
                    ),
            )
    }

    fn add_rate_band(&mut self, compensation_type: CompensationRuleType) {
        self.settings_draft
            .overtime_compensation
            .rate_bands
            .push(OvertimeRateBand {
                name: if compensation_type == CompensationRuleType::Overtime {
                    "Overtime".to_owned()
                } else {
                    "Evening OB".to_owned()
                },
                compensation_type,
                day_category: OvertimeDayCategory::ScheduledWorkdays,
                start_time: "18:00".parse().unwrap_or_else(|_| unreachable!()),
                end_time: "22:00".parse().unwrap_or_else(|_| unreachable!()),
                rate_type: CompensationRateType::HourlyPremiumPercent,
                rate_value: Decimal::from(50),
            });
    }

    fn cycle_band_day(&mut self, index: usize) {
        let Some(band) = self
            .settings_draft
            .overtime_compensation
            .rate_bands
            .get_mut(index)
        else {
            return;
        };
        let next = (i32::from(band.day_category) + 1) % 14;
        if let Ok(category) = OvertimeDayCategory::try_from(i64::from(next)) {
            band.day_category = category;
        }
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
        let workspace_delete = self.confirm_workspace_delete.clone();
        let pending_currency = self.pending_currency;
        let restore = self.confirm_restore.clone();
        let tidverk_import = self.confirm_import.clone();

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
                            .id("manage-workspaces")
                            .h(px(64.0))
                            .mx(px(12.0))
                            .px(px(16.0))
                            .flex()
                            .items_center()
                            .rounded(px(16.0))
                            .cursor_pointer()
                            .bg(colors.surface_container)
                            .when(!self.sidebar_collapsed, |item| item.child(workspace_name))
                            .on_click(cx.listener(|shell, _, _, cx| {
                                shell.manage_workspaces = true;
                                cx.notify();
                            })),
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
            .when(self.manage_workspaces, |root| {
                root.child(self.workspace_dialog(colors, cx))
            })
            .when_some(workspace_delete, |root, workspace_id| {
                root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::black().opacity(0.55))
                        .child(
                            div()
                                .w(px(460.0))
                                .p(px(24.0))
                                .flex()
                                .flex_col()
                                .gap(px(18.0))
                                .rounded(px(24.0))
                                .bg(colors.surface_container_high)
                                .child(div().text_size(px(20.0)).child("Delete workspace?"))
                                .child("Entries, projects, settings, and month records will be removed.")
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(16.0))
                                        .child(
                                            div()
                                                .id("cancel-workspace-delete")
                                                .cursor_pointer()
                                                .child("Cancel")
                                                .on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.confirm_workspace_delete = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("confirm-workspace-delete")
                                                .cursor_pointer()
                                                .text_color(colors.error)
                                                .child("Delete")
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    shell.delete_workspace(&workspace_id, cx)
                                                })),
                                        ),
                                ),
                        ),
                )
            })
            .when_some(pending_currency, |root, currency| {
                root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::black().opacity(0.55))
                        .child(
                            div()
                                .w(px(440.0))
                                .p(px(24.0))
                                .flex()
                                .flex_col()
                                .gap(px(18.0))
                                .rounded(px(24.0))
                                .bg(colors.surface_container_high)
                                .child(div().text_size(px(20.0)).child("Change currency?"))
                                .child("Dagsverk will not convert existing rates or report values.")
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(16.0))
                                        .child(
                                            div()
                                                .id("cancel-currency")
                                                .cursor_pointer()
                                                .child("Cancel")
                                                .on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.pending_currency = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("confirm-currency")
                                                .cursor_pointer()
                                                .text_color(colors.primary)
                                                .child("Change")
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    shell.settings_draft.currency_preference = currency;
                                                    shell.pending_currency = None;
                                                    cx.notify();
                                                })),
                                        ),
                                ),
                    ),
                )
            })
            .when_some(restore, |root, path| {
                let selected = path.clone();
                root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::black().opacity(0.55))
                        .child(
                            div()
                                .w(px(480.0))
                                .p(px(24.0))
                                .flex()
                                .flex_col()
                                .gap(px(18.0))
                                .rounded(px(24.0))
                                .bg(colors.surface_container_high)
                                .child(div().text_size(px(20.0)).child("Restore database?"))
                                .child("Current data will be backed up before replacement.")
                                .child(
                                    div()
                                        .text_color(colors.on_surface_variant)
                                        .child(path.display().to_string()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(16.0))
                                        .child(
                                            div()
                                                .id("cancel-restore")
                                                .cursor_pointer()
                                                .child("Cancel")
                                                .on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.confirm_restore = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("confirm-restore")
                                                .cursor_pointer()
                                                .text_color(colors.error)
                                                .child("Restore")
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    shell.run_restore(selected.clone(), cx)
                                                })),
                                        ),
                                ),
                        ),
                )
            })
            .when_some(tidverk_import, |root, path| {
                let selected = path.clone();
                root.child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .bg(gpui::black().opacity(0.55))
                        .child(
                            div()
                                .w(px(480.0))
                                .p(px(24.0))
                                .flex()
                                .flex_col()
                                .gap(px(18.0))
                                .rounded(px(24.0))
                                .bg(colors.surface_container_high)
                                .child(div().text_size(px(20.0)).child("Import Tidverk data?"))
                                .child("Dagsverk will create safety backups before import.")
                                .child(
                                    div()
                                        .text_color(colors.on_surface_variant)
                                        .child(path.display().to_string()),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .justify_end()
                                        .gap(px(16.0))
                                        .child(
                                            div()
                                                .id("cancel-tidverk-import")
                                                .cursor_pointer()
                                                .child("Cancel")
                                                .on_click(cx.listener(|shell, _, _, cx| {
                                                    shell.confirm_import = None;
                                                    cx.notify();
                                                })),
                                        )
                                        .child(
                                            div()
                                                .id("confirm-tidverk-import")
                                                .cursor_pointer()
                                                .text_color(colors.error)
                                                .child("Import")
                                                .on_click(cx.listener(move |shell, _, _, cx| {
                                                    shell.run_tidverk_import(selected.clone(), cx)
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

fn nonempty(value: &str) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

fn setting_chip(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    selected: bool,
    colors: M3ColorScheme,
) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(36.0))
        .px(px(12.0))
        .flex()
        .items_center()
        .rounded(px(18.0))
        .cursor_pointer()
        .bg(if selected {
            colors.secondary_container
        } else {
            colors.surface_container
        })
        .child(label.into())
}

fn maintenance_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    enabled: bool,
    colors: M3ColorScheme,
) -> Stateful<gpui::Div> {
    div()
        .id(id)
        .h(px(40.0))
        .px(px(18.0))
        .flex()
        .items_center()
        .rounded(px(20.0))
        .bg(colors.secondary_container)
        .text_color(colors.on_secondary_container)
        .opacity(if enabled { 1.0 } else { 0.38 })
        .when(enabled, |button| button.cursor_pointer())
        .child(label.into())
}

fn parse_non_negative_decimal(value: &str) -> Result<Decimal, &'static str> {
    let value = value
        .trim()
        .parse::<Decimal>()
        .map_err(|_| "Enter a valid non-negative number.")?;
    if value.is_sign_negative() {
        Err("Enter a valid non-negative number.")
    } else {
        Ok(value)
    }
}

fn parse_non_negative_i64(value: &str) -> Result<i64, &'static str> {
    let value = value
        .trim()
        .parse::<i64>()
        .map_err(|_| "Enter a valid non-negative whole number.")?;
    if value < 0 {
        Err("Enter a valid non-negative whole number.")
    } else {
        Ok(value)
    }
}

fn parse_i32(value: &str) -> Result<i32, &'static str> {
    value
        .trim()
        .parse::<i32>()
        .map_err(|_| "Enter a valid whole number.")
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{DateTime, Utc};
    use dagsverk_core::{clock::FixedClock, models::MonthViewPreference, tax::TaxEngine};
    use dagsverk_data::Database;
    use gpui::TestAppContext;
    use tempfile::tempdir;

    use super::{AppShell, AppShellServices, parse_non_negative_decimal, parse_scheduled_minutes};
    use crate::{
        platform::{NativeFileDialogService, NativeShellService},
        state::AppModel,
    };

    #[gpui::test]
    fn shell_shortcuts_and_background_backup_work(cx: &mut TestAppContext) {
        let directory = tempdir().expect("temporary data directory");
        let now = DateTime::parse_from_rfc3339("2026-08-18T10:00:00Z")
            .expect("fixed time")
            .with_timezone(&Utc);
        let clock = FixedClock::new(now);
        let repository = Arc::new(
            Database::open(directory.path().join("dagsverk.db"), clock)
                .expect("temporary database"),
        );
        let mut model = AppModel::new(
            repository.clone(),
            Arc::new(clock),
            TaxEngine::default(),
            false,
        );
        model.initialize().expect("application state");
        let services = AppShellServices {
            data: repository,
            file_dialog: Arc::new(NativeFileDialogService),
            shell: Arc::new(NativeShellService),
        };

        cx.update(AppShell::register_key_bindings);
        let (shell, cx) =
            cx.add_window_view(|window, cx| AppShell::new(model, services, window, cx));
        cx.simulate_keystrokes("ctrl-2");
        assert_eq!(
            shell.read_with(cx, |shell, _| shell.model.active_view),
            MonthViewPreference::Calendar
        );

        shell.update(cx, |shell, cx| shell.create_backup(cx));
        cx.run_until_parked();
        assert!(shell.read_with(cx, |shell, _| {
            !shell.maintenance_busy && shell.last_backup.as_ref().is_some_and(|path| path.exists())
        }));
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
        assert_eq!(
            parse_non_negative_decimal("123.45").expect("decimal value"),
            "123.45".parse().expect("expected decimal")
        );
        assert!(parse_non_negative_decimal("-0.01").is_err());
    }
}
