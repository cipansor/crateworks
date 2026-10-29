# Review of every crate against its mature comparison package (2026-10-29)

This is the follow-up to `review-0.1.9.md`. That round re-asked "of what we have,
where does a mature package in another ecosystem do more, and is that difference
worth closing", and the answer was ten more `holidays-rs` countries, from 55 to
65.

This round re-asks it once more. Every gate was re-run first, then each crate's
public surface was diffed against its reference. Three candidates came out of
that pass: two were closed - the missing `fuzzywuzzy.process` functions and a
`TIMEVALUE` parsing defect the widened spreadsheet gate caught - and the third,
the `python-stdnum` data tables, is blocked by a licence rather than by effort.
The remaining spreadsheet names were shown to be naming differences or
out-of-scope features rather than gaps.

## Correctness first: the differential gates were re-run

Every gate was re-run, and then every crate's public surface was diffed against
its reference. The full mapping, crate by crate, is in *Coverage* below; the
gate results come first.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_holidays.py` | PyPI `holidays` | 0 undocumented defects, 86 calendars |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` | 85,713 vectors agree |
| `verify_spreadsheet.py` | npm `@formulajs/formulajs` | 1,068/1,068 cases agree |
| `verify_aviation.py` | OurAirports `airports.csv` | 11,887 rows and 46,456 lookups agree |
| `verify_arabic.py` | PyPI `arabic-reshaper` 3.0.0 | 6,558/6,558 agree (per language) |
| `verify_hijri_locales.py` | PyPI `hijridate` 2.6.0 | 132/132 names agree |
| `verify_fuzzy_score.py` | PyPI `fuzzywuzzy` 0.18.0 | 0 mismatches, 5 process functions |
| `verify_units.py` | PyPI `pint` | 973 compared, 0 mismatches |
| `verify_mllp.py` | PyPI `hl7apy` | 135/135 fields, 8/8 ACK replies agree |
| `verify_openlocationcode.py` | PyPI `openlocationcode` 1.0.1 | 1,389 rows agree |
| `verify_iso20022.py` | the official ISO 20022 XSD | generated pacs.008 and pain.001 validate |
| `verify_proxy_addr.py` | npm `proxy-addr` 2.0.8 | 12,216 rows agree |
| `tests/oracle.rs` (fast-redact) | npm `fast-redact` 3.5.0 | 6,000 redactions and 90 path verdicts agree |

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

**`spreadsheet-formula`: the gate was widened, and it caught a real bug.** The
name-level diff against `@formulajs/formulajs` leaves 38 names. They are the
volatile functions (`NOW`, `RAND`, `RANDBETWEEN`, `TODAY`), the dynamic-array and
matrix family (`MMULT`, `TRANSPOSE`, `HSTACK`, `VSTACK`, `DROP`, `TAKE`,
`EXPAND`, `CHOOSECOLS`, `CHOOSEROWS`, `SORT`, `UNIQUE`, `MUNIT`, `FREQUENCY`,
`LINEST`, `LOGEST`, `TREND`, `GROWTH`), and a set of namespace objects
(`BETA.DIST`, `NORM.INV`, `POISSON.DIST`, `Z.TEST`, `ISO.CEILING`,
`ERROR.TYPE`). The first group is documented as out of scope, the second returns
array objects this scalar evaluator does not model, and most of the third is a
naming difference rather than a missing function: the crate already serves
`BETADIST`, `NORMINV`, `POISSONDIST`, `ZTEST` and `ISO`-style `CEILING` under
their unsuffixed spellings, and the gate mirrors each dotted name from its flat
spelling so both are checked.

`ERROR.TYPE` is the one genuine member of that group, and it stays out: it is the
only way to recover an error code, and this crate reports a failure as a Rust
`Result` with no error value inside the expression language, so there is nothing
for the argument to evaluate to. The reference's own `ERROR.TYPE` returns `#N/A`
for every input that is not one of its internal sentinels - `ERROR.TYPE(1)` and
`ERROR.TYPE(new Error('#DIV/0!'))` both answer `#N/A` - so it is not a behaviour
a caller could rely on either.

