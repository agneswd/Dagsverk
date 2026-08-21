import { Component, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { MatCardModule } from '@angular/material/card';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSlideToggleModule } from '@angular/material/slide-toggle';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatDialog, MatDialogModule } from '@angular/material/dialog';
import { firstValueFrom } from 'rxjs';
import { AppStateService } from '../../core/app-state.service';
import { Project } from '../../core/models';
import { ConfirmDialogComponent } from '../../core/confirm-dialog.component';
import { ColorPickerComponent } from '../../shared/color-picker/color-picker.component';
import { RenameDialogComponent } from '../../shared/rename-dialog/rename-dialog.component';

@Component({
  selector: 'app-projects',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    MatCardModule,
    MatButtonModule,
    MatIconModule,
    MatFormFieldModule,
    MatInputModule,
    MatSlideToggleModule,
    MatTooltipModule,
    MatDialogModule,
    ColorPickerComponent,
  ],
  templateUrl: './projects.component.html',
  styleUrls: ['./projects.component.scss'],
})
export class ProjectsComponent {
  public state = inject(AppStateService);
  private dialog = inject(MatDialog);

  public newProjectName = signal<string>('');
  public newProjectColor = signal<string>('#5F875F');

  public async onAddProject(): Promise<void> {
    const name = this.newProjectName().trim();
    if (!name) return;

    const newProj: Project = {
      workspaceId: this.state.activeWorkspaceId(),
      id: `proj-${Date.now()}`,
      name,
      color: this.newProjectColor(),
      isActive: true,
      isDefault: this.state.projects().length === 0,
    };

    await this.state.saveProject(newProj);
    this.newProjectName.set('');
  }

  public async onSetDefault(project: Project): Promise<void> {
    for (const p of this.state.projects()) {
      const updated = { ...p, isDefault: p.id === project.id };
      await this.state.saveProject(updated);
    }
  }

  public onProjectColorChange(project: Project, color: string): void {
    void this.state.saveProject({ ...project, color });
  }

  public async onRenameProject(project: Project): Promise<void> {
    const name = await firstValueFrom(
      this.dialog
        .open(RenameDialogComponent, {
          width: '440px',
          data: { title: 'Rename project', label: 'New name', initialName: project.name },
        })
        .afterClosed(),
    );
    if (name === undefined || name === project.name) return;
    await this.state.saveProject({ ...project, name });
  }

  public async onToggleActive(project: Project): Promise<void> {
    const updated = { ...project, isActive: !project.isActive };
    await this.state.saveProject(updated);
  }

  public async onDelete(project: Project): Promise<void> {
    if (project.isDefault) return;
    const confirmed = await firstValueFrom(
      this.dialog
        .open(ConfirmDialogComponent, {
          width: '440px',
          data: {
            title: 'Delete project?',
            message: `Delete "${project.name}"? Existing time entries keep the project name.`,
            confirmLabel: 'Delete project',
            destructive: true,
          },
        })
        .afterClosed(),
    );
    if (!confirmed) return;
    await this.state.deleteProject(project.id);
  }
}
