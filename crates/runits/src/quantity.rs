//! A numeric value together with a unit, and conversion between units.

use crate::Dimension;
use crate::parse::{ParseError, parse};
use crate::units::Unit;
use std::error::Error;
use std::fmt;

/// A value and the unit it is written in, such as `3.5 kg`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Quantity {
    value: f64,
    unit: Unit,
}

impl Quantity {
    /// Build a quantity from a value and a unit.
    #[must_use]
    pub const fn new(value: f64, unit: Unit) -> Self {
        Self { value, unit }
    }

    /// Parse a quantity written as text, such as `"3.5 kg"` or `"60 km/h"`.
    ///
    /// ```
    /// use runits::Quantity;
    ///
    /// let speed = Quantity::parse("60 km/h").unwrap();
    /// assert!((speed.to("m/s").unwrap().value() - 16.666).abs() < 0.001);
    /// ```
    pub fn parse(text: &str) -> Result<Self, ParseError> {
        parse(text)
    }

    /// The number, in the unit's own terms. `3.5 kg` is `3.5`.
    #[must_use]
    pub const fn value(&self) -> f64 {
        self.value
    }

    /// The unit the value is written in.
    #[must_use]
    pub const fn unit(&self) -> Unit {
        self.unit
    }

    /// The dimension this quantity measures.
    #[must_use]
    pub const fn dimension(&self) -> Dimension {
        self.unit.dimension()
    }

    /// The value expressed in the SI base units, with the offset applied.
    ///
    /// `25 °C` is `298.15` here, because the base unit is the kelvin.
    #[must_use]
    pub fn base(&self) -> f64 {
        self.value * self.unit.factor() + self.unit.offset()
    }

    /// Convert to another unit, given as a name or expression.
    ///
    /// ```
    /// use runits::Quantity;
    ///
    /// let mass = Quantity::parse("3.5 kg").unwrap();
    /// assert!((mass.to("lb").unwrap().value() - 7.716).abs() < 0.001);
    ///
    /// let boiling = Quantity::parse("100 degC").unwrap();
    /// assert!((boiling.to("degF").unwrap().value() - 212.0).abs() < 1e-9);
    /// ```
    pub fn to(&self, target: &str) -> Result<Self, ConvertError> {
        let parsed = parse(target).map_err(ConvertError::BadTarget)?;
        self.to_quantity(parsed)
    }

    /// Convert to a target that is itself a quantity expression, such as
    /// `"m/s"` or `"kg*m/s^2"`.
    pub fn to_quantity(&self, target: Self) -> Result<Self, ConvertError> {
        if self.dimension() != target.dimension() {
            return Err(ConvertError::Dimension {
                from: self.dimension(),
                to: target.dimension(),
            });
        }
        if target.unit.factor() == 0.0 {
            return Err(ConvertError::ZeroScale);
        }
        Ok(Self::new(
            (self.base() - target.unit.offset()) / target.unit.factor(),
            target.unit,
        ))
    }

    /// Multiply by another quantity, combining the units.
    pub fn times(&self, other: Self) -> Result<Self, ArithmeticError> {
        if self.unit.is_affine() || other.unit.is_affine() {
            return Err(ArithmeticError::AffineUnit);
        }
        let factor = self.unit.factor() * other.unit.factor();
        let dimension = combine(self.dimension(), other.dimension(), 1)?;
        Ok(Self::new(
            self.value * other.value,
            Unit::from_parts(factor, dimension),
        ))
    }

    /// Divide by another quantity, combining the units.
    pub fn divided_by(&self, other: Self) -> Result<Self, ArithmeticError> {
        if other.value == 0.0 {
            return Err(ArithmeticError::DivideByZero);
        }
        if self.unit.is_affine() || other.unit.is_affine() {
            return Err(ArithmeticError::AffineUnit);
        }
        let factor = self.unit.factor() / other.unit.factor();
        let dimension = combine(self.dimension(), other.dimension(), -1)?;
        Ok(Self::new(
            self.value / other.value,
            Unit::from_parts(factor, dimension),
        ))
    }
}

fn combine(left: Dimension, right: Dimension, sign: i32) -> Result<Dimension, ArithmeticError> {
    if sign > 0 {
        left.times(right)
    } else {
        left.divided_by(right)
    }
    .ok_or(ArithmeticError::DimensionOverflow)
}

impl fmt::Display for Quantity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.value)
    }
}

/// Why a conversion could not be carried out.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ConvertError {
    /// The two units measure different things, so no factor relates them.
    Dimension {
        /// The dimension of the quantity being converted.
        from: Dimension,
        /// The dimension of the requested target.
        to: Dimension,
    },
    /// The target text is not a valid unit expression.
    BadTarget(ParseError),
    /// The target unit's scale is zero, so the value cannot be recovered.
    ZeroScale,
}

impl fmt::Display for ConvertError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dimension { from, to } => write!(formatter, "cannot convert {from} to {to}"),
            Self::BadTarget(error) => write!(formatter, "bad target unit: {error}"),
            Self::ZeroScale => formatter.write_str("the target unit has a zero scale"),
        }
    }
}

impl Error for ConvertError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::BadTarget(error) => Some(error),
            _ => None,
        }
    }
}

/// Why a quantity could not be multiplied or divided.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ArithmeticError {
    /// An affine unit, such as `degC`, cannot take part in a product: the
    /// offset has no meaning once the unit is scaled or combined.
    AffineUnit,
    /// A division by a zero value.
    DivideByZero,
    /// The resulting exponent does not fit in the `i8` the table stores.
    DimensionOverflow,
}

impl fmt::Display for ArithmeticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AffineUnit => {
                formatter.write_str("an affine unit such as degC cannot be multiplied or divided")
            }
            Self::DivideByZero => formatter.write_str("division by zero"),
            Self::DimensionOverflow => {
                formatter.write_str("the resulting dimension exponent is out of range")
            }
        }
    }
}

impl Error for ArithmeticError {}

/// Convert a value between two named units.
///
/// ```
/// use runits::convert;
///
/// assert_eq!(convert(1.0, "m", "cm").unwrap(), 100.0);
/// ```
pub fn convert(value: f64, from: &str, to: &str) -> Result<f64, ConvertError> {
    let source = Quantity::parse(&format!("{value} {from}")).map_err(ConvertError::BadTarget)?;
    Ok(source.to(to)?.value())
}
