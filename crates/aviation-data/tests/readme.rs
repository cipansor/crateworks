//! Runs the README example, so the documentation cannot drift from the API.

#[test]
fn readme_examples() {
    use aviation_data::{Kind, distance_km, find, near};

    let lhr = find("LHR").unwrap();
    assert_eq!(lhr.name(), "London Heathrow Airport");
    assert_eq!(lhr.icao(), Some("EGLL"));
    assert_eq!(lhr.country(), "GB");
    assert_eq!(lhr.municipality(), Some("London"));
    assert_eq!(lhr.kind(), Kind::LargeAirport);

    // ICAO, the internal identifier and the lowercase spelling all resolve to
    // the same record.
    assert_eq!(find("EGLL").unwrap().iata(), Some("LHR"));
    assert_eq!(find("egll").unwrap().ident(), "EGLL");

    let jfk = find("JFK").unwrap();
    assert!((distance_km(lhr, jfk) - 5540.0).abs() < 1.0);

    let close = near(51.47, -0.46, 100.0);
    assert!(close.iter().any(|(a, _)| a.ident() == "EGLL"));
}
