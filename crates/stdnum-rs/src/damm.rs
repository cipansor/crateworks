//! The Damm check digit, used where the Luhn check is not enough.
//!
//! Damm is a single-table algorithm over an anti-symmetric quasigroup of order
//! ten. It catches every single-digit error and every adjacent transposition,
//! which is more than Luhn manages: Luhn misses the transposition `09` to `90`.

use crate::checksum;
use crate::{Error, Result};

/// A number that is valid only if it satisfies the Damm check.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Damm(String);

impl Damm {
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
        if checksum::damm(&digits) != 0 {
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
        let digits = checksum::digits_only(payload)?;
        Ok(checksum::damm(&digits))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_published_example() {
        // The reference's own doctest: `calc_check_digit('572')` is `'4'`.
        assert_eq!(Damm::check_digit_for("572").unwrap(), 4);
        let valid = Damm::parse("5724").unwrap();
        assert_eq!(valid.as_str(), "5724");
        assert_eq!(valid.check_digit(), 4);
    }

    #[test]
    fn catches_every_single_digit_error() {
        // Replace one digit of a valid number and the check must fail.
        for i in 0..4 {
            for d in 0u8..10 {
                let mut bytes = *b"5724";
                bytes[i] = b'0' + d;
                let candidate = std::str::from_utf8(&bytes).unwrap();
                assert_eq!(
                    Damm::parse(candidate).is_ok(),
                    candidate == "5724",
                    "{candidate} should be rejected"
                );
            }
        }
    }

    #[test]
    fn catches_adjacent_transposition() {
        // `09` to `90` is the transposition Luhn misses; Damm catches it. The
        // two payloads need different check digits, so a number built from one
        // is rejected when it carries the other's digit.
        let c1 = Damm::check_digit_for("109").unwrap();
        let c2 = Damm::check_digit_for("190").unwrap();
        assert_ne!(c1, c2);
        assert!(Damm::parse(&format!("109{c1}")).is_ok());
        assert_eq!(Damm::parse(&format!("190{c1}")), Err(Error::BadChecksum));
        assert!(Damm::parse(&format!("190{c2}")).is_ok());
    }

    #[test]
    fn rejects_non_digits_and_empty_input() {
        assert!(matches!(
            Damm::parse("12a4"),
            Err(Error::InvalidCharacter('A'))
        ));
        assert_eq!(Damm::parse("  "), Err(Error::Empty));
    }
}
