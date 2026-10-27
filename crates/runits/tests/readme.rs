//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use runits::{Quantity, convert};

    // A value and a unit written as text.
    let mass = Quantity::parse("3.5 kg").unwrap();
    assert!((mass.to("lb").unwrap().value() - 7.716).abs() < 0.001);

    // The affine temperature scales carry their offset.
    let boiling = Quantity::parse("100 degC").unwrap();
    assert!((boiling.to("degF").unwrap().value() - 212.0).abs() < 1e-9);

    // Compound units are parsed, not just named.
    let speed = Quantity::parse("60 km/h").unwrap();
    assert!((speed.to("m/s").unwrap().value() - 16.666).abs() < 0.001);

    // A one-line conversion between two named units.
    assert_eq!(convert(1.0, "m", "cm").unwrap(), 100.0);
}
