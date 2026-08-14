import { describe, it, expect } from 'vitest';
import {
  CompensationRateType,
  CompensationRuleType,
  ExpectedHoursSettings,
  MonthRecord,
  OvertimeCompensationMode,
  OvertimeCompensationSettings,
  OvertimeDayCategory,
  OvertimeThresholdMode,
  SalarySettings,
  SalaryType,
  WorkEntry,
  WorkEntryStatus
} from './models';
import { SwedishHolidayService } from './swedish-holiday.service';
import { MinuteMath, MonthlyCalculations, TimeInput } from './monthly-calculations';

describe('MonthlyCalculations & Engine', () => {
  const holidays = new SwedishHolidayService();

  const standardSchedule: ExpectedHoursSettings = {
    hoursPerWorkday: 8,
    workingWeekdays: [1, 2, 3, 4, 5],
    excludePublicHolidays: true
  };

  const hourlySalary: SalarySettings = {
    type: SalaryType.Hourly,
    hourlyRate: 200,
    monthlySalary: 0,
    employmentPercent: 100
  };

  const compTimeOvertime: OvertimeCompensationSettings = {
    mode: OvertimeCompensationMode.CompTime,
    defaultRateType: CompensationRateType.HourlyPremiumPercent,
    defaultRateValue: 50,
    dailyThresholdHours: 8,
    thresholdMode: OvertimeThresholdMode.ScheduledHours,
    rateBands: []
  };

  it('should normalize time strings correctly', () => {
    expect(TimeInput.tryNormalize('8')).toBe('08:00');
    expect(TimeInput.tryNormalize('830')).toBe('08:30');
    expect(TimeInput.tryNormalize('8.30')).toBe('08:30');
    expect(TimeInput.tryNormalize('08:30')).toBe('08:30');
    expect(TimeInput.tryNormalize('invalid')).toBeNull();
  });

  it('should calculate worked minutes accurately', () => {
    expect(MinuteMath.worked('08:00', '16:30', 30)).toBe(480); // 8 hours
    expect(MinuteMath.worked('08:00', '17:00', 30)).toBe(510); // 8.5 hours
    expect(MinuteMath.worked('08:00', '08:30', 45)).toBe(0); // Clamped to 0
  });

  it('should split overtime based on scheduled daily threshold', () => {
    const entry: WorkEntry = {
      date: '2026-08-17', // Monday
      status: WorkEntryStatus.Worked,
      startTime: '08:00',
      endTime: '18:30',
      lunchMinutes: 30, // Worked = 10h (600m)
      projectName: 'General',
      notes: null,
      scheduledMinutesOverride: null
    };

    const split = MonthlyCalculations.splitOvertime(entry, standardSchedule, compTimeOvertime, holidays);
    expect(split.regularMinutes).toBe(480); // 8h
    expect(split.overtimeMinutes).toBe(120); // 2h
  });

  it('should compute monthly summary with time balance roll-forward', () => {
    const monthRecord: MonthRecord = {
      year: 2026,
      month: 8,
      openingBalanceMinutes: 120, // 2h opening balance
      expectedMinutesOverride: null,
      openingBalanceWasEdited: false
    };

    const entries: WorkEntry[] = [
      {
        date: '2026-08-17',
        status: WorkEntryStatus.Worked,
        startTime: '08:00',
        endTime: '17:00',
        lunchMinutes: 30, // 8.5h worked (+30m delta)
        projectName: 'General',
        notes: null,
        scheduledMinutesOverride: null
      }
    ];

    const summary = MonthlyCalculations.calculateMonthlySummary(
      monthRecord,
      entries,
      standardSchedule,
      hourlySalary,
      compTimeOvertime,
      holidays,
      '2026-08-18'
    );

    expect(summary.workedMinutes).toBe(510);
    expect(summary.regularMinutes).toBe(480);
    expect(summary.overtimeMinutes).toBe(30);
    expect(summary.openingBalanceMinutes).toBe(120);
    expect(summary.completedDayCount).toBe(1);
  });
});
