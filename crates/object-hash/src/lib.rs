#![doc = include_str!("../README.md")]
mod encode;
mod md5;
mod sha1;
mod sha256;

use std::fmt;

/// The digest function used to turn the canonical form into a digest.
///
/// The default is SHA-1, which is what the reference package uses when no
/// algorithm is named. The others are here so a digest can match a system that
/// already speaks one of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Algorithm {
    /// SHA-1, 40 hexadecimal characters. The reference's default.
    #[default]
    Sha1,
    /// SHA-256, 64 hexadecimal characters.
    Sha256,
    /// MD5, 32 hexadecimal characters.
    Md5,
}

/// A value that can be hashed.
///
/// This is the set of things the reference can hash, minus the ones that only
/// exist in JavaScript: there is no case for a function, a symbol, or a `Date`,
/// because a Rust caller would not pass one.
///
/// [`Value::Map`] and [`Value::Object`] are not interchangeable. They are the
/// reference's `Map` and its plain object, which it encodes differently, so the
/// same entries produce a different digest under each. Pick the one that matches
/// what the other side of the conversation is hashing.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// The absence of a value. Written `Null`, not the empty string.
    Null,
    /// A boolean.
    Bool(bool),
    /// A number. An integer is written as a float that happens to be integral,
    /// because the reference has no separate integer case.
    Number(f64),
    /// A string.
    String(String),
    /// A sequence whose order matters.
    Array(Vec<Value>),
    /// A collection whose order does not matter. This is the reference's `Map`.
    Set(Vec<Value>),
    /// A mapping from strings to values, written in key order. This is the
    /// reference's `Map`.
    Map(Vec<(String, Value)>),
    /// A mapping from strings to values, written in key order with unquoted
    /// keys. This is the reference's plain object, and it is encoded differently
    /// from [`Value::Map`].
    Object(Vec<(String, Value)>),
}

/// The choices that change the canonical bytes, mirroring the reference's
/// options.
///
/// [`Options::default`] is what the reference does when nothing is passed, so a
/// digest computed with the default options matches one the reference computes
/// with no options.
///
/// The reference's `respectType` is deliberately absent. It is on by default
/// there and makes every plain object carry a preamble describing Node's
/// prototype chain, which is a fact about the JavaScript runtime rather than
/// about the value. This crate always encodes the `respectType: false` form; if
/// the other side hashes plain objects with the default, hash them as
/// [`Value::Map`] there or turn `respectType` off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Options {
    /// Write only the keys of an object, ignoring the values.
    ///
    /// This is the reference's `excludeValues`, and its `keys` and `keysMD5`
    /// helpers. It applies to [`Value::Object`] only; a [`Value::Map`] keeps its
    /// values, as the reference's does.
    pub exclude_values: bool,
    /// Sort an array's entries, so its order stops mattering.
    ///
    /// This is the reference's `unorderedArrays`, off by default.
    pub unordered_arrays: bool,
    /// Sort a set's entries.
    ///
    /// This is the reference's `unorderedSets`, on by default, and the reason a
    /// set is unordered at all.
    pub unordered_sets: bool,
    /// Sort an object's keys.
    ///
    /// This is the reference's `unorderedObjects`, on by default.
    pub unordered_objects: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            exclude_values: false,
            unordered_arrays: false,
            unordered_sets: true,
            unordered_objects: true,
        }
    }
}

impl Options {
    /// The reference's `keys` and `keysMD5`: hash an object's keys, not its
    /// values.
    #[must_use]
    pub fn keys() -> Self {
        Self {
            exclude_values: true,
            ..Self::default()
        }
    }

    /// The reference's `unorderedArrays: true`: ignore an array's order.
    #[must_use]
    pub fn unordered_arrays(self) -> Self {
        Self {
            unordered_arrays: true,
            ..self
        }
    }
}

impl From<Options> for encode::WriteOptions {
    fn from(options: Options) -> Self {
        Self {
            exclude_values: options.exclude_values,
            unordered_arrays: options.unordered_arrays,
            unordered_sets: options.unordered_sets,
            unordered_objects: options.unordered_objects,
        }
    }
}

impl Value {
    /// A string value from anything string-like.
    #[must_use]
    pub fn string(text: impl Into<String>) -> Self {
        Self::String(text.into())
    }

    /// A number value.
    #[must_use]
    pub fn number(number: impl Into<f64>) -> Self {
        Self::Number(number.into())
    }

    /// Whether this value is a scalar.
    ///
    /// Scalars are the values a JavaScript `Set` deduplicates; arrays and maps
    /// it compares by identity instead. Nothing outside the crate needs this,
    /// but it is the distinction the encoding turns on.
    pub(crate) fn is_scalar(&self) -> bool {
        matches!(
            self,
            Self::Null | Self::Bool(_) | Self::Number(_) | Self::String(_)
        )
    }

    /// The digest of this value, as lowercase hexadecimal SHA-1.
    #[must_use]
    pub fn hash(&self) -> String {
        hash(self)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&canonical(self))
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        Self::Bool(value)
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(value)
    }
}

