import { Component, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { MAT_DIALOG_DATA, MatDialogModule, MatDialogRef } from '@angular/material/dialog';
import { MatButtonModule } from '@angular/material/button';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatChipsModule } from '@angular/material/chips';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { AppStateService } from '../../core/app-state.service';
import {
  AppSettings,
  LanguagePreference,
  OvertimeCompensationMode,
  OvertimeThresholdMode,
  SalaryType,
  WorkspaceType,
} from '../../core/models';
import { WEEKDAY_OPTIONS } from './weekdays';
import { HolidayPayFieldComponent } from './holiday-pay-field.component';

export interface ProfileFormDialogData {
  // 'setup' fills in the first profile on first start. 'create' adds a new profile.
  mode: 'setup' | 'create';
}

const PROFILE_COLORS = ['#5F875F', '#00897B', '#039BE5', '#3F51B5', '#8E24AA', '#F4511E'];

// Asks for the few things a profile needs before the first entry: name, pay, and schedule.
// Everything else has a sensible default and lives on the Profile page.
@Component({
  selector: 'app-profile-form-dialog',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    MatDialogModule,
    MatButtonModule,
    MatButtonToggleModule,
    MatChipsModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
    HolidayPayFieldComponent,
  ],
  template: `
    <h2 mat-dialog-title>{{ isSetup ? 'Welcome to Dagsverk' : 'New profile' }}</h2>
    <mat-dialog-content class="profile-form">
      <p class="body-medium intro">
        {{
          isSetup
            ? 'Make a profile for the job you track. You can add more profiles later.'
            : 'Each profile has its own timesheet, projects, pay, and schedule.'
        }}
      </p>

      <div class="form-grid">
        <mat-form-field appearance="outline" subscriptSizing="dynamic">
          <mat-label>Profile name</mat-label>
          <input matInput [(ngModel)]="profileName" placeholder="e.g. Café job" />
        </mat-form-field>
        <mat-form-field appearance="outline" subscriptSizing="dynamic">
          <mat-label>Employer or client</mat-label>
          <input matInput [(ngModel)]="employerName" placeholder="Optional" />
        </mat-form-field>
        <mat-form-field appearance="outline" subscriptSizing="dynamic">
          <mat-label>Your name</mat-label>
          <input matInput [(ngModel)]="workerName" placeholder="Shown in reports" />
        </mat-form-field>
        @if (isSetup) {
          <mat-form-field appearance="outline" subscriptSizing="dynamic">
            <mat-label>Language</mat-label>
            <mat-select [(ngModel)]="language">
              <mat-option [value]="LanguagePreference.System">System</mat-option>
              <mat-option [value]="LanguagePreference.English">English</mat-option>
              <mat-option [value]="LanguagePreference.Swedish">Swedish</mat-option>
            </mat-select>
          </mat-form-field>
        }
      </div>

      <section class="form-section">
        <h3 class="title-small">Pay</h3>
        <div class="inline-row">
          <mat-button-toggle-group
            [(ngModel)]="salaryType"
            hideSingleSelectionIndicator="true"
            aria-label="Pay type"
          >
            <mat-button-toggle [value]="SalaryType.Hourly">Hourly</mat-button-toggle>
            <mat-button-toggle [value]="SalaryType.Monthly">Monthly</mat-button-toggle>
          </mat-button-toggle-group>
          <mat-form-field appearance="outline" subscriptSizing="dynamic" class="amount">
            <mat-label>{{
              salaryType === SalaryType.Hourly ? 'Hourly rate' : 'Monthly salary'
            }}</mat-label>
            <input matInput type="number" min="0" [(ngModel)]="salaryValue" class="numeric" />
            <span matTextSuffix>{{ currency }}</span>
          </mat-form-field>
          @if (salaryType === SalaryType.Hourly) {
            <app-holiday-pay-field [(percent)]="holidayPayPercent" />
          }
        </div>
      </section>

      <section class="form-section">
        <h3 class="title-small">Schedule</h3>
        <mat-button-toggle-group
          [(ngModel)]="fixedSchedule"
          hideSingleSelectionIndicator="true"
          aria-label="Schedule type"
        >
          <mat-button-toggle [value]="true">Fixed schedule</mat-button-toggle>
          <mat-button-toggle [value]="false">No fixed schedule</mat-button-toggle>
        </mat-button-toggle-group>
        @if (fixedSchedule) {
          <div class="inline-row">
            <mat-form-field appearance="outline" subscriptSizing="dynamic" class="amount">
              <mat-label>Hours per workday</mat-label>
              <input matInput type="number" min="0" step="0.5" [(ngModel)]="hoursPerDay" />
            </mat-form-field>
            <mat-chip-listbox multiple="true" aria-label="Workdays">
              @for (day of weekdays; track day.value) {
                <mat-chip-option
                  [selected]="workdays.includes(day.value)"
                  (click)="toggleWorkday(day.value)"
                  >{{ day.label }}</mat-chip-option
                >
              }
            </mat-chip-listbox>
          </div>
          <p class="body-small hint">
            Dagsverk tracks overtime and your time balance against this.
          </p>
        } @else {
          <p class="body-small hint">
            For shifts that change from week to week. Dagsverk counts the hours you log and does not
            expect any.
          </p>
        }
      </section>
    </mat-dialog-content>
    <mat-dialog-actions align="end">
      @if (!isSetup) {
        <button mat-button type="button" mat-dialog-close>Cancel</button>
      }
      <button mat-flat-button type="button" [disabled]="!canSave" (click)="save()">
        {{ isSetup ? 'Get started' : 'Create profile' }}
      </button>
    </mat-dialog-actions>
  `,
  styles: [
    `
      .profile-form {
        display: flex;
        flex-direction: column;
        gap: 24px;
      }
      .intro,
      .hint {
        margin: 0;
        color: var(--app-on-surface-variant);
      }
      .form-grid {
        display: grid;
        grid-template-columns: repeat(2, minmax(0, 1fr));
        gap: 16px;
      }
      .form-section {
        display: flex;
        flex-direction: column;
        gap: 12px;
      }
      .form-section h3 {
        margin: 0;
      }
      .inline-row {
        display: flex;
        flex-wrap: wrap;
        align-items: center;
        gap: 16px;
      }
      .amount {
        width: 200px;
      }
      @media (max-width: 640px) {
        .form-grid {
          grid-template-columns: minmax(0, 1fr);
        }
      }
    `,
  ],
})
export class ProfileFormDialogComponent {
  private state = inject(AppStateService);
  private dialogRef = inject(MatDialogRef<ProfileFormDialogComponent>);
  private data = inject<ProfileFormDialogData>(MAT_DIALOG_DATA);

