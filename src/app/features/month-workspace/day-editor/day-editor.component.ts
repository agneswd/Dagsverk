import { Component, HostListener, inject, signal, effect } from '@angular/core';
import { CommonModule } from '@angular/common';
import { FormsModule } from '@angular/forms';
import { MatFormFieldModule } from '@angular/material/form-field';
import { MatInputModule } from '@angular/material/input';
import { MatSelectModule } from '@angular/material/select';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatButtonToggleModule } from '@angular/material/button-toggle';
import { MatDividerModule } from '@angular/material/divider';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatChipsModule } from '@angular/material/chips';
import { MatSlideToggleModule } from '@angular/material/slide-toggle';
import {
  AppSettings,
  HourlyPayBasis,
  OvertimeCompensationMode,
  SalaryType,
  WorkEntry,
  WorkEntryStatus,
} from '../../../core/models';
import { MinuteMath, MonthlyCalculations, TimeInput } from '../../../core/monthly-calculations';
import { SwedishHolidayService } from '../../../core/swedish-holiday.service';
import { AppStateService } from '../../../core/app-state.service';
import { LocalizationService } from '../../../core/localization.service';

@Component({
  selector: 'app-day-editor',
  standalone: true,
  imports: [
    CommonModule,
    FormsModule,
    MatFormFieldModule,
    MatInputModule,
    MatSelectModule,
    MatButtonModule,
    MatIconModule,
    MatButtonToggleModule,
    MatDividerModule,
    MatTooltipModule,
    MatChipsModule,
    MatSlideToggleModule,
  ],
  templateUrl: './day-editor.component.html',
  styleUrls: ['./day-editor.component.scss'],
})
export class DayEditorComponent {
  public holidays = inject(SwedishHolidayService);
  public state = inject(AppStateService);
  private localization = inject(LocalizationService);

  public readonly WorkEntryStatus = WorkEntryStatus;

  public status = signal<WorkEntryStatus>(WorkEntryStatus.Incomplete);
  public startTime = signal<string>('08:00');
  public endTime = signal<string>('16:30');
  public lunchMinutes = signal<number>(30);
  public projectName = signal<string>('General');
  public dayOffReason = signal<string>('');
  public notes = signal<string>('');
  public useScheduledHoursOverride = signal<boolean>(false);
  public scheduledHours = signal<number>(8);
  public compTimeHours = signal<number>(0);
  public errorText = signal<string>('');

  public timePresets = [
    { label: '08:00-16:30', start: '08:00', end: '16:30', lunch: 30 },
    { label: '08:30-17:00', start: '08:30', end: '17:00', lunch: 30 },
    { label: '09:00-17:30', start: '09:00', end: '17:30', lunch: 30 },
  ];

  public lunchOptions = [0, 30, 45, 60];

  public dayOffReasons = [
    'Vacation',
    'Sick leave',
    'Care of child',
    'Leave of absence',
    'Parental leave',
    'Public holiday',
    'Comp time',
  ];

  public constructor() {
    effect(() => {
      const e = this.state.selectedEntry();
      const s = this.state.settings();
      if (e) {
        this.status.set(
          this.state.isCatchUpOpen() && e.status === WorkEntryStatus.Incomplete
            ? WorkEntryStatus.Worked
            : e.status,
        );
        this.startTime.set(e.startTime || s.defaultStartTime || '08:00');
        this.endTime.set(e.endTime || s.defaultEndTime || '16:30');
        this.lunchMinutes.set(e.lunchMinutes ?? s.defaultLunchMinutes ?? 30);
        this.projectName.set(e.projectName || s.defaultProject || 'General');
        const legacyReason = this.dayOffReasons.includes(e.notes || '') ? e.notes || '' : '';
        this.dayOffReason.set(e.dayOffReason || legacyReason);
        this.notes.set(!e.dayOffReason && legacyReason ? '' : e.notes || '');
        this.useScheduledHoursOverride.set(e.scheduledMinutesOverride !== null);
        this.scheduledHours.set(
          (e.scheduledMinutesOverride ?? s.expectedHours.hoursPerWorkday * 60) / 60,
        );
        this.compTimeHours.set((e.compTimeMinutes || 0) / 60);
        this.errorText.set('');
      }
    });
  }

  public get currentDateString(): string {
    return this.state.selectedDate() || this.state.todayString();
  }

