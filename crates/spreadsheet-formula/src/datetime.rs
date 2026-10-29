//! Serial date and time arithmetic.
//!
//! A spreadsheet stores a date as a number: the integer part counts days from
//! the 1899-12-30 epoch and the fraction is the time of day. The epoch is one
//! day earlier than the calendar suggests because Excel keeps Lotus 1-2-3's
//! deliberate bug - it treats 1900 as a leap year, so serial 60 is the 29th of
//! February 1900, a day that never existed. Reproducing that is not optional: a
//! workbook written by Excel and one written here have to agree on what serial
//! 45306 means.
//!
//! The day arithmetic below is integer arithmetic, so it is exact. Only the
//! time of day goes through a float.

/// Days from 1899-12-31 to the Unix epoch, 1970-01-01.
const EPOCH_OFFSET: i64 = 25_568;

/// Seconds in a day.
pub(crate) const SECONDS_PER_DAY: f64 = 86_400.0;

/// Days since 1970-01-01 for a proleptic Gregorian date.
///
/// Howard Hinnant's `days_from_civil`, which is exact for every date the
/// spreadsheet serial range covers.
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The inverse of [`days_from_civil`].
fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// Days from 1899-12-31 to a real calendar date.
fn real_days(y: i64, m: i64, d: i64) -> i64 {
    days_from_civil(y, m, d) + EPOCH_OFFSET
}

/// The spreadsheet serial for a real calendar date.
///
/// Dates from 1900-03-01 onwards are shifted by one day to leave room for the
/// phantom 29th of February 1900.
pub(crate) fn serial_from_ymd(y: i64, m: i64, d: i64) -> i64 {
    let real = real_days(y, m, d);
    if real >= 60 { real + 1 } else { real }
}

/// The calendar date a serial names.
///
/// Serial 60 is the phantom day, and is reported as 1900-02-29 so that a
/// round trip through [`date_serial`] is stable.
pub(crate) fn ymd_from_serial(serial: i64) -> (i64, i64, i64) {
    if serial == 60 {
        return (1900, 2, 29);
    }
    let real = if serial > 60 { serial - 1 } else { serial };
    civil_from_days(real - EPOCH_OFFSET)
}

/// `DATE(year, month, day)`, with the month and day allowed to overflow the way
/// a spreadsheet rolls them.
pub(crate) fn date_serial(year: f64, month: f64, day: f64) -> i64 {
    let months = year.trunc() as i64 * 12 + (month.trunc() as i64 - 1);
    let y = months.div_euclid(12);
    let m = months.rem_euclid(12) + 1;
    serial_from_ymd(y, m, 1) + (day.trunc() as i64 - 1)
}

/// The whole-day part of a serial.
pub(crate) fn date_part(serial: f64) -> i64 {
    serial.floor() as i64
}

/// The time of day as a fraction, always in `[0, 1)`.
pub(crate) fn time_part(serial: f64) -> f64 {
    serial - serial.floor()
}

/// Whether a year is a leap year in the proleptic Gregorian calendar.
pub(crate) fn is_leap_year(y: i64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

/// The number of days in a month, treating February 1900 as having 29 to match
/// the serial system.
pub(crate) fn days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if y == 1900 => 29,
        2 if is_leap_year(y) => 29,
        _ => 28,
    }
}

/// `EDATE`: the date `months` months after `start`, clamped to the end of the
/// target month.
pub(crate) fn edate(start: i64, months: i64) -> i64 {
    let (y, m, d) = ymd_from_serial(start);
    let total = y * 12 + (m - 1) + months;
    let ty = total.div_euclid(12);
    let tm = total.rem_euclid(12) + 1;
    let td = d.min(days_in_month(ty, tm));
    serial_from_ymd(ty, tm, td)
}

/// `EOMONTH`: the last day of the month `months` months after `start`.
pub(crate) fn eomonth(start: i64, months: i64) -> i64 {
    let (y, m, _) = ymd_from_serial(start);
    let total = y * 12 + (m - 1) + months;
    let ty = total.div_euclid(12);
    let tm = total.rem_euclid(12) + 1;
    serial_from_ymd(ty, tm, days_in_month(ty, tm))
}

