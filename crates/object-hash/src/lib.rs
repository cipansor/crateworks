#![doc = include_str!("../README.md")]
mod encode;
mod md5;
mod sha1;
mod sha256;
mod sha3;
mod sha512;

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
    /// SHA-224, 56 hexadecimal characters.
    Sha224,
    /// SHA-512, 128 hexadecimal characters.
    Sha512,
    /// SHA-384, 96 hexadecimal characters.
    Sha384,
    /// MD5, 32 hexadecimal characters.
    Md5,
    /// SHA3-256, 64 hexadecimal characters. The reference's `sha3-256`.
    Sha3_256,
    /// SHA3-224, 56 hexadecimal characters. The reference's `sha3-224`.
    Sha3_224,
    /// SHA3-512, 128 hexadecimal characters. The reference's `sha3-512`.
    Sha3_512,
    /// SHA3-384, 96 hexadecimal characters. The reference's `sha3-384`.
    Sha3_384,
    /// No digest: the canonical form is returned as it stands.
    ///
    /// This is the reference's `passthrough`, and it is what it uses internally
    /// to write the byte stream it hashes. It is here so a caller can see those
    /// bytes, which is the same string [`canonical`] returns; unlike the digests,
    /// the text form is not a hex or base64 rendering of something else.
    Passthrough,
}

/// How the digest is written out.
///
/// The reference package calls this `encoding`. Hexadecimal is its default and
/// the one a cache key is normally written in; base64 is shorter, and raw bytes
/// are what a caller wants when the digest is fed to something else.
///
/// The reference's four names map onto these three: `'hex'` is [`Self::Hex`],
/// `'base64'` is [`Self::Base64`], and both `'binary'` and `'buffer'` are the
/// same digest bytes. [`Self::Raw`] is the bytes as a `String` of one Latin-1
/// character each, which is the reference's `'binary'`; [`raw_hash_with`] is the
/// bytes on their own, which is its `'buffer'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Encoding {
    /// Lowercase hexadecimal, two characters per byte. The reference's default.
    #[default]
    Hex,
    /// Standard base64 with `+` and `/`, padded with `=`.
    Base64,
    /// The raw digest bytes, one Latin-1 character each. The reference's
    /// `'binary'`; for the bytes themselves use [`raw_hash_with`].
    Raw,
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
/// [`Algorithm::Passthrough`] is the exception to the text form: it returns the
/// canonical string itself, which is what the reference does.
///
/// # Example
///
/// ```
/// use object_hash::{Algorithm, Value, canonical, hash_with};
///
/// let value = Value::string("cipansor");
/// assert_eq!(hash_with(&value, Algorithm::Sha1).len(), 40);
/// assert_eq!(hash_with(&value, Algorithm::Sha256).len(), 64);
/// assert_eq!(hash_with(&value, Algorithm::Sha224).len(), 56);
/// assert_eq!(hash_with(&value, Algorithm::Md5).len(), 32);
/// assert_eq!(hash_with(&value, Algorithm::Sha3_256).len(), 64);
/// // Passthrough is the canonical form, not a digest of it.
/// assert_eq!(hash_with(&value, Algorithm::Passthrough), canonical(&value));
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

/// The digest of a value with chosen options, algorithm and encoding.
///
/// This is the full form; the shorter functions fix the encoding to
/// hexadecimal, which is the reference's default. [`Encoding::Raw`] returns the
/// digest bytes interpreted as one character each, so a caller that wants the
/// bytes should use [`raw_hash_with`] instead.
///
/// # Example
///
/// ```
/// use object_hash::{hash_with_encoding, Algorithm, Encoding, Value};
///
/// let value = Value::Number(1.0);
/// let hex = hash_with_encoding(&value, Algorithm::Sha256, Encoding::Hex);
/// assert_eq!(hex.len(), 64);
/// // Base64 of the same digest is shorter.
/// let b64 = hash_with_encoding(&value, Algorithm::Sha256, Encoding::Base64);
/// assert!(b64.len() < hex.len());
/// ```
#[must_use]
pub fn hash_with_encoding(value: &Value, algorithm: Algorithm, encoding: Encoding) -> String {
    hash_with_options_encoding(value, Options::default(), algorithm, encoding)
}

