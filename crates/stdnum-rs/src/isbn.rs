//! ISBN-10 and ISBN-13 book identifiers.

use crate::checksum;
use crate::{Error, Result};

/// An ISBN, in either its 10- or 13-digit form.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Isbn {
    digits: String,
    isbn13: bool,
}

impl Isbn {
    /// Validates `input`.
    ///
    /// Accepts an optional `ISBN` or `ISBN-13` prefix, spaces and hyphens. The
    /// final character of an ISBN-10 may be `X`, standing for ten.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let mut normalized = crate::normalize(input);
        for prefix in ["ISBN-13", "ISBN13", "ISBN"] {
            if let Some(rest) = normalized.strip_prefix(prefix) {
                normalized = rest.to_string();
                break;
            }
        }
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        match normalized.len() {
            10 => {
                if !Self::isbn10_checksum_ok(&normalized)? {
                    return Err(Error::BadChecksum);
                }
                Ok(Self {
                    digits: normalized,
                    isbn13: false,
                })
            }
            13 => {
                let digits = checksum::digits_only(&normalized)?;
                let (payload, check) = digits.split_at(12);
                if checksum::gs1_check_digit(payload) != check[0] {
                    return Err(Error::BadChecksum);
                }
                Ok(Self {
                    digits: normalized,
                    isbn13: true,
                })
            }
            other => Err(Error::WrongLength {
                expected: 13,
                found: other,
            }),
        }
    }

    fn isbn10_checksum_ok(s: &str) -> Result<bool> {
        let mut sum = 0u32;
        for (i, c) in s.chars().enumerate() {
            let value = if i == 9 && c == 'X' {
                10
            } else {
                c.to_digit(10).ok_or(Error::InvalidCharacter(c))?
            };
            // Weights run from 10 down to 1.
            sum += value * (10 - i as u32);
        }
        Ok(sum.is_multiple_of(11))
    }

    /// The ISBN-10 check character for the nine digits before it.
    ///
    /// Weights run from 1 to 9, the reference's convention; a remainder of ten
    /// is written `X`.
    fn isbn10_check_character(body: &str) -> Result<char> {
        let mut sum = 0u32;
        for (i, c) in body.chars().enumerate() {
            let digit = c.to_digit(10).ok_or(Error::InvalidCharacter(c))?;
            sum += (i as u32 + 1) * digit;
        }
        Ok(match sum % 11 {
            10 => 'X',
            r => char::from(b'0' + r as u8),
        })
    }

    /// The normalised identifier.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.digits
    }

    /// Whether this is the 13-digit form.
    #[must_use]
    pub fn is_isbn13(&self) -> bool {
        self.isbn13
    }

    /// Converts an ISBN-10 to its ISBN-13 equivalent.
    ///
    /// # Errors
    ///
    /// Returns an error only if the constructed number fails validation, which
    /// cannot happen for an already-valid ISBN-10.
    pub fn to_isbn13(&self) -> Result<Self> {
        if self.isbn13 {
            return Self::parse(&self.digits);
        }
        let body = &self.digits[..9];
        let payload = format!("978{body}");
        let digits = checksum::digits_only(&payload)?;
        let check = checksum::gs1_check_digit(&digits);
        Self::parse(&format!("{payload}{check}"))
    }

    /// Converts an ISBN-13 to its ISBN-10 equivalent.
    ///
    /// Only the `978` Bookland prefix has a ten-digit form; a `979` ISBN-13
    /// does not, and is rejected.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidPrefix`] if this is not a `978` ISBN-13, or an
    /// error from validation if the constructed number is rejected, which
    /// cannot happen for an already-valid ISBN-13.
    pub fn to_isbn10(&self) -> Result<Self> {
        if !self.isbn13 {
            return Self::parse(&self.digits);
        }
        if !self.digits.starts_with("978") {
            return Err(Error::InvalidPrefix);
        }
        let body = &self.digits[3..12];
        let check = Self::isbn10_check_character(body)?;
        Self::parse(&format!("{body}{check}"))
    }

    /// The registration group element, roughly the language area.
    ///
    /// The element boundaries are defined by ranges rather than by a fixed
    /// length, so this returns the part up to the first hyphen in the input and
    /// cannot be exact for a hyphen-less number.
    #[must_use]
    pub fn group_prefix(&self) -> &str {
        let body = if self.isbn13 {
            &self.digits[3..]
        } else {
            &self.digits
        };
        &body[..1.min(body.len())]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_known_isbn13() {
        let i = Isbn::parse("9780306406157").unwrap();
        assert!(i.is_isbn13());
        assert_eq!(i.as_str(), "9780306406157");
    }

    #[test]
    fn accepts_known_isbn10() {
        let i = Isbn::parse("0306406152").unwrap();
        assert!(!i.is_isbn13());
    }

    #[test]
    fn accepts_isbn10_ending_in_x() {
        // 097522980X is a published ISBN-10 with an X check character.
        assert!(Isbn::parse("097522980X").is_ok());
    }

    #[test]
    fn rejects_bad_checksum() {
        assert_eq!(Isbn::parse("9780306406158"), Err(Error::BadChecksum));
        assert_eq!(Isbn::parse("0306406153"), Err(Error::BadChecksum));
    }

    #[test]
    fn rejects_bad_length_and_characters() {
        assert!(matches!(
            Isbn::parse("12345"),
            Err(Error::WrongLength { .. })
        ));
        assert!(matches!(
            Isbn::parse("978030640615A"),
            Err(Error::InvalidCharacter('A'))
        ));
    }

    #[test]
    fn strips_the_isbn_prefix() {
        assert!(Isbn::parse("ISBN 978-0-306-40615-7").is_ok());
        assert!(Isbn::parse("ISBN-13: 978-0-306-40615-7").is_ok());
    }

    #[test]
    fn isbn10_converts_to_isbn13() {
        let ten = Isbn::parse("0306406152").unwrap();
        let thirteen = ten.to_isbn13().unwrap();
        assert_eq!(thirteen.as_str(), "9780306406157");
        assert!(thirteen.is_isbn13());
    }

    #[test]
    fn converting_isbn13_is_idempotent() {
        let i = Isbn::parse("9780306406157").unwrap();
        assert_eq!(i.to_isbn13().unwrap().as_str(), "9780306406157");
    }

    #[test]
    fn isbn13_converts_to_isbn10() {
        let thirteen = Isbn::parse("9780306406157").unwrap();
        let ten = thirteen.to_isbn10().unwrap();
        assert_eq!(ten.as_str(), "0306406152");
        assert!(!ten.is_isbn13());
    }

    #[test]
    fn converting_isbn10_to_ten_is_idempotent() {
        let ten = Isbn::parse("0306406152").unwrap();
        assert_eq!(ten.to_isbn10().unwrap().as_str(), "0306406152");
    }

    #[test]
    fn isbn10_check_character_may_be_x() {
        // 9780975229804 has an ISBN-10 form whose check character is X.
        let thirteen = Isbn::parse("9780975229804").unwrap();
        assert_eq!(thirteen.to_isbn10().unwrap().as_str(), "097522980X");
    }

    #[test]
    fn a_979_isbn13_has_no_isbn10_form() {
        let thirteen = Isbn::parse("9791234567896").unwrap();
        assert_eq!(thirteen.to_isbn10(), Err(Error::InvalidPrefix));
    }

    #[test]
    fn the_two_conversions_round_trip() {
        for code in ["9780306406157", "9780975229804", "9783161484100"] {
            let thirteen = Isbn::parse(code).unwrap();
            let ten = thirteen.to_isbn10().unwrap();
            assert_eq!(ten.to_isbn13().unwrap().as_str(), code);
        }
    }
}
