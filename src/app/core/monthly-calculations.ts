import {
  CompensationRateType,
  CompensationRuleType,
  DailyPayBreakdown,
  ExpectedHoursSettings,
  HourlyPayBasis,
  MonthlySummary,
  MonthRecord,
  ObOvertimeCombinationMode,
  OvertimeCompensationMode,
  OvertimeCompensationSettings,
  OvertimeDayCategory,
  OvertimeRateBand,
  OvertimeThresholdMode,
  SalarySettings,
  SalaryType,
  WorkEntry,
  WorkEntryStatus,
} from './models';
import { SwedishHolidayService } from './swedish-holiday.service';
import Decimal from 'decimal.js';

const roundMoney = (amount: Decimal.Value): number =>
  new Decimal(amount).toDecimalPlaces(2, Decimal.ROUND_HALF_UP).toNumber();

export class TimeInput {
  public static tryNormalize(input: string | null | undefined): string | null {
    if (!input || !input.trim()) {
      return null;
    }

    let candidate = input.trim().replace('.', ':');
    if (/^\d+$/.test(candidate)) {
      if (candidate.length === 1 || candidate.length === 2) {
        candidate = `${candidate}:00`;
      } else if (candidate.length === 3) {
        candidate = `${candidate.slice(0, 1)}:${candidate.slice(1)}`;
      } else if (candidate.length === 4) {
        candidate = `${candidate.slice(0, 2)}:${candidate.slice(2)}`;
      }
    }

    const match = candidate.match(/^(\d{1,2}):(\d{1,2})$/);
    if (!match) {
      return null;
    }

    const hours = parseInt(match[1], 10);
    const minutes = parseInt(match[2], 10);

    if (hours < 0 || hours > 23 || minutes < 0 || minutes > 59) {
      return null;
    }

    return `${String(hours).padStart(2, '0')}:${String(minutes).padStart(2, '0')}`;
  }

  public static toMinutes(timeStr: string): number {
    const [h, m] = timeStr.split(':').map(Number);
    return h * 60 + m;
  }

  public static fromMinutes(totalMinutes: number): string {
    const minutesInDay = 24 * 60;
    const normalized = ((totalMinutes % minutesInDay) + minutesInDay) % minutesInDay;
    const h = Math.floor(normalized / 60);
    const m = normalized % 60;
    return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
  }
}

export class MinuteMath {
  public static elapsed(startTime: string, endTime: string): number {
    const start = TimeInput.toMinutes(startTime);
    const end = TimeInput.toMinutes(endTime);
    const elapsed = end - start;
    return elapsed > 0 ? elapsed : elapsed + 24 * 60;
  }

  public static worked(
    startTime: string | null,
    endTime: string | null,
    lunchMinutes: number,
  ): number {
    if (!startTime || !endTime || startTime === endTime) {
      return 0;
    }
    return Math.max(0, this.elapsed(startTime, endTime) - (lunchMinutes || 0));
  }
}

export class OvertimeEngine {
  public static matchesRateBand(
    band: OvertimeRateBand,
    compensationType: CompensationRuleType,
    dateStr: string,
    timeStr: string,
    isScheduledWorkday: boolean,
    isPublicHoliday: boolean,
    isMajorHoliday: boolean,
  ): boolean {
    if (band.compensationType !== compensationType) {
      return false;
    }

    if (
      !this.matchesDayCategory(
        band.dayCategory,
        dateStr,
        isScheduledWorkday,
        isPublicHoliday,
        isMajorHoliday,
      )
    ) {
      return false;
    }

    return this.matchesTime(band.startTime, band.endTime, timeStr);
  }

