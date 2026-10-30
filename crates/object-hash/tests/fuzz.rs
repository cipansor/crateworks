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

/// Each line is a description, the reference's canonical form, its default
/// SHA-1 digest, and five more digests of the same bytes: SHA3-256 plus the six
/// the crate ported by hand, which Node `crypto` also computes.
struct Case {
    description: String,
    canonical: String,
    sha1: String,
    sha3_256: String,
    sha512_224: String,
    sha512_256: String,
    ripemd160: String,
    sm3: String,
    blake2b512: String,
    blake2s256: String,
    shake128: String,
    shake256: String,
}

fn corpus() -> Vec<Case> {
    CORPUS
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let fields: Vec<&str> = line.split('\t').collect();
            assert_eq!(fields.len(), 12, "malformed line: {line:?}");
            Case {
                description: fields[0].to_string(),
                canonical: fields[1].to_string(),
                sha1: fields[2].to_string(),
                sha3_256: fields[3].to_string(),
                sha512_224: fields[4].to_string(),
                sha512_256: fields[5].to_string(),
                ripemd160: fields[6].to_string(),
                sm3: fields[7].to_string(),
                blake2b512: fields[8].to_string(),
                blake2s256: fields[9].to_string(),
                shake128: fields[10].to_string(),
                shake256: fields[11].to_string(),
            }
        })
        .collect()
}

#[test]
fn every_random_value_matches_the_reference() {
    let cases = corpus();
    assert!(cases.len() >= 4000, "only {} cases", cases.len());

    let mut failures = Vec::new();
    for case in &cases {
        let description = &case.description;
        let canonical_form = &case.canonical;
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
        if got_digest != case.sha1 {
            failures.push(format!(
                "{description}\n  digest\n    expected {}\n    got      {got_digest}",
                case.sha1
            ));
        }
        // Every hand-ported digest over the same canonical bytes, so a varied
        // corpus exercises each absorb loop at many lengths, not only the two
        // the published vectors cover. SHA3-256 is here because the reference
        // reaches it the same way; the other six because the crate ported them
        // by hand and a wrong constant or rotation would show up somewhere in
        // four thousand lengths.
        for (algorithm, expected) in [
            (Algorithm::Sha3_256, &case.sha3_256),
            (Algorithm::Sha512_224, &case.sha512_224),
            (Algorithm::Sha512_256, &case.sha512_256),
            (Algorithm::Ripemd160, &case.ripemd160),
            (Algorithm::Sm3, &case.sm3),
            (Algorithm::Blake2b512, &case.blake2b512),
            (Algorithm::Blake2s256, &case.blake2s256),
            (Algorithm::Shake128, &case.shake128),
            (Algorithm::Shake256, &case.shake256),
        ] {
            let got = hash_with(&value, algorithm);
            if &got != expected {
                failures.push(format!(
                    "{description}\n  {algorithm:?}\n    expected {expected}\n    got      {got}"
                ));
            }
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
