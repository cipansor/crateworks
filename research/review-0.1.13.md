# Review of every crate against its mature comparison package (2026-10-30)

This is the follow-up to `review-0.1.12.md`. That round re-asked "of what we
have, where does a mature package in another ecosystem do more, and is that
difference worth closing", closed the `mllp` error location (`ERR-2`), and
recorded the remaining candidates with the reason each stays out.

This round re-runs every gate first, then diffs each crate's public surface
against its reference once more. One candidate came out of that pass and was
closed: the `factur-x` profile reader, the reference's `get_facturx_level` /
`get_level`. The crate could write and read an EN 16931 invoice but could not
say which profile the document claimed to follow, even though the claim is in a
field it already carried (`Invoice::guideline`, BT-24). PyPI `factur-x`
answers the question, so the gap was closable and gateable in one step. The
other candidates the diff surfaced are recorded below with the reason each
stays out.

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
| `verify_mllp.py` | PyPI `hl7apy` | 135/135 fields, 8/8 ACK, 8/8 enhanced ACK replies agree |
| `verify_openlocationcode.py` | PyPI `openlocationcode` 1.0.1 | 1,389 rows agree |
| `verify_iso20022.py` | the official ISO 20022 XSD | generated pacs.008 and pain.001 validate |
| `verify_proxy_addr.py` | npm `proxy-addr` 2.0.8 | 12,216 rows agree |
| `tests/oracle.rs` (fast-redact) | npm `fast-redact` 3.5.0 | 6,000 redactions and 90 path verdicts agree |

No gate regressed.

## What this round develops

**`factur-x` 0.2.0 -> 0.3.0, the profile reader.** Every Factur-X, ZUGFeRD and
Order-X document names a profile in its specification identifier (BT-24): one of
`minimum`, `basicwl`, `basic`, `en16931`, `extended`, the French
`extended-ctc-fr`, or Order-X's `comfort`. The crate carried the identifier as a
string but never turned it into the profile it names.

The port adds one enum, one function and one error variant:

| Item | Reference | What it is |
| --- | --- | --- |
| `Profile` | the level names | the seven profiles, with `as_str` and `ALL` |
| `profile_of` | `get_facturx_level` / `get_level` | reads the identifier from the element the syntax uses and returns the profile |
| `Profile::parse` | the level-reading half | reads a level from an identifier string |
| `Error::UnknownProfile` | the reference's `ValueError` | an identifier that names no known profile |

The identifier is a URN, and the profile is its last segment; when that is not a
known level the identifier is cut at the first `#` and the segment before the
last `:` is tried, which is how `urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:basic`
still names `basic`. `profile_of` reads CII's nested
`ram:GuidelineSpecifiedDocumentContextParameter/ram:ID` and UBL's
`cbc:CustomizationID`, on the `Invoice` and `CreditNote` roots alike, matching
the reference's two XPaths. An identifier that names no known profile is an
error rather than a guess, which is what the reference does.

**How it is trusted.** The new gate `research/tools/verify_facturx_level.py`
builds a corpus of 33 documents - every URN the reference's tables recognise,
the qualified spellings, the French CTC extension, a credit note, and a few
identifiers the reference rejects - in both syntaxes, asks the reference for the
level, and checks the crate agrees. All 33 agree, including the four that are the
error path (the reference raising is recorded as the empty expected level). The
corpus and expected levels are committed to
`crates/factur-x/tests/fixtures/profile.tsv` and `tests/fixtures/profile/`, and
`crates/factur-x/tests/profile.rs` replays them, so the gate runs in CI without
the reference installed.

**What is deliberately left out.** The crate reads *which* profile a document
claims but does not enforce the extra rules (code lists, the national profiles
such as XRechnung) that the profile adds. That is a separate job and the
reference does not do it in `get_level` either; it is recorded in the README's
"What it does not do".

## Coverage: the candidates the diff surfaced and why they stay out

The public-surface diff against each reference left the same candidates the
earlier rounds recorded. None is a gap left by oversight; each is recorded so
the next round does not re-open it.

**`object-hash`: additional digests.** The reference supports every algorithm in
Node's `crypto.getHashes()`; the crate carries SHA-1/224/256/384/512, MD5 and
the passthrough form, which are the ones a cache key uses. SHA-3 and the
`shake`/`ripemd`/`sm3` families are not in this crate, and adding a digest means
adding a hash implementation by hand, since the workspace forbids dependencies.
The set that is there is the set the reference's own README documents.

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

**`stdnum-rs`: the six missing modules.** `python-stdnum` has pure-algorithmic
modules the crate does not: `Casrn`, `Cfi`, `Imsi`, `Iso11649`, `Iso9362` and
`Vatin`. `Iso9362` (BIC) is already the crate's `Bic`; the others are
per-country or per-registry formats (CAS registry numbers, ISO 10962
classification codes, ISO 11649 creditor references, VATIN) whose check
algorithms overlap `CreditorReference`, `Isin` and `EuVat`. They are a
candidate for a later round rather than a gap in what is modelled.

**`hijridate` and `iso8583-msg`.** The surfaces match their references; the only
differences are Rust-idiomatic shape (borrowed returns, iterators) rather than
missing capabilities.

## Documentation

* `crates/factur-x/README.md`, which is the crate's `#![doc]` and therefore the
  docs.rs front page, gains a "Which profile a document claims" section with the
  `profile_of` example, and its "What it does not do" now says the crate reads
  the profile but does not enforce it.
* `PLAN.md`'s `factur-x` section moves the profile reader into "covers today" and
  narrows the gap to the national profiles.
* `research/tools/README.md` gains the `verify_facturx_level.py` row, and
  `crates/factur-x/tests/readme.rs` runs the new README example.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. This round's development came from the same loop:
diffing the crate's public surface against the reference's showed that the
reference turns the carried identifier into a profile and the crate did not, and
the reference both implements it and can be asked for the answer, so the gap was
closable and checkable in one step.
