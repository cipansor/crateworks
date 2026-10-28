//! Comparison of the *writing* side against Python's `javaproperties`.
//!
//! `oracle.rs` checks the reader: it feeds documents through the reference and
//! compares the pairs. A corpus of inputs can only ever check a reader, so the
//! writer is pinned from the other side, by recording the reference's own output
//! for the writer API.
//!
//! The fixtures are built by `scripts/fuzz/javaproperties/gen_writer.py`, which
//! calls `javaproperties.escape`, `join_key_value`, `to_comment`,
//! `java_timestamp` and `dumps(separator=, comments=, sort_keys=,
//! ensure_ascii=)`. Every expectation is the reference's; none is hand-written.
//!
//! One deliberate difference is encoded in the fixture rather than hidden: the
//! reference's `java_timestamp` renders the local timezone, and this crate
//! renders UTC because it has no timezone database. The generator runs under
//! `TZ=UTC` and the crate's function is named `java_timestamp_utc`.

use javaproperties::{
    Properties, WriteOptions, escape, java_timestamp_utc, join_key_value, to_comment, unescape,
};

/// Decodes the escaping the generator writes.
///
/// The generator emits `\n`, `\t`, `\r`, `\f`, `\xNN`, `\uNNNN` and `\UNNNNNNNN`,
/// so that control characters and non-ASCII survive an editing pass.
fn decode(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            Some('f') => out.push('\u{000c}'),
            Some('\\') => out.push('\\'),
            Some('x') => {
                let digits: String = (0..2).filter_map(|_| chars.next()).collect();
                let value = u32::from_str_radix(&digits, 16).unwrap_or(0xfffd);
                out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
            }
            Some('u') => {
                let digits: String = (0..4).filter_map(|_| chars.next()).collect();
                let value = u32::from_str_radix(&digits, 16).unwrap_or(0xfffd);
                out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
            }
            Some('U') => {
                let digits: String = (0..8).filter_map(|_| chars.next()).collect();
                let value = u32::from_str_radix(&digits, 16).unwrap_or(0xfffd);
                out.push(char::from_u32(value).unwrap_or('\u{fffd}'));
            }
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// The rows of a fixture, header skipped.
///
/// Only a genuinely empty line is skipped, which is the trailing one the
/// generator leaves. A row whose fields are all empty is a real case - an empty
/// input escapes to an empty string, and an empty pair list writes an empty
/// document - and it is still a row of tabs, so filtering on `trim().is_empty()`
/// would silently drop exactly those cases.
fn rows(text: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    text.lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\t').collect())
}

#[test]
fn escape_matches_the_reference() {
    let mut checked = 0;
    let mut failures = Vec::new();
    let sources = [
        include_str!("fixtures/escape.tsv"),
        include_str!("fixtures/escape-random.tsv"),
    ];
    for fields in sources.into_iter().flat_map(rows) {
        let input = decode(fields[0]);
        let expected = decode(fields[1]);
        let got = escape(&input);
        if got != expected {
            failures.push(format!("{input:?}\n  expected {expected:?}\n  got {got:?}"));
        }
        checked += 1;
    }
    assert!(checked > 2_000, "only {checked} cases");
    assert!(
        failures.is_empty(),
        "{} of {checked} escape cases disagreed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn join_key_value_matches_the_reference() {
    let mut checked = 0;
    let mut failures = Vec::new();
    let sources = [
        include_str!("fixtures/join.tsv"),
        include_str!("fixtures/join-random.tsv"),
    ];
    for fields in sources.into_iter().flat_map(rows) {
        let key = decode(fields[0]);
        let value = decode(fields[1]);
        let separator = decode(fields[2]);
        let expected = decode(fields[3]);
        let got = join_key_value(&key, &value, &separator);
        if got != expected {
            failures.push(format!(
                "{key:?} {separator:?} {value:?}\n  expected {expected:?}\n  got {got:?}"
            ));
        }
        checked += 1;
    }
    assert!(checked > 2_000, "only {checked} cases");
    assert!(
        failures.is_empty(),
        "{} of {checked} join cases disagreed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn to_comment_matches_the_reference() {
    let mut checked = 0;
    let mut failures = Vec::new();
    let sources = [
        include_str!("fixtures/comment.tsv"),
        include_str!("fixtures/comment-random.tsv"),
    ];
    for fields in sources.into_iter().flat_map(rows) {
        let input = decode(fields[0]);
        let expected = decode(fields[1]);
        let got = to_comment(&input);
        if got != expected {
            failures.push(format!("{input:?}\n  expected {expected:?}\n  got {got:?}"));
        }
        checked += 1;
    }
    assert!(checked > 2_000, "only {checked} cases");
    assert!(
        failures.is_empty(),
        "{} of {checked} comment cases disagreed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn java_timestamp_utc_matches_the_reference_in_utc() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for fields in rows(include_str!("fixtures/timestamp.tsv")) {
        let seconds: i64 = fields[0].parse().unwrap();
        let expected = decode(fields[1]);
        let got = java_timestamp_utc(seconds);
        if got != expected {
            failures.push(format!("{seconds}\n  expected {expected:?}\n  got {got:?}"));
        }
        checked += 1;
    }
    assert!(checked > 5, "only {checked} cases");
    assert!(
        failures.is_empty(),
        "{} of {checked} timestamp cases disagreed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn store_with_matches_dumps() {
    let mut checked = 0;
    let mut failures = Vec::new();
    let sources = [
        include_str!("fixtures/dumps.tsv"),
        include_str!("fixtures/dumps-random.tsv"),
    ];
    for fields in sources.into_iter().flat_map(rows) {
        // The pairs are count-prefixed, key and value alternating, so an empty
        // list is representable and a key holding `=` or `|` cannot be confused
        // with the separator. The comment column has a presence flag in front of
        // it because `Some("")` writes a bare `#` line and `None` writes nothing,
        // and both would otherwise encode as an empty field.
        let count: usize = fields[0].parse().expect("count");
        let pairs: Vec<(String, String)> = (0..count)
            .map(|index| {
                (
                    decode(fields[7 + index * 2]),
                    decode(fields[7 + index * 2 + 1]),
                )
            })
            .collect();
        let separator = decode(fields[1]);
        let comment = if fields[2] == "1" {
            Some(decode(fields[3]))
        } else {
            None
        };
        let sort_keys = fields[4] == "1";
        let ensure_ascii = fields[5] == "1";
        let expected = decode(fields[6]);

        let properties = Properties::from_pairs(pairs);
        let options = WriteOptions {
            separator: &separator,
            comment: comment.as_deref(),
            timestamp: None,
            sort_keys,
            ensure_ascii,
        };
        let got = properties.store_with(&options);
        if got != expected {
            failures.push(format!(
                "sep={separator:?} comment={comment:?} sort={sort_keys} ascii={ensure_ascii}\n  \
                 expected {expected:?}\n  got      {got:?}"
            ));
        }
        checked += 1;
    }
    assert!(checked > 400, "only {checked} cases");
    assert!(
        failures.is_empty(),
        "{} of {checked} dumps cases disagreed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn a_timestamp_comment_is_written_before_the_entries() {
    // The order is comment, then timestamp, then entries, which is what the
    // reference's `dump` does. It is easy to get backwards and nothing else
    // checks it, because the dumps fixture writes with `timestamp=False`.
    let properties = Properties::from_pairs(vec![("a".to_string(), "1".to_string())]);
    let text = properties.store_with(&WriteOptions {
        comment: Some("note"),
        timestamp: Some(0),
        ..WriteOptions::default()
    });
    assert_eq!(text, "#note\n#Thu Jan 01 00:00:00 UTC 1970\na=1\n");
}

#[test]
fn unescape_matches_the_reference() {
    // `unescape` is the decode half of the reader, and the reference exposes it
    // as a function of its own. The fixture records its output, with a valid
    // surrogate pair decoded and a lone surrogate normalised to U+FFFD, which is
    // the one place the crate cannot match Python: Rust text cannot hold a lone
    // surrogate. An invalid `\u` is the reference's `InvalidUEscapeError`, marked
    // as `!error` in the fixture.
    let mut checked = 0;
    let mut failures = Vec::new();
    let sources = [
        include_str!("fixtures/unescape.tsv"),
        include_str!("fixtures/unescape-random.tsv"),
    ];
    for fields in sources.into_iter().flat_map(rows) {
        let input = decode(fields[0]);
        let got = unescape(&input);
        let matches = match (fields[1], &got) {
            ("!error", Err(_)) => true,
            ("!error", Ok(_)) => false,
            (_, Err(_)) => false,
            (expected, Ok(value)) => *value == decode(expected),
        };
        if !matches {
            failures.push(format!(
                "{input:?}\n  expected {:?}\n  got {got:?}",
                decode(fields[1])
            ));
        }
        checked += 1;
    }
    assert!(checked > 2_000, "only {checked} cases");
    assert!(
        failures.is_empty(),
        "{} of {checked} unescape cases disagreed:\n{}",
        failures.len(),
        failures.join("\n")
    );
}
