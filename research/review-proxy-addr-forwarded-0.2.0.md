# `proxy-addr` `Forwarded` 0.2.0: nine findings, one deliberate divergence (2026-11-16)

## What was asked

Review the RFC 7239 `Forwarded` support added in 0.2.0 against the grammar it
claims to implement, close what the review found, and leave the crate's
comparison with its reference stronger than it was. 0.2.0 is unpublished, so the
work folds into that version rather than becoming 0.2.1: a version number on
crates.io cannot be withdrawn, and publishing a fix for an API nobody has yet is
a version spent for nothing.

## Why the grammar was the standard, not the reference

The reference is npm `forwarded-parse` 2.2.0, and the crate's job is to agree
with it. But "agree with the reference" is not a specification: where the
reference contradicts RFC 7239, agreeing means shipping a parser that accepts
what the RFC forbids. So each finding was checked against the ABNF first, and
the reference consulted second. Nine findings were raised; eight were real, and
one turned out to be a conflict between the two that had to be decided rather
than fixed.

## The findings

| # | Finding | Verdict |
| --- | --- | --- |
| 1 | A port was only representable on an IPv4/IPv6 address | Real |
| 2 | An obfuscated port (`obfport`) was unrepresentable | Real |
| 3 | `unknown` was matched case-sensitively | Real |
| 4 | Brackets were accepted around an IPv4 address | Real |
| 5 | The oracle's rendering could not distinguish a quoted separator from a parameter boundary | Real |
| 6 | Empty list elements were rejected | **Spec conflict** |
| 7 | The oracle version was not pinned | Real |
| 8 | `package-lock.json` was not regenerated, so `npm ci` failed | Real |
| 9 | The port accepted shapes `1*5DIGIT` does not | Real |

### 1-4: the node was a narrower type than the grammar

RFC 7239 gives

```
node      = nodename [ ":" node-port ]
nodename  = IPv4address / "[" IPv6address "]" / "unknown" / obfnode
node-port = port / obfport
```

The port sits **outside** `nodename`, so it is not a property of an address:
`unknown:443`, `_hidden:443`, `192.0.2.43:_edge` and `_hidden:_p` are all legal.
Typing it as `Option<u16>` on the address variant made every one of those
unparseable. The failure was silent in the worst way: `parse_forwarded`
succeeded, and only the typed layer dropped the node.

`Node` now carries a `Port` of its own on every variant:

```rust
pub enum Node {
    Address { ip: IpAddr, port: Option<Port> },
    Unknown { port: Option<Port> },
    Obfuscated { name: String, port: Option<Port> },
}

pub enum Port {
    Number(u16),
    Obfuscated(String),
}
```

`unknown` is an ABNF literal, so it is case-insensitive: `Unknown` is the same
identifier, and the reference agrees. Brackets are reserved for IPv6, so
`[192.0.2.43]` is not a node; the old check accepted it by stripping brackets
before deciding which parser to use.

`obfnode` and `obfport` are `"_" 1*( ALPHA / DIGIT / "." / "_" / "-" )`, which is
narrower than the `token` the outer grammar accepts. Validating them with the
token predicate accepted `_edge+2`, `_a!b` and `_a~b`. `is_obfname` now
implements the rule as written.

### 9: `1*5DIGIT` is not `u16::from_str`

Rust's integer parser accepts a leading `+` and any number of leading zeros, so
`"+443"` and `"000443"` both parse. Neither is a port: `port = 1*5DIGIT`. The
shape is now checked before the value.

### 5: a rendering that could not see the difference it was testing

The corpus rendered each element as a sorted `name=value` list. That cannot
distinguish `a="b,c=d"` from `a=b;c=d`: both render as the same text, so a parser
that split a quoted value into a second parameter would still agree with the
reference. The rendering now JSON-quotes every name and value, which keeps the
boundary visible.

Two seed headers were added so the case is in the corpus by construction rather
than by luck.

### 7 and 8: the gate was not reproducible

The generator asserted nothing about which reference it was using, so installing
a newer `forwarded-parse` would re-baseline the corpus silently and report as a
crate defect. It now pins 2.2.0 and exits 1 on a mismatch, which was verified by
installing 9.9.9 and watching it refuse.

`package.json` had gained `forwarded-parse` while `package-lock.json` was
untouched. `npm install` worked and `npm ci` failed with
`Missing: forwarded-parse@2.2.0 from lock file`, which is what CI runs. The lock
was regenerated; the diff adds that one package and nothing else, and a clean
`npm ci` now installs 52 packages.

### 6: the one divergence, and why it is not a bug

RFC 7230 section 7:

> A recipient MUST parse and ignore a reasonable number of empty list elements.

The reference does not. Its parser requires `start !== -1 || end !== -1` before
treating a `,` as a separator, so `for=a,,for=b` and a trailing `for=a,` are both
rejected. The crate follows the RFC.

Deleting those rows would drop coverage; asserting agreement would pin the crate
to the reference's bug. Instead each such row is marked `empty` and carries the
reference's verdict for the header with the empty elements removed, and
`the_divergence_is_only_about_empty_elements` re-derives that stripping them is
what changes the reference's mind. A header broken for a second reason therefore
cannot be relabelled into passing, which is the failure mode of a
"known divergence" list.

## What the divergence cost, twice

The tolerance was wrong in two directions, and both were caught by evidence
rather than by review.

**A comma after a dangling `;` was treated as an empty element.**
`for=1.1.1.1;,for=2.2.2.2` merged two hops into one, with no error. The comma
there follows a parameter that was opened and never closed, so it is a syntax
error. The tolerance now applies only when the element has collected no
parameters yet.

