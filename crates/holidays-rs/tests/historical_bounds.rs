//! Checks that a rule which ends in a given year does not also run backwards.
//!
//! `Rule::Until` sets an inclusive last year and no first year, so a holiday
//! whose modern form ends before the reference's `start_year` was reported in
//! every earlier year too. The reference answers nothing before its own start
//! year, so each case below pairs the first valid year with the year before it.
//!
//! Expectations come from the pinned `holidays==0.108` reference; the years
//! quoted are its `start_year` for the country.

use holidays_rs::{Calendar, Date};

fn d(y: i32, m: u8, day: u8) -> Date {
    Date::new(y, m, day).unwrap()
}

fn assert_holiday(cal: &Calendar, date: Date) {
    assert!(
        cal.is_holiday(date),
        "{date} should be a holiday for {}",
        cal.country().name()
    );
}

fn assert_not_holiday(cal: &Calendar, date: Date) {
    assert!(
        !cal.is_holiday(date),
        "{date} should not be a holiday for {}",
        cal.country().name()
    );
}

#[test]
fn estonia_sovereignty_day_starts_at_the_reference_start_year() {
    let cal = Calendar::estonia();
    // The declaration was in 1988, but the reference reports the holiday only
    // from its Estonia start year of 1991.
    assert_not_holiday(&cal, d(1987, 11, 16));
    assert_not_holiday(&cal, d(1988, 11, 16));
    assert_not_holiday(&cal, d(1990, 11, 16));
    assert_holiday(&cal, d(1991, 11, 16));
    assert_holiday(&cal, d(1993, 11, 16));
    // Replaced by Independence Restoration Day on 20 August from 1998.
    assert_not_holiday(&cal, d(1994, 11, 16));
}

#[test]
fn estonia_sovereignty_day_is_a_working_day_before_1991() {
    // Monday. Before the fix the rule made this a holiday, which also removed
    // it from the working-day count.
    let cal = Calendar::estonia();
    assert!(cal.is_working_day(d(1987, 11, 16)));
    assert!(!cal.is_working_day(d(1991, 11, 16)));
}

#[test]
fn lesotho_kings_birthday_may_form_starts_at_the_reference_start_year() {
    let cal = Calendar::lesotho();
    // The 2 May form ran to 1997, but the reference's Lesotho start year is
    // 1996, so it must not be reported back to Moshoeshoe II's birth.
    assert_not_holiday(&cal, d(1935, 5, 2));
    assert_not_holiday(&cal, d(1960, 5, 2));
    assert_not_holiday(&cal, d(1995, 5, 2));
    assert_holiday(&cal, d(1996, 5, 2));
    assert_holiday(&cal, d(1997, 5, 2));
    // Letsie III's birthday moved to 17 July in 1998.
    assert_not_holiday(&cal, d(1998, 5, 2));
    assert_holiday(&cal, d(1998, 7, 17));
}

#[test]
fn france_victory_day_has_both_ends() {
    let cal = Calendar::france();
    // Instituted in 1953, dropped by the 1959 decree, restored by the 1982 law.
    assert_not_holiday(&cal, d(1952, 5, 8));
    assert_holiday(&cal, d(1953, 5, 8));
    assert_holiday(&cal, d(1959, 5, 8));
    assert_not_holiday(&cal, d(1960, 5, 8));
    assert_not_holiday(&cal, d(1981, 5, 8));
    assert_holiday(&cal, d(1982, 5, 8));
}

#[test]
fn french_overseas_victory_day_matches_france() {
    // These inherit the metropolitan rule, so they share both ends of it.
    for cal in [
        Calendar::french_guiana(),
        Calendar::guadeloupe(),
        Calendar::martinique(),
        Calendar::reunion(),
        Calendar::mayotte(),
        Calendar::new_caledonia(),
        Calendar::french_polynesia(),
        Calendar::saint_pierre_and_miquelon(),
    ] {
        assert_not_holiday(&cal, d(1952, 5, 8));
        assert_holiday(&cal, d(1953, 5, 8));
        assert_not_holiday(&cal, d(1960, 5, 8));
        assert_holiday(&cal, d(1982, 5, 8));
    }
}

#[test]
fn french_southern_territories_victory_day_starts_at_its_own_start_year() {
    let cal = Calendar::french_southern_territories();
    // The territory's reference start year is 1956, so the earlier form is
    // bounded there rather than at France's 1953.
    assert_not_holiday(&cal, d(1955, 5, 8));
    assert_holiday(&cal, d(1956, 5, 8));
    assert_not_holiday(&cal, d(1960, 5, 8));
    assert_holiday(&cal, d(1982, 5, 8));
}

#[test]
fn wallis_and_futuna_victory_day_starts_in_1982() {
    let cal = Calendar::wallis_and_futuna();
    // The territory's reference start year is 1962 and it never reports the
    // 1953 form, only the post-1982 restoration.
    assert_not_holiday(&cal, d(1962, 5, 8));
    assert_not_holiday(&cal, d(1981, 5, 8));
    assert_holiday(&cal, d(1982, 5, 8));
}
