# Review of every crate against its mature comparison package (2026-10-29)

This is the follow-up to `review-0.1.9.md`. That round re-asked "of what we have,
where does a mature package in another ecosystem do more, and is that difference
worth closing", and the answer was ten more `holidays-rs` countries, from 55 to
65.

This round re-asks it once more. Every gate was re-run first, then each crate's
public surface was diffed against its reference. Three candidates came out of
that pass and all three are settled here: one was closed, one was already
covered under a different name, and one is blocked by a licence rather than by
effort.

## Correctness first: the differential gates were re-run

Every crate with an independent reference was re-checked against it.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_holidays.py` | PyPI `holidays` | 0 undocumented defects, 86 calendars |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` | 85,713 vectors agree |
| `verify_spreadsheet.py` | npm `@formulajs/formulajs` | 1,028/1,028 cases agree |
| `verify_aviation.py` | OurAirports `airports.csv` | 11,887 rows and 46,456 lookups agree |
| `verify_arabic.py` | PyPI `arabic-reshaper` 3.0.0 | 6,558/6,558 agree (per language) |
| `verify_hijri_locales.py` | PyPI `hijridate` 2.6.0 | 132/132 names agree |
| `verify_fuzzy_score.py` | PyPI `fuzzywuzzy` 0.18.0 | 0 mismatches, 5 process functions |
| `verify_units.py` | PyPI `pint` | 973 compared, 0 mismatches |
| `verify_mllp.py` | PyPI `hl7apy` | 135/135 fields, 8/8 ACK replies agree |
| `verify_openlocationcode.py` | PyPI `openlocationcode` 1.0.1 | 1,389 rows agree |
| `verify_iso20022.py` | the official ISO 20022 XSD | generated pacs.008 and pain.001 validate |

No gate regressed. The aviation gate was red on entry, and for the honest
reason: OurAirports is maintained continuously and the table had drifted. It was
regenerated rather than pinned down - 11,888 to 11,887 rows, after upstream
restored `FATR` and removed `FAVP` and `ZA-0072` - and the crate, its pin and its
READMEs moved together.

## What this round develops

**`fuzzy-score` 0.1.10 -> 0.2.0.** The crate ported three of the five
`fuzzywuzzy.process` functions. `extractBests` and `extractWithoutOrder` were
missing, and both are pure logic with no data behind them, so both are now
ported and pinned:

| Function | Reference | What it adds |
| --- | --- | --- |
| `extract_bests` | `process.extractBests` | `extract` with a `score_cutoff` |
| `extract_without_order` | `process.extractWithoutOrder` | the unranked form: input order kept, cutoff applied |

`extract` and `extract_bests` now share `extract_without_order` as their one
implementation, which is what the reference does: it is a generator, and both
ranked functions are `heapq.nlargest` over it. The regression gate grew from
three process functions to five, over the same 2,000-case corpus, and all five
agree with `fuzzywuzzy` 0.18.0.

**`stdnum-rs`: no change, and the reason is a licence.** The name pass flags
three reference modules with no counterpart: `cfi` (ISO 10962 classification of
financial instruments), `imsi` (mobile subscriber identity) and `gs1_128`
(application identifiers in a Code 128 barcode). Each needs a data table that
ships inside `python-stdnum` under the LGPL - `cfi.dat` is 36 KB, `imsi.dat` is
385 KB, `gs1_ai.dat` is 25 KB - and a table copied from an LGPL source cannot go
into an Apache-2.0 crate. The algorithms are trivial once the tables exist; the
tables are the whole of the work, and they are the part that cannot be taken.
Closing these would mean regenerating the tables from the issuing bodies' own
published lists (SIX, GS1), which is a data-sourcing project rather than a port.
The same logic closed `casrn` already, and it is why the gate's `NOTICE` names
`python-stdnum` without lifting from it.

**`spreadsheet-formula`: the remaining names are three kinds, none closable.** A
name-level diff against `@formulajs/formulajs` leaves 38 names. They are the
volatile functions (`NOW`, `RAND`, `RANDBETWEEN`, `TODAY`), the dynamic-array and
matrix family (`MMULT`, `TRANSPOSE`, `HSTACK`, `VSTACK`, `DROP`, `TAKE`,
`EXPAND`, `CHOOSECOLS`, `CHOOSEROWS`, `SORT`, `UNIQUE`, `MUNIT`, `FREQUENCY`,
`LINEST`, `LOGEST`, `TREND`, `GROWTH`), and a set of namespace objects
(`BETA.DIST`, `NORM.INV`, `POISSON.DIST`, `Z.TEST`, `ISO.CEILING`,
`ERROR.TYPE`). The first group is documented as out of scope, the second returns
array objects this scalar evaluator does not model, and the third is a naming
difference rather than a missing function: the crate already serves `BETADIST`,
`NORMINV`, `POISSONDIST`, `ZTEST`, `ISO`-style `CEILING` and `ERROR`-style
`TYPE` under their unsuffixed spellings. `PLAN.md`'s "not covered" note was
rewritten to say exactly that, so the count is explained rather than asserted.

## Documentation

* `crates/fuzzy-score/README.md`, which is the crate's `#![doc]` and therefore
  the docs.rs front page, documents `extract_bests` and `extract_without_order`
  beside `extract`, and its "how it is checked" paragraph moves from three
  process functions to five.
* `PLAN.md`'s `fuzzy-score` section lists the five process functions, its
  `spreadsheet-formula` section explains the 38 missing names in three groups
  rather than as an unexplained count, and its verification block moves the
  doctest count from 210 to 213.
* `research/tools/README.md` and the gate itself describe the five-function
  coverage.

## What is deliberately not done

The `cfi`, `imsi` and `gs1_128` tables, for the licence reason above. The 38
`formulajs` names, for the three reasons above. Neither is a gap left by
oversight; both are recorded so the next round does not re-open them.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. The one development this round, the two missing
`fuzzywuzzy.process` functions, was found by diffing the crate's public surface
against the reference's rather than by reading either one's documentation.
