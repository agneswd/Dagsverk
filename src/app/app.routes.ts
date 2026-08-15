import { Routes } from '@angular/router';
import { MonthWorkspaceComponent } from './features/month-workspace/month-workspace.component';
import { ProjectsComponent } from './features/projects/projects.component';
import { SettingsComponent } from './features/settings/settings.component';
import { BackupsComponent } from './features/backups/backups.component';

export const routes: Routes = [
  { path: '', redirectTo: 'timesheet', pathMatch: 'full' },
  { path: 'timesheet', component: MonthWorkspaceComponent },
  { path: 'workspaces', redirectTo: 'timesheet' },
  { path: 'projects', component: ProjectsComponent },
  { path: 'settings', component: SettingsComponent },
  { path: 'backups', component: BackupsComponent },
  { path: '**', redirectTo: 'timesheet' },
];
