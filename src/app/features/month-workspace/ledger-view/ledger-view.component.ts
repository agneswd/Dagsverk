import { Component, computed, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatTableModule } from '@angular/material/table';
import { MatButtonModule } from '@angular/material/button';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import { MatChipsModule } from '@angular/material/chips';
import { AppStateService } from '../../../core/app-state.service';
import { SwedishHolidayService } from '../../../core/swedish-holiday.service';
import { MinuteMath, MonthlyCalculations } from '../../../core/monthly-calculations';
import { WorkEntry, WorkEntryStatus } from '../../../core/models';

export interface LedgerRow {
  date: string;
  dayOfMonth: number;
  weekdayName: string;
  isToday: boolean;
  isWeekend: boolean;
  holidayName: string | null;
  isScheduledWorkday: boolean;
  status: WorkEntryStatus;
  entry?: WorkEntry;
  startTime: string | null;
  endTime: string | null;
  lunchMinutes: number;
  workedHours: number;
  overtimeHours: number;
  projectName: string | null;
  notes: string | null;
}

@Component({
  selector: 'app-ledger-view',
  standalone: true,
  imports: [
    CommonModule,
    MatTableModule,
    MatButtonModule,
    MatIconModule,
    MatTooltipModule,
    MatChipsModule
  ],
  templateUrl: './ledger-view.component.html',
  styleUrls: ['./ledger-view.component.scss']
})
export class LedgerViewComponent {
  public state = inject(AppStateService);
  private holidays = inject(SwedishHolidayService);

  public readonly WorkEntryStatus = WorkEntryStatus;
  public displayedColumns: string[] = [
    'date',
    'status',
    'interval',
    'lunch',
    'hours',
    'overtime',
    'project',
    'notes',
    'actions'
  ];

  public rows = computed<LedgerRow[]>(() => {
    const y = this.state.currentYear();
    const m = this.state.currentMonth();
    const today = this.state.todayString();
    const expected = this.state.settings().expectedHours;
    const overtime = this.state.settings().overtimeCompensation;

    const entriesMap = new Map<string, WorkEntry>();
    for (const e of this.state.entries()) {
      entriesMap.set(e.date, e);
    }

    const daysInMonth = new Date(Date.UTC(y, m, 0)).getUTCDate();
    const result: LedgerRow[] = [];
    const weekdayNames = ['Sun', 'Mon', 'Tue', 'Wed', 'Thu', 'Fri', 'Sat'];

    for (let day = 1; day <= daysInMonth; day++) {
      const date = `${y}-${String(m).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
      const d = new Date(Date.UTC(y, m - 1, day));
      const dayOfWeek = d.getUTCDay();
      const isWeekend = dayOfWeek === 0 || dayOfWeek === 6;
      const holidayName = this.holidays.getHolidayName(date);
      const isScheduled = MonthlyCalculations.isScheduledWorkday(date, expected, this.holidays);
      const entry = entriesMap.get(date);

      let status = WorkEntryStatus.Incomplete;
      let start: string | null = null;
      let end: string | null = null;
      let lunch = 0;
      let worked = 0;
      let ot = 0;
      let project: string | null = null;
      let notes: string | null = null;

      if (entry) {
        status = entry.status;
        start = entry.startTime;
        end = entry.endTime;
        lunch = entry.lunchMinutes;
        project = entry.projectName;
        notes = entry.notes;

        if (status === WorkEntryStatus.Worked && start && end) {
          const split = MonthlyCalculations.splitOvertime(entry, expected, overtime, this.holidays);
          worked = MinuteMath.worked(start, end, lunch) / 60;
          ot = split.overtimeMinutes / 60;
        }
      }

      result.push({
        date,
        dayOfMonth: day,
        weekdayName: weekdayNames[dayOfWeek],
        isToday: date === today,
        isWeekend,
        holidayName,
        isScheduledWorkday: isScheduled,
        status,
        entry,
        startTime: start,
        endTime: end,
        lunchMinutes: lunch,
        workedHours: worked,
        overtimeHours: ot,
        projectName: project,
        notes
      });
    }

    return result;
  });

  public formatDayNum(day: number): string {
    return String(day).padStart(2, '0');
  }

  public formatAccessibleDate(row: LedgerRow): string {
    const d = new Date(`${row.date}T00:00:00`);
    return d.toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric' });
  }

  public onRowClick(row: LedgerRow): void {
    this.state.openEditor(row.date);
  }

  public onRowKeyDown(event: KeyboardEvent, row: LedgerRow): void {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      this.state.openEditor(row.date);
    }
  }

  public getProjectColor(name: string | null): string {
    if (!name) return '#0B57D0';
    const p = this.state.projects().find(item => item.name.toLowerCase() === name.toLowerCase());
    return p?.color || '#0B57D0';
  }
}
