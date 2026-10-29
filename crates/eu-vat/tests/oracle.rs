//! Checks the crate against `python-stdnum` over a generated corpus.
//!
//! The fixture is one line per case, tab separated:
//!
//! ```text
//! input                  accepted   canonical
//! ATU13585627            1          ATU13585627
//! ATU14585627            0
//! ```
//!
//! `accepted` is the reference's verdict and `canonical` its normalised form,
//! empty when it rejects the input. A number the reference accepts must parse
//! here to exactly the recorded canonical; one it rejects must not parse. The
//! corpus and its generator are described in `scripts/fuzz/README.md`.

use eu_vat::Vat;

/// One line of the fixture.
struct Case {
    input: String,
    accepted: bool,
    canonical: String,
}

fn cases() -> Vec<Case> {
    include_str!("fixtures/oracle.tsv")
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let mut fields = line.split('\t');
            let input = fields.next().expect("input").to_string();
            let accepted = fields.next().expect("accepted") == "1";
            let canonical = fields.next().unwrap_or("").to_string();
            Case {
                input,
                accepted,
                canonical,
            }
        })
        .collect()
}

#[test]
fn the_crate_agrees_with_the_oracle_on_every_case() {
    let mut checked = 0;
    for case in cases() {
        checked += 1;
        match (Vat::parse(&case.input), case.accepted) {
            (Ok(vat), true) => assert_eq!(
                vat.to_string(),
                case.canonical,
                "{:?} parsed to {:?} but the oracle says {:?}",
                case.input,
                vat.to_string(),
                case.canonical
            ),
            (Ok(vat), false) => panic!(
                "{:?} was accepted as {:?} but the oracle rejects it",
                case.input,
                vat.to_string()
            ),
            (Err(_), false) => {}
            (Err(e), true) => panic!(
                "{:?} was rejected ({e}) but the oracle accepts it as {:?}",
                case.input, case.canonical
            ),
        }
    }
    assert!(checked > 400, "the corpus shrank to {checked} cases");
}
