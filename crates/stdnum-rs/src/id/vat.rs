//! Further European VAT and business identifiers: Cyprus, Malta, Greece,
//! Finland, Sweden and Czechia.
//!
//! Each is a weighted or tabular check over a fixed-length number. Where a VAT
//! number is a company number plus a suffix (Sweden) or can also be a personal
//! code (Czechia), the check is delegated to the identifier that already
//! implements it, so there is one implementation of each algorithm.

use crate::{Error, Result, checksum};

/// Cyprus: the VAT registration number.
pub(super) mod cyprus {
    use super::*;

    /// Validates a Cypriot VAT number.
    ///
    /// Nine characters: eight digits and a final letter. The letter is chosen
    /// by summing a table value for each digit in an even position and the
    /// digit itself in an odd one, modulo twenty-six. A number beginning `12`
    /// is not issued.
    pub(in crate::id) fn validate_vat(s: &str) -> Result<()> {
        let s = s.strip_prefix("CY").unwrap_or(s);
        if s.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: s.len(),
            });
        }
        if s.starts_with("12") {
            return Err(Error::OutOfRange);
        }
        let payload = &s[..8];
        let digits = checksum::digits_only(payload)?;
        // The table is the reference's: each digit in an even position maps to
        // an odd number, the odd positions count as themselves.
        const TABLE: [u32; 10] = [1, 0, 5, 7, 9, 13, 15, 17, 19, 21];
        let sum: u32 = digits
            .iter()
            .enumerate()
            .map(|(i, &d)| {
                if i % 2 == 0 {
                    TABLE[usize::from(d)]
                } else {
                    u32::from(d)
                }
            })
            .sum();
        let expected = (b'A' + (sum % 26) as u8) as char;
        let found = s.chars().nth(8).expect("length was checked");
        if found != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Malta: the VAT registration number.
pub(super) mod malta {
    use super::*;

    /// Validates a Maltese VAT number.
    ///
    /// Eight digits whose weighted sum is a multiple of thirty-seven. A leading
    /// zero is not issued.
    pub(in crate::id) fn validate_vat(s: &str) -> Result<()> {
        let s = s.strip_prefix("MT").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 8)?;
        if digits[0] == 0 {
            return Err(Error::InvalidCharacter('0'));
        }
        const WEIGHTS: [u32; 8] = [3, 4, 6, 7, 8, 9, 10, 1];
        let sum: u32 = digits
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| w * u32::from(d))
            .sum();
        if !sum.is_multiple_of(37) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Greece: ΦΠΑ / ΑΦΜ, the tax registration number.
pub(super) mod greece {
    use super::*;

    /// Validates a Greek VAT number.
    ///
    /// Nine digits, optionally prefixed `EL` or `GR`. The old eight-digit shape
    /// is left-padded with a zero. The check digit is the running value doubled
    /// mod eleven mod ten, which is a doubling chain rather than a weighted sum.
    pub(in crate::id) fn validate_vat(s: &str) -> Result<()> {
        let s = s.replace('/', "");
        let s = s
            .strip_prefix("EL")
            .or_else(|| s.strip_prefix("GR"))
            .unwrap_or(&s);
        let s = if s.len() == 8 {
            format!("0{s}")
        } else {
            s.to_string()
        };
        let digits = checksum::digits_only(&s)?;
        checksum::expect_len(&digits, 9)?;
        let expected = double_chain_check_digit(&digits[..8]);
        if digits[8] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The Greek check digit: fold the digits into one number by doubling and
    /// adding, then double once more and reduce.
    fn double_chain_check_digit(digits: &[u8]) -> u8 {
        let mut value = 0u32;
        for &d in digits {
            value = value * 2 + u32::from(d);
        }
        (value * 2 % 11 % 10) as u8
    }
}

/// Finland: ALV number and the Y-tunnus business identifier.
pub(super) mod finland {
    use super::*;

    /// Validates a Finnish VAT number, which is the same number as the
    /// Y-tunnus.
    ///
    /// Eight digits whose weighted sum is a multiple of eleven.
    pub(in crate::id) fn validate_alv(s: &str) -> Result<()> {
        let s = s.strip_prefix("FI").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 8)?;
        const WEIGHTS: [u32; 8] = [7, 9, 10, 5, 8, 4, 2, 1];
        let sum: u32 = digits
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| w * u32::from(d))
            .sum();
        if !sum.is_multiple_of(11) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Sweden: the VAT number, and the organisation number it is built on.
pub(super) mod sweden {
    use super::*;

    /// Validates a Swedish VAT number.
    ///
    /// Twelve digits ending `01`; the first ten are the organisation number and
    /// must pass its Luhn check.
    pub(in crate::id) fn validate_vat(s: &str) -> Result<()> {
        let s = s.strip_prefix("SE").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 12)?;
        if digits[10] != 0 || digits[11] != 1 {
            return Err(Error::InvalidCharacter('?'));
        }
        validate_org_number_digits(&digits[..10])
    }