/// The day of the week for a serial, where 0 is Sunday.
///
/// Serial 1 (1900-01-01) is a Sunday, which fixes the phase.
pub(crate) fn weekday_index(serial: i64) -> i64 {
    (serial - 1).rem_euclid(7)
}

/// `WEEKDAY` under one of the numbering types a spreadsheet accepts.
pub(crate) fn weekday(serial: i64, kind: i64) -> Option<i64> {
    let idx = weekday_index(serial);
    match kind {
        1 => Some(idx + 1),
        2 | 11 => Some((idx + 6).rem_euclid(7) + 1),
        3 => Some((idx + 6).rem_euclid(7)),
        12..=17 => {
            let start = (kind - 10).rem_euclid(7);
            Some((idx - start).rem_euclid(7) + 1)
        }
        _ => None,
    }
}

/// The day of the year, one-based.
pub(crate) fn day_of_year(serial: i64) -> i64 {
    let (y, _, _) = ymd_from_serial(serial);
    serial - serial_from_ymd(y, 1, 1) + 1
}

/// `WEEKNUM`: the week number under a week-starting type.
pub(crate) fn weeknum(serial: i64, kind: i64) -> Option<i64> {
    if kind == 21 {
        return Some(iso_week(serial));
    }
    let start = match kind {
        1 => 0,
        2 | 11 => 1,
        12..=17 => (kind - 10).rem_euclid(7),
        _ => return None,
    };
    let (y, _, _) = ymd_from_serial(serial);
    let jan1 = serial_from_ymd(y, 1, 1);
    let offset = (weekday_index(jan1) - start).rem_euclid(7);
    Some((day_of_year(serial) + offset).div_euclid(7) + 1)
}

/// The ISO 8601 week number.
pub(crate) fn iso_week(serial: i64) -> i64 {
    // The ISO week a date belongs to is decided by the Thursday of its
    // Monday-first week, so shift to that Thursday and count from the year it
    // lands in.
    let idx = weekday_index(serial);
    let monday_first = (idx + 6).rem_euclid(7);
    let thursday = serial - monday_first + 3;
    let (ty, _, _) = ymd_from_serial(thursday);
    let jan1 = serial_from_ymd(ty, 1, 1);
    (thursday - jan1).div_euclid(7) + 1
}

/// The number of days in a month of the real calendar.
fn real_days_in_month(y: i64, m: i64) -> i64 {
    match m {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(y) => 29,
        _ => 28,
    }
}

/// `DAYS360` on the US (NASD) convention.
///
/// The rule is not "31 becomes 30": the last day of *any* month becomes 30, and
/// when the end date is the last day of its month and the start day is below
/// 30, the end rolls into the first of the next month.
pub(crate) fn days360_us(start: i64, end: i64) -> i64 {
    let (y0, m0, d0) = ymd_from_serial(start);
    let (y1, m1, d1) = ymd_from_serial(end);
    let sd = if d0 == real_days_in_month(y0, m0) {
        30
    } else {
        d0
    };
    let (mut em, ed) = if d1 == real_days_in_month(y1, m1) {
        if sd < 30 { (m1 + 1, 1) } else { (m1, 30) }
    } else {
        (m1, d1)
    };
    // A month that rolled over belongs to the next year.
    let mut ey = y1;
    if em > 12 {
        em -= 12;
        ey += 1;
    }
    360 * (ey - y0) + 30 * (em - m0) + (ed - sd)
}

/// `DAYS360` on the European convention, where only a literal 31 becomes 30 -
/// unlike the US rule, a short month's last day is left alone.
pub(crate) fn days360_european(start: i64, end: i64) -> i64 {
    let (y0, m0, d0) = ymd_from_serial(start);
    let (y1, m1, d1) = ymd_from_serial(end);
    let sd = if d0 == 31 { 30 } else { d0 };
    let ed = if d1 == 31 { 30 } else { d1 };
    360 * (y1 - y0) + 30 * (m1 - m0) + (ed - sd)
}

