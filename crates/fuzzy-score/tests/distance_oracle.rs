//! Checks every distance against values produced by `rapidfuzz` 3.14 and
//! `jellyfish`, cross-checked against each other.
//!
//! The fixture is one case per line, tab separated:
//!
//! ```text
//! a   b   lev  damerau  osa  hamming  jaro  jw  lev_norm
//! ```
//!
//! `a` and `b` use the same escaping as `oracle.tsv`. The seven numbers after
//! them are what `rapidfuzz` returned; `scripts/fuzz/fuzzy-score/gen_distance.py`
//! also asked `jellyfish` for the five metrics it defines and refused to write
//! the file if the two disagreed, except for Jaro of two empty strings, where
//! they differ by design and this crate follows `rapidfuzz`.
//!
//! Floats are written with Python's `repr`, which round-trips exactly, so the
//! comparison here is exact rather than approximate.

use fuzzy_score::distance;

/// One line of the fixture.
struct Case {
    a: String,
    b: String,
    values: [f64; 7],
}

/// Undoes the escaping the generator applied.
fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('\\') => out.push('\\'),
            Some('t') => out.push('\t'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('u') => {
                let hex: String = chars.by_ref().take(4).collect();
                let cp = u32::from_str_radix(&hex, 16).expect("bad \\u escape");
                out.push(char::from_u32(cp).expect("bad codepoint"));
            }
            Some('U') => {
                let hex: String = chars.by_ref().take(8).collect();
                let cp = u32::from_str_radix(&hex, 16).expect("bad \\U escape");
                out.push(char::from_u32(cp).expect("bad codepoint"));
            }
            other => panic!("bad escape: {other:?}"),
        }
    }
    out
}

fn load() -> Vec<Case> {
    let text = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/distance.tsv"),
    )
    .expect("distance fixture is missing");

    let mut cases = Vec::new();
    for line in text.lines() {
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        assert_eq!(fields.len(), 9, "malformed fixture line: {line:?}");
        let mut values = [0.0f64; 7];
        for (i, field) in fields[2..].iter().enumerate() {
            values[i] = field.parse().expect("value is not a number");
        }
        cases.push(Case {
            a: unescape(fields[0]),
            b: unescape(fields[1]),
            values,
        });
    }
    assert!(cases.len() > 5000, "fixture looks truncated");
    cases
}

#[test]
fn matches_the_two_reference_implementations() {
    let cases = load();
    let mut failures = 0;
    for case in &cases {
        let (a, b) = (case.a.as_str(), case.b.as_str());
        let got = [
            distance::levenshtein(a, b) as f64,
            distance::damerau_levenshtein(a, b) as f64,
            distance::osa(a, b) as f64,
            distance::hamming(a, b) as f64,
            distance::jaro(a, b),
            distance::jaro_winkler(a, b),
            distance::levenshtein_similarity(a, b),
        ];
        if got != case.values {
            failures += 1;
            if failures <= 10 {
                eprintln!(
                    "mismatch for {a:?} vs {b:?}\n  expected {:?}\n  got      {:?}",
                    case.values, got
                );
            }
        }
    }
    assert_eq!(
        failures,
        0,
        "{failures} of {} cases differ from the reference implementations",
        cases.len()
    );
}
