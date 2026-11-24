# `mllp`: a written value was never escaped (2026-11-16)

This is a follow-up to `review-0.1.14.md`. That round re-ran every gate and found
them green. This round asked a narrower question of the same gates: *what would
each one miss?* The answer for `mllp` was a class of bug, not a missing feature.

## The defect

`ack::build` and `ack::build_enhanced` wrote every value straight into the reply.
MSA-3 is the human-readable failure text, and an interface fills it by quoting
the value that failed. When that value holds a field separator, the reply splits:

```text
MSA|AA|MSG1|bad | value \ here
```

MSA-3 is now `bad `, `value \ here` is two phantom fields, and MSA-4 has appeared
out of nowhere. The same applies to MSA-2 (the echoed control id) and to MSH-3
through MSH-6 (the applications and facilities), all of which the crate copies
out of the message it is answering - that is, all of which a peer chooses.

This is not a parse failure. `hl7apy` reads that reply without complaint and
reports `MSA-3` as `'bad '`. A receiver cannot detect it, which is what makes it
worse than a rejection: the acknowledgement silently means something the sender
never wrote.

## Why the gate did not catch it

`verify_mllp.py` built every ACK with the text `"ok"`, and no corpus message
carried a delimiter in MSH-3..MSH-6 or MSH-10. The gate checked *which* fields
were filled and never that a filled field could hold its value. That is the same
shape of gap `AGENTS.md` records for the README guard: the check existed, and the
input could not reach the code it was meant to guard.

## The fix

`message::escape` is now the inverse of `message::unescape`, exposed as
`Field::escape`. Every value the builders write goes through it, including the
ERR segment's text and the `ErrorLocation` segment id. The five standard escapes
are produced (`\F\ \S\ \R\ \T\ \E\`), which is what `hl7apy` produces when asked
to serialise a primitive holding a delimiter.

## The gate, strengthened

The corpus gains a message whose control id and facility names hold escaped
delimiters, and the driver now builds every ACK with an MSA-3 that holds all
five. `check_ack` compares MSA-3 against the escaping computed from *the reply's
own* MSH-1 and MSH-2, because a message using `#` and `*` leaves `|` and `^`
alone.

Verified by removing the fix: the gate then reports `0/9 ACK replies agree`,
with MSA-3 read as `'rejected '`. Restored, it reports `9/9`.

| Gate | Before | After |
| --- | --- | --- |
| `verify_mllp.py` fields | 135/135 | 148/148 |
| `verify_mllp.py` ACK | 8/8 | 9/9 |
| `verify_mllp.py` enhanced ACK | 8/8 | 9/9 |

## Two false alarms worth recording

Both cost a round, and both were the corpus's fault rather than the crate's - the
failure mode `AGENTS.md` warns about.

1. **A hardcoded expectation on a non-default delimiter set.** The first version
   of the new case expected `|` and `^` to be escaped. The message used `#` and
   `*`, so they are ordinary characters and must not be. The crate was right; the
   expectation assumed the default set.
2. **A malformed fixture.** The first version put a raw `~` in MSH-5. MSH-5 does
   not repeat, so the reference split it into two MSH-5 values and the crate
   looked wrong. The fixture tested the repetition split, not the escaping.

## Clippy 1.100 on untouched crates

`cargo clippy --workspace --all-targets -- -D warnings` failed on `ebics` and
`pades` before any change in this round, with `needless_range_loop`. Both walked
`0..self.limbs.len()` to index `self.limbs`; both now use
`.iter().enumerate()`, which keeps the `index` the body needs for
`other.limbs.get(index)`. Neither crate's behaviour changed.

## Documentation

* `crates/mllp/README.md` gains a section on escaping with a runnable example,
  and the crate's version moves `0.3.0` -> `0.4.0` (a new public function is a
  minor bump below `1.0`).
* `PLAN.md`'s `mllp` section says every written value is escaped and that
  `verify_mllp.py` now covers it.