/// `YEARFRAC`: the fraction of a year between two dates under a day-count
/// basis.
pub(crate) fn yearfrac(start: i64, end: i64, basis: i64) -> f64 {
    let (lo, hi) = (start.min(end), start.max(end));
    let (sy, sm, sd) = ymd_from_serial(lo);
    let (ey, em, ed) = ymd_from_serial(hi);
    let actual = (hi - lo) as f64;
    // The 30/360 bases share one formula; they differ only in how a day 31 is
    // brought back to 30.
    let thirty_360 = |mut sd: i64, mut ed: i64| -> f64 {
        if basis == 0 {
            if sd == 31 && ed == 31 {
                sd = 30;
                ed = 30;
            } else if sd == 31 {
                sd = 30;
            } else if sd == 30 && ed == 31 {
                ed = 30;
            }
        }
        (ed + em * 30 + ey * 360 - (sd + sm * 30 + sy * 360)) as f64 / 360.0
    };
    match basis {
        0 | 4 => thirty_360(sd, ed),
        2 => actual / 360.0,
        3 => actual / 365.0,
        // Basis 1 measures the span against the average length of the calendar
        // years it covers, except when both dates fall within one year, where
        // the length of that year is used.
        _ => {
            let same_span = sy == ey || (sy + 1 == ey && (sm > em || (sm == em && sd >= ed)));
            if same_span {
                let length = if (sy == ey && is_leap_year(sy))
                    || (is_leap_year(sy)
                        && lo < serial_from_ymd(sy, 3, 1)
                        && hi >= serial_from_ymd(sy, 3, 1))
                    || (is_leap_year(ey)
                        && hi >= serial_from_ymd(ey, 3, 1)
                        && lo < serial_from_ymd(ey, 3, 1))
                    || (em == 1 && ed == 29)
                {
                    366.0
                } else {
                    365.0
                };
                return actual / length;
            }
            let years = (ey - sy + 1) as f64;
            let span = (serial_from_ymd(ey + 1, 1, 1) - serial_from_ymd(sy, 1, 1)) as f64;
            actual / (span / years)
        }
    }
}

/// `DATEDIF`: the difference between two dates in whole units.
pub(crate) fn datedif(start: i64, end: i64, unit: &str) -> Option<f64> {
    if end < start {
        return None;
    }
    let (y0, m0, d0) = ymd_from_serial(start);
    let (y1, m1, d1) = ymd_from_serial(end);
    let value = match unit {
        "D" => (end - start) as f64,
        "Y" => {
            let mut years = y1 - y0;
            if (m1, d1) < (m0, d0) {
                years -= 1;
            }
            years as f64
        }
        "M" => {
            let mut months = (y1 - y0) * 12 + (m1 - m0);
            if d1 < d0 {
                months -= 1;
            }
            months as f64
        }
        // Whole months after the years are taken out.
        "YM" => {
            let mut months = (m1 - m0).rem_euclid(12);
            if d1 < d0 {
                months = (months - 1).rem_euclid(12);
            }
            months as f64
        }
        // Whole days after the years are taken out, ignoring the month.
        "YD" => {
            let mut anchor = serial_from_ymd(y1, m0, d0.min(days_in_month(y1, m0)));
            if anchor > end {
                anchor = serial_from_ymd(y1 - 1, m0, d0.min(days_in_month(y1 - 1, m0)));
            }
            (end - anchor) as f64
        }
        // Whole days after the whole months are taken out.
        "MD" => {
            let mut anchor = serial_from_ymd(y1, m1, d0.min(days_in_month(y1, m1)));
            if anchor > end {
                let (py, pm) = if m1 == 1 { (y1 - 1, 12) } else { (y1, m1 - 1) };
                anchor = serial_from_ymd(py, pm, d0.min(days_in_month(py, pm)));
            }
            (end - anchor) as f64
        }
        _ => return None,
    };
    Some(value)
}

