#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod table;

use std::fmt;

/// The kind of facility an airport record describes, from the OurAirports
/// `type` column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Kind {
    /// A large airport, normally with scheduled international service.
    LargeAirport,
    /// A medium airport.
    MediumAirport,
    /// A small airport.
    SmallAirport,
    /// A heliport.
    Heliport,
    /// A seaplane base.
    SeaplaneBase,
    /// A balloonport.
    Balloonport,
}

impl Kind {
    /// The OurAirports spelling of this kind, such as `"large_airport"`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::LargeAirport => "large_airport",
            Self::MediumAirport => "medium_airport",
            Self::SmallAirport => "small_airport",
            Self::Heliport => "heliport",
            Self::SeaplaneBase => "seaplane_base",
            Self::Balloonport => "balloonport",
        }
    }

    /// Parse the OurAirports `type` column. The generator rejects anything
    /// outside this set, so an unknown value means the data file was edited by
    /// hand.
    pub(crate) fn from_str(text: &str) -> Self {
        match text {
            "large_airport" => Self::LargeAirport,
            "medium_airport" => Self::MediumAirport,
            "small_airport" => Self::SmallAirport,
            "heliport" => Self::Heliport,
            "seaplane_base" => Self::SeaplaneBase,
            "balloonport" => Self::Balloonport,
            other => panic!("unknown airport kind {other:?} in the generated table"),
        }
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The codes an airport may be addressed by. Each is empty in the upstream data
/// for some rows, so each is optional.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(crate) struct Codes {
    pub iata: Option<&'static str>,
    pub icao: Option<&'static str>,
    pub gps: Option<&'static str>,
    pub local: Option<&'static str>,
}

/// Column indices in the generated TSV, matching `FIELDS` in
/// `research/tools/gen_aviation.py`. Shared by the parser and the table tests.
pub(crate) mod col {
    pub const IDENT: usize = 0;
    pub const KIND: usize = 1;
    pub const NAME: usize = 2;
    pub const LAT: usize = 3;
    pub const LON: usize = 4;
    pub const ELEVATION_FT: usize = 5;
    pub const CONTINENT: usize = 6;
    pub const COUNTRY: usize = 7;
    pub const REGION: usize = 8;
    pub const MUNICIPALITY: usize = 9;
    pub const SCHEDULED: usize = 10;
    pub const IATA: usize = 11;
    pub const ICAO: usize = 12;
    pub const GPS: usize = 13;
    pub const LOCAL: usize = 14;
    pub const COUNT: usize = 15;
}

