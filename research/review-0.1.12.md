# Review of every crate against its mature comparison package (2026-10-29)

This is the follow-up to `review-0.1.11.md`. That round re-asked "of what we
have, where does a mature package in another ecosystem do more, and is that
difference worth closing", closed the `mllp` enhanced acknowledgement (`MSA-4`
and the `ERR` segment), and recorded the remaining candidates with the reason
each stays out.

This round re-runs every gate first, then diffs each crate's public surface
against its reference once more. One candidate came out of that pass and was
closed: the `mllp` error *location* (`ERR-2`), which the previous round's own
review recorded as the one part of the enhanced acknowledgement still left out.
The other candidates that the diff surfaced are recorded below with the reason
each stays out.

## Correctness first: the differential gates were re-run

Every gate was re-run before any change. All were green on entry.

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
| `verify_mllp.py` | PyPI `hl7apy` | 135/135 fields, 8/8 ACK, **8/8 enhanced ACK** replies agree |
| `verify_openlocationcode.py` | PyPI `openlocationcode` 1.0.1 | 1,389 rows agree |
| `verify_iso20022.py` | the official ISO 20022 XSD | generated pacs.008 and pain.001 validate |
| `verify_proxy_addr.py` | npm `proxy-addr` 2.0.8 | 12,216 rows agree |
| `tests/oracle.rs` (fast-redact) | npm `fast-redact` 3.5.0 | 6,000 redactions and 90 path verdicts agree |

No gate regressed.

## What this round develops

**`mllp` 0.2.0 -> 0.3.0, the error location.** The enhanced acknowledgement
(previous round) said *what* went wrong - the error condition code and severity
- but not *where*. HL7 v2 puts that in `ERR-2`, an ERL value: the segment id, the
segment's one-based position in the message, the field's one-based position, and
then the repetition, component and subcomponent when the error is narrower than
a whole field. The previous review recorded this as the one piece left out,
because the builder was not told where the error was.

The port adds one type and one method:

| Item | Reference | What it is |
| --- | --- | --- |
| `ErrorLocation` | ERR-2 (ERL) | segment id, sequence, field, and optional repetition/component/subcomponent |
| `ErrorDetail::at` | ERR-2 | sets the location on the error condition |

`ErrorLocation::field` takes the three required parts and the three builder
methods (`with_repetition`, `with_component`, `with_subcomponent`) narrow it. A
part that is unset but followed by a set one becomes an empty component, so the
component lands in the position a reader expects; a location that stops at the
field writes `PID^2^3` and never a run of trailing separators.

**How it is trusted.** The gate already drove the crate's enhanced `ACK` through
`hl7apy`; it now also builds one with a location and asserts `ERR-2` is the ERL
the standard defines. It checks the raw `ERR-2` text *and* reads it back through
`hl7apy`'s ERL components (`ERL_1`, `ERL_2`, `ERL_3`, `ERL_5`), so a component
put in the wrong position is caught even when the text happens to match. The gate
is still 16 checked replies (8 minimal, 8 enhanced) and all 16 agree.

**What is deliberately left out of the enhanced mode.** ERR-1, the deprecated
"error code and location" field, is written empty. ERR-2 carries the location and
is what a current reader looks at. The builder still does not *find* the
offending field: what counts as an error, and where it is, is the application's
judgement, not the parser's, so the caller supplies the `ErrorLocation`. That is
the same division of labour the minimal reply already uses for MSH-7, the build
time.

## Coverage: the candidates the diff surfaced and why they stay out

The public-surface diff against each reference left the same candidates the last
two rounds recorded. None is a gap left by oversight; each is recorded so the
next round does not re-open it.

**`javaproperties`: the mutable-mapping surface.** The reference's `Properties`
is a `MutableMapping` with `setProperty`, `pop`, `popitem`, `setdefault`,
`update`, `clear`, `stringPropertyNames` and `propertyNames`; the crate is a
reader and a writer over an ordered list of pairs, with `get` returning the last
value for a repeated key, which is what Java does. Adding the whole mapping
surface is a design change (the crate models a document that may carry a key
twice, which a `dict` cannot), and the reference's own `PropertiesFile` keeps the
duplicate entries exactly as the crate does, so the difference is which of the
two reference types to mirror, not a missing capability.

**`object-hash`: `respectType`.** The reference's default makes every plain
object carry a preamble describing Node's prototype chain. That is a fact about
the JavaScript runtime, not about the value, so the crate always encodes the
`respectType: false` form and documents it.

**`runits`: cross-dimension comparison and the Gaussian units.** `pint` lets two
quantities of the same dimension compare; the crate converts for that already.
The Gaussian and CGS-EM units (`statC`, `Mx`) carry fractional dimension
exponents a vector of `i8` cannot hold, and the logarithmic scales (`dB`, `Np`)
are not a linear factor at all; both are reported as `Unsupported` rather than
`Unknown`.

**`spreadsheet-formula`: the dynamic-array and volatile family.** The volatile
functions (`NOW`, `RAND`, `TODAY`) are non-deterministic and so cannot be gated
against a fixture; the dynamic-array and matrix family (`MMULT`, `TRANSPOSE`,
`HSTACK`, `VSTACK`, `DROP`, `TAKE`, `SORT`, `UNIQUE`, `LINEST`, `TREND`) returns
array objects this scalar evaluator does not model.

**`holidays-rs`: the remaining countries.** The reference carries about 250
calendars; the crate has 86. Every remaining country fails the fit for the same
reason: its reference table shifts each holiday independently, while the crate
holds one crate-wide `SubstituteStyle` per country. The countries that fit have
all been added.

## Documentation

* `crates/mllp/README.md`, which is the crate's `#![doc]` and therefore the
  docs.rs front page, gains the `ErrorLocation` example and its explanation, and
  its "What it does not do" section now names ERR-1 instead of the whole location.
* `PLAN.md`'s `mllp` section moves the error location from "not covered" to
  "covers today" and narrows the gap to ERR-1.
* `research/tools/README.md` and the gate itself describe the `ERR-2` location
  the gate now checks.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. This round's development came from the same loop:
the previous review's own "not covered" line named the error location, and the
reference `hl7apy` both implements it and can read the result back, so the gap
was closable and checkable in one step.
