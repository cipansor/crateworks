//! Plain objects and the writer options, against the reference.
//!
//! The fuzz corpus reaches plain objects only by chance, and always with the
//! default options. This suite is 2,000 objects on purpose, each one hashed with
//! a random mix of the reference's `excludeValues` and `unorderedArrays`, so the
//! `object:` encoding and the option paths are pinned and not only the default.
//!
//! The reference's `respectType` is off in every row. It is on by default there
//! and prepends a description of Node's prototype chain to every plain object,
//! which cannot be reproduced outside JavaScript; see the crate documentation.
//! That is why this crate has [`object_hash::Value::Object`] and not just a map.

use object_hash::{Options, Value, canonical_with, hash_with_options};

mod support;
use support::Reader;

/// A row: the value, the option flags, the canonical form, and the digest.
struct Case {
    description: String,
    exclude_values: bool,
    unordered_arrays: bool,
    canonical: String,
    digest: String,
}

/// Reads the fixture, which is tab separated to avoid a JSON dependency.
fn cases() -> Vec<Case> {
    include_str!("fixtures/objects.tsv")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 4, "malformed line: {line:?}");
            let flags: Vec<char> = fields[1].chars().collect();
            Case {
                description: fields[0].to_string(),
                exclude_values: flags[0] == '1',
                unordered_arrays: flags[1] == '1',
                canonical: fields[2].to_string(),
                digest: fields[3].to_string(),
            }
        })
        .collect()
}

fn options_for(case: &Case) -> Options {
    Options {
        exclude_values: case.exclude_values,
        unordered_arrays: case.unordered_arrays,
        ..Options::default()
    }
}

#[test]
fn every_object_matches_the_reference() {
    let cases = cases();
    assert!(cases.len() >= 2000, "only {} cases", cases.len());

    let mut failures = Vec::new();
    for case in &cases {
        let mut reader = Reader::new(&case.description);
        let value = reader.value();
        assert_eq!(
            reader.at,
            case.description.len(),
            "not all of {:?} was read",
            case.description
        );

        let options = options_for(case);
        let got = canonical_with(&value, &options);
        if got != case.canonical {
            failures.push(format!(
                "{}\n  canonical\n    expected {:?}\n    got      {got:?}",
                case.description, case.canonical
            ));
            continue;
        }
        let got_digest = hash_with_options(&value, options);
        if got_digest != case.digest {
            failures.push(format!(
                "{}\n  digest\n    expected {}\n    got      {got_digest}",
                case.description, case.digest
            ));
        }
    }

    assert!(
        failures.is_empty(),
        "{} failures, first few:\n{}",
        failures.len(),
        failures
            .iter()
            .take(10)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

#[test]
fn an_object_and_a_map_with_the_same_entry_differ() {
    // The reference encodes a plain object as `object:N:` and a Map as
    // `map:…`, so the two are not interchangeable and a caller has to pick the
    // one the other side of the conversation uses.
    let object = Value::Object(vec![("a".into(), Value::number(1))]);
    let map = Value::Map(vec![("a".into(), Value::number(1))]);
    assert_ne!(
        canonical_with(&object, &Options::default()),
        canonical_with(&map, &Options::default())
    );
}

#[test]
fn keys_ignores_the_values_of_an_object() {
    let options = Options::keys();
    let one = Value::Object(vec![
        ("a".into(), Value::number(1)),
        ("b".into(), Value::number(2)),
    ]);
    let two = Value::Object(vec![
        ("a".into(), Value::number(9)),
        ("b".into(), Value::Null),
    ]);
    assert_eq!(
        canonical_with(&one, &options),
        canonical_with(&two, &options)
    );
    // But a map keeps its values even with the same options, as the reference's
    // `excludeValues` is only consulted by its object writer.
    let map_one = Value::Map(vec![("a".into(), Value::number(1))]);
    let map_two = Value::Map(vec![("a".into(), Value::number(2))]);
    assert_ne!(
        canonical_with(&map_one, &options),
        canonical_with(&map_two, &options)
    );
}

#[test]
fn an_object_is_written_with_unquoted_keys() {
    let object = Value::Object(vec![("a".into(), Value::number(1))]);
    assert_eq!(
        canonical_with(&object, &Options::default()),
        "object:1:string:1:a:number:1,"
    );
}

#[test]
fn unordered_arrays_makes_order_stop_mattering() {
    let forward = Value::Array(vec![Value::number(1), Value::number(2)]);
    let backward = Value::Array(vec![Value::number(2), Value::number(1)]);
    assert_ne!(
        canonical_with(&forward, &Options::default()),
        canonical_with(&backward, &Options::default())
    );
    let options = Options::default().unordered_arrays();
    assert_eq!(
        canonical_with(&forward, &options),
        canonical_with(&backward, &options)
    );
}
