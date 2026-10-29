//! Redact secrets from a JSON document before it is logged.
//!
//! A log line is a JSON document, and the fields that must never be written to
//! disk — a cookie, an `Authorization` header, a password — sit at known
//! locations in it. This crate takes those locations as paths and returns the
//! document with the values at them replaced, so a request can be logged whole
//! without any secret in it.
//!
//! It is a port of the npm package `fast-redact`, which is what the `pino`
//! logger uses. The behaviour, including the way wildcards work and the shape of
//! the output, matches the reference and is checked against it case by case in
//! `tests/oracle.rs`.
//!
//! # Example
//!
//! ```rust
//! use fast_redact::{Redactor, Value};
//!
//! let document = Value::parse(
//!     r#"{"headers":{"host":"example.com","cookie":"session=abc","x-forwarded-for":"1.2.3.4"}}"#,
//! ).unwrap();
//!
//! let redactor = Redactor::new([
//!     "headers.cookie",
//!     r#"headers["x-forwarded-for"]"#,
//! ]).unwrap();
//!
//! assert_eq!(
//!     redactor.redact(&document),
//!     r#"{"headers":{"host":"example.com","cookie":"[REDACTED]","x-forwarded-for":"[REDACTED]"}}"#,
//! );
//! ```
//!
//! # Paths
//!
//! A path names a location. Dot notation (`headers.cookie`), bracket notation
//! for keys that are not identifiers (`headers["x-forwarded-for"]`), array
//! indices (`items[0].secret`) and `*` wildcards (`users.*.token`) are all
//! accepted. A path is validated when the redactor is built, so a typo is an
//! error rather than a redaction that silently does nothing.
//!
//! A `*` matches every key of the value at that position. As in the reference,
//! a path is split at its **first** `*`: `a.*.b` reaches `a`, then walks `b`
//! from each of its keys. A second `*` in the same path does not behave like a
//! general glob — see [`Redactor`] and the module documentation of the internal
//! redaction for what it does instead, which is what the reference does.
//!
//! # What replaces a value
//!
//! By default a redacted value becomes the string `"[REDACTED]"`. Three
//! alternatives are available, and they are the reference's three options:
//!
//! - [`Redactor::with_censor`] replaces it with any [`Value`].
//! - [`Redactor::with_remove`] removes the key, which is the reference's
//!   `remove: true`. In an array the element becomes `null`, because that is
//!   what `JSON.stringify` does with the `undefined` the reference leaves.
//! - [`Redactor::with_censor_fn`] computes the replacement from the original
//!   value and its path.
//!
//! # What this crate is
//!
//! An independent Rust implementation with no dependencies. It carries its own
//! JSON reader and writer, because the output has to be byte-for-byte what
//! `JSON.stringify` produces and the workspace this crate belongs to takes no
//! dependencies. It reads a document, redacts a copy, and writes the copy; the
//! input is never modified, so unlike the reference there is no `restore` step
//! and none is needed.

mod json;
mod path;
mod redact;

pub use json::{ParseError, Value};
pub use path::{Path, PathError, parse as parse_path};
pub use redact::Redactor;

#[doc = include_str!("../README.md")]
#[cfg(doctest)]
pub struct ReadmeDoctests;
