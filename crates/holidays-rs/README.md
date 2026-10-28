# holidays-rs

[![crates.io](https://img.shields.io/crates/v/holidays-rs.svg)](https://crates.io/crates/holidays-rs)
[![docs.rs](https://docs.rs/holidays-rs/badge.svg)](https://docs.rs/holidays-rs)

Public holiday calendars and working-day arithmetic.

```rust
use holidays_rs::{Calendar, Date};

let cal = Calendar::indonesia();
let d = Date::new(2026, 8, 17).unwrap();
assert!(cal.is_holiday(d));
assert_eq!(cal.next_working_day(d), Date::new(2026, 8, 18).unwrap());
```

Holidays are rules, not tables, so a calendar stays correct for any year.
Country-specific weekend substitution is `SubstituteStyle`, applied per country
when a year is resolved: the United States moves a Saturday holiday back to
Friday and the United Kingdom moves a weekend holiday forward to Monday.

Covered: Indonesia, United States, United Kingdom, Germany, France,
Netherlands, Spain, Italy, Portugal, Belgium, Austria, Poland, Sweden, Norway,
Denmark, Finland, Ireland, Canada, New Zealand, Australia, Czechia, Slovakia,
Croatia, Slovenia, Estonia, Lithuania, Luxembourg, Iceland, Cyprus, Malta,
Greece, Romania, Mexico, Latvia, Brazil, El Salvador, Nicaragua, Peru, Uruguay,
Andorra, Cabo Verde, the Faroe Islands, Greenland, Switzerland (the national
set only), French Guiana, Saint Barthelemy, Saint Pierre and Miquelon, the
French Southern Territories, Reunion, Mayotte, Liberia, the Republic of the
Congo, the Pitcairn Islands, Tokelau and Samoa. `Country::ALL` lists them.

Greece, Cyprus and Romania keep the Orthodox calendar, so their movable feasts
are computed from Orthodox Easter with `Rule::OrthodoxEasterOffset` rather than
from the Western one; in 2024 the two Easters were five weeks apart, so sharing
a single rule would have been wrong for most years.

The crate answers four questions:

1. Which dates are public holidays in a given country and year?
2. Is a particular date a working day?
3. What is the *n*-th working day from a given date?
4. What is the nearest holiday before or after a given date?

`closest_holiday(date, Direction::Forward)` returns the first holiday strictly
after `date` and `Direction::Backward` the last one strictly before it, each as a
date and a name. A holiday that falls on `date` itself is not returned, matching
the reference's half-open search:

```rust
use holidays_rs::{Calendar, Date, Direction};

let cal = Calendar::united_states();
let d = Date::new(2026, 2, 18).unwrap();
let (date, name) = cal.closest_holiday(d, Direction::Forward).unwrap();
assert_eq!(name, "Memorial Day");
assert_eq!(date, Date::new(2026, 5, 25).unwrap());
```

## Scope

Calendars cover *national* holidays. A holiday is not always in force for every
year, so `Rule` can bound it: `Rule::Since` and `Rule::Until` for a holiday
introduced or abolished in a given year (Poland's Epiphany from 2011, Sweden's
Whit Monday until 2004), `Rule::Between` for a window (Italy's Republic Day,
movable from 1977 to 2000), `Rule::ExceptYears` for years that are skipped
(France's Whit Monday as a solidarity day 2005-2007), `Rule::Once` for a one-off
(New Zealand's 2022 Queen Elizabeth II Memorial Day), `Rule::EveryNYears` for a
recurring interval (the Netherlands' Liberation Day every five years),
`Rule::EveryNYearsSince` for an interval anchored at a year rather than at a
multiple (Mexico's sexennial transfer of federal power, which moved from 1
December to 1 October in 2024), `Rule::NeverSubstituted` for a holiday the
country does not move even though it moves its other ones (Latvia's New Year's
Day), `Rule::Absorbed` for a date that stops being a holiday at all on a given
weekday (Mexico's New Year's Day and Christmas, which are dropped when they
fall on a Sunday),
`Rule::ObservedFrom` for a holiday that gained a substitute day late (New
Zealand's Mondayised Waitangi Day and Anzac Day from 2014),
`Rule::ObservedOffWeekendAndOrthodoxEasterMonday` for a date moved off a weekend
and one day further when the landing day is already Orthodox Easter Monday
(Greece's Labour Day, which moved twice in 2021) and `Rule::Table` for
dates that are published rather than computed (Matariki). Regional observances
are out of scope; add them with `Calendar::with_extra` when needed.

`holidays_in` reports the date a holiday is *observed*, not the date it falls.
A bank holiday that lands at a weekend therefore appears on its substitute date
and not on the raw one, which is the date a payroll or a calendar grid needs.

## Verification

Every country table is checked against the Python `holidays` package over
2000-2050 by `research/tools/verify_holidays.py`, which exits non-zero on any
difference that is not listed as deliberate. The deliberate differences are
Indonesia (Islamic and Chinese observances are set by annual decree and are
available through `Calendar::with_extra`), Spain (a national holiday that falls
on a Sunday is kept rather than dropped), Ireland (Saint Stephen's Day is
observed on the substitute Monday), Canada (the federal-wide set is used) and
Latvia (the extra rest days the cabinet grants by annual decision, which
`Calendar::with_extra` takes).
Run it from the repository root:

```sh
python3 research/tools/verify_holidays.py
```

`closest_holiday` is checked separately against the same package by
`research/tools/gen_holidays_closest.py` and the `closest` test: 17,200 rows over
43 countries whose holiday set already matches the reference, three probes a
month for eleven years plus the year boundaries where the answer crosses into
the next year. The countries it skips are the ones `verify_holidays.py` already
marks as deliberate differences - a nearest-holiday query over a different set
answers a different question - and the script records why each is skipped.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
