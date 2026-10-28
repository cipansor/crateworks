//! Comparison of the XML properties format against Python's `javaproperties`.
//!
//! `oracle.rs` and `writer_oracle.rs` cover the line-based format. This covers
//! the second one, `java.util.Properties.loadFromXML` and `storeToXML`, which
//! the reference exposes as `loads_xml` and `dumps_xml`.
//!
//! The fixtures are built by `scripts/fuzz/javaproperties/gen_xml.py`, which
//! records the reference's own output for the writer and the pairs it reads for
//! the reader. Every expectation is the reference's; none is hand-written.

use javaproperties::{Error, dumps_xml, loads_xml};

/// Decodes the escaping the generator writes, the same scheme as the other
/// fixtures in this crate.
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

fn rows(text: &'static str) -> impl Iterator<Item = Vec<&'static str>> {
    text.lines()
        .skip(1)
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\t').collect())
}

#[test]
fn dumps_xml_matches_the_reference() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for fields in rows(include_str!("fixtures/xml_dumps.tsv")) {
        let count: usize = fields[0].parse().unwrap();
        let comment = (fields[1] == "1").then(|| decode(fields[2]));
        let sort_keys = fields[3] == "1";
        let expected = decode(fields[4]);

        let mut pairs = Vec::with_capacity(count);
        for i in 0..count {
            pairs.push((decode(fields[5 + 2 * i]), decode(fields[6 + 2 * i])));
        }

        let got = dumps_xml(&pairs, comment.as_deref(), sort_keys);
        if got != expected {
            failures.push(format!(
                "pairs {pairs:?} comment {comment:?} sort {sort_keys}\n  expected {expected:?}\n  got {got:?}"
            ));
        }
        checked += 1;
    }
    assert_eq!(
        failures.len(),
        0,
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(checked > 200, "only {checked} cases checked");
}

#[test]
fn loads_xml_matches_the_reference() {
    let mut checked = 0;
    let mut failures = Vec::new();
    for fields in rows(include_str!("fixtures/xml_loads.tsv")) {
        let document = decode(fields[0]);
        let count: usize = fields[1].parse().unwrap();
        let mut expected = Vec::with_capacity(count);
        for i in 0..count {
            expected.push((decode(fields[2 + 2 * i]), decode(fields[3 + 2 * i])));
        }

        match loads_xml(&document) {
            Ok(got) if got == expected => {}
            Ok(got) => failures.push(format!(
                "{document:?}\n  expected {expected:?}\n  got {got:?}"
            )),
            Err(error) => failures.push(format!(
                "{document:?}\n  expected {expected:?}\n  error {error}"
            )),
        }
        checked += 1;
    }
    assert_eq!(
        failures.len(),
        0,
        "{} mismatches:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert!(checked > 500, "only {checked} cases checked");
}

#[test]
fn the_rejected_documents_name_their_fault() {
    assert_eq!(
        loads_xml("<config><entry key=\"a\">1</entry></config>").unwrap_err(),
        Error::NotPropertiesRoot
    );
    assert_eq!(
        loads_xml("<properties><entry>1</entry></properties>").unwrap_err(),
        Error::MissingEntryKey
    );
    assert_eq!(
        loads_xml("<properties><entry key=\"a\">1</properties>").unwrap_err(),
        Error::MalformedXml
    );
}
