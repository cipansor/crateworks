# object-hash

[![crates.io](https://img.shields.io/crates/v/object-hash.svg)](https://crates.io/crates/object-hash)
[![docs.rs](https://docs.rs/object-hash/badge.svg)](https://docs.rs/object-hash)

A stable hash of a structured value.

Given a map, an array, a string, a number, nested in any combination, this
produces a short digest that is the same every time for values that are equal —
across runs, across processes, and across machines.

```rust
use object_hash::{Value, hash};

let one = Value::Map(vec![
    ("name".into(), Value::String("cipansor".into())),
    ("year".into(), Value::Number(2026.0)),
]);
// The same entries, inserted the other way round.
let two = Value::Map(vec![
    ("year".into(), Value::Number(2026.0)),
    ("name".into(), Value::String("cipansor".into())),
]);

assert_eq!(hash(&one), hash(&two));
```

## Why `Hash` is not enough

`std::hash::Hash` looks like the right tool and is not. The default hasher,
`RandomState`, is seeded per process, so the same value hashes differently every
time the program starts. That is exactly right for a `HashMap` — the seed is what
stops an attacker choosing collisions — and exactly wrong for anything that has
to be written down or compared across processes. The other half of the problem is
that a `HashMap` iterates in an unspecified order, so even with a fixed hasher two
equal maps can hash differently.

So the value is normalised first, into a canonical form that depends only on what
the value *is*, and then those bytes are hashed. This crate does both.

## What it is for

- **Cache keys** that hit the same entry for the same request.
- **ETags** that change when the content changes and not when a field is
  reordered.
- **Idempotency keys** that recognise a repeated request as the same action.
- **Change detection and deduplication** by comparing two digests.

## The equality rules

| Value | Rule |
|---|---|
| `Map` | Written in key order; insertion order does not matter; a repeated key keeps the last value |
| `Object` | A plain object, encoded differently from a `Map`; keys are written unquoted, in key order, and a repeated key keeps the last value |
| `Array` | Order matters |
| `Set` | Order does not matter; repeated scalars are dropped, repeated arrays are not (a JavaScript `Set` compares arrays by identity) |
| `Number` | An integer and an equal float are the same number; `-0.0` is `0`; `NaN`, `Infinity` and `-Infinity` have names; the text is JavaScript's, so `1e21` is `1e+21` and `1e-7` is `1e-7` |
| `String` | Carries its length in UTF-16 code units, so an emoji counts as two |
| `Null` | Its own value, distinct from the empty string |

The length prefix is what stops `{"ab": 1}` and `{"a": "b1"}` from writing the
same bytes.

### `Map` and `Object` are not interchangeable

The reference has both a `Map` and a plain object and encodes them differently,
so the same entries give two different digests:

```rust
use object_hash::{Value, hash};

let object = Value::Object(vec![("a".into(), Value::number(1))]);
let map = Value::Map(vec![("a".into(), Value::number(1))]);
assert_ne!(hash(&object), hash(&map));
```

Pick the one the other side of the conversation is hashing. If a Node service
passes a plain object, hash `Value::Object`; if it passes a `Map`, hash
`Value::Map`.

## Options

`Options` mirrors the reference's options, and `Options::default()` is what it
does when nothing is passed. Pass them to `canonical_with`, `hash_with_options`
or `hash_with_options_and`.

| Field | Default | Effect |
|---|---|---|
| `exclude_values` | `false` | Write only an object's keys, ignoring the values. This is the reference's `keys` helper |
| `unordered_arrays` | `false` | Sort an array's entries, so order stops mattering |
| `unordered_sets` | `true` | Sort a set's entries. Turning it off keeps the insertion order |
| `unordered_objects` | `true` | Sort an object's keys |

```rust
use object_hash::{Options, Value, canonical_with};

let keys = Value::Object(vec![
    ("a".into(), Value::number(1)),
    ("b".into(), Value::number(2)),
]);
// Only the keys are written.
assert_eq!(canonical_with(&keys, &Options::keys()), "object:2:string:1:a:,string:1:b:,");
```

## Compatibility with the npm package

The canonical form is the one the `object-hash` npm package produces, byte for
byte, and the digest is SHA-1 in lowercase hexadecimal, which is its default. A
digest computed here equals one computed there for the same value, so a Rust
service and a Node service can agree on a cache key without either knowing about
the other. That is why a map is written as an array of pairs, and why an integral
float has no decimal point: the reference writes them that way.

The one deliberate difference is the reference's `respectType`, which is on by
default there. It makes every plain object carry a preamble describing Node's
prototype chain — `prototype`, `__proto__` and `constructor` — which is a fact
about the JavaScript runtime rather than about the value, and can even depend on
where the object sits in the traversal. A Rust caller has no prototype to respect.
This crate always encodes the `respectType: false` form, which is why
`Value::Object` and `Value::Map` are both here and both reproducible: hash a
plain object there with `{respectType: false}` to match `Value::Object`, or pass
it as a `Map` there to match `Value::Map`.

## Choosing the digest

`hash` uses SHA-1, the reference's default. `hash_with` takes an `Algorithm`
when a digest has to match a system that already speaks another one:

| `Algorithm` | Characters | When |
|---|---|---|
| `Sha1` | 40 | The default, and what the npm package produces |
| `Sha256` | 64 | A digest that is not SHA-1 |
| `Sha224` | 56 | Matching a system that already uses SHA-224 |
| `Sha512` | 128 | Matching a system that already uses SHA-512 |
| `Sha384` | 96 | Matching a system that already uses SHA-384 |
| `Md5` | 32 | Matching an existing MD5 in the field |
| `Sha3_256` | 64 | Matching a system that already uses SHA3-256 |
| `Sha3_224` | 56 | Matching a system that already uses SHA3-224 |
| `Sha3_512` | 128 | Matching a system that already uses SHA3-512 |
| `Sha3_384` | 96 | Matching a system that already uses SHA3-384 |
| `Sha512_224` | 56 | Matching a system that already uses SHA-512/224 |
| `Sha512_256` | 64 | Matching a system that already uses SHA-512/256 |
| `Ripemd160` | 40 | Matching a system that already uses RIPEMD-160 |
| `Sm3` | 64 | Matching a system that already uses SM3 |
| `Blake2b512` | 128 | Matching a system that already uses BLAKE2b-512 |
| `Blake2s256` | 64 | Matching a system that already uses BLAKE2s-256 |
| `Shake128` | 32 | Matching a system that already uses SHAKE128 |
| `Shake256` | 64 | Matching a system that already uses SHAKE256 |
| `Passthrough` | - | The canonical bytes themselves, no digest |

The canonical bytes are the same whatever the algorithm, so the eighteen digests
agree on which values are equal and differ only in the text they produce. Every
digest is written out here, so the crate still has no dependencies. SHA-224 is
SHA-256 and SHA-384 is SHA-512, each with a different initial state and a
truncated output, so each pair shares its round code; SHA-512/224 and SHA-512/256
are the same function again with their own initial states, so they join that
family. The four SHA-3 members and the two SHAKE members are one Keccak sponge
with a different rate, domain-separation byte and output length, so they share a
round function too. The reference reaches all of these through Node's `crypto`,
and its `algorithm` option accepts any name there, which is what `sha3-256`,
`sha512-224`, `ripemd160`, `sm3`, `blake2b512`, `blake2s256`, `shake128` and
`shake256` name.

RIPEMD-160, SM3 and the BLAKE2 widths are their own constructions - RIPEMD-160
runs two lines over a block and combines them, SM3 has SHA-256's shape but its
own expansion and constants, BLAKE2 is a ChaCha-derived ARX hash - so each is a
separate implementation rather than a variant of something already here. None of
them is offered as a security choice: like SHA-1 and MD5, they are here so a
digest can match a system that already speaks them.

`Shake128` and `Shake256` are the extendable-output members of the SHA-3 family:
the output length is a caller parameter, but the reference's `algorithm` is a
fixed string with no place to put one, so both sides use Node's default for the
name - 16 bytes and 32 bytes. That fixed-length reading is what makes them
gateable against the reference like any other digest.

`Passthrough` is the reference's odd one out: it is not
a digest at all but the byte stream the digests are taken over, so
`hash_with(value, Algorithm::Passthrough)` returns the same string `canonical`
does. It is here because the reference names it, and because seeing those bytes
without a second function is occasionally what a caller wants.

`hash_with_encoding` takes an `Encoding` as well, mirroring the reference's
`encoding` option:

| `Encoding` | When |
|---|---|
| `Hex` | The default, and what the npm package produces |
| `Base64` | A shorter key, standard base64 with padding |
| `Raw` | The digest bytes as one character each |

`raw_hash_with` and `raw_hash_with_options` return the digest as `Vec<u8>`
directly, which is the reference's `encoding: 'buffer'` without a text form in
between.

## What it is not

SHA-1 and MD5 are not used here as security primitives. A digest names a value;
it does not protect it, and it is not a signature. `canonical` is exported, so if
a different digest is wanted, hash the canonical bytes with any other function.

## How it is checked

Against the `object-hash` npm package, 84 curated cases: the reference's own
canonical byte stream and its default digest for strings (including emoji and
CJK), nested maps, plain objects, arrays and sets, the deduplication rules, and
the sorting of unordered collections. Numbers get a file of their own, 18 cases,
because Rust and JavaScript disagree about where to put the decimal point: Rust
prints `1e21` as twenty-two digits where JavaScript prints `1e+21`, and `1e-7` as
`0.0000001` where JavaScript prints `1e-7`. The text is generated by the
ECMAScript rules rather than by Rust's `Display`, and the boundaries are checked
in both directions. All eighteen digests are checked: the curated cases carry the
reference's SHA-1, SHA-256, SHA-224, SHA-512, SHA-384, MD5, the four SHA-3
digests, the two SHA-512 truncations, RIPEMD-160, SM3, the two BLAKE2 widths and
the two SHAKE outputs over its own canonical bytes, along with the base64 of the
SHA-1 digest, and each algorithm is checked against its published vectors,
including the long ones.
SHA-384 is the one that caught a transcription error: a single wrong digit in its
initial state still produced a plausible-looking 96-character digest, so only a
digest taken from the reference (or the FIPS vector) rather than written by hand
could tell it apart. SHA-224 is pinned the same way, against both the reference
and the FIPS vectors, and the SHA-3 members are pinned against the FIPS 202
vectors at lengths on either side of their rate, where the padding changes shape.
The eight later additions are pinned the same way, against their own published
vectors - RIPEMD-160's paper, GB/T 32905-2016 for SM3, RFC 7693 for BLAKE2 and
FIPS 202 for the SHAKE outputs, at lengths on either side of their rate - and
against the reference over the whole corpus, which is what caught the two
transcription slips the vectors alone would have missed.

There is also a corpus of 4,000 values built at random - nesting arrays, sets,
maps and plain objects, numbers at both ends of the range, strings holding emoji,
CJK and U+FFFF, repeated keys and entries - with the reference's canonical form
and ten of its digests for each, so every hand-ported absorb loop is exercised
at four thousand different lengths. It is what found the number formatting and
the UTF-16 ordering; the curated cases had missed both. A second corpus of 2,000
plain objects pins the `object:` encoding and the `excludeValues` and
`unorderedArrays` options. The generator that built both is in
`scripts/fuzz/object-hash/`, so the corpora can be rebuilt and extended.

`Passthrough` is checked against the same canonical column every other algorithm
is taken over, so the reference's own bytes pin it in both the curated file and
the random corpus rather than only in a hand-written example.

The plain-object rows are hashed with `respectType: false`, because the
reference's default prepends a Node prototype preamble that no other language can
reproduce. That is the one option this crate does not mirror; everything else it
does.

One ordering rule is worth stating because it is not obvious: entries of a set or
map are ordered by UTF-16 code unit, as JavaScript orders strings, not by UTF-8
byte. The two disagree above the basic plane, where an emoji sorts before U+FFFF.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
