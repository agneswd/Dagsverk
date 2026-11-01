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
  TimeRange,
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
  // One period for a normal day, more for a split shift. Gaps become unpaid breaks.
  public periods = signal<TimeRange[]>([{ start: '08:00', end: '16:30' }]);
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
        this.periods.set(
          e.startTime && e.endTime
            ? MinuteMath.periods(e)
            : [{ start: s.defaultStartTime || '08:00', end: s.defaultEndTime || '16:30' }],
        );
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
    return MinuteMath.workedForEntry(this.draftEntry()) / 60;
  }

  public get dailyPay(): { regularPay: number; overtimePay: number; obPay: number; total: number } {
    if (this.status() !== WorkEntryStatus.Worked) {
      return { regularPay: 0, overtimePay: 0, obPay: 0, total: 0 };
    }
    return MonthlyCalculations.calculateDailyPay(
      this.draftEntry(),
      this.state.settings().expectedHours,
      this.state.settings().salary,
      this.state.settings().overtimeCompensation,
      this.holidays,
    );
  }

  public isPresetSelected(preset: { start: string; end: string; lunch: number }): boolean {
    const periods = this.periods();
    return (
      periods.length === 1 &&
      periods[0].start === preset.start &&
      periods[0].end === preset.end &&
      this.lunchMinutes() === preset.lunch
    );
  }

  public applyPreset(preset: { start: string; end: string; lunch: number }): void {
    this.status.set(WorkEntryStatus.Worked);
    this.periods.set([{ start: preset.start, end: preset.end }]);
    this.lunchMinutes.set(preset.lunch);
  }

  // Adds a period that starts one hour after the last one ends.
  public addPeriod(): void {
    const last = this.periods()[this.periods().length - 1];
    const start = TimeInput.fromMinutes(TimeInput.toMinutes(last.end) + 60);
    const end = TimeInput.fromMinutes(TimeInput.toMinutes(start) + 120);
    this.periods.set([...this.periods(), { start, end }]);
  }

  public removePeriod(index: number): void {
    this.periods.set(this.periods().filter((_, i) => i !== index));
  }

  public setPeriodTime(index: number, field: keyof TimeRange, value: string): void {
    this.periods.set(
      this.periods().map((period, i) => (i === index ? { ...period, [field]: value } : period)),
    );
  }

  public onLunchInput(value: number | null): void {
    this.lunchMinutes.set(Number.isFinite(value) ? Number(value) : 0);
  }

  public applyNormalDay(): void {
    const settings = this.state.settings();
    this.status.set(WorkEntryStatus.Worked);
    this.periods.set([{ start: settings.defaultStartTime, end: settings.defaultEndTime }]);
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
    this.periods.set(
      entry.startTime && entry.endTime
        ? MinuteMath.periods(entry)
        : [{ start: '08:00', end: '16:30' }],
    );
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

  public onTimeBlur(index: number, field: keyof TimeRange): void {
    const norm = TimeInput.tryNormalize(this.periods()[index][field]);
    if (norm) this.setPeriodTime(index, field, norm);
  }

  // The form state as an entry, used for live hours, pay, and comp time.
  private draftEntry(): WorkEntry {
    const worked = this.status() === WorkEntryStatus.Worked;
    const times = worked
      ? MinuteMath.fromPeriods(this.periods())
      : { startTime: null, endTime: null, breaks: [] };
    return {
      date: this.currentDateString,
      status: this.status(),
      ...times,
      lunchMinutes: worked ? this.lunchMinutes() : 0,
      projectName: worked ? this.projectName() : null,
      dayOffReason: this.status() === WorkEntryStatus.Off ? this.dayOffReason() || null : null,
      notes: this.notes() || null,
      scheduledMinutesOverride: this.useScheduledHoursOverride()
        ? Math.round(this.scheduledHours() * 60)
        : null,
      compTimeMinutes: Math.round(this.compTimeHours() * 60),
    };
  }

  // Returns an error text when the times or the lunch cannot be saved.
  private timesError(): string | null {
    if (this.status() !== WorkEntryStatus.Worked) return null;
    const periods = this.periods();
    if (periods.some((p) => !TimeInput.tryNormalize(p.start) || !TimeInput.tryNormalize(p.end))) {
      return 'Enter all times as HH:mm.';
    }
    const periodsError = MinuteMath.validatePeriods(periods);
    if (periodsError) return periodsError;
    const lunch = this.lunchMinutes();
    if (!Number.isInteger(lunch) || lunch < 0) return 'Lunch must be zero or more whole minutes.';
    const paidMinutes = MinuteMath.worked(
      periods[0].start,
      periods[periods.length - 1].end,
      0,
      MinuteMath.fromPeriods(periods).breaks,
    );
    if (lunch >= paidMinutes) return 'Lunch must be shorter than the time worked.';
    return null;
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
    const entry: WorkEntry = { ...this.draftEntry(), compTimeMinutes: 0 };
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
    this.periods.set(
      this.periods().map((period) => ({
        start: TimeInput.tryNormalize(period.start) ?? period.start,
        end: TimeInput.tryNormalize(period.end) ?? period.end,
      })),
    );
    const timesError = this.timesError();
    if (timesError) {
      this.errorText.set(timesError);
      return;
    }
    if (
      this.useScheduledHoursOverride() &&
      (!Number.isFinite(this.scheduledHours()) || this.scheduledHours() < 0)
    ) {
      this.errorText.set('Scheduled hours must be zero or more.');
      return;
    }
    const compTimeHours = this.compTimeHours();
    const compTimeMinutes = Math.round(compTimeHours * 60);
    const availableCompTimeMinutes = Math.round(this.availableCompTimeHours() * 60);
    if (
      !Number.isFinite(compTimeHours) ||
      compTimeMinutes < 0 ||
      compTimeMinutes > availableCompTimeMinutes
    ) {
      this.errorText.set(
        `Comp time used must be between 0 and ${this.availableCompTimeHours()} hours.`,
      );
      return;
    }
    if (
      this.status() === WorkEntryStatus.Off &&
      ((this.dayOffReason() === 'Comp time' && compTimeMinutes === 0) ||
        (this.dayOffReason() !== 'Comp time' && compTimeMinutes > 0))
    ) {
      this.errorText.set('Select Comp time as the day off type before using comp hours.');
      return;
    }
    const updated: WorkEntry = { ...e, ...this.draftEntry(), compTimeMinutes };
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
