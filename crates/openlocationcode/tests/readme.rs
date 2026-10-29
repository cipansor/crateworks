//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use openlocationcode::{decode, encode, recover_nearest, shorten};

    let code = encode(47.365590, 8.524997, 10).unwrap();
    assert_eq!(code, "8FVC9G8F+6X");

    let area = decode("8FVC9G8F+6X").unwrap();
    assert!((area.latitude_center - 47.3655625).abs() < 1e-6);
    assert!(area.latitude_high > area.latitude_low);

    let short = shorten("8FVC9G8F+6X", 47.365590, 8.524997).unwrap();
    assert_eq!(short, "+6X");
    assert_eq!(
        recover_nearest(&short, 47.365590, 8.524997).unwrap(),
        "8FVC9G8F+6X"
    );
}
