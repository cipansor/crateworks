//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use hijridate::{Gregorian, Hijri};

    let h = Hijri::new(1445, 9, 1).unwrap();
    assert_eq!(h.to_gregorian().to_string(), "2024-03-11");
    assert_eq!(h.month_name(), "Ramadan");

    let g = Gregorian::new(2024, 3, 11).unwrap();
    assert_eq!(g.to_hijri().to_string(), "1445-09-01");

    let h = Hijri::new(1447, 9, 1).unwrap();
    assert_eq!(h.isoformat(), "1447-09-01");
    assert_eq!(h.dmyformat("/", true), "01/09/1447");
    assert_eq!(h.dmyformat("-", false), "1-9-1447");
    assert_eq!(h.datetuple(), (1447, 9, 1));
    assert_eq!(h.month_name(), "Ramadan");
    assert_eq!(h.day_name(), "Wednesday");
    assert_eq!(h.year_length(), 355);
    assert_eq!(h.notation(), "AH");

    let g = Gregorian::new(2026, 2, 18).unwrap();
    assert_eq!(g.month_name(), "February");
    assert_eq!(g.notation(), "CE");
    assert_eq!(g.isoweekday(), 3);
}