/// The digest of a value with every choice named.
#[must_use]
pub fn hash_with_options_encoding(
    value: &Value,
    options: Options,
    algorithm: Algorithm,
    encoding: Encoding,
) -> String {
    apply(canonical_with(value, &options), algorithm, encoding)
}

/// The raw digest bytes of a value.
///
/// The reference returns a `Buffer` for `encoding: 'buffer'`; this is the same
/// digest without a text form in between.
#[must_use]
pub fn raw_hash_with(value: &Value, algorithm: Algorithm) -> Vec<u8> {
    raw_digest(canonical(value).as_bytes(), algorithm)
}

/// The raw digest bytes of a value with chosen options.
#[must_use]
pub fn raw_hash_with_options(value: &Value, options: Options, algorithm: Algorithm) -> Vec<u8> {
    raw_digest(canonical_with(value, &options).as_bytes(), algorithm)
}

/// The digest of an already-canonical byte string, as lowercase hexadecimal.
fn digest(canonical: &str, algorithm: Algorithm) -> String {
    apply(canonical.to_string(), algorithm, Encoding::Hex)
}

/// Applies the algorithm and encoding to an already-canonical form.
///
/// [`Algorithm::Passthrough`] has no digest to encode, so the canonical form is
/// the answer whatever the encoding says, which is what the reference does.
fn apply(canonical: String, algorithm: Algorithm, encoding: Encoding) -> String {
    if matches!(algorithm, Algorithm::Passthrough) {
        return canonical;
    }
    encode_digest(&raw_digest(canonical.as_bytes(), algorithm), encoding)
}

/// The raw digest bytes of `data` under `algorithm`.
///
/// [`Algorithm::Passthrough`] is the identity, which is what "no digest" means:
/// the bytes handed in are the bytes handed back.
fn raw_digest(data: &[u8], algorithm: Algorithm) -> Vec<u8> {
    match algorithm {
        Algorithm::Sha1 => sha1::digest(data).to_vec(),
        Algorithm::Sha256 => sha256::digest(data).to_vec(),
        Algorithm::Sha224 => sha256::digest224(data).to_vec(),
        Algorithm::Sha512 => sha512::digest(data).to_vec(),
        Algorithm::Sha384 => sha512::digest384(data).to_vec(),
        Algorithm::Md5 => md5::digest(data).to_vec(),
        Algorithm::Sha3_256 => sha3::digest256(data).to_vec(),
        Algorithm::Sha3_224 => sha3::digest224(data).to_vec(),
        Algorithm::Sha3_512 => sha3::digest512(data).to_vec(),
        Algorithm::Sha3_384 => sha3::digest384(data).to_vec(),
        Algorithm::Passthrough => data.to_vec(),
    }
}

/// Writes out digest bytes in the chosen encoding.
fn encode_digest(bytes: &[u8], encoding: Encoding) -> String {
    match encoding {
        Encoding::Hex => {
            let mut out = String::with_capacity(bytes.len() * 2);
            for byte in bytes {
                out.push(char::from_digit(u32::from(byte >> 4), 16).expect("nibble"));
                out.push(char::from_digit(u32::from(byte & 0x0f), 16).expect("nibble"));
            }
            out
        }
        Encoding::Base64 => base64(bytes),
        Encoding::Raw => bytes.iter().map(|b| char::from(*b)).collect(),
    }
}

/// Standard base64 with padding, written out for the same reason as the digests.
fn base64(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = chunk.get(1).copied().map_or(0, u32::from);
        let b2 = chunk.get(2).copied().map_or(0, u32::from);
        let triple = (b0 << 16) | (b1 << 8) | b2;
        out.push(char::from(ALPHABET[(triple >> 18) as usize & 0x3f]));
        out.push(char::from(ALPHABET[(triple >> 12) as usize & 0x3f]));
        // The last group is padded rather than truncated.
        out.push(if chunk.len() > 1 {
            char::from(ALPHABET[(triple >> 6) as usize & 0x3f])
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            char::from(ALPHABET[triple as usize & 0x3f])
        } else {
            '='
        });
    }
    out
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
