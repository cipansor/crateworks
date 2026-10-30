//! The canonical form of a value.
//!
//! A structured value is written out as a single byte string in a way that
//! depends only on what the value *is*, never on how it was built. Two values
//! that a caller would call equal produce the same bytes, so hashing the bytes
//! gives a stable name for the value.
//!
//! The encoding is the one the `object-hash` npm package uses, so the bytes here
//! are byte-for-byte the same and a digest computed here matches one computed
//! there. That is deliberate: it lets a Rust service and a Node service agree on
//! a cache key or an ETag without either one knowing about the other.
//!
//! # What the value is, not how it was made
//!
//! - A map is written in key order, so two maps with the same entries in
//!   different insertion order are the same value.
//! - An array keeps its order; a set does not.
//! - Integers and floats of equal value are the same number. There is no
//!   separate integer case, because the reference has none.
//! - Every string carries its length, so no two different structures can write
//!   the same bytes by accident.

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use crate::Value;

/// The choices that change the canonical bytes.
///
/// These mirror the reference's options, minus the ones that only mean something
/// in JavaScript. In particular there is no `respectType`: the reference's
/// default adds a Node-runtime prototype preamble to every plain object, which
/// is a property of the JavaScript object model rather than of the value, and
/// can even depend on where the object sits in the traversal. Reproducing it is
/// impossible in a language without that object model, so this crate always
/// encodes the `respectType: false` form. See [`crate::Options`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WriteOptions {
    /// Write only the keys of a map or object, ignoring the values.
    pub exclude_values: bool,
    /// Sort an array's entries, so its order stops mattering.
    pub unordered_arrays: bool,
    /// Sort a set's entries. The reference does this by default.
    pub unordered_sets: bool,
    /// Sort a plain object's keys. The reference does this by default.
    pub unordered_objects: bool,
}

/// Writes the canonical form of a value into a string.
///
/// The result is text because the encoding is text: it is what the reference
/// writes into its hash stream. The bytes are its UTF-8 encoding.
pub(crate) fn write(value: &Value, options: &WriteOptions, out: &mut String) {
    match value {
        Value::Null => out.push_str("Null"),
        Value::Bool(true) => out.push_str("bool:true"),
        Value::Bool(false) => out.push_str("bool:false"),
        Value::Number(number) => write_number(*number, out),
        Value::String(text) => write_string(text, out),
        Value::Array(items) => write_array(items, options, out),
        Value::Set(items) => {
            out.push_str("set:");
            write_set(items, options, out);
        }
        Value::Map(entries) => {
            out.push_str("map:");
            write_map(entries, options, out);
        }
        Value::Object(entries) => {
            out.push_str("object:");
            write_object(entries, options, out);
        }
    }
}

/// Numbers, written the way JavaScript writes them.
///
/// This is the part of the encoding that is easiest to get subtly wrong. Rust
/// and JavaScript both print the shortest text that reads back as the same
/// number, but they disagree about *where* to put the decimal point. For
/// `1e21` Rust prints twenty-two digits; JavaScript prints `1e+21`. For `1e-7`
/// Rust prints `0.0000001`; JavaScript prints `1e-7`. Getting this wrong makes
/// every large or small number hash differently from the reference, silently.
///
/// So the rules from the ECMAScript specification are followed directly rather
/// than Rust's `Display`. The number is first reduced to its significant digits
/// and an exponent, which is the form both languages agree on, and then the
/// specification's four cases decide the layout.
///
/// The specials: `NaN`, `Infinity` and `-Infinity` are named, and negative zero
/// is `0`, because `String(-0)` is `"0"`.
fn write_number(number: f64, out: &mut String) {
    out.push_str("number:");
    out.push_str(&number_to_string(number));
}

