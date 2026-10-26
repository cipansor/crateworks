//! Runs the README examples, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use address_parser::{Component, parse_location};

    let address = parse_location("1005 N Gravenstein Hwy Suite 500 Sebastopol, CA").unwrap();
    assert_eq!(address.number(), Some("1005"));
    assert_eq!(address.get(Component::Prefix), Some("N"));
    assert_eq!(address.street(), Some("Gravenstein"));
    assert_eq!(address.get(Component::Type), Some("Hwy"));
    assert_eq!(address.get(Component::SecUnitType), Some("Suite"));
    assert_eq!(address.get(Component::SecUnitNum), Some("500"));
    assert_eq!(address.city(), Some("Sebastopol"));
    assert_eq!(address.state(), Some("CA"));

    for input in [
        "1005 Gravenstein Hwy 95472",
        "1005 Gravenstein Hwy, 95472",
        "1005 Gravenstein Highway North, 95472",
        "1005 N Gravenstein Highway, Sebastopol, CA",
        "1005 N Gravenstein Hwy Suite 500 Sebastopol, CA",
    ] {
        assert_eq!(
            parse_location(input).unwrap().street(),
            Some("Gravenstein"),
            "{input}"
        );
    }
}
