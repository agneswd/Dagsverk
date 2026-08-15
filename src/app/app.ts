import { Component, DestroyRef, effect, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { Router, RouterOutlet } from '@angular/router';
import { MatSidenavModule } from '@angular/material/sidenav';
import { MatDialog, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatButtonModule } from '@angular/material/button';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { HeaderComponent } from './layout/header/header.component';
import { SidebarComponent } from './layout/sidebar/sidebar.component';
import { AppStateService } from './core/app-state.service';
import { ElectronBridgeService } from './core/electron-bridge.service';
import { LanguagePreference, MonthViewPreference, SalaryType, WorkspaceType } from './core/models';

@Component({
  selector: 'app-setup-dialog',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    MatDialogModule,
    MatButtonModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
  ],
  template: `
    <h2 mat-dialog-title>Set up Dagsverk</h2>
    <mat-dialog-content class="setup-content">
      <p>Set the identity, schedule, and pay defaults for your first workspace.</p>
      <div class="setup-grid">
        <mat-form-field appearance="outline"
          ><mat-label>Workspace name</mat-label><input matInput [(ngModel)]="workspaceName"
        /></mat-form-field>
        <mat-form-field appearance="outline"
          ><mat-label>Workspace type</mat-label
          ><mat-select [(ngModel)]="workspaceType"
            ><mat-option [value]="WorkspaceType.Employment">Employment</mat-option
            ><mat-option [value]="WorkspaceType.Contract">Contract or client</mat-option
            ><mat-option [value]="WorkspaceType.Personal">Personal</mat-option></mat-select
          ></mat-form-field
        >
        <mat-form-field appearance="outline"
          ><mat-label>Worker name</mat-label><input matInput [(ngModel)]="workerName"
        /></mat-form-field>
        @if (workspaceType !== WorkspaceType.Personal) {
          <mat-form-field appearance="outline"
            ><mat-label>Organization</mat-label><input matInput [(ngModel)]="organizationName"
          /></mat-form-field>
        }
        <mat-form-field appearance="outline"
          ><mat-label>Hours per workday</mat-label
          ><input matInput type="number" min="0" step="0.5" [(ngModel)]="hoursPerDay"
        /></mat-form-field>
        <mat-form-field appearance="outline"
          ><mat-label>Interface language</mat-label
          ><mat-select [(ngModel)]="language"
            ><mat-option [value]="LanguagePreference.System">System</mat-option
            ><mat-option [value]="LanguagePreference.English">English</mat-option
            ><mat-option [value]="LanguagePreference.Swedish">Swedish</mat-option></mat-select
          ></mat-form-field
        >
        <mat-form-field appearance="outline"
          ><mat-label>Salary model</mat-label
          ><mat-select [(ngModel)]="salaryType"
            ><mat-option [value]="SalaryType.Hourly">Hourly rate</mat-option
            ><mat-option [value]="SalaryType.Monthly">Monthly salary</mat-option></mat-select
          ></mat-form-field
        >
        <mat-form-field appearance="outline"
          ><mat-label>{{
            salaryType === SalaryType.Hourly ? 'Hourly rate' : 'Monthly salary'
          }}</mat-label
          ><input matInput type="number" min="0" [(ngModel)]="salaryValue"
        /></mat-form-field>
      </div>
    </mat-dialog-content>
    <mat-dialog-actions align="end"
      ><button mat-flat-button type="button" [disabled]="!workspaceName.trim()" (click)="save()">
        Save and continue
      </button></mat-dialog-actions
    >
  `,
  styles: [
    `
      .setup-content {
        display: flex;
        flex-direction: column;
        gap: 24px;
        width: 100%;
        overflow: hidden;
      }
      .setup-grid {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 16px;
      }
      .setup-grid mat-form-field {
        width: 100%;
        min-width: 0;
      }
      @media (max-width: 640px) {
        .setup-grid {
          grid-template-columns: minmax(0, 1fr);
        }
      }
    `,
  ],
})
export class SetupDialogComponent {
  public state = inject(AppStateService);
  public dialogRef = inject(MatDialogRef<SetupDialogComponent>);
  public WorkspaceType = WorkspaceType;
  public LanguagePreference = LanguagePreference;
  public SalaryType = SalaryType;
  public workspaceName = this.state.activeWorkspace().name || 'Main workspace';
  public workspaceType = this.state.activeWorkspace().type;
  public workerName = this.state.activeWorkspace().workerName || '';
  public organizationName = this.state.activeWorkspace().organizationName || '';
  public hoursPerDay = this.state.settings().expectedHours.hoursPerWorkday;
  public salaryType = this.state.settings().salary.type;
  public salaryValue =
    this.salaryType === SalaryType.Hourly
      ? this.state.settings().salary.hourlyRate
      : this.state.settings().salary.monthlySalary;
  public language = this.state.preferences().languagePreference;

