# Review of every crate against its mature comparison package (2026-11-16)

This is the follow-up to `review-0.1.16.md`. That round finished an ISO 20022
message pair and added the gate case that keeps it correct. This round closes the
one gap `runits` had named in its own README since it was written: the
logarithmic scales.

## What this round develops

### `runits` 0.1.10 -> 0.2.0, the logarithmic scales

`runits` is a port of PyPI `pint`: a unit name is looked up in a table compiled
from `pint`'s own definition files, so a conversion needs no network call and no
database. Its README carried a "Scope" section listing what the table leaves out,
and the first item was the logarithmic scales - `dB`, `Np`, `octave` - with the
reason that they "are not a linear factor at all, so storing one as a factor
would be wrong". That was a true statement about the representation, and it was
also a gap: `pint` converts them, and a caller converting an audio level or a
link budget expects to.

The seven scales are now carried, and the representation that makes them correct
is the point. A logarithmic value is not a linear multiple of a base unit; it is
a logarithm of a *ratio* to a reference quantity:

```text
value = divisor * log_base(quantity / reference)
```

`dBm` has `base = 10`, `divisor = 10` and a one-milliwatt reference, so `0 dBm`
is that reference and `30 dBm` is one watt. `dB` and `Np` anchor to the
dimensionless one, and `Np` is base `e` with `divisor = 0.5`. The generated table
grew a second static, `LOGARITHMIC`, holding `base`, `divisor`, `reference` and
the reference's dimension vector, and the name table marks a logarithmic name
with `LOG_FLAG` so one lookup serves both kinds. `unit("dBm").dimension()` is a
power, because that is the dimension of its reference.

**The traps this surfaced, now in `AGENTS.md`.** The first is that a generated
float can equal a named maths constant: `Np`'s base is `e`, and clippy's
`approx_constant` fails the build on the literal `2.718281828459045`. The crate
forbids an `allow`, so the generator now writes `std::f64::consts::E` by name
when a base equals `e`. The second is subtler and is why the arithmetic is
refused rather than approximated: `pint` computes `0 dBm + 1 dBm` by multiplying
the two references and returning a *squared* watt, a quantity neither operand
named. There is no logarithm to return, so `runits` refuses a sum of two
dimensioned logarithms instead of inventing a number; a sum of two dimensionless
ones is well defined (`3 dB + 3 dB` is `6 dB`, `3 dB + 1 Np` is `11.686 dB`) and
is allowed.

## The gate that covers it

Two differential gates cover `runits`, both driven by the installed `pint`
library rather than by a hand-written expectation.

* `research/tools/verify_units.py` checks every name's scale and dimensions
  against `pint`. It now resolves a logarithmic name through the same lookup the
  crate uses and compares it on `base`, `divisor` and `reference` rather than on
  a linear factor, and it writes the 13 logarithmic names to
  `crates/runits/tests/fixtures/pint_log_units.tsv`. `tests/pint.rs` replays that
  fixture against `unit(name).log_scale()`, so the numbers come from `pint` and
  the crate is checked against them.
* `scripts/fuzz/runits/gen.py` (moved from `research/tools/`, per the corpus
  rule that a generator lives under `scripts/fuzz/<name>/`) is the oracle for
  `add` and `subtract`. It gained a slice of the four dimensionless scales, where
  `pint`'s sum is the expectation, and a slice of the three power ratios, where
  `pint` returns a base quantity the crate refuses, so every row there expects an
  error. The dimensionless slice is 2,000 pairs and the power-ratio slice covers
  every ordered pair of scales and every magnitude, so the corpus is now 4,730
  rows rather than 1,378.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_units.py` | PyPI `pint` 0.26.2 | 973 names and 13 log scales agree, 0 mismatches |
| `scripts/fuzz/runits/gen.py` | PyPI `pint` | 4,730 sums replayed as sums and differences, 1,633 expected errors |
| `cargo test -p runits` | the crate's own suite | 17 integration tests, 1 README test and 8 doctests pass |

### Three defects the first version carried, found in review

1. **A finite logarithm outside the linear range was lost.** Converting `400 Np`
   to `dB` is `3474.36 dB`, but the first version went through the base quantity
   `e^800`, which overflows `f64` to infinity; `-400 Np` underflowed to zero and
   came back as `NotPositive`. Both scales describe the same quantity, so the
   conversion is now done in log space - `divisor * (log(value) * ln(from.base)
   + ln(from.reference) - ln(to.reference)) / ln(to.base)` - and the same helper
   backs the dimensionless-logarithm sum, so `200 Np + 200 Np` is `400 Np`. The
   positive-base check stays for a linear-to-logarithmic conversion, where the
   reference genuinely has to be positive. `tests/pint.rs::a_logarithm_outside_the_linear_range_still_converts`
   covers the positive and negative out-of-range cases, the round trip and the
   sum.
2. **The logarithmic corpus was a sample, not a corpus.** 160 pairs and the
   generator outside `scripts/fuzz/`; both are now fixed, and the magnitudes stop
   at 150 because that is the last value `pint`, the oracle, can label before it
   overflows on its own linear intermediate. The crate's behaviour above that
   range is checked by the regression test above rather than by a fixture, since
   the oracle has no answer there.
3. **The name table listed 46 names twice.** Both builders appended the same
   unsupported names, so the table held 1,066 entries for 1,020 distinct names
   and the README's name count was inflated. The two builders agree on the index,
   so merging by name changes no lookup; the count is 1,020, and the README and
   `PLAN.md` say so.

The rest of the workspace was checked too: `cargo test --workspace --all-targets`
ran 114 test binaries with no failure, and `cargo clippy --workspace
--all-targets -- -D warnings` is clean.

## Documentation

* `crates/runits/README.md`, which is the crate's `#![doc]` and therefore the
  docs.rs front page, gains a logarithmic example (`30 dBm` is one watt), a
  bullet under "What it does", and a rewritten "Scope" section that states the
  representation, the refusal of multiplication and of a prefix, and the
  dimensioned-sum refusal. The gate output and the arithmetic corpus counts are
  updated. `tests/readme.rs` executes the example.
* `PLAN.md`'s `runits` section moves the logarithmic scales from "Not covered"
  into "Covers today" and narrows the remaining gap to the Gaussian and CGS-EM
  units alone.
* `research/tools/README.md` records that the generator emits the `LOGARITHMIC`
  table and that the gate writes `pint_log_units.tsv`.
* `AGENTS.md` gains the `approx_constant` trap and the dimensioned-log-sum trap.

## Method

The crate was chosen from the round-28 gap scan, and the target was chosen
because the oracle already existed: `pint` is installed for the other two
`runits` gates, so the same library that supplies the table supplies the answer
to check it against. That is the pattern the review rounds have settled on - a
gap is worth closing when a mature implementation can be asked for the right
answer, and here it could, so the change was a representation change plus two
gate extensions rather than a domain measurement. The one place the crate
deliberately diverges from `pint` - the dimensioned-logarithm sum - is recorded
with the reason, because "matches the reference except here, and here is why" is
a stronger statement than either a silent match or a silent gap.