  public static matchesDayCategory(
    category: OvertimeDayCategory,
    dateStr: string,
    isScheduledWorkday: boolean,
    isPublicHoliday: boolean,
    isMajorHoliday: boolean,
  ): boolean {
    const [y, m, d] = dateStr.split('-').map(Number);
    const dayOfWeek = new Date(Date.UTC(y, m - 1, d)).getUTCDay(); // 0=Sun, 1=Mon... 6=Sat

    switch (category) {
      case OvertimeDayCategory.ScheduledWorkdays:
        return isScheduledWorkday;
      case OvertimeDayCategory.NonWorkdays:
        return !isScheduledWorkday;
      case OvertimeDayCategory.PublicHolidays:
        return isPublicHoliday;
      case OvertimeDayCategory.ScheduledWeekdays:
        return isScheduledWorkday && dayOfWeek >= 1 && dayOfWeek <= 5;
      case OvertimeDayCategory.Weekends:
        return dayOfWeek === 0 || dayOfWeek === 6;
      case OvertimeDayCategory.MajorHolidays:
        return isMajorHoliday;
      case OvertimeDayCategory.Monday:
        return dayOfWeek === 1;
      case OvertimeDayCategory.Tuesday:
        return dayOfWeek === 2;
      case OvertimeDayCategory.Wednesday:
        return dayOfWeek === 3;
      case OvertimeDayCategory.Thursday:
        return dayOfWeek === 4;
      case OvertimeDayCategory.Friday:
        return dayOfWeek === 5;
      case OvertimeDayCategory.Saturday:
        return dayOfWeek === 6;
      case OvertimeDayCategory.Sunday:
        return dayOfWeek === 0;
      case OvertimeDayCategory.AllDays:
      default:
        return true;
    }
  }

  public static matchesTime(startTime: string, endTime: string, timeStr: string): boolean {
    const start = TimeInput.toMinutes(startTime);
    const end = TimeInput.toMinutes(endTime);
    const target = TimeInput.toMinutes(timeStr);

    if (start === end) {
      return true;
    }

    return start < end ? target >= start && target < end : target >= start || target < end;
  }

  public static getHourlyAmount(
    rateType: CompensationRateType,
    rateValue: number,
    salary: SalarySettings,
    includeHourlyBase: boolean,
  ): number {
    return this.getHourlyAmountDecimal(rateType, rateValue, salary, includeHourlyBase).toNumber();
  }

  private static getHourlyAmountDecimal(
    rateType: CompensationRateType,
    rateValue: number,
    salary: SalarySettings,
    includeHourlyBase: boolean,
  ): Decimal {
    switch (rateType) {
      case CompensationRateType.HourlyPremiumPercent:
        return new Decimal(salary.hourlyRate).times(
          new Decimal(rateValue).dividedBy(100).plus(includeHourlyBase ? 1 : 0),
        );
      case CompensationRateType.FixedHourlyAmount:
        return new Decimal(rateValue);
      case CompensationRateType.FullTimeMonthlySalaryDivisor:
        if (salary.type === SalaryType.Monthly && rateValue > 0) {
          return new Decimal(salary.monthlySalary)
            .times(100)
            .dividedBy(salary.employmentPercent)
            .dividedBy(rateValue);
        }
        return new Decimal(0);
      default:
        return new Decimal(0);
    }
  }

  public static hourlyAmountAt(
    compensationType: CompensationRuleType,
    salary: SalarySettings,
    overtimeCompensation: OvertimeCompensationSettings,
    dateStr: string,
    timeStr: string,
    isScheduledWorkday: boolean,
    isPublicHoliday: boolean,
    isMajorHoliday: boolean,
  ): number {
    return this.hourlyAmountAtDecimal(
      compensationType,
      salary,
      overtimeCompensation,
      dateStr,
      timeStr,
      isScheduledWorkday,
      isPublicHoliday,
      isMajorHoliday,
    ).toNumber();
  }

  public static hourlyAmountAtDecimal(
    compensationType: CompensationRuleType,
    salary: SalarySettings,
    overtimeCompensation: OvertimeCompensationSettings,
    dateStr: string,
    timeStr: string,
    isScheduledWorkday: boolean,
    isPublicHoliday: boolean,
    isMajorHoliday: boolean,
  ): Decimal {
    let highest = new Decimal(0);
    let matched = false;

    for (const band of overtimeCompensation.rateBands) {
      if (
        this.matchesRateBand(
          band,
          compensationType,
          dateStr,
          timeStr,
          isScheduledWorkday,
          isPublicHoliday,
          isMajorHoliday,
        )
      ) {
        const amount = this.getHourlyAmountDecimal(
          band.rateType,
          band.rateValue,
          salary,
          compensationType === CompensationRuleType.Overtime,
        );
        highest = matched ? Decimal.max(highest, amount) : amount;
        matched = true;
      }
    }

    if (matched || compensationType === CompensationRuleType.Ob) {
      return matched ? highest : new Decimal(0);
    }

    return this.getHourlyAmountDecimal(
      overtimeCompensation.defaultRateType,
      overtimeCompensation.defaultRateValue,
      salary,
      true,
    );
  }
}

