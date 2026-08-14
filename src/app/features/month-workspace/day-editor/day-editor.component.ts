import { Component, computed, inject, input, output, signal, effect } from '@angular/core';
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
import { AppSettings, WorkEntry, WorkEntryStatus } from '../../../core/models';
import { MinuteMath, MonthlyCalculations, TimeInput } from '../../../core/monthly-calculations';
import { SwedishHolidayService } from '../../../core/swedish-holiday.service';
import { AppStateService } from '../../../core/app-state.service';

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
    MatChipsModule
  ],
  templateUrl: './day-editor.component.html',
  styleUrls: ['./day-editor.component.scss']
})
export class DayEditorComponent {
  public holidays = inject(SwedishHolidayService);
  public state = inject(AppStateService);

  public readonly WorkEntryStatus = WorkEntryStatus;

  public status = signal<WorkEntryStatus>(WorkEntryStatus.Incomplete);
  public startTime = signal<string>('08:00');
  public endTime = signal<string>('16:30');
  public lunchMinutes = signal<number>(30);
  public projectName = signal<string>('General');
  public notes = signal<string>('');

  public timePresets = [
    { label: '08:00 – 16:30', start: '08:00', end: '16:30', lunch: 30 },
    { label: '08:30 – 17:00', start: '08:30', end: '17:00', lunch: 30 },
    { label: '09:00 – 17:30', start: '09:00', end: '17:30', lunch: 30 }
  ];

  public lunchOptions = [0, 30, 45, 60];

  public dayOffReasons = [
    'Vacation / Semester',
    'Sick / Sjuk',
    'VAB',
    'Leave / Tjänstledig',
    'Parental / Föräldraledig',
    'Holiday / Helg'
  ];

  public constructor() {
    effect(() => {
      const e = this.state.selectedEntry();
      const s = this.state.settings();
      if (e) {
        this.status.set(e.status);
        this.startTime.set(e.startTime || s.defaultStartTime || '08:00');
        this.endTime.set(e.endTime || s.defaultEndTime || '16:30');
        this.lunchMinutes.set(e.lunchMinutes ?? s.defaultLunchMinutes ?? 30);
        this.projectName.set(e.projectName || s.defaultProject || 'General');
        this.notes.set(e.notes || '');
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
    return dateObj.toLocaleDateString('en-US', { weekday: 'long', month: 'short', day: 'numeric', year: 'numeric' });
  }

  public get holidayName(): string | null {
    const d = this.state.selectedDate();
    return d ? this.holidays.getHolidayName(d) : null;
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
      notes: this.notes(),
      scheduledMinutesOverride: null
    };
    return MonthlyCalculations.calculateDailyPay(
      fakeEntry,
      this.state.settings().expectedHours,
      this.state.settings().salary,
      this.state.settings().overtimeCompensation,
      this.holidays
    );
  }

  public applyPreset(preset: { start: string; end: string; lunch: number }): void {
    this.status.set(WorkEntryStatus.Worked);
    this.startTime.set(preset.start);
    this.endTime.set(preset.end);
    this.lunchMinutes.set(preset.lunch);
  }

  public onTimeBlur(field: 'start' | 'end', val: string): void {
    const norm = TimeInput.tryNormalize(val);
    if (norm) {
      if (field === 'start') this.startTime.set(norm);
      else this.endTime.set(norm);
    }
  }

  public onSave(): void {
    const e = this.state.selectedEntry();
    if (!e) return;
    const updated: WorkEntry = {
      ...e,
      status: this.status(),
      startTime: this.status() === WorkEntryStatus.Worked ? this.startTime() : null,
      endTime: this.status() === WorkEntryStatus.Worked ? this.endTime() : null,
      lunchMinutes: this.status() === WorkEntryStatus.Worked ? this.lunchMinutes() : 0,
      projectName: this.status() === WorkEntryStatus.Worked ? this.projectName() : null,
      notes: this.notes() || null
    };
    this.state.saveEntry(updated);
    this.state.closeEditor();
  }

  public onReset(): void {
    const e = this.state.selectedEntry();
    if (!e) return;
    const cleared: WorkEntry = {
      ...e,
      status: WorkEntryStatus.Incomplete,
      startTime: null,
      endTime: null,
      lunchMinutes: 0,
      projectName: null,
      notes: null
    };
    this.state.saveEntry(cleared);
    this.state.closeEditor();
  }

  public onClose(): void {
    this.state.closeEditor();
  }
}
