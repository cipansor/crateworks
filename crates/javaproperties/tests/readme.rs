//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use javaproperties::{Properties, WriteOptions, join_key_value, parse, to_comment};

    let pairs = parse(b"name = cipansor\ncity\\:bandung = west java\n").unwrap();
    assert_eq!(pairs[0], ("name".to_string(), "cipansor".to_string()));
    // The escaped colon is part of the key, not a separator.
    assert_eq!(
        pairs[1],
        ("city:bandung".to_string(), "west java".to_string())
    );

    let properties = Properties::from_pairs(vec![
        ("a b".to_string(), "one".to_string()),
        ("café".to_string(), "two".to_string()),
    ]);
    assert_eq!(properties.store(None), "a\\ b=one\ncaf\\u00E9=two\n");

    assert_eq!(join_key_value("a b", "c d", "="), "a\\ b=c d");
    assert_eq!(to_comment("one\ntwo"), "#one\n#two");

    let properties = Properties::from_pairs(vec![
        ("b".to_string(), "2".to_string()),
        ("a".to_string(), "1".to_string()),
    ]);
    let text = properties.store_with(&WriteOptions {
        separator: ": ",
        sort_keys: true,
        timestamp: Some(0),
        ..WriteOptions::default()
    });
    assert_eq!(text, "#Thu Jan 01 00:00:00 UTC 1970\na: 1\nb: 2\n");
}
