# Review of every crate against its mature comparison package (2026-11-16)

This is the follow-up to `review-0.1.16.md`. That round finished an ISO 20022
message pair and added the gate case that keeps it correct. This round returns
to `holidays-rs`, whose own `PLAN.md` section names its remaining gap in one
sentence: the reference carries around 250 calendars and the crate covers 101, so
the remaining countries are "the obvious next step" and every one can be checked
against the reference that is already installed.

## What this round develops

### `holidays-rs` 0.6.0 -> 0.7.0, eleven more calendars

`holidays-rs` states its holidays as rules rather than tables, so the cost of a
new country is not a table of dates but a fit: every holiday of the reference
country has to be expressible in the crate's `Rule` vocabulary, and the fit has
to hold outside the window it was fitted on. That is exactly what
`research/tools/derive_holidays.py` does, and this round ran it as a **filter**
rather than a generator: it was pointed at every reference country the crate did
not yet cover, and only the ones it accepted were added.

That filter is the point of the round. Of the missing countries, eleven fit and
were added:

| Country | Code | Dates | Substitution |
| --- | --- | --- | --- |
| Antigua and Barbuda | AG | 560 | none |
| Cook Islands | CK | 672 | none |
| Isle of Man | IM | 515 | none |
| Saint Kitts and Nevis | KN | 617 | none |
| Cayman Islands | KY | 534 | none |
| Saint Helena | SH | 185 | none |
| San Marino | SM | 910 | none |
| Turks and Caicos Islands | TC | 623 | none |
| Vatican City | VA | 1,468 | none |
| Saint Vincent and the Grenadines | VC | 638 | none |
| South Africa | ZA | 627 | none |

The rest of the missing set was refused, and the refusals are the more
informative half:

* **Cuba, Madagascar, Eswatini, Vanuatu, Bolivia, the Bahamas, Laos and
  Dominica** each carry a holiday the `Rule` set cannot express - a one-off, a
  holiday whose date is set by decree, or a shape no combination of the existing
  rules reproduces. They are left out rather than approximated, because a table
  that only reproduces the fitted years is a lookup, not a rule, and
  `derive_holidays.py` already refuses to emit one.
* **Bouvet Island, Heard Island and McDonald Islands, and the British Indian
  Ocean Territory** fit perfectly and were still left out: they are
  uninhabited, so the reference carries zero holidays for them and there is
  nothing to express.
* **Venezuela** fits the dates but not the shape - its substitution is
  conditional in a way the crate's per-country style cannot state - so it is
  refused rather than mis-modelled.

Every one of the eleven is checked two ways. `research/tools/verify_holidays.py`
compares the whole table against the reference over 2000-2050 and reports
`0 undocumented defect(s) out of 112 calendars`; `closest_holiday` is checked
separately by `research/tools/gen_holidays_closest.py`, which grew the two
countries whose holiday set matches the reference exactly (San Marino and
Vatican City, the only two of the eleven that move no holiday) to 67 compared
countries and 26,800 rows.

## The trap this surfaced: the reference moves under a crate that has not

Running the gate for the first time in this round produced four failures -
Martinique, Jamaica, Namibia and Hungary - none of which this round had touched.
The instinct is to fix the crate. The right move was to check the reference's
version: the gate's docstring named `holidays` 0.107 and the environment had
0.109, and against a clean checkout (the change stashed) the same four failed.
The reference had moved its own holiday data between releases; the crate had
not.

The version that reconciles the whole workspace is **0.108**: against 0.109 four
countries diverge and against 0.107 two (Botswana and Namibia), while 0.108 is
clean over all 112. The gate, the fixture generator and `research/tools/README.md`
now pin `holidays==0.108` and say why, and `AGENTS.md` records the trap. This is
the same class of mistake as an unpinned toolchain: a differential gate against
a moving reference silently reports the reference's changes as the crate's
defects, and the fix is to read the failure as "the reference moved" and diff the
two releases before touching a table.

## Documentation

* `crates/holidays-rs/README.md` - the "Covered" list gains the eleven, and the
  closest-holiday paragraph moves from 65 to 67 compared countries.
* `PLAN.md` - the `holidays-rs` "Covers today" count moves from 101 to 112.
* `research/tools/verify_holidays.py` and `research/tools/README.md` pin
  `holidays==0.108` and explain the drift.
* `AGENTS.md` gains the "pin the reference, or its release becomes your defect"
  trap.

## Method

The crate was chosen because its own `PLAN.md` named the gap and because the
oracle already existed - the same `holidays` package that supplies the
expectations for the existing 101 calendars supplies them for the new eleven.
The round is a filter rather than a hand-port: `derive_holidays.py` decides what
fits, and the eleven it accepted are mechanical, which is what makes the
refusals meaningful - the crate does not quietly approximate a country whose
holidays are not rules.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_holidays.py` | PyPI `holidays` 0.108 | 0 undocumented defects across 112 calendars |
| `gen_holidays_closest.py` | PyPI `holidays` 0.108 | 26,800 rows over 67 compared countries |
| `cargo test -p holidays-rs` | the crate's own suite | 111 unit tests, 1 closest test, 1 README test and 4 doctests pass |

## Follow-up: the gate was only looking where the fit had looked

A review of the eleven calendars above asked the obvious question the round had
not: if the tables were fitted over 2000-2050, what do they answer *before*
2000? `verify_holidays.py` compared over the same 2000-2050 window, so it could
only ever confirm the years the fitter had already reproduced. Widening the gate
to the reference's own start year turned eleven passing calendars into eleven
questionable ones, and the causes were all in the generator, not the tables.

**A fixed fit window back-projects every introduction year.** Fitting from 2000
makes a holiday introduced in 1995 look as though it had always existed, and the
fit absorbs the mismatch by moving the base. The Isle of Man answered Tynwald
Day on 6 July, because the fitter hid the real 5 July base behind an
`ObservedShift`; South Africa answered Human Rights Day for 1990, five years
before the holiday existed. `derive_holidays.py` now starts the window at the
reference's `start_year` and gates each run there, so a query before that year
returns nothing, as the reference does.

**A large shift table is a wrong base in disguise.** South Africa's Y2K
Changeover is 31 December 1999 and 2 January 2000; the fitter accepted 31
December with a 363-day shift for the 2000 weekday rather than split the two
dates. `fit_shift` now rejects any shift beyond a week, and each observed date is
attributed to its nearest base once per name, so a substitute near a year
boundary is not handed to the wrong year.

**A run that starts at the fit's first year is not open at the bottom.** With a
per-country first year, a run starting there rendered as `Until`, which fires
from the beginning of time. `rust_rule` and `rule_applies` now gate it at that
year unless it is the floor, and the two keep mirroring each other.

The eleven tables were regenerated; the diff touches only those eleven statics.
Every one now passes the historical gate (`0 undocumented defect(s) out of 11`)
and the generator's own held-out check, and the modern window stays clean
(`0 undocumented defect(s) out of 112`). The remaining historical failures are
in the 101 pre-existing calendars, which this round did not regenerate: the same
generator fix applies to them, but regenerating a calendar that carries
hand-curated rules (New Zealand's transcribed Matariki table, Ireland's
one-off Oireachtas days) would delete them, and the repository's one-country
per pull request rule puts that work in its own round rather than this one.
`verify_holidays.py` now defaults to the historical window, so the next round
starts from a gate that sees the defect.

