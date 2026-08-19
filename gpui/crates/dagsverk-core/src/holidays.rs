use std::collections::BTreeMap;

use chrono::{Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, Weekday};

use crate::models::{ClockTime, IsoDate};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SwedishHoliday {
    pub date: IsoDate,
    pub name: &'static str,
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SwedishHolidayCalendar;

impl SwedishHolidayCalendar {
    pub fn holidays(self, year: i32) -> Vec<SwedishHoliday> {
        self.named_holidays(year)
            .into_iter()
            .map(|(date, name)| SwedishHoliday { date, name })
            .collect()
    }

    pub fn is_public_holiday(self, date: IsoDate) -> bool {
        date.as_naive_date().weekday() == Weekday::Sun
            || self
                .named_holidays(date.as_naive_date().year())
                .contains_key(&date)
    }

    pub fn holiday_name(self, date: IsoDate) -> Option<&'static str> {
        self.named_holidays(date.as_naive_date().year())
            .get(&date)
            .copied()
    }

    pub fn is_major_holiday_period(self, date: IsoDate, time: ClockTime) -> bool {
        let target = date.as_naive_date().and_time(time.as_naive_time());
        let year = date.as_naive_date().year();
        [year - 1, year]
            .into_iter()
            .flat_map(|candidate| self.major_holiday_periods(candidate))
            .any(|(start, end)| target >= start && target < end)
    }

    fn named_holidays(self, year: i32) -> BTreeMap<IsoDate, &'static str> {
        let easter = easter_sunday(year);
        [
            (date(year, 1, 1), "New Year's Day"),
            (date(year, 1, 6), "Epiphany"),
            (easter - Duration::days(2), "Good Friday"),
            (easter, "Easter Sunday"),
            (easter + Duration::days(1), "Easter Monday"),
            (date(year, 5, 1), "May Day"),
            (easter + Duration::days(39), "Ascension Day"),
            (easter + Duration::days(49), "Whit Sunday"),
            (date(year, 6, 6), "National Day"),
            (saturday_on_or_after(year, 6, 20), "Midsummer Day"),
            (saturday_on_or_after(year, 10, 31), "All Saints' Day"),
            (date(year, 12, 25), "Christmas Day"),
            (date(year, 12, 26), "Boxing Day"),
        ]
        .into_iter()
        .map(|(date, name)| (IsoDate::new(date), name))
        .collect()
    }

    fn major_holiday_periods(self, year: i32) -> [(NaiveDateTime, NaiveDateTime); 5] {
        let easter = easter_sunday(year);
        let midsummer = saturday_on_or_after(year, 6, 20);
        [
            period(easter - Duration::days(3), easter + Duration::days(2)),
            period(easter + Duration::days(47), easter + Duration::days(50)),
            period(midsummer - Duration::days(2), midsummer + Duration::days(2)),
            period(date(year, 12, 23), self.next_weekday(date(year, 12, 24))),
            period(date(year, 12, 30), self.next_weekday(date(year, 12, 31))),
        ]
    }

    fn next_weekday(self, start: NaiveDate) -> NaiveDate {
        let mut candidate = start + Duration::days(1);
        while matches!(candidate.weekday(), Weekday::Sat | Weekday::Sun)
            || self.is_public_holiday(IsoDate::new(candidate))
        {
            candidate += Duration::days(1);
        }
        candidate
    }
}

fn date(year: i32, month: u32, day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(year, month, day).unwrap_or_else(|| unreachable!())
}

fn saturday_on_or_after(year: i32, month: u32, day: u32) -> NaiveDate {
    let candidate = date(year, month, day);
    let offset =
        (Weekday::Sat.num_days_from_monday() + 7 - candidate.weekday().num_days_from_monday()) % 7;
    candidate + Duration::days(i64::from(offset))
}

fn period(start: NaiveDate, end: NaiveDate) -> (NaiveDateTime, NaiveDateTime) {
    (
        start.and_time(NaiveTime::from_hms_opt(19, 0, 0).unwrap_or_else(|| unreachable!())),
        end.and_time(NaiveTime::from_hms_opt(7, 0, 0).unwrap_or_else(|| unreachable!())),
    )
}

/// Anonymous Gregorian computus.
fn easter_sunday(year: i32) -> NaiveDate {
    let golden_number = year % 19;
    let century = year / 100;
    let year_in_century = year % 100;
    let century_leap_days = century / 4;
    let century_remainder = century % 4;
    let lunar_correction = (century + 8) / 25;
    let lunar_shift = (century - lunar_correction + 1) / 3;
    let epact = (19 * golden_number + century - century_leap_days - lunar_shift + 15) % 30;
    let year_leap_days = year_in_century / 4;
    let year_remainder = year_in_century % 4;
    let weekday_offset =
        (32 + 2 * century_remainder + 2 * year_leap_days - epact - year_remainder) % 7;
    let correction = (golden_number + 11 * epact + 22 * weekday_offset) / 451;
    let march_offset = epact + weekday_offset - 7 * correction + 114;
    date(
        year,
        (march_offset / 31) as u32,
        (march_offset % 31 + 1) as u32,
    )
}