/// The text JavaScript's `String(number)` would produce.
fn number_to_string(number: f64) -> String {
    if number.is_nan() {
        return "NaN".to_string();
    }
    if number == f64::INFINITY {
        return "Infinity".to_string();
    }
    if number == f64::NEG_INFINITY {
        return "-Infinity".to_string();
    }
    if number == 0.0 {
        // Covers -0.0: JavaScript prints it as "0".
        return "0".to_string();
    }

    let sign = if number < 0.0 { "-" } else { "" };
    // `{:e}` gives the shortest digits that read back as the same number, in the
    // form `1.23456e7`, which is the one piece both languages agree on.
    let scientific = format!("{:e}", number.abs());
    let (mantissa, exponent) = scientific.split_once('e').expect("scientific form");
    let exponent: i32 = exponent.parse().expect("exponent");

    let mut digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    while digits.len() > 1 && digits.ends_with('0') {
        digits.pop();
    }

    let digits = digits.as_str();
    let count = digits.len() as i32;
    // Where the decimal point sits, as JavaScript counts it.
    let point = exponent + 1;

    if count <= point && point <= 21 {
        // Whole number: the digits then enough zeros to reach the point.
        let mut text = String::from(digits);
        text.extend(std::iter::repeat_n('0', (point - count) as usize));
        format!("{sign}{text}")
    } else if 0 < point && point <= 21 {
        // The point falls inside the digits.
        let split = point as usize;
        format!("{sign}{}.{}", &digits[..split], &digits[split..])
    } else if -6 < point && point <= 0 {
        // Below one: a leading zero, then the point's zeros, then the digits.
        let mut text = String::from("0.");
        text.extend(std::iter::repeat_n('0', (-point) as usize));
        text.push_str(digits);
        format!("{sign}{text}")
    } else if count == 1 {
        // One digit in exponential form.
        format!(
            "{sign}{digits}e{}{}",
            exponent_sign(point - 1),
            (point - 1).abs()
        )
    } else {
        format!(
            "{sign}{}.{}e{}{}",
            &digits[..1],
            &digits[1..],
            exponent_sign(point - 1),
            (point - 1).abs()
        )
    }
}

/// JavaScript writes a plus sign on a non-negative exponent.
fn exponent_sign(exponent: i32) -> char {
    if exponent < 0 { '-' } else { '+' }
}

/// Strings carry their length, counted in UTF-16 code units.
///
/// JavaScript measures strings that way, so a character above the basic plane
/// counts as two. Counting `char`s instead would give a different length for any
/// emoji and break compatibility.
fn write_string(text: &str, out: &mut String) {
    let length: usize = text.chars().map(|c| c.len_utf16()).sum();
    let _ = write!(out, "string:{length}:");
    out.push_str(text);
}

/// Arrays keep their order and carry their length.
///
/// With `unordered_arrays` the entries are written out first, sorted as text,
/// and then written again as strings. Sorting the *written* forms rather than
/// the values is what the reference does, and it is why the result is
/// `array:N:array:N:string:…` rather than just the sorted entries.
fn write_array(items: &[Value], options: &WriteOptions, out: &mut String) {
    let _ = write!(out, "array:{}:", items.len());
    if !options.unordered_arrays || items.len() <= 1 {
        for item in items {
            write(item, options, out);
        }
        return;
    }
    let mut sorted: Vec<String> = items.iter().map(|item| canonical(item, options)).collect();
    sorted.sort_by(|a, b| utf16_cmp(a, b));
    let _ = write!(out, "array:{}:", sorted.len());
    for entry in sorted {
        write_string(&entry, out);
    }
}