/// A parsed wall-clock time, in seconds since midnight.
pub(crate) fn parse_time(text: &str) -> Option<f64> {
    let text = text.trim();
    // A date-time is accepted, as Excel's `TIMEVALUE` accepts one: the date
    // half is dropped and only the clock time is kept. A bare date is midnight,
    // so it is zero. The separator is a space or an ISO 8601 `T`.
    let clock = match text.split_once([' ', 'T']) {
        Some((date, clock)) => {
            parse_date(date)?;
            clock.trim()
        }
        None if parse_date(text).is_some() => return Some(0.0),
        None => text,
    };
    if clock.is_empty() {
        return Some(0.0);
    }
    let mut parts = clock.split(':');
    let h: f64 = parts.next()?.trim().parse().ok()?;
    let m: f64 = parts.next()?.trim().parse().ok()?;
    let s: f64 = match parts.next() {
        Some(sec) => sec.trim().parse().ok()?,
        None => 0.0,
    };
    if parts.next().is_some() {
        return None;
    }
    Some(h * 3600.0 + m * 60.0 + s)
}

/// `DATEVALUE` for the `YYYY-MM-DD` form.
pub(crate) fn parse_date(text: &str) -> Option<i64> {
    let text = text.trim();
    let mut parts = text.split('-');
    let y: i64 = parts.next()?.trim().parse().ok()?;
    let m: i64 = parts.next()?.trim().parse().ok()?;
    let d: i64 = parts.next()?.trim().parse().ok()?;
    if parts.next().is_some() || !(1..=12).contains(&m) {
        return None;
    }
    if !(1..=days_in_month(y, m)).contains(&d) {
        return None;
    }
    Some(serial_from_ymd(y, m, d))
}

/// The weekday numbers a working-week calculation treats as the weekend.
fn is_weekend(serial: i64) -> bool {
    matches!(weekday_index(serial), 0 | 6)
}

/// `NETWORKDAYS`: whole working days between two dates, inclusive.
pub(crate) fn networkdays(start: i64, end: i64, holidays: &[i64]) -> i64 {
    let (lo, hi) = (start.min(end), start.max(end));
    let mut count = 0;
    let mut day = lo;
    while day <= hi {
        if !is_weekend(day) && !holidays.contains(&day) {
            count += 1;
        }
        day += 1;
    }
    count
}

/// Weekend days (0 is Sunday) named by a `weekend` code.
///
/// Codes 1 and 2 are Saturday/Sunday and Sunday/Monday; 3..7 shift the pair a
/// day at a time, 11..17 make both days the same, and 0 selects no weekend at
/// all. 8, 9 and 10 name nothing, so they are an error.
pub(crate) fn weekend_code(code: i64) -> Option<Vec<i64>> {
    Some(match code {
        0 => Vec::new(),
        1 => vec![6, 0],
        2 => vec![0, 1],
        3 => vec![1, 2],
        4 => vec![2, 3],
        5 => vec![3, 4],
        6 => vec![4, 5],
        7 => vec![5, 6],
        11..=17 => vec![code - 11],
        _ => return None,
    })
}

/// Weekend days named by a seven-character mask of `0` and `1`.
///
/// The mask reads Monday first, so its last character is Sunday.
pub(crate) fn weekend_mask(mask: &str) -> Option<Vec<i64>> {
    let bytes = mask.as_bytes();
    if bytes.len() != 7 || !bytes.iter().all(|b| *b == b'0' || *b == b'1') {
        return None;
    }
    const ORDER: [i64; 7] = [1, 2, 3, 4, 5, 6, 0];
    let mut days = Vec::new();
    for (i, b) in bytes.iter().enumerate() {
        if *b == b'1' {
            days.push(ORDER[i]);
        }
    }
    Some(days)
}

