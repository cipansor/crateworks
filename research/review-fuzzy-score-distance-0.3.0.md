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

6,094 -> 6,435 -> **7,718** pairs: 300 long pairs (20 to 60 characters), a
grapheme matrix, and the 1,283 pairs an earlier widening had silently dropped,
restored.

## Round three: five more findings, all real

A second review found five issues. All five were real; three are in the
generator rather than the crate, which is the pattern worth noticing - the
implementation was reviewed twice and the *tooling that proves the
implementation* was reviewed once.

### The `>` command emptied the fixture on failure (real)

The documented command was `gen_distance.py > tests/fixtures/distance.tsv`. The
shell truncates the target before the script runs, so the version check - which
exists precisely to refuse a wrong environment - left an empty fixture behind and
the oracle test failed until git restored it. A safety check that destroys the
thing it protects is worse than no check.

The script now writes to a temporary sibling and renames, and takes the path as
`--out`. Verified by simulating a mismatch: the run exits 1 and the fixture is
byte-identical afterwards.

### The corpus lost 1,283 pairs (real)

Widening the corpus added draws to a shared `random.seed`, which shifts every
later draw, so pairs the previous corpus had were no longer generated. Measured:
6,094 old pairs, 1,283 of them absent from the new set. That is the opposite of
what a corpus is for - it silently traded coverage for coverage.

The generator now reads the existing fixture, re-asks those pairs and merges. The
restored corpus is **7,718** pairs, a superset of both earlier versions, and the
log reports how many were carried so a shrink is visible rather than silent.

### Grapheme disagreements were accepted, not explained (real)

The classifier said "this pair contains a multi-scalar grapheme, so the
references may disagree" and accepted anything on such a pair. That passes a
genuine segmentation difference and a genuine bug identically.

It is now a proof. Both strings are reduced to one scalar per grapheme cluster -
sharing the mapping, so the reduction is an isomorphism that preserves equality
and adjacency - and `rapidfuzz` is asked again. Its answer must equal what
`jellyfish` returned. All 86 grapheme disagreements satisfy it exactly; one that
did not would fail the run. The pinned value table stays as a spot-check on the
values themselves.

### The memory guarantee was overstated (real)

The README said memory is "linear in the shorter input". It is `O(s * min(m, n))`
for the unrestricted Damerau-Levenshtein, and `s` is not bounded. Measured at
8 KiB on 64-bit:

| Distinct characters | Peak |
|---|---|
| 26 (English text) | 1 MiB |
| 2,048 | 94 MiB |
| 8,192 | 416 MiB |

I tried to tighten the bound before documenting it: snapshot only characters
occurring at least twice, since the transposition term reads the *previous*
occurrence's row. That is wrong, and the unit tests caught it immediately - `ab`
against `ba` needs row 0 through a character that occurs once. So the honest
statement is the one now in the README: the table of per-function bounds, the
measurements, and a pointer to Zhao-Sahni as the true-linear alternative that is
not implemented.

### The tooling index omitted both pins (real)

`research/tools/README.md` listed the `fuzzywuzzy` gate but not this generator,
so the two pinned oracle versions appeared nowhere a reader would look. The
generator now has an entry naming `rapidfuzz` 3.14.6 and `jellyfish` 1.3.0, and
an operational warning about `>` redirects.

## Round four: four findings, three real and one already fixed

### The wide-alphabet test proved nothing (real)

The test I added last round used a *substitution* in a wide alphabet. A
substitution scores one edit through the ordinary recurrence, so the answer was
1 whether or not the transposition term ran - which means it passed with the
snapshots deleted and with the snapshot taken after the row swap, the two
mutations it was written to catch. I confirmed both before changing anything.

The input is now an adjacent swap in an all-distinct alphabet, the one shape the
transposition term is required for: Damerau says 1, Levenshtein says 2. Both
mutations now fail it. This is the second time this file has recorded the same
lesson - a test is only evidence once it has been seen to fail - and it is now a
trap in `AGENTS.md`.

Its peak-memory assertion was also absent, as the review noted. Adding one
exposed a measurement problem worth recording: `VmHWM` is a per-process maximum,
so a delta around a single call reads zero after any larger earlier allocation,
and untouched `vec![0; n]` pages are not resident at all - a 64 MiB buffer showed
as +4 KiB. `clear_refs` resets `VmHWM` but not `VmPeak`. A counting allocator
would measure the right thing and needs `unsafe`, which this workspace forbids.
The measurement therefore lives in `tests/memory.rs`, which re-executes its own
binary so the reading comes from a fresh process. Verified in both directions:
correct code peaks at ~33 MiB against a 48 MiB ceiling, and a reintroduced full
matrix peaks at 52 MiB and fails.

### Memory bounds omitted the input copies (real)

The table claimed `O(min(m, n))` for four functions. Each of them first does
`chars().collect::<Vec<char>>()` on **both** inputs, which is `O(m + n)` at four
bytes per scalar - measurable, and larger than the row buffers for short strings.

The README now states the copies explicitly, and `hamming` is called out as the
only function that allocates nothing at all, which is what makes it the right
choice for a very long comparison that only needs a count.

