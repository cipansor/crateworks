# Review of every crate against its mature comparison package (2026-11-16)

This is the follow-up to `review-0.1.14.md`. That round re-ran every gate, closed
the `object-hash` SHA-3 family and the remaining Node `crypto` digests, added the
`iso20022-rs` `pacs.002` status report, and recorded the remaining candidates
with the reason each stays out. It named one next step above the rest: more
holiday calendars.

This round took that step, and the step turned out to be a tooling bug rather
than a data limit. `review-0.1.14.md` recorded that "the countries that fit have
all been added" and that every remaining country fails for the same reason - the
reference shifts each holiday independently while the crate held one
`SubstituteStyle` per country. The second half of that is true of the calendars
that still fail. The first half was wrong: `derive_holidays.py`, the tool that
decides whether a country fits, was dropping data, and the countries it called
unfittable included thirteen that fit exactly. The same defect had already forced
a workaround in a country the previous round shipped: the old tool could not
produce Botswana's four one-off `Public Holiday` entries, so they were written
into `countries.rs` by hand. Both are fixed, and the crate now carries 102
calendars rather than 86.

The work was reconciled with upstream before it landed. While this branch was in
flight, `cipansor/main` released `holidays-rs` 0.5.0 with a smaller, different
set: six calendars (Aruba, the Åland Islands, the Caribbean Netherlands, Curaçao,
Namibia and Sint Maarten) that its own generator reached through a separate
inline-observed fallback. A published version number cannot be reused, so this
branch rebases on that release and ships the union as **0.6.0**. The generator
here subsumes the upstream fallback - it fits all six of those countries too -
so the branch keeps one generator and one table set rather than two. The final
count is 102: the 86 at the merge base, upstream's 6, and the 10 this branch adds
beyond them.

## Correctness first: the differential gates were re-run

Every gate was re-run before any change. All passed against the data they were
cut from; `verify_aviation.py` reads OurAirports' live CSV and now reports
upstream rows that were added, renamed or re-coordinated since, which is data
freshness rather than a crate defect - its row and lookup counts are unchanged.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_holidays.py` | PyPI `holidays` | 0 undocumented defects (86 calendars, the count before this round) |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` | 85,713 vectors agree |
| `verify_spreadsheet.py` | npm `@formulajs/formulajs` | 1,068/1,068 cases agree |
| `verify_aviation.py` | OurAirports `airports.csv` (live) | crate fixture: 11,887 rows and 46,456 lookups |
| `verify_arabic.py` | PyPI `arabic-reshaper` 3.0.0 | 6,558/6,558 agree (per language) |
| `verify_hijri_locales.py` | PyPI `hijridate` 2.6.0 | 132/132 names agree |
| `verify_fuzzy_score.py` | PyPI `fuzzywuzzy` 0.18.0 | 0 mismatches, 5 process functions |
| `verify_units.py` | PyPI `pint` | 973 compared, 0 mismatches |
| `verify_mllp.py` | PyPI `hl7apy` | 135/135 fields, 8/8 ACK, 8/8 enhanced ACK replies agree |
| `verify_openlocationcode.py` | PyPI `openlocationcode` 1.0.1 | 1,389 rows agree |
| `verify_iso20022.py` | the official ISO 20022 XSD | generated pacs.008, pain.001 and pacs.002 validate |
| `verify_proxy_addr.py` | npm `proxy-addr` 2.0.8 | 12,216 rows agree |
| `tests/oracle.rs` (fast-redact) | npm `fast-redact` 3.5.0 | 6,000 redactions and 90 path verdicts agree |
| `tests/{fuzz,oracle}.rs` (object-hash) | npm `object-hash` 3.0.0 | 4,000 random values and 66 curated cases agree, all sixteen digests |
| `tests/closest.rs` (holidays-rs) | PyPI `holidays` | 26,400 rows agree over 66 calendars |

No gate regressed.

## What this round develops

### `holidays-rs` 0.4.1 -> 0.6.0, sixteen more countries, reconciled with upstream's 0.5.0

The crate's own README and `PLAN.md` named the remaining countries as the
obvious next step and said the additions were mechanical because
`derive_holidays.py` refuses to emit a table that disagrees with the reference.
That is the right design. The tool had a defect that made the guarantee false in
one direction: it silently dropped data, so a calendar it could have expressed
was reported as unexpressible instead.

**The defect.** `split_observed` grouped a holiday's dates by name into a
`{year: date}` map. A name that occurs more than once in a year - Hungary's
`Nemzeti ünnep` is on 15 March and 23 October - kept only the last date, so the
fit was asked to reproduce a calendar with a holiday missing. The fit then
failed, or worse, succeeded against the wrong data. Grouping into
`{year: [dates]}` fixes it, and `name_entries` splits a multi-date name into as
many crate rules as it needs. The same defect was already visible in a country
the previous round shipped: the old generator answers `unexpressible: Public
Holiday` for Botswana, whose reference uses that name for four one-off dates
(two of them in 2024), and the previous round wrote those four entries into
`countries.rs` by hand rather than fixing the tool that could not produce them.

