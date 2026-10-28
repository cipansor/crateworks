#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod countries;
mod date;

pub use countries::{Calendar, Country, SubstituteStyle};
pub use date::Date;

/// The direction to search for the nearest holiday.
///
/// Used by [`Calendar::closest_holiday`], which mirrors the reference's
/// `get_closest_holiday`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    /// Search forwards, to the first holiday strictly after the date.
    Forward,
    /// Search backwards, to the last holiday strictly before the date.
    Backward,
}

/// A rule that determines the date of a holiday within a year.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Rule {
    /// A fixed month and day, for example 17 August.
    Fixed {
        /// Month, 1-12.
        month: u8,
        /// Day, 1-31.
        day: u8,
    },
    /// A date relative to Easter Sunday, in days. Negative is before Easter.
    EasterOffset(i32),
    /// A date relative to Orthodox Easter Sunday, in days.
    ///
    /// Orthodox Easter is computed with the Julian computus and then converted
    /// to the Gregorian calendar, which puts it between one and five weeks
    /// after the Western Easter. Greece, Cyprus, Romania and Bulgaria all
    /// observe the movable feasts on this date, and it is a different date from
    /// [`Rule::EasterOffset`] often enough that sharing one rule would be
    /// wrong.
    OrthodoxEasterOffset(i32),
    /// The *n*-th weekday of a month. `nth` of 5 means "last".
    NthWeekday {
        /// Month, 1-12.
        month: u8,
        /// Weekday, 0 is Sunday.
        weekday: u8,
        /// Which occurrence, 1-based. 5 means the last one in the month.
        nth: u8,
    },
    /// A fixed date, except that when it falls on a given weekday a different
    /// day of the same month is used instead.
    ///
    /// The Netherlands uses this for Koningsdag: 27 April, or 26 April when
    /// the 27th is a Sunday.
    FixedUnlessWeekday {
        /// Month, 1-12.
        month: u8,
        /// Day that normally holds the holiday.
        day: u8,
        /// Weekday that triggers the alternative, 0 is Sunday.
        weekday: u8,
        /// Day of the same month to use instead.
        then_day: u8,
    },
    /// The fixed date when it falls on a given weekday, otherwise the first
    /// `weekday` on or after it.
    ///
    /// Ireland uses this for Saint Brigid's Day: 1 February when that is a
    /// Friday, otherwise the first Monday in February.
    FixedOnWeekdayElseWeekdayOnOrAfter {
        /// Month, 1-12.
        month: u8,
        /// The fixed date.
        day: u8,
        /// Weekday on which the fixed date is kept, 0 is Sunday.
        on_weekday: u8,
        /// Weekday to advance to otherwise, 0 is Sunday.
        weekday: u8,
    },
    /// The first `weekday` on or after a fixed date.
    ///
    /// The result may fall in the following month, which is required by the
    /// Nordic All Saints' Day: the Saturday between 31 October and
    /// 6 November.
    WeekdayOnOrAfter {
        /// Month, 1-12.
        month: u8,
        /// Earliest day the holiday may fall on.
        day: u8,
        /// Weekday, 0 is Sunday.
        weekday: u8,
    },
    /// The last `weekday` on or before a fixed date.
    ///
    /// The result may fall in the preceding month.
    WeekdayOnOrBefore {
        /// Month, 1-12.
        month: u8,
        /// Latest day the holiday may fall on.
        day: u8,
        /// Weekday, 0 is Sunday.
        weekday: u8,
    },
    /// A holiday that exists in one year only, such as a one-off jubilee or a
    /// day moved for a national event.
    Once {
        /// The only year in which the holiday exists.
        year: i32,
        /// Month, 1-12.
        month: u8,
        /// Day, 1-31.
        day: u8,
    },
    /// A rule that does not apply in some years, because a one-off date took
    /// its place. The United Kingdom moved its early May bank holiday in 2020
    /// rather than adding a second one.
    ExceptYears {
        /// The rule that normally applies.
        base: &'static Rule,
        /// Years in which it does not.
        years: &'static [i32],
    },
    /// A fixed date moved forward off a weekend, and one day further when the
    /// day it lands on is Orthodox Easter Monday.
    ///
    /// Greece moves Labour Day this way. 1 May 2021 fell on a Saturday, so it
    /// was observed on the next working day, but that Monday was Orthodox
    /// Easter Monday and already a holiday, so it moved on to the Tuesday. In
    /// 2000 the first of May *was* Orthodox Easter Monday and moved to the
    /// second. Both shapes are the same rule.
    ObservedOffWeekendAndOrthodoxEasterMonday {
        /// Month, 1-12.
        month: u8,
        /// Day that normally holds the holiday.
        day: u8,
    },
    /// A holiday that begins in a given year and applies from then on.
    ///
    /// Juneteenth became a United States federal holiday in 2021 and Saint
    /// Brigid's Day an Irish public holiday in 2023. Before those years the
    /// date is an ordinary working day, so resolving it would invent a day off.
    Since {
        /// The first year in which the holiday applies, inclusive.
        year: i32,
        /// The rule that applies from that year on.
        base: &'static Rule,
    },
    /// A holiday that stops applying after a given year.
    ///
    /// Denmark abolished Great Prayer Day after 2023, so it resolves to nothing
    /// from 2024 on.
    Until {
        /// The last year in which the holiday applies, inclusive.
        year: i32,
        /// The rule that applies up to and including that year.
        base: &'static Rule,
    },
    /// A fixed date that recurs only in some years, selected by the year's
    /// remainder.
    ///
    /// The Netherlands observes Liberation Day on 5 May only every fifth year,
    /// which is a property of the year rather than of the date.
    EveryNYears {
        /// The interval in years. The holiday applies when the year is a
        /// multiple of this.
        step: u32,
        /// Month, 1-12.
        month: u8,
        /// Day, 1-31.
        day: u8,
    },
    /// A holiday that falls every *n* years from a given year.
    ///
    /// Mexico's sexennial transfer of federal power is on 1 December in 1970,
    /// 1976 and so on, and on 1 October from 2024, so the series is anchored
    /// at a year rather than being "every year divisible by six".
    EveryNYearsSince {
        /// The first year of the series.
        first: i32,
        /// The interval in years.
        step: u32,
        /// Month, 1-12.
        month: u8,
        /// Day, 1-31.
        day: u8,
    },
    /// A holiday whose dates are published as a table rather than computed.
    ///
    /// New Zealand's Matariki follows an official calendar that is set by
    /// advisory group rather than by a formula, so a formula would be a guess.
    /// A year absent from the table has no occurrence.
    Table {
        /// The year, month and day of each occurrence.
        dates: &'static [(i32, u8, u8)],
    },
    /// A holiday that only gained a substitute day from a given year.
    ///
    /// New Zealand has Mondayised Waitangi Day and Anzac Day only since 2014,
    /// when the Holidays (Full Recognition of Waitangi Day and ANZAC Day)
    /// Amendment Act came into force. Before that a weekend occurrence was
    /// simply lost, so applying the country style would invent a day off.
    ///
    /// Mondayisation moves the holiday to the following Monday even when that
    /// Monday is already a holiday, which is what `shared` records: Anzac Day
    /// 2038 falls on a Sunday and shares Monday 26 April with Easter Monday.
    ObservedFrom {
        /// The first year in which a weekend occurrence is moved.
        year: i32,
        /// Whether the moved date may coincide with another holiday.
        shared: bool,
        /// The rule for the date the holiday falls on.
        base: &'static Rule,
    },
    /// A holiday that never gains a substitute day, even when the country
    /// moves its other weekend holidays.
    ///
    /// Serbia substitutes the first day of each of its two-day holidays but
    /// not the second, so 1 January 2022 moved to Monday the 3rd while 2
    /// January stayed on the Sunday it fell on. The country style cannot say
    /// that, so the exception is carried by the holiday.
    NeverSubstituted {
        /// The rule for the date the holiday falls on.
        base: &'static Rule,
    },
    /// A fixed date that is not a holiday at all when it falls on a given
    /// weekday.
    ///
    /// Mexico does not move a holiday off a Sunday: 1 January and 25 December
    /// are ordinary days again when they land there. That is the opposite of a
    /// substitute day, and it cannot be expressed by a country style, because
    /// the country still observes its Monday-based holidays on a Sunday.
    Absorbed {
        /// Month, 1-12.
        month: u8,
        /// Day, 1-31.
        day: u8,
        /// The weekday on which the holiday is dropped, 0 is Sunday.
        weekday: u8,
    },
    /// A rule that applies only within an inclusive range of years.
    ///
    /// Italy's Republic Day was a movable first Sunday in June from 1977 to
    /// 2000 and a fixed 2 June either side of that, so the two rules are
    /// separated by the years they cover.
    Between {
        /// First year the rule applies, inclusive.
        from: i32,
        /// Last year the rule applies, inclusive.
        to: i32,
        /// The rule that applies within the range.
        base: &'static Rule,
    },
    /// A holiday that moves off particular weekdays by a fixed number of days.
    ///
    /// The country-wide [`SubstituteStyle`] cannot describe every country. The
    /// reference libraries decide substitution per holiday, so a single country
    /// may keep one holiday where it falls, roll another forward a day and roll
    /// a third forward two: Jamaica observes Christmas Day on the Tuesday when
    /// it falls on a Sunday, because Boxing Day already holds the Monday.
    ///
    /// `shifts` is the same shape the reference uses: a list of
    /// `(weekday, days)` pairs. A weekday absent from the list keeps the
    /// holiday on the date it falls, which is how a country says "this holiday
    /// is never moved". Before `from` the holiday is not moved at all, because
    /// a country adopts the rule at a particular time.
    ///
    /// The shifted date is placed exactly where the rule puts it, even when
    /// another holiday already holds that date: Niue's Constitution Day Holiday
    /// lands on the Monday that is also Peniamina Gospel Day, and the reference
    /// reports both on it.
    ObservedShift {
        /// The rule for the date the holiday falls on.
        base: &'static Rule,
        /// The first year in which the shift applies, inclusive.
        from: i32,
        /// `(weekday, days)` pairs. Weekday 0 is Sunday, as everywhere else in
        /// this crate.
        shifts: &'static [(u8, i32)],
    },
}

