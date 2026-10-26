//! The timestamp comment Java writes at the top of a stored document.
//!
//! `java.util.Properties.store` writes `#` followed by `java.util.Date.toString`,
//! which looks like `Mon Sep 02 14:00:54 EDT 2016`. The format is the C locale's
//! `ctime` without the trailing newline and with the zone abbreviation in the
//! middle.
//!
//! Java renders the timestamp in the **local** timezone, which it reads from the
//! host. This crate has no timezone database and takes no dependency, so it
//! renders in UTC and names it `UTC`. That is a deliberate difference, and it is
//! the reason the function is named `java_timestamp_utc` rather than
//! `java_timestamp`: a caller who needs the local time should format it there
//! and pass the string in.

const DAY_NAMES: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];

const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Formats a Unix timestamp the way `java.util.Date.toString` does, in UTC.
///
/// The result is `Mon Sep 02 14:00:54 UTC 2016`. Seconds before the epoch are
/// accepted and rendered with a proleptic Gregorian calendar.
///
/// # Example
///
/// ```
/// use javaproperties::java_timestamp_utc;
///
/// assert_eq!(java_timestamp_utc(0), "Thu Jan 01 00:00:00 UTC 1970");
/// assert_eq!(
///     java_timestamp_utc(1_700_000_000),
///     "Tue Nov 14 22:13:20 UTC 2023"
/// );
/// ```
#[must_use]
pub fn java_timestamp_utc(seconds: i64) -> String {
    let days = seconds.div_euclid(86_400);
    let second_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    // 1970-01-01 was a Thursday, and the table above starts at Monday.
    let weekday = (days + 3).rem_euclid(7) as usize;
    let hour = second_of_day / 3_600;
    let minute = (second_of_day % 3_600) / 60;
    let second = second_of_day % 60;
    format!(
        "{} {} {:02} {:02}:{:02}:{:02} UTC {year:04}",
        DAY_NAMES[weekday],
        MONTH_NAMES[month as usize - 1],
        day,
        hour,
        minute,
        second
    )
}

/// Converts a count of days since 1970-01-01 into a civil date.
///
/// Howard Hinnant's `civil_from_days`, which is exact for the whole proleptic
/// Gregorian calendar and has no branches that depend on the year's sign.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    // Shift the epoch to 0000-03-01 so that a leap day is the last day of a year.
    let shifted = days + 719_468;
    let era = if shifted >= 0 {
        shifted
    } else {
        shifted - 146_096
    } / 146_097;
    let day_of_era = (shifted - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_epoch_is_a_thursday() {
        assert_eq!(java_timestamp_utc(0), "Thu Jan 01 00:00:00 UTC 1970");
    }

    #[test]
    fn a_known_instant_formats_the_way_java_does() {
        assert_eq!(
            java_timestamp_utc(1_700_000_000),
            "Tue Nov 14 22:13:20 UTC 2023"
        );
        assert_eq!(
            java_timestamp_utc(946_684_800),
            "Sat Jan 01 00:00:00 UTC 2000"
        );
        assert_eq!(
            java_timestamp_utc(1_000_000_000),
            "Sun Sep 09 01:46:40 UTC 2001"
        );
    }

    #[test]
    fn a_leap_day_is_a_leap_day() {
        // 2000-02-29 is a leap day in a century year divisible by 400.
        assert_eq!(
            java_timestamp_utc(951_782_400),
            "Tue Feb 29 00:00:00 UTC 2000"
        );
    }

    #[test]
    fn seconds_before_the_epoch_go_backwards() {
        assert_eq!(java_timestamp_utc(-1), "Wed Dec 31 23:59:59 UTC 1969");
    }
}
