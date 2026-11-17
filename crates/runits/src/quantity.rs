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
    /// `25 °C` is `298.15` here, because the base unit is the kelvin. For a
    /// logarithmic unit this is the reference quantity scaled by the ratio the
    /// value names: `30 dBm` is `1.0`, one watt.
    #[must_use]
    pub fn base(&self) -> f64 {
        self.unit.base_of(self.value)
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
        if target.unit.factor() == 0.0 && !target.unit.is_logarithmic() {
            return Err(ConvertError::ZeroScale);
        }
        // Two logarithmic scales convert in log space, so a finite value whose
        // linear equivalent overflows `f64` still converts: `400 Np` is about
        // `3474 dB`, and `e^800` would have been the lost intermediate.
        if let (Some(from), Some(to)) = (self.unit.log_scale(), target.unit.log_scale()) {
            return Ok(Self::new(to.value_from(from, self.value), target.unit));
        }
        let base = self.base();
        // A zero or negative quantity has no logarithm, so it cannot be written
        // on a logarithmic scale; the reference raises the same way.
        if target.unit.is_logarithmic() && base <= 0.0 {
            return Err(ConvertError::NotPositive);
        }
        Ok(Self::new(target.unit.value_from(base), target.unit))
    }

    /// Multiply by another quantity, combining the units.
    pub fn times(&self, other: Self) -> Result<Self, ArithmeticError> {
        if self.unit.is_affine() || other.unit.is_affine() {
            return Err(ArithmeticError::AffineUnit);
        }
        if self.unit.is_logarithmic() || other.unit.is_logarithmic() {
            return Err(ArithmeticError::LogarithmicUnit);
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
        if self.unit.is_logarithmic() || other.unit.is_logarithmic() {
            return Err(ArithmeticError::LogarithmicUnit);
        }
        let factor = self.unit.factor() / other.unit.factor();
        let dimension = combine(self.dimension(), other.dimension(), -1)?;
        Ok(Self::new(
            self.value / other.value,
            Unit::from_parts(factor, dimension),
        ))
    }

    /// Add another quantity, keeping this one's unit.
    ///
    /// The right operand is first expressed in the left one's unit, so
    /// `3.5 kg + 500 g` is `4 kg`. The two must measure the same dimension, and
    /// neither may be an affine scale such as `degC`, whose offset has no
    /// meaning in a sum.
    ///
    /// ```
    /// use runits::Quantity;
    ///
    /// let total = Quantity::parse("3.5 kg")
    ///     .unwrap()
    ///     .add(Quantity::parse("500 g").unwrap())
    ///     .unwrap();
    /// assert!((total.value() - 4.0).abs() < 1e-9);
    /// ```
    pub fn add(&self, other: Self) -> Result<Self, ArithmeticError> {
        self.shifted_by(other, 1.0)
    }

    /// Subtract another quantity, keeping this one's unit.
    ///
    /// The mirror of [`Quantity::add`], with the same rules.
    ///
    /// ```
    /// use runits::Quantity;
    ///
    /// let left = Quantity::parse("1 m")
    ///     .unwrap()
    ///     .subtract(Quantity::parse("25 cm").unwrap())
    ///     .unwrap();
    /// assert!((left.value() - 0.75).abs() < 1e-9);
    /// ```
    pub fn subtract(&self, other: Self) -> Result<Self, ArithmeticError> {
        self.shifted_by(other, -1.0)
    }

    /// `self + sign * other`, with `other` converted into `self`'s unit.
    fn shifted_by(&self, other: Self, sign: f64) -> Result<Self, ArithmeticError> {
        if self.unit.is_affine() || other.unit.is_affine() {
            return Err(ArithmeticError::AffineUnit);
        }
        if self.unit.is_logarithmic() || other.unit.is_logarithmic() {
            return self.shifted_logarithmically(other, sign);
        }
        if self.dimension() != other.dimension() {
            return Err(ArithmeticError::DimensionMismatch {
                left: self.dimension(),
                right: other.dimension(),
            });
        }
        let in_self = sign * other.value * other.unit.factor() / self.unit.factor();
        Ok(Self::new(self.value + in_self, self.unit))
    }

    /// A sum or difference where at least one operand is on a logarithmic scale.
    ///
    /// A dimensionless logarithmic value is a logarithm of a ratio, so adding
    /// two of them adds their logarithms: two `3 dB` gains make `6 dB`, and
    /// `3 dB + 1 Np` is `11.686 dB`. The right operand is first expressed in
    /// `self`'s scale through the base quantity they share, and the result
    /// keeps `self`'s scale.
    ///
    /// Only the dimensionless scales can be added. For a dimensioned one such
    /// as `dBm`, the reference multiplies the two reference quantities and
    /// returns the base unit - `0 dBm + 1 dBm` becomes a squared watt, a
    /// quantity neither operand named - so the crate refuses it rather than
    /// report a number with no meaning. Adding a logarithmic value to a linear
    /// one is refused too: `0 dBm + 1 mW` has no meaning, and the reference
    /// refuses that as well.
    fn shifted_logarithmically(&self, other: Self, sign: f64) -> Result<Self, ArithmeticError> {
        if self.dimension() != other.dimension() {
            return Err(ArithmeticError::DimensionMismatch {
                left: self.dimension(),
                right: other.dimension(),
            });
        }
        if !(self.unit.is_logarithmic() && other.unit.is_logarithmic()) {
            return Err(ArithmeticError::LogarithmicUnit);
        }
        if !self.dimension().is_dimensionless() {
            return Err(ArithmeticError::LogarithmicUnit);
        }
        // Both operands are logarithms, so express the right one in `self`'s
        // scale in log space and add the two numbers. This keeps a finite
        // result whose linear equivalent is outside `f64`: `200 Np + 200 Np` is
        // `400 Np`, whose base quantity is `e^800`.
        let right = self.unit.log_scale().expect("self is logarithmic");
        let other_scale = other.unit.log_scale().expect("other is logarithmic");
        let in_self = right.value_from(other_scale, other.value);
        Ok(Self::new(self.value + sign * in_self, self.unit))
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
    /// A zero or negative quantity cannot be written on a logarithmic scale.
    NotPositive,
}

impl fmt::Display for ConvertError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Dimension { from, to } => write!(formatter, "cannot convert {from} to {to}"),
            Self::BadTarget(error) => write!(formatter, "bad target unit: {error}"),
            Self::ZeroScale => formatter.write_str("the target unit has a zero scale"),
            Self::NotPositive => formatter.write_str("only a positive quantity has a logarithm"),
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
    /// A sum or difference of two quantities that measure different things.
    DimensionMismatch {
        /// The dimension of the left operand, whose unit the result keeps.
        left: Dimension,
        /// The dimension of the right operand.
        right: Dimension,
    },
    /// A logarithmic scale took part in an operation that needs a linear one: a
    /// product, a quotient, or a sum with a linear quantity.
    LogarithmicUnit,
}

impl fmt::Display for ArithmeticError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AffineUnit => {
                formatter.write_str("an affine unit such as degC cannot be combined with another")
            }
            Self::DivideByZero => formatter.write_str("division by zero"),
            Self::DimensionOverflow => {
                formatter.write_str("the resulting dimension exponent is out of range")
            }
            Self::DimensionMismatch { left, right } => {
                write!(formatter, "cannot add or subtract {left} and {right}")
            }
            Self::LogarithmicUnit => formatter.write_str(
                "a logarithmic scale cannot be multiplied, divided or added to a linear quantity",
            ),
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
