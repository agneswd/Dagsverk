use dagsverk_core::{
    clock::Clock,
    models::{
        AppPreferences, AppSettings, BalanceHistoryMonth, IsoDate, Minutes, MonthRecord, Project,
        ProjectId, WorkEntry, Workspace, WorkspaceId, YearMonth,
    },
};

use crate::{Database, Result};

pub trait DagsverkRepository: Send + Sync {
    fn list_workspaces(&self) -> Result<Vec<Workspace>>;
    fn save_workspace(&self, workspace: &Workspace) -> Result<()>;
    fn delete_workspace(&self, id: &WorkspaceId) -> Result<()>;
    fn load_preferences(&self) -> Result<AppPreferences>;
    fn save_preferences(&self, preferences: &AppPreferences) -> Result<()>;
    fn load_settings(&self, workspace: &WorkspaceId) -> Result<AppSettings>;
    fn save_settings(&self, workspace: &WorkspaceId, settings: &AppSettings) -> Result<()>;
    fn load_entries(&self, workspace: &WorkspaceId, month: YearMonth) -> Result<Vec<WorkEntry>>;
    fn save_entry(&self, workspace: &WorkspaceId, entry: &WorkEntry) -> Result<()>;
    fn save_entries(&self, workspace: &WorkspaceId, entries: &[WorkEntry]) -> Result<()>;
    fn delete_entry(&self, workspace: &WorkspaceId, date: IsoDate) -> Result<()>;
    fn load_month_record(
        &self,
        workspace: &WorkspaceId,
        month: YearMonth,
        default_opening: Minutes,
    ) -> Result<MonthRecord>;
    fn save_month_record(&self, workspace: &WorkspaceId, record: &MonthRecord) -> Result<()>;
    fn reset_month(&self, workspace: &WorkspaceId, month: YearMonth) -> Result<()>;
    fn load_balance_history(
        &self,
        workspace: &WorkspaceId,
        before: YearMonth,
    ) -> Result<Vec<BalanceHistoryMonth>>;
    fn list_projects(&self, workspace: &WorkspaceId) -> Result<Vec<Project>>;
    fn save_project(&self, workspace: &WorkspaceId, project: &Project) -> Result<()>;
    fn delete_project(&self, workspace: &WorkspaceId, id: &ProjectId) -> Result<()>;
}

impl<C: Clock> DagsverkRepository for Database<C> {
    fn list_workspaces(&self) -> Result<Vec<Workspace>> {
        Database::list_workspaces(self)
    }

    fn save_workspace(&self, workspace: &Workspace) -> Result<()> {
        Database::save_workspace(self, workspace)
    }

    fn delete_workspace(&self, id: &WorkspaceId) -> Result<()> {
        Database::delete_workspace(self, id)
    }

    fn load_preferences(&self) -> Result<AppPreferences> {
        Database::load_preferences(self)
    }

    fn save_preferences(&self, preferences: &AppPreferences) -> Result<()> {
        Database::save_preferences(self, preferences)
    }

    fn load_settings(&self, workspace: &WorkspaceId) -> Result<AppSettings> {
        Database::load_settings(self, workspace)
    }

    fn save_settings(&self, workspace: &WorkspaceId, settings: &AppSettings) -> Result<()> {
        Database::save_settings(self, workspace, settings)
    }

    fn load_entries(&self, workspace: &WorkspaceId, month: YearMonth) -> Result<Vec<WorkEntry>> {
        Database::load_entries(self, workspace, month)
    }

    fn save_entry(&self, workspace: &WorkspaceId, entry: &WorkEntry) -> Result<()> {
        Database::save_entry(self, workspace, entry)
    }

    fn save_entries(&self, workspace: &WorkspaceId, entries: &[WorkEntry]) -> Result<()> {
        Database::save_entries(self, workspace, entries)
    }

    fn delete_entry(&self, workspace: &WorkspaceId, date: IsoDate) -> Result<()> {
        Database::delete_entry(self, workspace, date)
    }

    fn load_month_record(
        &self,
        workspace: &WorkspaceId,
        month: YearMonth,
        default_opening: Minutes,
    ) -> Result<MonthRecord> {
        Database::load_month_record(self, workspace, month, default_opening)
    }

    fn save_month_record(&self, workspace: &WorkspaceId, record: &MonthRecord) -> Result<()> {
        Database::save_month_record(self, workspace, record)
    }

    fn reset_month(&self, workspace: &WorkspaceId, month: YearMonth) -> Result<()> {
        Database::reset_month(self, workspace, month)
    }

    fn load_balance_history(
        &self,
        workspace: &WorkspaceId,
        before: YearMonth,
    ) -> Result<Vec<BalanceHistoryMonth>> {
        Database::load_balance_history(self, workspace, before)
    }

    fn list_projects(&self, workspace: &WorkspaceId) -> Result<Vec<Project>> {
        Database::list_projects(self, workspace)
    }

    fn save_project(&self, workspace: &WorkspaceId, project: &Project) -> Result<()> {
        Database::save_project(self, workspace, project)
    }

    fn delete_project(&self, workspace: &WorkspaceId, id: &ProjectId) -> Result<()> {
        Database::delete_project(self, workspace, id)
    }
}
