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

export interface ColorOption {
  name: string;
  hex: string;
}

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
  ],
  templateUrl: './projects.component.html',
  styleUrls: ['./projects.component.scss'],
})
export class ProjectsComponent {
  public state = inject(AppStateService);
  private dialog = inject(MatDialog);

  public newProjectName = signal<string>('');
  public newProjectColor = signal<string>('#5F875F');

  public availableColors: ColorOption[] = [
    { name: 'Dagsverk green', hex: '#5F875F' },
    { name: 'Blue', hex: '#0B57D0' },
    { name: 'Teal', hex: '#00838F' },
    { name: 'Green', hex: '#2E7D32' },
    { name: 'Orange', hex: '#ED6C02' },
    { name: 'Pink', hex: '#C2185B' },
    { name: 'Purple', hex: '#7B1FA2' },
    { name: 'Indigo', hex: '#5C6BC0' },
    { name: 'Slate', hex: '#455A64' },
  ];

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
