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
import { AppStateService } from '../../core/app-state.service';
import { TaxCalculatorService } from '../../core/tax-calculator.service';
import {
  AppSettings,
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
  TaxMode
} from '../../core/models';

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
    MatExpansionModule
  ],
  templateUrl: './settings.component.html',
  styleUrls: ['./settings.component.scss']
})
export class SettingsComponent {
  public state = inject(AppStateService);
  private taxCalculator = inject(TaxCalculatorService);
  private snackBar = inject(MatSnackBar);

  public readonly SalaryType = SalaryType;
  public readonly OvertimeCompensationMode = OvertimeCompensationMode;
  public readonly OvertimeThresholdMode = OvertimeThresholdMode;
  public readonly TaxMode = TaxMode;
  public readonly CompensationRuleType = CompensationRuleType;
  public readonly CompensationRateType = CompensationRateType;
  public readonly OvertimeDayCategory = OvertimeDayCategory;

  // Local Form Model
  public model: AppSettings = JSON.parse(JSON.stringify(this.state.settings()));

  public dayOptions = [
    { label: 'Mon', value: 1 },
    { label: 'Tue', value: 2 },
    { label: 'Wed', value: 3 },
    { label: 'Thu', value: 4 },
    { label: 'Fri', value: 5 },
    { label: 'Sat', value: 6 },
    { label: 'Sun', value: 0 }
  ];

  public dayCategories = [
    { label: 'Scheduled Workdays', value: OvertimeDayCategory.ScheduledWorkdays },
    { label: 'All Days', value: OvertimeDayCategory.AllDays },
    { label: 'Weekends', value: OvertimeDayCategory.Weekends },
    { label: 'Saturday', value: OvertimeDayCategory.Saturday },
    { label: 'Sunday', value: OvertimeDayCategory.Sunday },
    { label: 'Public Holidays', value: OvertimeDayCategory.PublicHolidays }
  ];

  public testGrossSalary = signal<number>(35000);

  public constructor() {
    effect(() => {
      this.model = JSON.parse(JSON.stringify(this.state.settings()));
    });
  }

  public isWeekdaySelected(dayNum: number): boolean {
    const list = this.model.expectedHours?.workingWeekdays || [];
    return list.includes(dayNum);
  }

  public toggleWeekday(dayNum: number): void {
    const list = this.model.expectedHours.workingWeekdays || [];
    if (list.includes(dayNum)) {
      this.model.expectedHours.workingWeekdays = list.filter(d => d !== dayNum);
    } else {
      this.model.expectedHours.workingWeekdays = [...list, dayNum];
    }
  }

  public onAddRateBand(): void {
    const newBand: OvertimeRateBand = {
      name: 'Evening OB',
      dayCategory: OvertimeDayCategory.ScheduledWorkdays,
      startTime: '18:00',
      endTime: '22:00',
      compensationType: CompensationRuleType.Ob,
      rateType: CompensationRateType.HourlyPremiumPercent,
      rateValue: 50
    };
    this.model.overtimeCompensation.rateBands = [
      ...(this.model.overtimeCompensation.rateBands || []),
      newBand
    ];
  }

  public onRemoveRateBand(idx: number): void {
    const list = [...(this.model.overtimeCompensation.rateBands || [])];
    list.splice(idx, 1);
    this.model.overtimeCompensation.rateBands = list;
  }

  public getDayCategoryLabel(cat: OvertimeDayCategory): string {
    const found = this.dayCategories.find(c => c.value === cat);
    return found ? found.label : 'All Days';
  }

  public get testTaxEstimate() {
    return this.taxCalculator.calculate(this.testGrossSalary(), this.model.taxSettings);
  }

  public async onSave(): Promise<void> {
    await this.state.updateSettings(this.model);
    this.snackBar.open('Settings saved successfully', 'Close', {
      duration: 3000,
      horizontalPosition: 'center',
      verticalPosition: 'bottom'
    });
  }
}
