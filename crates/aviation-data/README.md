# aviation-data

[![crates.io](https://img.shields.io/crates/v/aviation-data.svg)](https://crates.io/crates/aviation-data)
[![docs.rs](https://docs.rs/aviation-data/badge.svg)](https://docs.rs/aviation-data)

Airport reference data: turn an IATA or ICAO code into a record, and search,
filter or measure the airports around a point.

```rust
use aviation_data::{Airport, Kind, find};

let lhr = find("LHR").unwrap();
assert_eq!(lhr.name(), "London Heathrow Airport");
assert_eq!(lhr.icao(), Some("EGLL"));
assert_eq!(lhr.country(), "GB");
assert_eq!(lhr.municipality(), Some("London"));
assert_eq!(lhr.kind(), Kind::LargeAirport);

// ICAO, the internal identifier and the lowercase spelling all resolve to the
// same record.
assert_eq!(find("EGLL").unwrap().iata(), Some("LHR"));
assert_eq!(find("egll").unwrap().ident(), "EGLL");
```

The crate ships the [OurAirports](https://ourairports.com/data/) table compiled
in: **11,888 airports**, every one that is not closed and carries an IATA or an
ICAO code. There is no network call, no file to load and no database to build -
`find` is a lookup in a table that is already in the binary.

```rust
use aviation_data::{distance_km, find, near};

let lhr = find("LHR").unwrap();
let jfk = find("JFK").unwrap();
assert!((distance_km(lhr, jfk) - 5540.0).abs() < 1.0);

// The airports within 100 km of a coordinate, nearest first.
let close = near(51.47, -0.46, 100.0);
assert!(close.iter().any(|(a, _)| a.ident() == "EGLL"));
```

## What it covers

* **Lookup** by IATA, ICAO, the GPS code, the local code and the OurAirports
  `ident`, case-insensitively. When a code appears in more than one space the
  code columns win over `ident`, in the order IATA, ICAO, GPS, local, `ident`:
  `WABK` is the `ident` of Karubaga Airport but the ICAO code of Biri Airstrip,
  and a caller passing `WABK` means the ICAO code. `find` returns one airport;
  `all` returns the whole table.
* **Filtering** by country (`in_country`, using the ISO 3166-1 alpha-2 code),
  by continent (`in_continent`) and by kind (`of_kind`).
* **Search** by name or municipality (`search`), case-insensitively.
* **Geometry**: `distance_km` between two airports and `near` for the airports
  within a radius, nearest first, by the haversine formula on a mean-Earth
  sphere.
* **Fields** per airport: name, kind, latitude, longitude, elevation, continent,
  ISO country and region, municipality, whether it has scheduled service, and
  the IATA, ICAO, GPS and local codes.

## Scope

The data is a **snapshot**, not a live feed. It is regenerated from the upstream
CSV by `research/tools/gen_aviation.py` and pinned at 11,888 rows, so a silent
upstream change fails the generator rather than shrinking the crate quietly. A
new airport is a new release.

`find` matches a code, not a name, so it is exact. `search` is a plain
case-insensitive substring match with no ranking or fuzzy matching; it is meant
for a short list, not for autocomplete over the whole table. Airports that are
closed, and those with no IATA or ICAO code (about 60,000 small private fields,
heliports and closed strips), are not in the table.

## Data provenance

The table is generated from [OurAirports](https://ourairports.com/data/), a
public-domain dataset published by David Megginson. OurAirports places its data
in the public domain, so it can be redistributed here under Apache-2.0; the
provenance is recorded in `NOTICE` and in the header of the generated file.

## Licence

Apache-2.0. See `LICENSE` and `NOTICE`.
