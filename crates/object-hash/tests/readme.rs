//! Runs the README examples, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use object_hash::{Options, Value, canonical_with, hash};

    let one = Value::Map(vec![
        ("name".into(), Value::String("cipansor".into())),
        ("year".into(), Value::Number(2026.0)),
    ]);
    // The same entries, inserted the other way round.
    let two = Value::Map(vec![
        ("year".into(), Value::Number(2026.0)),
        ("name".into(), Value::String("cipansor".into())),
    ]);

    assert_eq!(hash(&one), hash(&two));

    // A plain object and a map with the same entry are not interchangeable.
    let object = Value::Object(vec![("a".into(), Value::number(1))]);
    let map = Value::Map(vec![("a".into(), Value::number(1))]);
    assert_ne!(hash(&object), hash(&map));

    // `Options::keys` writes only an object's keys.
    let keys = Value::Object(vec![
        ("a".into(), Value::number(1)),
        ("b".into(), Value::number(2)),
    ]);
    assert_eq!(
        canonical_with(&keys, &Options::keys()),
        "object:2:string:1:a:,string:1:b:,"
    );
}
