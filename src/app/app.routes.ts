import { Routes } from '@angular/router';
import { MonthWorkspaceComponent } from './features/month-workspace/month-workspace.component';
import { ProjectsComponent } from './features/projects/projects.component';
import { ProfileSettingsComponent } from './features/profiles/profile-settings.component';
import { SettingsComponent } from './features/settings/settings.component';

export const routes: Routes = [
  { path: '', redirectTo: 'timesheet', pathMatch: 'full' },
  { path: 'timesheet', component: MonthWorkspaceComponent },
  { path: 'workspaces', redirectTo: 'timesheet' },
  { path: 'projects', component: ProjectsComponent },
  { path: 'profile', component: ProfileSettingsComponent },
  { path: 'settings', component: SettingsComponent },
  { path: 'settings/data-backups', redirectTo: 'settings' },
  { path: 'backups', redirectTo: 'settings' },
  { path: '**', redirectTo: 'timesheet' },
];
