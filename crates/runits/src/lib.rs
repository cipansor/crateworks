#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod table;

pub mod parse;
pub mod quantity;
pub mod units;

pub use parse::{ParseError, parse};
pub use quantity::{ArithmeticError, ConvertError, Quantity, convert};
pub use units::{Unit, UnknownUnit, unit};

use std::fmt;

/// The seven SI base dimensions, in the order the table stores them.
///
/// A quantity's dimension is a vector of exponents over these, so `kg·m/s²` has
/// `[1, -2, 0, 0, 1, 0, 0]`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimension {
    /// `[length]`, in metres.
    pub length: i8,
    /// `[time]`, in seconds.
    pub time: i8,
    /// `[mass]`, in grams.
    pub mass: i8,
    /// `[current]`, in amperes.
    pub current: i8,
    /// `[temperature]`, in kelvin.
    pub temperature: i8,
    /// `[substance]`, in moles.
    pub substance: i8,
    /// `[luminosity]`, in candelas.
    pub luminosity: i8,
}

impl Dimension {
    /// The dimension of a dimensionless quantity, such as an angle.
    pub const NONE: Self = Self {
        length: 0,
        time: 0,
        mass: 0,
        current: 0,
        temperature: 0,
        substance: 0,
        luminosity: 0,
    };

    pub(crate) const fn from_table(dim: [i8; 7]) -> Self {
        Self {
            length: dim[0],
            time: dim[1],
            mass: dim[2],
            current: dim[3],
            temperature: dim[4],
            substance: dim[5],
            luminosity: dim[6],
        }
    }

    /// Whether this is the dimension of a plain number.
    #[must_use]
    pub fn is_dimensionless(self) -> bool {
        self == Self::NONE
    }

    /// Whether two dimensions are the same, so a conversion between them is a
    /// pure scale change with no offset.
    #[must_use]
    pub fn same_as(self, other: Self) -> bool {
        self == other
    }

    /// The dimension of a product of two quantities.
    #[must_use]
    pub fn times(self, other: Self) -> Option<Self> {
        self.combine(other, 1)
    }

    /// The dimension of a quotient of two quantities.
    #[must_use]
    pub fn divided_by(self, other: Self) -> Option<Self> {
        self.combine(other, -1)
    }

    /// The dimension of a quantity raised to a power. Returns `None` if an
    /// exponent leaves the `i8` range the table stores.
    #[must_use]
    pub fn pow(self, exponent: i32) -> Option<Self> {
        let scale = |value: i8| {
            let product = i32::from(value).checked_mul(exponent)?;
            i8::try_from(product).ok()
        };
        Some(Self {
            length: scale(self.length)?,
            time: scale(self.time)?,
            mass: scale(self.mass)?,
            current: scale(self.current)?,
            temperature: scale(self.temperature)?,
            substance: scale(self.substance)?,
            luminosity: scale(self.luminosity)?,
        })
    }

    fn combine(self, other: Self, sign: i32) -> Option<Self> {
        let add = |left: i8, right: i8| {
            let sum = i32::from(left) + sign * i32::from(right);
            i8::try_from(sum).ok()
        };
        Some(Self {
            length: add(self.length, other.length)?,
            time: add(self.time, other.time)?,
            mass: add(self.mass, other.mass)?,
            current: add(self.current, other.current)?,
            temperature: add(self.temperature, other.temperature)?,
            substance: add(self.substance, other.substance)?,
            luminosity: add(self.luminosity, other.luminosity)?,
        })
    }
}

impl fmt::Display for Dimension {
    /// Renders the dimension as a product of base symbols, such as `kg·m/s²`.
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let symbols = [
            ("m", self.length),
            ("s", self.time),
            ("g", self.mass),
            ("A", self.current),
            ("K", self.temperature),
            ("mol", self.substance),
            ("cd", self.luminosity),
        ];
        if self.is_dimensionless() {
            return formatter.write_str("dimensionless");
        }
        let mut first = true;
        for (symbol, exponent) in symbols {
            if exponent == 0 {
                continue;
            }
            if !first {
                formatter.write_str("·")?;
            }
            first = false;
            formatter.write_str(symbol)?;
            match exponent {
                1 => {}
                _ => write!(formatter, "^{exponent}")?,
            }
        }
        Ok(())
    }
}