  public get formattedDateTitle(): string {
    const d = this.state.selectedDate();
    if (!d) return '';
    const dateObj = new Date(`${d}T00:00:00`);
    return dateObj.toLocaleDateString(this.localization.language() === 'sv' ? 'sv-SE' : 'en-US', {
      weekday: 'long',
      month: 'short',
      day: 'numeric',
      year: 'numeric',
    });
  }

  public get holidayName(): string | null {
    const d = this.state.selectedDate();
    const holiday = d ? this.holidays.getHolidayName(d) : null;
    return holiday ? this.localization.t(holiday) : null;
  }

  public get workedHours(): number {
    if (this.status() !== WorkEntryStatus.Worked) return 0;
    const mins = MinuteMath.worked(this.startTime(), this.endTime(), this.lunchMinutes());
    return mins / 60;
  }

  public get dailyPay(): { regularPay: number; overtimePay: number; obPay: number; total: number } {
    if (this.status() !== WorkEntryStatus.Worked) {
      return { regularPay: 0, overtimePay: 0, obPay: 0, total: 0 };
    }
    const fakeEntry: WorkEntry = {
      date: this.currentDateString,
      status: this.status(),
      startTime: this.startTime(),
      endTime: this.endTime(),
      lunchMinutes: this.lunchMinutes(),
      projectName: this.projectName(),
      dayOffReason: null,
      notes: this.notes(),
      compTimeMinutes: Math.round(this.compTimeHours() * 60),
      scheduledMinutesOverride: this.useScheduledHoursOverride()
        ? Math.round(this.scheduledHours() * 60)
        : null,
    };
    return MonthlyCalculations.calculateDailyPay(
      fakeEntry,
      this.state.settings().expectedHours,
      this.state.settings().salary,
      this.state.settings().overtimeCompensation,
      this.holidays,
    );
  }

  public applyPreset(preset: { start: string; end: string; lunch: number }): void {
    this.status.set(WorkEntryStatus.Worked);
    this.startTime.set(preset.start);
    this.endTime.set(preset.end);
    this.lunchMinutes.set(preset.lunch);
  }

  public applyNormalDay(): void {
    const settings = this.state.settings();
    this.status.set(WorkEntryStatus.Worked);
    this.startTime.set(settings.defaultStartTime);
    this.endTime.set(settings.defaultEndTime);
    this.lunchMinutes.set(settings.defaultLunchMinutes);
    this.projectName.set(settings.defaultProject);
    this.dayOffReason.set('');
    this.useScheduledHoursOverride.set(false);
    this.compTimeHours.set(0);
    this.errorText.set('');
  }

  public copyPrevious(): void {
    const current = this.currentDateString;
    const previous = this.state
      .entries()
      .filter((entry) => entry.date < current && entry.status === WorkEntryStatus.Worked)
      .sort((left, right) => right.date.localeCompare(left.date))[0];
    this.copyEntry(previous);
  }

  public copyLastWeek(): void {
    const date = new Date(`${this.currentDateString}T00:00:00`);
    date.setDate(date.getDate() - 7);
    this.copyEntry(this.state.entries().find((entry) => entry.date === this.toDateString(date)));
  }

  private copyEntry(entry?: WorkEntry): void {
    if (!entry || entry.status !== WorkEntryStatus.Worked) {
      this.errorText.set('No completed day is available to copy.');
      return;
    }
    this.status.set(WorkEntryStatus.Worked);
    this.startTime.set(entry.startTime || '08:00');
    this.endTime.set(entry.endTime || '16:30');
    this.lunchMinutes.set(entry.lunchMinutes);
    this.projectName.set(entry.projectName || this.state.settings().defaultProject);
    this.dayOffReason.set('');
    this.useScheduledHoursOverride.set(entry.scheduledMinutesOverride !== null);
    this.scheduledHours.set((entry.scheduledMinutesOverride || 0) / 60);
    this.compTimeHours.set(0);
    this.errorText.set('');
  }

  private toDateString(date: Date): string {
    const year = date.getFullYear();
    const month = String(date.getMonth() + 1).padStart(2, '0');
    const day = String(date.getDate()).padStart(2, '0');
    return `${year}-${month}-${day}`;
  }

  public onTimeBlur(field: 'start' | 'end', val: string): void {
    const norm = TimeInput.tryNormalize(val);
    if (norm) {
      if (field === 'start') this.startTime.set(norm);
      else this.endTime.set(norm);
    }
  }

