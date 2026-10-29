# Review of every crate against its mature comparison package (2026-10-29)

This is the follow-up to `review-0.1.10.md`. That round re-asked "of what we
have, where does a mature package in another ecosystem do more, and is that
difference worth closing", closed two gaps and explained a third as blocked by a
licence.

This round re-runs every gate first, then diffs each crate's public surface
against its reference once more. One candidate came out of that pass and was
closed: the `mllp` enhanced acknowledgement (`MSA-4` and the `ERR` segment),
which the crate's own PLAN recorded as not covered and which PyPI `hl7apy`
supports and can therefore gate. The other candidates that the diff surfaced are
recorded below with the reason each stays out.

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

**`mllp` 0.1.9 -> 0.2.0, the enhanced acknowledgement.** The crate built the
minimal `ACK` - `MSH` and `MSA` with the acceptance code and the echoed control
id - and the PLAN said the enhanced form was left to the caller. The enhanced
form is what an interface sends when "rejected" is not enough: MSA-4 carries the
error condition code and an `ERR` segment repeats it as a coded-with-exceptions
value with its text and a severity.

The port adds two types and one function:

| Item | Reference | What it is |
| --- | --- | --- |
| `ErrorDetail` | ERR-3 | the error condition: code, text, coding system, severity |
| `Severity` | ERR-4 (HL7 table 0516) | `Warning` (`W`), `Error` (`E`), `Fatal` (`F`) |
| `build_ack_enhanced` | the enhanced `ACK` | `build_ack` plus MSA-4 and the `ERR` segment |

`build` and `build_enhanced` share one header writer and one `MSA` writer, so the
enhanced reply cannot drift from the minimal one: the only difference is the
appended MSA-4 and the `ERR` segment. The reply still uses the delimiters of the
message it answers, so an `ERR-3` for a peer that separates components with `*`
is written with `*`.

**How it is trusted.** The gate already drove the crate's `ACK` through
`hl7apy`; it now drives the enhanced `ACK` too. For every message in the corpus
the crate builds both replies, and `hl7apy` reads each back: MSA-4 must hold the
code, ERR-3 must read as a CWE with the code, text and coding system in the
right components, and ERR-4 must be the severity. The gate went from 8 to 16
checked replies and all 16 agree.

**What is deliberately left out of the enhanced mode.** ERR-1 and ERR-2 name the
segment and the field the error is in. The builder is not told where the error
is - the caller is - so they are written empty and the caller sets them after,
which is the same division of labour the minimal reply already uses for MSH-7,
the build time.

## Coverage: the candidates the diff surfaced and why they stay out

The public-surface diff against each reference left four more candidates. None is
a gap left by oversight; each is recorded so the next round does not re-open it.

**`object-hash`: `respectType`.** The reference's default makes every plain
object carry a preamble describing Node's prototype chain. That is a fact about
the JavaScript runtime, not about the value, so the crate always encodes the
`respectType: false` form and documents it. Encoding the default would make a
Rust digest depend on Node internals, which is the opposite of what a caller
wants from a content hash.

**`runits`: cross-dimension comparison and the Gaussian units.** `pint` lets two
quantities of the same dimension compare; the crate converts for that already.
The Gaussian and CGS-EM units (`statC`, `G`, `Mx`) carry fractional dimension
exponents a vector of `i8` cannot hold, and the logarithmic scales (`dB`, `Np`)
are not a linear factor at all; both are already reported as `Unsupported` rather
than `Unknown`, so a real unit is never mistaken for a typo. Closing them would
mean a second dimension representation for a small set of units.

**`spreadsheet-formula`: the dynamic-array and volatile family.** The 38 names
left by the diff are the volatile functions (`NOW`, `RAND`, `TODAY`), the
dynamic-array and matrix family (`MMULT`, `TRANSPOSE`, `HSTACK`, `VSTACK`,
`DROP`, `TAKE`, `SORT`, `UNIQUE`, `LINEST`, `TREND`), and namespace objects that
are naming differences the crate already serves unsuffixed. The first group is
non-deterministic and so cannot be gated against a fixture; the second returns
array objects this scalar evaluator does not model. Both are out of scope rather
than missing.

**`holidays-rs`: the remaining countries.** The reference carries about 250
calendars; the crate has 86. `research/tools/fit_holidays.py` ranks the rest, and
every remaining country fails the fit for the same reason: its reference table
shifts each holiday independently (an irregular observed day, a "ponte", a
one-year government decree), while the crate holds one crate-wide
`SubstituteStyle` per country. The countries that fit have all been added; the
rest need a richer shift model before any of them can be pinned by the gate.

## Documentation

* `crates/mllp/README.md`, which is the crate's `#![doc]` and therefore the
  docs.rs front page, gains an "Explaining an error" section with a runnable
  example, and its "What it does not do" section now names ERR-1 and ERR-2
  instead of the whole enhanced mode.
* `PLAN.md`'s `mllp` section moves the enhanced mode from "not covered" to
  "covers today" and narrows the gap to ERR-1 and ERR-2.
* `research/tools/README.md` and the gate itself describe the two replies the
  gate now checks.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. This round's development came from the same
loop: the PLAN's own "not covered" line named the enhanced acknowledgement, and
the reference `hl7apy` both implements it and can read the result back, so the
gap was closable and checkable in one step rather than by reading a
specification and hoping.