  public readonly SalaryType = SalaryType;
  public readonly LanguagePreference = LanguagePreference;
  public readonly weekdays = WEEKDAY_OPTIONS;
  public readonly isSetup = this.data.mode === 'setup';

  private current = this.state.activeWorkspace();
  public profileName = '';
  public employerName = this.isSetup ? this.current.organizationName || '' : '';
  public workerName = this.current.workerName || '';
  public language = this.state.preferences().languagePreference;
  public salaryType = SalaryType.Hourly;
  public salaryValue = this.state.settings().salary.hourlyRate;
  // Semesterlagen gives hourly staff at least 12 %.
  public holidayPayPercent = 12;
  public fixedSchedule = true;
  public hoursPerDay = 8;
  public workdays = [1, 2, 3, 4, 5];
  public currency = this.state.settings().currencyPreference;
  private saving = false;

  public get canSave(): boolean {
    return Boolean(this.profileName.trim()) && !this.saving;
  }

  public toggleWorkday(day: number): void {
    this.workdays = this.workdays.includes(day)
      ? this.workdays.filter((d) => d !== day)
      : [...this.workdays, day];
  }

  public async save(): Promise<void> {
    if (!this.canSave) return;
    this.saving = true;
    const now = new Date().toISOString();
    const name = this.profileName.trim();
    const identity = {
      name,
      type: WorkspaceType.Employment,
      workerName: this.workerName.trim() || undefined,
      organizationName: this.employerName.trim() || undefined,
      updatedAt: now,
    };

    if (this.isSetup) {
      await this.state.saveWorkspace({ ...this.current, ...identity });
    } else {
      const id = `ws-${Date.now()}`;
      const color = PROFILE_COLORS[this.state.workspaces().length % PROFILE_COLORS.length];
      await this.state.saveWorkspace({ id, color, createdAt: now, ...identity });
      await this.state.switchWorkspace(id);
      // A new profile starts with one project named after it.
      await this.state.saveProject({
        id: `proj-${Date.now()}`,
        name,
        color,
        isActive: true,
        isDefault: true,
      });
    }

    const settings: AppSettings = structuredClone(this.state.settings());
    settings.employeeName = identity.workerName ?? '';
    settings.employerName = identity.organizationName ?? '';
    if (!this.isSetup) settings.defaultProject = name;
    settings.salary.type = this.salaryType;
    if (this.salaryType === SalaryType.Hourly) {
      settings.salary.hourlyRate = this.salaryValue;
      settings.salary.holidayPayPercent = this.holidayPayPercent;
    } else {
      settings.salary.monthlySalary = this.salaryValue;
    }
    settings.expectedHours.workingWeekdays = this.fixedSchedule ? this.workdays : [];
    settings.expectedHours.hoursPerWorkday = this.fixedSchedule ? this.hoursPerDay : 0;
    // Comp time needs a time balance, and only a fixed schedule has one. Without a
    // schedule, overtime starts after a fixed number of hours per day.
    if (!this.fixedSchedule) {
      settings.overtimeCompensation.mode = OvertimeCompensationMode.Paid;
      settings.overtimeCompensation.thresholdMode = OvertimeThresholdMode.FixedDailyHours;
    }
    await this.state.updateSettings(settings);

    if (this.isSetup) {
      await this.state.updatePreferences({
        ...this.state.preferences(),
        languagePreference: this.language,
        hasCompletedSetup: true,
      });
    }
    this.dialogRef.close(true);
  }
}
