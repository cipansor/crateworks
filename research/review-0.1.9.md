# Review of every crate against its mature comparison package (2026-10-27)

This is the follow-up to `review-0.1.8.md`. That round re-asked "of what we
have, where does a mature package in another ecosystem do more, and is that
difference worth closing", and the answer was the seventeen national
identifiers `python-stdnum` covers and the crate did not.

This round re-asks it again and finds the remaining gap that is both
oracle-verifiable and worth closing: **`holidays-rs` against PyPI `holidays`**.
The reference covers about 200 countries; the crate had 55. The ten added this
round are the ones that fit the crate's `Rule` vocabulary exactly, so each can
be pinned by the same gate the other calendars already use.

## Correctness first: the differential gates were re-run

Every crate with an independent reference was re-checked against it.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_holidays.py` | PyPI `holidays` | **0 undocumented defects, 65 calendars** (was 55) |
| `gen_holidays_closest.py` | PyPI `holidays` | **21,200 rows over 53 countries agree** (was 17,200 / 43) |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` | 49,300 vectors agree |
| `verify_aviation.py` | OurAirports `airports.csv` | 11,888 rows and 22,477 codes agree |
| `verify_arabic.py` | PyPI `arabic-reshaper` 3.0.0 | 6,558/6,558 agree (per language) |
| `verify_hijri_locales.py` | PyPI `hijridate` 2.6.0 | 132/132 names agree |
| `verify_fuzzy_score.py` | PyPI `fuzzywuzzy` 0.18.0 | 0 mismatches |
| `verify_units.py` | PyPI `pint` | 973 compared, 0 mismatches |
| `verify_mllp.py` | PyPI `hl7apy` | 135/135 fields, 8/8 ACK replies agree |
| `verify_spreadsheet.py` | npm `@formulajs/formulajs` | 1,003/1,003 cases agree |
| `verify_iso20022.py` | the official ISO 20022 XSD | generated pacs.008 and pain.001 validate |

No gate regressed.

## What this round develops

**`holidays-rs` 0.3.0 -> 0.4.0.** Ten countries were added, from 55 to 65,
each taken from the reference's own table and verified by `verify_holidays.py`:

| Country | Code | Shape |
| --- | --- | --- |
| Liechtenstein | LI | the Catholic fixed set plus the Easter cycle |
| Saint Martin | MF | the French set, all of it from 2008 |
| New Caledonia | NC | the French set, Citizenship Day from 2004 |
| Martinique | MQ | the French set plus its own abolition day |
| Wallis and Futuna | WF | the French set plus Territory Day |
| Guadeloupe | GP | the French set plus Mi-Carême and its abolition day |
| French Polynesia | PF | the French set; Matāri'i from 2025 |
| Haiti | HT | the independence and revolutionary days |
| Svalbard and Jan Mayen | SJ | the Norwegian set |
| Lesotho | LS | the Heroes Day to Africa/Heroes Day rename in 2003 |

The candidate list came from `research/tools/fit_holidays.py`, which ranks every
reference country by how well it fits the crate's `Rule` set. The countries it
flags as *not* clean were left alone on purpose: San Marino is missing a
2006-04-01 entry, Svalbard's reference carries twelve spurious differences
before the fix, and Honduras has dates the crate cannot express. Fitting first
and building second is what keeps the gate green rather than red.

Two countries needed a `Since`/`Until` window rather than a plain rule, which is
the same mechanism Romania's holidays already use: Saint Martin's whole calendar
starts in 2008, and New Caledonia's Citizenship Day replaced Possession Day in
2004 while its Whit Monday was suspended from 2005 to 2007.

The `closest_holiday` fixture grew with the set: `gen_holidays_closest.py` now
covers the ten new countries, so `closest.tsv` moved from 17,200 to 21,200 rows
and from 43 to 53 countries.

## Documentation

* `crates/holidays-rs/README.md` names the ten new countries in its coverage
  list, which is the crate's `#![doc]` and therefore the docs.rs front page, and
  its `closest_holiday` paragraph moves from 17,200 rows over 43 countries to
  21,200 over 53.
* `PLAN.md` moves from 45 to 65 countries.
* `research/tools/verify_holidays.py` no longer claims the crate "covers 19"; it
  reads 65.

## What is deliberately not done

`spreadsheet-formula` still lacks 36 `formulajs` functions, and they are left
out because they cannot be verified cleanly: the majority are volatile
(`NOW`, `RAND`, `TODAY`), which this crate documents as out of scope, and the
rest are matrix and dynamic-array functions (`MMULT`, `MUNIT`, `TRANSPOSE`,
`HSTACK`, `VSTACK`, `DROP`, `TAKE`, `EXPAND`) whose reference implementation
returns array objects the crate's scalar evaluator does not model. Adding them
without a matching oracle would be inventing expectations, which is the one
thing the method forbids.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. The cost of this round was in `holidays-rs`,
whose reference had the most left to give once the fitting tool was used to pick
the cheap countries.
