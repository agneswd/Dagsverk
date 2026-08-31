import { describe, it, expect } from 'vitest';
import {
  CompensationRateType,
  CompensationRuleType,
  ExpectedHoursSettings,
  HourlyPayBasis,
  MonthRecord,
  ObOvertimeCombinationMode,
  OvertimeCompensationMode,
  OvertimeCompensationSettings,
  OvertimeDayCategory,
  OvertimeThresholdMode,
  SalarySettings,
  SalaryType,
  WorkEntry,
  WorkEntryStatus,
} from './models';
import { SwedishHolidayService } from './swedish-holiday.service';
import { MinuteMath, MonthlyCalculations, TimeInput } from './monthly-calculations';

describe('MonthlyCalculations & Engine', () => {
  const holidays = new SwedishHolidayService();

  const standardSchedule: ExpectedHoursSettings = {
    hoursPerWorkday: 8,
    workingWeekdays: [1, 2, 3, 4, 5],
    excludePublicHolidays: true,
  };

  const hourlySalary: SalarySettings = {
    type: SalaryType.Hourly,
    hourlyRate: 200,
    monthlySalary: 0,
    employmentPercent: 100,
    hourlyPayBasis: HourlyPayBasis.DailyRegularHours,
  };

  const compTimeOvertime: OvertimeCompensationSettings = {
    mode: OvertimeCompensationMode.CompTime,
    defaultRateType: CompensationRateType.HourlyPremiumPercent,
    defaultRateValue: 50,
    dailyThresholdHours: 8,
    thresholdMode: OvertimeThresholdMode.ScheduledHours,
    rateBands: [],
    obOvertimeCombination: ObOvertimeCombinationMode.ExcludeOb,
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
      dayOffReason: null,
      notes: null,
      scheduledMinutesOverride: null,
      compTimeMinutes: 0,
    };

    const split = MonthlyCalculations.splitOvertime(
      entry,
      standardSchedule,
      compTimeOvertime,
      holidays,
    );
    expect(split.regularMinutes).toBe(480); // 8h
    expect(split.overtimeMinutes).toBe(120); // 2h
  });

  it('limits comp time to the unworked scheduled time', () => {
    const partialDay = workedEntry('2026-08-28', 330);
    const dayOff: WorkEntry = {
      ...partialDay,
      status: WorkEntryStatus.Off,
      startTime: null,
      endTime: null,
    };

    expect(
      MonthlyCalculations.availableCompTimeMinutes(
        partialDay,
        standardSchedule,
        holidays,
      ),
    ).toBe(150);
    expect(
      MonthlyCalculations.availableCompTimeMinutes(
        dayOff,
        standardSchedule,
        holidays,
      ),
    ).toBe(480);
    expect(
      MonthlyCalculations.availableCompTimeMinutes(
        { ...dayOff, date: '2026-08-29' },
        standardSchedule,
        holidays,
      ),
    ).toBe(0);
  });

  it('does not pay stored comp time beyond the scheduled availability', () => {
    const weekday = {
      ...workedEntry('2026-08-03', 0),
      status: WorkEntryStatus.Off,
      startTime: null,
      endTime: null,
      dayOffReason: 'Comp time',
      compTimeMinutes: 600,
    };
    const weekend = { ...weekday, date: '2026-08-08', compTimeMinutes: 480 };

    const summary = MonthlyCalculations.calculateMonthlySummary(
      month(2026, 8, 168 * 60),
      [weekday, weekend],
      standardSchedule,
      { ...hourlySalary, hourlyPayBasis: HourlyPayBasis.MonthlyExpectedHours },
      compTimeOvertime,
      holidays,
      '2026-08-31',
    );

    expect(summary.compTimeUsedMinutes).toBe(480);
    expect(summary.ordinaryPaidMinutes).toBe(480);
  });

  it('should compute monthly summary with time balance roll-forward', () => {
    const monthRecord: MonthRecord = {
      year: 2026,
      month: 8,
      openingBalanceMinutes: 120, // 2h opening balance
      expectedMinutesOverride: null,
      openingBalanceWasEdited: false,
    };

    const entries: WorkEntry[] = [
      {
        date: '2026-08-17',
        status: WorkEntryStatus.Worked,
        startTime: '08:00',
        endTime: '17:00',
        lunchMinutes: 30, // 8.5h worked (+30m delta)
        projectName: 'General',
        dayOffReason: null,
        notes: null,
        scheduledMinutesOverride: null,
        compTimeMinutes: 0,
      },
    ];

    const summary = MonthlyCalculations.calculateMonthlySummary(
      monthRecord,
      entries,
      standardSchedule,
      hourlySalary,
      compTimeOvertime,
      holidays,
      '2026-08-18',
    );

    expect(summary.workedMinutes).toBe(510);
    expect(summary.regularMinutes).toBe(480);
    expect(summary.overtimeMinutes).toBe(30);
    expect(summary.openingBalanceMinutes).toBe(120);
    expect(summary.completedDayCount).toBe(1);
  });

  it('accrues current-month expected hours only through today', () => {
    const entries = [
      [3, 480],
      [4, 480],
      [5, 480],
      [6, 480],
      [7, 480],
      [10, 540],
      [11, 480],
      [12, 690],
      [13, 810],
    ].map(([day, minutes]) => workedEntry(`2026-08-${String(day).padStart(2, '0')}`, minutes));

    const summary = MonthlyCalculations.calculateMonthlySummary(
      month(2026, 8, 154 * 60),
      entries,
      { ...standardSchedule, excludePublicHolidays: false },
      { ...hourlySalary, hourlyRate: 202 },
      compTimeOvertime,
      holidays,
      '2026-08-14',
    );

    expect(summary.workedHours).toBe(82);
    expect(summary.expectedMinutes).toBe(4400);
    expect(summary.closingBalanceMinutes).toBe(520);
  });

  it('uses decimal-safe daily salary arithmetic', () => {
    const pay = MonthlyCalculations.calculateDailyPay(
      workedEntry('2026-07-01', 90, 0),
      standardSchedule,
      { ...hourlySalary, hourlyRate: 123.45 },
      compTimeOvertime,
      holidays,
    );

    expect(pay.regularPay).toBe(185.18);
  });

  it('matches Tidverk monthly expected-hours pay basis', () => {
    const shifts = [
      [8, 450],
      [9, 510],
      [10, 630],
      [11, 480],
      [12, 390],
      [15, 510],
      [16, 480],
      [17, 480],
      [18, 510],
      [19, 480],
      [22, 570],
      [23, 450],
      [24, 510],
      [25, 480],
      [26, 540],
      [29, 480],
      [30, 510],
    ];
    const entries = shifts.map(([day, minutes]) =>
      workedEntry(`2026-06-${String(day).padStart(2, '0')}`, minutes),
    );
    const summary = MonthlyCalculations.calculateMonthlySummary(
      month(2026, 6, 136 * 60),
      entries,
      standardSchedule,
      {
        ...hourlySalary,
        hourlyRate: 180,
        hourlyPayBasis: HourlyPayBasis.MonthlyExpectedHours,
      },
      compTimeOvertime,
      holidays,
      '2026-07-01',
    );

    expect(summary.workedHours).toBe(141);
    expect(summary.regularHours).toBe(133.5);
    expect(summary.overtimeHours).toBe(7.5);
    expect(summary.ordinaryPaidHours).toBe(136);
    expect(summary.grossSalary).toBe(24480);
    expect(summary.monthlyDifferenceMinutes).toBe(300);
  });

  it('pays used comp time while preserving worked time and the net balance', () => {
    const workedDays = [
      [3, 480],
      [4, 480],
      [5, 480],
      [6, 480],
      [7, 480],
      [10, 540],
      [11, 480],
      [12, 690],
      [13, 810],
      [17, 570],
      [18, 510],
      [19, 600],
      [20, 810],
      [25, 510],
      [26, 510],
      [27, 480],
      [28, 330],
    ].map(([day, minutes]) => workedEntry(`2026-08-${String(day).padStart(2, '0')}`, minutes));
    const compDays = [14, 21, 24, 31].map<WorkEntry>((day) => ({
      date: `2026-08-${day}`,
      status: WorkEntryStatus.Off,
      startTime: null,
      endTime: null,
      lunchMinutes: 0,
      projectName: null,
      dayOffReason: 'Comp time',
      notes: null,
      scheduledMinutesOverride: null,
      compTimeMinutes: 480,
    }));

    const summary = MonthlyCalculations.calculateMonthlySummary(
      month(2026, 8, 168 * 60),
      [...workedDays, ...compDays],
      standardSchedule,
      {
        ...hourlySalary,
        hourlyRate: 202,
        hourlyPayBasis: HourlyPayBasis.MonthlyExpectedHours,
      },
      compTimeOvertime,
      holidays,
      '2026-08-31',
    );

    expect(summary.workedHours).toBe(154);
    expect(summary.ordinaryPaidHours).toBe(168);
    expect(summary.compTimeEarnedHours).toBe(18);
    expect(summary.compTimeUsedHours).toBe(32);
    expect(summary.monthlyDifferenceMinutes).toBe(-14 * 60);
    expect(summary.grossSalary).toBe(33936);
  });

  it('uses accrued expected hours for in-progress monthly pay and comp time', () => {
    const entries = [
      [3, 480],
      [4, 480],
      [5, 480],
      [6, 480],
      [7, 480],
      [10, 540],
      [11, 480],
      [12, 690],
      [13, 810],
      [17, 480],
    ].map(([day, minutes]) => workedEntry(`2026-08-${String(day).padStart(2, '0')}`, minutes));

    const summary = MonthlyCalculations.calculateMonthlySummary(
      month(2026, 8, null),
      entries,
      { ...standardSchedule, excludePublicHolidays: false },
      {
        ...hourlySalary,
        hourlyRate: 202,
        hourlyPayBasis: HourlyPayBasis.MonthlyExpectedHours,
      },
      compTimeOvertime,
      holidays,
      '2026-08-17',
    );

    expect(summary.workedHours).toBe(90);
    expect(summary.expectedHours).toBe(88);
    expect(summary.ordinaryPaidHours).toBe(88);
    expect(summary.grossSalary).toBe(17776);
    expect(summary.monthlyDifferenceMinutes).toBe(120);
  });

  it('counts overnight work and applies OB on the real clock date', () => {
    const overnight = workedEntry('2026-07-01', 8 * 60, 0, '22:00');
    const compensation: OvertimeCompensationSettings = {
      ...compTimeOvertime,
      obOvertimeCombination: ObOvertimeCombinationMode.IncludeOb,
      rateBands: [
        {
          name: 'Night OB',
          dayCategory: OvertimeDayCategory.AllDays,
          startTime: '22:00',
          endTime: '06:00',
          compensationType: CompensationRuleType.Ob,
          rateType: CompensationRateType.FixedHourlyAmount,
          rateValue: 50,
        },
      ],
    };

    const pay = MonthlyCalculations.calculateDailyPay(
      overnight,
      standardSchedule,
      hourlySalary,
      compensation,
      holidays,
    );

    expect(MinuteMath.worked(overnight.startTime, overnight.endTime, 0)).toBe(480);
    expect(pay.obMinutes).toBe(480);
    expect(pay.obPay).toBe(400);
  });

  it('places lunch between ordinary and overtime clock blocks', () => {
    const compensation: OvertimeCompensationSettings = {
      ...compTimeOvertime,
      obOvertimeCombination: ObOvertimeCombinationMode.IncludeOb,
      rateBands: [
        {
          name: 'Evening OB',
          dayCategory: OvertimeDayCategory.AllDays,
          startTime: '18:00',
          endTime: '22:00',
          compensationType: CompensationRuleType.Ob,
          rateType: CompensationRateType.FixedHourlyAmount,
          rateValue: 40,
        },
      ],
    };

    const pay = MonthlyCalculations.calculateDailyPay(
      workedEntry('2026-07-01', 600, 60),
      standardSchedule,
      hourlySalary,
      compensation,
      holidays,
    );

    expect(pay.obMinutes).toBe(60);
    expect(pay.obPay).toBe(40);
  });

  it('excludes paid overtime from time balance', () => {
    const paid: OvertimeCompensationSettings = {
      ...compTimeOvertime,
      mode: OvertimeCompensationMode.Paid,
    };
    const summary = MonthlyCalculations.calculateMonthlySummary(
      month(2026, 7, 8 * 60),
      [workedEntry('2026-07-01', 10 * 60)],
      standardSchedule,
      hourlySalary,
      paid,
      holidays,
      '2026-08-01',
    );

    expect(summary.overtimeHours).toBe(2);
    expect(summary.grossSalary).toBe(2200);
    expect(summary.closingBalanceMinutes).toBe(0);
  });
});

function month(
  year: number,
  monthNumber: number,
  expectedMinutesOverride: number | null,
): MonthRecord {
  return {
    year,
    month: monthNumber,
    openingBalanceMinutes: 0,
    expectedMinutesOverride,
    openingBalanceWasEdited: false,
  };
}

function workedEntry(
  date: string,
  workedMinutes: number,
  lunchMinutes = 30,
  startTime = '08:00',
): WorkEntry {
  const endTime = TimeInput.fromMinutes(
    TimeInput.toMinutes(startTime) + workedMinutes + lunchMinutes,
  );
  return {
    date,
    status: WorkEntryStatus.Worked,
    startTime,
    endTime,
    lunchMinutes,
    projectName: null,
    dayOffReason: null,
    notes: null,
    scheduledMinutesOverride: null,
    compTimeMinutes: 0,
  };
}