export class MonthlyCalculations {
  public static getDatesInMonth(year: number, month: number): string[] {
    const daysCount = new Date(Date.UTC(year, month, 0)).getUTCDate();
    const dates: string[] = [];
    for (let day = 1; day <= daysCount; day++) {
      const dStr = String(day).padStart(2, '0');
      const mStr = String(month).padStart(2, '0');
      dates.push(`${year}-${mStr}-${dStr}`);
    }
    return dates;
  }

  public static isScheduledWorkday(
    dateStr: string,
    expectedHours: ExpectedHoursSettings,
    holidays: SwedishHolidayService,
  ): boolean {
    const [y, m, d] = dateStr.split('-').map(Number);
    const dayOfWeek = new Date(Date.UTC(y, m - 1, d)).getUTCDay();
    const isExpectedWeekday = expectedHours.workingWeekdays.includes(dayOfWeek);
    return (
      isExpectedWeekday &&
      (!expectedHours.excludePublicHolidays || !holidays.isPublicHoliday(dateStr))
    );
  }

  public static getExpectedWorkdays(
    year: number,
    month: number,
    expectedHours: ExpectedHoursSettings,
    holidays: SwedishHolidayService,
  ): string[] {
    return this.getDatesInMonth(year, month).filter((d) =>
      this.isScheduledWorkday(d, expectedHours, holidays),
    );
  }

  public static thresholdForEntry(
    entry: WorkEntry,
    expectedHours: ExpectedHoursSettings,
    overtimeCompensation: OvertimeCompensationSettings,
    holidays: SwedishHolidayService,
  ): number {
    if (entry.scheduledMinutesOverride !== null && entry.scheduledMinutesOverride !== undefined) {
      return entry.scheduledMinutesOverride;
    }

    if (overtimeCompensation.thresholdMode === OvertimeThresholdMode.ScheduledHours) {
      return this.isScheduledWorkday(entry.date, expectedHours, holidays)
        ? Math.round(expectedHours.hoursPerWorkday * 60)
        : 0;
    }

    return Math.round(overtimeCompensation.dailyThresholdHours * 60);
  }

  public static availableCompTimeMinutes(
    entry: WorkEntry,
    expectedHours: ExpectedHoursSettings,
    holidays: SwedishHolidayService,
  ): number {
    if (entry.status === WorkEntryStatus.Incomplete) return 0;
    const scheduled = this.scheduledMinutesForEntry(entry, expectedHours, holidays);
    const worked =
      entry.status === WorkEntryStatus.Worked && entry.startTime && entry.endTime
        ? MinuteMath.worked(entry.startTime, entry.endTime, entry.lunchMinutes)
        : 0;
    return Math.max(0, scheduled - Math.min(worked, scheduled));
  }

  public static scheduledMinutesForEntry(
    entry: WorkEntry,
    expectedHours: ExpectedHoursSettings,
    holidays: SwedishHolidayService,
  ): number {
    if (entry.scheduledMinutesOverride !== null && entry.scheduledMinutesOverride !== undefined) {
      return entry.scheduledMinutesOverride;
    }
    return this.isScheduledWorkday(entry.date, expectedHours, holidays)
      ? Math.round(expectedHours.hoursPerWorkday * 60)
      : 0;
  }

  public static splitOvertime(
    entry: WorkEntry,
    expectedHours: ExpectedHoursSettings,
    overtimeCompensation: OvertimeCompensationSettings,
    holidays: SwedishHolidayService,
  ): { regularMinutes: number; overtimeMinutes: number } {
    const workedMinutes = MinuteMath.worked(entry.startTime, entry.endTime, entry.lunchMinutes);
    const threshold = this.thresholdForEntry(entry, expectedHours, overtimeCompensation, holidays);
    const regular = Math.min(workedMinutes, threshold);
    return {
      regularMinutes: regular,
      overtimeMinutes: Math.max(0, workedMinutes - regular),
    };
  }