  public usesMonthlyHourlyPayBasis(): boolean {
    const settings = this.state.settings();
    return (
      settings.salary.type === SalaryType.Hourly &&
      settings.salary.hourlyPayBasis === HourlyPayBasis.MonthlyExpectedHours &&
      settings.overtimeCompensation.mode === OvertimeCompensationMode.CompTime
    );
  }

  public onStatusChange(status: WorkEntryStatus): void {
    this.status.set(status);
    if (status === WorkEntryStatus.Incomplete) this.compTimeHours.set(0);
  }

  public onDayOffReasonChange(reason: string): void {
    this.dayOffReason.set(reason);
    if (reason === 'Comp time' && this.compTimeHours() === 0) {
      this.compTimeHours.set(this.availableCompTimeHours());
    } else if (reason !== 'Comp time' && this.status() === WorkEntryStatus.Off) {
      this.compTimeHours.set(0);
    }
  }

  public availableCompTimeHours(): number {
    const entry: WorkEntry = {
      date: this.currentDateString,
      status: this.status(),
      startTime: this.status() === WorkEntryStatus.Worked ? this.startTime() : null,
      endTime: this.status() === WorkEntryStatus.Worked ? this.endTime() : null,
      lunchMinutes: this.status() === WorkEntryStatus.Worked ? this.lunchMinutes() : 0,
      projectName: null,
      dayOffReason: this.status() === WorkEntryStatus.Off ? this.dayOffReason() || null : null,
      notes: null,
      scheduledMinutesOverride: this.useScheduledHoursOverride()
        ? Math.round(this.scheduledHours() * 60)
        : null,
      compTimeMinutes: 0,
    };
    return (
      MonthlyCalculations.availableCompTimeMinutes(
        entry,
        this.state.settings().expectedHours,
        this.holidays,
      ) / 60
    );
  }

  public async onSave(saveAndNext = false): Promise<void> {
    const e = this.state.selectedEntry();
    if (!e) return;
    if (
      this.useScheduledHoursOverride() &&
      (!Number.isFinite(this.scheduledHours()) || this.scheduledHours() < 0)
    ) {
      this.errorText.set('Scheduled hours must be zero or more.');
      return;
    }
    const compTimeHours = this.compTimeHours();
    if (
      !Number.isFinite(compTimeHours) ||
      compTimeHours < 0 ||
      compTimeHours > this.availableCompTimeHours()
    ) {
      this.errorText.set(
        `Comp time used must be between 0 and ${this.availableCompTimeHours()} hours.`,
      );
      return;
    }
    if (
      this.status() === WorkEntryStatus.Off &&
      ((this.dayOffReason() === 'Comp time' && compTimeHours === 0) ||
        (this.dayOffReason() !== 'Comp time' && compTimeHours > 0))
    ) {
      this.errorText.set('Select Comp time as the day off type before using comp hours.');
      return;
    }
    const updated: WorkEntry = {
      ...e,
      status: this.status(),
      startTime: this.status() === WorkEntryStatus.Worked ? this.startTime() : null,
      endTime: this.status() === WorkEntryStatus.Worked ? this.endTime() : null,
      lunchMinutes: this.status() === WorkEntryStatus.Worked ? this.lunchMinutes() : 0,
      projectName: this.status() === WorkEntryStatus.Worked ? this.projectName() : null,
      dayOffReason: this.status() === WorkEntryStatus.Off ? this.dayOffReason() || null : null,
      notes: this.notes() || null,
      scheduledMinutesOverride: this.useScheduledHoursOverride()
        ? Math.round(this.scheduledHours() * 60)
        : null,
      compTimeMinutes: Math.round(compTimeHours * 60),
    };
    await this.state.saveEntry(updated);
    if (saveAndNext && this.state.isCatchUpOpen()) this.state.moveCatchUp(1);
    else this.state.closeEditor();
  }

  public async onReset(): Promise<void> {
    const e = this.state.selectedEntry();
    if (!e) return;
    await this.state.deleteEntry(e.date);
  }

  public onClose(): void {
    if (this.state.isCatchUpOpen()) this.state.closeCatchUp();
    else this.state.closeEditor();
  }

  @HostListener('window:dagsverk-save')
  public onKeyboardSave(): void {
    void this.onSave(this.state.isCatchUpOpen());
  }
}
