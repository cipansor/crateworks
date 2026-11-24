# `fuzzy-score` 0.3.0: the numbers every competitor ships (2026-11-16)

This round asked a different question from the ones before it. Earlier rounds
asked whether a crate *reproduced* its reference. This one asked whether the
crate was **competitive**: what does a caller get from the libraries it would
otherwise reach for, that this crate does not provide at all?

## The gap

`fuzzy-score` 0.2.0 reproduces `fuzzywuzzy` exactly, quirks included, and is
checked against 2,100 scored pairs plus 2,000 query-and-choices cases. Measured
against `fuzzywuzzy`, it is not merely competitive, it is the only option: the
`fuzzywuzzy` crate on crates.io disagrees with the reference on 26 to 80 percent
of ASCII inputs and panics on non-ASCII.

Measured against the *maintained* libraries, the picture is different. `strsim`
(about 40 million downloads a quarter), `rapidfuzz` and `jellyfish` all ship two
families of function:

1. the fuzzy **score**, a policy that decides ranking, and
2. the **metric**, a number that satisfies the triangle inequality and can be
   compared across string lengths.

`fuzzy-score` shipped only the first. A caller who needed "how many edits
separate these two strings" had to add a second crate, and then reconcile two
different opinions about whether a string is a sequence of bytes or of
characters. That is the gap this round closes.

It is worth being precise about why the two families are not interchangeable.
`ratio("abc", "abd")` and `ratio("abc", "xyz")` are on one scale, but that scale
is not a distance: `ratio` does not satisfy the triangle inequality, so it cannot
be used to cluster, to index with a metric tree, or to set a threshold that means
the same thing for short and long strings. `levenshtein` can.

## What was added

`fuzzy_score::distance`:

| Function | Definition |
|---|---|
| `levenshtein` | insertions, deletions, substitutions |
| `damerau_levenshtein` | plus transposition of adjacent characters, unrestricted |
| `osa` | plus transposition, restricted to non-overlapping edits |
| `hamming` | positions that differ; unequal lengths count extra positions |
| `jaro` | similarity in `0.0..=1.0` |
| `jaro_winkler` | Jaro plus a bonus for a shared prefix, capped at four |
| `levenshtein_similarity` | a distance as a similarity |
| `damerau` | chooses between the two transposition variants |

Every function iterates `chars()`, so a multi-byte character counts once.

The two Damerau variants are both present on purpose, and this is the detail most
crates get wrong by omission. `strsim` documents `damerau_levenshtein` as the
*restricted* algorithm (OSA) while naming it after the unrestricted one;
`rapidfuzz` and `jellyfish` offer the unrestricted one. They disagree on short
inputs - `CA` against `ABC` is 2 unrestricted and 3 restricted - so a crate that
offers only one under a single name cannot be checked against the other. Both are
here, named for what they compute.

## How correctness was established

Following the rule in `AGENTS.md`: no expectation was invented. Both references
were installed, asked for the answer, and their answers recorded.

`scripts/fuzz/fuzzy-score/gen_distance.py` generates 6,094 pairs - curated
strings, random ASCII, random Unicode including astral-plane characters, and
empty strings - and asks `rapidfuzz` 3.14 **and** `jellyfish` for every metric.

The two references are cross-checked before the fixture is written. This matters:
a fixture generated from one library encodes that library's bugs as truth, and
the cross-check is what turns "we match a reference" into "we match two
independent implementations that agree". They agree on 30,468 of 30,470
comparisons, and the script asserts that the only two exceptions are the same
pair in both directions - Jaro of two empty strings - so a future release of
either library cannot widen the divergence without the regeneration failing.

`jellyfish` returns `0.0` for that pair and `rapidfuzz` returns `1.0`. This crate
returns `1.0`, with `rapidfuzz` and `strsim`: two empty strings are identical, and
a similarity of `0.0` between identical inputs is the surprising answer.

