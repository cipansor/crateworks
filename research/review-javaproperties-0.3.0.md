# `javaproperties` 0.3.0: closing the escaping-mode gap (2026-11-16)

## What was asked

Extend another crate so it can compete with the package it is measured against,
on a new branch with its own pull request. The previous round closed a review of
`holidays-rs`; this round picks a crate with a gap that can be *proved* against
its reference rather than argued about.

## Why `javaproperties`

Its reference is PyPI `javaproperties` 0.8.2, the crate's own named comparison
package, and it was already installed for the writer corpus. That makes the
comparison cheap and exact: the reference can be asked for the answer, so no
expectation has to be invented.

The crate covered the format completely and the writer's `separator`,
`comments`, `timestamp`, `sort_keys` and `ensure_ascii` options. It did not cover
the reference's **second, independent** escaping control, nor the `ensure_ascii`
argument the reference puts on its three standalone helpers. Four functions were
missing or narrowed:

| Reference | Crate before | Now |
| --- | --- | --- |
| `escape(field, ensure_ascii=True)` | `escape(field)` | `escape` + `escape_with` |
| `join_key_value(k, v, sep, ensure_ascii=True)` | three arguments | `join_key_value` + `join_key_value_with` |
| `to_comment(comment, ensure_ascii=None)` | `to_comment(comment)` | `to_comment` + `to_comment_with` |
| `dumps(..., ensure_ascii_comments=None)` | absent | `WriteOptions::ensure_ascii_comments` |

This matters beyond completeness: a caller who wants to write a UTF-8 properties
file with a readable accented comment - the common case for a file read by a
Spring service that is not stuck on ISO-8859-1 - had no way to ask for it. The
existing `ensure_ascii: false` escapes the entries but, before this change, had
no effect on the comment.

## The rule the crate was missing

`ensure_ascii` is not one switch but two, and the comment one has three
positions rather than two. Measured against 0.8.2:

```
to_comment("café",       None) -> "#café"          # Latin-1 written through
to_comment("café",       True) -> "#caf\\u00e9"    # escaped
to_comment("café",      False) -> "#café"          # nothing escaped
to_comment("中文",       None) -> "#\\u4e2d\\u6587" # above Latin-1, escaped
to_comment("中文",      False) -> "#中文"
```

`None` is the reference's default and escapes exactly what ISO-8859-1 cannot
hold. It is **not** the same as following `ensure_ascii`, and it is not the same
as `Some(false)`. That is why the new argument is `Option<bool>`: a `bool` would
have made the default unrepresentable and quietly changed the output of every
existing caller. A two-valued test would have passed with the wrong default, so
the fixture spells the mode out as a `none` / `1` / `0` column.

## How it is checked

`scripts/fuzz/javaproperties/gen_writer.py` was extended to record the new modes
and regenerated. Nothing was hand-written; every expectation is the reference's
own output, normalised only for the reference's lower-case hex (Java writes upper
case, and the crate follows Java).

New corpora:

| Fixture | Cases | Covers |
| --- | --- | --- |
| `escape-modes.tsv` | 56 | `escape_with` in both modes |
| `join-modes.tsv` | 120 | `join_key_value_with` in both modes |
| `comment-modes.tsv` | 51 | `to_comment_with` in all three modes |
| `comment-modes-random.tsv` | 6,000 | the same over generated strings |
| `escape-modes-random.tsv` | 4,000 | `escape_with` over generated strings |
| `join-modes-random.tsv` | 4,000 | `join_key_value_with` over generated strings |
| `dumps-comment-modes.tsv` | 60 | `WriteOptions::ensure_ascii_comments` |

The last two were added after review pointed out that the standalone modes had
random coverage only through the default wrappers: `escape-random.tsv` and
`join-random.tsv` call `escape` and `join_key_value`, so a `_with` that ignored
its argument would have passed them. Mutating `escape_with` to force
`ensure_ascii = true` now fails 1,226 of 4,056 cases; against the curated file
alone it would have been 18.

The existing `dumps.tsv` and `dumps-random.tsv` gained one column for the new
option. Regenerating them was checked rather than assumed: after the change,
removing column 6 from each new row reproduces the old row exactly, for all 801
and 2,001 rows. No other fixture moved.

The random block was appended at the **end** of the generator on purpose. The
generator seeds `random` once and every corpus draws from that stream, so a new
`piece()` call placed before an existing corpus silently regenerates it. Placing
it last is what kept `unescape-random.tsv` byte-identical; the first attempt did
not, and the diff showed it.

### The tests were shown to fail

A test that cannot fail is not evidence. Both new rules were mutated in turn:

| Mutation | Result |
| --- | --- |
| default comment mode `> 0xff` changed to `> 0x7f` | 933 / 2,017 comment, 223 / 2,860 dumps, 939 / 6,051 comment-mode cases fail |
| `ensure_ascii_comments` ignored, `to_comment` called instead | 8 / 2,860 dumps cases fail |

Both were reverted and the suite is green.

## Result

`cargo fmt --all -- --check`, `cargo clippy --workspace --all-targets -- -D
warnings` and `cargo test --workspace --all-targets` all pass. Clippy was forced
to rebuild (`find crates/javaproperties -name '*.rs' -exec touch {} +`) before the
result was believed, because a cached run prints nothing.

