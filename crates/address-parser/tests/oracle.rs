//! Compares the crate against the reference implementation, case by case.
//!
//! `tests/fixtures/oracle.tsv` holds one input per line with the answer the
//! reference gave, produced by `scripts/fuzz/address-parser/gen.js`. The first
//! column is the input and the second is either `null` or the reference's fields
//! as `key=value` pairs sorted by key and joined with `;`. Both columns escape a
//! backslash, a tab, a newline, a carriage return, a semicolon and an equals
//! sign, so a field value containing one of those cannot break the layout.
//!
//! The corpus is the reference's own test table plus a few thousand generated
//! addresses and a dozen degenerate inputs. It is the measurement: the crate
//! either reproduces every one of them or it does not.

use std::collections::BTreeMap;

use address_parser::{Address, Component, parse_location};

/// Undoes the escaping `gen.js` applied.
fn unescape(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// Parses the second column into the reference's fields.
fn expected(answer: &str) -> Option<BTreeMap<String, String>> {
    if answer == "null" {
        return None;
    }
    let mut fields = BTreeMap::new();
    if answer.is_empty() {
        return Some(fields);
    }
    for pair in answer.split(';') {
        let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
        fields.insert(unescape(key), unescape(value));
    }
    Some(fields)
}

/// The crate's result as the same shape, so the two can be compared directly.
fn actual(address: &Address) -> BTreeMap<String, String> {
    address
        .iter()
        .map(|(component, value)| (component.name().to_string(), value.to_string()))
        .collect()
}

#[test]
fn every_labelled_address_agrees_with_the_reference() {
    let fixture = include_str!("fixtures/oracle.tsv");
    let mut agree = 0u32;
    let mut disagreements = Vec::new();
    for line in fixture.lines() {
        let Some((input, answer)) = line.split_once('\t') else {
            continue;
        };
        let input = unescape(input);
        let want = expected(answer);
        let got = parse_location(&input).map(|address| actual(&address));
        if got == want {
            agree += 1;
        } else {
            disagreements.push(format!("{input:?}\n  want {want:?}\n  got  {got:?}"));
        }
    }
    assert!(agree > 3_900, "only {agree} cases were compared");
    assert!(
        disagreements.is_empty(),
        "{} addresses disagree with the reference:\n{}",
        disagreements.len(),
        disagreements
            .iter()
            .take(20)
            .cloned()
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// The reference's own test table, checked field by field.
///
/// This is the part of the corpus a reader can verify by eye against the
/// reference's `test.js`, so it is worth failing loudly on its own.
#[test]
fn the_reference_test_table_is_reproduced() {
    let cases: [(&str, &[(&str, &str)]); 6] = [
        (
            "1005 Gravenstein Hwy 95472",
            &[
                ("number", "1005"),
                ("street", "Gravenstein"),
                ("type", "Hwy"),
                ("zip", "95472"),
            ],
        ),
        (
            "1005 Gravenstein Hwy N, 95472",
            &[
                ("number", "1005"),
                ("street", "Gravenstein"),
                ("type", "Hwy"),
                ("suffix", "N"),
                ("zip", "95472"),
            ],
        ),
        (
            "1005 N Gravenstein Highway, Sebastopol, CA",
            &[
                ("number", "1005"),
                ("prefix", "N"),
                ("street", "Gravenstein"),
                ("type", "Hwy"),
                ("city", "Sebastopol"),
                ("state", "CA"),
            ],
        ),
        (
            "1005 N Gravenstein Highway, Suite 500, Sebastopol, CA",
            &[
                ("number", "1005"),
                ("prefix", "N"),
                ("street", "Gravenstein"),
                ("type", "Hwy"),
                ("sec_unit_type", "Suite"),
                ("sec_unit_num", "500"),
                ("city", "Sebastopol"),
                ("state", "CA"),
            ],
        ),
        (
            "P.O. Box 1234, Dallas, TX 75201",
            &[
                ("sec_unit_type", "PO Box"),
                ("sec_unit_num", "1234"),
                ("city", "Dallas"),
                ("state", "TX"),
                ("zip", "75201"),
            ],
        ),
        (
            "Mission & Valencia San Francisco CA",
            &[
                ("street1", "Mission"),
                ("street2", "Valencia"),
                ("city", "San Francisco"),
                ("state", "CA"),
                ("type1", ""),
                ("type2", ""),
            ],
        ),
    ];
    for (input, fields) in cases {
        let address = parse_location(input).unwrap_or_else(|| panic!("no parse for {input:?}"));
        for (name, value) in fields {
            let component = Component::from_name(name).unwrap();
            assert_eq!(
                address.get(component),
                Some(*value),
                "{input:?} field {name}"
            );
        }
    }
}

/// An intersection reports each street separately, including its direction.
#[test]
fn an_intersection_reports_both_streets() {
    use address_parser::{Component, parse_location};

    let address = parse_location("N Main St & S Oak Ave, Springfield IL").unwrap();
    assert_eq!(address.get(Component::Street1), Some("Main"));
    assert_eq!(address.get(Component::Prefix1), Some("N"));
    assert_eq!(address.get(Component::Type1), Some("St"));
    assert_eq!(address.get(Component::Street2), Some("Oak"));
    assert_eq!(address.get(Component::Prefix2), Some("S"));
    assert_eq!(address.get(Component::Type2), Some("Ave"));
    assert_eq!(address.city(), Some("Springfield"));
    assert_eq!(address.state(), Some("IL"));

    // A trailing directional belongs to the street it follows.
    let address = parse_location("Main St N & Oak Ave S, Springfield IL").unwrap();
    assert_eq!(address.get(Component::Suffix1), Some("N"));
    assert_eq!(address.get(Component::Suffix2), Some("S"));

    // Both type slots are present even when the input had no street type.
    let address = parse_location("Mission & Valencia San Francisco CA").unwrap();
    assert_eq!(address.get(Component::Type1), Some(""));
    assert_eq!(address.get(Component::Type2), Some(""));
}

/// Nothing matched means nothing matched, which is the reference's `null`.
#[test]
fn an_unparseable_input_gives_nothing() {
    assert!(parse_location("").is_none());
}

/// A field the input did not carry is absent, not empty.
#[test]
fn absent_fields_are_absent() {
    let address = parse_location("1005 Gravenstein Hwy 95472").unwrap();
    assert_eq!(address.city(), None);
    assert_eq!(address.state(), None);
    assert_eq!(address.get(Component::Prefix), None);
}