**The second change is a check the tool did not have.** A fit over 2000-2050
cannot tell a rule from a table: a lookup that reproduces exactly those fifty-one
years passes. The crate's first sentence is "Holidays are rules, not tables, so
a calendar stays correct for any year", so a fit that only reproduces the fitted
window is a defect that the fit check is blind to by construction.
`derive_holidays.py` now re-resolves each candidate over a held-out window
(2051-2075) and refuses the table if it disagrees there. This is not
hypothetical: Antarctica's Midwinter Day falls on 20 June in leap years and 21
June otherwise, which no `Rule` variant expresses. The fitter can only
approximate it with a stack of per-year gates (`Once` for each leap year,
`Between` runs for the rest) that reproduce 2000-2050 exactly and nothing after;
the held-out check rejects it, and the country is not added. Before this change
that same stack would have been emitted as a calendar.

The two fixes together turn the "remaining countries" question from a judgement
call into a measurement, and the measurement added thirteen: Armenia, Aruba,
Barbados, Belarus, Curacao, Georgia, Guatemala, Honduras, Hungary, Moldova,
Madagascar, Namibia and Seychelles. Three of those (Aruba, Curacao, Namibia)
were released upstream as 0.5.0 in parallel, so the reconciled crate goes from 86
calendars to 102. Belarus, Georgia and Moldova bring Orthodox Easter into three
more countries; Hungary is the country whose two-date name exposed the defect.

A third, smaller change makes the output readable. A shape that matches a
holiday's dates with no shift is now preferred over one that needs a shift
table. Madagascar's Father's Day is the third Sunday of June; the old ordering
fitted it as a fixed 21 June plus a six-entry weekday shift table - correct, and
a puzzle to read. It is now `Rule::NthWeekday { month: 6, weekday: 0, nth: 3 }`.

**How it is trusted.** `verify_holidays.py` is the gate, and it now covers 102
calendars against the reference over 2000-2050 with 0 undocumented defects. The
closest-holiday fixture is regenerated from the same reference and grows from
21,200 rows over 53 calendars to 26,400 rows over 66; Barbados, Namibia and
Seychelles are excluded for the same reason the other shifting calendars are
(their reference set carries the raw date and the observed one, so a
nearest-holiday query over it answers a different question). The held-out check
inside `derive_holidays.py` is a second gate on the same question the crate's
README states, and it is what keeps a lookup from being shipped as a rule.

### `holidays-rs`, a test for the country list

`Country::ALL` is hand-maintained and every test that walks the calendars walks
it. The enum's match arms are exhaustive, so a variant added without a line in
`ALL` compiles, and every `Country::ALL` test then skips it silently. The new
`all_lists_every_country_exactly_once` test pins the count at 102 - so adding a
country forces the test to be updated - and rejects a repeated country.

## The candidates the diff surfaced

**`holidays-rs`: the countries that still fail.** With the generator fixed, the
remaining reference calendars fail the held-out check or the fit for a real
reason, and the reason is now recorded per country rather than asserted. The
classes:

* A reference that carries non-formulaic days the crate documents as out of
  scope - Indonesia's ministerial-decree observances are the type case.
* A reference that shifts each holiday independently in a way a per-holiday
  shift table cannot express within `MAX_GROUPS`.
* A reference whose data is an over-fit (Antarctica's leap-year Midwinter Day)
  or whose fitted window is too narrow to be a rule.

Those are the same walls `review-0.1.14.md` named, now with the tool able to tell
them apart from a defect in the tool itself.

**`stdnum-rs`: the three missing modules.** Unchanged from `review-0.1.14.md`.
`Cfi`, `Imsi` and `Vatin` are blocked by tables that ship under LGPL-2.1+ in
`python-stdnum` and cannot be copied into an Apache-2.0 crate. It remains a
licence question, not a port.

## Documentation

* `crates/holidays-rs/README.md` names the sixteen new calendars in its covered
  list, records the held-out fit check in "Verification", and updates the
  closest-holiday fixture figures to 26,400 rows over 66 countries.
* `PLAN.md`'s `holidays-rs` section moves the count from 86 to 102, names the new
  Orthodox-Easter calendars, and states that `derive_holidays.py` refuses a fit
  that stops agreeing outside the fitted window.
* `research/tools/README.md` describes the two-rule split, the shift-table
  preference and the held-out check in `derive_holidays.py`'s row.

## Housekeeping in the same round

Three small changes that are not holiday data:

* The redundant `homepage` field is dropped from the workspace and from every
  crate. crates.io already shows the repository URL from `repository`, so the
  second field adds nothing but a place to drift out of date.
* `ebics` and `pades` each carry a copy of `BigUint::sub`, and clippy 1.100
  fails both under `-D warnings` with `needless_range_loop` - an untouched crate
  broken by a newer lint, the trap `AGENTS.md` records. Both loops now iterate
  the limbs directly, so the fix is in the code rather than an `allow`.
* `AGENTS.md` gains the note that a gate reading live data fails on the data,
  not the code: `verify_aviation.py` reads OurAirports' CSV, so a row added or
  re-coordinated upstream shows up as a red gate with no crate change behind it.

## Method

The gates are the review, and this round is the clearest case yet of why. The
previous round's conclusion - that no more countries fit - was a reading of the
tool's output, and the tool was wrong. The gate that compares the crate against
the reference caught the same defect on a country already shipped. The lesson is
in `AGENTS.md` already: a generator is code, and its bugs look exactly like crate
bugs. The fix here was to make the generator's own promise - "refuses to emit a
table that disagrees with the reference" - true again, and to add the held-out
check that makes "a rule, not a table" a property the tool tests rather than a
claim it assumes.
