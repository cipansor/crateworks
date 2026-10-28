//! Latin American identifiers.

use crate::{Error, Result, checksum};

/// Removes the given separators, strips whitespace and upper-cases.
///
/// This is the reference's `clean()` plus `upper()`. The separator set differs
/// per identifier, which is why it is a parameter.
fn compact(input: &str, separators: &[char]) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !separators.contains(c))
        .flat_map(char::to_uppercase)
        .collect()
}

/// Chile: RUT (Rol Único Tributario).
pub(super) mod chile {
    use super::*;

    /// Validates a Chilean RUT.
    ///
    /// The number is eight or nine digits followed by a check character that
    /// is a digit or the letter K. The check is a weighted sum modulo eleven,
    /// with the weights cycling 2, 3, 4, 5, 6, 7 from the right.
    pub(in crate::id) fn validate_rut(s: &str) -> Result<()> {
        let s = s.strip_prefix("CL").unwrap_or(s);
        let Some(check) = s.chars().next_back() else {
            return Err(Error::Empty);
        };
        if !check.is_ascii() {
            return Err(Error::InvalidCharacter(check));
        }
        let check = check.to_ascii_uppercase() as u8;
        let body = &s[..s.len() - 1];
        let digits = checksum::digits_only(body)?;
        // The reference bounds the whole number, check character included, so
        // the body is seven or eight digits.
        if digits.len() != 7 && digits.len() != 8 {
            return Err(Error::WrongLength {
                expected: 8,
                found: digits.len() + 1,
            });
        }
        // Weight of the digit at distance `i` from the right: 4 + ((5 - i) mod 6),
        // computed without a negative intermediate so the cycle is explicit.
        let sum: u32 = digits
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &n)| u32::from(n) * (4 + ((11 - (i % 6)) % 6) as u32))
            .sum();
        if check != b"0123456789K"[(sum % 11) as usize] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Colombia: NIT (Número de Identificación Tributaria).
pub(super) mod colombia {
    use super::*;

