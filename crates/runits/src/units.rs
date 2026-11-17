//! Single-unit lookup: turning a written name such as `"kg"` into a scale.

use crate::Dimension;
use crate::table::{DEFINITIONS, LOG_FLAG, LOGARITHMIC, NAMES, PREFIXES, UNSUPPORTED};
use std::error::Error;
use std::fmt;

/// A logarithmic scale, such as `dB`, `Np` or `octave`.
///
/// The value is a logarithm of a ratio to a reference quantity:
/// `value = divisor * log_base(quantity / reference)`, where `reference` is
/// itself expressed in the SI base units. So `dBm` has `base = 10`,
/// `divisor = 10` and a one-milliwatt reference, and `0 dBm` is that reference.
/// `dB` and `Np` anchor to the dimensionless one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LogScale {
    pub(crate) base: f64,
    pub(crate) divisor: f64,
    pub(crate) reference: f64,
}

impl LogScale {
    /// The base of the logarithm: 10 for `dB`, e for `Np`, 2 for `octave`.
    #[must_use]
    pub const fn base(&self) -> f64 {
        self.base
    }

    /// The divisor applied to the logarithm: 10 for `dB`, 1 for `decade`.
    #[must_use]
    pub const fn divisor(&self) -> f64 {
        self.divisor
    }

    /// The reference quantity the ratio is taken against, in the crate's SI
    /// base units: `1.0` for `dB` (the dimensionless one) and `1.0` for `dBm`
    /// too, because one milliwatt is one gram-square-metre per second cubed
    /// when mass is measured in grams.
    #[must_use]
    pub const fn reference(&self) -> f64 {
        self.reference
    }

    /// The value on this scale that a quantity on `source` with value `value`
    /// converts to, staying in log space.
    ///
    /// Both scales describe the same quantity, so the conversion is
    /// `divisor * (log(value / source.divisor) * ln(source.base) - ln(self.reference
    /// / source.reference)) / ln(self.base)`. Exponentiating the source and taking
    /// the logarithm of the target - what [`Unit::base_of`] and
    /// [`Unit::value_from`] do - would lose a finite result whenever the
    /// intermediate base quantity overflows or underflows, and a logarithmic
    /// scale can name a value far outside `f64`'s range: `400 Np` is about
    /// `3474 dB`, whose linear equivalent is `e^800`.
    pub(crate) fn value_from(self, source: Self, value: f64) -> f64 {
        let ratio = value / source.divisor;
        self.divisor * (ratio * source.base.ln() + source.reference.ln() - self.reference.ln())
            / self.base.ln()
    }
}

/// A scale to the SI base units, together with the dimension it measures and an
/// additive offset for the affine temperature scales.
///
/// For a linear unit `base = value * factor + offset`, so `°C` has
/// `factor = 1` and `offset = 273.15`, while `°F` has `factor = 5/9` and
/// `offset = 255.372...`. A compound unit, such as `km/h`, always has an offset
/// of zero: an offset cannot survive multiplication.
///
/// A logarithmic unit is not linear at all; it carries a [`LogScale`] and its
/// `factor` is the reference quantity's own factor, so `unit("dBm").factor()`
/// is one milliwatt in grams.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Unit {
    pub(crate) factor: f64,
    pub(crate) dimension: Dimension,
    pub(crate) offset: f64,
    pub(crate) logarithmic: Option<LogScale>,
}

impl Unit {
    /// The factor to the SI base units, ignoring any offset. For a logarithmic
    /// unit this is the factor of its reference quantity.
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

    /// The dimension of this unit. For a logarithmic unit this is the dimension
    /// of its reference quantity, so `dB` is dimensionless and `dBm` is a power.
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

    /// Whether this is a logarithmic scale such as `dB` or `Np`.
    #[must_use]
    pub const fn is_logarithmic(&self) -> bool {
        self.logarithmic.is_some()
    }

    /// The logarithmic scale, or `None` for a linear unit.
    #[must_use]
    pub const fn log_scale(&self) -> Option<LogScale> {
        self.logarithmic
    }

    /// The value in the SI base units.
    pub(crate) fn base_of(self, value: f64) -> f64 {
        match self.logarithmic {
            Some(log) => log.reference * log.base.powf(value / log.divisor),
            None => value * self.factor + self.offset,
        }
    }

    /// The inverse of [`Unit::base_of`]: the value that has this base value.
    pub(crate) fn value_from(self, base: f64) -> f64 {
        match self.logarithmic {
            Some(log) => log.divisor * (base / log.reference).log(log.base),
            None => (base - self.offset) / self.factor,
        }
    }

    pub(crate) fn scale_by(&mut self, factor: f64) {
        self.factor *= factor;
    }

    /// A unit built from arithmetic on other units, which is never affine or
    /// logarithmic.
    pub(crate) const fn from_parts(factor: f64, dimension: Dimension) -> Self {
        Self {
            factor,
            dimension,
            offset: 0.0,
            logarithmic: None,
        }
    }
}

/// Why a unit name could not be turned into a [`Unit`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum UnknownUnit {
    /// No name, symbol or prefix combination matches.
    Unknown(String),
    /// The name is a real unit, but the crate cannot represent it. The Gaussian
    /// and CGS-EM units (`statC`, `Mx`) carry fractional dimension exponents,
    /// and a logarithmic scale (`dB`, `Np`) cannot take an SI prefix, since
    /// `mdB` would be a logarithm of a milli-ratio rather than a milli-logarithm.
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
        return resolve(row, name, None);
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
            if let Some(row) = exact(candidate) {
                return resolve(row, name, Some(*factor));
            }
        }
    }

    // The plural of an exact name, as in `"metres"`.
    if let Some(row) = name.strip_suffix('s').and_then(exact) {
        return resolve(row, name, None);
    }

    Err(UnknownUnit::Unknown(name.to_owned()))
}

/// Turn a name-table row into a unit, applying a prefix factor when one was
/// stripped. A logarithmic scale rejects the prefix: `mdB` is not a milli-log.
fn resolve(row: u16, name: &str, prefix: Option<f64>) -> Result<Unit, UnknownUnit> {
    if row == UNSUPPORTED {
        return Err(UnknownUnit::Unsupported(name.to_owned()));
    }
    let mut found = build(row);
    if let Some(factor) = prefix {
        if found.is_logarithmic() {
            return Err(UnknownUnit::Unsupported(name.to_owned()));
        }
        found.scale_by(factor);
    }
    Ok(found)
}

/// The exact-name lookup, before any prefix or plural handling.
fn exact(name: &str) -> Option<u16> {
    NAMES
        .binary_search_by(|(candidate, _)| (*candidate).cmp(name))
        .ok()
        .map(|position| NAMES[position].1)
}

fn build(row: u16) -> Unit {
    if row & LOG_FLAG != 0 {
        let definition = &LOGARITHMIC[(row & !LOG_FLAG) as usize];
        return Unit {
            factor: definition.reference,
            dimension: Dimension::from_table(definition.dim),
            offset: 0.0,
            logarithmic: Some(LogScale {
                base: definition.base,
                divisor: definition.factor,
                reference: definition.reference,
            }),
        };
    }
    let definition = &DEFINITIONS[row as usize];
    Unit {
        factor: definition.factor,
        dimension: Dimension::from_table(definition.dim),
        offset: definition.offset,
        logarithmic: None,
    }
}