impl Rule {
    /// Whether this rule is deliberately tied to particular years, so that
    /// resolving it to nothing outside them is correct rather than a mistake.
    #[must_use]
    pub const fn is_year_restricted(&self) -> bool {
        match self {
            Self::Once { .. }
            | Self::ExceptYears { .. }
            | Self::Since { .. }
            | Self::Until { .. }
            | Self::EveryNYears { .. }
            | Self::EveryNYearsSince { .. }
            | Self::Table { .. }
            | Self::Between { .. } => true,
            Self::Fixed { .. }
            | Self::EasterOffset(_)
            | Self::OrthodoxEasterOffset(_)
            | Self::NthWeekday { .. }
            | Self::FixedUnlessWeekday { .. }
            | Self::FixedOnWeekdayElseWeekdayOnOrAfter { .. }
            | Self::WeekdayOnOrAfter { .. }
            | Self::WeekdayOnOrBefore { .. }
            | Self::ObservedFrom { .. }
            | Self::NeverSubstituted { .. }
            | Self::Absorbed { .. }
            | Self::ObservedOffWeekendAndOrthodoxEasterMonday { .. } => false,
            // The shift wraps another rule and does not itself restrict the
            // year, but a base that does (`Between`, `Since`, `Once`) still
            // makes the holiday absent outside its window.
            Self::ObservedShift { base, .. } => base.is_year_restricted(),
        }
    }

