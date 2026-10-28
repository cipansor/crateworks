//! A corpus of randomly built values, hashed by the reference and by this crate.
//!
//! The curated fixtures cover the cases somebody thought of. This covers the
//! ones nobody did: 4,000 values built at random out of nesting arrays, sets and
//! maps, mixing numbers at both ends of the range, strings that are empty or
//! hold emoji, CJK or U+FFFF, and repeated keys and entries. The reference
//! `object-hash` npm package wrote both the canonical form and the digest for
//! each one.
//!
//! Two real bugs came out of this corpus and nowhere else:
//!
//! * Numbers at the edges of the range: Rust writes `1e21` as twenty-two digits
//!   where JavaScript writes `1e+21`, and `1e-7` as `0.0000001` where JavaScript
//!   writes `1e-7`.
//! * Ordering entries above the basic plane: JavaScript sorts by UTF-16 code
//!   unit, so an emoji (U+D83D…) sorts *before* U+FFFF. Comparing UTF-8 bytes
//!   reverses those two, which changed the digest of any set holding both.
//!
//! The descriptions are a small tagged JSON, parsed here rather than pulled in
//! as a dependency, since this crate has none.

use object_hash::{Algorithm, canonical, hash, hash_with};

mod support;
use support::Reader;

const CORPUS: &str = include_str!("fixtures/fuzz.tsv");

/// Each line is a description, the reference's canonical form, and its digest.
fn corpus() -> Vec<(String, String, String)> {
    CORPUS
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 3, "malformed line: {line:?}");
            (
                fields[0].to_string(),
                fields[1].to_string(),
                fields[2].to_string(),
            )
        })
        .collect()
}

#[test]
fn every_random_value_matches_the_reference() {
    let cases = corpus();
    assert!(cases.len() >= 4000, "only {} cases", cases.len());

    let mut failures = Vec::new();
    for (description, canonical_form, digest) in &cases {
        let mut reader = Reader::new(description);
        let value = reader.value();
        assert_eq!(
            reader.at,
            description.len(),
            "not all of {description:?} was read"
        );

        let got_canonical = canonical(&value);
        if got_canonical != *canonical_form {
            failures.push(format!(
                "{description}\n  canonical\n    expected {canonical_form:?}\n    got      {got_canonical:?}"
            ));
            continue;
        }
        let got_digest = hash(&value);
        if got_digest != *digest {
            failures.push(format!(
                "{description}\n  digest\n    expected {digest}\n    got      {got_digest}"
            ));
        }
        // The reference's `passthrough` algorithm, held to the same canonical
        // form. It is the identity, so it agrees with the reference for free,
        // but this keeps it from being forgotten if the encoding ever changes.
        let got_passthrough = hash_with(&value, Algorithm::Passthrough);
        if got_passthrough != *canonical_form {
            failures.push(format!(
                "{description}\n  passthrough\n    expected {canonical_form:?}\n    got      {got_passthrough:?}"
            ));
        }
    }

    assert!(
        failures.len() < 10,
        "{} failures, first few:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(
        failures.is_empty(),
        "{} failures:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
