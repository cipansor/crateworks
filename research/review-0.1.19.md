# Review of every crate against its mature comparison package (2026-11-16)

This round develops `cssprint`, the HTML and CSS layout engine checked against
WeasyPrint. It widens the selector engine from descendant and child selectors to
the rest of what a stylesheet actually uses, and - the part that matters - grows
the oracle corpus in the same change so the new grammar is checked against the
reference rather than against a hand-written expectation.

## What this round develops

### `cssprint` 0.3.0 -> 0.4.0, the rest of the selector engine

The crate could reach a descendant and a direct child. That is enough to lay out
a page and not enough to style one: almost every real stylesheet tests an
attribute (`input[type="text"]`, `a[href^="https"]`), a structural position
(`li:first-child`, `tr:nth-child(odd)`) or a sibling (`h1 + p`, `h1 ~ p`). A
crate that parses those selectors and then silently matches nothing is worse than
one that rejects them, because the page lays out without the rule and nothing
says so.

The grammar now covers:

* the four combinators - descendant, child (`>`), adjacent sibling (`+`) and
  general sibling (`~`) - so a chain may step sideways as well as down;
* the attribute selectors `[a]`, `[a=v]`, `[a~=v]`, `[a|=v]`, `[a^=v]`,
  `[a$=v]` and `[a*=v]`, with a valueless attribute (`<input disabled>`) kept
  distinct from one set to the empty string;
* the structural pseudo-classes `:root`, `:first-child`, `:last-child`,
  `:only-child`, `:nth-child()` and `:nth-last-child()`, with `odd`, `even`, a
  bare index and an `an+b` argument;
* `:not()` over a single compound, as level 3 allows.

Matching moved from "walk the ancestors" to a `MatchContext` that carries the
element's parent, its 0-based position among the parent's element children and
the count, so a structural pseudo-class and a sibling combinator read the same
context. A sibling step reuses the context with a different index, because the
siblings share the parent. Backtracking is unchanged: the chain is still matched
right to left, and a step that satisfies one compound but strands the next backs
up, which is what makes `div > p span` work when the nearest `p` has no `div`
above it.

Specificity grew with the grammar. An attribute selector and a structural
pseudo-class each count as a class, and `:not()` counts as its argument rather
than for itself, which is the level-3 rule the reference follows.

The DOM gained an `attributes: Vec<(String, Option<String>)>` field, populated
by the HTML parser from the same attribute string that already produced `id`,
`classes` and `style`. Those three are the raw attributes unpacked, kept because
the layout reads them far more often than it reads a raw attribute; the new field
is what an attribute selector reads. A valueless attribute is kept with a `None`
value so `[disabled]` matches it.

**The traps this surfaced, now in `AGENTS.md`.** Four parser bugs that the new
grammar exposed, each of which a curated case would have missed:

* The compound-name boundary `ends_name` has to stop at `[`, `:` and the sibling
  combinators, not only at whitespace and `>`, or `div[a]` is read as the tag
  `div[a]`.
* `read_parenthesized` has to count depth, because `:not(:nth-child(2n+1))` nests
  a `)` inside the `:not()` argument. The random corpus drew this on its very
  first case; the curated cases did not.
* An attribute name ends at the operator sigil (`~ | ^ $ *`), not only at `=`, or
  `[data-x~=v]` reads the name as `data-x~` and never matches.
* The universal selector `*` is a whole compound, so `parse_compound` must accept
  and skip a leading `*` before it reads a name, or the existing `*` tests fail.

## The gate that covers it

The selector corpus is the oracle, and it grew with the grammar. Every case is
laid out by WeasyPrint and recorded as a box tree; `tests/layout.rs` lays out the
same document and compares the two box by box. A rule whose subject may be
inline carries `font-size`, an inherited property that moves the text, because a
box-model declaration on a non-replaced inline is ignored and would hide a broken
match.

The generator, `scripts/fuzz/cssprint/gen.py`, now draws attribute selectors
(every operator, over a small overlapping value pool), the structural
pseudo-classes, `:not()` and the sibling combinators, and writes attributes onto
the elements it builds. A `--seed` option was added so a fresh seed can be
explored; the committed corpus keeps the fixed seed, and the four curated
root-pseudo-class cases pin that `html` is a child of the page box, so
`:first-child` and `:only-child` match it as the reference treats it.