/// A field that is empty in the upstream data becomes `None`. Every field is
/// borrowed from the embedded table, so it is `'static`.
fn optional(fields: &[&'static str], index: usize) -> Option<&'static str> {
    let value = fields[index];
    if value.is_empty() { None } else { Some(value) }
}

/// One airport record.
///
/// Fields that are empty in the upstream data are `None`; `ident`, `name`,
/// `country`, `region` and `continent` are always present, because the
/// generator drops any row where they would be empty.
#[derive(Debug, Clone, PartialEq)]
pub struct Airport {
    ident: &'static str,
    kind: Kind,
    name: &'static str,
    lat: f64,
    lon: f64,
    elevation_ft: Option<i32>,
    continent: &'static str,
    country: &'static str,
    region: &'static str,
    municipality: Option<&'static str>,
    scheduled: bool,
    codes: Codes,
}

impl Airport {
    /// Build from one generated TSV row. The column order is [`col`], which the
    /// generator and this function must agree on; `table` asserts the row width
    /// before calling.
    pub(crate) fn from_fields(fields: &[&'static str]) -> Self {
        Self {
            ident: fields[col::IDENT],
            kind: Kind::from_str(fields[col::KIND]),
            name: fields[col::NAME],
            lat: fields[col::LAT].parse().expect("latitude"),
            lon: fields[col::LON].parse().expect("longitude"),
            elevation_ft: optional(fields, col::ELEVATION_FT)
                .map(|v| v.parse().expect("elevation")),
            continent: fields[col::CONTINENT],
            country: fields[col::COUNTRY],
            region: fields[col::REGION],
            municipality: optional(fields, col::MUNICIPALITY),
            scheduled: fields[col::SCHEDULED] == "yes",
            codes: Codes {
                iata: optional(fields, col::IATA),
                icao: optional(fields, col::ICAO),
                gps: optional(fields, col::GPS),
                local: optional(fields, col::LOCAL),
            },
        }
    }

    /// The OurAirports identifier, such as `"EGLL"`. Unique across the table.
    #[must_use]
    pub const fn ident(&self) -> &'static str {
        self.ident
    }

    /// The facility kind.
    #[must_use]
    pub const fn kind(&self) -> Kind {
        self.kind
    }

    /// The airport name as published, such as `"London Heathrow Airport"`.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Latitude in decimal degrees, positive north.
    #[must_use]
    pub const fn latitude(&self) -> f64 {
        self.lat
    }

    /// Longitude in decimal degrees, positive east.
    #[must_use]
    pub const fn longitude(&self) -> f64 {
        self.lon
    }

    /// Elevation in feet above mean sea level, when published.
    #[must_use]
    pub const fn elevation_ft(&self) -> Option<i32> {
        self.elevation_ft
    }

    /// The two-letter continent code, such as `"EU"`.
    #[must_use]
    pub const fn continent(&self) -> &'static str {
        self.continent
    }

    /// The ISO 3166-1 alpha-2 country code, such as `"GB"`.
    #[must_use]
    pub const fn country(&self) -> &'static str {
        self.country
    }

    /// The ISO 3166-2 region code, such as `"GB-ENG"`.
    #[must_use]
    pub const fn region(&self) -> &'static str {
        self.region
    }

    /// The city or place the airport serves, when published.
    #[must_use]
    pub const fn municipality(&self) -> Option<&'static str> {
        self.municipality
    }

    /// Whether the airport has scheduled commercial service.
    #[must_use]
    pub const fn has_scheduled_service(&self) -> bool {
        self.scheduled
    }

    /// The IATA code, such as `"LHR"`, when the airport has one.
    #[must_use]
    pub const fn iata(&self) -> Option<&'static str> {
        self.codes.iata
    }

    /// The ICAO code, such as `"EGLL"`, when the airport has one.
    #[must_use]
    pub const fn icao(&self) -> Option<&'static str> {
        self.codes.icao
    }

    /// The GPS code, when published.
    #[must_use]
    pub const fn gps(&self) -> Option<&'static str> {
        self.codes.gps
    }

    /// The local code, when published.
    #[must_use]
    pub const fn local(&self) -> Option<&'static str> {
        self.codes.local
    }

    /// The coordinates as `(latitude, longitude)` in decimal degrees.
    #[must_use]
    pub const fn coordinates(&self) -> (f64, f64) {
        (self.lat, self.lon)
    }
}

impl fmt::Display for Airport {
    /// `ident` and name, such as `EGLL (London Heathrow Airport)`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} ({})", self.ident, self.name)
    }
}

/// Every airport in the table, in `ident` order.
#[must_use]
pub fn all() -> &'static [Airport] {
    table::all()
}

/// The number of airports in the table.
#[must_use]
pub fn count() -> usize {
    table::all().len()
}

/// Case-insensitive equality against a code, treating an empty code as no match.
fn code_eq(code: Option<&str>, query: &str) -> bool {
    code.is_some_and(|c| c.eq_ignore_ascii_case(query))
}

