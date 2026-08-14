import {
  CompensationRateType,
  CompensationRuleType,
  DailyPayBreakdown,
  ExpectedHoursSettings,
  MonthlySummary,
  MonthRecord,
  OvertimeCompensationMode,
  OvertimeCompensationSettings,
  OvertimeDayCategory,
  OvertimeRateBand,
  OvertimeThresholdMode,
  SalarySettings,
  SalaryType,
  WorkEntry,
  WorkEntryStatus
} from './models';
import { SwedishHolidayService } from './swedish-holiday.service';

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
    const h = Math.floor(totalMinutes / 60) % 24;
    const m = totalMinutes % 60;
    return `${String(h).padStart(2, '0')}:${String(m).padStart(2, '0')}`;
  }
}

export class MinuteMath {
  public static worked(startTime: string | null, endTime: string | null, lunchMinutes: number): number {
    if (!startTime || !endTime) {
      return 0;
    }
    const start = TimeInput.toMinutes(startTime);
    const end = TimeInput.toMinutes(endTime);
    if (end <= start) {
      return 0;
    }
    const elapsed = end - start;
    return Math.max(0, elapsed - (lunchMinutes || 0));
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
    isMajorHoliday: boolean
  ): boolean {
    if (band.compensationType !== compensationType) {
      return false;
    }

    if (!this.matchesDayCategory(band.dayCategory, dateStr, isScheduledWorkday, isPublicHoliday, isMajorHoliday)) {
      return false;
    }

    return this.matchesTime(band.startTime, band.endTime, timeStr);
  }

  public static matchesDayCategory(
    category: OvertimeDayCategory,
    dateStr: string,
    isScheduledWorkday: boolean,
    isPublicHoliday: boolean,
    isMajorHoliday: boolean
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

    return start < end
      ? target >= start && target < end
      : target >= start || target < end;
  }

