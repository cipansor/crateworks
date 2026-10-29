//! Replays the random corpus built by `scripts/fuzz/openlocationcode/gen.py`.
//!
//! The four CSV files replayed in `tests/reference.rs` are the cases the
//! reference's authors wrote down; this corpus is the cases nobody wrote down,
//! each labelled by the reference. The two together are what turn "the cases we
//! thought of are right" into "the cases we checked agree".
//!
//! The generator records the reference's own answer, so a disagreement is a bug
//! in this crate, not a wrong expectation - except on the `E` rows, where the
//! reference's degree-to-integer conversion is allowed to land one step out and
//! it accepts a small mismatch rate for exactly that reason. The exact integer
//! path is covered by `encoding.csv` instead.

use openlocationcode::{decode, encode, is_full, is_short, is_valid, recover_nearest, shorten};

fn lines() -> Vec<Vec<String>> {
    include_str!("fixtures/fuzz.tsv")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| line.split('\t').map(str::to_string).collect())
        .collect()
}

#[test]
fn validity_corpus_agrees_with_the_reference() {
    let mut checked = 0;
    for row in lines() {
        if row[0] != "V" {
            continue;
        }
        let code = &row[1];
        assert_eq!(is_valid(code), row[2] == "true", "is_valid {code:?}");
        assert_eq!(is_short(code), row[3] == "true", "is_short {code:?}");
        assert_eq!(is_full(code), row[4] == "true", "is_full {code:?}");
        checked += 1;
    }
    assert!(checked > 1000, "the corpus lost its validity rows");
}

#[test]
fn decode_corpus_agrees_with_the_reference() {
    let mut checked = 0;
    for row in lines() {
        if row[0] != "D" {
            continue;
        }
        let code = &row[1];
        let area = decode(code).unwrap_or_else(|e| panic!("decode {code:?}: {e}"));
        let expected = [
            row[2].parse::<f64>().unwrap(),
            row[3].parse::<f64>().unwrap(),
            row[4].parse::<f64>().unwrap(),
            row[5].parse::<f64>().unwrap(),
        ];
        let got = [
            area.latitude_low,
            area.longitude_low,
            area.latitude_high,
            area.longitude_high,
        ];
        for (name, (g, w)) in ["south", "west", "north", "east"]
            .iter()
            .zip(got.iter().zip(&expected))
        {
            assert!((g - w).abs() < 1e-9, "decode {code:?} {name}: {g} vs {w}");
        }
        assert_eq!(
            area.code_length,
            row[6].parse::<usize>().unwrap(),
            "length of {code:?}"
        );
        checked += 1;
    }
    assert!(checked > 500, "the corpus lost its decode rows");
}

#[test]
fn shorten_and_recover_corpus_agree_with_the_reference() {
    let mut shortened = 0;
    let mut recovered = 0;
    for row in lines() {
        match row[0].as_str() {
            "S" => {
                let code = &row[1];
                let latitude: f64 = row[2].parse().unwrap();
                let longitude: f64 = row[3].parse().unwrap();
                assert_eq!(
                    shorten(code, latitude, longitude).unwrap(),
                    row[4],
                    "shorten {code:?}"
                );
                shortened += 1;
            }
            "R" => {
                let short = &row[1];
                let latitude: f64 = row[2].parse().unwrap();
                let longitude: f64 = row[3].parse().unwrap();
                assert_eq!(
                    recover_nearest(short, latitude, longitude).unwrap(),
                    row[4],
                    "recover {short:?}"
                );
                recovered += 1;
            }
            _ => {}
        }
    }
    assert!(
        shortened > 200 && recovered > 200,
        "the corpus lost its shorten rows"
    );
}

#[test]
fn encode_corpus_agrees_with_the_reference_within_tolerance() {
    let mut total = 0;
    let mut mismatches = 0;
    for row in lines() {
        if row[0] != "E" {
            continue;
        }
        let latitude: f64 = row[1].parse().unwrap();
        let longitude: f64 = row[2].parse().unwrap();
        let length: usize = row[3].parse().unwrap();
        let got = encode(latitude, longitude, length).unwrap();
        total += 1;
        if got != row[4] {
            mismatches += 1;
        }
    }
    let rate = mismatches as f64 / total as f64;
    assert!(total > 500, "the corpus lost its encode rows");
    assert!(
        rate <= 0.05,
        "degree encoding mismatch rate {rate} is too high"
    );
}
