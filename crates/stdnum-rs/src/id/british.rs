//! United Kingdom identifiers.
//!
//! Four unrelated numbers that share a country: the NHS number, the Unique
//! Taxpayer Reference, the VAT registration number and the SEDOL securities
//! identifier. Each is a weighted checksum over a fixed shape, and each is
//! cross-checked against `python-stdnum` by `tests/fixtures/stdnum_vectors.tsv`.

use crate::{Error, Result, checksum};

/// Removes the given separators, strips whitespace and upper-cases.
///
/// This is the reference's `clean()` plus `upper()`. The separator set differs
/// per identifier, which is why it is a parameter; the callers pass the raw
/// input so a separator the reference rejects is not silently dropped by the
/// crate's global normalisation.
fn compact(input: &str, separators: &[char]) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !separators.contains(c))
        .flat_map(char::to_uppercase)
        .collect()
}

/// United Kingdom: NHS number, UTR, VAT number and SEDOL.
pub(super) mod united_kingdom {
    use super::*;

    /// Validates an NHS number.
    ///
    /// Ten digits. The check is a weighted sum over the whole number with
    /// weights 1..10 from the right, which must be a multiple of eleven.
    pub(in crate::id) fn validate_nhs(s: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(s, &['-']))?;
        checksum::expect_len(&digits, 10)?;
        let sum: u32 = digits
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &d)| (i as u32 + 1) * u32::from(d))
            .sum();
        if !sum.is_multiple_of(11) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Unique Taxpayer Reference.
    ///
    /// Ten digits. The first is a check character drawn from `21987654321`,
    /// chosen by the weighted sum of the remaining nine; the reference indexes
    /// that string by the remainder, so a remainder of ten yields `2`.
    pub(in crate::id) fn validate_utr(s: &str) -> Result<()> {
        let s = compact(s, &[]);
        let s = s.strip_prefix('K').unwrap_or(&s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 10)?;
        const WEIGHTS: [u32; 9] = [6, 7, 8, 9, 10, 5, 4, 3, 2];
        const TABLE: [u8; 11] = [2, 1, 9, 8, 7, 6, 5, 4, 3, 2, 1];
        let sum: u32 = WEIGHTS
            .iter()
            .zip(&digits[1..])
            .map(|(w, &d)| w * u32::from(d))
            .sum();
        if digits[0] != TABLE[(sum % 11) as usize] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a United Kingdom VAT registration number.
    ///
    /// Three shapes are accepted: a five-character government or health
    /// authority number, an eleven-character one, and the standard nine or
    /// twelve digit number. The standard number's check is a weighted sum
    /// modulo 97, and the permitted remainders depend on whether the number
    /// belongs to a branch trader.
    pub(in crate::id) fn validate_vat(s: &str) -> Result<()> {
        let s = compact(s, &['-', '.']);
        let s = s
            .strip_prefix("GB")
            .or_else(|| s.strip_prefix("XI"))
            .unwrap_or(&s);
        match s.len() {
            5 => {
                let body = s[2..].parse::<u32>().map_err(|_| Error::InvalidPrefix)?;
                let ok =
                    (s.starts_with("GD") && body < 500) || (s.starts_with("HA") && body >= 500);
                if !ok {
                    return Err(Error::OutOfRange);
                }
                Ok(())
            }
            11 if s.starts_with("GD8888") || s.starts_with("HA8888") => {
                let mid = s[6..9].parse::<u32>().map_err(|_| Error::InvalidPrefix)?;
                let tail = s[9..11].parse::<u32>().map_err(|_| Error::InvalidPrefix)?;
                let ok = (s.starts_with("GD") && mid < 500) || (s.starts_with("HA") && mid >= 500);
                if !ok {
                    return Err(Error::OutOfRange);
                }
                if mid % 97 != tail {
                    return Err(Error::BadChecksum);
                }
                Ok(())
            }
            9 | 12 => {
                let digits = checksum::digits_only(s)?;
                // A twelve-digit number is a branch trader: only the first nine
                // digits carry the check.
                let remainder = vat_checksum(&digits[..9])?;
                let first: u32 = s[..3].parse().map_err(|_| Error::InvalidPrefix)?;
                if first >= 100 {
                    if !matches!(remainder, 0 | 42 | 55) {
                        return Err(Error::BadChecksum);
                    }
                } else if remainder != 0 {
                    return Err(Error::BadChecksum);
                }
                Ok(())
            }
            _ => Err(Error::WrongLength {
                expected: 9,
                found: s.chars().count(),
            }),
        }
    }

    /// The VAT checksum over the first nine digits, modulo 97.
    fn vat_checksum(digits: &[u8]) -> Result<u32> {
        const WEIGHTS: [u32; 9] = [8, 7, 6, 5, 4, 3, 2, 10, 1];
        if digits.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits.len(),
            });
        }
        Ok(WEIGHTS
            .iter()
            .zip(digits)
            .map(|(w, &d)| w * u32::from(d))
            .sum::<u32>()
            % 97)
    }

    /// Validates a SEDOL, the seven-character London Stock Exchange security
    /// identifier.
    ///
    /// Six characters from the digit and consonant alphabet followed by a
    /// check digit. A number whose first character is a digit must be fully
    /// numeric: the reference rejects a mixed old-style number.
    pub(in crate::id) fn validate_sedol(s: &str) -> Result<()> {
        // The reference alphabet is `0123456789 BCD FGH JKLMN PQRST VWXYZ`,
        // where each space is a placeholder for a vowel. That makes the index of
        // a character exactly its base-36 value, so a digit or consonant is
        // scored base-36 and a vowel is rejected as not in the alphabet.
        const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";
        let s = compact(s, &[]);
        checksum::expect_len_str(&s, 7)?;
        let bytes = s.as_bytes();
        let mut values = [0u8; 7];
        for (i, &c) in bytes.iter().enumerate() {
            if matches!(c, b'A' | b'E' | b'I' | b'O' | b'U') {
                return Err(Error::InvalidCharacter(char::from(c)));
            }
            let Some(pos) = ALPHABET.iter().position(|&a| a == c) else {
                return Err(Error::InvalidCharacter(char::from(c)));
            };
            values[i] = pos as u8;
        }
        if bytes[0].is_ascii_digit() && !bytes.iter().all(u8::is_ascii_digit) {
            return Err(Error::InvalidPrefix);
        }
        const WEIGHTS: [u32; 6] = [1, 3, 1, 7, 3, 9];
        let sum: u32 = WEIGHTS
            .iter()
            .zip(&values[..6])
            .map(|(w, &v)| w * u32::from(v))
            .sum();
        if (10 - sum % 10) % 10 != u32::from(values[6]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn nhs_examples() {
        assert!(united_kingdom::validate_nhs("943-476-5870").is_ok());
        assert_eq!(
            united_kingdom::validate_nhs("9434765871"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn utr_examples() {
        assert!(united_kingdom::validate_utr("1955839661").is_ok());
        assert_eq!(
            united_kingdom::validate_utr("2955839661"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn vat_examples() {
        for value in ["GB 980 7806 84", "980780684", "GD001", "HA501"] {
            assert!(united_kingdom::validate_vat(value).is_ok(), "{value}");
        }
        assert_eq!(
            united_kingdom::validate_vat("802311781"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn sedol_examples() {
        assert!(united_kingdom::validate_sedol("B15KXQ8").is_ok());
        assert_eq!(
            united_kingdom::validate_sedol("B15KXQ7"),
            Err(Error::BadChecksum)
        );
        // A vowel is not in the SEDOL alphabet.
        assert_eq!(
            united_kingdom::validate_sedol("B15KXQA"),
            Err(Error::InvalidCharacter('A'))
        );
    }
}