/// Compare two strings the way JavaScript's default sort does.
///
/// JavaScript compares strings by UTF-16 code unit. Comparing the UTF-8 bytes
/// instead agrees for everything in the basic plane but not above it: an emoji
/// starts at U+D83D, which is below U+FFFF, while its UTF-8 bytes start at 0xF0,
/// which is above. Sorting a set holding both an emoji and U+FFFF would put them
/// in the wrong order and change the digest. The UTF-16 units have to be built
/// to get this right.
fn utf16_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// Sets and maps are unordered, so their entries are sorted.
///
/// This is where the encoding stops being obvious. The reference sorts them by
/// writing each entry out first, sorting those strings, and then writing the
/// sorted strings *as strings*, length prefix and all. So a two-entry map is
/// `array:2:` followed by `array:2:` again and then each entry quoted as a
/// string. With one entry or none there is nothing to sort and the entries are
/// written straight out, which is why a one-entry map looks different from a
/// two-entry one.
fn write_unordered(entries: Vec<String>, sorted: bool, out: &mut String) {
    let count = entries.len();
    let _ = write!(out, "array:{count}:");
    if !sorted || count <= 1 {
        for entry in entries {
            out.push_str(&entry);
        }
        return;
    }
    let mut entries = entries;
    entries.sort_by(|a, b| utf16_cmp(a, b));
    let _ = write!(out, "array:{count}:");
    for entry in entries {
        write_string(&entry, out);
    }
}

/// A set ignores the order of its entries.
///
/// Scalar entries are deduplicated, because a JavaScript `Set` does that: it
/// keeps one of each distinct value. Arrays and maps are not, because in
/// JavaScript they are compared by identity, and two equal-looking arrays are
/// still two different objects. That asymmetry is the reference's, not a choice
/// made here.
fn write_set(items: &[Value], options: &WriteOptions, out: &mut String) {
    let mut seen = HashSet::new();
    let entries = items
        .iter()
        .filter_map(|item| {
            if item.is_scalar() {
                let written = canonical(item, options);
                if !seen.insert(written.clone()) {
                    return None;
                }
                Some(written)
            } else {
                Some(canonical(item, options))
            }
        })
        .collect();
    write_unordered(entries, options.unordered_sets, out);
}

/// A map is written in key order, whatever order it was built in.
///
/// A repeated key keeps the last value, because that is what a JavaScript `Map`
/// does, and the entries are sorted by their written form rather than by the key.
/// Sorting by the written form is what makes the order stable for keys whose own
/// order would not settle it, and it is the reference's behaviour.
fn write_map(entries: &[(String, Value)], options: &WriteOptions, out: &mut String) {
    let serialized = serialized_entries(entries, options);
    write_unordered(serialized, options.unordered_sets, out);
}

/// A plain object, which the reference writes as `object:N:` with unquoted keys.
///
/// This is not the same encoding as a `Map`, and the difference is not cosmetic:
/// the reference gives an object its own `object:` prefix, sorts its keys by
/// default, and does not quote the keys. Two values that a JavaScript caller
/// would consider different - `{a: 1}` and `new Map([["a", 1]])` - hash
/// differently, and this crate matches that.
///
/// A repeated key keeps the last value, as it does for a map.
fn write_object(entries: &[(String, Value)], options: &WriteOptions, out: &mut String) {
    let mut keys: Vec<&str> = entries.iter().map(|(key, _)| key.as_str()).collect();
    if options.unordered_objects {
        keys.sort_by(|a, b| utf16_cmp(a, b));
    }
    // `Object.keys` lists a repeated key once, in first-appearance order, and
    // the lookup that follows finds its last assignment. Those are two different
    // questions, so they need two indexes: `last` answers the value, `seen`
    // answers the order. One index cannot do both, because the first occurrence
    // fixes the position while the last fixes the value.
    let mut last: HashMap<&str, &Value> = HashMap::with_capacity(entries.len());
    for (key, value) in entries {
        last.insert(key.as_str(), value);
    }
    let mut kept: Vec<&str> = Vec::with_capacity(keys.len());
    let mut seen: HashSet<&str> = HashSet::with_capacity(keys.len());
    for key in keys {
        if seen.insert(key) {
            kept.push(key);
        }
    }
    let _ = write!(out, "{}:", kept.len());
    for key in kept {
        write_string(key, out);
        out.push(':');
        if !options.exclude_values {
            let value = last.get(key).expect("key came from the entries");
            write(value, options, out);
        }
        out.push(',');
    }
}

