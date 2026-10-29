//! Replays the reference implementation's own test data.
//!
//! The four CSV files under `tests/fixtures/` are copied unchanged from
//! `google/open-location-code` (Apache-2.0), the reference implementation. Each
//! is the case set its authors published, so replaying it is what turns "the
//! cases we thought of are right" into "the cases the reference checks agree".
//! `research/tools/verify_openlocationcode.py` regenerates them and reruns the
//! same comparisons against the reference itself.

use openlocationcode::{
    MAX_CODE_LENGTH, decode, encode, encode_integers, is_full, is_short, is_valid,
    location_to_integers, recover_nearest, shorten,
};

/// The data rows of a fixture: comments (which start with `#`) and blank lines
/// dropped.
fn rows(csv: &str) -> Vec<Vec<String>> {
    csv.lines()
        .filter(|line| !line.starts_with('#') && !line.trim().is_empty())
        .map(|line| line.split(',').map(str::to_string).collect())
        .collect()
}

#[test]
fn validity_agrees_with_the_reference() {
    let data = rows(include_str!("fixtures/validityTests.csv"));
    assert!(data.len() > 20, "the fixture lost its cases");
    for row in &data {
        let code = &row[0];
        assert_eq!(is_valid(code), row[1] == "true", "is_valid {code:?}");
        assert_eq!(is_short(code), row[2] == "true", "is_short {code:?}");
        assert_eq!(is_full(code), row[3] == "true", "is_full {code:?}");
    }
}

#[test]
fn decoding_agrees_with_the_reference() {
    let data = rows(include_str!("fixtures/decoding.csv"));
    assert!(data.len() > 400, "the fixture lost its cases");
    for row in &data {
        let code = &row[0];
        let expected_length: usize = row[1].parse().unwrap();
        let lat_low: f64 = row[2].parse().unwrap();
        let lng_low: f64 = row[3].parse().unwrap();
        let lat_high: f64 = row[4].parse().unwrap();
        let lng_high: f64 = row[5].parse().unwrap();

        let area = decode(code).unwrap_or_else(|e| panic!("decode {code:?}: {e}"));
        assert_eq!(area.code_length, expected_length, "length of {code:?}");
        // The reference rounds its expected values to ten decimals, so the
        // comparison is to 1e-10 as its own test is.
        assert!(
            (area.latitude_low - lat_low).abs() < 1e-10,
            "south of {code:?}: {} vs {lat_low}",
            area.latitude_low
        );
        assert!(
            (area.latitude_high - lat_high).abs() < 1e-10,
            "north of {code:?}: {} vs {lat_high}",
            area.latitude_high
        );
        assert!(
            (area.longitude_low - lng_low).abs() < 1e-10,
            "west of {code:?}: {} vs {lng_low}",
            area.longitude_low
        );
        assert!(
            (area.longitude_high - lng_high).abs() < 1e-10,
            "east of {code:?}: {} vs {lng_high}",
            area.longitude_high
        );
    }
}

#[test]
fn integer_encoding_agrees_with_the_reference() {
    let data = rows(include_str!("fixtures/encoding.csv"));
    assert!(data.len() > 300, "the fixture lost its cases");
    for row in &data {
        let lat: i64 = row[2].parse().unwrap();
        let lng: i64 = row[3].parse().unwrap();
        let length: usize = row[4].parse().unwrap();
        let expected = &row[5];
        // The integer path is exact, so it is compared exactly. A length past
        // the maximum is truncated to fifteen, which the fixture exercises.
        let got = encode_integers(lat, lng, length).unwrap();
        assert_eq!(&got, expected, "encode_integers({lat}, {lng}, {length})");
        assert!(got.len() <= MAX_CODE_LENGTH + 1, "length cap on {got:?}");
    }
}

#[test]
fn degree_encoding_agrees_with_the_reference_within_tolerance() {
    let data = rows(include_str!("fixtures/encoding.csv"));
    let mut mismatches = 0;
    for row in &data {
        let latitude: f64 = row[0].parse().unwrap();
        let longitude: f64 = row[1].parse().unwrap();
        let length: usize = row[4].parse().unwrap();
        let expected = &row[5];
        let got = encode(latitude, longitude, length).unwrap();
        // Converting degrees to integers loses precision, and the reference
        // accepts a small error rate for exactly this reason (its own test
        // allows 5%). The integer column above is the exact oracle.
        if &got != expected {
            mismatches += 1;
        }
    }
    let rate = mismatches as f64 / data.len() as f64;
    assert!(
        rate <= 0.05,
        "degree encoding mismatch rate {rate} is too high"
    );
}

#[test]
fn location_to_integers_agrees_with_the_reference() {
    let data = rows(include_str!("fixtures/encoding.csv"));
    for row in &data {
        let latitude: f64 = row[0].parse().unwrap();
        let longitude: f64 = row[1].parse().unwrap();
        let lat_int: i64 = row[2].parse().unwrap();
        let lng_int: i64 = row[3].parse().unwrap();
        let (got_lat, got_lng) = location_to_integers(latitude, longitude);
        // The reference allows its integer to be one below the expected value,
        // a floating-point artefact it documents.
        assert!(
            lat_int - 1 <= got_lat && got_lat <= lat_int,
            "latitude {latitude}: want {lat_int}, got {got_lat}"
        );
        assert!(
            lng_int - 1 <= got_lng && got_lng <= lng_int,
            "longitude {longitude}: want {lng_int}, got {got_lng}"
        );
    }
}

#[test]
fn shortening_and_recovery_agree_with_the_reference() {
    let data = rows(include_str!("fixtures/shortCodeTests.csv"));
    assert!(data.len() > 15, "the fixture lost its cases");
    for row in &data {
        let full = &row[0];
        let latitude: f64 = row[1].parse().unwrap();
        let longitude: f64 = row[2].parse().unwrap();
        let short = &row[3];
        let kind = &row[4];
        if kind == "B" || kind == "S" {
            assert_eq!(
                &shorten(full, latitude, longitude).unwrap(),
                short,
                "shorten {full:?}"
            );
        }
        if kind == "B" || kind == "R" {
            assert_eq!(
                &recover_nearest(short, latitude, longitude).unwrap(),
                full,
                "recover {short:?}"
            );
        }
    }
}