    /// Validates a Swedish organisation number.
    ///
    /// Ten digits whose last is a Luhn check digit.
    pub(in crate::id) fn validate_org_number(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 10)?;
        validate_org_number_digits(&digits)
    }

    /// The shared Luhn check over a ten-digit organisation number.
    fn validate_org_number_digits(digits: &[u8]) -> Result<()> {
        if !checksum::luhn_matches(&digits[..9], digits[9]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Czechia: DIČ, the VAT number.
pub(super) mod czechia {
    use super::*;

    /// Validates a Czech VAT number.
    ///
    /// Three shapes: an eight-digit legal entity with its own check digit, a
    /// nine-digit number beginning `6` for a person without a birth number, or
    /// a nine- or ten-digit birth number, which is checked as one. The
    /// reference accepts the `CZ` prefix.
    pub(in crate::id) fn validate_dic(s: &str) -> Result<()> {
        let s = s.replace('/', "");
        let s = s.strip_prefix("CZ").unwrap_or(&s).to_string();
        let digits = checksum::digits_only(&s)?;
        match digits.len() {
            8 => {
                // Legal entities. A number beginning nine is not issued.
                if digits[0] == 9 {
                    return Err(Error::OutOfRange);
                }
                let check = legal_check_digit(&digits[..7]);
                if digits[7] != check {
                    return Err(Error::BadChecksum);
                }
            }
            9 if digits[0] == 6 => {
                // A person without a birth number. The first digit is skipped.
                let check = special_check_digit(&digits[1..8]);
                if digits[8] != check {
                    return Err(Error::BadChecksum);
                }
            }
            9 | 10 => return crate::id::europe_ext::czechoslovakia::validate_rc(&s),
            other => {
                return Err(Error::WrongLength {
                    expected: 8,
                    found: other,
                });
            }
        }
        Ok(())
    }

    /// The check digit for an eight-digit legal entity, over the first seven
    /// digits.
    fn legal_check_digit(digits: &[u8]) -> u8 {
        let sum: i64 = digits
            .iter()
            .enumerate()
            .map(|(i, &d)| (8 - i as i64) * i64::from(d))
            .sum();
        let check = (11 - sum % 11) % 11;
        // A remainder of zero becomes one rather than zero.
        ((if check == 0 { 1 } else { check }) % 10) as u8
    }

    /// The check digit for the nine-digit special case, over the middle seven
    /// digits.
    fn special_check_digit(digits: &[u8]) -> u8 {
        let sum: i64 = digits
            .iter()
            .enumerate()
            .map(|(i, &d)| (8 - i as i64) * i64::from(d))
            .sum();
        // Signed throughout: the inner subtraction goes negative for a
        // remainder above eight, and an unsigned type would panic there.
        ((8 - (10 - sum % 11) % 11) % 10) as u8
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize;

    #[test]
    fn cypriot_example() {
        assert!(cyprus::validate_vat(&normalize("CY-10259033P")).is_ok());
        assert_eq!(
            cyprus::validate_vat(&normalize("CY-10259033Z")),
            Err(Error::BadChecksum)
        );
        assert!(
            cyprus::validate_vat("CY12059033P").is_err(),
            "a number beginning 12 is not issued"
        );
    }

    #[test]
    fn maltese_example() {
        assert!(malta::validate_vat(&normalize("MT 1167-9112")).is_ok());
        assert_eq!(
            malta::validate_vat(&normalize("1167-9113")),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn greek_example_and_the_old_eight_digit_shape() {
        assert!(greece::validate_vat(&normalize("EL 094259216")).is_ok());
        assert_eq!(
            greece::validate_vat(&normalize("EL 123456781")),
            Err(Error::BadChecksum)
        );
        // The old shape has eight digits and is padded with a zero, so the
        // compact form is the same nine digits.
        assert!(greece::validate_vat(&normalize("GR 94259216")).is_ok());
    }

    #[test]
    fn finnish_example() {
        assert!(finland::validate_alv(&normalize("FI 20774740")).is_ok());
        assert_eq!(
            finland::validate_alv(&normalize("FI 20774741")),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn swedish_vat_and_organisation_number() {
        assert!(sweden::validate_vat(&normalize("SE 123456789701")).is_ok());
        assert_eq!(
            sweden::validate_vat("123456789101"),
            Err(Error::BadChecksum)
        );
        // The organisation number is the VAT number without the `01`.
        assert!(sweden::validate_org_number("1234567897").is_ok());
    }

    #[test]
    fn czech_vat_covers_all_three_shapes() {
        // A legal entity, a person without a birth number, and a birth number.
        assert!(czechia::validate_dic(&normalize("CZ 25123891")).is_ok());
        assert!(czechia::validate_dic("7103192745").is_ok());
        assert!(czechia::validate_dic("640903926").is_ok());
    }
}
