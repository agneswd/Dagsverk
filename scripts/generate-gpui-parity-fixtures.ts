import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
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
} from '../src/app/core/models';
import {
  MinuteMath,
  MonthlyCalculations,
  OvertimeEngine,
  TimeInput,
} from '../src/app/core/monthly-calculations';
import { SwedishHolidayService } from '../src/app/core/swedish-holiday.service';

const root = join(import.meta.dirname, '..');
const outputDirectory = join(root, 'gpui', 'fixtures', 'parity');
const sourceCommit = execFileSync('git', ['rev-parse', 'HEAD'], {
  cwd: root,
  encoding: 'utf8',
}).trim();
const generatedAt = execFileSync('git', ['show', '-s', '--format=%cI', sourceCommit], {
  cwd: root,
  encoding: 'utf8',
}).trim();
const holidays = new SwedishHolidayService();

const expectedHours: ExpectedHoursSettings = {
  hoursPerWorkday: 8,
  workingWeekdays: [1, 2, 3, 4, 5],
  excludePublicHolidays: true,
};
const salary: SalarySettings = {
  type: SalaryType.Hourly,
  hourlyRate: 200,
  monthlySalary: 0,
  employmentPercent: 100,
  hourlyPayBasis: HourlyPayBasis.DailyRegularHours,
};
const overtime: OvertimeCompensationSettings = {
  mode: OvertimeCompensationMode.CompTime,
  defaultRateType: CompensationRateType.HourlyPremiumPercent,
  defaultRateValue: 50,
  dailyThresholdHours: 8,
  thresholdMode: OvertimeThresholdMode.ScheduledHours,
  rateBands: [],
  obOvertimeCombination: ObOvertimeCombinationMode.ExcludeOb,
};

function fixture(cases: unknown[]) {
  return { sourceCommit, generatedAt, schemaVersion: 1, cases };
}

function write(name: string, cases: unknown[]) {
  writeFileSync(join(outputDirectory, name), `${JSON.stringify(fixture(cases), null, 2)}\n`);
}

function worked(
  date: string,
  startTime: string,
  endTime: string,
  lunchMinutes = 0,
  scheduledMinutesOverride: number | null = null,
): WorkEntry {
  return {
    date,
    status: WorkEntryStatus.Worked,
    startTime,
    endTime,
    lunchMinutes,
    projectName: null,
    notes: null,
    scheduledMinutesOverride,
  };
}

mkdirSync(outputDirectory, { recursive: true });

write(
  'time.json',
  [
    '',
    '   ',
    '8',
    '08',
    '830',
    '0830',
    '8.30',
    '8:3',
    '08:30',
    'invalid',
    '24:00',
    '-1:00',
    '08:60',
  ].map((input) => ({ input, output: TimeInput.tryNormalize(input) })),
);

write(
  'minutes.json',
  [
    ['normal', '08:00', '16:30', 30],
    ['equal', '08:00', '08:00', 0],
    ['lunch-longer-than-shift', '08:00', '08:30', 45],
    ['overnight', '22:00', '06:00', 0],
    ['cross-midnight-with-lunch', '21:30', '06:15', 30],
    ['no-lunch', '08:00', '17:00', 0],
  ].map(([name, startTime, endTime, lunchMinutes]) => ({
    name,
    input: { startTime, endTime, lunchMinutes },
    output: {
      elapsed: MinuteMath.elapsed(startTime as string, endTime as string),
      worked: MinuteMath.worked(startTime as string, endTime as string, lunchMinutes as number),
    },
  })),
);

const thresholdEntries = [
  worked('2026-08-17', '08:00', '18:30', 30),
  worked('2026-08-16', '08:00', '12:00'),
  worked('2026-12-25', '08:00', '12:00'),
  worked('2026-08-17', '08:00', '12:00', 0, 0),
  worked('2026-08-17', '08:00', '18:00', 0, 360),
];
write(
  'overtime-threshold.json',
  thresholdEntries.flatMap((entry) =>
    [OvertimeThresholdMode.ScheduledHours, OvertimeThresholdMode.FixedDailyHours].map(
      (thresholdMode) => ({
        input: { entry, expectedHours, overtime: { ...overtime, thresholdMode } },
        output: {
          thresholdMinutes: MonthlyCalculations.thresholdForEntry(
            entry,
            expectedHours,
            { ...overtime, thresholdMode },
            holidays,
          ),
          split: MonthlyCalculations.splitOvertime(
            entry,
            expectedHours,
            { ...overtime, thresholdMode },
            holidays,
          ),
        },
      }),
    ),
  ),
);

