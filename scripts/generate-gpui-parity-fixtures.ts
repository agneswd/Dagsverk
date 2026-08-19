import { execFileSync } from 'node:child_process';
import { readFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import '@angular/compiler';
import { createEnvironmentInjector, runInInjectionContext } from '@angular/core';
import { HttpClient } from '@angular/common/http';
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
  TaxMode,
  TaxSettings,
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
import { matchingWeekdayOccurrence } from '../src/app/core/app-state.service';
import {
  TaxCalculatorService,
  TaxTableFile,
  TaxTableRange,
} from '../src/app/core/tax-calculator.service';

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

function workedForMinutes(
  date: string,
  minutes: number,
  lunchMinutes = 30,
  startTime = '08:00',
): WorkEntry {
  return worked(
    date,
    startTime,
    TimeInput.fromMinutes(TimeInput.toMinutes(startTime) + minutes + lunchMinutes),
    lunchMinutes,
  );
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
  {
    name: 'lunch-between-regular-and-overtime',
    entry: workedForMinutes('2026-07-01', 600, 60),
    salary,
    overtime: {
      ...overtime,
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
    },
  },
  ...[ObOvertimeCombinationMode.ExcludeOb, ObOvertimeCombinationMode.IncludeOb].map(
    (obOvertimeCombination) => ({
      name: `ob-overtime-${obOvertimeCombination}`,
      entry: worked('2026-07-01', '17:00', '20:00'),
      salary,
      overtime: {
        ...overtime,
        mode: OvertimeCompensationMode.Paid,
        dailyThresholdHours: 1,
        thresholdMode: OvertimeThresholdMode.FixedDailyHours,
        obOvertimeCombination,
        rateBands: [
          {
            name: 'Evening OB lower',
            dayCategory: OvertimeDayCategory.AllDays,
            startTime: '18:00',
            endTime: '22:00',
            compensationType: CompensationRuleType.Ob,
            rateType: CompensationRateType.FixedHourlyAmount,
            rateValue: 30,
          },
          {
            name: 'Evening OB highest',
            dayCategory: OvertimeDayCategory.AllDays,
            startTime: '18:00',
            endTime: '22:00',
            compensationType: CompensationRuleType.Ob,
            rateType: CompensationRateType.FixedHourlyAmount,
            rateValue: 50,
          },
        ],
      },
    }),
  ),
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
const currentAccrualEntries = [
  [3, 480],
  [4, 480],
  [5, 480],
  [6, 480],
  [7, 480],
  [10, 540],
  [11, 480],
  [12, 690],
  [13, 810],
].map(([day, minutes]) => workedForMinutes(`2026-08-${String(day).padStart(2, '0')}`, minutes));
write(
  'monthly-summary.json',
  [
    {
      name: 'empty-current',
      entries: [],
      today: '2026-08-14',
      record: monthRecord,
      expectedHours,
      salary,
      overtime,
    },
    {
      name: 'worked-off-incomplete-current',
      entries: monthlyEntries,
      today: '2026-08-14',
      record: monthRecord,
      expectedHours,
      salary,
      overtime,
    },
    {
      name: 'past-month-override',
      entries: monthlyEntries,
      today: '2026-09-01',
      record: { ...monthRecord, expectedMinutesOverride: 6000 },
      expectedHours,
      salary,
      overtime,
    },
    {
      name: 'future-month',
      entries: monthlyEntries,
      today: '2026-07-01',
      record: monthRecord,
      expectedHours,
      salary,
      overtime,
    },
    {
      name: 'current-month-accrual',
      entries: currentAccrualEntries,
      today: '2026-08-14',
      record: { ...monthRecord, openingBalanceMinutes: 0, expectedMinutesOverride: 154 * 60 },
      expectedHours: { ...expectedHours, excludePublicHolidays: false },
      salary: { ...salary, hourlyRate: 202 },
      overtime,
    },
    {
      name: 'monthly-expected-hours-pay',
      entries: [
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
      ].map(([day, minutes]) =>
        workedForMinutes(`2026-06-${String(day).padStart(2, '0')}`, minutes),
      ),
      today: '2026-07-01',
      record: {
        ...monthRecord,
        year: 2026,
        month: 6,
        openingBalanceMinutes: 0,
        expectedMinutesOverride: 136 * 60,
      },
      expectedHours,
      salary: { ...salary, hourlyRate: 180, hourlyPayBasis: HourlyPayBasis.MonthlyExpectedHours },
      overtime,
    },
    {
      name: 'paid-overtime-excluded-from-balance',
      entries: [workedForMinutes('2026-07-01', 10 * 60)],
      today: '2026-08-01',
      record: {
        ...monthRecord,
        month: 7,
        openingBalanceMinutes: 0,
        expectedMinutesOverride: 8 * 60,
      },
      expectedHours,
      salary,
      overtime: { ...overtime, mode: OvertimeCompensationMode.Paid },
    },
    {
      name: 'duplicate-date-last-wins',
      entries: [worked('2026-08-03', '08:00', '12:00'), worked('2026-08-03', '08:00', '18:00')],
      today: '2026-09-01',
      record: monthRecord,
      expectedHours,
      salary,
      overtime,
    },
  ].map((testCase) => ({
    input: testCase,
    output: MonthlyCalculations.calculateMonthlySummary(
      testCase.record,
      testCase.entries,
      testCase.expectedHours,
      testCase.salary,
      testCase.overtime,
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

write(
  'copy-paste-month.json',
  [
    ['2026-06-01', 2026, 7],
    ['2026-06-29', 2026, 2],
    ['2026-06-30', 2026, 9],
    ['2026-07-05', 2026, 8],
    ['2026-12-31', 2027, 1],
  ].map(([sourceDate, targetYear, targetMonth]) => ({
    input: { sourceDate, targetYear, targetMonth },
    output: matchingWeekdayOccurrence(
      sourceDate as string,
      targetYear as number,
      targetMonth as number,
    ),
  })),
);

const taxFile = JSON.parse(
  readFileSync(join(root, 'public', 'tax-data', 'tax-2026.json'), 'utf8'),
) as TaxTableFile;
const injector = createEnvironmentInjector([{ provide: HttpClient, useValue: {} }], null as never);
const taxCalculator = runInInjectionContext(injector, () => new TaxCalculatorService());
taxCalculator.registerTaxData(taxFile.TaxYear, taxFile);
const taxSettings: TaxSettings = {
  mode: TaxMode.PrimaryIncomeTaxTable,
  taxYear: taxFile.TaxYear,
  tableNumber: 30,
  column: 1,
  manualMonthlyDeduction: null,
};
const fixedRange = taxFile.Ranges.find(
  (range) => range.TableNumber === 30 && range.AmountKind === 'B' && range.LowerBound > 3000,
) as TaxTableRange;
const percentageRange = taxFile.Ranges.find(
  (range) => range.TableNumber === 30 && range.AmountKind === '%',
) as TaxTableRange;
const taxCases: Array<{ grossPay: number; settings: TaxSettings }> = [
  { grossPay: 10000, settings: { ...taxSettings, mode: TaxMode.Disabled } },
  { grossPay: -100, settings: { ...taxSettings, mode: TaxMode.Disabled } },
  { grossPay: 1000.99, settings: { ...taxSettings, mode: TaxMode.SecondaryIncomeThirtyPercent } },
  { grossPay: 1000, settings: { ...taxSettings, mode: TaxMode.ManualMonthlyDeduction } },
  {
    grossPay: 1000,
    settings: {
      ...taxSettings,
      mode: TaxMode.ManualMonthlyDeduction,
      manualMonthlyDeduction: -100,
    },
  },
  {
    grossPay: 1000,
    settings: {
      ...taxSettings,
      mode: TaxMode.ManualMonthlyDeduction,
      manualMonthlyDeduction: 1500,
    },
  },
  { grossPay: 0, settings: taxSettings },
  { grossPay: 10000, settings: { ...taxSettings, taxYear: 1900 } },
  { grossPay: 10000, settings: { ...taxSettings, tableNumber: 999 } },
];
for (const range of [fixedRange, percentageRange]) {
  for (const grossPay of [
    range.LowerBound - 1,
    range.LowerBound,
    range.UpperBound,
    range.UpperBound + 1,
    range.LowerBound + 0.75,
  ]) {
    for (let column = 1; column <= 6; column++) {
      taxCases.push({ grossPay, settings: { ...taxSettings, column } });
    }
  }
}
write(
  'tax.json',
  taxCases.map((input) => ({
    input,
    output: taxCalculator.calculate(input.grossPay, input.settings),
  })),
);
injector.destroy();

console.log(`Wrote parity fixtures to ${outputDirectory}`);