impl From<i64> for Value {
    fn from(value: i64) -> Self {
        Self::Number(value as f64)
    }
}

impl From<i32> for Value {
    fn from(value: i32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<u32> for Value {
    fn from(value: u32) -> Self {
        Self::Number(f64::from(value))
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.to_string())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(value: Vec<T>) -> Self {
        Self::Array(value.into_iter().map(Into::into).collect())
    }
}

/// The canonical form of a value, as text, with the default options.
///
/// Two values that are equal have the same canonical form. It is returned rather
/// than hashed so a caller can use a different digest, or none.
#[must_use]
pub fn canonical(value: &Value) -> String {
    canonical_with(value, &Options::default())
}

/// The canonical form of a value, as text, with chosen options.
#[must_use]
pub fn canonical_with(value: &Value, options: &Options) -> String {
    encode::canonical(value, &(*options).into())
}

/// The digest of a value, as lowercase hexadecimal SHA-1.
///
/// The digest is stable across runs and machines for equal values. See the
/// crate documentation for what equal means here.
///
/// # Example
///
/// ```
/// use object_hash::{hash, Value};
///
/// let left = Value::Map(vec![
///     ("b".into(), Value::number(2)),
///     ("a".into(), Value::string("one")),
/// ]);
/// // A map's key order does not change the digest.
/// let right = Value::Map(vec![
///     ("a".into(), Value::string("one")),
///     ("b".into(), Value::number(2)),
/// ]);
/// assert_eq!(hash(&left), hash(&right));
///
/// // An array's order does.
/// assert_ne!(
///     hash(&Value::Array(vec![Value::number(1), Value::number(2)])),
///     hash(&Value::Array(vec![Value::number(2), Value::number(1)])),
/// );
/// ```
#[must_use]
pub fn hash(value: &Value) -> String {
    hash_with(value, Algorithm::Sha1)
}

/// The digest of a value with a chosen algorithm, as lowercase hexadecimal.
///
/// The canonical bytes are the same whatever the algorithm, so two algorithms
/// agree on which values are equal and differ only in the text they produce.
///
/// # Example
///
/// ```
/// use object_hash::{Algorithm, Value, hash_with};
///
/// let value = Value::string("cipansor");
/// assert_eq!(hash_with(&value, Algorithm::Sha1).len(), 40);
/// assert_eq!(hash_with(&value, Algorithm::Sha256).len(), 64);
/// assert_eq!(hash_with(&value, Algorithm::Md5).len(), 32);
/// ```
#[must_use]
pub fn hash_with(value: &Value, algorithm: Algorithm) -> String {
    digest(&canonical(value), algorithm)
}

/// The digest of a value with chosen options, as lowercase hexadecimal SHA-1.
///
/// # Example
///
/// ```
/// use object_hash::{Options, Value, hash_with_options};
///
/// let object = Value::Object(vec![("a".into(), Value::number(1))]);
/// // A plain object and a map with the same entry hash differently, which is
/// // the reference's behaviour and the reason both cases exist.
/// let map = Value::Map(vec![("a".into(), Value::number(1))]);
/// assert_ne!(hash_with_options(&object, Options::default()), hash_with_options(&map, Options::default()));
/// ```
#[must_use]
pub fn hash_with_options(value: &Value, options: Options) -> String {
    hash_with_options_and(value, options, Algorithm::Sha1)
}

/// The digest of a value with chosen options and algorithm.
#[must_use]
pub fn hash_with_options_and(value: &Value, options: Options, algorithm: Algorithm) -> String {
    digest(&canonical_with(value, &options), algorithm)
}

/// The digest of an already-canonical byte string.
fn digest(canonical: &str, algorithm: Algorithm) -> String {
    let bytes = canonical.as_bytes();
    match algorithm {
        Algorithm::Sha1 => sha1::hex(bytes),
        Algorithm::Sha256 => sha256::hex(bytes),
        Algorithm::Md5 => md5::hex(bytes),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_digest_is_forty_hexadecimal_characters() {
        let digest = hash(&Value::Null);
        assert_eq!(digest.len(), 40);
        assert!(digest.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn equal_values_hash_the_same_and_different_values_do_not() {
        assert_eq!(hash(&Value::Number(1.0)), hash(&Value::Number(1.0)));
        assert_ne!(hash(&Value::Number(1.0)), hash(&Value::Number(2.0)));
        assert_ne!(hash(&Value::Number(1.0)), hash(&Value::String("1".into())));
        assert_ne!(hash(&Value::Null), hash(&Value::String("Null".into())));
    }

    #[test]
    fn the_conversions_build_the_expected_values() {
        assert_eq!(Value::from("a"), Value::String("a".into()));
        assert_eq!(Value::from(1_i32), Value::Number(1.0));
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(
            Value::from(vec![1_i32, 2]),
            Value::Array(vec![Value::Number(1.0), Value::Number(2.0)])
        );
    }

    #[test]
    fn the_display_form_is_the_canonical_form() {
        assert_eq!(Value::Bool(true).to_string(), "bool:true");
        assert_eq!(Value::Null.to_string(), "Null");
    }
}