Widening the gate to the lookup and control-flow family (`VLOOKUP`, `HLOOKUP`,
`INDEX`, `MATCH`, `LOOKUP`, `IF`, `IFS`, `SWITCH`, `IFERROR`, the right-tail
distributions, `WORKDAY`, `WORKDAY.INTL`, `TIMEVALUE`, `XNPV`) is what turned up
this round's second development. It also exposed a hole in the harness: the JS
driver mapped every object result to `error`, so a date-returning function the
reference does not serialise looked like a mismatch. Teaching it to read a `Date`
back as a serial made `WORKDAY` and `WORKDAY.INTL` gateable, and left one real
defect standing: **`TIMEVALUE` rejected a full date-time string.** Excel and
`formulajs` both accept `TIMEVALUE("2024-01-01 12:00")` and return `0.5`; the
crate parsed only a bare clock time and returned an error. `parse_time` now
accepts a date half separated by a space or an ISO 8601 `T`, keeps the clock
time, and treats a bare date as midnight, which is what both references do.

The gate grew from 1,028 to 1,068 cases and all 1,068 agree. `PLAN.md`'s "not
covered" note was rewritten to say exactly what the 38 names are, so the count
is explained rather than asserted.

**`fast-redact` 0.1.0, a new crate.** This round also added a crate rather than
only extending one. npm `fast-redact`, which is what the `pino` logger uses to
keep cookies and tokens out of log lines, takes the locations of the secret
fields as paths and replaces the values at them. The redaction crates on
crates.io are regex- or serde-rule-based and none walks a path through the
document, so the port fills a gap the other reviews did not cover. It carries
its own JSON reader and writer, because the output has to be byte-for-byte what
`JSON.stringify` produces, and it is checked against the npm package over 6,000
generated redactions and 90 path verdicts. That corpus pinned the one shape
worth knowing: a path is split at its first `*`, so a run of stars does not nest
the way it reads, which the reference's own tests exercise and the crate now
reproduces.

## Coverage: every crate, its reference, and where the check lives

The gate table above is the set of crates whose reference is a *program* that can
be driven over a corpus. The rest are checked by a committed fixture built from
the reference, or by a test that names the specification, because their reference
is a data set or a document rather than a runnable package. Either way, no crate
in this workspace is reviewed by reading alone.

