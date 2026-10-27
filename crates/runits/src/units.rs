//! Single-unit lookup: turning a written name such as `"kg"` into a scale.

use crate::Dimension;
use crate::table::{DEFINITIONS, NAMES, PREFIXES, UNSUPPORTED};
use std::error::Error;
use std::fmt;

/// A scale to the SI base units, together with the dimension it measures and an
/// additive offset for the affine temperature scales.
///
/// For a single unit `base = value * factor + offset`, so `°C` has
/// `factor = 1` and `offset = 273.15`, while `°F` has `factor = 5/9` and
/// `offset = 255.372...`. A compound unit, such as `km/h`, always has an offset
/// of zero: an offset cannot survive multiplication.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub(crate) factor: f64,
    pub(crate) dimension: Dimension,
    pub(crate) offset: f64,
}

impl Unit {
    /// The factor to the SI base units, ignoring any offset.
    #[must_use]
    pub const fn factor(&self) -> f64 {
        self.factor
    }

    /// The additive offset, zero for every unit except the affine temperature
    /// scales.
    #[must_use]
    pub const fn offset(&self) -> f64 {
        self.offset
    }

    /// The dimension of this unit.
    #[must_use]
    pub const fn dimension(&self) -> Dimension {
        self.dimension
    }

    /// Whether this unit has an offset, so a conversion to or from it is affine
    /// rather than a pure scale change.
    #[must_use]
    pub fn is_affine(&self) -> bool {
        self.offset != 0.0
    }

    pub(crate) fn scale_by(&mut self, factor: f64) {
        self.factor *= factor;
    }

    /// A unit built from arithmetic on other units, which is never affine.
    pub(crate) const fn from_parts(factor: f64, dimension: Dimension) -> Self {
        Self {
            factor,
            dimension,
            offset: 0.0,
        }
    }
}

/// Why a unit name could not be turned into a [`Unit`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnknownUnit {
    /// No name, symbol or prefix combination matches.
    Unknown(String),
    /// The name is a real unit, but it cannot be written as a vector over the
    /// seven SI base dimensions. The Gaussian and CGS-EM units and the
    /// logarithmic scales (`dB`, `Np`, `octave`) are in this set.
    Unsupported(String),
}

impl fmt::Display for UnknownUnit {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(name) => write!(formatter, "unknown unit {name:?}"),
            Self::Unsupported(name) => write!(
                formatter,
                "{name:?} is a real unit but cannot be expressed in the seven SI base dimensions"
            ),
        }
    }
}

impl Error for UnknownUnit {}

/// Look up a single unit by name, symbol or prefix.
///
/// The lookup order matches pint's own: an exact name first, then a prefix
/// applied to a known unit, then the plural of an exact name.
///
/// ```
/// use runits::unit;
///
/// assert_eq!(unit("kg").unwrap().dimension().mass, 1);
/// assert_eq!(unit("kilometre").unwrap().factor(), 1000.0);
/// assert!(unit("florps").is_err());
/// ```
pub fn unit(name: &str) -> Result<Unit, UnknownUnit> {
    if let Some(row) = exact(name) {
        return match row {
            UNSUPPORTED => Err(UnknownUnit::Unsupported(name.to_owned())),
            index => Ok(build(index)),
        };
    }

    // A prefix applied to a known unit. `PREFIXES` is ordered longest first, so
    // `da` is read as `deka` rather than as `d` + `a`.
    for (prefix, factor) in PREFIXES {
        let Some(remainder) = name.strip_prefix(prefix) else {
            continue;
        };
        if remainder.is_empty() {
            continue;
        }
        let singular = remainder.strip_suffix('s');
        for candidate in [Some(remainder), singular].into_iter().flatten() {
            match exact(candidate) {
                Some(UNSUPPORTED) => return Err(UnknownUnit::Unsupported(name.to_owned())),
                Some(index) => {
                    let mut found = build(index);
                    found.scale_by(*factor);
                    return Ok(found);
                }
                None => {}
            }
        }
    }

    // The plural of an exact name, as in `"metres"`.
    if let Some(row) = name.strip_suffix('s').and_then(exact) {
        return match row {
            UNSUPPORTED => Err(UnknownUnit::Unsupported(name.to_owned())),
            index => Ok(build(index)),
        };
    }

    Err(UnknownUnit::Unknown(name.to_owned()))
}

/// The exact-name lookup, before any prefix or plural handling.
fn exact(name: &str) -> Option<u16> {
    NAMES
        .binary_search_by(|(candidate, _)| (*candidate).cmp(name))
        .ok()
        .map(|position| NAMES[position].1)
}

fn build(index: u16) -> Unit {
    let definition = &DEFINITIONS[index as usize];
    Unit {
        factor: definition.factor,
        dimension: Dimension::from_table(definition.dim),
        offset: definition.offset,
    }
}