| Gate | Reference | Result |
| --- | --- | --- |
| `cargo test -p cssprint` | the crate's own suite | 32 unit, 4 layout-corpus, 11 README and 5 doctest tests pass |
| `tests/fixtures/selector_layout.tsv` | WeasyPrint 70.0 | 52 curated cases, 304 boxes, all equal |
| `tests/fixtures/selector_fuzz.tsv` | WeasyPrint 70.0 | 1200 random cases, 13795 boxes, all equal |
| `tests/fixtures/layout.tsv` | WeasyPrint 70.0 | 87 curated cases, 645 boxes, all equal |
| `tests/fixtures/fuzz.tsv` | WeasyPrint 70.0 | 2000 random cases, 235805 boxes, all equal |
| `cargo clippy -p cssprint --all-targets -- -D warnings` | the workspace lint set | clean |

The random selector corpus was also run against four fresh seeds (7, 12345,
424242, 999983) at 1500-2000 cases each, and the crate agreed with WeasyPrint on
every box, so the agreement is not an artefact of the committed seed.

## The code review of this round, and what it turned up

The widened grammar was reviewed before merge, and the review found seven
defects, all in the code this round added. Each is fixed, and the selector corpus
grew to hold a case for it, so the fix is checked against WeasyPrint rather than
against the review's own description:

* **A quadratic match on flat documents.** `MatchContext::of` counted a parent's
  element children and searched them for the subject on every rule evaluation, so
  a 1000-child list with one `div` rule cost about 1.5M sibling visits. The
  position and count are now computed once by the traversal that already walks
  the parent and threaded down as `Ancestor { element, index, total }`. On an
  8000-child document the 4x-scaling ratio fell from 15.2 (superlinear) to 3.5
  (linear).
* **An `an+b` overflow.** `:nth-child(-n-9223372036854775807)` made `index - b`
  overflow `i64` - a panic in debug, and in release a wrapped value that then
  panicked on `% -1`. The arithmetic is now `i128`, where the difference of two
  `i64`s is exact.
* **Quoted strings read as syntax.** `[title="Doe, Jane"]` split on the comma,
  `[title="a]b"]` closed at the quoted `]`, and `:not([title=")"])` closed at the
  quoted `)`. The comment stripper, the rule-brace finder, the selector-list
  splitter, the attribute-close finder and `read_parenthesized` now step over a
  quoted string (honouring `\`), sharing `string_length`, `find_unquoted` and
  `split_unquoted`. A quoted value's escapes are decoded (`\62` is `b`) before
  the comparison.
* **Chained negations rejected.** `div:not(.a):not(.b)` failed to parse because a
  compound held one `Option<Compound>` negation. It holds a `Vec` now, every
  argument must fail to match, and their specificities add.
* **Upper-case pseudo-classes rejected.** `:FIRST-CHILD` was reported as
  unsupported. The parser now lower-cases the pseudo-class name only; the case of
  a negation argument's class or id stays significant.
* **Unicode spaces split attribute words.** `[data-x~="b"]` matched a value
  holding U+00A0, because `split_whitespace` treats it as a separator. CSS word
  matching is ASCII, so the match now uses `split_ascii_whitespace`.
* **A generator seed mismatch.** With a non-default `--seed`, the layout fixture
  rendered default-seed documents (`all_cases(args.random)` never received the
  seed), so the generated inputs and the expected trees described different
  documents. `all_cases` now takes the seed and `main` passes it.

The last one is worth its own note: it is a bug in the generator, not the crate,
and it would have produced a fixture that disagreed with its own inputs the first
time someone explored a fresh seed - exactly the "a generator is code, and its
bugs look exactly like crate bugs" trap.

## Documentation

* `crates/cssprint/README.md` describes the widened grammar, gains a runnable
  example that uses a sibling combinator and an attribute selector, and its
  "What it does not do" list now names the non-structural pseudo-classes and the
  level-4 `:not()` selector list rather than the grammar that is now supported.
* `PLAN.md`'s `cssprint` section records the new coverage under "Covers today"
  and corrects the random corpus count to 2000.
* `scripts/fuzz/README.md` records the selector corpus and the fresh-seed option.
* `AGENTS.md` gains the four parser traps above and the rule that the selector
  engine and its corpus grow together.

## Method

The target was chosen so the oracle already existed: WeasyPrint defines the
behaviour, the crate's generator already reproduced its box tree, and the new
grammar is more of the same measurement. The work was therefore less "measure a
domain" and more "widen the grammar, widen the corpus, and read the diff the
corpus produces" - and the diff produced four real parser bugs the curated cases
passed straight over, which is the argument for growing the corpus with the code
rather than after it.