  public async save(): Promise<void> {
    await this.state.saveWorkspace({
      ...this.state.activeWorkspace(),
      name: this.workspaceName.trim(),
      type: this.workspaceType,
      workerName: this.workerName.trim() || undefined,
      organizationName:
        this.workspaceType === WorkspaceType.Personal
          ? undefined
          : this.organizationName.trim() || undefined,
      updatedAt: new Date().toISOString(),
    });
    const settings = structuredClone(this.state.settings());
    settings.expectedHours.hoursPerWorkday = this.hoursPerDay;
    settings.salary.type = this.salaryType;
    if (this.salaryType === SalaryType.Hourly) settings.salary.hourlyRate = this.salaryValue;
    else settings.salary.monthlySalary = this.salaryValue;
    await this.state.updateSettings(settings);
    await this.state.updatePreferences({
      ...this.state.preferences(),
      languagePreference: this.language,
      hasCompletedSetup: true,
    });
    this.dialogRef.close();
  }
}

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [
    CommonModule,
    RouterOutlet,
    MatSidenavModule,
    MatSnackBarModule,
    HeaderComponent,
    SidebarComponent,
  ],
  templateUrl: './app.html',
  styleUrl: './app.scss',
})
export class App {
  public state = inject(AppStateService);
  private dialog = inject(MatDialog);
  private router = inject(Router);
  private destroyRef = inject(DestroyRef);
  private bridge = inject(ElectronBridgeService);
  private snackBar = inject(MatSnackBar);
  private setupOpened = false;
  private notifiedUpdateVersion?: string;
  private isNarrowWindow = window.innerWidth < 1200;
  public sidebarCollapsed = signal(this.isNarrowWindow);

  public constructor() {
    const handleResize = () => {
      const isNarrowWindow = window.innerWidth < 1200;
      if (isNarrowWindow && !this.isNarrowWindow) this.sidebarCollapsed.set(true);
      this.isNarrowWindow = isNarrowWindow;
    };
    window.addEventListener('resize', handleResize);
    this.destroyRef.onDestroy(() => window.removeEventListener('resize', handleResize));

    effect(() => {
      const update = this.bridge.updateState();
      if (update.status !== 'ready' || update.availableVersion === this.notifiedUpdateVersion)
        return;
      this.notifiedUpdateVersion = update.availableVersion;
      const notification = this.snackBar.open(
        `Dagsverk ${update.availableVersion || ''} is ready to install.`,
        'Restart now',
        { duration: 0 },
      );
      notification.onAction().subscribe(() => this.bridge.restartToUpdate());
    });

    effect(() => {
      if (
        this.state.isInitialized() &&
        !this.state.preferences().hasCompletedSetup &&
        !this.setupOpened
      ) {
        this.setupOpened = true;
        this.dialog.open(SetupDialogComponent, {
          disableClose: true,
          width: '720px',
          maxWidth: 'calc(100vw - 32px)',
        });
      }
    });
  }

  public onShortcut(event: KeyboardEvent): void {
    const target = event.target as HTMLElement | null;
    const editing = target?.matches('input, textarea, select, [contenteditable="true"]');
    if (editing && !(event.ctrlKey && event.key.toLowerCase() === 's')) return;

    if (event.ctrlKey && event.key === '1') {
      event.preventDefault();
      this.router.navigate(['/timesheet']);
      this.state.setView(MonthViewPreference.Ledger);
    } else if (event.ctrlKey && event.key === '2') {
      event.preventDefault();
      this.router.navigate(['/timesheet']);
      this.state.setView(MonthViewPreference.Calendar);
    } else if (event.ctrlKey && event.key.toLowerCase() === 'm') {
      event.preventDefault();
      this.state.startCatchUp();
    } else if (event.ctrlKey && event.key.toLowerCase() === 'e') {
      event.preventDefault();
      this.state.exportExcel();
    } else if (event.ctrlKey && event.key === ',') {
      event.preventDefault();
      this.router.navigate(['/settings']);
    } else if (event.ctrlKey && event.key.toLowerCase() === 's') {
      event.preventDefault();
      window.dispatchEvent(new Event('dagsverk-save'));
    } else if (event.key === 'PageUp') {
      event.preventDefault();
      this.state.previousMonth();
    } else if (event.key === 'PageDown') {
      event.preventDefault();
      this.state.nextMonth();
    } else if (event.key === 'Escape' && this.state.isEditorOpen()) {
      event.preventDefault();
      this.state.closeCatchUp();
    }
  }
}
