# address-parser

[![crates.io](https://img.shields.io/crates/v/address-parser.svg)](https://crates.io/crates/address-parser)
[![docs.rs](https://docs.rs/address-parser/badge.svg)](https://docs.rs/address-parser)

Parse a free-text US postal address into its components, in pure Rust, with no
dependencies.

An address arrives as one line of text and a business system needs the parts.
Splitting on commas is not enough: the same address is written a dozen ways, and
half of those carry no comma at all.

```rust
use address_parser::{parse_location, Component};

let address = parse_location("1005 N Gravenstein Hwy Suite 500 Sebastopol, CA").unwrap();
assert_eq!(address.number(), Some("1005"));
assert_eq!(address.get(Component::Prefix), Some("N"));
assert_eq!(address.street(), Some("Gravenstein"));
assert_eq!(address.get(Component::Type), Some("Hwy"));
assert_eq!(address.get(Component::SecUnitType), Some("Suite"));
assert_eq!(address.get(Component::SecUnitNum), Some("500"));
assert_eq!(address.city(), Some("Sebastopol"));
assert_eq!(address.state(), Some("CA"));
```

Every form below is understood, and all of them give the same street and city:

```rust
use address_parser::parse_location;

for input in [
    "1005 Gravenstein Hwy 95472",
    "1005 Gravenstein Hwy, 95472",
    "1005 Gravenstein Highway North, 95472",
    "1005 N Gravenstein Highway, Sebastopol, CA",
    "1005 N Gravenstein Hwy Suite 500 Sebastopol, CA",
] {
    assert_eq!(parse_location(input).unwrap().street(), Some("Gravenstein"));
}
```

## What it parses

`parse_location` tries each shape in turn and returns the first that matches,
which is what the reference does:

| Function | Shape |
| --- | --- |
| `parse_location` | anything: intersection, PO box, street address or informal |
| `parse_intersection` | `Mission & Valencia, San Francisco CA` |
| `parse_po_address` | `P.O. Box 1234, Dallas, TX 75201` |
| `parse_address` | `123 Main St, Springfield, IL 62704` |
| `parse_informal_address` | `123 main st springfield il` |

A component the input did not carry is absent, not empty. Read one with
`address.get(Component::Zip)` or the shorthand `address.zip()`. The components
are the reference's own field names: `Number`, `Prefix`, `Street`, `Type`,
`Suffix`, `SecUnitType`, `SecUnitNum`, `City`, `State`, `Zip` and `Plus4`.

An intersection reports each street separately, with the numbered fields
`Street1`/`Street2`, `Type1`/`Type2`, `Prefix1`/`Prefix2` and
`Suffix1`/`Suffix2`, and always carries both type slots even when they are
empty:

```rust
use address_parser::{parse_location, Component};

let address = parse_location("N Main St & S Oak Ave, Springfield IL").unwrap();
assert_eq!(address.get(Component::Street1), Some("Main"));
assert_eq!(address.get(Component::Prefix1), Some("N"));
assert_eq!(address.get(Component::Type1), Some("St"));
assert_eq!(address.get(Component::Street2), Some("Oak"));
assert_eq!(address.get(Component::Prefix2), Some("S"));
```

## What it does not do

It parses; it does not validate. `1005 Gravenstein Hwy` and `1005 Nowhere Hwy`
parse the same way. Whether an address exists is a question for a geocoder, and
this crate does not call one.

The rules are the United States ones. A UK or German address has a different
shape and will not parse usefully.

## Normalisation

The reference normalises some fields, and so does this crate. A direction word
comes back as its code (`North` becomes `N`), a street type as its USPS
abbreviation (`Highway` becomes `Hwy`), and a state as its two-letter code
(`California` becomes `CA`).

## Where the rules come from

The rules are a reimplementation of the `parse-address` JavaScript package by
Hassansin, which descends from the Perl `Geo::StreetAddress::US` module and
ultimately from the USPS publication 28 street-type table. That package is
downloaded tens of thousands of times a week, and the Rust crates in the area
are either thin bindings to the C `libpostal`, which needs a gigabyte model and
a C toolchain, or abandoned single-country regular expressions.

The reference is a single backtracking regular expression with named capture
groups, lookahead and lazy quantifiers. Rust's `regex` crate cannot express it,
and this crate depends on the standard library alone, so it carries a small
backtracking engine and runs the reference's own pattern text, generated from
the reference by `scripts/fuzz/address-parser/extract_patterns.js`. The grammar
is data rather than a translation, so a difference from the reference can only
come from the engine.

Every case in `tests/fixtures/oracle.tsv` was labelled by the reference itself,
including the reference's own test table and a few thousand generated addresses.

## Licence

Apache-2.0. See `LICENSE` and `NOTICE`.
