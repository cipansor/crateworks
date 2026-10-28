//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use holidays_rs::{Calendar, Date, Direction};

    let cal = Calendar::indonesia();
    let d = Date::new(2026, 8, 17).unwrap();
    assert!(cal.is_holiday(d));
    assert_eq!(cal.next_working_day(d), Date::new(2026, 8, 18).unwrap());

    let us = Calendar::united_states();
    let (date, name) = us
        .closest_holiday(Date::new(2026, 2, 18).unwrap(), Direction::Forward)
        .unwrap();
    assert_eq!(name, "Memorial Day");
    assert_eq!(date, Date::new(2026, 5, 25).unwrap());
}
