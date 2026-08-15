import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { AppStateService } from '../../core/app-state.service';
import { Workspace, WorkspaceType } from '../../core/models';

@Component({
  selector: 'app-workspaces',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
    MatTooltipModule,
    MatSnackBarModule
  ],
  templateUrl: './workspaces.component.html',
  styleUrls: ['./workspaces.component.scss']
})
export class WorkspacesComponent {
  public state = inject(AppStateService);
  private snackBar = inject(MatSnackBar);

  public newWorkspaceName = signal<string>('');
  public newWorkspaceType = signal<WorkspaceType>(WorkspaceType.Employment);
  public newOrganizationName = signal<string>('');
  public newWorkerName = signal<string>('');
  public newWorkspaceColor = signal<string>('#5F875F');
  public isAdding = signal<boolean>(false);
  public WorkspaceType = WorkspaceType;

  public availableColors = [
    '#5F875F', // Dagsverk green
    '#0B57D0', // Blue
    '#00838F', // Teal
    '#2E7D32', // Green
    '#ED6C02', // Orange
    '#C2185B', // Pink
    '#7B1FA2', // Purple
    '#5C6BC0', // Indigo
    '#455A64'  // Slate
  ];

  public async onAddWorkspace(): Promise<void> {
    const name = this.newWorkspaceName().trim();
    if (!name) return;

    const newWs: Workspace = {
      id: `ws-${Date.now()}`,
      name,
      type: this.newWorkspaceType(),
      organizationName: this.newOrganizationName().trim() || undefined,
      workerName: this.newWorkerName().trim() || undefined,
      color: this.newWorkspaceColor(),
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString()
    };

    await this.state.saveWorkspace(newWs);
    this.newWorkspaceName.set('');
    this.newWorkspaceType.set(WorkspaceType.Employment);
    this.newOrganizationName.set('');
    this.newWorkerName.set('');
    this.isAdding.set(false);
    this.snackBar.open(`Workspace "${newWs.name}" created`, 'Close', { duration: 3000 });
  }

  public onSelectWorkspace(ws: Workspace): void {
    this.state.switchWorkspace(ws.id);
  }

  public workspaceSubtitle(workspace: Workspace): string {
    if (workspace.type === WorkspaceType.Personal) return workspace.workerName || 'Personal workspace';
    return workspace.organizationName || (workspace.type === WorkspaceType.Contract ? 'Independent contract' : 'Employment');
  }

  public async onDeleteWorkspace(ws: Workspace): Promise<void> {
    if (this.state.workspaces().length <= 1) {
      this.snackBar.open('Cannot delete the only remaining workspace', 'Close', { duration: 3000 });
      return;
    }

    if (typeof confirm !== 'undefined' && !confirm(`Are you sure you want to delete workspace "${ws.name}"? All associated entries, projects, and settings will be permanently removed.`)) {
      return;
    }

    await this.state.deleteWorkspace(ws.id);
    this.snackBar.open(`Workspace "${ws.name}" deleted`, 'Close', { duration: 3000 });
  }
}