**A trailing `;` was accepted.** `zz=198.51.100.17;` opened a parameter and the
header ended, so the element is truncated; the parser accepted it and dropped the
parameter. The two cases look identical to a "nothing is open, so we are done"
branch, which is exactly what the code had. It now records whether the last
separator was a `,` or a `;` and allows the end only after a comma.

The second one shipped through a green build: 23 tests, clippy and the committed
6,019-case corpus all passed. It was caught by generating a corpus from a
**different seed** and replaying it, which is now the standing check. Four seeds
(`12345`, `777`, `424242`, `99999`) agree on 24,000 cases that the committed
corpus does not contain.

The generator's own helper was also wrong, in the way that makes a crate look
guilty: `withoutEmptyElements` trimmed every surviving segment, which repaired a
leading space - a separate fault - and then joined with `", "`, inserting a space
into the header being measured. Both would have reported a crate defect that did
not exist.

## How it is checked

`scripts/fuzz/proxy-addr/gen_forwarded.js` writes 6,023 rows: the header in hex,
a strictness column (`strict` or `empty`), and the reference's verdict.

| Outcome | `strict` | `empty` |
| --- | --- | --- |
| accepted | 2,125 | 16 |
| rejected, unexpected character (index compared) | 3,676 | 23 |
| rejected, early end | 155 | 0 |
| rejected, remainder still invalid (rejection only) | 0 | 28 |

The 23 `empty` rows with an index are the ones where the fault precedes the
first removed character, so the index is still the index in the original header.
They are compared exactly; only the 28 rows where the index genuinely moved
compare rejection alone.

`tests/forwarded_oracle.rs` replays all of it, comparing the character index of
every rejection, and `tests/forwarded_nodes.rs` covers the typed layer the
reference has no notion of: the three node forms, both port forms, `Display`
round-tripping, and the empty-element rules above.

Gates: `cargo fmt --check`, clippy `-D warnings` on stable and on 1.88 (forced
rebuild, since clippy prints nothing on a cached one), 1,632 workspace tests,
255 doctests, `cargo doc` with no warnings, `check_attribution.py` and
`check_actions_pinned.py`.

## Three findings from review of the corpus tooling

A review of the generator and its documentation raised three defects, all real,
all in the tooling rather than the parser. None of them changed a line of
`forwarded_header.rs`; each made the *evidence* weaker than it claimed to be,
which is the harder kind to notice.

### 1. The oracle pin was documented in one place, not two

`PINNED` in the generator records `forwarded-parse` 2.2.0 and the generator
refuses to run against any other version. `research/tools/README.md` documented
only the `proxy-addr` oracle behind `verify_proxy_addr.py`, so a reader looking
for the version to install found the X-Forwarded-For one and nothing about
`Forwarded`. That matters because `Forwarded` is a **second, independent
oracle**: a different grammar, a different npm package, a different fixture.
`AGENTS.md` asks for the pinned version to be recorded beside the gate and in
`research/tools/README.md`; the row is now there, naming the package, the
version, the fixture, the divergence and the `OUT`/`SEED` overrides.

### 2. Stripping an empty element left its separator's whitespace behind

The list rule is

```
#element => [ element ] *( OWS "," OWS [ element ] )
```

so the space in `, for=192.0.2.43` belongs to the comma. `withoutEmptyElements`
dropped the empty first segment but kept its text, producing ` for=192.0.2.43`
- a header the reference rejects for **leading whitespace**, which has nothing
to do with empty elements. The row therefore expected `!` where the crate
correctly accepted the header. Sixteen committed rows had their rewrite changed
by this, and exactly two had their verdict flip - both from `!` to `OK`, which
is the false rejection the review named: `, for=192.0.2.43` and the mirror case
`for=192.0.2.43 ,`. The other fourteen still ended in the same verdict because
they carried a second fault, so the rewrite was wrong in sixteen rows and the
recorded expectation in two.

The transform now drops whitespace only on the side where the adjacent segment
was dropped. Whitespace next to a *surviving* comma is that comma's OWS and
stays, which is why an earlier attempt that trimmed every segment changed 2,497
verdicts: it rewrote headers whose separators were never touched.

Four seed headers were added so the shape is in the corpus by construction
rather than by luck, and a curated test in `forwarded_nodes.rs` covers it.

### 3. An `empty` marker discarded an index that had not moved

`row` replaced every `empty` row's index with a bare `!`, on the grounds that
skipping an element shifts later positions. That is true only from the first
removed character onward. ` for=a,,for=b` fails at index 0 for its leading space
whether or not the empty element is there, so the index was still comparable and
throwing it away made the corpus weaker than it could be.

The transform now reports the offset of the first character it dropped, and a
row keeps `!pos:<n>` when `n` precedes it. Twenty-three rows were strengthened
this way; each was verified to report the same index on the original header and
on the rewritten one. Twenty-eight rows still compare rejection only, because
their index really did move.

Both indices are UTF-16 code units, the reference's own convention (confirmed
with an astral character before an error: `for=a;zz=😀` reports index 9, its
code-unit position, not 8), so the two are directly comparable.

### What these three have in common

Each was a case of the tooling being *less* strict than it looked. The pin was
enforced but not documented; the divergence was recorded but with a rewrite that
invented a second fault; the index was dropped for a reason that applied to only
part of the rows. A corpus that overstates its own coverage is worse than a
smaller honest one, because the green build is what stops anyone looking.

The unseen-seed replay was re-run over all four seeds after these changes:
24,092 cases, no disagreement. The committed fixture is byte-identical across
two runs (md5 `09ee89b572a796ddf661b64c8cff8f70`).