/// The written form of each map entry, in the order the map resolves to.
///
/// A repeated key keeps the last value, because that is what a JavaScript `Map`
/// does. Unlike a plain object, a map keeps its values when `exclude_values` is
/// set: the reference's `excludeValues` is only consulted in its object writer.
fn serialized_entries(entries: &[(String, Value)], options: &WriteOptions) -> Vec<String> {
    // A repeated key keeps the last value. The map records which position each
    // key resolves to, so this is linear rather than a scan per entry.
    let mut position: HashMap<&str, usize> = HashMap::with_capacity(entries.len());
    let mut kept: Vec<(&String, &Value)> = Vec::with_capacity(entries.len());
    for (key, value) in entries {
        match position.get(key.as_str()) {
            Some(&index) => kept[index].1 = value,
            None => {
                position.insert(key.as_str(), kept.len());
                kept.push((key, value));
            }
        }
    }

    kept.iter()
        .map(|(key, value)| {
            let mut entry = String::from("array:2:");
            write_string(key, &mut entry);
            write(value, options, &mut entry);
            entry
        })
        .collect()
}

/// The canonical form of one value, on its own.
pub(crate) fn canonical(value: &Value, options: &WriteOptions) -> String {
    let mut out = String::new();
    write(value, options, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The options the crate's public functions use, and the reference's defaults.
    const DEFAULT: WriteOptions = WriteOptions {
        exclude_values: false,
        unordered_arrays: false,
        unordered_sets: true,
        unordered_objects: true,
    };

    #[test]
    fn a_null_is_the_word() {
        assert_eq!(canonical(&Value::Null, &DEFAULT), "Null");
    }

    #[test]
    fn booleans_are_labelled() {
        assert_eq!(canonical(&Value::Bool(true), &DEFAULT), "bool:true");
        assert_eq!(canonical(&Value::Bool(false), &DEFAULT), "bool:false");
    }

    #[test]
    fn integers_and_equal_floats_are_the_same() {
        assert_eq!(canonical(&Value::Number(1.0), &DEFAULT), "number:1");
        assert_eq!(canonical(&Value::Number(100.0), &DEFAULT), "number:100");
        assert_eq!(canonical(&Value::Number(-1.0), &DEFAULT), "number:-1");
    }

    #[test]
    fn negative_zero_is_written_as_zero() {
        assert_eq!(canonical(&Value::Number(-0.0), &DEFAULT), "number:0");
    }

    #[test]
    fn the_special_numbers_have_names() {
        assert_eq!(canonical(&Value::Number(f64::NAN), &DEFAULT), "number:NaN");
        assert_eq!(
            canonical(&Value::Number(f64::INFINITY), &DEFAULT),
            "number:Infinity"
        );
        assert_eq!(
            canonical(&Value::Number(f64::NEG_INFINITY), &DEFAULT),
            "number:-Infinity"
        );
    }

    #[test]
    fn a_string_carries_its_utf16_length() {
        assert_eq!(
            canonical(&Value::String(String::new()), &DEFAULT),
            "string:0:"
        );
        assert_eq!(
            canonical(&Value::String("hello".into()), &DEFAULT),
            "string:5:hello"
        );
        assert_eq!(
            canonical(&Value::String("café".into()), &DEFAULT),
            "string:4:café"
        );
        assert_eq!(
            canonical(&Value::String("中文".into()), &DEFAULT),
            "string:2:中文"
        );
        // An emoji is one character but two UTF-16 code units.
        assert_eq!(
            canonical(&Value::String("😀".into()), &DEFAULT),
            "string:2:😀"
        );
    }

    #[test]
    fn an_array_keeps_its_order() {
        assert_eq!(canonical(&Value::Array(vec![]), &DEFAULT), "array:0:");
        assert_eq!(
            canonical(
                &Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]),
                &DEFAULT
            ),
            "array:2:number:1number:2"
        );
        assert_eq!(
            canonical(
                &Value::Array(vec![Value::Number(2.0), Value::Number(1.0)]),
                &DEFAULT
            ),
            "array:2:number:2number:1"
        );
    }

    #[test]
    fn a_map_is_written_in_key_order() {
        let forward = Value::Map(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(2.0)),
        ]);
        let backward = Value::Map(vec![
            ("b".into(), Value::Number(2.0)),
            ("a".into(), Value::Number(1.0)),
        ]);
        assert_eq!(
            canonical(&forward, &DEFAULT),
            canonical(&backward, &DEFAULT)
        );
    }

    #[test]
    fn a_set_ignores_order_but_an_array_does_not() {
        let one = Value::Number(1.0);
        let two = Value::Number(2.0);
        let set_forward = Value::Set(vec![one.clone(), two.clone()]);
        let set_backward = Value::Set(vec![two.clone(), one.clone()]);
        assert_eq!(
            canonical(&set_forward, &DEFAULT),
            canonical(&set_backward, &DEFAULT)
        );

        let array_forward = Value::Array(vec![one.clone(), two.clone()]);
        let array_backward = Value::Array(vec![two, one]);
        assert_ne!(
            canonical(&array_forward, &DEFAULT),
            canonical(&array_backward, &DEFAULT)
        );
    }

    #[test]
    fn a_set_drops_repeated_scalars_but_not_repeated_arrays() {
        // A JavaScript Set keeps one of each distinct scalar.
        assert_eq!(
            canonical(
                &Value::Set(vec![Value::Number(1.0), Value::Number(1.0)]),
                &DEFAULT
            ),
            "set:array:1:number:1"
        );
        assert_eq!(
            canonical(
                &Value::Set(vec![Value::Number(f64::NAN), Value::Number(f64::NAN)]),
                &DEFAULT
            ),
            "set:array:1:number:NaN"
        );
        // Two equal arrays are still two objects, so both are kept.
        let array = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        let two_arrays = Value::Set(vec![array.clone(), array]);
        assert!(canonical(&two_arrays, &DEFAULT).starts_with("set:array:2:"));
    }

    #[test]
    fn entries_are_ordered_by_utf16_code_unit() {
        // An emoji begins at U+D83D, below U+FFFF, so it sorts first. Its UTF-8
        // bytes begin at 0xF0, above the UTF-8 bytes of U+FFFF, so a byte
        // comparison would put them the other way round.
        let emoji_first = Value::Set(vec![
            Value::String("\u{ffff}z".into()),
            Value::String("\u{1f600}".into()),
        ]);
        let emoji_last = Value::Set(vec![
            Value::String("\u{1f600}".into()),
            Value::String("\u{ffff}z".into()),
        ]);
        assert_eq!(
            canonical(&emoji_first, &DEFAULT),
            canonical(&emoji_last, &DEFAULT)
        );
        let written = canonical(&emoji_first, &DEFAULT);
        let emoji_at = written.find('\u{1f600}').expect("emoji present");
        let max_at = written.find('\u{ffff}').expect("U+FFFF present");
        assert!(
            emoji_at < max_at,
            "emoji should sort before U+FFFF: {written}"
        );
    }

    #[test]
    fn a_repeated_map_key_keeps_the_last_value() {
        let repeated = Value::Map(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(9.0)),
            ("a".into(), Value::Number(2.0)),
        ]);
        let resolved = Value::Map(vec![
            ("a".into(), Value::Number(2.0)),
            ("b".into(), Value::Number(9.0)),
        ]);
        assert_eq!(
            canonical(&repeated, &DEFAULT),
            canonical(&resolved, &DEFAULT)
        );
    }

    #[test]
    fn a_map_key_cannot_be_confused_with_a_value() {
        // The length prefix is what stops `{"ab": 1}` and `{"a": "b1"}` from
        // writing the same bytes.
        let first = Value::Map(vec![("ab".into(), Value::Number(1.0))]);
        let second = Value::Map(vec![("a".into(), Value::String("b1".into()))]);
        assert_ne!(canonical(&first, &DEFAULT), canonical(&second, &DEFAULT));
    }

    #[test]
    fn an_object_is_not_a_map() {
        // The reference writes a plain object as `object:N:` with unquoted keys
        // and a Map as `map:array:…`, so the same entry gives two digests. This
        // crate keeps both cases rather than collapsing them.
        let object = Value::Object(vec![("a".into(), Value::Number(1.0))]);
        let map = Value::Map(vec![("a".into(), Value::Number(1.0))]);
        assert_eq!(
            canonical(&object, &DEFAULT),
            "object:1:string:1:a:number:1,"
        );
        assert_eq!(
            canonical(&map, &DEFAULT),
            "map:array:1:array:2:string:1:anumber:1"
        );
        assert_ne!(canonical(&object, &DEFAULT), canonical(&map, &DEFAULT));
    }

    #[test]
    fn an_object_is_written_in_key_order() {
        let forward = Value::Object(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(2.0)),
        ]);
        let backward = Value::Object(vec![
            ("b".into(), Value::Number(2.0)),
            ("a".into(), Value::Number(1.0)),
        ]);
        assert_eq!(
            canonical(&forward, &DEFAULT),
            canonical(&backward, &DEFAULT)
        );
    }

    #[test]
    fn an_object_keeps_the_last_value_for_a_repeated_key() {
        // A JavaScript object literal cannot hold a duplicate key, so the fuzz
        // corpus never reaches this; a Rust caller can build it, and the
        // reference's lookup would find the last assignment.
        let repeated = Value::Object(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::Number(9.0)),
            ("a".into(), Value::Number(2.0)),
        ]);
        let resolved = Value::Object(vec![
            ("a".into(), Value::Number(2.0)),
            ("b".into(), Value::Number(9.0)),
        ]);
        assert_eq!(
            canonical(&repeated, &DEFAULT),
            canonical(&resolved, &DEFAULT)
        );
    }

    #[test]
    fn exclude_values_writes_only_the_keys_of_an_object() {
        let options = WriteOptions {
            exclude_values: true,
            ..DEFAULT
        };
        let object = Value::Object(vec![
            ("a".into(), Value::Number(1.0)),
            ("b".into(), Value::String("x".into())),
        ]);
        assert_eq!(
            canonical(&object, &options),
            "object:2:string:1:a:,string:1:b:,"
        );
        // A map keeps its values: the reference consults `excludeValues` only
        // in its object writer.
        let map = Value::Map(vec![("a".into(), Value::Number(1.0))]);
        assert_eq!(
            canonical(&map, &options),
            "map:array:1:array:2:string:1:anumber:1"
        );
    }

    #[test]
    fn unordered_arrays_sorts_the_entries() {
        let options = WriteOptions {
            unordered_arrays: true,
            ..DEFAULT
        };
        let forward = Value::Array(vec![Value::Number(2.0), Value::Number(1.0)]);
        let backward = Value::Array(vec![Value::Number(1.0), Value::Number(2.0)]);
        assert_ne!(
            canonical(&forward, &DEFAULT),
            canonical(&backward, &DEFAULT)
        );
        assert_eq!(
            canonical(&forward, &options),
            canonical(&backward, &options)
        );
        assert_eq!(
            canonical(&forward, &options),
            "array:2:array:2:string:8:number:1string:8:number:2"
        );
    }

    #[test]
    fn unordered_sets_off_keeps_the_insertion_order() {
        let options = WriteOptions {
            unordered_sets: false,
            ..DEFAULT
        };
        let forward = Value::Set(vec![Value::Number(1.0), Value::Number(2.0)]);
        let backward = Value::Set(vec![Value::Number(2.0), Value::Number(1.0)]);
        assert_eq!(
            canonical(&forward, &DEFAULT),
            canonical(&backward, &DEFAULT)
        );
        assert_ne!(
            canonical(&forward, &options),
            canonical(&backward, &options)
        );
        assert_eq!(
            canonical(&forward, &options),
            "set:array:2:number:1number:2"
        );
    }
}
