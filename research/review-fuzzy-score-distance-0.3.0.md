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

## Round two: five review findings

A review of the first round raised five issues. Four were real and one was a
documentation gap; all five are fixed here, and each was verified against the
references before anything changed.

### The memory blowup (real, and the worst of them)

Both algorithms allocated a `usize` per pair of positions. Measured, not
estimated: an 8,192-character pair took the process from 2 MiB to 514 MiB, so a
caller comparing two descriptions of ordinary length could have the process
aborted by the allocator. The signature returns `usize` and has no way to report
that.

- **OSA** reads only the previous two rows, so three rolling rows suffice.
  Measured after the change: 1 MiB for the same input.
- **Unrestricted Damerau-Levenshtein** reaches into an arbitrary earlier row for
  its transposition term, so rolling rows cannot work - the review was right
  about that. What does work is saving one copy of the row per distinct character,
  taken when that character's last occurrence is processed, at which point the
  row it needs is the previous one. That is `O(s * min(m, n))` in the number of
  shared distinct characters, which for text is small and for adversarial input
  is not. It is documented as such rather than called linear.

Verified two ways: the 6,435-pair corpus, and 300 extra pairs of 20 to 60
characters against `rapidfuzz` directly - the corpus alone was too short to
distinguish a correct snapshot from a subtly wrong one. A mutation that snapshots
the row after the swap instead of before fails 384 cases, so the corpus does
cover the mechanism.

### The unit is the scalar, and the docs now say so (real)

`jellyfish` 1.3 compares extended grapheme clusters. `rapidfuzz`, `strsim` and
this crate compare Unicode scalar values. On `"e"` against `"e"` + U+0301 the
Jaro similarity is `0.833` here and `0.0` in `jellyfish`; on two joined emoji the
Levenshtein distance is 2 here and 1 there.

The first round's docs said "characters", which is ambiguous and, on this
question, wrong. They now say scalar values, state that the module is not
jellyfish-compatible for multi-scalar text, give the measured pairs, and tell a
caller comparing user input to normalize to NFC first. The corpus contains the
divergent pairs and the generator pins both reference values for them, so the
divergence is measured rather than skipped.

### `hamming` has a different contract from `strsim` (real)

`strsim::hamming("ab", "abc")` is `Err(DifferentLengthArgs)`; this is `1`. Both
conventions exist. The function docs now name `strsim` explicitly, state which
convention is followed and why, and note that a caller requiring the strict
contract needs one length comparison.

### The generator was unpinned and under-asserted (real)

Two gaps, both fixed:

- Neither oracle version was checked, so regeneration silently depended on
  whatever was installed. Both are now pinned and verified, and the script
  refuses to run otherwise. This matters concretely: `jellyfish` changed its unit
  in 1.2, so an unpinned regeneration would have produced a different fixture.
- The empty-string assertion only required the disagreement to be *somewhere* in
  the empty-string rows, so a reference could have changed to agree and the
  script would still pass. All four comparisons - Jaro and Jaro-Winkler, in both
  references - are now asserted value-for-value.

### Corpus growth

6,094 -> 6,435 pairs: 300 long pairs (20 to 60 characters) and a grapheme
matrix, both of which the review's findings required to be testable at all.

## Mutation proof

A fixture that passes proves nothing unless it would fail. Each fix was reverted
in turn and the corpus re-run:

| Reverted fix | Cases failing |
|---|---|
| Jaro integer division | 400 / 6,094 |
| character-based `hamming` | 1,640 / 6,094 |
| Damerau sentinel index | 3,366 / 6,094 |

Round two, against the 6,435-pair corpus:

| Reverted fix | Cases failing |
|---|---|
| Jaro integer division | 498 / 6,435 |
| Damerau snapshot taken after the row swap | 384 / 6,435 |

The Damerau mutation also fails one of the ten curated unit tests, which is the
one case where a hand-written test caught it - the divergence pair `CA`/`ABC`,
where the two Damerau variants differ. Without that case the curated suite would
have been fully green on a 3,366-case bug.

## Gates

- `cargo fmt --all -- --check`: clean
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo test --workspace --all-targets`: green, including the new
  `distance_oracle` (6,435 pairs) and 14 unit tests
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
- **Adversarial memory for Damerau-Levenshtein.** The snapshot approach is
  `O(s * min(m, n))`, so a string of thousands of distinct characters still costs
  a large allocation. Zhao-Sahni gives true linear space at the cost of a more
  intricate recurrence; it is the next step if that input class matters. OSA, the
  Levenshtein and the Jaro functions are already linear.
- **`normalized_distance` and the other derived forms.** Only the primitive and
  the one similarity are exposed. The rest are arithmetic on top and would be
  guesses about which convention to follow.
- **Unicode normalization.** `"café"` with a combining accent and with a
  precomposed `é` are different sequences and score as different - two edits
  apart, not zero. `rapidfuzz` and `strsim` behave the same way, so this is a
  property of scalar comparison rather than a gap, but a caller comparing user
  input will want `unicode-normalization` first. The crate does not depend on it
  and does not normalize silently, because a silent normalization would make the
  function stop matching the reference it is checked against.