    /// Resolves this rule to the date on which the holiday *falls*, before any
    /// weekend substitution is applied.
    ///
    /// Returns `None` only when the rule describes an impossible date, which
    /// can happen for the fixed rules on a short month.
    #[must_use]
    pub fn resolve(&self, year: i32) -> Option<Date> {
        match *self {
            Self::Fixed { month, day } => Date::new(year, month, day),
            Self::EasterOffset(offset) => Date::easter(year)?.add_days(offset),
            Self::OrthodoxEasterOffset(offset) => Date::orthodox_easter(year)?.add_days(offset),
            Self::ObservedOffWeekendAndOrthodoxEasterMonday { month, day } => {
                let date = Date::new(year, month, day)?;
                // A weekend occurrence moves to the next working day, as the
                // country does for any holiday.
                let moved = match date.weekday() {
                    6 => date.add_days(2)?,
                    0 => date.add_days(1)?,
                    _ => date,
                };
                // If that working day is already Orthodox Easter Monday the
                // holiday moves one day further, because a day cannot be
                // observed twice.
                let easter_monday = Date::orthodox_easter(year)?.add_days(1)?;
                if moved == easter_monday {
                    moved.add_days(1)
                } else {
                    Some(moved)
                }
            }
            Self::NthWeekday {
                month,
                weekday,
                nth,
            } => Date::nth_weekday(year, month, weekday, nth),
            Self::FixedUnlessWeekday {
                month,
                day,
                weekday,
                then_day,
            } => {
                let date = Date::new(year, month, day)?;
                if date.weekday() == weekday {
                    Date::new(year, month, then_day)
                } else {
                    Some(date)
                }
            }
            Self::FixedOnWeekdayElseWeekdayOnOrAfter {
                month,
                day,
                on_weekday,
                weekday,
            } => {
                let date = Date::new(year, month, day)?;
                if date.weekday() == on_weekday {
                    Some(date)
                } else {
                    let resolved = date.next_or_same_weekday(weekday);
                    // Staying inside the month keeps a mistyped rule visible
                    // rather than silently inventing a holiday in the next one.
                    (resolved.month == month).then_some(resolved)
                }
            }
            Self::WeekdayOnOrAfter {
                month,
                day,
                weekday,
            } => {
                let date = Date::new(year, month, day)?;
                Some(date.next_or_same_weekday(weekday))
            }
            Self::WeekdayOnOrBefore {
                month,
                day,
                weekday,
            } => {
                let date = Date::new(year, month, day)?;
                let back = (date.weekday() as i64 - weekday as i64).rem_euclid(7) as i32;
                date.add_days(-back)
            }
            Self::Once {
                year: only,
                month,
                day,
            } => (year == only)
                .then(|| Date::new(year, month, day))
                .flatten(),
            Self::ExceptYears { base, years } => {
                if years.contains(&year) {
                    None
                } else {
                    base.resolve(year)
                }
            }
            Self::Since { year: first, base } => {
                if year >= first {
                    base.resolve(year)
                } else {
                    None
                }
            }
            Self::Until { year: last, base } => {
                if year <= last {
                    base.resolve(year)
                } else {
                    None
                }
            }
            Self::EveryNYears { step, month, day } => {
                if step > 0 && year.rem_euclid(step as i32) == 0 {
                    Date::new(year, month, day)
                } else {
                    None
                }
            }
            Self::EveryNYearsSince {
                first,
                step,
                month,
                day,
            } => {
                if step > 0 && year >= first && (year - first).rem_euclid(step as i32) == 0 {
                    Date::new(year, month, day)
                } else {
                    None
                }
            }
            Self::Table { dates } => dates
                .iter()
                .find(|(y, _, _)| *y == year)
                .and_then(|(y, m, d)| Date::new(*y, *m, *d)),
            Self::Absorbed {
                month,
                day,
                weekday,
            } => {
                let date = Date::new(year, month, day)?;
                (date.weekday() != weekday).then_some(date)
            }
            Self::NeverSubstituted { base } | Self::ObservedFrom { base, .. } => base.resolve(year),
            Self::ObservedShift { base, .. } => base.resolve(year),
            Self::Between { from, to, base } => {
                if (from..=to).contains(&year) {
                    base.resolve(year)
                } else {
                    None
                }
            }
        }
    }