  public static calculateDailyPay(
    entry: WorkEntry,
    expectedHours: ExpectedHoursSettings,
    salary: SalarySettings,
    overtimeCompensation: OvertimeCompensationSettings,
    holidays: SwedishHolidayService,
  ): DailyPayBreakdown {
    if (entry.status !== WorkEntryStatus.Worked || !entry.startTime || !entry.endTime) {
      return { regularPay: 0, overtimePay: 0, obPay: 0, obMinutes: 0, total: 0 };
    }

    const { regularMinutes, overtimeMinutes } = this.splitOvertime(
      entry,
      expectedHours,
      overtimeCompensation,
      holidays,
    );
    const regularPay =
      salary.type === SalaryType.Hourly
        ? new Decimal(regularMinutes).times(salary.hourlyRate).dividedBy(60)
        : new Decimal(0);
    const paysOvertime =
      overtimeCompensation.mode === OvertimeCompensationMode.Paid && overtimeMinutes > 0;
    const paysOb = overtimeCompensation.rateBands.some(
      (band) => band.compensationType === CompensationRuleType.Ob,
    );
    if (!paysOvertime && !paysOb) {
      const roundedRegularPay = roundMoney(regularPay);
      return {
        regularPay: roundedRegularPay,
        overtimePay: 0,
        obPay: 0,
        obMinutes: 0,
        total: roundedRegularPay,
      };
    }

    let overtimePay = new Decimal(0);
    let obPay = new Decimal(0);
    let obMinutes = 0;

    for (let minute = 0; minute < regularMinutes + overtimeMinutes; minute++) {
      const isOvertimeMinute = minute >= regularMinutes;
      const clock = this.clockAt(entry, isOvertimeMinute ? minute + entry.lunchMinutes : minute);
      const isScheduled = this.isScheduledWorkday(clock.date, expectedHours, holidays);
      const isPublicHoliday = holidays.isPublicHoliday(clock.date);
      const isMajorHoliday = holidays.isMajorHolidayPeriod(clock.date, clock.time);
      const paysObForMinute =
        paysOb &&
        (!isOvertimeMinute ||
          (overtimeCompensation.obOvertimeCombination ?? ObOvertimeCombinationMode.ExcludeOb) ===
            ObOvertimeCombinationMode.IncludeOb);

      if (paysObForMinute) {
        const hourlyOb = OvertimeEngine.hourlyAmountAtDecimal(
          CompensationRuleType.Ob,
          salary,
          overtimeCompensation,
          clock.date,
          clock.time,
          isScheduled,
          isPublicHoliday,
          isMajorHoliday,
        );
        if (hourlyOb.greaterThan(0)) {
          obPay = obPay.plus(hourlyOb.dividedBy(60));
          obMinutes++;
        }
      }

      if (isOvertimeMinute && paysOvertime) {
        overtimePay = overtimePay.plus(
          OvertimeEngine.hourlyAmountAtDecimal(
            CompensationRuleType.Overtime,
            salary,
            overtimeCompensation,
            clock.date,
            clock.time,
            isScheduled,
            isPublicHoliday,
            isMajorHoliday,
          ).dividedBy(60),
        );
      }
    }

    const rPay = roundMoney(regularPay);
    const oPay = roundMoney(overtimePay);
    const bPay = roundMoney(obPay);

    return {
      regularPay: rPay,
      overtimePay: oPay,
      obPay: bPay,
      obMinutes,
      total: new Decimal(rPay).plus(oPay).plus(bPay).toNumber(),
    };
  }

