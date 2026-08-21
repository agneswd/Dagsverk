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
import { MatDialog, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { firstValueFrom } from 'rxjs';
import { AppStateService } from '../../core/app-state.service';
import { Workspace, WorkspaceType } from '../../core/models';
import { ConfirmDialogComponent } from '../../core/confirm-dialog.component';
import { ColorPickerComponent } from '../../shared/color-picker/color-picker.component';
import { RenameDialogComponent } from '../../shared/rename-dialog/rename-dialog.component';

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
    MatSnackBarModule,
    MatDialogModule,
    ColorPickerComponent,
  ],
  templateUrl: './workspaces.component.html',
  styleUrls: ['./workspaces.component.scss'],
})
export class WorkspacesComponent {
  public state = inject(AppStateService);
  public dialogRef = inject(MatDialogRef<WorkspacesComponent>, { optional: true });
  private snackBar = inject(MatSnackBar);
  private dialog = inject(MatDialog);

  public newWorkspaceName = signal<string>('');
  public newWorkspaceType = signal<WorkspaceType>(WorkspaceType.Employment);
  public newOrganizationName = signal<string>('');
  public newWorkerName = signal<string>('');
  public newWorkspaceColor = signal<string>('#5F875F');
  public isAdding = signal<boolean>(false);
  public WorkspaceType = WorkspaceType;

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
      updatedAt: new Date().toISOString(),
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

  public onWorkspaceColorChange(ws: Workspace, color: string): void {
    void this.state.saveWorkspace({ ...ws, color, updatedAt: new Date().toISOString() });
  }

  public async onRenameWorkspace(ws: Workspace): Promise<void> {
    const name = await firstValueFrom(
      this.dialog
        .open(RenameDialogComponent, {
          width: '440px',
          data: { title: 'Rename workspace', label: 'New name', initialName: ws.name },
        })
        .afterClosed(),
    );
    if (name === undefined || name === ws.name) return;
    await this.state.saveWorkspace({ ...ws, name, updatedAt: new Date().toISOString() });
  }

  public workspaceSubtitle(workspace: Workspace): string {
    if (workspace.type === WorkspaceType.Personal)
      return workspace.workerName || 'Personal workspace';
    return (
      workspace.organizationName ||
      (workspace.type === WorkspaceType.Contract ? 'Independent contract' : 'Employment')
    );
  }

  public async onDeleteWorkspace(ws: Workspace): Promise<void> {
    if (this.state.workspaces().length <= 1) {
      this.snackBar.open('Cannot delete the only remaining workspace', 'Close', { duration: 3000 });
      return;
    }

    const confirmed = await firstValueFrom(
      this.dialog
        .open(ConfirmDialogComponent, {
          width: '440px',
          data: {
            title: 'Delete workspace?',
            message: `Delete "${ws.name}" and all its entries, projects, and settings?`,
            confirmLabel: 'Delete workspace',
            destructive: true,
          },
        })
        .afterClosed(),
    );
    if (!confirmed) return;

    await this.state.deleteWorkspace(ws.id);
    this.snackBar.open(`Workspace "${ws.name}" deleted`, 'Close', { duration: 3000 });
  }
}