* `AGENTS.md` records the escaping trap, the malformed-fixture trap and the
  clippy 1.100 sites.

## Follow-up: composite header fields

A review of the first version found that escaping every written value was right
for the primitives and wrong for the composites. MSH-3..MSH-6 are HD values and
MSH-12 is a VID, so `LAB^1.2.3^ISO` is three components. `AckContext` decoded
those fields to plain text, and `write_header` then escaped the whole string,
turning the two structural `^` into `\S\`: the reply carried one component where
the source had three. A peer matching on the universal id would no longer find
it. Every `ACK` was affected, since all five fields are written by both builders.

The cause was one string carrying two meanings. `Message::value` resolves
escapes but leaves separators, so a decoded field cannot say which character was
structure and which was data. The fix keeps them apart by never decoding:

- `AckContext` stores the five header fields as ER7, plus the `separators` they
  were read with. Only `control_id` stays plain text, because MSH-10 is an ST.
- `write_header` transcodes rather than escapes: a component separator stays a
  component separator, moved to the reply's delimiter set when it differs, and a
  delimiter that was escaped stays escaped. `message::transcode` holds that rule.

`Field::escape` was also wrong and is gone. A `Field` comes from parsing, so its
`value()` is already ER7; escaping it turned `A\F\B` into `A\E\F\E\B`, which
reads back as `A\F\B` instead of `A|B`. The encoder is now the free function
`mllp::escape`, which takes plain text, and `Field::text()` is the way to
re-emit a parsed field unchanged.

The gate had two holes of its own. `ACK_TEXT` held neither `#` nor `*`, so the
custom-delimiter case proved `|` and `^` stay literal but never that the
replacements are escaped; it now holds both sets. And the custom-delimiter row
used `|` between its header values, which put the control id in MSH-7 and left
MSH-10 empty, so the echo check was testing an absent field.

Verified by mutation: escaping the composites again makes the reference reject
the reply (`2.5.1\S\ABC` is not a version) and the gate reports 6/10 ACK and
9/10 enhanced ACK instead of crashing.

`mllp` 0.4.0 -> 0.5.0: `AckContext` gained a field, so old literals no longer
compile, and `Field::escape` was removed.

## Follow-up: locally generated values, and an unrepresentable header

A second review found two more places where the split between "came from the
wire" and "generated here" was decided wrongly, and that the gate could not see
delimiter conversion at all.

**The fallback version was treated as ER7.** When the source carries no MSH-12,
`write_header` invents `2.5.1`. That string is generated, not parsed, but it went
through `transcode`, which reads its input as shaped by the source's separators.
A source naming `.` as its component separator turned the reply's MSH-12 into
`2^5^1`, and a source naming `.` as its *field* separator split it into three
fields. Both are malformed replies. The fallback is now escaped as plain text;
only a version the context actually holds is transcoded.

**An opaque escape body can be unrepresentable.** `transcode` copied a
site-defined escape such as `\ZLAB#1\` verbatim after swapping its delimiters.
The body is the site's to define, so nothing can be inserted into it - but if it
holds one of the reply's delimiters, the reader splits the field there before it
resolves any escape. With `#` as the reply's field separator, `MSH-5` ended early
and the control id landed in MSH-9, the version in MSH-11. There is no correct
rewrite, so `transcode` now returns the offending body and the builders report
`AckError::UnrepresentableEscape`, naming the field and pointing at the way out:
answer with the original's separators, which is what `build(&ctx, ...,
message.separators())` already does and which never needs a conversion.

That made the builders fallible, so `build` and `build_enhanced` return
`Result<String, AckError>`. It is a `Result` rather than an option to keep the
reply or a silent rewrite because the caller has a decision to make and the error
carries what is needed to make it.

**The gate never converted a delimiter.** Every reply it built used
`message.separators()`, so `transcode` was exercised only along the identity
path, where it returns its input unchanged. The composite fixture proved
preservation, not conversion. The driver now also builds each reply with a
different field/component pair, chosen per message so it cannot collide with the
source's own, and `check_converted` compares the HD identities the reference
reads on each side. Comparison is by decoded component list, not by wire string:
`\F\` in the source denotes a literal `|`, which needs no escape once the reply's
field separator is `#`, so two correct renderings of one identity would fail a
text comparison.

