//! Checks the 0.8.0 calendars against the Python `holidays` package.
//!
//! The rows come from `scripts/fuzz/holidays-rs/gen.py`, which records the
//! reference's answer for Bermuda, Dominica, Grenada, Gibraltar, the Solomon
//! Islands and the British Virgin Islands over each country's own window. The
//! reference is not installed here - the fixture is the frozen answer - so this
//! runs in CI, where `verify_holidays.py` does not.
//!
//! Three kinds of row, the same classes the gate uses. `must` is a reference
//! holiday on a working day the crate owes. `allow` is a reference holiday at a
//! weekend, which the crate may keep or move, so it only bounds the crate from
//! above. `absent` is a date the reference has nothing for, where the crate must
//! be silent. A crate date outside `must ∪ allow` invents a day off; a `must`
//! date the crate does not report drops a working day it owes.

use std::collections::{HashMap, HashSet};

use holidays_rs::{Calendar, Country, Date};

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
fn new_calendars_agree_with_the_reference() {
    // country -> (must, allow, absent)
    let mut must: HashMap<String, HashSet<Date>> = HashMap::new();
    let mut allow: HashMap<String, HashSet<Date>> = HashMap::new();
    let mut absent: HashMap<String, HashSet<Date>> = HashMap::new();

    for line in include_str!("fixtures/holidays_oracle.tsv").lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        let bucket = match f[2] {
            "must" => &mut must,
            "allow" => &mut allow,
            "absent" => &mut absent,
            other => panic!("unknown kind {other}"),
        };
        bucket
            .entry(f[0].to_string())
            .or_default()
            .insert(date(f[1]));
    }

    let mut problems = Vec::new();
    let mut checked = 0;
    for (name, wanted) in &must {
        let cal = country(name);
        let allowed = &allow[name];
        let absent = &absent[name];

        // The crate's own answer over the fixture's window.
        let (first, last) = window(wanted, allowed, absent);
        let mut ours: HashSet<Date> = HashSet::new();
        for year in first..=last {
            for (d, _) in cal.holidays_in(year) {
                ours.insert(d);
            }
        }

        for d in wanted {
            if !ours.contains(d) {
                problems.push(format!("{name} {d}: required date not reported"));
            }
        }
        for d in &ours {
            if !wanted.contains(d) && !allowed.contains(d) {
                problems.push(format!(
                    "{name} {d}: reported but the reference has no holiday"
                ));
            }
        }
        for d in absent {
            if ours.contains(d) {
                problems.push(format!("{name} {d}: reported, but the reference has none"));
            }
        }
        checked += wanted.len();
    }

    assert!(
        problems.is_empty(),
        "{} disagreement(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
    assert!(
        checked > 5_000,
        "only {checked} required dates were checked"
    );
}

/// The inclusive year range the fixture covers, from its own dates.
fn window(must: &HashSet<Date>, allow: &HashSet<Date>, absent: &HashSet<Date>) -> (i32, i32) {
    let all = must.iter().chain(allow).chain(absent);
    let mut first = i32::MAX;
    let mut last = i32::MIN;
    for d in all {
        first = first.min(d.year);
        last = last.max(d.year);
    }
    (first, last)
}
