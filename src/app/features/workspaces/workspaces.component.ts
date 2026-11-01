import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { Router } from '@angular/router';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { MatDialog, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { firstValueFrom } from 'rxjs';
import { AppStateService } from '../../core/app-state.service';
import { Workspace } from '../../core/models';
import { ConfirmDialogComponent } from '../../core/confirm-dialog.component';
import { ProfileFormDialogComponent } from '../profiles/profile-form-dialog.component';

@Component({
  selector: 'app-workspaces',
  standalone: true,
  imports: [
    CommonModule,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatTooltipModule,
    MatSnackBarModule,
    MatDialogModule,
  ],
  templateUrl: './workspaces.component.html',
  styleUrls: ['./workspaces.component.scss'],
})
// Lists profiles. New profiles use the profile form; details are edited on the Profile page.
export class WorkspacesComponent {
  public state = inject(AppStateService);
  public dialogRef = inject(MatDialogRef<WorkspacesComponent>, { optional: true });
  private snackBar = inject(MatSnackBar);
  private dialog = inject(MatDialog);
  private router = inject(Router);

  public onNewProfile(): void {
    this.dialogRef?.close();
    this.dialog.open(ProfileFormDialogComponent, {
      data: { mode: 'create' },
      width: '640px',
      maxWidth: 'calc(100vw - 32px)',
    });
  }

  public onSelectWorkspace(ws: Workspace): void {
    void this.state.switchWorkspace(ws.id);
  }

  public async onEditProfile(ws: Workspace): Promise<void> {
    await this.state.switchWorkspace(ws.id);
    this.dialogRef?.close();
    await this.router.navigate(['/profile']);
  }

  public workspaceSubtitle(workspace: Workspace): string {
    return workspace.organizationName || workspace.workerName || 'Profile';
  }

  public async onDeleteWorkspace(ws: Workspace): Promise<void> {
    if (this.state.workspaces().length <= 1) {
      this.snackBar.open('You cannot delete the last profile', 'Close', { duration: 3000 });
      return;
    }

    const confirmed = await firstValueFrom(
      this.dialog
        .open(ConfirmDialogComponent, {
          width: '440px',
          data: {
            title: 'Delete profile?',
            message: `Delete "${ws.name}" and all its entries, projects, and settings?`,
            confirmLabel: 'Delete profile',
            destructive: true,
          },
        })
        .afterClosed(),
    );
    if (!confirmed) return;

    await this.state.deleteWorkspace(ws.id);
    this.snackBar.open(`Profile "${ws.name}" deleted`, 'Close', { duration: 3000 });
  }
}
