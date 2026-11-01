import { Component, effect, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { MatTabsModule } from '@angular/material/tabs';
import { MatCardModule } from '@angular/material/card';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatButtonModule } from '@angular/material/button';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatCheckboxModule } from '@angular/material/checkbox';
import { MatSlideToggleModule } from '@angular/material/slide-toggle';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatSnackBar, MatSnackBarModule } from '@angular/material/snack-bar';
import { MatChipsModule } from '@angular/material/chips';
import { MatDividerModule } from '@angular/material/divider';
import { MatExpansionModule } from '@angular/material/expansion';
import { MatDialog, MatDialogModule } from '@angular/material/dialog';
import { firstValueFrom } from 'rxjs';
import { AppStateService } from '../../core/app-state.service';
import {
  AppSettings,
  CompensationRateType,
  CompensationRuleType,
  ExportLanguagePreference,
  HourlyPayBasis,
  ObOvertimeCombinationMode,
  OvertimeCompensationMode,
  OvertimeDayCategory,
  OvertimeRateBand,
  OvertimeThresholdMode,
  SalaryType,
  TaxMode,
} from '../../core/models';
import { ConfirmDialogComponent } from '../../core/confirm-dialog.component';
import { ColorPickerComponent } from '../../shared/color-picker/color-picker.component';
import { WEEKDAY_OPTIONS } from './weekdays';

// Editable identity of the active profile. Saved with the settings.
interface ProfileIdentity {
  name: string;
  workerName: string;
  organizationName: string;
  color: string;
}

// Settings for the active profile: identity, schedule, pay, tax, and overtime.
// App-wide preferences live on the Settings page.
@Component({
  selector: 'app-profile-settings',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    MatTabsModule,
    MatCardModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
    MatButtonModule,
    MatButtonToggleModule,
    MatCheckboxModule,
    MatSlideToggleModule,
    MatIconModule,
    MatTooltipModule,
    MatSnackBarModule,
    MatChipsModule,
    MatDividerModule,
    MatExpansionModule,
    MatDialogModule,
    ColorPickerComponent,
  ],
  templateUrl: './profile-settings.component.html',
  styleUrls: ['./profile-settings.component.scss'],
})
export class ProfileSettingsComponent {
  public state = inject(AppStateService);
  private snackBar = inject(MatSnackBar);
  private dialog = inject(MatDialog);

  public readonly SalaryType = SalaryType;
  public readonly OvertimeCompensationMode = OvertimeCompensationMode;
  public readonly OvertimeThresholdMode = OvertimeThresholdMode;
  public readonly TaxMode = TaxMode;
  public readonly CompensationRuleType = CompensationRuleType;
  public readonly CompensationRateType = CompensationRateType;
  public readonly OvertimeDayCategory = OvertimeDayCategory;
  public readonly ExportLanguagePreference = ExportLanguagePreference;
  public readonly HourlyPayBasis = HourlyPayBasis;
  public readonly ObOvertimeCombinationMode = ObOvertimeCombinationMode;

  // Local form models. Nothing is saved until Save changes.
  public model: AppSettings = structuredClone(this.state.settings());
  public identity: ProfileIdentity = this.readIdentity();
  private savedState = '';
  public readonly dayOptions = WEEKDAY_OPTIONS;

  public dayCategories = [
    { label: 'All Days', value: OvertimeDayCategory.AllDays },
    { label: 'Scheduled Workdays', value: OvertimeDayCategory.ScheduledWorkdays },
    { label: 'Scheduled Weekdays', value: OvertimeDayCategory.ScheduledWeekdays },
    { label: 'Non-workdays', value: OvertimeDayCategory.NonWorkdays },
    { label: 'Weekends', value: OvertimeDayCategory.Weekends },
    { label: 'Monday', value: OvertimeDayCategory.Monday },
    { label: 'Tuesday', value: OvertimeDayCategory.Tuesday },
    { label: 'Wednesday', value: OvertimeDayCategory.Wednesday },
    { label: 'Thursday', value: OvertimeDayCategory.Thursday },
    { label: 'Friday', value: OvertimeDayCategory.Friday },
    { label: 'Saturday', value: OvertimeDayCategory.Saturday },
    { label: 'Sunday', value: OvertimeDayCategory.Sunday },
    { label: 'Public Holidays', value: OvertimeDayCategory.PublicHolidays },
    { label: 'Major Holidays', value: OvertimeDayCategory.MajorHolidays },
  ];

