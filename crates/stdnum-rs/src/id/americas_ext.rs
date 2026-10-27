//! The identifiers `python-stdnum` covers that the crate reached past without
//! finishing: Iceland, Ecuador, Guatemala, Costa Rica, the Dominican Republic,
//! Guinea, Cuba and Mauritius.
//!
//! Each is a shape and check-digit test. The reference defines no registry
//! lookup for any of them, so a number that passes may still be unissued.
//!
//! Every one of these cleans the raw input itself, because the reference's
//! `compact` removes a different separator set per country. Taking the crate's
//! global normalisation would silently accept input the reference rejects.

use crate::{Error, Result, checksum};

/// Removes the given separators, strips whitespace and upper-cases.
fn compact(input: &str, separators: &[char]) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !separators.contains(c))
        .flat_map(char::to_uppercase)
        .collect()
}

/// Whether `day`/`month`/`year` is a real calendar date.
///
/// The reference calls `datetime.date(...)`, so the leap-year rule must match
/// Python's rather than accept 31 February.
fn is_real_date(year: i32, month: u32, day: u32) -> bool {
    if !(1..=12).contains(&month) || day == 0 {
        return false;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let max_day = match month {
        2 => {
            if leap {
                29
            } else {
                28
            }
        }
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    };
    day <= max_day
}

/// Iceland: kennitala, the personal and organisation identity code.
pub(super) mod iceland {
    use super::*;

    /// Validates an Icelandic kennitala.
    ///
    /// Ten digits: `DDMMYY-RRCC`, where the last digit is a century marker
    /// (`9` for 1900-1999, `0` for 2000 and later) and the digit before it is
    /// the check digit. An organisation's kennitala carries its registration
    /// date with 40 added to the day.
    pub(in crate::id) fn validate_kennitala(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        // The reference's pattern fixes the shape before the date is read: the
        // day tens digit is 0-7, the month tens digit 0-1, and the century
        // marker is exactly 0 or 9. Anything else is a format error, not a
        // bad date, so it is rejected here rather than falling through to the
        // calendar check.
        if digits[0] > 7 || digits[2] > 1 || (digits[9] != 0 && digits[9] != 9) {
            return Err(Error::OutOfRange);
        }
        let day = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let year = i32::from(digits[4]) * 10 + i32::from(digits[5]);
        let year = if digits[9] == 9 {
            year + 1900
        } else {
            year + 2000
        };
        let day = if day >= 40 { day - 40 } else { day };
        if !is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        let weights = [3, 2, 7, 6, 5, 4, 3, 2, 1, 0];
        let sum: u32 = digits
            .iter()
            .zip(weights)
            .map(|(&n, w)| u32::from(n) * w)
            .sum();
        if !sum.is_multiple_of(11) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates an Icelandic VSK number, the VAT registration.
    ///
    /// Five or six digits with no check digit; an `IS` prefix is allowed.
    pub(in crate::id) fn validate_vsk(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let number = number.strip_prefix("IS").unwrap_or(&number);
        let digits = checksum::digits_only(number)?;
        if digits.len() != 5 && digits.len() != 6 {
            return Err(Error::WrongLength {
                expected: 5,
                found: digits.len(),
            });
        }
        Ok(())
    }
}

/// Ecuador: CI personal identity code and RUC taxpayer number.
pub(super) mod ecuador {
    use super::*;

    /// The CI check: double every second digit from the left, subtract nine
    /// from anything above nine, and the total must be a multiple of ten.
    fn ci_checksum_ok(digits: &[u8]) -> bool {
        let sum: u32 = digits
            .iter()
            .enumerate()
            .map(|(i, &n)| {
                let product = u32::from(n) * if i % 2 == 0 { 2 } else { 1 };
                if product > 9 { product - 9 } else { product }
            })
            .sum();
        sum.is_multiple_of(10)
    }

    /// The province code the first two digits must carry.
    fn province_ok(digits: &[u8]) -> bool {
        let code = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        (1..=24).contains(&code) || code == 30 || code == 50
    }

    /// Validates an Ecuadorian CI, the personal identity code.
    ///
    /// Ten digits: a province code, then six digits, then a check digit.
    pub(in crate::id) fn validate_ci(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        if !province_ok(&digits) {
            return Err(Error::OutOfRange);
        }
        if digits[2] > 6 {
            return Err(Error::OutOfRange);
        }
        if !ci_checksum_ok(&digits) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates an Ecuadorian RUC, the taxpayer number.
    ///
    /// Thirteen digits: the third digit names the taxpayer kind. Below `6` the
    /// number is a CI plus an establishment suffix; `6` is a public body and
    /// `9` a legal entity, each with its own weighted check.
    pub(in crate::id) fn validate_ruc(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 13 {
            return Err(Error::WrongLength {
                expected: 13,
                found: digits.len(),
            });
        }
        if !province_ok(&digits) {
            return Err(Error::OutOfRange);
        }
        let weighted = |slice: &[u8], weights: &[u32]| -> bool {
            let sum: u32 = slice
                .iter()
                .zip(weights)
                .map(|(&n, &w)| u32::from(n) * w)
                .sum();
            sum.is_multiple_of(11)
        };
        let natural = || -> Result<()> {
            if digits[10..].iter().all(|&d| d == 0) {
                return Err(Error::OutOfRange);
            }
            // The reference validates the first ten digits as a CI, so an
            // impossible province or check digit rejects the whole number.
            if digits[2] > 6 {
                return Err(Error::OutOfRange);
            }
            if !ci_checksum_ok(&digits[..10]) {
                return Err(Error::BadChecksum);
            }
            Ok(())
        };
        let public = || -> Result<()> {
            if digits[9..].iter().all(|&d| d == 0) {
                return Err(Error::OutOfRange);
            }
            if !weighted(&digits[..9], &[3, 2, 7, 6, 5, 4, 3, 2, 1]) {
                return Err(Error::BadChecksum);
            }
            Ok(())
        };
        let juridical = || -> Result<()> {
            if digits[10..].iter().all(|&d| d == 0) {
                return Err(Error::OutOfRange);
            }
            if !weighted(&digits[..10], &[4, 3, 2, 7, 6, 5, 4, 3, 2, 1]) {
                return Err(Error::BadChecksum);
            }
            Ok(())
        };
        match digits[2] {
            0..=5 => natural(),
            6 => public().or_else(|_| natural()),
            9 => public().or_else(|_| juridical()),
            _ => Err(Error::OutOfRange),
        }
    }
}

/// Guatemala: NIT.
pub(super) mod guatemala {
    use super::*;

    /// Validates a Guatemalan NIT.
    ///
    /// Up to twelve characters: all but the last are digits, the last is a
    /// digit or `K`. Leading zeros are dropped, which is what the reference's
    /// `lstrip('0')` does and why a short number is still legal.
    pub(in crate::id) fn validate_nit(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let number = number.trim_start_matches('0');
        if number.len() < 2 || number.len() > 12 {
            return Err(Error::WrongLength {
                expected: 12,
                found: number.len(),
            });
        }
        let (body, check) = number.split_at(number.len() - 1);
        if !body.bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::OutOfRange);
        }
        let Some(check) = check.chars().next() else {
            return Err(Error::Empty);
        };
        if !check.is_ascii_digit() && check != 'K' {
            return Err(Error::InvalidCharacter(check));
        }
        // Weighted from the right, starting at 2.
        let sum: u32 = body
            .bytes()
            .rev()
            .enumerate()
            .map(|(i, b)| u32::from(b - b'0') * (i as u32 + 2))
            .sum();
        let remainder = (11 - (sum % 11)) % 11;
        let expected = if remainder == 10 {
            'K'
        } else {
            char::from_digit(remainder, 10).unwrap_or('?')
        };
        if check != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Costa Rica: CPJ, the juridical person number.
pub(super) mod costa_rica {
    use super::*;

    /// Validates a Costa Rican CPJ.
    ///
    /// Ten digits with no check digit: the first is the class of juridical
    /// person and the next three a type code that depends on the class.
    pub(in crate::id) fn validate_cpj(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        let class = digits[0];
        let kind = u32::from(digits[1]) * 100 + u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let ok = match class {
            2 => matches!(kind, 100 | 200 | 300 | 400),
            3 => (2..=14).contains(&kind) || (101..=110).contains(&kind),
            4 => kind == 0,
            5 => kind == 1,
            _ => false,
        };
        if !ok {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Dominican Republic: RNC, the taxpayer registry number.
pub(super) mod dominican_republic {
    use super::*;

    /// RNCs the DGII issues that do not satisfy the check digit.
    ///
    /// The reference carries this list, so a number in it is valid regardless
    /// of the arithmetic. Dropping it would reject numbers the reference
    /// accepts.
    const WHITELIST: &[&str] = &[
        "101581601",
        "101582245",
        "101595422",
        "101595785",
        "10233317",
        "131188691",
        "401007374",
        "501341601",
        "501378067",
        "501620371",
        "501651319",
        "501651823",
        "501651845",
        "501651926",
        "501656006",
        "501658167",
        "501670785",
        "501676936",
        "501680158",
        "504654542",
        "504680029",
        "504681442",
        "505038691",
    ];

    /// Validates a Dominican RNC.
    ///
    /// Nine digits with a mod-11 check, except for the whitelisted numbers.
    pub(in crate::id) fn validate_rnc(s: &str) -> Result<()> {
        let number = compact(s, &['-', ' ']);
        let digits = checksum::digits_only(&number)?;
        if WHITELIST.contains(&number.as_str()) {
            return Ok(());
        }
        if digits.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits.len(),
            });
        }
        let weights = [7, 9, 8, 6, 5, 4, 3, 2];
        let sum: u32 = digits[..8]
            .iter()
            .zip(weights)
            .map(|(&n, w)| u32::from(n) * w)
            .sum();
        let check = (10 - (sum % 11)) % 9 + 1;
        if check != u32::from(digits[8]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Guinea: NIFp.
pub(super) mod guinea {
    use super::*;

    /// Validates a Guinean NIFp.
    ///
    /// Nine digits with a Luhn check.
    pub(in crate::id) fn validate_nifp(s: &str) -> Result<()> {
        let number = compact(s, &['-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits.len(),
            });
        }
        if !checksum::luhn_matches(&digits[..8], digits[8]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Cuba: NI, the identity number.
pub(super) mod cuba {
    use super::*;

    /// Validates a Cuban NI.
    ///
    /// Eleven digits: a date of birth, then a digit that encodes the century,
    /// then a serial whose last digit's parity is the sex. There is no check
    /// digit; the reference checks the date and nothing else.
    pub(in crate::id) fn validate_ni(s: &str) -> Result<()> {
        let number = compact(s, &[' ', '-']);
        let digits = checksum::digits_only(&number)?;
        if digits.len() != 11 {
            return Err(Error::WrongLength {
                expected: 11,
                found: digits.len(),
            });
        }
        let year = i32::from(digits[0]) * 10 + i32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        // The century digit is 9 for the 1800s, 0-5 for the 1900s, else 2000s.
        let year = match digits[6] {
            9 => year + 1800,
            0..=5 => year + 1900,
            _ => year + 2000,
        };
        if !is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Mauritius: NID, the national identifier.
pub(super) mod mauritius {
    use super::*;

    /// The alphabet the check character is drawn from.
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    /// Validates a Mauritian NID.
    ///
    /// Fourteen characters: a leading letter, twelve more, then a check
    /// character over the first thirteen. The date is the day, month and year
    /// at offsets 1, 3 and 5, and the reference reads its century as 2000.
    pub(in crate::id) fn validate_nid(s: &str) -> Result<()> {
        let number = compact(s, &[' ', '-']);
        if number.len() != 14 {
            return Err(Error::WrongLength {
                expected: 14,
                found: number.len(),
            });
        }
        let bytes = number.as_bytes();
        if !bytes[0].is_ascii_uppercase() {
            return Err(Error::InvalidCharacter(bytes[0] as char));
        }
        let mut values = [0u32; 14];
        for (i, &b) in bytes.iter().enumerate() {
            let Some(position) = ALPHABET.iter().position(|&a| a == b) else {
                return Err(Error::InvalidCharacter(b as char));
            };
            values[i] = position as u32;
        }
        // The check character is over the first thirteen, weighted from 14 down.
        let sum: u32 = values[..13]
            .iter()
            .enumerate()
            .map(|(i, &v)| (14 - i as u32) * v)
            .sum();
        let expected = ALPHABET[((17 - sum % 17) % 17) as usize];
        if bytes[13] != expected {
            return Err(Error::BadChecksum);
        }
        let day = values[1] * 10 + values[2];
        let month = values[3] * 10 + values[4];
        let year = 2000 + i32::from((values[5] * 10 + values[6]) as u16);
        if !is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icelandic_kennitala_examples() {
        // The two values the reference publishes, one of each kind.
        assert!(iceland::validate_kennitala("450401-3150").is_ok());
        assert!(iceland::validate_kennitala("1201743399").is_ok());
        assert_eq!(
            iceland::validate_kennitala("530575-0299"),
            Err(Error::BadChecksum)
        );
        // A day the calendar does not have.
        assert!(iceland::validate_kennitala("320174-3399").is_err());
    }

    #[test]
    fn icelandic_vsk_examples() {
        assert!(iceland::validate_vsk("12345").is_ok());
        assert!(iceland::validate_vsk("IS123456").is_ok());
        assert!(iceland::validate_vsk("1234").is_err());
    }

    #[test]
    fn ecuadorian_examples() {
        assert!(ecuador::validate_ci("1701234567").is_ok());
        assert!(ecuador::validate_ruc("1792060346001").is_ok());
        assert_eq!(
            ecuador::validate_ruc("1763154690001"),
            Err(Error::BadChecksum)
        );
        // A province code outside the country's numbering.
        assert!(ecuador::validate_ci("9912345678").is_err());
    }

    #[test]
    fn guatemalan_nit_examples() {
        assert!(guatemala::validate_nit("12345679").is_ok());
        // A leading zero is dropped before the check, so the two spellings of
        // the same number are both accepted.
        assert!(guatemala::validate_nit("012345679").is_ok());
        assert_eq!(guatemala::validate_nit("12345670"), Err(Error::BadChecksum));
    }

    #[test]
    fn costa_rican_cpj_examples() {
        assert!(costa_rica::validate_cpj("3-101-999999").is_ok());
        assert!(costa_rica::validate_cpj("3-534-123559").is_err());
    }

    #[test]
    fn dominican_rnc_examples() {
        assert!(dominican_republic::validate_rnc("1-01-85004-3").is_ok());
        assert_eq!(
            dominican_republic::validate_rnc("101850042"),
            Err(Error::BadChecksum)
        );
        // The whitelist holds numbers the check digit would reject.
        assert!(dominican_republic::validate_rnc("101581601").is_ok());
    }

    #[test]
    fn guinean_nifp_examples() {
        assert!(guinea::validate_nifp("123456782").is_ok());
        assert_eq!(guinea::validate_nifp("123456789"), Err(Error::BadChecksum));
    }

    #[test]
    fn cuban_ni_examples() {
        assert!(cuba::validate_ni("85010112345").is_ok());
        // Month 13 is impossible.
        assert!(cuba::validate_ni("85130112345").is_err());
    }

    #[test]
    fn mauritian_nid_examples() {
        assert!(mauritius::validate_nid("A1501920000003").is_ok());
        assert!(mauritius::validate_nid("B0203901234568").is_ok());
        assert_eq!(
            mauritius::validate_nid("A1501920000004"),
            Err(Error::BadChecksum)
        );
    }
}