    /// Validates a Colombian NIT.
    ///
    /// The check digit is the last character; the body before it is weighted
    /// by the first fifteen primes and reduced modulo eleven against a lookup
    /// string.
    pub(in crate::id) fn validate_nit(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        if !(8..=16).contains(&digits.len()) {
            return Err(Error::WrongLength {
                expected: 8,
                found: digits.len(),
            });
        }
        const WEIGHTS: [u32; 15] = [3, 7, 13, 17, 19, 23, 29, 37, 41, 43, 47, 53, 59, 67, 71];
        let body = &digits[..digits.len() - 1];
        let sum: u32 = body
            .iter()
            .rev()
            .zip(WEIGHTS)
            .map(|(&n, w)| u32::from(n) * w)
            .sum();
        let expected = b"01987654321"[(sum % 11) as usize] - b'0';
        if digits[digits.len() - 1] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Peru: RUC (Registro Único de Contribuyentes).
pub(super) mod peru {
    use super::*;

    /// Validates a Peruvian RUC.
    ///
    /// Eleven digits whose first two identify the kind of taxpayer, and whose
    /// last is a check digit.
    pub(in crate::id) fn validate_ruc(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        let prefix = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        if !matches!(prefix, 10 | 15 | 17 | 20) {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 10] = [5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
        let sum: u32 = digits
            .iter()
            .zip(WEIGHTS)
            .map(|(&n, w)| u32::from(n) * w)
            .sum();
        let expected = ((11 - sum % 11) % 10) as u8;
        if digits[10] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Peruvian CUI, the identity number on the DNI.
    ///
    /// Eight digits with an optional ninth check character, which may be a
    /// digit or one of the letters the reference's two lookup strings provide.
    pub(in crate::id) fn validate_cui(s: &str) -> Result<()> {
        let cleaned = crate::id::clean(s, &[' ', '-']);
        let chars: Vec<char> = cleaned.chars().collect();
        if !matches!(chars.len(), 8 | 9) {
            return Err(Error::WrongLength {
                expected: 8,
                found: chars.len(),
            });
        }
        let mut digits = Vec::with_capacity(8);
        for &c in &chars[..8] {
            digits.push(
                c.to_digit(10)
                    .map(|d| d as u8)
                    .ok_or(Error::InvalidCharacter(c))?,
            );
        }
        if chars.len() == 9 {
            let extra = chars[8];
            const WEIGHTS: [u32; 8] = [3, 2, 7, 6, 5, 4, 3, 2];
            let sum: u32 = digits
                .iter()
                .zip(WEIGHTS)
                .map(|(&d, w)| u32::from(d) * w)
                .sum();
            let index = (sum % 11) as usize;
            const DIGITS: &[u8] = b"65432110987";
            const LETTERS: &[u8] = b"KJIHGFEDCBA";
            if extra != char::from(DIGITS[index]) && extra != char::from(LETTERS[index]) {
                return Err(Error::BadChecksum);
            }
        }
        Ok(())
    }
}

/// Uruguay: RUT (Registro Único Tributario).
pub(super) mod uruguay {
    use super::*;

    /// Validates a Uruguayan RUT.
    ///
    /// Twelve digits: a two-digit issuer, a six-digit body that may not be all
    /// zeroes, the fixed group `001`, and a check digit.
    pub(in crate::id) fn validate_rut(s: &str) -> Result<()> {
        let s = s.strip_prefix("UY").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 12)?;
        let issuer = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        if !(1..=22).contains(&issuer) || digits[2..8].iter().all(|&d| d == 0) {
            return Err(Error::OutOfRange);
        }
        if digits[8] != 0 || digits[9] != 0 || digits[10] != 1 {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 11] = [4, 3, 2, 9, 8, 7, 6, 5, 4, 3, 2];
        let total: u32 = digits
            .iter()
            .zip(WEIGHTS)
            .map(|(&n, w)| u32::from(n) * w)
            .sum();
        let expected = ((11 - total % 11) % 11) as u8;
        if digits[11] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Argentina: CUIT (Clave Única de Identificación Tributaria).
pub(super) mod argentina {
    use super::*;

    /// Validates an Argentine CUIT.
    ///
    /// Eleven digits whose first two are one of a fixed set of taxpayer codes
    /// and whose last is a check digit.
    pub(in crate::id) fn validate_cuit(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        let prefix = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        if !matches!(prefix, 20 | 23 | 24 | 27 | 30 | 33 | 34 | 50 | 51 | 55) {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 10] = [5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
        let sum: u32 = digits[..10]
            .iter()
            .zip(WEIGHTS)
            .map(|(&n, w)| u32::from(n) * w)
            .sum();
        // The reference indexes a lookup string with `11 - sum % 11`, which is
        // eleven when the remainder is zero and falls off the end of that
        // string. The reference rejects such a number, so this does too.
        let index = 11 - (sum % 11);
        let expected = match b"012345678990".get(index as usize) {
            Some(&b) => b - b'0',
            None => return Err(Error::BadChecksum),
        };
        if digits[10] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates an Argentine DNI, the national identity number.
    ///
    /// Seven or eight digits, with no check digit: the reference checks only
    /// the length and the character class.
    pub(in crate::id) fn validate_dni(s: &str) -> Result<()> {
        let cleaned = crate::id::clean(s, &[' ', '.']);
        let digits = checksum::digits_only(&cleaned)?;
        if !matches!(digits.len(), 7 | 8) {
            return Err(Error::WrongLength {
                expected: 8,
                found: digits.len(),
            });
        }
        Ok(())
    }

    /// Validates an Argentine CBU, the bank account number (Clave Bancaria
    /// Uniforme).
    ///
    /// Twenty-two digits in two blocks, each ending in a check digit. The bank
    /// and branch block is seven digits plus a check, then a single-digit
    /// separator, then the thirteen-digit account plus a check. Each check is a
    /// weighted sum with the weights 3, 1, 7, 9 cycling from the right.
    pub(in crate::id) fn validate_cbu(s: &str) -> Result<()> {
        let cleaned = compact(s, &[' ', '-']);
        let digits = checksum::digits_only(&cleaned)?;
        checksum::expect_len(&digits, 22)?;
        if cbu_check_digit(&digits[..7]) != digits[7] {
            return Err(Error::BadChecksum);
        }
        if cbu_check_digit(&digits[8..21]) != digits[21] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The CBU check digit over `payload`: the digits are weighted 3, 1, 7, 9
    /// from the right and the total subtracted from ten, modulo ten.
    fn cbu_check_digit(payload: &[u8]) -> u8 {
        const WEIGHTS: [u32; 4] = [3, 1, 7, 9];
        let total: u32 = payload
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &d)| u32::from(d) * WEIGHTS[i % 4])
            .sum();
        ((10 - total % 10) % 10) as u8
    }
}

/// Venezuela: RIF (Registro de Información Fiscal).
pub(super) mod venezuela {
    use super::*;

    /// Validates a Venezuelan RIF.
    ///
    /// A taxpayer-type letter (V, E, J, P or G) followed by eight digits and a
    /// check digit. The letter contributes a fixed value to the sum.
    pub(in crate::id) fn validate_rif(s: &str) -> Result<()> {
        let mut chars = s.chars();
        let Some(kind) = chars.next() else {
            return Err(Error::Empty);
        };
        let base = match kind {
            'V' => 4,
            'E' => 8,
            'J' => 12,
            'P' => 16,
            'G' => 20,
            _ => return Err(Error::InvalidCharacter(kind)),
        };
        let rest: String = chars.collect();
        let digits = checksum::digits_only(&rest)?;
        checksum::expect_len(&digits, 9)?;
        const WEIGHTS: [u32; 8] = [3, 2, 7, 6, 5, 4, 3, 2];
        let total: u32 = base
            + digits[..8]
                .iter()
                .zip(WEIGHTS)
                .map(|(&n, w)| u32::from(n) * w)
                .sum::<u32>();
        let expected = b"00987654321"[(total % 11) as usize] - b'0';
        if digits[8] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Paraguay: RUC, the taxpayer registry number.
pub(super) mod paraguay {
    use super::*;

    /// Validates a Paraguayan RUC.
    ///
    /// Up to nine digits whose last is a weighted sum of the preceding ones,
    /// taken with weights increasing from two at the right, negated modulo
    /// eleven and then modulo ten.
    pub(in crate::id) fn validate_ruc(s: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(s, &['-']))?;
        if digits.is_empty() {
            return Err(Error::Empty);
        }
        if digits.len() > 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits.len(),
            });
        }
        let sum: u32 = digits[..digits.len() - 1]
            .iter()
            .rev()
            .enumerate()
            .map(|(i, &d)| (i as u32 + 2) * u32::from(d))
            .sum();
        let expected = ((-(sum as i64)).rem_euclid(11)) % 10;
        if u32::from(digits[digits.len() - 1]) != expected as u32 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// El Salvador: NIT, the tax identification number.
pub(super) mod el_salvador {
    use super::*;

    /// Validates an El Salvadoran NIT.
    ///
    /// Fourteen digits. Numbers whose last three digits are at most `100` use
    /// the old weighting and take the sum modulo eleven then ten; newer numbers
    /// use a different weighting and negate the sum before reducing. The first
    /// digit is never 2 to 8.
    pub(in crate::id) fn validate_nit(s: &str) -> Result<()> {
        let s = compact(s, &['-']);
        let s = s.strip_prefix("SV").unwrap_or(&s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 14)?;
        if !matches!(digits[0], 0 | 1 | 9) {
            return Err(Error::OutOfRange);
        }
        // The reference chooses the weighting from the *string* comparison of
        // characters ten to thirteen, so the digits are rebuilt as a string
        // rather than compared numerically.
        let tail: String = digits[10..13]
            .iter()
            .map(|d| char::from(b'0' + d))
            .collect();
        let old = tail.as_str() <= "100";
        const OLD: [u32; 14] = [14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 0];
        const NEW: [u32; 14] = [2, 7, 6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2, 0];
        let weights = if old { OLD } else { NEW };
        let total: u32 = digits
            .iter()
            .zip(weights)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        let expected = if old {
            (total % 11) % 10
        } else {
            (-(total as i64)).rem_euclid(11) as u32 % 10
        };
        if u32::from(digits[13]) != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chilean_examples() {
        for value in ["760864285", "125319092"] {
            assert!(chile::validate_rut(value).is_ok(), "{value}");
        }
        assert_eq!(chile::validate_rut("125319093"), Err(Error::BadChecksum));
        assert!(chile::validate_rut("76086A285").is_err());
    }

    #[test]
    fn colombian_examples() {
        for value in ["2131234321", "8909039388"] {
            assert!(colombia::validate_nit(value).is_ok(), "{value}");
        }
        assert_eq!(
            colombia::validate_nit("2131234325"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn peruvian_examples() {
        assert!(peru::validate_ruc("20512333797").is_ok());
        assert_eq!(peru::validate_ruc("20512333798"), Err(Error::BadChecksum));
        // A prefix that is not a taxpayer code is rejected before the check.
        assert_eq!(peru::validate_ruc("30512333797"), Err(Error::OutOfRange));
    }

    #[test]
    fn uruguayan_examples() {
        for value in ["211003420017", "211406340011"] {
            assert!(uruguay::validate_rut(value).is_ok(), "{value}");
        }
        assert_eq!(
            uruguay::validate_rut("211003420010"),
            Err(Error::BadChecksum)
        );
        // A body of all zeroes is not a RUT.
        assert_eq!(
            uruguay::validate_rut("210000000014"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn argentine_examples() {
        assert!(argentina::validate_cuit("20055361682").is_ok());
        assert_eq!(
            argentina::validate_cuit("20055361683"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            argentina::validate_cuit("99055361682"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn venezuelan_examples() {
        assert!(venezuela::validate_rif("V114702834").is_ok());
        assert_eq!(
            venezuela::validate_rif("V114702833"),
            Err(Error::BadChecksum)
        );
        assert!(venezuela::validate_rif("X114702834").is_err());
    }

    #[test]
    fn paraguayan_examples() {
        for value in ["80028061-0", "2660-3", "9991603"] {
            assert!(paraguay::validate_ruc(value).is_ok(), "{value}");
        }
        assert_eq!(paraguay::validate_ruc("800532492"), Err(Error::BadChecksum));
        assert_eq!(
            paraguay::validate_ruc("80123456789"),
            Err(Error::WrongLength {
                expected: 9,
                found: 11,
            })
        );
    }

    #[test]
    fn salvadoran_examples() {
        for value in ["0614-050707-104-8", "SV 0614-050707-104-8"] {
            assert!(el_salvador::validate_nit(value).is_ok(), "{value}");
        }
        assert_eq!(
            el_salvador::validate_nit("0614-050707-104-0"),
            Err(Error::BadChecksum)
        );
    }
}