Mutation-tested: escaping the composites again makes the converted check report
`MSH-5 is ['LAB^1.2.3^ISO'], expected ['LAB', '1.2.3', 'ISO']` and 4/11
converted replies, while the identity-set ACK check still reads 11/11 for the
enhanced form - which is exactly why conversion needed its own coverage.

Gate: 178/178 fields, 11/11 ACK, 11/11 enhanced, 11/11 converted.

`mllp` 0.5.0 -> 0.6.0: both builders return `Result`, which breaks every caller.

## Follow-up: the fallback must never refuse, and the gate was too trusting

A third review found that the previous round's fix had broken the documented
fallback, and that the gate added to prove it could not have caught the break.

**The identity path was gone.** The round-two `transcode` dropped the
`from == to` early return and ran the collision check unconditionally. A parsed
header is already valid ER7 under its own delimiters and copying it introduces no
delimiter, so `\ZID#1\` inside MSH-3 with `#` as the source's component separator
is legal - `Message::parse` accepts it - yet building a rejection with
`message.separators()` returned `UnrepresentableEscape`. The error then advised
the caller to "answer with the original's separators", which is precisely what
they had done. That is the worst shape a bug can take: the documented recovery
path refuses, and the refusal names itself as the fix.

The identity path is back, and it checks the one thing that can still be wrong
there: a raw field separator, which no parsed field can hold, so it can only come
from a hand-built `AckContext`. That case now has its own variant,
`AckError::RawFieldSeparator`, naming the field.

**Opaque bodies block conversion in both directions.** Round two refused only
when the body held a delimiter of `to`. But the *source* delimiters give such a
body its structure, because a reader splits before it resolves escapes: `hl7apy`
reads `LAB\ZID#1\` as two components, `LAB\E\ZID` and `1\E\`, when `#` is the
source's component separator. Copying that body into a reply where `#` is ordinary
data would collapse two components into one. Neither direction can be preserved,
so a delimiter from either set refuses - only conversion, never the identity path.

**The gate accepted refusals without checking they were due.** `check_converted`
treated any refusal whose text matched as success, so a crate that refused every
conversion would have scored full marks while parsing nothing. It now derives the
expectation from the fixture (`_conversion_must_refuse`) and fails in both
directions: an unexpected refusal, and a missing one where the fixture cannot be
converted.

**The gate never checked that the delimiters changed.** A reply that kept the
source's pair would agree trivially with the identity comparison. It now asserts
that `reply_ack[3]` and `reply_ack[4]` are the pair `converted_pair` chose.

The corpus gained the fixture that would have caught the round-two regression: an
opaque escape body holding the source's own component separator, which must
produce a same-delimiter rejection ACK.

Mutation-tested, five ways, each caught:

| Mutation | Reported |
| --- | --- |
| remove the identity path | driver panics: `a reply in the source's separators cannot refuse: UnrepresentableEscape { field: "MSH-5", body: "ZID#1" ... }` |
| converted reply keeps source delimiters | `cnv 0: reply uses '|'/'^', expected '#'/'*' - nothing was converted` |
| refuse every conversion | `cnv 0: refused, but this fixture can be converted to '#'/'*'` |
| refuse only when `body.is_empty()` | `cnv 11: expected a refusal ... but a reply was produced` |
| driver panic | reported as `the driver exited 101; the first failing case is named in the panic above`, not a traceback |

Gate: 191/191 fields, 12/12 ACK, 12/12 enhanced, 12/12 converted.

`mllp` 0.6.0 -> 0.7.0: `AckError` gained a variant. The builders' signatures are
unchanged from 0.6.0, so this is additive for callers that already handle the
`Result`.
