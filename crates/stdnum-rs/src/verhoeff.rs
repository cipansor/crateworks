//! The Verhoeff check digit, a decimal checksum stronger than Luhn.
//!
//! Verhoeff is built from a multiplication table and a permutation table. It
//! catches every single-digit error and every adjacent transposition, including
//! the `09`/`90` swap that Luhn misses, and it is used by several national
//! identifiers where a stronger check is wanted.

use crate::checksum;
use crate::{Error, Result};

/// A number that is valid only if it satisfies the Verhoeff check.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Verhoeff(String);

impl Verhoeff {
    /// Validates `input` and returns the normalised digits.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`] or
    /// [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        let digits = checksum::digits_only(&normalized)?;
        if checksum::verhoeff(&digits) != 0 {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The check digit.
    #[must_use]
    pub fn check_digit(&self) -> u8 {
        self.0.as_bytes()[self.0.len() - 1] - b'0'
    }

    /// Computes the check digit for a payload, so callers can generate numbers.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCharacter`] if `payload` is not all digits.
    pub fn check_digit_for(payload: &str) -> Result<u8> {
        let mut digits = checksum::digits_only(payload)?;
        // The check digit is the one that drives the checksum back to zero.
        // Trying each in turn is what the reference's table lookup amounts to,
        // and the table has exactly one such digit per row.
        for candidate in 0u8..=9 {
            digits.push(candidate);
            let valid = checksum::verhoeff(&digits) == 0;
            digits.pop();
            if valid {
                return Ok(candidate);
            }
        }
        unreachable!("exactly one candidate satisfies the Verhoeff check")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_published_example() {
        // The reference's own doctest: `calc_check_digit('1234')` is `'0'`.
        assert_eq!(Verhoeff::check_digit_for("1234").unwrap(), 0);
        assert!(Verhoeff::parse("12340").is_ok());
    }

    #[test]
    fn catches_every_single_digit_error() {
        for i in 0..5 {
            for d in 0u8..10 {
                let mut bytes = *b"12340";
                bytes[i] = b'0' + d;
                let candidate = std::str::from_utf8(&bytes).unwrap();
                assert_eq!(
                    Verhoeff::parse(candidate).is_ok(),
                    candidate == "12340",
                    "{candidate} should be rejected"
                );
            }
        }
    }

    #[test]
    fn catches_adjacent_transposition() {
        let c1 = Verhoeff::check_digit_for("109").unwrap();
        let c2 = Verhoeff::check_digit_for("190").unwrap();
        assert_ne!(c1, c2);
        assert!(Verhoeff::parse(&format!("109{c1}")).is_ok());
        assert_eq!(
            Verhoeff::parse(&format!("190{c1}")),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn rejects_non_digits_and_empty_input() {
        assert!(matches!(
            Verhoeff::parse("12x4"),
            Err(Error::InvalidCharacter('X'))
        ));
        assert_eq!(Verhoeff::parse(""), Err(Error::Empty));
    }
}