/// Find one airport by any of its codes: IATA, ICAO, the GPS code, the local
/// code, or the OurAirports `ident`. Case-insensitive.
///
/// Codes are searched in **space order**, and the code columns outrank `ident`:
/// every airport is checked for an IATA match, then an ICAO match, and so on,
/// before any airport is checked for an `ident` match. The order matters because
/// the spaces overlap. `ident` is an internal key that sometimes names a
/// *different* airport's code (35 rows are like this, 15 of them through ICAO),
/// so an `ident`-first search would return the wrong airport for
/// codes such as `WABK`, where `ident` is on Karubaga Airport but the ICAO code
/// `WABK` is on Biri Airstrip. `IMO` is the other shape: an IATA code on one
/// airport and a local code on another, which is why IATA outranks local.
///
/// Within a space only `local` repeats, so the first match in `ident` order is
/// the winner there and the only match everywhere else.
#[must_use]
pub fn find(code: &str) -> Option<&'static Airport> {
    let table = table::all();
    table
        .iter()
        .find(|a| code_eq(a.codes.iata, code))
        .or_else(|| table.iter().find(|a| code_eq(a.codes.icao, code)))
        .or_else(|| table.iter().find(|a| code_eq(a.codes.gps, code)))
        .or_else(|| table.iter().find(|a| code_eq(a.codes.local, code)))
        .or_else(|| table.iter().find(|a| a.ident.eq_ignore_ascii_case(code)))
}

/// Every airport in a country, by ISO 3166-1 alpha-2 code such as `"ID"`.
/// Case-insensitive. The order is the table order, which is `ident` order.
#[must_use]
pub fn in_country(country: &str) -> Vec<&'static Airport> {
    table::all()
        .iter()
        .filter(|a| a.country.eq_ignore_ascii_case(country))
        .collect()
}

/// Every airport on a continent, by two-letter code such as `"AS"`.
/// Case-insensitive.
#[must_use]
pub fn in_continent(continent: &str) -> Vec<&'static Airport> {
    table::all()
        .iter()
        .filter(|a| a.continent.eq_ignore_ascii_case(continent))
        .collect()
}

/// Every airport of a given kind.
#[must_use]
pub fn of_kind(kind: Kind) -> Vec<&'static Airport> {
    table::all().iter().filter(|a| a.kind == kind).collect()
}

/// Airports whose name or municipality contains `query`, case-insensitively.
///
/// This is a plain substring match with no ranking, so it is meant for a short
/// list, not for autocomplete over the whole table. An empty query matches
/// everything.
#[must_use]
pub fn search(query: &str) -> Vec<&'static Airport> {
    let needle = query.to_lowercase();
    table::all()
        .iter()
        .filter(|a| {
            a.name.to_lowercase().contains(&needle)
                || a.municipality
                    .is_some_and(|m| m.to_lowercase().contains(&needle))
        })
        .collect()
}

/// The mean Earth radius in kilometres, the sphere the haversine formula uses.
const EARTH_RADIUS_KM: f64 = 6371.0088;

/// The great-circle distance between two coordinates in kilometres, by the
/// haversine formula on a sphere of mean Earth radius.
#[must_use]
pub fn haversine_km(from_lat: f64, from_lon: f64, to_lat: f64, to_lon: f64) -> f64 {
    let (lat1, lon1) = (from_lat.to_radians(), from_lon.to_radians());
    let (lat2, lon2) = (to_lat.to_radians(), to_lon.to_radians());
    let dlat = lat2 - lat1;
    let dlon = lon2 - lon1;
    let a = (dlat / 2.0).sin().powi(2) + lat1.cos() * lat2.cos() * (dlon / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_KM * a.sqrt().asin()
}

/// The great-circle distance between two airports in kilometres.
#[must_use]
pub fn distance_km(from: &Airport, to: &Airport) -> f64 {
    haversine_km(from.lat, from.lon, to.lat, to.lon)
}

/// The airports within `radius_km` of a coordinate, nearest first, each with its
/// distance in kilometres.
///
/// The whole table is scanned, so this is linear in the number of airports; it
/// is meant for occasional queries, not for a hot path.
#[must_use]
pub fn near(lat: f64, lon: f64, radius_km: f64) -> Vec<(&'static Airport, f64)> {
    let mut out: Vec<(&'static Airport, f64)> = table::all()
        .iter()
        .filter_map(|a| {
            let d = haversine_km(lat, lon, a.lat, a.lon);
            (d <= radius_km).then_some((a, d))
        })
        .collect();
    out.sort_by(|x, y| x.1.total_cmp(&y.1));
    out
}
