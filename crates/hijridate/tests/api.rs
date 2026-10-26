//! Checks the formatting and naming API against `hijridate`.
//!
//! `reference.rs` pins the conversions, the month lengths and the weekday, which
//! are the parts with arithmetic in them. This pins the part that is pure
//! formatting and naming, which is where a plausible-looking implementation
//! drifts: a month name with a different apostrophe, a `dmyformat` that pads the
//! year, an `isoweekday` off by one.
//!
//! The rows come from `research/tools/gen_hijridate_api.py`, which calls the
//! `hijridate` Python package. Nothing here is hand-written.

use hijridate::{Gregorian, Hijri};

/// One row of the fixture.
struct Row {
    kind: String,
    year: i32,
    month: u8,
    day: u8,
    isoformat: String,
    dmyformat: String,
    month_name: String,
    day_name: String,
    year_length: Option<u16>,
    month_length: Option<u8>,
    isoweekday: u8,
    notation: String,
}

fn rows() -> Vec<Row> {
    let mut out = Vec::new();
    for line in include_str!("fixtures/hijridate_api.tsv").lines().skip(1) {
        if line.is_empty() {
            continue;
        }
        let f: Vec<&str> = line.split('\t').collect();
        out.push(Row {
            kind: f[0].to_string(),
            year: f[1].parse().unwrap(),
            month: f[2].parse().unwrap(),
            day: f[3].parse().unwrap(),
            isoformat: f[4].to_string(),
            dmyformat: f[5].to_string(),
            month_name: f[6].to_string(),
            day_name: f[7].to_string(),
            year_length: f[8].parse().ok(),
            month_length: f[9].parse().ok(),
            isoweekday: f[10].parse().unwrap(),
            notation: f[11].to_string(),
        });
    }
    out
}

#[test]
fn every_formatting_row_agrees_with_hijridate() {
    let rows = rows();
    assert!(rows.len() > 140, "only {} rows", rows.len());

    let mut disagreements = Vec::new();
    for row in &rows {
        // The suffix says which call produced the row: `n` is `dmyformat("-")`,
        // `p` is `dmyformat` with padding off, `i` is a `fromisoformat` round
        // trip. The prefix says which type.
        let hijri = row.kind.starts_with('H');
        let (
            isoformat,
            dmyformat,
            month_name,
            day_name,
            isoweekday,
            notation,
            year_length,
            month_length,
        ) = if hijri {
            let date = Hijri::new(row.year, row.month, row.day).unwrap();
            let dmyformat = match row.kind.as_str() {
                "Hn" => date.dmyformat("-", true),
                "Hp" => date.dmyformat("/", false),
                "Hi" => Hijri::fromisoformat(&row.isoformat)
                    .unwrap()
                    .dmyformat("/", true),
                _ => date.dmyformat("/", true),
            };
            let isoformat = match row.kind.as_str() {
                "Hi" => Hijri::fromisoformat(&row.isoformat).unwrap().isoformat(),
                _ => date.isoformat(),
            };
            let parsed = match row.kind.as_str() {
                "Hi" => Hijri::fromisoformat(&row.isoformat).unwrap(),
                _ => date,
            };
            (
                isoformat,
                dmyformat,
                parsed.month_name().to_string(),
                parsed.day_name().to_string(),
                parsed.isoweekday(),
                parsed.notation().to_string(),
                Some(parsed.year_length()),
                Some(parsed.month_length()),
            )
        } else {
            let date = Gregorian::new(row.year, row.month, row.day).unwrap();
            let dmyformat = match row.kind.as_str() {
                "Gn" => date.dmyformat("-", true),
                "Gp" => date.dmyformat("/", false),
                "Gi" => Gregorian::fromisoformat(&row.isoformat)
                    .unwrap()
                    .dmyformat("/", true),
                _ => date.dmyformat("/", true),
            };
            let isoformat = match row.kind.as_str() {
                "Gi" => Gregorian::fromisoformat(&row.isoformat)
                    .unwrap()
                    .isoformat(),
                _ => date.isoformat(),
            };
            let parsed = match row.kind.as_str() {
                "Gi" => Gregorian::fromisoformat(&row.isoformat).unwrap(),
                _ => date,
            };
            (
                isoformat,
                dmyformat,
                parsed.month_name().to_string(),
                parsed.day_name().to_string(),
                parsed.isoweekday(),
                parsed.notation().to_string(),
                row.year_length.map(|_| parsed.year_length()),
                row.month_length.map(|_| parsed.month_length()),
            )
        };

        let mut check = |what: &str, got: String, want: String| {
            if got != want {
                disagreements.push(format!(
                    "{} {}-{}-{} {what}: {got:?} != {want:?}",
                    row.kind, row.year, row.month, row.day
                ));
            }
        };
        check("isoformat", isoformat, row.isoformat.clone());
        check("dmyformat", dmyformat, row.dmyformat.clone());
        check("month_name", month_name, row.month_name.clone());
        check("day_name", day_name, row.day_name.clone());
        check("notation", notation, row.notation.clone());
        if let Some(want) = row.year_length {
            check(
                "year_length",
                year_length.unwrap().to_string(),
                want.to_string(),
            );
        }
        if let Some(want) = row.month_length {
            check(
                "month_length",
                month_length.unwrap().to_string(),
                want.to_string(),
            );
        }
        if isoweekday != row.isoweekday {
            disagreements.push(format!(
                "{} {}-{}-{} isoweekday: {isoweekday} != {}",
                row.kind, row.year, row.month, row.day, row.isoweekday
            ));
        }
    }

    assert!(
        disagreements.is_empty(),
        "{} rows disagree with hijridate: {:?}",
        disagreements.len(),
        &disagreements[..disagreements.len().min(20)]
    );
}

#[test]
fn datetuple_is_the_year_month_day() {
    let h = Hijri::new(1447, 9, 1).unwrap();
    assert_eq!(h.datetuple(), (1447, 9, 1));
    let g = Gregorian::new(2026, 2, 18).unwrap();
    assert_eq!(g.datetuple(), (2026, 2, 18));
}

#[test]
fn a_malformed_iso_date_is_reported() {
    assert!(Hijri::fromisoformat("1447-09").is_err());
    assert!(Hijri::fromisoformat("1447-09-01-01").is_err());
    assert!(Hijri::fromisoformat("not a date").is_err());
    assert!(Gregorian::fromisoformat("2026/02/18").is_err());
}

#[test]
fn the_weekday_conventions_differ_by_one() {
    // Both are common, and mixing them silently is the bug the two names exist
    // to make visible: `weekday` is 0-based from Monday, `isoweekday` 1-based.
    let h = Hijri::new(1447, 9, 1).unwrap();
    assert_eq!(h.weekday(), 2);
    assert_eq!(h.isoweekday(), 3);
    assert_eq!(h.day_name(), "Wednesday");
}

#[test]
fn today_round_trips_through_both_calendars() {
    let today = Gregorian::today().unwrap();
    let hijri = Hijri::today().unwrap();
    assert_eq!(today.to_hijri(), hijri);
}