    /// Whether a weekend occurrence of this holiday moves to a substitute day
    /// in `year`.
    ///
    /// Almost every rule does, because the country style decides that. The
    /// exception is [`Rule::ObservedFrom`], which models a holiday that only
    /// gained a substitute day from a given year.
    #[must_use]
    pub const fn substitutes_in(&self, year: i32) -> bool {
        match self {
            // The year wrappers pass the question down, so a
            // `NeverSubstituted` or an `ObservedFrom` nested inside a `Since`
            // is still seen.
            Self::NeverSubstituted { .. } => false,
            Self::ObservedFrom { year: first, .. } => year >= *first,
            // A per-holiday shift replaces the country style entirely, so the
            // country style must not apply on top of it.
            Self::ObservedShift { .. } => false,
            Self::Since { year: first, base } => year >= *first && base.substitutes_in(year),
            Self::Until { year: last, base } => year <= *last && base.substitutes_in(year),
            Self::Between { from, to, base } => {
                *from <= year && year <= *to && base.substitutes_in(year)
            }
            // `slice::contains` is not a const fn, so the year list is walked
            // by hand here.
            Self::ExceptYears { base, years } => {
                let mut excluded = false;
                let mut i = 0;
                while i < years.len() {
                    excluded = excluded || years[i] == year;
                    i += 1;
                }
                !excluded && base.substitutes_in(year)
            }
            _ => true,
        }
    }

