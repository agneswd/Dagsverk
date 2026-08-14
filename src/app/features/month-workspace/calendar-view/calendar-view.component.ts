import { Component, computed, inject } from '@angular/core';
import { CommonModule } from '@angular/common';
import { MatCardModule } from '@angular/material/card';
import { MatIconModule } from '@angular/material/icon';
import { MatTooltipModule } from '@angular/material/tooltip';
import { AppStateService } from '../../../core/app-state.service';
import { SwedishHolidayService } from '../../../core/swedish-holiday.service';
import { MinuteMath, MonthlyCalculations } from '../../../core/monthly-calculations';
import { WorkEntry, WorkEntryStatus } from '../../../core/models';

export interface CalendarDayCell {
  date: string;
  dayOfMonth: number;
  isCurrentMonth: boolean;
  isToday: boolean;
  isWeekend: boolean;
  holidayName: string | null;
  isScheduledWorkday: boolean;
  entry?: WorkEntry;
  status: WorkEntryStatus;
  startTime?: string | null;
  endTime?: string | null;
  workedHours: number;
  overtimeHours: number;
  projectName: string | null;
  notes: string | null;
}

@Component({
  selector: 'app-calendar-view',
  standalone: true,
  imports: [CommonModule, MatCardModule, MatIconModule, MatTooltipModule],
  templateUrl: './calendar-view.component.html',
  styleUrls: ['./calendar-view.component.scss']
})
export class CalendarViewComponent {
  public state = inject(AppStateService);
  private holidays = inject(SwedishHolidayService);

  public readonly WorkEntryStatus = WorkEntryStatus;
  public readonly weekdays = ['MON', 'TUE', 'WED', 'THU', 'FRI', 'SAT', 'SUN'];

  public calendarCells = computed<CalendarDayCell[]>(() => {
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
    const firstDayOfWeek = (new Date(Date.UTC(y, m - 1, 1)).getUTCDay() + 6) % 7; // 0=Mon, 6=Sun

    const cells: CalendarDayCell[] = [];

    // Leading padding days from previous month
    const prevMonthDays = new Date(Date.UTC(y, m - 1, 0)).getUTCDate();
    for (let i = firstDayOfWeek - 1; i >= 0; i--) {
      const day = prevMonthDays - i;
      const prevM = m === 1 ? 12 : m - 1;
      const prevY = m === 1 ? y - 1 : y;
      const date = `${prevY}-${String(prevM).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
      cells.push({
        date,
        dayOfMonth: day,
        isCurrentMonth: false,
        isToday: date === today,
        isWeekend: true,
        holidayName: null,
        isScheduledWorkday: false,
        status: WorkEntryStatus.Incomplete,
        startTime: null,
        endTime: null,
        workedHours: 0,
        overtimeHours: 0,
        projectName: null,
        notes: null
      });
    }

    // Days in current month
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
      let worked = 0;
      let ot = 0;
      let project: string | null = null;
      let notes: string | null = null;

      if (entry) {
        status = entry.status;
        start = entry.startTime;
        end = entry.endTime;
        project = entry.projectName;
        notes = entry.notes;
        if (status === WorkEntryStatus.Worked && entry.startTime && entry.endTime) {
          const split = MonthlyCalculations.splitOvertime(entry, expected, overtime, this.holidays);
          worked = MinuteMath.worked(entry.startTime, entry.endTime, entry.lunchMinutes) / 60;
          ot = split.overtimeMinutes / 60;
        }
      }

      cells.push({
        date,
        dayOfMonth: day,
        isCurrentMonth: true,
        isToday: date === today,
        isWeekend,
        holidayName,
        isScheduledWorkday: isScheduled,
        entry,
        status,
        startTime: start,
        endTime: end,
        workedHours: worked,
        overtimeHours: ot,
        projectName: project,
        notes
      });
    }

    // Trailing padding days to complete grid (multiples of 7)
    const remaining = (7 - (cells.length % 7)) % 7;
    for (let i = 1; i <= remaining; i++) {
      const nextM = m === 12 ? 1 : m + 1;
      const nextY = m === 12 ? y + 1 : y;
      const date = `${nextY}-${String(nextM).padStart(2, '0')}-${String(i).padStart(2, '0')}`;
      cells.push({
        date,
        dayOfMonth: i,
        isCurrentMonth: false,
        isToday: date === today,
        isWeekend: true,
        holidayName: null,
        isScheduledWorkday: false,
        status: WorkEntryStatus.Incomplete,
        startTime: null,
        endTime: null,
        workedHours: 0,
        overtimeHours: 0,
        projectName: null,
        notes: null
      });
    }

    return cells;
  });

  public formatAccessibleCellName(cell: CalendarDayCell): string {
    const d = new Date(`${cell.date}T00:00:00`);
    const dateStr = d.toLocaleDateString('en-US', { weekday: 'long', month: 'long', day: 'numeric' });
    if (cell.holidayName) return `${dateStr}, Holiday ${cell.holidayName}`;
    if (cell.status === WorkEntryStatus.Worked) return `${dateStr}, Worked ${cell.workedHours.toFixed(1)} hours`;
    if (cell.status === WorkEntryStatus.Off) return `${dateStr}, Day off`;
    if (cell.isScheduledWorkday) return `${dateStr}, unlogged`;
    return `${dateStr}, rest day`;
  }

  public onCellClick(cell: CalendarDayCell): void {
    if (cell.isCurrentMonth) {
      this.state.openEditor(cell.date);
    }
  }

  public onCellKeyDown(event: KeyboardEvent, cell: CalendarDayCell): void {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault();
      this.onCellClick(cell);
    }
  }

  public getProjectColor(name: string | null): string {
    if (!name) return '#0B57D0';
    const p = this.state.projects().find(item => item.name.toLowerCase() === name.toLowerCase());
    return p?.color || '#0B57D0';
  }
}
