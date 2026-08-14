import { describe, it, expect, beforeEach } from 'vitest';
import { SwedishHolidayService } from './swedish-holiday.service';

describe('SwedishHolidayService', () => {
  let service: SwedishHolidayService;

  beforeEach(() => {
    service = new SwedishHolidayService();
  });

  it('should identify statutory holidays in 2026', () => {
    expect(service.isPublicHoliday('2026-01-01')).toBe(true); // New Year's Day
    expect(service.getHolidayName('2026-01-01')).toBe("New Year's Day");

    expect(service.isPublicHoliday('2026-01-06')).toBe(true); // Epiphany
    expect(service.getHolidayName('2026-01-06')).toBe('Epiphany');

    expect(service.isPublicHoliday('2026-04-03')).toBe(true); // Good Friday
    expect(service.getHolidayName('2026-04-03')).toBe('Good Friday');

    expect(service.isPublicHoliday('2026-04-05')).toBe(true); // Easter Sunday
    expect(service.getHolidayName('2026-04-05')).toBe('Easter Sunday');

    expect(service.isPublicHoliday('2026-04-06')).toBe(true); // Easter Monday
    expect(service.getHolidayName('2026-04-06')).toBe('Easter Monday');

    expect(service.isPublicHoliday('2026-05-01')).toBe(true); // May Day
    expect(service.isPublicHoliday('2026-05-14')).toBe(true); // Ascension Day
    expect(service.isPublicHoliday('2026-05-24')).toBe(true); // Whit Sunday
    expect(service.isPublicHoliday('2026-06-06')).toBe(true); // National Day
    expect(service.isPublicHoliday('2026-06-20')).toBe(true); // Midsummer Day
    expect(service.isPublicHoliday('2026-10-31')).toBe(true); // All Saints' Day
    expect(service.isPublicHoliday('2026-12-25')).toBe(true); // Christmas Day
    expect(service.isPublicHoliday('2026-12-26')).toBe(true); // Boxing Day
  });

  it('should identify all Sundays as statutory public holidays but return null for getHolidayName on regular Sundays', () => {
    expect(service.isPublicHoliday('2026-08-02')).toBe(true); // Sunday
    expect(service.getHolidayName('2026-08-02')).toBe(null); // Not a named holiday

    expect(service.isPublicHoliday('2026-08-09')).toBe(true); // Sunday
    expect(service.getHolidayName('2026-08-09')).toBe(null);
  });

  it('should recognize normal working weekdays as non-holidays', () => {
    expect(service.isPublicHoliday('2026-08-17')).toBe(false); // Monday
    expect(service.isPublicHoliday('2026-08-18')).toBe(false); // Tuesday
  });
});
