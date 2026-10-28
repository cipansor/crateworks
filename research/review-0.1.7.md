# Review of every crate against its mature comparison package (2026-10-27)

This is the follow-up to `review-0.1.6.md`. That round asked "of what we have,
where does a mature package in another ecosystem do more, and is that difference
worth closing". This round re-asks it after the aviation refresh, and the answer
is that the crates which already had an independent reference were sound and the
one genuine, oracle-verifiable gap left was in `spreadsheet-formula`.

## Correctness first: the differential gates were re-run

Every crate with an independent reference was re-checked against it.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_spreadsheet.py` | npm `@formulajs/formulajs` | **826/826 cases agree** (was 171) |
| `verify_aviation.py` | OurAirports `airports.csv` | **11,888 rows and 22,477 codes agree** |
| `verify_arabic.py` | PyPI `arabic-reshaper` | 6,558/6,558 agree |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` | 31,136 vectors agree |
| `verify_holidays.py` | PyPI `holidays` | 0 undocumented defects |
| `verify_hijri_locales.py` | PyPI `hijridate` 2.6.0 | 132/132 names agree |
| `verify_fuzzy_score.py` | PyPI `fuzzywuzzy` 0.18.0 | 0 mismatches |

The aviation gate failed at the start of the round and the failure was a **stale
snapshot, not a crate bug**: upstream OurAirports had run its periodic South
African cleanup, so the table drifted by +3, -4 rows and 14 changed fields. The
snapshot was regenerated to **11,888 rows**, and `EXPECTED_ROWS` in the generator,
the README, the crate comment and the test were moved with it. The gate then
passed unchanged, which is the point: the generator pins the row count so a
silent upstream shrink fails loudly rather than shipping.

## What this round develops

**`spreadsheet-formula`** - the gate had been driving only the functions the
reference publishes under a flat name. Extending `CASES` to the conditional
aggregates (`COUNTIF`, `SUMIF`, `COUNTIFS`, `SUMIFS`, `AVERAGEIF`, `AVERAGEIFS`,
`MAXIFS`, `MINIFS`, `COUNTBLANK`, `SUMPRODUCT`), the distributions and financial
functions it did not yet exercise (`CHIDISTRT`, `COVARIANCEP`, `DB`, `DISC`,
`PRICEDISC`) and the information functions (`ISBLANK`, `ISERR`, `ISNA`, `IFNA`)
took the gate from 171 to **826 cases**, all agreeing.

That pass also found a real gap: the **confidence-interval family** -
`CONFIDENCE`, `CONFIDENCE.NORM` and `CONFIDENCE.T` - which the crate did not
implement. All three are shipped:

* `CONFIDENCE` and `CONFIDENCE.NORM` are the same function under the pre-2010 and
  modern names, `NORM.S.INV(1 - alpha/2) * sigma / sqrt(n)`.
* `CONFIDENCE.T` is `T.INV.2T(alpha, n - 1) * sigma / sqrt(n)`, built on the
  crate's existing one-tailed `studentt_inv` (the lower tail, made positive), and
  rejects a sample smaller than two because it divides by `n - 1`.

The three names canonicalise to one arm, so the dotted spelling a modern formula
uses and the flat one a legacy formula uses evaluate identically. Six new gate
cases and a crate unit test pin the reference values, including the error edges
(alpha outside `(0, 1)`, a non-positive standard deviation, `n < 2` for the t
form). The README function count moved from 342 to **345**.

**`aviation-data`** - the table was refreshed (above) and a clippy
`doc_lazy_continuation` defect in the `find` doc comment was fixed, so
`cargo clippy --workspace --all-targets -- -D warnings` is clean again.

## Documentation drift found and fixed

* `PLAN.md` still said the aviation table carries 11,889 airports and is pinned
  at 11,889 rows; the refreshed table is **11,888**. Both sites corrected.
* `address-parser` and `cssprint` were the only two crates whose README carried
  no crates.io or docs.rs badge, so their published pages had no link back to the
  registry or to their own documentation. Both now carry the same two badges as
  the other twenty-four crates.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. Re-running them is cheap; the round's cost is in
the one gate whose corpus had to grow before it could answer.