const categories = Object.values(OvertimeDayCategory).filter(
  (value): value is OvertimeDayCategory => typeof value === 'number',
);
write(
  'rate-bands.json',
  categories.flatMap((dayCategory) =>
    [
      ['18:00', '22:00', '18:00'],
      ['18:00', '22:00', '22:00'],
      ['22:00', '06:00', '23:59'],
      ['22:00', '06:00', '06:00'],
      ['00:00', '00:00', '12:00'],
    ].map(([startTime, endTime, time]) => ({
      input: {
        band: {
          name: 'Fixture',
          dayCategory,
          startTime,
          endTime,
          compensationType: CompensationRuleType.Overtime,
          rateType: CompensationRateType.FixedHourlyAmount,
          rateValue: 100,
        },
        compensationType: CompensationRuleType.Overtime,
        date: '2026-08-16',
        time,
        isScheduledWorkday: false,
        isPublicHoliday: true,
        isMajorHoliday: false,
      },
      output: OvertimeEngine.matchesRateBand(
        {
          name: 'Fixture',
          dayCategory,
          startTime,
          endTime,
          compensationType: CompensationRuleType.Overtime,
          rateType: CompensationRateType.FixedHourlyAmount,
          rateValue: 100,
        },
        CompensationRuleType.Overtime,
        '2026-08-16',
        time,
        false,
        true,
        false,
      ),
    })),
  ),
);

const dailyPayCases = [
  {
    name: 'decimal-hourly',
    entry: worked('2026-07-01', '08:00', '09:30'),
    salary: { ...salary, hourlyRate: 123.45 },
    overtime,
  },
  {
    name: 'paid-overtime',
    entry: worked('2026-07-01', '08:00', '18:30', 30),
    salary,
    overtime: { ...overtime, mode: OvertimeCompensationMode.Paid },
  },
  {
    name: 'overnight-ob',
    entry: worked('2026-07-01', '22:00', '06:00'),
    salary,
    overtime: {
      ...overtime,
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
    },
  },
  {
    name: 'monthly-salary-divisor',
    entry: worked('2026-07-01', '08:00', '18:00'),
    salary: {
      ...salary,
      type: SalaryType.Monthly,
      hourlyRate: 0,
      monthlySalary: 40000,
      employmentPercent: 80,
    },
    overtime: {
      ...overtime,
      mode: OvertimeCompensationMode.Paid,
      defaultRateType: CompensationRateType.FullTimeMonthlySalaryDivisor,
      defaultRateValue: 165,
    },
  },
];
write(
  'daily-pay.json',
  dailyPayCases.map((testCase) => ({
    input: testCase,
    output: MonthlyCalculations.calculateDailyPay(
      testCase.entry,
      expectedHours,
      testCase.salary,
      testCase.overtime,
      holidays,
    ),
  })),
);

const monthRecord: MonthRecord = {
  year: 2026,
  month: 8,
  openingBalanceMinutes: 120,
  expectedMinutesOverride: null,
  openingBalanceWasEdited: false,
};
const monthlyEntries = [
  worked('2026-08-03', '08:00', '16:30', 30),
  worked('2026-08-04', '08:00', '18:30', 30),
  { ...worked('2026-08-05', '08:00', '16:30', 30), status: WorkEntryStatus.Off },
  { ...worked('2026-08-06', '08:00', '16:30', 30), status: WorkEntryStatus.Incomplete },
];
write(
  'monthly-summary.json',
  [
    { name: 'empty-current', entries: [], today: '2026-08-14', record: monthRecord },
    {
      name: 'worked-off-incomplete-current',
      entries: monthlyEntries,
      today: '2026-08-14',
      record: monthRecord,
    },
    {
      name: 'past-month-override',
      entries: monthlyEntries,
      today: '2026-09-01',
      record: { ...monthRecord, expectedMinutesOverride: 6000 },
    },
    {
      name: 'future-month',
      entries: monthlyEntries,
      today: '2026-07-01',
      record: monthRecord,
    },
  ].map((testCase) => ({
    input: testCase,
    output: MonthlyCalculations.calculateMonthlySummary(
      testCase.record,
      testCase.entries,
      expectedHours,
      salary,
      overtime,
      holidays,
      testCase.today,
    ),
  })),
);

const holidayCases = [];
for (let year = 2024; year <= 2035; year++) {
  holidayCases.push({
    year,
    named: holidays.getHolidays(year),
    sundays: MonthlyCalculations.getDatesInMonth(year, 1)
      .concat(
        ...Array.from({ length: 11 }, (_, index) =>
          MonthlyCalculations.getDatesInMonth(year, index + 2),
        ),
      )
      .filter((date) => new Date(`${date}T00:00:00Z`).getUTCDay() === 0)
      .map((date) => ({
        date,
        isPublicHoliday: holidays.isPublicHoliday(date),
        name: holidays.getHolidayName(date),
      })),
    majorBoundaries: [
      `${year}-12-23T18:59`,
      `${year}-12-23T19:00`,
      `${year}-12-30T18:59`,
      `${year}-12-30T19:00`,
      `${year + 1}-01-02T07:00`,
    ].map((value) => {
      const [date, time] = value.split('T');
      return { date, time, output: holidays.isMajorHolidayPeriod(date, time) };
    }),
  });
}
write('holidays.json', holidayCases);

console.log(`Wrote parity fixtures to ${outputDirectory}`);