While measuring this I found a real inefficiency rather than only a documentation
gap: `levenshtein` did not put the shorter string in the column role, so
`levenshtein(short, long)` allocated rows as long as the *long* argument. `osa`
already did the swap. Fixed, and measured: 112 KiB before, 12 KiB after, on an
8 KiB against 4 KiB comparison.

### The Factur-X verifier deleted before it wrote (real)

`verify_facturx_level.py` removed every `*.xml` in the corpus directory and then
wrote the replacements in place. A missing `factur-x`, a reference change or a
Ctrl-C between those two steps left the tracked fixtures destroyed with nothing
to restore them from - the same class of bug as the `>` redirect, in a script
that had no `--out` to escape to.

It now builds the whole corpus in a staging directory, writes the table there
too, and swaps both in only after every document has been written and answered
for. Verified by failing it deliberately part-way through: the table is
byte-identical afterwards and no staging directory is left behind.

### The corpus losing 1,283 pairs (not reproducible)

This finding repeats last round's, and the code no longer has the defect. The
generator reads the existing fixture before generating and merges, and I checked
the claim directly rather than trusting the comment: all 6,094 pairs from
`50a60f1` are present in the current 7,718. Nothing to fix.

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

Round three, against the 7,718-pair corpus: the same mutations fail, and the
memory test at 2,048 distinct characters fails if the snapshot is dropped or
taken after the row swap.

| Rejected change | Cases failing |
|---|---|
| Snapshot only repeated characters | 2 unit tests (the corpus is unaffected) |
| Full `m * n` matrix restored (memory test) | 1 test, 52 MiB against a 48 MiB ceiling |
| Snapshots deleted (wide-alphabet unit test) | 1 test, before the input was fixed this passed |

The Damerau mutation also fails one of the ten curated unit tests, which is the
one case where a hand-written test caught it - the divergence pair `CA`/`ABC`,
where the two Damerau variants differ. Without that case the curated suite would
have been fully green on a 3,366-case bug.

## Gates

- `cargo fmt --all -- --check`: clean
- `cargo clippy --workspace --all-targets -- -D warnings`: clean
- `cargo test --workspace --all-targets`: green, including the new
  `distance_oracle` (7,718 pairs), 15 unit tests, and a child-process
  peak-memory test
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
  `O(s * min(m, n))`, and `s` is unbounded: 8,192 distinct characters at 8 KiB
  cost 416 MiB. Zhao-Sahni gives true linear space at the cost of a more
  intricate recurrence; it is the next step if that input class matters. OSA,
  Levenshtein, Hamming and the Jaro functions are already `O(min(m, n))`.
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

## Round five: the memory gate could not tell snapshots from a matrix

Five findings. All five were real; two of them were real in a way the finding did
not say.

**The peak-memory test had a ceiling that both implementations fit.** It used
2,048 distinct characters. With `s == n` there, `s * (n + 1)` snapshot cells and
an `n * n` matrix are both about 32 MiB, so the 48 MiB ceiling accepted either.
Reintroducing a matrix now fails at 36 MiB against a 16 MiB ceiling, because the
case is 4,096 characters over a two-character alphabet: 64 KiB of snapshots
against 128 MiB of matrix. The gap is four orders of magnitude, not a factor of
1.5, which is what a ceiling needs to be worth having.

**The test needed `/proc`, and panicked without it.** Reading `VmHWM` on macOS or
Windows returned `None`, and the child asserted on it. It now returns early, so
the crate's suite passes on a platform that cannot measure instead of acquiring a
test that only passes on Linux. Verified by pointing the reader at a path that
does not exist: the test passes and prints the skip.

**The README's copy arithmetic was wrong by a factor of two.** It said an 8 KiB
comparison allocates "about 32 KiB for the copies". Two 8 KiB inputs are 8,192
scalars each at four bytes, twice: 64 KiB. The claim that the copies are
negligible is also only true for similar lengths - only the rows scale with the
shorter string, so on very unequal inputs the copies dominate. Measured rather
than asserted: `levenshtein` of 1 MiB against 16 characters peaks at 3,968 KiB,
almost all of it the two copies.

**Regeneration deleted files it did not write.** `shutil.rmtree` on the corpus
directory removed a maintainer's `notes.txt` and any subdirectory, on every
successful run, because the staging tree held only generated documents. A
maintainer investigating a reference disagreement would lose their notes exactly
when the run succeeded.

**Publication was not atomic across the table and the documents.** The table was
installed first and the corpus directory replaced second, so an interrupt between
the two left the new table naming documents that did not exist. Reproduced: the
table named 33 documents and the directory was gone.

Both are fixed by one change of shape. Each corpus now lives in
`tests/fixtures/profile/<digest>/`, named after a hash of its own documents, and
carries a `.generated` marker. The only fixed path is `profile.tsv`, so publishing
is: move the documents into their new directory, then one `os.replace` of the
table. Before that rename the old pair is consistent; after it the new pair is
consistent; there is no ordering in between. Cleanup runs last, skips any
directory without the marker, and skips the one the table now names.

Verified: a `notes.txt` and a `scratch/draft.md` beside the documents survive a
successful run; an interrupt forced at the table swap leaves 0 missing documents
and no staging tree; an unmarked `deadbeefdeadbeef/` directory is preserved; a
marked stale one is pruned. The regenerated documents are byte-identical to the
previous corpus, and all 33 cases still agree with the reference.