    /// The per-holiday shift this rule applies in `year`, if any.
    ///
    /// Returns the number of days the holiday moves. A rule without its own
    /// shift returns `None`, and the country style decides instead. A weekday
    /// absent from the shift table is not moved, which is how the table says
    /// "this holiday is never moved".
    ///
    /// The year wrappers pass the question down, so a shift nested inside a
    /// `Since` or a `Between` is still seen.
    #[must_use]
    pub const fn observed_shift(&self, year: i32, weekday: u8) -> Option<i32> {
        match self {
            Self::ObservedShift { from, shifts, .. } => {
                if year < *from {
                    return None;
                }
                let mut i = 0;
                while i < shifts.len() {
                    if shifts[i].0 == weekday {
                        return Some(shifts[i].1);
                    }
                    i += 1;
                }
                None
            }
            Self::Since { year: first, base } => {
                if year >= *first {
                    base.observed_shift(year, weekday)
                } else {
                    None
                }
            }
            Self::Until { year: last, base } => {
                if year <= *last {
                    base.observed_shift(year, weekday)
                } else {
                    None
                }
            }
            Self::Between { from, to, base } => {
                if *from <= year && year <= *to {
                    base.observed_shift(year, weekday)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Whether a substitute date may coincide with another holiday.
    ///
    /// Most substitutes move to the next free day, because two holidays on one
    /// date is what the collision rule exists to avoid. Mondayisation is the
    /// exception: New Zealand's Anzac Day 2038 falls on a Sunday and moves to
    /// Monday 26 April even though Easter Monday is already there.
    #[must_use]
    pub const fn substitute_may_share(&self) -> bool {
        matches!(self, Self::ObservedFrom { shared: true, .. })
    }
}

/// A named holiday.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Holiday {
    /// Local name of the holiday.
    pub name: &'static str,
    /// Rule that produces the date.
    pub rule: Rule,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_rule_resolves() {
        assert_eq!(
            Rule::Fixed { month: 8, day: 17 }.resolve(2026),
            Date::new(2026, 8, 17)
        );
    }

    #[test]
    fn fixed_rule_rejects_impossible_dates() {
        assert!(Rule::Fixed { month: 2, day: 30 }.resolve(2026).is_none());
        assert!(Rule::Fixed { month: 2, day: 29 }.resolve(2026).is_none());
        assert!(Rule::Fixed { month: 2, day: 29 }.resolve(2028).is_some());
    }

    #[test]
    fn easter_offsets() {
        // Easter Sunday 2024 was 31 March; Good Friday is two days before.
        let good_friday = Rule::EasterOffset(-2).resolve(2024).unwrap();
        assert_eq!((good_friday.month, good_friday.day), (3, 29));
    }

    #[test]
    fn nth_weekday_handles_last_occurrence() {
        // Last Monday of May 2026 is the 25th.
        let d = Rule::NthWeekday {
            month: 5,
            weekday: 1,
            nth: 5,
        }
        .resolve(2026)
        .unwrap();
        assert_eq!(d.day, 25);
    }

    #[test]
    fn nth_weekday_handles_last_when_month_has_five() {
        // Thursdays in October 2026: 1, 8, 15, 22, 29.
        let d = Rule::NthWeekday {
            month: 10,
            weekday: 4,
            nth: 5,
        }
        .resolve(2026)
        .unwrap();
        assert_eq!(d.day, 29);
    }

    #[test]
    fn fixed_unless_weekday_switches_only_on_that_day() {
        let rule = Rule::FixedUnlessWeekday {
            month: 4,
            day: 27,
            weekday: 0,
            then_day: 26,
        };
        // 27 April 2025 is a Sunday, so Koningsdag moves to the 26th.
        assert_eq!(rule.resolve(2025).unwrap().day, 26);
        // 27 April 2026 is a Monday, so it stays.
        assert_eq!(rule.resolve(2026).unwrap().day, 27);
        // A Saturday does not trigger the switch.
        assert_eq!(rule.resolve(2030).unwrap().day, 27);
    }

    #[test]
    fn fixed_on_weekday_else_weekday_on_or_after() {
        let rule = Rule::FixedOnWeekdayElseWeekdayOnOrAfter {
            month: 2,
            day: 1,
            on_weekday: 5,
            weekday: 1,
        };
        // 1 February 2030 is a Friday, so the fixed date is kept.
        assert_eq!(rule.resolve(2030).unwrap().day, 1);
        // Otherwise the first Monday follows: 2026-02-02.
        let d = rule.resolve(2026).unwrap();
        assert_eq!((d.month, d.day, d.weekday()), (2, 2, 1));
    }

    #[test]
    fn weekday_on_or_after_follows_the_fixed_date() {
        let rule = Rule::WeekdayOnOrAfter {
            month: 6,
            day: 20,
            weekday: 6,
        };
        // Saturdays in June 2026: 6, 13, 20, 27.
        assert_eq!(rule.resolve(2026).unwrap().day, 20);
        // Saturdays in June 2027: 5, 12, 19, 26.
        assert_eq!(rule.resolve(2027).unwrap().day, 26);
    }

    #[test]
    fn weekday_on_or_after_crosses_into_the_next_month() {
        // Nordic All Saints' Day: the Saturday between 31 October and
        // 6 November, which in 2024 was 2 November.
        let rule = Rule::WeekdayOnOrAfter {
            month: 10,
            day: 31,
            weekday: 6,
        };
        let d = rule.resolve(2024).unwrap();
        assert_eq!((d.month, d.day), (11, 2));
        // In 2026 the 31st is already a Saturday.
        let d = rule.resolve(2026).unwrap();
        assert_eq!((d.month, d.day), (10, 31));
    }

    #[test]
    fn weekday_on_or_before_steps_back_from_the_fixed_date() {
        let rule = Rule::WeekdayOnOrBefore {
            month: 5,
            day: 24,
            weekday: 1,
        };
        // Victoria Day: the Monday on or before 24 May.
        assert_eq!(rule.resolve(2024).unwrap().day, 20);
        assert_eq!(rule.resolve(2027).unwrap().day, 24);
        // A date early in the month can step into the previous one.
        let crossing = Rule::WeekdayOnOrBefore {
            month: 5,
            day: 1,
            weekday: 1,
        };
        let d = crossing.resolve(2026).unwrap();
        assert_eq!((d.month, d.day), (4, 27));
    }

    #[test]
    fn absorbed_drops_a_fixed_date_on_one_weekday() {
        // Mexico's New Year's Day is not observed at all when it is a Sunday,
        // which is the opposite of a substitute day.
        const NEW_YEAR: Rule = Rule::Absorbed {
            month: 1,
            day: 1,
            weekday: 0,
        };
        // 1 January 2023 was a Sunday.
        assert!(NEW_YEAR.resolve(2023).is_none());
        // 1 January 2024 was a Monday.
        assert_eq!(NEW_YEAR.resolve(2024), Date::new(2024, 1, 1));
        assert!(!NEW_YEAR.is_year_restricted());
    }

    #[test]
    fn never_substituted_overrides_the_country_style() {
        const DAY: Rule = Rule::NeverSubstituted {
            base: &Rule::Fixed { month: 1, day: 1 },
        };
        assert!(DAY.resolve(2028).is_some());
        assert!(!DAY.substitutes_in(2028));
        // The wrapper is transparent to `resolve`.
        assert_eq!(DAY.resolve(2028), Date::new(2028, 1, 1));
    }

    #[test]
    fn every_n_years_since_is_anchored_at_a_year() {
        // The sexennial transfer of federal power is 1970, 1976, ... rather
        // than every year divisible by six.
        const TRANSFER: Rule = Rule::EveryNYearsSince {
            first: 1970,
            step: 6,
            month: 12,
            day: 1,
        };
        assert!(TRANSFER.resolve(1969).is_none());
        assert!(TRANSFER.resolve(1970).is_some());
        assert!(TRANSFER.resolve(1971).is_none());
        assert!(TRANSFER.resolve(1976).is_some());
        assert!(TRANSFER.is_year_restricted());
    }

    #[test]
    fn a_wrapper_passes_the_substitution_question_down() {
        // `NeverSubstituted` nested inside a `Since` must still be seen, which
        // is what `substitutes_in` recursing into the year wrappers buys.
        const NESTED: Rule = Rule::Since {
            year: 2007,
            base: &Rule::NeverSubstituted {
                base: &Rule::Fixed { month: 1, day: 7 },
            },
        };
        assert!(!NESTED.substitutes_in(2010));
        assert!(NESTED.resolve(2006).is_none());
        assert!(NESTED.resolve(2010).is_some());
    }

    #[test]
    fn since_until_and_between_bound_a_rule() {
        const EPIPHANY: Rule = Rule::Fixed { month: 1, day: 6 };
        let since = Rule::Since {
            year: 2011,
            base: &EPIPHANY,
        };
        assert!(since.resolve(2010).is_none());
        assert!(since.resolve(2011).is_some());
        assert!(since.is_year_restricted());

        let until = Rule::Until {
            year: 2004,
            base: &EPIPHANY,
        };
        assert!(until.resolve(2004).is_some());
        assert!(until.resolve(2005).is_none());

        let between = Rule::Between {
            from: 1977,
            to: 2000,
            base: &EPIPHANY,
        };
        assert!(between.resolve(1976).is_none());
        assert!(between.resolve(1977).is_some());
        assert!(between.resolve(2000).is_some());
        assert!(between.resolve(2001).is_none());
    }

    #[test]
    fn observed_from_defers_substitution_until_a_year() {
        let anzac = Rule::ObservedFrom {
            year: 2014,
            shared: true,
            base: &Rule::Fixed { month: 4, day: 25 },
        };
        // The date it falls on is unaffected.
        assert_eq!(
            anzac.resolve(2013).unwrap(),
            Date::new(2013, 4, 25).unwrap()
        );
        assert!(!anzac.substitutes_in(2013));
        assert!(anzac.substitutes_in(2014));
        assert!(anzac.substitute_may_share());
        assert!(!Rule::Fixed { month: 4, day: 25 }.substitute_may_share());
    }

    #[test]
    fn observed_shift_moves_a_holiday_off_named_weekdays() {
        // Jamaica's Christmas Day: two days forward from a Sunday, so it does
        // not collide with Boxing Day, and not moved otherwise.
        const CHRISTMAS: Rule = Rule::ObservedShift {
            base: &Rule::Fixed { month: 12, day: 25 },
            from: 2000,
            shifts: &[(0, 2)],
        };
        // 2022-12-25 is a Sunday.
        let d = CHRISTMAS.resolve(2022).unwrap();
        assert_eq!(d.weekday(), 0);
        assert_eq!(CHRISTMAS.observed_shift(2022, 0), Some(2));
        // 2024-12-25 is a Wednesday, which the table does not name.
        let d = CHRISTMAS.resolve(2024).unwrap();
        assert_eq!(d.weekday(), 3);
        assert_eq!(CHRISTMAS.observed_shift(2024, 3), None);
        // The shift does not apply before `from`.
        assert_eq!(CHRISTMAS.observed_shift(1999, 0), None);
        // A shift is not itself substituted by the country style.
        assert!(!CHRISTMAS.substitutes_in(2022));
        assert!(!CHRISTMAS.substitute_may_share());
        // A base that restricts the year is still seen through the shift.
        const SINCE: Rule = Rule::ObservedShift {
            base: &Rule::Since {
                year: 2015,
                base: &Rule::Fixed { month: 1, day: 4 },
            },
            from: 2000,
            shifts: &[(0, -1)],
        };
        assert!(SINCE.is_year_restricted());
        assert!(SINCE.resolve(2014).is_none());
        assert!(SINCE.resolve(2015).is_some());
    }
}
