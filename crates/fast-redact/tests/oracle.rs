//! The corpus gate: every recorded case must reproduce the reference exactly.
//!
//! `tests/fixtures/fuzz.tsv` and `tests/fixtures/paths.tsv` were produced by
//! `scripts/fuzz/fast-redact/gen.js`, which asked the npm package `fast-redact`
//! for the answer to a few thousand inputs. This test replays them through the
//! crate. It is the thing that turns "the cases we thought of are right" into
//! "the cases we checked agree".
//!
//! The reference is not a dependency; the answers are recorded, not computed
//! here. The generator can rebuild the files.

use fast_redact::{Redactor, Value};

/// Paths the reference accepts but this crate refuses, on purpose.
///
/// The reference validates a path by compiling it as JavaScript, so it accepts
/// anything that evaluates — a global object (`a[Math]`), `this`, arithmetic
/// (`a[1+1]`), an array literal (`a[[]]`). This crate never turns a path into
/// code and refuses those. Each names a key no JSON document can have, so no
/// redaction is lost. Listed here so the divergence is asserted, not assumed:
/// if it ever grows, this test fails.
const DIVERGENT: &[&str] = &[
    // The reference accepts these because they are JavaScript that evaluates:
    // `this`, a global object, arithmetic, an array literal, or a keyword that
    // is a valid property name. None is a key a JSON document can hold.
    "a[this]",
    "a[Math]",
    "a[1+1]",
    "a[[]]",
    "a[true]",
    "a[null]",
    "a[undefined]",
    "a[NaN]",
    "a[Infinity]",
];

#[test]
fn reference_redactions_are_reproduced() {
    let fixture = include_str!("fixtures/fuzz.tsv");
    let mut cases = 0usize;
    let mut failures = Vec::new();

    for (number, line) in fixture.lines().enumerate() {
        let mut fields = line.split('\t');
        let mode = fields.next().expect("mode");
        let paths = fields.next().expect("paths");
        let input = fields.next().expect("input");
        let expected = fields.next().expect("output");

        let paths: Vec<String> =
            serde_free_json_string_array(paths).expect("a JSON array of strings");
        let document = Value::parse(input).expect("the recorded input parses");

        let mut redactor = Redactor::new(paths.clone()).expect("the recorded paths are valid");
        if mode == "remove" {
            redactor = redactor.with_remove();
        }

        let got = redactor.redact(&document);
        cases += 1;
        if got != expected {
            failures.push(format!(
                "line {}: paths {paths:?}\n  input    {input}\n  expected {expected}\n  got      {got}",
                number + 1
            ));
        }
    }

    assert!(cases > 5000, "the corpus is too small: {cases} cases");
    assert!(
        failures.is_empty(),
        "{} of {cases} cases diverged from the reference:\n\n{}",
        failures.len(),
        failures.join("\n\n")
    );
}

#[test]
fn path_acceptance_matches_the_reference() {
    let fixture = include_str!("fixtures/paths.tsv");
    let mut divergences = Vec::new();

    for line in fixture.lines() {
        let (verdict, path) = line.split_once('\t').expect("verdict and path");
        let path = serde_free_json_string(path).expect("a JSON string path");
        let reference_accepts = verdict == "accept";
        let we_accept = fast_redact::parse_path(&path).is_ok();
        if reference_accepts != we_accept {
            divergences.push((path, reference_accepts, we_accept));
        }
    }

    let unexpected: Vec<_> = divergences
        .iter()
        .filter(|(path, _, _)| !DIVERGENT.contains(&path.as_str()))
        .collect();
    assert!(
        unexpected.is_empty(),
        "paths where acceptance diverged from the reference and are not in the \
         documented list:\n{unexpected:#?}"
    );

    // And the documented list must be exactly the divergence, so a path that
    // stops diverging (or a new one that starts) is noticed.
    let actual: Vec<String> = divergences.iter().map(|(p, _, _)| p.clone()).collect();
    for path in DIVERGENT {
        assert!(
            actual.iter().any(|p| p == path),
            "the documented divergence {path:?} no longer diverges; update the list"
        );
    }
}

/// Parse one JSON string, for the path column.
fn serde_free_json_string(text: &str) -> Option<String> {
    let mut chars = text.chars();
    if chars.next()? != '"' || chars.next_back()? != '"' {
        return None;
    }
    let mut out = String::new();
    let mut chars = text[1..text.len() - 1].chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next()? {
                '"' => out.push('"'),
                '\\' => out.push('\\'),
                '/' => out.push('/'),
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                'u' => {
                    let hex: String = (0..4).filter_map(|_| chars.next()).collect();
                    let code = u32::from_str_radix(&hex, 16).ok()?;
                    out.push(char::from_u32(code)?);
                }
                other => out.push(other),
            }
        } else {
            out.push(c);
        }
    }
    Some(out)
}

/// Parse a JSON array of strings without pulling in a JSON library.
///
/// The path column is always `["a","b"]` with no escapes beyond `\"` and `\\`,
/// so a small reader is enough and keeps this test dependency-free.
fn serde_free_json_string_array(text: &str) -> Option<Vec<String>> {
    let inner = text.strip_prefix('[')?.strip_suffix(']')?;
    if inner.is_empty() {
        return Some(Vec::new());
    }
    let mut out = Vec::new();
    let mut chars = inner.chars().peekable();
    while chars.peek().is_some() {
        // Opening quote.
        if chars.next() != Some('"') {
            return None;
        }
        let mut item = String::new();
        loop {
            match chars.next()? {
                '"' => break,
                '\\' => match chars.next()? {
                    '"' => item.push('"'),
                    '\\' => item.push('\\'),
                    '/' => item.push('/'),
                    'n' => item.push('\n'),
                    't' => item.push('\t'),
                    'r' => item.push('\r'),
                    other => item.push(other),
                },
                c => item.push(c),
            }
        }
        out.push(item);
        match chars.peek() {
            Some(',') => {
                chars.next();
            }
            Some(']') | None => break,
            _ => return None,
        }
    }
    Some(out)
}
