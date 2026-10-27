//! Lookup, filter, search and geometry, checked against the fixtures the
//! generator wrote from the same upstream rows. The fixtures are a slice of the
//! real table, so this test fails if a field is read from the wrong column or a
//! lookup resolves to the wrong airport, without needing the network.

use std::collections::HashMap;

use aviation_data::{Kind, all, count, distance_km, find, in_continent, in_country, near, of_kind};

/// The fixture rows, keyed by `ident`, as `(kind, name, iata, icao, lat, lon)`.
fn fixtures() -> HashMap<String, Vec<String>> {
    let raw = include_str!("fixtures/lookups.tsv");
    let mut out = HashMap::new();
    for line in raw.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let fields: Vec<String> = line.split('\t').map(str::to_owned).collect();
        out.insert(fields[0].clone(), fields);
    }
    out
}

#[test]
fn every_fixture_row_matches_the_crate() {
    let fixtures = fixtures();
    assert!(
        fixtures.len() >= 10,
        "the fixture file is too small to be useful"
    );
    for (ident, fields) in &fixtures {
        let airport = find(ident).unwrap_or_else(|| panic!("{ident} is not in the table"));
        assert_eq!(airport.ident(), ident);
        assert_eq!(airport.kind().as_str(), fields[1], "{ident} kind");
        assert_eq!(airport.name(), fields[2], "{ident} name");
        assert_eq!(airport.iata().unwrap_or(""), fields[3], "{ident} iata");
        assert_eq!(airport.icao().unwrap_or(""), fields[4], "{ident} icao");
        assert_eq!(
            airport.latitude().to_string(),
            fields[5],
            "{ident} latitude"
        );
        assert_eq!(
            airport.longitude().to_string(),
            fields[6],
            "{ident} longitude"
        );
    }
}

#[test]
fn a_code_resolves_to_its_own_airport() {
    // Every code on every row must resolve, and to the row that owns it. Only
    // `local` is not unique, so the check runs over the whole table.
    for airport in all() {
        for code in [
            Some(airport.ident()),
            airport.iata(),
            airport.icao(),
            airport.gps(),
            airport.local(),
        ]
        .into_iter()
        .flatten()
        {
            let hit = find(code).unwrap_or_else(|| panic!("{code} did not resolve"));
            assert!(
                hit.ident() == airport.ident()
                    || hit.iata() == Some(code)
                    || hit.icao() == Some(code)
                    || hit.gps() == Some(code)
                    || hit.local() == Some(code),
                "{code} resolved to {} which does not carry it",
                hit.ident()
            );
        }
    }
}

#[test]
fn lookup_is_case_insensitive() {
    assert_eq!(find("lhr").unwrap().ident(), "EGLL");
    assert_eq!(find("Lhr").unwrap().ident(), "EGLL");
    assert_eq!(find("egll").unwrap().iata(), Some("LHR"));
    assert_eq!(find("nope-not-an-airport"), None);
}

#[test]
fn the_code_spaces_outrank_the_ident_space() {
    // `IMO` is the IATA code of FEFZ and the local code of AYIO. IATA is the
    // more authoritative space, so it must win regardless of table order.
    let hit = find("IMO").expect("IMO must resolve");
    assert_eq!(hit.iata(), Some("IMO"));
    assert_ne!(hit.local(), Some("IMO"));

    // `WABK` is the ident of Karubaga Airport and the ICAO code of Biri
    // Airstrip. The code column is what a caller means, so ICAO must win.
    let hit = find("WABK").expect("WABK must resolve");
    assert_eq!(hit.icao(), Some("WABK"));
    assert_ne!(hit.ident(), "WABK");
}

#[test]
fn table_shape_is_stable() {
    assert_eq!(count(), all().len());
    assert_eq!(count(), 11_848);
    // Sorted by ident, and idents are unique, so a binary search would work.
    let mut previous = "";
    for airport in all() {
        assert!(airport.ident() > previous, "table is not sorted by ident");
        previous = airport.ident();
        assert!(!airport.name().is_empty());
        assert!(!airport.country().is_empty());
        assert!((-90.0..=90.0).contains(&airport.latitude()));
        assert!((-180.0..=180.0).contains(&airport.longitude()));
    }
}

#[test]
fn filters_return_only_matching_airports() {
    let indonesia = in_country("ID");
    assert!(indonesia.len() > 100);
    assert!(indonesia.iter().all(|a| a.country() == "ID"));
    assert!(
        in_country("id").len() == indonesia.len(),
        "case-insensitive"
    );

    let asia = in_continent("AS");
    assert!(asia.iter().all(|a| a.continent() == "AS"));

    let large = of_kind(Kind::LargeAirport);
    assert!(large.iter().all(|a| a.kind() == Kind::LargeAirport));
    // Every large airport has a code, so the set is non-trivial.
    assert!(large.len() > 500);
}

#[test]
fn search_matches_name_and_municipality() {
    let heathrow = aviation_data::search("heathrow");
    assert!(heathrow.iter().any(|a| a.ident() == "EGLL"));

    // "London" is a municipality for several airports, and a name fragment for
    // others; all matches must mention it in one of the two fields.
    let london = aviation_data::search("LONDON");
    assert!(london.len() > 1);
    for airport in &london {
        let hit = airport.name().to_lowercase().contains("london")
            || airport
                .municipality()
                .is_some_and(|m| m.to_lowercase().contains("london"));
        assert!(hit, "{} matched without naming London", airport.ident());
    }
}

#[test]
fn distance_and_radius_agree() {
    let lhr = find("LHR").unwrap();
    let jfk = find("JFK").unwrap();
    // The haversine on a mean-Earth sphere, which is a few kilometres short of
    // the WGS84 geodesic (5,555 km) because the sphere is not the ellipsoid.
    let d = distance_km(lhr, jfk);
    assert!((d - 5539.8).abs() < 1.0, "LHR-JFK was {d}");
    assert_eq!(distance_km(lhr, lhr), 0.0);

    // Symmetry, and the triangle inequality on a sphere.
    assert!((distance_km(jfk, lhr) - d).abs() < 1e-9);

    let close = near(51.470748, -0.459909, 50.0);
    assert!(!close.is_empty());
    assert_eq!(close[0].0.ident(), "EGLL", "the query point is Heathrow");
    assert!(
        close.windows(2).all(|w| w[0].1 <= w[1].1),
        "not nearest-first"
    );
    for (airport, km) in &close {
        assert!(*km <= 50.0);
        assert_eq!(*km, distance_km(lhr, airport));
    }
}
