//! Parsing free-text US postal addresses into their components.
//!
//! An address arrives as one line of text - `1005 N Gravenstein Highway, Suite
//! 500, Sebastopol, CA 95472` - and a business system needs the parts: the house
//! number, the street, the unit, the city, the state and the ZIP. Splitting on
//! commas is not enough, because the same address is written a dozen ways and
//! half of them carry no comma at all.
//!
//! This crate is a pure-Rust parser for that job. It is a reimplementation of
//! the `parse-address` JavaScript package by Hassansin, whose rules descend from
//! the Perl `Geo::StreetAddress::US` module and ultimately from the USPS
//! publication 28 street-type table. Every case in this crate's test corpus was
//! labelled by that reference.
//!
//! ```
//! use address_parser::{parse_location, Component};
//!
//! let address = parse_location("1005 N Gravenstein Hwy Suite 500 Sebastopol, CA").unwrap();
//! assert_eq!(address.number(), Some("1005"));
//! assert_eq!(address.get(Component::Prefix), Some("N"));
//! assert_eq!(address.street(), Some("Gravenstein"));
//! assert_eq!(address.get(Component::Type), Some("Hwy"));
//! assert_eq!(address.get(Component::SecUnitType), Some("Suite"));
//! assert_eq!(address.get(Component::SecUnitNum), Some("500"));
//! assert_eq!(address.city(), Some("Sebastopol"));
//! assert_eq!(address.state(), Some("CA"));
//! ```
//!
//! # What it does and does not do
//!
//! It parses, it does not validate. `1005 Gravenstein Hwy` and `1005 Nowhere
//! Hwy` parse the same way; whether the address exists is a question for a
//! geocoder. A field the input did not carry is absent from the result, and a
//! field the reference normalises - a state name to its code, a direction word
//! to `N`, a street type to its USPS abbreviation - comes back normalised.
//!
//! The rules are the United States ones. A UK or German address has a different
//! shape and will not parse usefully.
//!
//! # How it is built
//!
//! The reference is a single backtracking regular expression with named capture
//! groups, lookahead and lazy quantifiers. Rust's `regex` crate cannot express
//! it, and this workspace's crates depend on the standard library alone, so the
//! crate carries a small backtracking engine (the `regex` module) and runs the
//! reference's *own* pattern text (the `patterns` module, generated from the
//! reference by `scripts/fuzz/address-parser/extract_patterns.js`). The grammar
//! is data, not a translation, so a difference from the reference can only come
//! from the engine.

#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod parser;
mod patterns;
mod regex;

pub use crate::parser::{
    Address, Component, parse_address, parse_informal_address, parse_intersection, parse_location,
    parse_po_address,
};
