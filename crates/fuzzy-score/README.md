# fuzzy-score

[![crates.io](https://img.shields.io/crates/v/fuzzy-score.svg)](https://crates.io/crates/fuzzy-score)
[![docs.rs](https://docs.rs/fuzzy-score/badge.svg)](https://docs.rs/fuzzy-score)

Fuzzy string matching that gives the same scores as Python's `fuzzywuzzy`.
`"mariners vs angels"` and `"los angeles angels at seattle mariners"` score 86,
`"new york mets"` and `"the wonderful new york mets"` score 90, and both numbers
are the ones a Python service using `fuzzywuzzy` would report for the same
inputs.

That compatibility is the point. If a Python service already ranks records with
`fuzzywuzzy` - or its maintained fork `thefuzz` - this crate lets the same
ranking run in Rust, or alongside it, without the order changing. A fuzzy match
score is a policy: it decides which customer a payment is applied to, or whether
two product records are the same thing. Re-tuning it silently when a service is
moved is the kind of change nobody notices until a ledger is wrong.

## Example

```rust
use fuzzy_score::{ratio, partial_ratio, token_set_ratio, w_ratio};

// The raw character similarity. Punctuation counts.
assert_eq!(ratio("this is a test", "this is a test!"), 97);

// One string is a fragment of the other.
assert_eq!(partial_ratio("hello world", "hello"), 100);
assert_eq!(ratio("hello world", "hello"), 62);

// Word order does not matter.
assert_eq!(token_set_ratio("new york mets vs atlanta braves",
                           "atlanta braves vs new york mets"), 100);

// A partial match is scaled down, so 100 still means a full match.
assert_eq!(w_ratio("new york mets", "the wonderful new york mets"), 90);
```

## Choosing a scorer

| Function | What it ignores | Use it when |
|---|---|---|
| `ratio` | nothing | the strings should be the same length |
| `partial_ratio` | everything outside the best substring | one string may be a fragment of the other |
| `token_sort_ratio` | word order | the same words may appear in a different order |
| `token_set_ratio` | word order and extra words | one string may carry extra words |
| `q_ratio` | case, punctuation, leading and trailing space | you want `ratio` after cleaning up |
| `w_ratio` | case, punctuation, and the above, taking the best of them | you do not know which of these applies |

The differences are not cosmetic. The same pair of strings can score 29, 61, 91
or 86 depending on which you pick:

```rust
use fuzzy_score::{ratio, token_set_ratio, token_sort_ratio, w_ratio};

let a = "mariners vs angels";
let b = "los angeles angels at seattle mariners";

assert_eq!(ratio(a, b), 29);
assert_eq!(token_sort_ratio(a, b), 61);
assert_eq!(token_set_ratio(a, b), 91);
assert_eq!(w_ratio(a, b), 86);
```

`w_ratio` is the default for a reason: it tries the others and keeps the best,
with the partial scores scaled so that only a full match reaches 100.

## Choosing from a list

`extract` and `extract_one` rank a query against many choices. Both take a
`Scorer`, which names the function to rank by.

```rust
use fuzzy_score::{Scorer, extract, extract_one};

let teams = [
    "new york mets vs chicago cubs",
    "chicago cubs vs chicago white sox",
    "philadelphia phillies vs atlanta braves",
    "braves vs mets",
];

// The single best match, or None if nothing reaches the cutoff.
let best = extract_one("new york mets at atlanta braves", teams, 0, Scorer::WRatio);
assert_eq!(best.unwrap().0, "braves vs mets");

// Or every match above the cutoff, best first.
let all = extract("new york mets at atlanta braves", teams, Some(2), Scorer::WRatio);
assert_eq!(all.len(), 2);
assert!(all[0].1 >= all[1].1);

// A cutoff keeps a poor match from being returned at all. Here the query has
// nothing to do with any choice, so without a cutoff the best of a bad lot
// would still come back.
let unrelated = [
    "new york mets vs chicago cubs",
    "chicago cubs at new york mets",
    "atlanta braves vs pittsbugh pirates",
    "new york yankees vs boston red sox",
];
let query = "los angeles dodgers vs san francisco giants";
assert!(extract_one(query, unrelated, 50, Scorer::WRatio).is_none());
```

`extract_bests` is `extract` with a cutoff, and `extract_without_order` is the
unranked form underneath both - it keeps the input order and only drops what
falls below the cutoff. Both mirror the `fuzzywuzzy.process` functions of the
same names.

```rust
use fuzzy_score::{Scorer, extract_bests, extract_without_order};

let teams = ["braves vs mets", "new york mets vs chicago cubs"];

// Only matches at or above the cutoff, best first.
let ranked = extract_bests("braves vs mets", teams, 40, Some(2), Scorer::WRatio);
assert_eq!(ranked[0].0, "braves vs mets");

// Input order kept, cutoff applied.
let raw = extract_without_order("braves vs mets", teams, 40, Scorer::WRatio);
assert_eq!(raw[0].0, "braves vs mets");
assert_eq!(raw[0].1, 100);
```

`dedupe` removes near-duplicates, keeping the longest spelling of each, on the
grounds that it carries the most information.

```rust
use fuzzy_score::dedupe;

let names = ["Frodo Baggin", "Frodo Baggins", "F. Baggins", "Samwise G.", "Gandalf"];
let unique = dedupe(names, 70);
assert!(unique.contains(&"Frodo Baggins".to_string()));
```

## What it does not do

It does not give the same numbers as `rapidfuzz`, and it is not meant to.
`rapidfuzz` computes several of the same metrics with its own formulations and
is much faster; `fuzzywuzzy` computes them with `difflib.SequenceMatcher`. The
two disagree on ordinary inputs, so pick one and stay with it.

It does not stem, transliterate or spell-correct. `ratio("Cães", "Caes")` is 75,
because `ã` is dropped rather than folded to `a`. See below.

## Getting the same answer as Python

Two details of `fuzzywuzzy` are surprising enough that an implementation written
from the documentation alone will get them wrong. Both are reproduced here.

The underscore is a word character. Python's `\w` includes it, so `"a_b"` stays
`"a_b"` rather than becoming `"a b"`, and `ratio("a_b", "a b")` is 67.

Folding to ASCII removes only `U+0080..=U+00FF`. This is what
`fuzzywuzzy.utils.asciidammit` does, despite the name: `"Á"` is dropped, but
`"中"` is kept. Accented Latin letters are therefore lost rather than
transliterated, which is why `ratio("Cães", "Caes")` is 75 and not 100. Use the
`u`-prefixed functions - `uw_ratio`, `uq_ratio` - to keep them.

## Why not the existing Rust crate

There is already a `fuzzywuzzy` crate on crates.io, last released in 2020 and
still pulled about 225,000 times in the last 90 days. It is not a drop-in
replacement, and the gap is not one of degree.

Run against this crate's own fixture - 2,100 pairs whose scores come from
`fuzzywuzzy` 0.18 - it disagrees on:

| Function | Pure-ASCII inputs (1,101 cases) |
|---|---|
| `ratio` | 28.2% |
| `partial_ratio` | 25.6% |
| `token_sort_ratio` | 67.5% |
| `token_set_ratio` | 79.9% |
| `w_ratio` | 65.2% |

On the 999 cases containing non-ASCII text it does not merely score
differently: it **panics** on 924 of them, with `end byte index 8 is not a char
boundary`. It indexes strings by byte where Python indexes by character, so any
input with a multi-byte character can abort the process. The remaining 75 cases
all score differently.

## How it is checked

Two ways, and the crate is not considered correct without both.

The upstream test suite from `fuzzywuzzy` 0.18 is transcribed in
`tests/upstream.rs`, each test named after the one it came from. A failure there
means a divergence from the reference, not from someone's opinion.

`tests/oracle.rs` replays 2,100 input pairs whose scores were produced by
`fuzzywuzzy` itself. `research/tools/verify_fuzzy_score.py` is the regression
gate: it runs the crate and the reference over a generated corpus and exits
non-zero on any difference. The gate covers the ten `fuzz` scorers *and* the
five `process` functions (`extract`, `extract_bests`, `extract_one`,
`extract_without_order`, `dedupe`) over a second corpus of 2,000
query-and-choices cases.

That second corpus found a divergence the scorer corpus could not: the
reference runs the *query* through `full_process` twice - once as the default
processor, then again as the scorer's pre-processor - but each *choice* only
once, because its `processor` is swapped for a no-op before the loop. The two
passes are not the same as one when `force_ascii` is on: the first pass keeps
Unicode letters and only the second folds them away, so
`full_process(full_process(q, false), true)` can differ from
`full_process(q, true)`. The crate had processed both sides once, which scored
`60` where the reference scored `44` on a query of mostly non-ASCII letters. It
now processes the query twice and the choices once, matching the reference.

The same corpus exposed a reference crash. `fuzzywuzzy.process.dedupe` indexes
into an empty list, and raises `IndexError`, when a choice preprocesses to the
empty string: it scores zero against everything including itself, so nothing
clears the threshold. There is no reference answer for those cases, so the gate
skips them for `dedupe` and counts them; the crate keeps such a choice unchanged
rather than panicking.

One caution about the reference. When the `Levenshtein` package is installed,
`fuzzywuzzy` silently swaps in a `Levenshtein`-backed matcher that returns
*different* ratios - 36 where `difflib` gives 24 on a two-character case. Any
comparison made in that state is meaningless, so the gate refuses to run unless
the pure-Python `difflib.SequenceMatcher` is in use. Uninstall
`python-Levenshtein` before trusting a result.

## Licence

Part of the [crateworks](https://github.com/cipansor/crateworks) workspace.
Licensed under the Apache License, Version 2.0, copyright Yayasan Pesantren
Cipansor and contributors. The scores reproduce those of
[fuzzywuzzy](https://github.com/seatgeek/fuzzywuzzy) by SeatGeek (MIT); see
`NOTICE`.
