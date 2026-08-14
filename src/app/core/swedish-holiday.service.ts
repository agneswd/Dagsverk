import { Injectable } from '@angular/core';

export interface SwedishHoliday {
  date: string; // YYYY-MM-DD
  name: string;
}

@Injectable({
  providedIn: 'root'
})
export class SwedishHolidayService {
  private namedHolidaysCache = new Map<number, Map<string, string>>();
  private allHolidaysCache = new Map<number, Set<string>>();

  public getHolidays(year: number): SwedishHoliday[] {
    const map = this.getNamedHolidaysMap(year);
    const result: SwedishHoliday[] = [];
    for (const [date, name] of map.entries()) {
      result.push({ date, name });
    }
    return result.sort((a, b) => a.date.localeCompare(b.date));
  }

  public isPublicHoliday(dateStr: string): boolean {
    const year = parseInt(dateStr.substring(0, 4), 10);
    this.ensureYearCached(year);
    return this.allHolidaysCache.get(year)!.has(dateStr);
  }

  /**
   * Returns holiday name ONLY for actual named statutory holidays.
   * Returns null for regular Sundays.
   */
  public getHolidayName(dateStr: string): string | null {
    const year = parseInt(dateStr.substring(0, 4), 10);
    const map = this.getNamedHolidaysMap(year);
    return map.get(dateStr) ?? null;
  }

  public isMajorHolidayPeriod(dateStr: string, timeStr: string): boolean {
    const [year, month, day] = dateStr.split('-').map(Number);
    const [hours, minutes] = timeStr.split(':').map(Number);
    const target = new Date(Date.UTC(year, month - 1, day, hours, minutes));

    const periods = [
      ...this.getMajorHolidayPeriods(year - 1),
      ...this.getMajorHolidayPeriods(year)
    ];

    return periods.some(period => target >= period.start && target < period.end);
  }

  private ensureYearCached(year: number): void {
    if (!this.namedHolidaysCache.has(year)) {
      this.getNamedHolidaysMap(year);
    }
  }

  private getNamedHolidaysMap(year: number): Map<string, string> {
    if (this.namedHolidaysCache.has(year)) {
      return this.namedHolidaysCache.get(year)!;
    }

    const namedMap = new Map<string, string>();
    const allSet = new Set<string>();
    const easter = this.calculateEasterSunday(year);

    // Statutory Named Holidays
    this.addNamedHoliday(namedMap, allSet, `${year}-01-01`, "New Year's Day");
    this.addNamedHoliday(namedMap, allSet, `${year}-01-06`, 'Epiphany');
    this.addNamedHoliday(namedMap, allSet, this.offsetDate(easter, -2), 'Good Friday');
    this.addNamedHoliday(namedMap, allSet, easter, 'Easter Sunday');
    this.addNamedHoliday(namedMap, allSet, this.offsetDate(easter, 1), 'Easter Monday');
    this.addNamedHoliday(namedMap, allSet, `${year}-05-01`, 'May Day');
    this.addNamedHoliday(namedMap, allSet, this.offsetDate(easter, 39), 'Ascension Day');
    this.addNamedHoliday(namedMap, allSet, this.offsetDate(easter, 49), 'Whit Sunday');
    this.addNamedHoliday(namedMap, allSet, `${year}-06-06`, 'National Day');
    this.addNamedHoliday(namedMap, allSet, this.saturdayOnOrAfter(year, 6, 20), 'Midsummer Day');
    this.addNamedHoliday(namedMap, allSet, this.saturdayOnOrAfter(year, 10, 31), "All Saints' Day");
    this.addNamedHoliday(namedMap, allSet, `${year}-12-25`, 'Christmas Day');
    this.addNamedHoliday(namedMap, allSet, `${year}-12-26`, 'Boxing Day');

    // Add all Sundays to allSet (for legal public holiday status/calculations)
    const isLeap = (year % 4 === 0 && year % 100 !== 0) || year % 400 === 0;
    const daysInYear = isLeap ? 366 : 365;
    const firstDay = new Date(Date.UTC(year, 0, 1));
    const firstSundayOffset = (7 - firstDay.getUTCDay()) % 7;

    for (let offset = firstSundayOffset; offset < daysInYear; offset += 7) {
      const d = new Date(Date.UTC(year, 0, 1 + offset));
      allSet.add(this.formatDate(d));
    }

    this.namedHolidaysCache.set(year, namedMap);
    this.allHolidaysCache.set(year, allSet);
    return namedMap;
  }

