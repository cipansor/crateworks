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

## Follow-up: validate the value, then judge the roles

A fourth review found that the round-three rule asked the wrong question. It
checked which *characters* appeared in an opaque body; what decides whether a
reply can carry the body is which *role* each character plays.

**The verdict depended on the reply.** The raw field separator check sat inside
the `from == to` branch, so a hand-built context holding `LAB|1` was refused with
default reply delimiters and accepted with `#` as the reply's field separator.
That inverts the order of the argument: the value is invalid before a reply
exists, so no reply delimiter set may make it acceptable. The check now runs
first, and a test asserts the same error for four different reply sets.

**A field-only change was refused for no reason.** The round-three rule refused
an opaque body containing *any* delimiter from either set. But a body holding the
source's component separator survives a conversion that changes only the field
separator: the component boundaries do not move. `hl7apy` reads `LAB\ZID#1\` as
`['LAB\E\ZID', '1\E\']` in both the source and such a reply, so the reply carries
it byte for byte. The rule is now a role comparison per character: a separator
that becomes data stops splitting the field, and data that becomes a separator
starts splitting it, so both are refused; a role that is unchanged is not.

**The message blamed the wrong message.** `UnrepresentableEscape` said the body
"contains a delimiter of the reply", which is false when the collision is with the
*source* component separator. It now carries the offending character and both
roles, and renders as `'#' , a component separator in the message but a field
separator in the reply`.

**The gate could traceback.** `check_converted` indexed `reply_ack[3]` before any
guard, so an omitted reply raised `IndexError` and aborted the run without naming
the fixture. Verified by removing the guard: `IndexError: string index out of
range`. A missing reply is now reported as `cnv N: no converted reply was produced
for this fixture`.

Mutation-tested, four ways, each caught: moving the raw-field check back inside
the identity branch (the new four-set test fails), refusing on delimiter presence
instead of role (`a_field_only_change_carries_an_opaque_body_unchanged` fails),
blaming the reply in the message (`a_role_change_for_an_unchanged_delimiter_is_an_error`
fails on its wording assertions), and removing the gate guard (`IndexError`).

Gate: 191/191 fields, 12/12 ACK, 12/12 enhanced, 12/12 converted.

`mllp` 0.7.0 -> 0.8.0: `AckError::UnrepresentableEscape` gained `delimiter`,
`in_source` and `in_reply`, and `DelimiterRole` is public.

## Follow-up: read the structure before rewriting it

A fifth review found the deepest error yet, and it was a modelling one.

**An escape that straddles a component boundary is not an escape.** The walk
consumed `\ZID#1\` from the raw text, so `LAB\ZID#1\@` read as one escape with a
body holding `#` and `@`. But a reader splits on the component separator *first*,
so it sees `LAB\ZID` and `1\@` - two stranded markers, no escape at all. Rewriting
the pair as an escape changed literal characters: with `^` as the component and
`%` as the escape, `LAB%ZID^1%` came back as `LAB\ZID` + `1\`, and the
default-escape case turned `1\@` into `1\\F\`. `transcode` now splits on the
source's component separator and walks each component, where markers are balanced
by construction. Both reviewer examples were reproduced first and are now
regressions: `a_split_escape_marker_is_not_treated_as_an_escape` and
`a_field_only_change_keeps_the_decoded_identity`.

The lesson generalises: this crate's whole escaping problem is that a reader splits
before it resolves, and every rule has to be written in that order. Two of the five
rounds failed by reading the raw text as though it were already parsed.

**A generated value is not a copied value.** `write_header` put MSH-9 in the same
loop as the copied HD fields, so `ACK` was judged against the *incoming* field
separator. A source using `K` made it look like a raw field separator, and the
builder refused a reply that carries `ACK` unchanged. MSH-9 is now escaped like the
generated fallback version, and
`a_generated_message_type_is_never_judged_against_the_source` covers it.

**The gate had the same bug, in both directions.** `converted_pair` always replaced
the component separator, so a field-only conversion was never checked at all -
the accepted case had no coverage, and a rule refusing every opaque body would have
scored full marks. There is now a `field_only_pair` and a `check_field_only`, and
`_conversion_must_refuse` compares roles instead of delimiter presence.

Finding the fixture took two attempts, and the first was wrong in a way worth
recording: a body can *never* hold the component separator, because that splits the
component before any reader sees a body. The fixture that distinguishes a role rule
from a presence rule is an opaque body holding the **subcomponent** separator,
which a field-only reply keeps. With that fixture, the presence mutation is caught
(`fld 13: refused a field-only conversion`); without it, it passed. The corpus is
now 14 messages, 217 fields.

**Documentation.** The README paragraph above the role rule still said a body
holding "a delimiter" is unrepresentable, contradicting the paragraph after it, and
`build`'s error docs omitted the separator-becomes-data direction. Both now state
the role rule and name `RawFieldSeparator` separately.

Mutation-tested, four ways: the raw-text walk restored (three tests fail), MSH-9
back through `transcode` (`a_generated_message_type_is_never_judged_against_the_source`
fails), the presence rule restored (the field-only gate fails once the fixture
holds a subcomponent separator), and the gate's per-field body search restored
(`cnv 10` and `fld 11` disagree with `hl7apy`).

Gate: 217/217 fields, 14/14 ACK, 14/14 enhanced, 14/14 converted, 14/14 field-only.

`mllp` 0.8.0 -> 0.9.0: `transcode`'s contract changed for split markers and MSH-9
is no longer judged against the source, so a reply that was refused now succeeds
and one that was written wrongly now carries the identity it should.

## Follow-up: a marker is data, the boundaries after it are not

A sixth review found the same mistake once more, in the smallest place left.

**An unmatched marker was swallowing the structure after it.** When a component
ends without a closer, `transcode_component` escaped the marker and then escaped
*everything it had read* as plain text. That turned real subcomponent separators
into data: `LAB\ZID&1^2\` splits into components `LAB\ZID&1` and `2\`, so the first
component's marker has no pair, and the reply wrote `LAB\E\ZID\T\1` - one
subcomponent where the source had two. The reply parsed, so nothing but a boundary
count could see it. Only the marker is data now; the delimiters after it are
re-emitted as structure. Reproduced first, and
`an_unmatched_marker_keeps_the_subcomponent_boundaries_after_it` asserts the count
through `Field::subcomponent`.

**Decoded text cannot see a boundary.** This is why the earlier field-only test
passed while the bug was present: escaping a separator as `\T\` decodes back to
`&`, so the decoded identity is unchanged whether the boundary survived or not. The
gate now compares `_boundaries` - the number of subcomponents per component from
the raw split - and `check_field_only` reports `[1, 1]` against `[2, 1]`.

**The README claimed bytes were preserved.** A field-only conversion keeps every
boundary but not every byte: a stranded marker has to be re-encoded as `\E\` once
the reply's escape character differs, as the previous round's fix established. The
paragraph said "byte for byte", which the same paragraph's own reasoning
contradicts. It now says what is preserved is the decoded value and the boundaries,
not the bytes, and states that a body can never hold the component separator.

**The gate never checked MSA-2.** `check_field_only` verified the reply's own
MSH-10 but not the echoed original control id, which is what the sender matches the
reply against - an ACK carrying the wrong original id passed. It is checked now,
and verified by breaking the crate rather than the check: with MSA-2 hardcoded to
`WRONGID`, the gate reports `fld 0: MSA-2 is ['WRONGID'], expected the original's
MSH-10 ['MSG0001']`. Mutating the check instead would have proved nothing.

Mutation-tested, two ways: the whole-tail escape restored (the new unit test fails
and the gate reports `[1, 1]` against `[2, 1]`), and MSA-2 broken in the crate (the
gate reports it per fixture).

Gate: 230/230 fields, 15/15 ACK, 15/15 enhanced, 15/15 converted, 15/15 field-only.

`mllp` 0.9.0 -> 0.10.0: the reply for an unmatched marker now keeps the
subcomponent boundaries the source had.

## Follow-up: a count cannot see a separator move

A seventh review found the check I had just added was one level too weak, plus two
gaps around it.

**A count of subcomponents cannot see a boundary move.** `LAB\T\X&Y` and
`LAB&X\T\Y` each hold one raw `&`, so both split into two subcomponents, and both
decode to `LAB&X&Y`. Swapping which `&` is literal and which is structural changes
the field's meaning while leaving the count and the decoded component string
identical - the exact blind spot the count was added to close, one level down. The
gate now compares `_shape`: each component split on its raw subcomponent separator
with every part decoded, so the boundary stays where the reader put it. A fixture
holds the case, and the mutation that moves the boundary reports
`[['LAB&X', 'Y']]` against `[['LAB', 'X&Y']]`.

The general lesson is the third instance of one mistake: a summary of structure is
not structure. A decoded string hides boundaries; a count hides which boundary. Each
round replaced the summary with something closer to what a reader does, and only
splitting every level the reader splits was enough.

**A missing field crashed the gate instead of naming the fixture.** `check_field_only`
indexed `["MSA-2"]` directly, so a reply with an `MSA` segment but no original id
raised `KeyError` and stopped the run - losing every check after it, including the
replies that would have passed. It is now reported against the fixture and the run
continues.

**The README said a field-only reply is never refused.** It is refused when the new
field separator appears inside an opaque escape body - the role rule again, since
the character was data in the message and would become a boundary in the reply. The
example directly below the paragraph was that very refusal, so the prose and its own
example disagreed. Both the refusal and its reason are stated now.

Mutation-tested: the whole-tail escape restored is caught by the field-only gate with
the moved-boundary fixture present, and the missing-MSA-2 path is exercised directly.

Gate: 243/243 fields, 16/16 ACK, 16/16 enhanced, 16/16 converted, 16/16 field-only.

`mllp` 0.10.0 -> 0.11.0: no behaviour change in the crate itself - the gate and the
documentation were wrong, not the reply.

## Follow-up: fix the check once, then apply it everywhere

An eighth review found the field-only path and the converted path disagreeing about
what "the same identity" means.

`check_field_only` compares `_shape` - every component split into decoded
subcomponents. `check_converted` still compared `_identity`, the decoded component
string. So a boundary move limited to replies that change *both* separators passed,
and that is the path with the most room to move one, since the component separator
itself is rewritten.

The fix is two lines, and the interesting part is why it was needed. The previous
round replaced the count with a split in one place and did not look for other places
holding the same weakened comparison. A check that is correct in one path and weaker
in another reads as green, because the gate reports a single total.

Verified by mutating the crate so that a subcomponent separator is escaped as data
**only when the component separator changes**. Before the fix that mutation passed;
now `cnv 15` reports `[['LAB&X&Y']]` against `[['LAB&X', 'Y']]` while field-only stays
16/16, which is the signature of a gap specific to the two-separator path.

The standing lesson from rounds five to eight: when a comparison turns out to be too
weak, grep for every use of it. Structure is compared one way in a gate, not one way
per call site.

Gate: 243/243 fields, 16/16 ACK, 16/16 enhanced, 16/16 converted, 16/16 field-only.

`mllp` 0.11.0 -> 0.12.0: the gate was weaker than the crate, not the other way round.

## Follow-up: count replies, not findings

A ninth review found the gate's arithmetic wrong, and wrong in a way that hid
nothing today but would have hidden a lot later.

Every checker returned the number of *findings* it printed, and the summary
computed `len(CORPUS) - problems`. So a reply with two bad fields counted as two
failures, and the printed "N/16 agree" understated how many replies passed. With
enough findings the fraction could go negative, which reads as nonsense rather than
as a bug.

Two failures on one reply is not two failed replies. Every checker now prints each
finding and returns `1` if any occurred, and the aggregate is a count of bad
replies. Verified by echoing the source's sending application into MSH-6, which
makes both the identity and the shape check fire on the same field of every reply:
the summary now reads `1/16 converted` instead of a negative number, and the
findings are still all printed.

This is the fourth round in a row where the gate, not the crate, was wrong. The
pattern across five to nine: a summary stood in for the thing it summarised -
decoded text for structure, a count for which boundary, one call site for all of
them, and now findings for replies. Each time the implementation was right and the
report of it was not.

Gate: 243/243 fields, 16/16 ACK, 16/16 enhanced, 16/16 converted, 16/16 field-only.

`mllp` 0.12.0 -> 0.13.0: the gate's arithmetic, not the crate.