  public static calculateMonthlySummary(
    monthRecord: MonthRecord,
    entries: WorkEntry[],
    expectedHours: ExpectedHoursSettings,
    salary: SalarySettings,
    overtimeCompensation: OvertimeCompensationSettings,
    holidays: SwedishHolidayService,
    todayStr: string,
  ): MonthlySummary {
    const entriesByDate = new Map<string, WorkEntry>();
    for (const e of entries) {
      if (e.date.startsWith(`${monthRecord.year}-${String(monthRecord.month).padStart(2, '0')}`)) {
        entriesByDate.set(e.date, e);
      }
    }

    const monthEntries = [...entriesByDate.values()];
    const expectedMinutes = this.calculateExpectedMinutes(
      monthRecord,
      monthEntries,
      expectedHours,
      holidays,
      todayStr,
    );

    let totalWorkedMinutes = 0;
    let totalRegularMinutes = 0;
    let totalOvertimeMinutes = 0;
    let totalCompTimeUsedMinutes = 0;
    let totalOvertimePay = 0;
    let totalObPay = 0;
    let totalObMinutes = 0;
    let totalGrossSalary = new Decimal(
      salary.type === SalaryType.Monthly ? salary.monthlySalary : 0,
    );
    let completedDayCount = 0;

    for (const entry of entriesByDate.values()) {
      const requestedCompTime = Number.isInteger(entry.compTimeMinutes)
        ? Math.max(0, entry.compTimeMinutes)
        : 0;
      totalCompTimeUsedMinutes += Math.min(
        requestedCompTime,
        this.availableCompTimeMinutes(entry, expectedHours, holidays),
      );
      if (entry.status === WorkEntryStatus.Worked && entry.startTime && entry.endTime) {
        completedDayCount++;
        const worked = MinuteMath.worked(entry.startTime, entry.endTime, entry.lunchMinutes);
        const { regularMinutes, overtimeMinutes } = this.splitOvertime(
          entry,
          expectedHours,
          overtimeCompensation,
          holidays,
        );
        const pay = this.calculateDailyPay(
          entry,
          expectedHours,
          salary,
          overtimeCompensation,
          holidays,
        );

        totalWorkedMinutes += worked;
        totalRegularMinutes += regularMinutes;
        totalOvertimeMinutes += overtimeMinutes;
        totalGrossSalary = totalGrossSalary.plus(pay.total);
        totalOvertimePay += pay.overtimePay;
        totalObPay += pay.obPay;
        totalObMinutes += pay.obMinutes;
      } else if (entry.status === WorkEntryStatus.Off) {
        completedDayCount++;
      }
    }

    const balanceEligibleMinutes =
      overtimeCompensation.mode === OvertimeCompensationMode.CompTime
        ? totalWorkedMinutes
        : totalRegularMinutes;

    let ordinaryPaidMinutes = salary.type === SalaryType.Hourly ? totalRegularMinutes : 0;
    let compTimeEarnedMinutes =
      overtimeCompensation.mode === OvertimeCompensationMode.CompTime ? totalOvertimeMinutes : 0;
    if (
      salary.type === SalaryType.Hourly &&
      overtimeCompensation.mode === OvertimeCompensationMode.CompTime &&
      (salary.hourlyPayBasis ?? HourlyPayBasis.DailyRegularHours) ===
        HourlyPayBasis.MonthlyExpectedHours
    ) {
      ordinaryPaidMinutes = Math.min(
        totalWorkedMinutes + totalCompTimeUsedMinutes,
        expectedMinutes,
      );
      compTimeEarnedMinutes = Math.max(
        0,
        totalWorkedMinutes + totalCompTimeUsedMinutes - expectedMinutes,
      );
      totalGrossSalary = new Decimal(ordinaryPaidMinutes)
        .times(salary.hourlyRate)
        .dividedBy(60)
        .plus(totalObPay);
    }

    const monthlyDifferenceMinutes = balanceEligibleMinutes - expectedMinutes;
    const closingBalanceMinutes = monthRecord.openingBalanceMinutes + monthlyDifferenceMinutes;

    const expectedWorkdays = this.getExpectedWorkdays(
      monthRecord.year,
      monthRecord.month,
      expectedHours,
      holidays,
    );
    const missingPastDays = expectedWorkdays.filter((dateStr) => {
      if (dateStr >= todayStr) {
        return false;
      }
      const entry = entriesByDate.get(dateStr);
      return !entry || entry.status === WorkEntryStatus.Incomplete;
    });

    return {
      year: monthRecord.year,
      month: monthRecord.month,
      workedMinutes: totalWorkedMinutes,
      regularMinutes: totalRegularMinutes,
      overtimeMinutes: totalOvertimeMinutes,
      ordinaryPaidMinutes,
      compTimeEarnedMinutes,
      compTimeUsedMinutes: totalCompTimeUsedMinutes,
      balanceEligibleMinutes,
      expectedMinutes,
      monthlyDifferenceMinutes,
      openingBalanceMinutes: monthRecord.openingBalanceMinutes,
      closingBalanceMinutes,
      grossSalary: roundMoney(totalGrossSalary),
      baseSalary: salary.type === SalaryType.Monthly ? salary.monthlySalary : 0,
      overtimeCompensation: Math.round(totalOvertimePay * 100) / 100,
      obCompensation: Math.round(totalObPay * 100) / 100,
      obMinutes: totalObMinutes,
      completedDayCount,
      missingPastDays,
      workedHours: Math.round((totalWorkedMinutes / 60) * 100) / 100,
      regularHours: Math.round((totalRegularMinutes / 60) * 100) / 100,
      overtimeHours: Math.round((totalOvertimeMinutes / 60) * 100) / 100,
      ordinaryPaidHours: ordinaryPaidMinutes / 60,
      compTimeEarnedHours: compTimeEarnedMinutes / 60,
      compTimeUsedHours: totalCompTimeUsedMinutes / 60,
      obHours: Math.round((totalObMinutes / 60) * 100) / 100,
      expectedHours: Math.round((expectedMinutes / 60) * 100) / 100,
    };
  }

