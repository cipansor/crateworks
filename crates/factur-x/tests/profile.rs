//! Replays the profile fixture built from the reference `factur-x` package.
//!
//! `tests/fixtures/profile.tsv` is one line per case:
//!
//! ```text
//! name                 syntax  file             level
//! urn00_cii            cii     urn00_cii.xml    minimum
//! urn13_ubl            ubl     urn13_ubl.xml
//! ```
//!
//! The last column is the reference's answer, or empty when the reference
//! raises - the error path. The generator is
//! `research/tools/verify_facturx_level.py`, which drives the reference over
//! the corpus; the fixture is committed so this test needs no reference
//! installed.

use factur_x::{Profile, profile_of};

fn fixtures() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
}

struct Case {
    name: String,
    file: String,
    level: String,
}

fn cases() -> Vec<Case> {
    include_str!("fixtures/profile.tsv")
        .lines()
        .filter(|line| !line.trim().is_empty() && !line.starts_with('#'))
        .map(|line| {
            let mut fields = line.split('\t');
            let name = fields.next().expect("name").to_string();
            let _syntax = fields.next().expect("syntax");
            let file = fields.next().expect("file").to_string();
            let level = fields.next().unwrap_or("").to_string();
            Case { name, file, level }
        })
        .collect()
}

#[test]
fn the_level_matches_the_reference_for_every_document() {
    let mut checked = 0;
    for case in cases() {
        let path = fixtures().join("profile").join(&case.file);
        let xml = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()));
        let ours = profile_of(&xml);
        if case.level.is_empty() {
            assert!(
                ours.is_err(),
                "{}: the reference raises but the crate returned {ours:?}",
                case.name
            );
        } else {
            let profile = ours.unwrap_or_else(|error| panic!("{}: {error}", case.name));
            assert_eq!(
                profile.as_str(),
                case.level,
                "{}: crate and reference disagree",
                case.name
            );
            assert!(
                Profile::ALL.iter().any(|p| p.as_str() == case.level),
                "{}: {} is not a listed profile",
                case.name,
                case.level
            );
        }
        checked += 1;
    }
    assert!(checked >= 30, "only {checked} cases were read");
}