Version `0.2.0 -> 0.3.0`, a **minor** bump: it adds public functions *and* adds a
field to the public `WriteOptions` struct, which breaks a caller using a struct
literal without `..Default::default()`. Below 1.0 that is the same bump, and
`scripts/bump.py javaproperties minor` is what computed it. Nothing depends on
`javaproperties`, so no other crate needs a release with it.

## What is still not covered

Nothing in the reference's writer surface: `dump`, `dumps`, `escape`,
`join_key_value`, `to_comment`, `java_timestamp` and `unescape` are all present,
and the two the crate narrows deliberately are the timestamp (UTC, because the
crate takes no timezone database) and the reader's handling of a lone surrogate
(U+FFFD, because Rust text cannot hold one). Both differences are recorded in the
fixtures and in the README rather than left to be discovered.

The stream writer carries the options too: `store_to_with` takes a
`WriteOptions` and writes exactly what `store_with` returns, so a caller who
needs a separator, sorting or a comment mode does not have to render the text and
write it by hand. `store_to` stays as the two-argument convenience over
`store_bytes`.

## Review round two

Three gaps were found reviewing the first round, and all three were real:

- **The standalone modes had no random coverage.** Fixed by
  `escape-modes-random.tsv` and `join-modes-random.tsv`, as described above.
- **`java_case` could rewrite literal comment text.** It upper-cases `\uXXXX`
  hex to match what Java writes, which is sound for keys and values because the
  reference doubles a backslash there, so a `\u` in the output is always an
  escape. A comment never escapes a backslash: `to_comment("a\\u00e9b")` returns
  `#a\u00e9b` unchanged, and the rewrite turned that into `#a\u00E9b`. The
  corpora happened to use digits-only hex, where upper-casing is a no-op, so no
  fixture was wrong — but the generator was. `_check_literal_escapes` now rejects
  any curated or token string whose literal `\uXXXX` has a non-digit, so the
  hazard cannot come back silently.
- **`store_to` could not select a comment mode.** Fixed by `store_to_with`.

The third one was found by reading the code, not by a failing test, which is why
the review listed it: no fixture exercised the stream writer at all.

## Review round three

Three more findings, all real:

- **The guard could be bypassed by the random corpora.** `_check_literal_escapes`
  ran over the curated lists only, so a literal `\u00e9` reaching a comment
  through `piece()` or the separate document cases was still upper-cased. The
  check now runs on every input path: inside `piece()` (which every random string
  is built from), inside `dumps_row` for both the comment and the pairs, over the
  curated lists, and in `comment_java`, which is now the only way a `to_comment`
  call is made. Injecting `"\\u00e9"` into `TOKENS` stops the run instead of
  producing a corrupt fixture.

  The first attempt at this was wrong and the fixtures caught it. Making
  `java_case` skip a `\u` that followed an escaped backslash looked right and
  broke genuine escapes: a comment never doubles a backslash, so a caller's `\`
  followed by an escaped emoji renders as `\\uD83D\uDE00`, and the parity rule
  read the second backslash as the escape's, leaving `\uD83D` lower-case. Three
  fixtures changed, the diff showed lower-case hex, and the parity rule was
  reverted. Guarding the input is the fix; guessing which `\u` is real is not.

- **The generator never checked the oracle's version.** `AGENTS.md` requires it
  for any generator that reads an oracle, and `holidays-rs/gen.py` is the
  precedent. `gen_writer.py` now asserts `PINNED = "0.8.2"` and stops otherwise,
  so a fixture cannot be re-baselined against another release silently.

- **The byte writers emitted UTF-8, not Java's ISO-8859-1.** `store_bytes` was
  `store().into_bytes()`, so it always encoded UTF-8, while its documentation
  promised "the way Java writes a stream" and bytes that are "also valid
  ISO-8859-1". The gap is invisible in the case the docs reasoned about, because
  `ensure_ascii: true` escapes every entry character to ASCII. It is not
  invisible in the default case, which is the one that matters: the comment mode
  is a *separate* switch whose default (`None`) passes Latin-1 through, so
  `store_bytes(Some("café"))` emitted `#caf\xC3\xA9` where Java emits
  `#caf\xE9`. A reader that keeps comments and decodes ISO-8859-1 saw `cafÃ©`.

  The first attempt only documented the divergence, which was wrong: the
  function claimed a guarantee it did not meet, and a `store_to` caller had no
  way to obtain Java's bytes at all. Both byte writers now encode ISO-8859-1
  through one `latin1` helper, and the reference confirms the expected bytes:

  ```
  javaproperties.dump({'café': 'naïve'}, buf, comments='café', timestamp=False)
  text.encode('iso-8859-1')  # b'#caf\xe9\ncaf\\u00e9=na\\u00efve\n'
  ```

  `store_bytes` and `store_to` stay infallible, because the default options are
  exactly the two that keep every character inside ISO-8859-1, so the `latin1`
  call there cannot fail; the `expect` states that invariant. `store_to_with`
  takes arbitrary options, so it can fail, and a character above `U+00FF`
  reaching the encoder is `Error::NotLatin1` rather than a silent transcode that
  would read back as different text. Both settings that can let such a character
  through (`ensure_ascii: false`, `ensure_ascii_comments: Some(false)`) are
  covered by a test, and so is the `café` comment being one byte.

Regenerating after the guard change left every fixture byte-identical, which is
the check that the guard rejects nothing legitimate.


