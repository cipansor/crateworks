//! Checks the crate against the official `factur-x` Python package.
//!
//! `tests/fixtures/oracle.tsv` is one line per case, tab separated:
//!
//! ```text
//! case   syntax   file
//! 0      cii      0000_cii.xml
//! 0      ubl      0000_ubl.xml
//! ```
//!
//! Every file is the reference's own output for a random invoice, built by
//! `scripts/fuzz/factur-x/gen.py`. For each case the test reads the document,
//! parses it into the crate's `Invoice`, generates it again, and checks that
//! the two carry the same element paths with the same text. The comparison is
//! on the set of paths rather than the bytes, because element order within a
//! parent and the set of namespace declarations are not part of the meaning;
//! what matters is that every field the reference writes is written here too,
//! and no field is invented.
//!
//! The generator, and why the reference's CII output is not checked against its
//! own schema, are described in `scripts/fuzz/README.md`.

use std::collections::BTreeSet;

use factur_x::{Syntax, detect_syntax, generate, parse};
use xmlkit::{Element, local_name};

/// One line of the fixture.
struct Case {
    name: String,
    syntax: Syntax,
    file: String,
}

fn fixture(name: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read {}: {error}", path.display()))
}

fn cases() -> Vec<Case> {
    include_str!("fixtures/oracle.tsv")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            let mut fields = line.split('\t');
            let _number = fields.next().expect("case number");
            let syntax = match fields.next().expect("syntax") {
                "cii" => Syntax::Cii,
                "ubl" => Syntax::Ubl,
                other => panic!("unknown syntax {other}"),
            };
            Case {
                name: line.to_string(),
                syntax,
                file: fields.next().expect("file").to_string(),
            }
        })
        .collect()
}

/// Every element path with non-blank text, as `parent/child=text`.
fn paths(element: &Element, prefix: &str, out: &mut BTreeSet<String>) {
    let here = format!("{prefix}/{}", local_name(&element.name));
    let text = element.text();
    if !text.trim().is_empty() {
        out.insert(format!("{here}={text}"));
    }
    for node in &element.children {
        if let xmlkit::Node::Element(child) = node {
            paths(child, &here, out);
        }
    }
}

fn differences(ours: &str, reference: &str) -> Vec<String> {
    let ours = xmlkit::parse(ours).expect("our document must be well formed");
    let reference = xmlkit::parse(reference).expect("the fixture must be well formed");
    let mut our_paths = BTreeSet::new();
    let mut reference_paths = BTreeSet::new();
    paths(&ours, "", &mut our_paths);
    paths(&reference, "", &mut reference_paths);

    let mut out = Vec::new();
    for path in our_paths.difference(&reference_paths) {
        out.push(format!("only in ours: {path}"));
    }
    for path in reference_paths.difference(&our_paths) {
        out.push(format!("only in the reference: {path}"));
    }
    out
}

#[test]
fn the_reference_documents_carry_the_same_fields_as_we_generate() {
    let mut checked = 0;
    for case in cases() {
        let xml = fixture(&format!("oracle/{}", case.file));
        assert_eq!(
            detect_syntax(&xml),
            Some(case.syntax),
            "{}: syntax not detected",
            case.name
        );
        let invoice =
            parse(&xml, case.syntax).unwrap_or_else(|error| panic!("{}: {error}", case.name));
        let ours = generate(&invoice, case.syntax);
        let diffs = differences(&ours, &xml);
        assert!(diffs.is_empty(), "{}:\n{}", case.name, diffs.join("\n"));
        checked += 1;
    }
    assert!(checked > 400, "only {checked} cases were read");
}
