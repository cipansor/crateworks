# Review of every crate against its mature comparison package (2026-11-16)

This is the follow-up to `review-0.1.18.md`. That round added eleven calendars
to `holidays-rs` by pointing `derive_holidays.py` at every reference country the
crate did not cover and keeping the ones it accepted, then fixing the generator
so a fit was checked from the reference's own start year rather than a fixed
2000. This round runs the same filter again over the countries the reference
still has and the crate does not, now that the generator anchors a fit
correctly, and keeps the six the improved fit accepts.

## What this round develops

### `holidays-rs` 0.7.0 -> 0.8.0, six more calendars

The filter is unchanged in shape: `derive_holidays.py` reads every holiday the
reference reports for a country from that country's own start year through 2050,
fits each to the crate's `Rule` vocabulary, re-resolves the fitted table over a
held-out window (2051-2075), and refuses a table that disagrees with the
reference there or that stops producing a holiday the reference reports every
year. Six of the countries the crate did not cover now pass, and they are added:

| Country | Code | Dates | Substitution |
| --- | --- | --- | --- |
| Bermuda | BM | 1,035 | none |
| Dominica | DM | 736 | none |
| Grenada | GD | 690 | none |
| Gibraltar | GI | 606 | none |
| Solomon Islands | SB | 650 | none |
| British Virgin Islands | VG | 1,179 | none |

Each is a rule set, not a table of dates. The shapes are the ones the crate
already had - a fixed date under a `Since`, a movable feast under
`EasterOffset`, an *n*-th weekday, and a per-holiday `ObservedShift` for the
weekend move - so the round needed no new `Rule` variant. The interesting part
is that the crate's `ObservedShift` carries a shift table keyed on the weekday
the base lands on, and three of the six exercise a shift the earlier rounds did
not: the Solomon Islands move a weekend holiday *backward* to the preceding
Friday (`shifts: &[(6, -1)]`), Gibraltar moves a Saturday holiday two days
forward past a Sunday (`shifts: &[(0, 1), (6, 2)]`) and Bermuda's Christmas and
Boxing Day share that two-day move so the pair never collides.

The reference's own quirks survive the fit rather than being smoothed away.
Bermuda Day was fixed on 24 May through 2017 and has been the Friday before the
last Monday of May since, with a handful of years the reference reports as
one-offs where the rule and the recorded date disagree - the table carries both
the rule and those `Once` entries, because the gate is the reference and not a
tidier calendar. Grenada's Emancipation Day was the first Monday of August from
2000, with four years the reference reports on 1 August regardless, then a fixed
1 August from 2025; the table keeps the alternation. That is the difference
between a fit and a hand-port: the generator emits what reconciles the
reference, including the parts a person would have "cleaned up" and thereby
broken.

### The rest of the missing set, and why it is refused

The refusals are the more informative half, because they are what keeps a fit a
rule rather than a lookup:

* **Madagascar (MG)** fits the window it was fitted on but *loses* a holiday -
  `Fetin'ny reny` (Mother's Day) is reported in the fitted years and vanishes in
  the held-out window, so no single rule reproduces it and the generator refuses
  the table. A calendar that only answers the years it was fitted on is a
  lookup.
* **Dominican Republic (DO)**, **Zambia (ZM)**, **Kiribati (KI)** and others
  over-fit the same way: the held-out window has dates the fitted window does
  not, so the fit does not generalise.
* **Eswatini (SZ)**, **Cuba (CU)**, **Tuvalu (TV)**, **Zimbabwe (ZW)**,
  **Vanuatu (VU)**, **Montserrat (MS)** and more carry a holiday whose date is
  set by decree or whose shape no combination of the existing rules reproduces.
  They are left out rather than approximated.
* **Bouvet Island (BV)**, **Heard Island and McDonald Islands (HM)** and the
  **British Indian Ocean Territory (IO)** fit perfectly and are still left out:
  they are uninhabited, so the reference carries zero holidays and there is
  nothing to express.

### Verification

Every one of the six is checked the same two ways as the rest of the crate.

`research/tools/verify_holidays.py` compares each whole table against the
reference over the historical window, from the country's own start year, and
reports `ok` for all six:

```
ok  Bermuda
ok  Dominica
ok  Grenada
ok  Gibraltar
ok  SolomonIslands
ok  BritishVirginIslands

0 undocumented defect(s) out of 6 calendars
```

Run over the whole crate the same gate reports `63 undocumented defect(s) out of
118 calendars`. The six new calendars add no defect; the 63 are the pre-existing
ones `review-0.1.18.md` left for their own round - a calendar that carries
hand-curated rules (New Zealand's transcribed Matariki table, Ireland's one-off
Oireachtas days) cannot be regenerated without deleting them, so the same
generator fix still has to be applied to those calendars one at a time.

`closest_holiday` is checked separately by `research/tools/gen_holidays_closest.py`.
None of the six joins the compared set: all six move at least one holiday, so the
reference lists the raw date and the observed one and the reference's set is
larger than the crate's - a nearest-holiday query over that set answers a
different question - so each is added to the script's `EXCLUDED` map with that
reason, exactly as the nine moved calendars from 0.7.0 were. The regenerated
fixture is byte-identical: 26,800 rows over 67 compared countries.

## Documentation

* `crates/holidays-rs/README.md` - the "Covered" list gains the six.
* `PLAN.md` - the `holidays-rs` "Covers today" count moves from 112 to 118.
* `research/tools/verify_holidays.py` and
  `research/tools/gen_holidays_closest.py` gain the six, with the closest script
  recording why each is excluded from the comparison.
* `crates/holidays-rs/src/countries.rs` gains six unit tests, one per country,
  each asserting a date the reference reports - Bermuda Day's move off 24 May,
  Dominica's Carnival Monday and Tuesday, Grenada's Thanksgiving move off a
  Sunday, Gibraltar's National Day move, the Solomon Islands King's Birthday,
  and the British Virgin Islands' three Emancipation days.

## Method

The target was chosen because the gap was already measured and the oracle
already installed: `derive_holidays.py` is the filter, the reference `holidays`
package is the expectation, and the round is "run the filter again and keep what
it accepts". That is what makes the six mechanical and the refusals meaningful -
the crate does not quietly approximate a country whose holidays are not rules,
and the six it accepts are the ones that are.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_holidays.py` | PyPI `holidays` 0.108 | 0 undocumented defects across the 6 new calendars; 63 pre-existing remain |
| `gen_holidays_closest.py` | PyPI `holidays` 0.108 | 26,800 rows over 67 compared countries, unchanged |
| `cargo test -p holidays-rs` | the crate's own suite | 117 unit tests, 1 closest test, 1 README test and 4 doctests pass |

## Follow-up

The remaining work is the same shape again: the 63 pre-existing historical
defects need the improved generator applied one calendar at a time, without
losing the hand-curated rules those calendars carry, and the reference's other
uncovered countries need either a new `Rule` variant or a decision to leave them
out. Both are their own rounds; this one is the mechanical continuation of the
filter the last round established.