  private addNamedHoliday(map: Map<string, string>, set: Set<string>, dateStr: string, name: string): void {
    map.set(dateStr, name);
    set.add(dateStr);
  }

  private saturdayOnOrAfter(year: number, month: number, day: number): string {
    const date = new Date(Date.UTC(year, month - 1, day));
    const dayOfWeek = date.getUTCDay(); // 0=Sun, 6=Sat
    const offset = (6 - dayOfWeek + 7) % 7;
    date.setUTCDate(date.getUTCDate() + offset);
    return this.formatDate(date);
  }

  private offsetDate(dateStr: string, days: number): string {
    const [y, m, d] = dateStr.split('-').map(Number);
    const date = new Date(Date.UTC(y, m - 1, d + days));
    return this.formatDate(date);
  }

  private formatDate(date: Date): string {
    const y = date.getUTCFullYear();
    const m = String(date.getUTCMonth() + 1).padStart(2, '0');
    const d = String(date.getUTCDate()).padStart(2, '0');
    return `${y}-${m}-${d}`;
  }

  /** Anonymous Gregorian computus */
  private calculateEasterSunday(year: number): string {
    const goldenNumber = year % 19;
    const century = Math.floor(year / 100);
    const yearInCentury = year % 100;
    const centuryLeapDays = Math.floor(century / 4);
    const centuryRemainder = century % 4;
    const lunarCorrection = Math.floor((century + 8) / 25);
    const lunarShift = Math.floor((century - lunarCorrection + 1) / 3);
    const epact = (19 * goldenNumber + century - centuryLeapDays - lunarShift + 15) % 30;
    const yearLeapDays = Math.floor(yearInCentury / 4);
    const yearRemainder = yearInCentury % 4;
    const weekdayOffset = (32 + 2 * centuryRemainder + 2 * yearLeapDays - epact - yearRemainder) % 7;
    const correction = Math.floor((goldenNumber + 11 * epact + 22 * weekdayOffset) / 451);
    const marchOffset = epact + weekdayOffset - 7 * correction + 114;
    const month = Math.floor(marchOffset / 31);
    const day = (marchOffset % 31) + 1;

    return `${year}-${String(month).padStart(2, '0')}-${String(day).padStart(2, '0')}`;
  }

  private getMajorHolidayPeriods(year: number): Array<{ start: Date; end: Date }> {
    const easter = this.calculateEasterSunday(year);
    const midsummerDay = this.saturdayOnOrAfter(year, 6, 20);

    const makePeriod = (startDateStr: string, endDateStr: string) => {
      const [sy, sm, sd] = startDateStr.split('-').map(Number);
      const [ey, em, ed] = endDateStr.split('-').map(Number);
      return {
        start: new Date(Date.UTC(sy, sm - 1, sd, 19, 0)),
        end: new Date(Date.UTC(ey, em - 1, ed, 7, 0))
      };
    };

    return [
      makePeriod(this.offsetDate(easter, -3), this.offsetDate(easter, 2)),
      makePeriod(this.offsetDate(easter, 47), this.offsetDate(easter, 50)),
      makePeriod(this.offsetDate(midsummerDay, -2), this.offsetDate(midsummerDay, 2)),
      makePeriod(`${year}-12-23`, this.nextWeekday(year, 12, 24)),
      makePeriod(`${year}-12-30`, this.nextWeekday(year, 12, 31))
    ];
  }

  private nextWeekday(year: number, month: number, day: number): string {
    const date = new Date(Date.UTC(year, month - 1, day));
    do {
      date.setUTCDate(date.getUTCDate() + 1);
    } while (date.getUTCDay() === 0 || date.getUTCDay() === 6 || this.isPublicHoliday(this.formatDate(date)));

    return this.formatDate(date);
  }
}
