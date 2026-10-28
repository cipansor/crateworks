# hijridate

[![crates.io](https://img.shields.io/crates/v/hijridate.svg)](https://crates.io/crates/hijridate)
[![docs.rs](https://docs.rs/hijridate/badge.svg)](https://docs.rs/hijridate)

Hijri (Islamic) and Gregorian date conversion, based on the Umm al-Qura
calendar. No dependencies.

```rust
use hijridate::{Gregorian, Hijri};

// 1 Ramadan 1445 fell on 11 March 2024.
let h = Hijri::new(1445, 9, 1).unwrap();
assert_eq!(h.to_gregorian().to_string(), "2024-03-11");
assert_eq!(h.month_name(), "Ramadan");

let g = Gregorian::new(2024, 3, 11).unwrap();
assert_eq!(g.to_hijri().to_string(), "1445-09-01");
```

The Hijri year is about eleven days shorter than the Gregorian year and a month
begins at the crescent sighting, so the calendar cannot be computed from a
formula. Umm al-Qura is the Saudi sighting-based calendar that most software
treats as the reference. This crate carries its published month starts for Hijri
years 1343 to 1500 (Gregorian 1924 to 2077), so conversions in that range are
exact, and `Hijri::month_length` returns the real 29 or 30 day month rather than
a guess.

Outside that range the crate falls back to the classical arithmetic calendar and
sets `Hijri::is_tabular`, which can be a day or two off. Treat those dates as a
hint, not a fact.

## Formatting and naming

The date types carry the accessors the reference does, so a value can be turned
into text without pulling in a formatting crate:

```rust
use hijridate::{Gregorian, Hijri};

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
```

`weekday` counts from 0 for Monday and `isoweekday` from 1, which is the two
conventions the reference uses and a silent bug when they are mixed, so both are
named. `fromisoformat` parses `YYYY-MM-DD` back, and `today` reads the system
clock.

A `Gregorian` date also has the ordinal the reference inherits from
`datetime.date`, the day count from 1 January of year 1, and the inverse and a
`replace`:

```rust
use hijridate::Gregorian;

let g = Gregorian::new(2026, 2, 18).unwrap();
assert_eq!(g.toordinal(), 739_665);
assert_eq!(Gregorian::fromordinal(739_665).unwrap(), g);
assert_eq!(g.replace(None, None, Some(1)).unwrap().isoformat(), "2026-02-01");
assert_eq!(g.replace(Some(2024), Some(2), Some(29)).unwrap().isoformat(), "2024-02-29");
```

`replace` takes an `Option` per component and keeps the current value where the
option is `None`; it reports an error for a date that does not exist, such as
29 February in a common year. The ordinal range is 1 (1 January of year 1)
through 3,652,059 (31 December 9999), the same as the reference.

The month and day names are available in the four languages the reference
publishes - English, Arabic, Bengali and Turkish - through `Language`.
`month_name`, `day_name` and the era constants answer in English; the `_in`
forms take a language, and `Language::from_tag` accepts a tag such as `"ar"` or
`"ar-SA"` so a locale setting can select one.

```rust
use hijridate::{Language, Hijri};

let h = Hijri::new(1445, 9, 1).unwrap();
assert_eq!(h.month_name_in(Language::Arabic), "رمضان");
assert_eq!(Language::from_tag("ar-SA"), Some(Language::Arabic));
```

## Regional differences

Umm al-Qura is one authority among several. Indonesia (Kemenag), Malaysia and
many communities announce their own start of the month, so a date from this crate
can differ by a day from what a country actually observes. That is not a bug to
fix, it is a fact about the calendar; expect to override the date when a local
authority publishes its own. Use `Hijri::month_name` and the table to get the
reference date, and expect to override it when a local authority announces
something different.

## Data provenance

The month-start table and the test fixtures are generated from the
[`hijridate`](https://github.com/dralshehri/hijridate) Python package by Mohammed
Alshehri, which is MIT licensed and reproduces the Umm al-Qura calendar as
published by the Saudi authorities. See [`NOTICE`](https://github.com/cipansor/crateworks/blob/main/crates/hijridate/NOTICE).

`tests/fixtures/hijridate_vectors.tsv` holds 7,872 rows produced by that
package: both directions of the conversion for every month in the tabulated
range, the month lengths, whether each day exists, and the weekday. The
`reference` test replays it. The conversions, the month lengths and the
tabulated-versus-arithmetic boundary all agree row for row, so the test is a
regression guard rather than a bug hunt.

`tests/fixtures/hijridate_api.tsv` holds 145 further rows for the formatting and
naming API - `isoformat`, `dmyformat`, `month_name`, `day_name`, `year_length`,
`month_length`, `isoweekday` and `notation` - over every month of seven years
either end of the table, all seven weekdays, both Gregorian year lengths, and the
`fromisoformat` round trip. That is the half where a plausible implementation
drifts quietly: a month name with a different apostrophe, a `dmyformat` that pads
the year, an `isoweekday` off by one. `tests/fixtures/hijridate_ordinal.tsv`
holds 44 rows for the Gregorian ordinal, its inverse and `replace`, across year
1, the Gregorian reform years, leap and common years, and year 9999. Both
fixtures come from the same package, regenerated by
`research/tools/gen_hijri_table.py` and
`research/tools/gen_hijridate_api.py`.

## Scope

The table covers Hijri years 1343 to 1500 (Gregorian 1924-08-01 to
2077-11-16). Outside that range conversion falls back to a tabular
arithmetic approximation, which can be a day or two out; see
`Hijri::is_tabular` for how to tell.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