One column was deliberately *not* recorded.
`rapidfuzz`'s `JaroWinkler.normalized_similarity` is, for a similarity already in
`0..=1`, computed as `1 - (1 - s)`. That re-rounds the last bit and differs from
`s` by one ULP on 5 of the 6,094 pairs. Asserting bit-equality against it would
pin a floating-point artifact of one library's expression order rather than the
algorithm, so the fixture records the similarity and the test compares that.

## What the corpus caught, and why the order was wrong

The implementation was written first and the fixture second. That is the wrong
order, and the corpus punished it within one run: three bugs, none of which the
curated unit tests noticed.

**`hamming` counted bytes.** The length difference was `a.len().abs_diff(b.len())`
- bytes, not characters. For `""` against `"aà😀😀à😀"` it reported 17 where the
reference reports 6. Any non-ASCII input was wrong, and the curated tests used
only ASCII.

**Jaro halved transpositions as a float.** The count was divided by `2.0`, but
both references use integer division - `jellyfish` writes `trans_count //= 2`
literally. An odd transposition count therefore lost its remainder and produced a
score that was too high, on roughly half of all pairs.

**The Damerau-Levenshtein matrix was read unshifted.** The published transcription
reserves `d[0][*]` and `d[*][0]` as a sentinel so the transposition term is
unreachable at the edges, which means the cell for characters `i` and `j` is
`d[i + 1][j + 1]`. Reading `d[i - 1][j - 1]` reads the sentinel and under-reports
the distance. On `"ABC"` against `"DIXON"` it returned 4 where both references
return 5.

The third is the interesting one. It is not a typo or an off-by-one that a reader
would spot; it is an index shift that the algorithm's own sentinel makes
invisible on ordinary inputs. It only surfaces on pairs with **no characters in
common**, where the sentinel is the value that gets read. A curated test built
from examples - the kind this repository's other crates use, and which passed
here - does not contain such a pair, because a human writing test cases picks
pairs that share something.

## Mutation proof

A fixture that passes proves nothing unless it would fail. Each fix was reverted
in turn and the corpus re-run:

| Reverted fix | Cases failing |
|---|---|
| Jaro integer division | 400 / 6,094 |
| character-based `hamming` | 1,640 / 6,094 |
| Damerau sentinel index | 3,366 / 6,094 |

The Damerau mutation also fails one of the ten curated unit tests, which is the
one case where a hand-written test caught it - the divergence pair `CA`/`ABC`,
where the two Damerau variants differ. Without that case the curated suite would
have been fully green on a 3,366-case bug.

## Gates

- `cargo fmt --all -- --check`: clean
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo test --workspace --all-targets`: green, including the new
  `distance_oracle` (6,094 pairs) and 10 unit tests
- `cargo test --workspace --doc`: green, including the new README example
- MSRV 1.88 `clippy` and doctests: clean, with `RUSTUP_TOOLCHAIN=1.88.0` set so
  `rust-toolchain.toml` cannot override it
- `research/tools/verify_fuzzy_score.py`: 0 mismatches over 2,000 cases x 5
  process functions, unchanged
- `scripts/check_attribution.py`: 28 upstream projects, all named in `NOTICE`
- `scripts/check_actions_pinned.py`: clean

## State

`fuzzy-score` `0.2.0` -> `0.3.0`. New public API below `1.0` is a minor bump by
the rule in `AGENTS.md`, not a patch.

`NOTICE` names `rapidfuzz` and `jellyfish` as reference material, and
`scripts/attribution.toml` carries both with a distinctive marker, so the check
in CI enforces the credit rather than a reviewer's memory.

## What remains uncovered

- **Performance.** `rapidfuzz` uses bit-parallel Myers and SIMD; these are plain
  dynamic programs. The values agree, the throughput does not. A caller ranking
  millions of pairs should still reach for `rapidfuzz`; a caller that needs the
  same numbers without a second dependency now has them.
- **`normalized_distance` and the other derived forms.** Only the primitive and
  the one similarity are exposed. The rest are arithmetic on top and would be
  guesses about which convention to follow.
- **Unicode normalization.** `"café"` with a combining accent and with a
  precomposed `é` are different sequences and score as different. Both references
  behave the same way, so this is a property of the metric rather than a gap, but
  a caller comparing user input will want `unicode-normalization` first.
