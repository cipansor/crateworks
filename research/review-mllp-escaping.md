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