  private static calculateExpectedMinutes(
    monthRecord: MonthRecord,
    entries: WorkEntry[],
    expectedHours: ExpectedHoursSettings,
    holidays: SwedishHolidayService,
    through?: string,
  ): number {
    const workdays = this.getExpectedWorkdays(
      monthRecord.year,
      monthRecord.month,
      expectedHours,
      holidays,
    );
    const dailyMinutes = Math.round(expectedHours.hoursPerWorkday * 60);
    const fullExpected = monthRecord.expectedMinutesOverride ?? workdays.length * dailyMinutes;
    const monthStart = `${monthRecord.year}-${String(monthRecord.month).padStart(2, '0')}-01`;
    const monthEnd = this.getDatesInMonth(monthRecord.year, monthRecord.month).at(-1)!;
    const fullMonth = !through || through >= monthEnd;
    if (!fullMonth && through < monthStart) {
      return 0;
    }

    const elapsedWorkdays = fullMonth
      ? workdays.length
      : workdays.filter((date) => date <= through).length;
    if (monthRecord.expectedMinutesOverride !== null) {
      if (fullMonth) {
        return fullExpected;
      }
      if (workdays.length === 0) {
        return 0;
      }
      return new Decimal(fullExpected)
        .times(elapsedWorkdays)
        .dividedBy(workdays.length)
        .toDecimalPlaces(0, Decimal.ROUND_HALF_UP)
        .toNumber();
    }

    const lastIncluded = fullMonth ? monthEnd : through!;
    let adjusted = fullMonth ? fullExpected : elapsedWorkdays * dailyMinutes;
    for (const entry of entries) {
      if (entry.date > lastIncluded || entry.scheduledMinutesOverride === null) {
        continue;
      }
      adjusted -= this.isScheduledWorkday(entry.date, expectedHours, holidays) ? dailyMinutes : 0;
      adjusted += entry.scheduledMinutesOverride;
    }
    return Math.max(0, adjusted);
  }

  private static clockAt(entry: WorkEntry, offsetMinutes: number): { date: string; time: string } {
    const absoluteMinutes = TimeInput.toMinutes(entry.startTime!) + offsetMinutes;
    const dayOffset = Math.floor(absoluteMinutes / (24 * 60));
    return {
      date: this.addDays(entry.date, dayOffset),
      time: TimeInput.fromMinutes(absoluteMinutes),
    };
  }

  private static addDays(dateStr: string, days: number): string {
    const [year, month, day] = dateStr.split('-').map(Number);
    const date = new Date(Date.UTC(year, month - 1, day + days));
    return date.toISOString().slice(0, 10);
  }
}
