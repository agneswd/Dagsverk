import { Component, DestroyRef, effect, inject, signal } from '@angular/core';
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
import { TaxCalculatorService } from '../../core/tax-calculator.service';
import {
  AppSettings,
  AppPreferences,
  CompensationRateType,
  CompensationRuleType,
  CurrencyPreference,
  ExportLanguagePreference,
  LanguagePreference,
  OvertimeCompensationMode,
  OvertimeDayCategory,
  OvertimeRateBand,
  OvertimeThresholdMode,
  SalaryType,
  TaxMode,
  ThemePreference,
  UpdateState,
} from '../../core/models';
import { ElectronBridgeService } from '../../core/electron-bridge.service';
import { ConfirmDialogComponent } from '../../core/confirm-dialog.component';

@Component({
  selector: 'app-settings',
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
  ],
  templateUrl: './settings.component.html',
  styleUrls: ['./settings.component.scss'],
})
export class SettingsComponent {
  public state = inject(AppStateService);
  private taxCalculator = inject(TaxCalculatorService);
  private snackBar = inject(MatSnackBar);
  public bridge = inject(ElectronBridgeService);
  private destroyRef = inject(DestroyRef);
  private dialog = inject(MatDialog);

  public readonly SalaryType = SalaryType;
  public readonly OvertimeCompensationMode = OvertimeCompensationMode;
  public readonly OvertimeThresholdMode = OvertimeThresholdMode;
  public readonly TaxMode = TaxMode;
  public readonly CompensationRuleType = CompensationRuleType;
  public readonly CompensationRateType = CompensationRateType;
  public readonly OvertimeDayCategory = OvertimeDayCategory;
  public readonly ThemePreference = ThemePreference;
  public readonly LanguagePreference = LanguagePreference;
  public readonly ExportLanguagePreference = ExportLanguagePreference;

  // Local Form Model
  public model: AppSettings = JSON.parse(JSON.stringify(this.state.settings()));
  public preferencesModel: AppPreferences = { ...this.state.preferences() };
  private savedSettings = '';
  private savedPreferences = '';
  public scaleOptions = [80, 90, 100, 110, 125, 150];

  public dayOptions = [
    { label: 'Mon', value: 1 },
    { label: 'Tue', value: 2 },
    { label: 'Wed', value: 3 },
    { label: 'Thu', value: 4 },
    { label: 'Fri', value: 5 },
    { label: 'Sat', value: 6 },
    { label: 'Sun', value: 0 },
  ];

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

  public testGrossSalary = signal<number>(35000);
  public updateState = signal<UpdateState>({ status: 'idle', currentVersion: '' });

  public constructor() {
    effect(() => {
      this.model = JSON.parse(JSON.stringify(this.state.settings()));
      this.preferencesModel = { ...this.state.preferences() };
      this.captureSavedState();
    });
    void this.bridge.getUpdateState().then((state) => this.updateState.set(state));
    const unsubscribe = this.bridge.onUpdateState((state) => this.updateState.set(state));
    this.destroyRef.onDestroy(unsubscribe);
  }

  public async onCheckForUpdates(): Promise<void> {
    await this.bridge.checkForUpdates();
  }

  public onRestartToUpdate(): void {
    this.bridge.restartToUpdate();
  }

  public updateStatusText(): string {
    const update = this.updateState();
    switch (update.status) {
      case 'checking':
        return 'Checking for updates...';
      case 'available':
        return `Downloading version ${update.availableVersion}...`;
      case 'downloading':
        return `Downloading update - ${update.progress || 0}%`;
      case 'ready':
        return `Version ${update.availableVersion} is ready to install.`;
      case 'current':
        return 'Dagsverk is up to date.';
      case 'error':
        return update.message || 'Update check failed.';
      case 'unavailable':
        return 'Update checks are available in the installed desktop app.';
      default:
        return `Version ${update.currentVersion || 'development'}`;
    }
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

  public get testTaxEstimate() {
    return this.taxCalculator.calculate(this.testGrossSalary(), this.model.taxSettings);
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
    await this.state.updateSettings(this.model);
    await this.state.updatePreferences(this.preferencesModel);
    this.captureSavedState();
    this.snackBar.open('Settings saved successfully', 'Close', {
      duration: 3000,
      horizontalPosition: 'center',
      verticalPosition: 'bottom',
    });
  }

  public onDiscard(): void {
    this.model = JSON.parse(JSON.stringify(this.state.settings()));
    this.preferencesModel = { ...this.state.preferences() };
  }

  public get isDirty(): boolean {
    return (
      JSON.stringify(this.model) !== this.savedSettings ||
      JSON.stringify(this.preferencesModel) !== this.savedPreferences
    );
  }

  private captureSavedState(): void {
    this.savedSettings = JSON.stringify(this.model);
    this.savedPreferences = JSON.stringify(this.preferencesModel);
  }
}
