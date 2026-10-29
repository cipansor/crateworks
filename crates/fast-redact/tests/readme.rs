//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use fast_redact::{Redactor, Value};

    let document =
        Value::parse(r#"{"headers":{"host":"example.com","cookie":"session=abc"}}"#).unwrap();

    let redactor = Redactor::new(["headers.cookie"]).unwrap();

    assert_eq!(
        redactor.redact(&document),
        r#"{"headers":{"host":"example.com","cookie":"[REDACTED]"}}"#,
    );
    assert_eq!(
        document.to_json(),
        r#"{"headers":{"host":"example.com","cookie":"session=abc"}}"#
    );
}