/// `NETWORKDAYS.INTL`: whole working days between two dates, for any weekend.
pub(crate) fn networkdays_intl(start: i64, end: i64, weekend: &[i64], holidays: &[i64]) -> i64 {
    let (lo, hi) = (start.min(end), start.max(end));
    let mut count = 0;
    let mut day = lo;
    while day <= hi {
        if !weekend.contains(&weekday_index(day)) && !holidays.contains(&day) {
            count += 1;
        }
        day += 1;
    }
    count
}

/// `WORKDAY.INTL`: the date `days` working days from `start`, for any weekend.
pub(crate) fn workday_intl(start: i64, days: i64, weekend: &[i64], holidays: &[i64]) -> i64 {
    let step = if days < 0 { -1 } else { 1 };
    let mut remaining = days.abs();
    let mut day = start;
    while remaining > 0 {
        day += step;
        if !weekend.contains(&weekday_index(day)) && !holidays.contains(&day) {
            remaining -= 1;
        }
    }
    day
}

/// `WORKDAY`: the date `days` working days after `start`.
pub(crate) fn workday(start: i64, days: i64, holidays: &[i64]) -> i64 {
    let step = if days < 0 { -1 } else { 1 };
    let mut remaining = days.abs();
    let mut day = start;
    while remaining > 0 {
        day += step;
        if !is_weekend(day) && !holidays.contains(&day) {
            remaining -= 1;
        }
    }
    day
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference library disagrees with Excel inside the 1900 phantom-leap
    /// window, so these expectations come from Excel itself and not from a
    /// differential run.
    #[test]
    fn the_1900_phantom_leap_day_round_trips() {
        // 1900 is not a leap year, but the serial system reserves day 60 for
        // the 29th of February so that every date from 1900-03-01 matches.
        assert_eq!(serial_from_ymd(1900, 1, 1), 1);
        assert_eq!(serial_from_ymd(1900, 2, 28), 59);
        assert_eq!(serial_from_ymd(1900, 3, 1), 61);
        assert_eq!(ymd_from_serial(59), (1900, 2, 28));
        assert_eq!(ymd_from_serial(60), (1900, 2, 29));
        assert_eq!(ymd_from_serial(61), (1900, 3, 1));
        // The known anchors of the serial system.
        assert_eq!(serial_from_ymd(2024, 1, 15), 45_306);
        assert_eq!(serial_from_ymd(2000, 1, 1), 36_526);
        assert_eq!(serial_from_ymd(1970, 1, 1), 25_569);
    }

    #[test]
    fn date_rolls_overflowing_components() {
        assert_eq!(date_serial(2024.0, 1.0, 15.0), 45_306);
        // Month 13 is January of the next year; day 0 is the last of the month
        // before.
        assert_eq!(date_serial(2024.0, 13.0, 1.0), serial_from_ymd(2025, 1, 1));
        assert_eq!(date_serial(2024.0, 1.0, 0.0), serial_from_ymd(2023, 12, 31));
        assert_eq!(date_serial(2024.0, 2.0, 30.0), serial_from_ymd(2024, 3, 1));
    }

    #[test]
    fn edate_and_eomonth_clamp_to_the_month_end() {
        let jan31 = serial_from_ymd(2024, 1, 31);
        assert_eq!(edate(jan31, 1), serial_from_ymd(2024, 2, 29));
        assert_eq!(
            edate(serial_from_ymd(2024, 1, 15), -2),
            serial_from_ymd(2023, 11, 15)
        );
        assert_eq!(eomonth(serial_from_ymd(2024, 1, 15), 0), jan31);
        assert_eq!(
            eomonth(serial_from_ymd(2024, 2, 10), 1),
            serial_from_ymd(2024, 3, 31)
        );
    }

    #[test]
    fn weekday_and_week_numbers_use_the_iso_rules() {
        // 2024-01-15 was a Monday.
        let monday = 45_306;
        assert_eq!(weekday(monday, 1), Some(2));
        assert_eq!(weekday(monday, 2), Some(1));
        assert_eq!(weekday(monday, 3), Some(0));
        assert_eq!(weekday(monday, 21), None);
        assert_eq!(weeknum(monday, 1), Some(3));
        assert_eq!(iso_week(monday), 3);
        // 2021-01-01 was a Friday and belongs to ISO week 53 of 2020.
        assert_eq!(iso_week(serial_from_ymd(2021, 1, 1)), 53);
    }

    #[test]
    fn days360_treats_the_month_end_as_thirty() {
        // Both dates on the last day of a 31-day month.
        assert_eq!(
            days360_us(serial_from_ymd(2024, 1, 31), serial_from_ymd(2024, 3, 31)),
            60
        );
        // February's short month counts as 30 as well.
        assert_eq!(
            days360_us(serial_from_ymd(2024, 1, 30), serial_from_ymd(2024, 2, 29)),
            30
        );
        assert_eq!(
            days360_european(serial_from_ymd(2024, 1, 30), serial_from_ymd(2024, 2, 29)),
            29
        );
    }

    #[test]
    fn yearfrac_covers_every_basis() {
        let (start, end) = (serial_from_ymd(2024, 1, 1), serial_from_ymd(2025, 7, 1));
        assert!((yearfrac(start, end, 0) - 1.5).abs() < 1e-12);
        assert!((yearfrac(start, end, 2) - 547.0 / 360.0).abs() < 1e-12);
        assert!((yearfrac(start, end, 3) - 547.0 / 365.0).abs() < 1e-12);
        assert!((yearfrac(start, end, 4) - 1.5).abs() < 1e-12);
    }

    #[test]
    fn datedif_units_match_the_reference() {
        let (start, end) = (serial_from_ymd(2020, 3, 15), serial_from_ymd(2024, 7, 20));
        assert_eq!(datedif(start, end, "Y"), Some(4.0));
        assert_eq!(datedif(start, end, "M"), Some(52.0));
        assert_eq!(datedif(start, end, "D"), Some(1588.0));
        assert_eq!(datedif(start, end, "MD"), Some(5.0));
        assert_eq!(datedif(start, end, "YM"), Some(4.0));
        assert_eq!(datedif(start, end, "YD"), Some(127.0));
        // A unit the function does not know, and an end before the start.
        assert_eq!(datedif(start, end, "Q"), None);
        assert_eq!(datedif(end, start, "D"), None);
    }

    #[test]
    fn workday_skips_weekends_and_holidays() {
        let start = serial_from_ymd(2024, 1, 1);
        assert_eq!(workday(start, 10, &[]), serial_from_ymd(2024, 1, 15));
        // A holiday inside the window pushes the answer out by one day.
        assert_eq!(
            workday(start, 10, &[serial_from_ymd(2024, 1, 8)]),
            serial_from_ymd(2024, 1, 16)
        );
        assert_eq!(workday(start, -5, &[]), serial_from_ymd(2023, 12, 25));
    }

    #[test]
    fn networkdays_counts_both_endpoints() {
        let start = serial_from_ymd(2024, 1, 1);
        let end = serial_from_ymd(2024, 1, 31);
        assert_eq!(networkdays(start, end, &[]), 23);
        assert_eq!(networkdays(start, end, &[serial_from_ymd(2024, 1, 8)]), 22);
        // The order of the two dates does not matter.
        assert_eq!(networkdays(end, start, &[]), 23);
    }

    #[test]
    fn time_of_day_is_the_fraction_of_a_day() {
        assert!((time_part(0.5) - 0.5).abs() < 1e-12);
        assert_eq!(parse_time("12:30"), Some(45_000.0));
        assert_eq!(parse_time("00:00"), Some(0.0));
        assert_eq!(parse_time("nonsense"), None);
        // A date-time keeps only its clock time; a bare date is midnight.
        assert_eq!(parse_time("2024-01-15 12:30"), Some(45_000.0));
        assert_eq!(parse_time("2024-01-15T06:00"), Some(21_600.0));
        assert_eq!(parse_time("2024-01-15"), Some(0.0));
        assert_eq!(parse_date("2024-01-15"), Some(45_306));
        assert_eq!(parse_date("2024-13-01"), None);
    }
}