  public constructor() {
    effect(() => {
      this.model = structuredClone(this.state.settings());
      this.identity = this.readIdentity();
      this.captureSavedState();
    });
  }

  // A fixed schedule needs both workdays and hours. Without one, nothing is expected.
  public get fixedSchedule(): boolean {
    return (
      this.model.expectedHours.workingWeekdays.length > 0 &&
      this.model.expectedHours.hoursPerWorkday > 0
    );
  }

  public setFixedSchedule(fixed: boolean): void {
    this.model.expectedHours.workingWeekdays = fixed ? [1, 2, 3, 4, 5] : [];
    this.model.expectedHours.hoursPerWorkday = fixed ? 8 : 0;
    // Comp time needs a time balance, and only a fixed schedule has one.
    if (!fixed) this.model.overtimeCompensation.mode = OvertimeCompensationMode.Paid;
  }

  public isWeekdaySelected(dayNum: number): boolean {
    const list = this.model.expectedHours?.workingWeekdays || [];
    return list.includes(dayNum);
  }

  public toggleWeekday(dayNum: number): void {
    const list = this.model.expectedHours.workingWeekdays || [];
    if (list.includes(dayNum)) {
      this.model.expectedHours.workingWeekdays = list.filter((d) => d !== dayNum);
    } else {
      this.model.expectedHours.workingWeekdays = [...list, dayNum];
    }
  }

  public onAddRateBand(type: CompensationRuleType): void {
    const newBand: OvertimeRateBand = {
      name: type === CompensationRuleType.Ob ? 'Evening OB' : 'Overtime',
      dayCategory: OvertimeDayCategory.ScheduledWorkdays,
      startTime: '18:00',
      endTime: '22:00',
      compensationType: type,
      rateType: CompensationRateType.HourlyPremiumPercent,
      rateValue: 50,
    };
    this.model.overtimeCompensation.rateBands = [
      ...(this.model.overtimeCompensation.rateBands || []),
      newBand,
    ];
  }

  public onRemoveRateBand(idx: number): void {
    const list = [...(this.model.overtimeCompensation.rateBands || [])];
    list.splice(idx, 1);
    this.model.overtimeCompensation.rateBands = list;
  }

  public getDayCategoryLabel(cat: OvertimeDayCategory): string {
    const found = this.dayCategories.find((c) => c.value === cat);
    return found ? found.label : 'All Days';
  }

  public async onSave(): Promise<void> {
    const currentCurrency = this.state.settings().currencyPreference;
    if (currentCurrency !== this.model.currencyPreference) {
      const confirmed = await firstValueFrom(
        this.dialog
          .open(ConfirmDialogComponent, {
            width: '440px',
            data: {
              title: 'Change currency?',
              message: 'Dagsverk will not convert existing rates or report values.',
              confirmLabel: 'Change currency',
            },
          })
          .afterClosed(),
      );
      if (!confirmed) return;
    }
    const workspace = this.state.activeWorkspace();
    const workerName = this.identity.workerName.trim();
    const organizationName = this.identity.organizationName.trim();
    await this.state.saveWorkspace({
      ...workspace,
      name: this.identity.name.trim(),
      workerName: workerName || undefined,
      organizationName: organizationName || undefined,
      color: this.identity.color,
      updatedAt: new Date().toISOString(),
    });
    await this.state.updateSettings({
      ...this.model,
      employeeName: workerName,
      employerName: organizationName,
    });
    this.captureSavedState();
    this.snackBar.open('Profile saved', 'Close', {
      duration: 3000,
      horizontalPosition: 'center',
      verticalPosition: 'bottom',
    });
  }

  public onDiscard(): void {
    this.model = structuredClone(this.state.settings());
    this.identity = this.readIdentity();
  }

  public get isDirty(): boolean {
    return this.serialize() !== this.savedState;
  }

  private readIdentity(): ProfileIdentity {
    const workspace = this.state.activeWorkspace();
    return {
      name: workspace.name,
      workerName: workspace.workerName ?? '',
      organizationName: workspace.organizationName ?? '',
      color: workspace.color || '#5F875F',
    };
  }

  private serialize(): string {
    return JSON.stringify([this.model, this.identity]);
  }

  private captureSavedState(): void {
    this.savedState = this.serialize();
  }
}
