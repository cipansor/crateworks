//! Checks `Calendar::closest_holiday` against the Python `holidays` package.
//!
//! The rows come from `research/tools/gen_holidays_closest.py`. It lists only
//! the countries whose holiday set already matches the reference, because a
//! closest-holiday query over a different set answers a different question; the
//! excluded countries and why are in that script's `EXCLUDED` map.

use holidays_rs::{Calendar, Country, Date, Direction};

fn date(text: &str) -> Date {
    let p: Vec<i32> = text.split('-').map(|x| x.parse().unwrap()).collect();
    Date::new(p[0], p[1] as u8, p[2] as u8).unwrap()
}

fn country(name: &str) -> Calendar {
    let c = Country::ALL
        .iter()
        .copied()
        .find(|c| format!("{c:?}") == name)
        .unwrap_or_else(|| panic!("unknown country {name}"));
    Calendar::new(c)
}

#[test]
fn closest_holiday_agrees_with_the_reference() {
    let mut checked = 0;
    let mut mismatches = Vec::new();
    for line in include_str!("fixtures/closest.tsv").lines().skip(1) {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let cal = country(f[0]);
        let probe = date(f[1]);
        let fwd = cal
            .closest_holiday(probe, Direction::Forward)
            .map(|(d, _)| d);
        let bwd = cal
            .closest_holiday(probe, Direction::Backward)
            .map(|(d, _)| d);
        let want_fwd = (f[2] != "-").then(|| date(f[2]));
        let want_bwd = (f[3] != "-").then(|| date(f[3]));
        if fwd != want_fwd || bwd != want_bwd {
            mismatches.push(format!(
                "{} {} forward {:?} want {:?}, backward {:?} want {:?}",
                f[0], f[1], fwd, want_fwd, bwd, want_bwd
            ));
        }
        checked += 1;
    }
    assert!(
        mismatches.is_empty(),
        "{} of {checked} rows disagree:\n{}",
        mismatches.len(),
        mismatches.join("\n")
    );
    assert!(checked > 10_000, "only {checked} rows were checked");
}