| Crate | Reference | Where the check lives |
| --- | --- | --- |
| `xmlkit` | Python's `expat` (well-formedness) | `scripts/fuzz/xmlkit/gen.py` -> `tests/fixtures/wellformed.tsv`, replayed by `tests/wellformed.rs` |
| `iso20022-rs` | the official ISO 20022 XSD | `verify_iso20022.py` |
| `ebics` | node-ebics-client; Python `cryptography` for the crypto | `tests/rsa_interop.rs`, `signature_oracle.rs`, `c14n_oracle.rs`, `response_oracle.rs` |
| `soap-client` | the SOAP 1.1/1.2 and WSDL specifications | inline tests, 32 of them, plus the README example |
| `wssec` | OASIS UsernameToken Profile 1.1; `zeep` | `tests/fixtures.rs` against `zeep`'s structural paths |
| `factur-x` | the official `factur-x` package (akretion) | `tests/oracle.rs` |
| `pdfa` | PDF/A-3 and Factur-X specifications; reference PDFs | `tests/reference.rs`, `tests/incremental.rs` |
| `pdfmake` | the pdfmake JS model; Adobe AFM metrics via `reportlab`/`pdfminer.six` | inline tests, 68 of them, plus `tests/readme.rs` |
| `pades` | RFC 5652 (CMS) and the PAdES baseline profiles | `tests/verify.rs` over the signed-PDF fixtures |
| `stdnum-rs` | PyPI `python-stdnum` | `gen_stdnum_vectors.py`, 85,713 vectors |
| `eu-vat` | PyPI `python-stdnum` | `tests/oracle.rs` over the derived fixture |
| `iso6346` | npm `container-validator` (sameershemna) | `tests/oracle.rs` |
| `holidays-rs` | PyPI `holidays` | `verify_holidays.py` |
| `hijridate` | PyPI `hijridate` | `verify_hijri_locales.py`, `gen_hijri_table.py` |
| `adhan` | JS `adhan` (batoulapps) | `tests/oracle.rs`, `reference.rs`, `qibla_sunnah.rs`, `navigation.rs` |
| `arabic` | PyPI `arabic-reshaper` | `verify_arabic.py` |
| `spreadsheet-formula` | npm `@formulajs/formulajs`, `jStat`, `bessel` | `verify_spreadsheet.py`, `gen_convert_units.py` |
| `runits` | PyPI `pint` | `verify_units.py`, `gen_units_arithmetic.py` |
| `object-hash` | npm `object-hash` (puleos) | `tests/oracle.rs`, `fuzz.rs`, `numbers.rs` |
| `javaproperties` | PyPI `javaproperties` (jwodder) | `tests/oracle.rs`, `writer_oracle.rs`, `xml_oracle.rs` |
| `mllp` | PyPI `hl7apy` | `verify_mllp.py` |
| `fuzzy-score` | PyPI `fuzzywuzzy` | `verify_fuzzy_score.py` |
| `address-parser` | npm `parse-address` (hassansin) | `tests/oracle.rs` |
| `aviation-data` | OurAirports `airports.csv` | `verify_aviation.py` |
| `cssprint` | WeasyPrint (Kozea) | `gen_weasyprint_layout.py` -> `tests/fixtures/layout.tsv`, replayed by `tests/layout.rs` |
| `openlocationcode` | google/open-location-code | `verify_openlocationcode.py` |
| `proxy-addr` | npm `proxy-addr` (jshttp) | `scripts/fuzz/proxy-addr/gen.js` -> `tests/fixtures/oracle.tsv`, replayed by `tests/oracle.rs`, and `verify_proxy_addr.py` |
| `iso8583-msg` | PyPI `pyiso8583` | `tests/oracle.rs` |
| `fast-redact` | npm `fast-redact` (pino's redactor) | `scripts/fuzz/fast-redact/gen.js` -> `tests/fixtures/`, replayed by `tests/oracle.rs` |

Two crates have no runnable reference and are checked against the specification
itself: `soap-client` (the SOAP and WSDL documents) and `pades` (RFC 5652 and the
PAdES baseline profiles), each with fixtures that a conforming implementation
produced. Neither difference is a gap; both are named so the next round knows
where to look.

## Documentation

* `crates/fuzzy-score/README.md`, which is the crate's `#![doc]` and therefore
  the docs.rs front page, documents `extract_bests` and `extract_without_order`
  beside `extract`, and its "how it is checked" paragraph moves from three
  process functions to five.
* `crates/spreadsheet-formula/README.md` gains a paragraph in *Scope* naming the
  text forms `DATEVALUE` and `TIMEVALUE` accept, including the date-time form
  this round fixed.
* `PLAN.md`'s `fuzzy-score` section lists the five process functions, its
  `spreadsheet-formula` section explains the 38 missing names in three groups
  rather than as an unexplained count, and its verification block moves the
  doctest count from 210 to 213.
* `research/tools/README.md` and the gate itself describe the five-function
  coverage and the widened spreadsheet case set.

* `crates/fast-redact/README.md`, which is the crate's `#![doc]` and therefore
  the docs.rs front page, documents the path syntax, the three censor forms and
  the JSON fidelity the writer guarantees.

## What is deliberately not done

The `cfi`, `imsi` and `gs1_128` tables, for the licence reason above. The 38
`formulajs` names, for the three reasons above, of which only `ERROR.TYPE` is a
genuinely missing function rather than a naming difference or an out-of-scope
feature. Neither is a gap left by
oversight; both are recorded so the next round does not re-open them.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. Both developments this round came out of that
loop rather than out of reading documentation: the two missing
`fuzzywuzzy.process` functions were found by diffing the crate's public surface
against the reference's, and the `TIMEVALUE` defect surfaced only once the
spreadsheet gate was widened to cover the functions that had been checked by a
hand-written test alone. That is the argument for the gate over the test: a case
someone wrote down records what the implementation did, while a case generated
from the reference records what the reference does.