  public static getHourlyAmount(
    rateType: CompensationRateType,
    rateValue: number,
    salary: SalarySettings,
    includeHourlyBase: boolean
  ): number {
    switch (rateType) {
      case CompensationRateType.HourlyPremiumPercent:
        return salary.hourlyRate * ((includeHourlyBase ? 1 : 0) + rateValue / 100);
      case CompensationRateType.FixedHourlyAmount:
        return rateValue;
      case CompensationRateType.FullTimeMonthlySalaryDivisor:
        if (salary.type === SalaryType.Monthly && rateValue > 0) {
          const fullTimeSalary = salary.monthlySalary * 100 / (salary.employmentPercent || 100);
          return fullTimeSalary / rateValue;
        }
        return 0;
      default:
        return 0;
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
    isMajorHoliday: boolean
  ): number {
    let highest = 0;
    let matched = false;

    for (const band of overtimeCompensation.rateBands) {
      if (this.matchesRateBand(band, compensationType, dateStr, timeStr, isScheduledWorkday, isPublicHoliday, isMajorHoliday)) {
        const amount = this.getHourlyAmount(
          band.rateType,
          band.rateValue,
          salary,
          compensationType === CompensationRuleType.Overtime
        );
        highest = matched ? Math.max(highest, amount) : amount;
        matched = true;
      }
    }

    if (matched || compensationType === CompensationRuleType.Ob) {
      return matched ? highest : 0;
    }

    return this.getHourlyAmount(
      overtimeCompensation.defaultRateType,
      overtimeCompensation.defaultRateValue,
      salary,
      true
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
    holidays: SwedishHolidayService
  ): boolean {
    const [y, m, d] = dateStr.split('-').map(Number);
    const dayOfWeek = new Date(Date.UTC(y, m - 1, d)).getUTCDay();
    const isExpectedWeekday = expectedHours.workingWeekdays.includes(dayOfWeek);
    return isExpectedWeekday && (!expectedHours.excludePublicHolidays || !holidays.isPublicHoliday(dateStr));
  }

  public static getExpectedWorkdays(
    year: number,
    month: number,
    expectedHours: ExpectedHoursSettings,
    holidays: SwedishHolidayService
  ): string[] {
    return this.getDatesInMonth(year, month).filter(d => this.isScheduledWorkday(d, expectedHours, holidays));
  }

  public static thresholdForEntry(
    entry: WorkEntry,
    expectedHours: ExpectedHoursSettings,
    overtimeCompensation: OvertimeCompensationSettings,
    holidays: SwedishHolidayService
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

  public static splitOvertime(
    entry: WorkEntry,
    expectedHours: ExpectedHoursSettings,
    overtimeCompensation: OvertimeCompensationSettings,
    holidays: SwedishHolidayService
  ): { regularMinutes: number; overtimeMinutes: number } {
    const workedMinutes = MinuteMath.worked(entry.startTime, entry.endTime, entry.lunchMinutes);
    const threshold = this.thresholdForEntry(entry, expectedHours, overtimeCompensation, holidays);
    const regular = Math.min(workedMinutes, threshold);
    return {
      regularMinutes: regular,
      overtimeMinutes: Math.max(0, workedMinutes - regular)
    };
  }

  public static calculateDailyPay(
    entry: WorkEntry,
    expectedHours: ExpectedHoursSettings,
    salary: SalarySettings,
    overtimeCompensation: OvertimeCompensationSettings,
    holidays: SwedishHolidayService
  ): DailyPayBreakdown {
    if (entry.status !== WorkEntryStatus.Worked || !entry.startTime || !entry.endTime) {
      return { regularPay: 0, overtimePay: 0, obPay: 0, obMinutes: 0, total: 0 };
    }

    const { regularMinutes, overtimeMinutes } = this.splitOvertime(entry, expectedHours, overtimeCompensation, holidays);
    const regularPay = salary.type === SalaryType.Hourly
      ? (regularMinutes * salary.hourlyRate) / 60
      : 0;

    let overtimePay = 0;
    let obPay = 0;
    let obMinutes = 0;

    const isPublicHoliday = holidays.isPublicHoliday(entry.date);
    const isScheduled = this.isScheduledWorkday(entry.date, expectedHours, holidays);

    const startMinutes = TimeInput.toMinutes(entry.startTime);
    for (let minute = 0; minute < regularMinutes; minute++) {
      const timeStr = TimeInput.fromMinutes(startMinutes + minute);
      const isMajorHoliday = holidays.isMajorHolidayPeriod(entry.date, timeStr);
      const hourlyOb = OvertimeEngine.hourlyAmountAt(
        CompensationRuleType.Ob,
        salary,
        overtimeCompensation,
        entry.date,
        timeStr,
        isScheduled,
        isPublicHoliday,
        isMajorHoliday
      );
      if (hourlyOb > 0) {
        obPay += hourlyOb / 60;
        obMinutes++;
      }
    }

    if (overtimeCompensation.mode === OvertimeCompensationMode.Paid && overtimeMinutes > 0) {
      const endMinutes = TimeInput.toMinutes(entry.endTime);
      const overtimeStartMinutes = endMinutes - overtimeMinutes;
      for (let minute = 0; minute < overtimeMinutes; minute++) {
        const timeStr = TimeInput.fromMinutes(overtimeStartMinutes + minute);
        const isMajorHoliday = holidays.isMajorHolidayPeriod(entry.date, timeStr);
        const hourlyOvertime = OvertimeEngine.hourlyAmountAt(
          CompensationRuleType.Overtime,
          salary,
          overtimeCompensation,
          entry.date,
          timeStr,
          isScheduled,
          isPublicHoliday,
          isMajorHoliday
        );
        overtimePay += hourlyOvertime / 60;
      }
    }

    const rPay = Math.round(regularPay * 100) / 100;
    const oPay = Math.round(overtimePay * 100) / 100;
    const bPay = Math.round(obPay * 100) / 100;

    return {
      regularPay: rPay,
      overtimePay: oPay,
      obPay: bPay,
      obMinutes,
      total: Math.round((rPay + oPay + bPay) * 100) / 100
    };
  }

  public static calculateMonthlySummary(
    monthRecord: MonthRecord,
    entries: WorkEntry[],
    expectedHours: ExpectedHoursSettings,
    salary: SalarySettings,
    overtimeCompensation: OvertimeCompensationSettings,
    holidays: SwedishHolidayService,
    todayStr: string
  ): MonthlySummary {
    const datesInMonth = this.getDatesInMonth(monthRecord.year, monthRecord.month);
    const entriesByDate = new Map<string, WorkEntry>();
    for (const e of entries) {
      if (e.date.startsWith(`${monthRecord.year}-${String(monthRecord.month).padStart(2, '0')}`)) {
        entriesByDate.set(e.date, e);
      }
    }

    let expectedMinutes = 0;
    if (monthRecord.expectedMinutesOverride !== null && monthRecord.expectedMinutesOverride !== undefined) {
      expectedMinutes = monthRecord.expectedMinutesOverride;
    } else {
      const expectedDays = this.getExpectedWorkdays(monthRecord.year, monthRecord.month, expectedHours, holidays);
      expectedMinutes = expectedDays.length * Math.round(expectedHours.hoursPerWorkday * 60);

      // Adjust for scheduledMinutesOverride on individual entries
      for (const entry of entriesByDate.values()) {
        if (entry.scheduledMinutesOverride !== null && entry.scheduledMinutesOverride !== undefined) {
          const defaultDaily = this.isScheduledWorkday(entry.date, expectedHours, holidays)
            ? Math.round(expectedHours.hoursPerWorkday * 60)
            : 0;
          expectedMinutes = expectedMinutes - defaultDaily + entry.scheduledMinutesOverride;
        }
      }
      expectedMinutes = Math.max(0, expectedMinutes);
    }

    let totalWorkedMinutes = 0;
    let totalRegularMinutes = 0;
    let totalOvertimeMinutes = 0;
    let totalOvertimePay = 0;
    let totalObPay = 0;
    let totalObMinutes = 0;
    let totalGrossSalary = salary.type === SalaryType.Monthly ? salary.monthlySalary : 0;
    let completedDayCount = 0;

    for (const entry of entriesByDate.values()) {
      if (entry.status === WorkEntryStatus.Worked && entry.startTime && entry.endTime) {
        completedDayCount++;
        const worked = MinuteMath.worked(entry.startTime, entry.endTime, entry.lunchMinutes);
        const { regularMinutes, overtimeMinutes } = this.splitOvertime(entry, expectedHours, overtimeCompensation, holidays);
        const pay = this.calculateDailyPay(entry, expectedHours, salary, overtimeCompensation, holidays);

        totalWorkedMinutes += worked;
        totalRegularMinutes += regularMinutes;
        totalOvertimeMinutes += overtimeMinutes;
        totalGrossSalary += pay.total;
        totalOvertimePay += pay.overtimePay;
        totalObPay += pay.obPay;
        totalObMinutes += pay.obMinutes;
      } else if (entry.status === WorkEntryStatus.Off) {
        completedDayCount++;
      }
    }

    const balanceEligibleMinutes = overtimeCompensation.mode === OvertimeCompensationMode.CompTime
      ? totalWorkedMinutes
      : totalRegularMinutes;

    const monthlyDifferenceMinutes = balanceEligibleMinutes - expectedMinutes;
    const closingBalanceMinutes = monthRecord.openingBalanceMinutes + monthlyDifferenceMinutes;

    const expectedWorkdays = this.getExpectedWorkdays(monthRecord.year, monthRecord.month, expectedHours, holidays);
    const missingPastDays = expectedWorkdays.filter(dateStr => {
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
      balanceEligibleMinutes,
      expectedMinutes,
      monthlyDifferenceMinutes,
      openingBalanceMinutes: monthRecord.openingBalanceMinutes,
      closingBalanceMinutes,
      grossSalary: Math.round(totalGrossSalary * 100) / 100,
      baseSalary: salary.type === SalaryType.Monthly ? salary.monthlySalary : 0,
      overtimeCompensation: Math.round(totalOvertimePay * 100) / 100,
      obCompensation: Math.round(totalObPay * 100) / 100,
      obMinutes: totalObMinutes,
      completedDayCount,
      missingPastDays,
      workedHours: Math.round((totalWorkedMinutes / 60) * 100) / 100,
      regularHours: Math.round((totalRegularMinutes / 60) * 100) / 100,
      overtimeHours: Math.round((totalOvertimeMinutes / 60) * 100) / 100,
      obHours: Math.round((totalObMinutes / 60) * 100) / 100,
      expectedHours: Math.round((expectedMinutes / 60) * 100) / 100
    };
  }
}
