use std::{collections::BTreeMap, sync::Arc};

use chrono::Datelike;
use dagsverk_core::{
    DomainError,
    calculations::{
        PasteMonthError, calculate_monthly_summary, estimate_opening_balance, expected_workdays,
        paste_month_entries, threshold_for_entry,
    },
    clock::Clock,
    holidays::SwedishHolidayCalendar,
    models::{
        AppPreferences, AppSettings, ExportLanguagePreference, IsoDate, LanguagePreference,
        Minutes, MonthRecord, MonthViewPreference, MonthlySummary, Project, ProjectId,
        ReportExportRequest, TaxEstimate, ThemePreference, UpdateState, UpdateStatus, WorkEntry,
        WorkEntryStatus, Workspace, WorkspaceId, WorkspaceType, YearMonth, default_preferences,
        default_settings, default_workspace,
    },
    tax::TaxEngine,
};
use dagsverk_data::{DagsverkRepository, DataError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Route {
    Timesheet,
    Projects,
    Settings,
    DataBackups,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTheme {
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Swedish,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct EditorState {
    pub is_open: bool,
    pub draft: Option<WorkEntry>,
    pub validation_error: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CatchUpSession {
    pub dates: Vec<IsoDate>,
    pub index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopiedMonth {
    pub workspace_id: WorkspaceId,
    pub month: YearMonth,
    pub title: String,
    pub entries: Vec<WorkEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InitializationState {
    NotStarted,
    Loading,
    Ready,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadKey {
    pub generation: u64,
    pub workspace_id: WorkspaceId,
    pub month: YearMonth,
}

#[derive(Debug)]
pub struct WorkspaceMonthData {
    pub settings: AppSettings,
    pub projects: Vec<Project>,
    pub month_record: MonthRecord,
    pub entries: Vec<WorkEntry>,
}

#[derive(Debug, thiserror::Error)]
pub enum AppStateError {
    #[error(transparent)]
    Data(#[from] DataError),
    #[error(transparent)]
    Domain(#[from] DomainError),
    #[error("the database contains no workspaces")]
    NoWorkspaces,
    #[error("workspace {0} does not exist")]
    UnknownWorkspace(String),
    #[error(transparent)]
    PasteMonth(#[from] PasteMonthError),
}

pub type Result<T> = std::result::Result<T, AppStateError>;

pub struct AppModel {
    pub route: Route,
    pub workspaces: Vec<Workspace>,
    pub active_workspace_id: WorkspaceId,
    pub preferences: AppPreferences,
    pub settings: AppSettings,
    pub projects: Vec<Project>,
    pub current_month: YearMonth,
    pub month_record: MonthRecord,
    pub entries: Vec<WorkEntry>,
    pub selected_date: Option<IsoDate>,
    pub editor: EditorState,
    pub active_view: MonthViewPreference,
    pub resolved_theme: ResolvedTheme,
    pub interface_scale: f32,
    pub language: Language,
    pub catch_up: Option<CatchUpSession>,
    pub copied_month: Option<CopiedMonth>,
    pub update_state: UpdateState,
    pub initialization: InitializationState,
    pub active_load_generation: u64,
    pub transient_error: Option<String>,
    repository: Arc<dyn DagsverkRepository>,
    holidays: SwedishHolidayCalendar,
    tax: TaxEngine,
    clock: Arc<dyn Clock>,
    system_dark: bool,
}

impl AppModel {
    pub fn new(
        repository: Arc<dyn DagsverkRepository>,
        clock: Arc<dyn Clock>,
        tax: TaxEngine,
        system_dark: bool,
    ) -> Self {
        let workspace = default_workspace(clock.as_ref());
        let preferences = default_preferences();
        let settings = default_settings();
        let today = clock.today();
        let current_month =
            YearMonth::new(today.year(), today.month()).unwrap_or_else(|_| unreachable!());
        let month_record = MonthRecord {
            workspace_id: Some(workspace.id.clone()),
            year: current_month.year,
            month: current_month.month,
            opening_balance_minutes: Minutes::ZERO,
            expected_minutes_override: None,
            opening_balance_was_edited: false,
        };
        let mut model = Self {
            route: Route::Timesheet,
            workspaces: vec![workspace.clone()],
            active_workspace_id: workspace.id,
            preferences,
            settings,
            projects: Vec::new(),
            current_month,
            month_record,
            entries: Vec::new(),
            selected_date: None,
            editor: EditorState::default(),
            active_view: MonthViewPreference::Ledger,
            resolved_theme: ResolvedTheme::Light,
            interface_scale: 1.0,
            language: Language::English,
            catch_up: None,
            copied_month: None,
            update_state: UpdateState {
                status: UpdateStatus::Unavailable,
                current_version: env!("CARGO_PKG_VERSION").to_owned(),
                available_version: None,
                progress: None,
                message: None,
            },
            initialization: InitializationState::NotStarted,
            active_load_generation: 0,
            transient_error: None,
            repository,
            holidays: SwedishHolidayCalendar,
            tax,
            clock,
            system_dark,
        };
        model.apply_preferences();
        model
    }

    pub fn initialize(&mut self) -> Result<()> {
        self.initialization = InitializationState::Loading;
        match self.try_initialize() {
            Ok(()) => {
                self.initialization = InitializationState::Ready;
                self.transient_error = None;
                Ok(())
            }
            Err(error) => {
                self.initialization = InitializationState::Failed;
                self.transient_error = Some(error.to_string());
                Err(error)
            }
        }
    }

    fn try_initialize(&mut self) -> Result<()> {
        let workspaces = self.repository.list_workspaces()?;
        let mut preferences = self.repository.load_preferences()?;
        let active = workspaces
            .iter()
            .find(|workspace| workspace.id == preferences.active_workspace_id)
            .or_else(|| workspaces.first())
            .ok_or(AppStateError::NoWorkspaces)?
            .id
            .clone();
        preferences.active_workspace_id = active.clone();
        self.workspaces = workspaces;
        self.active_workspace_id = active.clone();
        self.preferences = preferences;
        self.active_view = self.preferences.month_view_preference;
        self.apply_preferences();

        let key = self.begin_load(active, self.current_month);
        let data = self.load_for_key(&key)?;
        let _ = self.apply_load(&key, data);
        Ok(())
    }

    pub fn begin_load(&mut self, workspace_id: WorkspaceId, month: YearMonth) -> LoadKey {
        self.active_load_generation = self.active_load_generation.wrapping_add(1);
        LoadKey {
            generation: self.active_load_generation,
            workspace_id,
            month,
        }
    }

    pub fn load_for_key(&self, key: &LoadKey) -> Result<WorkspaceMonthData> {
        let settings = self.repository.load_settings(&key.workspace_id)?;
        let projects = self.repository.list_projects(&key.workspace_id)?;
        let history = self
            .repository
            .load_balance_history(&key.workspace_id, key.month)?;
        let opening = estimate_opening_balance(
            &history,
            settings.opening_balance_minutes,
            &settings.expected_hours,
            &settings.salary,
            &settings.overtime_compensation,
            self.holidays,
            IsoDate::new(self.clock.today()),
        );
        let month_record =
            self.repository
                .load_month_record(&key.workspace_id, key.month, opening)?;
        let entries = self.repository.load_entries(&key.workspace_id, key.month)?;
        Ok(WorkspaceMonthData {
            settings,
            projects,
            month_record,
            entries,
        })
    }

    pub fn apply_load(&mut self, key: &LoadKey, data: WorkspaceMonthData) -> bool {
        if key.generation != self.active_load_generation
            || key.workspace_id != self.active_workspace_id
            || key.month != self.current_month
        {
            return false;
        }
        self.settings = data.settings;
        self.projects = data.projects;
        self.month_record = data.month_record;
        self.entries = data.entries;
        true
    }

    pub fn reload_current_month(&mut self) -> Result<()> {
        let key = self.begin_load(self.active_workspace_id.clone(), self.current_month);
        let data = self.load_for_key(&key)?;
        let _ = self.apply_load(&key, data);
        Ok(())
    }

    pub fn switch_workspace(&mut self, workspace_id: &WorkspaceId) -> Result<()> {
        if &self.active_workspace_id == workspace_id {
            return Ok(());
        }
        if !self
            .workspaces
            .iter()
            .any(|workspace| &workspace.id == workspace_id)
        {
            return Err(AppStateError::UnknownWorkspace(
                workspace_id.as_str().to_owned(),
            ));
        }
        let mut preferences = self.preferences.clone();
        preferences.active_workspace_id = workspace_id.clone();
        self.repository.save_preferences(&preferences)?;
        self.preferences = preferences;
        self.active_workspace_id = workspace_id.clone();
        self.copied_month = None;
        self.close_catch_up();
        self.reload_current_month()
    }

    pub fn select_month(&mut self, month: YearMonth) -> LoadKey {
        self.current_month = month;
        self.close_editor();
        self.begin_load(self.active_workspace_id.clone(), month)
    }

    pub fn next_month(&mut self) -> LoadKey {
        let (year, month) = if self.current_month.month == 12 {
            (self.current_month.year + 1, 1)
        } else {
            (self.current_month.year, self.current_month.month + 1)
        };
        self.select_month(YearMonth::new(year, month).unwrap_or_else(|_| unreachable!()))
    }

    pub fn previous_month(&mut self) -> LoadKey {
        let (year, month) = if self.current_month.month == 1 {
            (self.current_month.year - 1, 12)
        } else {
            (self.current_month.year, self.current_month.month - 1)
        };
        self.select_month(YearMonth::new(year, month).unwrap_or_else(|_| unreachable!()))
    }

    pub fn go_to_today(&mut self) -> LoadKey {
        let today = IsoDate::new(self.clock.today());
        let month = YearMonth::new(today.as_naive_date().year(), today.as_naive_date().month())
            .unwrap_or_else(|_| unreachable!());
        let key = self.select_month(month);
        self.open_editor(today);
        key
    }

    pub fn open_editor(&mut self, date: IsoDate) {
        self.selected_date = Some(date);
        self.editor.is_open = true;
        self.editor.validation_error = None;
        self.editor.draft = self.selected_entry();
    }

    pub fn close_editor(&mut self) {
        self.selected_date = None;
        self.editor = EditorState::default();
    }

    pub fn start_month(&mut self) {
        let today = IsoDate::new(self.clock.today());
        let target = if self.current_month.contains(today) {
            today
        } else {
            expected_workdays(
                self.current_month.year,
                self.current_month.month,
                &self.settings.expected_hours,
                self.holidays,
            )
            .into_iter()
            .next()
            .unwrap_or_else(|| {
                format!(
                    "{}-{:02}-01",
                    self.current_month.year, self.current_month.month
                )
                .parse()
                .unwrap_or_else(|_| unreachable!())
            })
        };
        self.open_editor(target);
    }

    pub fn save_entry(&mut self, mut entry: WorkEntry) -> Result<()> {
        entry.workspace_id = Some(self.active_workspace_id.clone());
        self.repository
            .save_entry(&self.active_workspace_id, &entry)?;
        self.entries.retain(|current| current.date != entry.date);
        self.entries.push(entry);
        self.entries.sort_by_key(|current| current.date);
        Ok(())
    }

    pub fn delete_entry(&mut self, date: IsoDate) -> Result<()> {
        self.repository
            .delete_entry(&self.active_workspace_id, date)?;
        self.entries.retain(|entry| entry.date != date);
        self.close_editor();
        Ok(())
    }

    pub fn fillable_dates(&self) -> Vec<IsoDate> {
        let occupied: std::collections::BTreeSet<_> = self
            .entries
            .iter()
            .filter(|entry| entry.status != WorkEntryStatus::Incomplete)
            .map(|entry| entry.date)
            .collect();
        expected_workdays(
            self.current_month.year,
            self.current_month.month,
            &self.settings.expected_hours,
            self.holidays,
        )
        .into_iter()
        .filter(|date| !occupied.contains(date))
        .collect()
    }

    pub fn fill_normal_workdays(&mut self) -> Result<usize> {
        let entries: Vec<_> = self
            .fillable_dates()
            .into_iter()
            .map(|date| WorkEntry {
                workspace_id: Some(self.active_workspace_id.clone()),
                date,
                status: WorkEntryStatus::Worked,
                start_time: Some(self.settings.default_start_time),
                end_time: Some(self.settings.default_end_time),
                lunch_minutes: self.settings.default_lunch_minutes,
                project_name: Some(self.settings.default_project.clone()),
                notes: None,
                scheduled_minutes_override: None,
                created_at: None,
                updated_at: None,
            })
            .collect();
        if entries.is_empty() {
            return Ok(0);
        }
        self.repository
            .save_entries(&self.active_workspace_id, &entries)?;
        let count = entries.len();
        self.reload_current_month()?;
        Ok(count)
    }

    pub fn copy_month(&mut self) -> usize {
        let entries: Vec<_> = self
            .entries
            .iter()
            .filter(|entry| entry.status != WorkEntryStatus::Incomplete)
            .cloned()
            .collect();
        self.copied_month = Some(CopiedMonth {
            workspace_id: self.active_workspace_id.clone(),
            month: self.current_month,
            title: format!(
                "{}-{:02}",
                self.current_month.year, self.current_month.month
            ),
            entries,
        });
        self.copied_month
            .as_ref()
            .map_or(0, |copied| copied.entries.len())
    }

    pub fn can_paste_month(&self) -> bool {
        self.copied_month.as_ref().is_some_and(|copied| {
            copied.workspace_id == self.active_workspace_id && copied.month != self.current_month
        })
    }

    pub fn pasteable_entries(&self) -> Result<Vec<WorkEntry>> {
        let Some(copied) = &self.copied_month else {
            return Ok(Vec::new());
        };
        paste_month_entries(
            &copied.workspace_id,
            copied.month,
            &copied.entries,
            &self.active_workspace_id,
            self.current_month,
            &self.entries,
        )
        .map_err(Into::into)
    }

    pub fn paste_month(&mut self) -> Result<usize> {
        let entries = self.pasteable_entries()?;
        if entries.is_empty() {
            return Ok(0);
        }
        self.repository
            .save_entries(&self.active_workspace_id, &entries)?;
        let count = entries.len();
        self.reload_current_month()?;
        Ok(count)
    }

    pub fn reset_month(&mut self) -> Result<()> {
        self.repository
            .reset_month(&self.active_workspace_id, self.current_month)?;
        self.close_catch_up();
        self.reload_current_month()
    }

    pub fn save_month_record(&mut self, mut record: MonthRecord) -> Result<()> {
        record.workspace_id = Some(self.active_workspace_id.clone());
        self.repository
            .save_month_record(&self.active_workspace_id, &record)?;
        self.month_record = record;
        Ok(())
    }

    pub fn update_settings(&mut self, mut settings: AppSettings) -> Result<()> {
        settings.workspace_id = Some(self.active_workspace_id.clone());
        self.repository
            .save_settings(&self.active_workspace_id, &settings)?;
        self.settings = settings;
        Ok(())
    }

    pub fn update_preferences(&mut self, preferences: AppPreferences) -> Result<()> {
        self.repository.save_preferences(&preferences)?;
        self.preferences = preferences;
        self.active_view = self.preferences.month_view_preference;
        self.apply_preferences();
        Ok(())
    }

    pub fn set_view(&mut self, view: MonthViewPreference) -> Result<()> {
        let mut preferences = self.preferences.clone();
        preferences.month_view_preference = view;
        self.update_preferences(preferences)
    }

    pub fn toggle_theme(&mut self) -> Result<()> {
        let mut preferences = self.preferences.clone();
        preferences.theme_preference = match self.resolved_theme {
            ResolvedTheme::Light => ThemePreference::Dark,
            ResolvedTheme::Dark => ThemePreference::Light,
        };
        self.update_preferences(preferences)
    }

    pub fn save_project(&mut self, mut project: Project) -> Result<()> {
        project.workspace_id = Some(self.active_workspace_id.clone());
        self.repository
            .save_project(&self.active_workspace_id, &project)?;
        self.projects.retain(|current| current.id != project.id);
        self.projects.push(project);
        self.projects
            .sort_by(|left, right| left.name.cmp(&right.name));
        Ok(())
    }

    pub fn delete_project(&mut self, id: &ProjectId) -> Result<()> {
        self.repository
            .delete_project(&self.active_workspace_id, id)?;
        self.projects.retain(|project| &project.id != id);
        Ok(())
    }

    pub fn set_default_project(&mut self, id: &ProjectId) -> Result<()> {
        let mut projects = self.projects.clone();
        for project in &mut projects {
            project.is_default = &project.id == id;
        }
        self.repository
            .save_projects(&self.active_workspace_id, &projects)?;
        self.projects = projects;
        Ok(())
    }

    pub fn save_workspace(&mut self, workspace: Workspace) -> Result<()> {
        self.repository.save_workspace(&workspace)?;
        self.workspaces.retain(|current| current.id != workspace.id);
        self.workspaces.push(workspace);
        self.workspaces.sort_by_key(|current| current.created_at);
        Ok(())
    }

    pub fn create_workspace(
        &mut self,
        name: String,
        color: String,
        workspace_type: WorkspaceType,
        worker_name: Option<String>,
        organization_name: Option<String>,
    ) -> Result<WorkspaceId> {
        let id = WorkspaceId::new(format!("ws-{}", uuid::Uuid::new_v4()))?;
        let now = self.clock.now_utc();
        self.save_workspace(Workspace {
            id: id.clone(),
            name,
            color,
            workspace_type,
            worker_name,
            organization_name,
            created_at: now,
            updated_at: now,
        })?;
        Ok(id)
    }

    pub fn delete_workspace(&mut self, id: &WorkspaceId) -> Result<()> {
        self.repository.delete_workspace(id)?;
        self.workspaces.retain(|workspace| &workspace.id != id);
        if &self.active_workspace_id == id
            && let Some(next) = self
                .workspaces
                .first()
                .map(|workspace| workspace.id.clone())
        {
            self.switch_workspace(&next)?;
        }
        Ok(())
    }

    pub fn start_catch_up(&mut self) {
        let dates = self.summary().missing_past_days;
        if let Some(first) = dates.first().copied() {
            self.catch_up = Some(CatchUpSession { dates, index: 0 });
            self.open_editor(first);
        }
    }

    pub fn move_catch_up(&mut self, delta: isize) {
        let Some(session) = &mut self.catch_up else {
            return;
        };
        let next = session.index as isize + delta;
        if next < 0 {
            return;
        }
        if next as usize >= session.dates.len() {
            self.close_catch_up();
            return;
        }
        session.index = next as usize;
        let date = session.dates[session.index];
        self.open_editor(date);
    }

    pub fn close_catch_up(&mut self) {
        self.catch_up = None;
        self.close_editor();
    }

    pub fn active_workspace(&self) -> Option<&Workspace> {
        self.workspaces
            .iter()
            .find(|workspace| workspace.id == self.active_workspace_id)
    }

    pub fn selected_entry(&self) -> Option<WorkEntry> {
        let date = self.selected_date?;
        self.entries
            .iter()
            .find(|entry| entry.date == date)
            .cloned()
            .or_else(|| {
                Some(WorkEntry {
                    workspace_id: Some(self.active_workspace_id.clone()),
                    date,
                    status: WorkEntryStatus::Incomplete,
                    start_time: Some(self.settings.default_start_time),
                    end_time: Some(self.settings.default_end_time),
                    lunch_minutes: self.settings.default_lunch_minutes,
                    project_name: Some(self.settings.default_project.clone()),
                    notes: None,
                    scheduled_minutes_override: None,
                    created_at: None,
                    updated_at: None,
                })
            })
    }

    pub fn summary(&self) -> MonthlySummary {
        calculate_monthly_summary(
            &self.month_record,
            &self.entries,
            &self.settings.expected_hours,
            &self.settings.salary,
            &self.settings.overtime_compensation,
            self.holidays,
            IsoDate::new(self.clock.today()),
        )
    }

    pub fn tax_estimate(&self) -> TaxEstimate {
        self.tax
            .calculate(self.summary().gross_salary, &self.settings.tax_settings)
    }

    pub fn export_request(&self) -> ReportExportRequest {
        let workspace = self.active_workspace();
        let threshold_minutes_by_date: BTreeMap<_, _> = self
            .entries
            .iter()
            .map(|entry| {
                (
                    entry.date,
                    threshold_for_entry(
                        entry,
                        &self.settings.expected_hours,
                        &self.settings.overtime_compensation,
                        self.holidays,
                    ),
                )
            })
            .collect();
        let language = match self.settings.export_language_preference {
            ExportLanguagePreference::System => match self.language {
                Language::English => ExportLanguagePreference::English,
                Language::Swedish => ExportLanguagePreference::Swedish,
            },
            language => language,
        };
        ReportExportRequest {
            year: self.current_month.year,
            month: self.current_month.month,
            employee_name: workspace
                .and_then(|workspace| workspace.worker_name.clone())
                .filter(|name| !name.trim().is_empty())
                .unwrap_or_else(|| "Worker".to_owned()),
            employer_name: workspace
                .and_then(|workspace| workspace.organization_name.clone())
                .unwrap_or_default(),
            entries: self.entries.clone(),
            summary: self.summary(),
            language,
            expected_hours: Some(self.settings.expected_hours.clone()),
            overtime_settings: Some(self.settings.overtime_compensation.clone()),
            overtime_mode: self.settings.overtime_compensation.mode,
            daily_overtime_threshold_hours: self
                .settings
                .overtime_compensation
                .daily_threshold_hours,
            hourly_pay_basis: self.settings.salary.hourly_pay_basis,
            threshold_minutes_by_date,
        }
    }

    pub fn is_month_unstarted(&self) -> bool {
        !self
            .entries
            .iter()
            .any(|entry| entry.status != WorkEntryStatus::Incomplete)
    }

    pub fn can_reset_month(&self) -> bool {
        !self.entries.is_empty()
            || self.month_record.opening_balance_was_edited
            || self.month_record.expected_minutes_override.is_some()
    }

    pub fn missing_days_count(&self) -> usize {
        self.summary().missing_past_days.len()
    }

    pub fn today(&self) -> IsoDate {
        IsoDate::new(self.clock.today())
    }

    fn apply_preferences(&mut self) {
        self.resolved_theme = match self.preferences.theme_preference {
            ThemePreference::Dark => ResolvedTheme::Dark,
            ThemePreference::Light => ResolvedTheme::Light,
            ThemePreference::System if self.system_dark => ResolvedTheme::Dark,
            ThemePreference::System => ResolvedTheme::Light,
        };
        self.interface_scale = self.preferences.interface_scale_percent as f32 / 100.0;
        self.language = match self.preferences.language_preference {
            LanguagePreference::Swedish => Language::Swedish,
            LanguagePreference::English | LanguagePreference::System => Language::English,
        };
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use chrono::{DateTime, Utc};
    use dagsverk_core::{
        clock::FixedClock,
        models::{
            IsoDate, Money, MonthViewPreference, Project, ProjectId, WorkspaceType, YearMonth,
        },
        tax::TaxEngine,
    };
    use dagsverk_data::Database;
    use tempfile::TempDir;

    use super::{AppModel, InitializationState};

    const TAX_DATA: &str = include_str!("../../../../../public/tax-data/tax-2026.json");

    fn model() -> (TempDir, AppModel) {
        let directory = tempfile::tempdir().expect("temporary data directory");
        let now = DateTime::parse_from_rfc3339("2026-08-18T10:00:00Z")
            .expect("fixed timestamp")
            .with_timezone(&Utc);
        let clock = Arc::new(FixedClock::new(now));
        let repository = Arc::new(
            Database::open(directory.path().join("dagsverk.db"), *clock)
                .expect("temporary database"),
        );
        let mut tax = TaxEngine::default();
        tax.register_json(TAX_DATA).expect("tax fixture");
        (directory, AppModel::new(repository, clock, tax, false))
    }

    #[test]
    fn initialization_loads_the_active_workspace_and_current_month() {
        let (_directory, mut model) = model();
        model.initialize().expect("initialize state");

        assert_eq!(model.initialization, InitializationState::Ready);
        assert_eq!(model.active_workspace_id.as_str(), "ws-default");
        assert_eq!(model.current_month, YearMonth::new(2026, 8).expect("month"));
        assert_eq!(model.active_view, MonthViewPreference::Ledger);
        assert!(!model.preferences.has_completed_setup);
        assert!(model.is_month_unstarted());
        assert_eq!(model.tax_estimate().gross_pay, Money::ZERO);
    }

    #[test]
    fn stale_month_results_are_rejected() {
        let (_directory, mut model) = model();
        model.initialize().expect("initialize state");
        let old_key = model.begin_load(model.active_workspace_id.clone(), model.current_month);
        let old_data = model.load_for_key(&old_key).expect("old month data");

        let new_key = model.next_month();
        assert_eq!(new_key.month, YearMonth::new(2026, 9).expect("month"));
        assert!(!model.apply_load(&old_key, old_data));
        assert_eq!(model.current_month, new_key.month);
    }

    #[test]
    fn month_navigation_and_editor_defaults_follow_the_fixed_clock() {
        let (_directory, mut model) = model();
        model.initialize().expect("initialize state");

        model.select_month(YearMonth::new(2026, 1).expect("month"));
        assert_eq!(
            model.previous_month().month,
            YearMonth::new(2025, 12).expect("month")
        );
        let today_key = model.go_to_today();
        assert_eq!(today_key.month, YearMonth::new(2026, 8).expect("month"));
        assert_eq!(
            model.selected_date,
            Some("2026-08-18".parse::<IsoDate>().expect("date"))
        );
        let draft = model.editor.draft.as_ref().expect("editor draft");
        assert_eq!(draft.start_time, Some(model.settings.default_start_time));
        assert_eq!(draft.project_name.as_deref(), Some("General"));
    }

    #[test]
    fn month_workflows_persist_and_reload_from_the_repository() {
        let (_directory, mut model) = model();
        model.initialize().expect("initialize state");

        let filled = model.fill_normal_workdays().expect("fill month");
        assert!(filled > 0);
        assert_eq!(model.entries.len(), filled);
        assert_eq!(model.copy_month(), filled);

        let key = model.select_month(YearMonth::new(2026, 9).expect("month"));
        let data = model.load_for_key(&key).expect("target month");
        assert!(model.apply_load(&key, data));
        let pasted = model.paste_month().expect("paste month");
        assert!(pasted > 0);
        assert!(model.can_reset_month());
        model.reset_month().expect("reset month");
        assert!(model.entries.is_empty());

        let key = model.select_month(YearMonth::new(2026, 8).expect("month"));
        let data = model.load_for_key(&key).expect("source month");
        assert!(model.apply_load(&key, data));
        model.reset_month().expect("reset source month");
        model.start_catch_up();
        assert!(model.catch_up.is_some());
        assert!(model.editor.is_open);
    }

    #[test]
    fn workspace_and_preference_mutations_follow_persistence() {
        let (_directory, mut model) = model();
        model.initialize().expect("initialize state");
        let workspace_id = model
            .create_workspace(
                "Second".to_owned(),
                "#5F875F".to_owned(),
                WorkspaceType::Contract,
                Some("Worker".to_owned()),
                Some("Client".to_owned()),
            )
            .expect("create workspace");
        let workspace = model
            .workspaces
            .iter()
            .find(|workspace| workspace.id == workspace_id)
            .expect("created workspace")
            .clone();
        model
            .switch_workspace(&workspace.id)
            .expect("switch workspace");
        assert_eq!(model.active_workspace_id, workspace.id);

        let original_theme = model.resolved_theme;
        model.toggle_theme().expect("toggle theme");
        assert_ne!(model.resolved_theme, original_theme);
        model
            .set_view(MonthViewPreference::Calendar)
            .expect("save view");
        assert_eq!(model.active_view, MonthViewPreference::Calendar);

        let first_project = Project {
            workspace_id: Some(workspace.id.clone()),
            id: ProjectId::new("first").expect("project id"),
            name: "First".to_owned(),
            color: Some("#5F875F".to_owned()),
            is_active: true,
            is_default: true,
        };
        model
            .save_project(first_project)
            .expect("save first project");
        let second_project = Project {
            workspace_id: Some(workspace.id.clone()),
            id: ProjectId::new("second").expect("project id"),
            name: "Second".to_owned(),
            color: Some("#5F875F".to_owned()),
            is_active: true,
            is_default: false,
        };
        model
            .save_project(second_project.clone())
            .expect("save project");
        model
            .set_default_project(&second_project.id)
            .expect("set default project");
        assert_eq!(
            model
                .projects
                .iter()
                .filter(|project| project.is_default)
                .count(),
            1
        );
        assert!(
            model
                .projects
                .iter()
                .find(|project| project.id == second_project.id)
                .is_some_and(|project| project.is_default)
        );

        model
            .delete_workspace(&workspace.id)
            .expect("delete active workspace");
        assert_eq!(model.active_workspace_id.as_str(), "ws-default");
        assert_eq!(model.workspaces.len(), 1);
    }
}
