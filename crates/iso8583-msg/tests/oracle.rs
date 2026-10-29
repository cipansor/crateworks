//! Checks the crate against `pyiso8583` over a generated corpus.
//!
//! The fixture is one line per case, tab separated:
//!
//! ```text
//! mti    field=value;field=value    hex
//! 0200   2=4111111111111111;3=000000    30323030...
//! ```
//!
//! A row means: a message carrying these fields, encoded with `default_ascii`,
//! is exactly these bytes. Each row is both encoded and decoded, so a
//! divergence in the bitmap chain, a length prefix or a field's encoding shows
//! up as a named case. The corpus and its generator are described in
//! `scripts/fuzz/README.md`.

use iso8583_msg::{Message, default_ascii};

/// One line of the fixture.
struct Case {
    mti: String,
    fields: Vec<(u8, String)>,
    bytes: Vec<u8>,
}

fn unhex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks(2)
        .map(|pair| {
            let hi = (pair[0] as char).to_digit(16).expect("hex");
            let lo = (pair[1] as char).to_digit(16).expect("hex");
            ((hi << 4) | lo) as u8
        })
        .collect()
}

fn cases() -> Vec<Case> {
    include_str!("fixtures/oracle.tsv")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut parts = line.split('\t');
            let mti = parts.next().expect("mti").to_string();
            let fields = parts
                .next()
                .expect("fields")
                .split(';')
                .map(|pair| {
                    let (field, value) = pair.split_once('=').expect("field=value");
                    (field.parse().expect("field number"), value.to_string())
                })
                .collect();
            let bytes = unhex(parts.next().expect("hex"));
            Case { mti, fields, bytes }
        })
        .collect()
}

fn build(case: &Case) -> Message {
    let mut msg = Message::new(&case.mti).unwrap_or_else(|e| panic!("{}: {e}", case.mti));
    for (field, value) in &case.fields {
        msg.set(*field, value)
            .unwrap_or_else(|e| panic!("{} field {field}: {e}", case.mti));
    }
    msg
}

#[test]
fn encoding_matches_the_reference_bytes() {
    let spec = default_ascii();
    for case in cases() {
        let encoded = build(&case)
            .encode(&spec)
            .unwrap_or_else(|e| panic!("{} {:?}: {e}", case.mti, case.fields));
        assert_eq!(
            encoded,
            case.bytes,
            "{} {:?} encoded to {}",
            case.mti,
            case.fields,
            encoded
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect::<String>()
        );
    }
}

#[test]
fn decoding_the_reference_bytes_recovers_the_fields() {
    let spec = default_ascii();
    for case in cases() {
        let decoded = Message::decode(&case.bytes, &spec)
            .unwrap_or_else(|e| panic!("{} {:?}: {e}", case.mti, case.fields));
        assert_eq!(decoded.mti(), case.mti);
        for (field, value) in &case.fields {
            assert_eq!(
                decoded.get(*field),
                Some(value.as_str()),
                "{} field {field}",
                case.mti
            );
        }
        assert_eq!(
            decoded.len(),
            case.fields.len(),
            "{} {:?}",
            case.mti,
            case.fields
        );
    }
}
