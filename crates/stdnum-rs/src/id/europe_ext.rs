//! Further European and Near Eastern identifiers.
//!
//! These are the identifiers that are neither in the first continental
//! European batch (`europe.rs`) nor in the Nordic set (`nordics.rs`).

use crate::{Error, Result, checksum};

/// Ireland: VAT number, and the PPS number built on the same check.
pub(super) mod ireland {
    use super::*;

    /// The letters a VAT check character is drawn from.
    const ALPHABET: &[u8] = b"WABCDEFGHIJKLMNOPQRSTUV";

    /// The check character for a VAT or PPS payload.
    ///
    /// The digits are left-padded with zeroes to seven characters before
    /// weighting, which is what the reference does; the padding matters for the
    /// old-shape numbers whose payload is only six digits. `extra` is the
    /// trailing letter, when there is one, and contributes a fixed weight of
    /// nine.
    fn vat_check_char(digits: &[u8], extra: Option<u8>) -> Result<u8> {
        let mut payload = vec![0u8; 7usize.saturating_sub(digits.len())];
        payload.extend_from_slice(digits);
        let mut sum: u32 = payload
            .iter()
            .enumerate()
            .map(|(i, &d)| (8 - i as u32) * u32::from(d))
            .sum();
        if let Some(c) = extra {
            let position = ALPHABET
                .iter()
                .position(|&a| a == c)
                .ok_or(Error::InvalidCharacter(char::from(c)))?;
            sum += 9 * position as u32;
        }
        Ok(ALPHABET[(sum % 23) as usize])
    }

    /// Validates an Irish VAT number.
    ///
    /// Two shapes: seven digits followed by one or two letters, or an older
    /// shape whose second character is a letter or symbol.
    pub(in crate::id) fn validate_vat(s: &str) -> Result<()> {
        let s = s.strip_prefix("IE").unwrap_or(s);
        let bytes = s.as_bytes();
        if !(8..=9).contains(&bytes.len()) {
            return Err(Error::WrongLength {
                expected: 8,
                found: bytes.len(),
            });
        }
        if !bytes[0].is_ascii_digit() || !bytes[2..7].iter().all(u8::is_ascii_digit) {
            return Err(Error::InvalidCharacter(char::from(bytes[0])));
        }
        if !bytes[7..].iter().all(|b| ALPHABET.contains(b)) {
            return Err(Error::InvalidCharacter(char::from(bytes[7])));
        }
        if bytes[..7].iter().all(u8::is_ascii_digit) {
            // New shape: the check is over the seven digits and any second
            // letter, which sits after the check character.
            let extra = if bytes.len() == 9 {
                Some(bytes[8])
            } else {
                None
            };
            let digits = checksum::digits_only(&s[..7])?;
            if bytes[7] != vat_check_char(&digits, extra)? {
                return Err(Error::BadChecksum);
            }
        } else if matches!(bytes[1], b'A'..=b'Z' | b'+' | b'*') {
            // Old shape: the first digit moves to the end of the six-digit
            // payload, which the check then left-pads to seven.
            let payload = format!("{}{}", &s[2..7], char::from(bytes[0]));
            let digits = checksum::digits_only(&payload)?;
            if bytes[7] != vat_check_char(&digits, None)? {
                return Err(Error::BadChecksum);
            }
        } else {
            return Err(Error::InvalidCharacter(char::from(bytes[1])));
        }
        Ok(())
    }

    /// Validates an Irish PPS number.
    ///
    /// Seven digits, a check letter, and optionally a second letter. In the
    /// 2013 shape that second letter is part of the check; in the older shape
    /// it is ignored.
    pub(in crate::id) fn validate_pps(s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        if !(8..=9).contains(&bytes.len()) {
            return Err(Error::WrongLength {
                expected: 8,
                found: bytes.len(),
            });
        }
        if !bytes[..7].iter().all(u8::is_ascii_digit)
            || !(b'A'..=b'W').contains(&bytes[7])
            || bytes.get(8).is_some_and(|b| !b"ABHWTX".contains(b))
        {
            return Err(Error::InvalidCharacter(char::from(bytes[0])));
        }
        let extra = match bytes.get(8) {
            Some(&c) if matches!(c, b'A' | b'B' | b'H') => Some(c),
            _ => None,
        };
        let digits = checksum::digits_only(&s[..7])?;
        if bytes[7] != vat_check_char(&digits, extra)? {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Hungary: adószám, the tax number.
pub(super) mod hungary {
    use super::*;

    /// Validates a Hungarian tax number.
    ///
    /// Eight digits whose weighted sum, with weights 9, 7, 3, 1 repeating, is
    /// a multiple of ten.
    pub(in crate::id) fn validate_anum(s: &str) -> Result<()> {
        let s = s.strip_prefix("HU").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 8)?;
        const WEIGHTS: [u32; 8] = [9, 7, 3, 1, 9, 7, 3, 1];
        let sum: u32 = digits
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        if !sum.is_multiple_of(10) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Croatia: OIB (Osobni identifikacijski broj).
pub(super) mod croatia {
    use super::*;

    /// Validates a Croatian OIB.
    ///
    /// Eleven digits with the ISO 7064 Mod 11, 10 check, the same one the
    /// German tax number uses.
    pub(in crate::id) fn validate_oib(s: &str) -> Result<()> {
        let s = s.strip_prefix("HR").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        if !checksum::iso7064_mod_11_10_is_valid(&digits) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Bulgaria: EGN (personal) and PNF (foreigner).
pub(super) mod bulgaria {
    use super::*;

    /// Validates a Bulgarian EGN.
    ///
    /// Ten digits whose first six encode the birth date, with the month offset
    /// by twenty or forty to record the century.
    pub(in crate::id) fn validate_egn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 10)?;
        birth_date(&digits)?;
        const WEIGHTS: [u32; 9] = [2, 4, 8, 5, 10, 9, 7, 3, 6];
        let sum: u32 = digits[..9]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        if digits[9] != (sum % 11 % 10) as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Bulgarian PNF, the number issued to a foreigner.
    pub(in crate::id) fn validate_pnf(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 10)?;
        const WEIGHTS: [u32; 9] = [21, 19, 17, 13, 11, 9, 7, 3, 1];
        let sum: u32 = digits[..9]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        if digits[9] != (sum % 10) as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Checks the birth date an EGN encodes.
    fn birth_date(digits: &[u8]) -> Result<()> {
        let year = 1900 + u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let mut month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let year = if month > 40 {
            month -= 40;
            year + 100
        } else if month > 20 {
            month -= 20;
            year - 100
        } else {
            year
        };
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Czech Republic and Slovakia: RČ (rodné číslo), the birth number.
pub(super) mod czechoslovakia {
    use super::*;

    /// Validates a Czech or Slovak birth number.
    ///
    /// Nine or ten digits. The first six are the birth date, with fifty added
    /// to the month for women and twenty more when the serial overflows. Nine
    /// digit numbers were issued only until 1954.
    pub(in crate::id) fn validate_rc(s: &str) -> Result<()> {
        // The number is written with a slash between the date and the serial.
        let s = s.replace('/', "");
        let digits = checksum::digits_only(&s)?;
        if digits.len() != 9 && digits.len() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        let mut year = 1900 + u32::from(digits[0]) * 10 + u32::from(digits[1]);
        // The whole two-digit month is reduced, so the addition must bind
        // before the modulo.
        let encoded_month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let month = encoded_month % 50 % 20;
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        if digits.len() == 9 {
            if year >= 1980 {
                year -= 100;
            }
            if year > 1953 {
                return Err(Error::OutOfRange);
            }
        } else if year < 1954 {
            year += 100;
        }
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        if digits.len() == 10 {
            let head = digits[..9]
                .iter()
                .fold(0u64, |acc, &d| acc * 10 + u64::from(d));
            if digits[9] != (head % 11 % 10) as u8 {
                return Err(Error::BadChecksum);
            }
        }
        Ok(())
    }
}

/// Estonia: isikukood.
pub(super) mod estonia {
    use super::*;

    /// Validates an Estonian personal code.
    ///
    /// Eleven digits whose first encodes the century and gender.
    pub(in crate::id) fn validate_ik(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        let century = match digits[0] {
            1 | 2 => 1800,
            3 | 4 => 1900,
            5 | 6 => 2000,
            7 | 8 => 2100,
            _ => return Err(Error::OutOfRange),
        };
        let year = century + u32::from(digits[1]) * 10 + u32::from(digits[2]);
        let month = u32::from(digits[3]) * 10 + u32::from(digits[4]);
        let day = u32::from(digits[5]) * 10 + u32::from(digits[6]);
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        // The first pass weighs 1 through 9 repeating; a remainder of ten is
        // retried with weights starting at three.
        let first = weighted(&digits[..10], 1);
        let check = if first == 10 {
            weighted(&digits[..10], 3) % 11
        } else {
            first
        };
        if digits[10] != (check % 10) as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The modular weight sum the Baltic codes share: the weight cycles one
    /// through nine from `start`.
    fn weighted(digits: &[u8], start: u32) -> u32 {
        digits
            .iter()
            .enumerate()
            .map(|(i, &d)| ((i as u32 + start - 1) % 9 + 1) * u32::from(d))
            .sum::<u32>()
            % 11
    }
}

/// Lithuania: asmens kodas.
pub(super) mod lithuania {
    use super::*;

    /// Validates a Lithuanian personal code.
    ///
    /// Eleven digits; the first encodes the century, except that nine marks a
    /// number issued before the current scheme and carries no date.
    pub(in crate::id) fn validate_asmens(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        if digits[0] != 9 {
            let century = match digits[0] {
                1 | 2 => 1800,
                3 | 4 => 1900,
                5 | 6 => 2000,
                7 | 8 => 2100,
                _ => return Err(Error::OutOfRange),
            };
            let year = century + u32::from(digits[1]) * 10 + u32::from(digits[2]);
            let month = u32::from(digits[3]) * 10 + u32::from(digits[4]);
            let day = u32::from(digits[5]) * 10 + u32::from(digits[6]);
            if !checksum::is_real_date(year, month, day) {
                return Err(Error::OutOfRange);
            }
        }
        let first: u32 = digits[..10]
            .iter()
            .enumerate()
            .map(|(i, &d)| ((i as u32 % 9) + 1) * u32::from(d))
            .sum::<u32>()
            % 11;
        let check = if first == 10 {
            digits[..10]
                .iter()
                .enumerate()
                .map(|(i, &d)| (((i as u32 + 2) % 9) + 1) * u32::from(d))
                .sum::<u32>()
                % 11
        } else {
            first
        };
        if digits[10] != (check % 10) as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Luxembourg: TVA, the VAT number.
pub(super) mod luxembourg {
    use super::*;

    /// Validates a Luxembourg VAT number.
    ///
    /// Eight digits whose last two are the first six modulo 89.
    pub(in crate::id) fn validate_tva(s: &str) -> Result<()> {
        let s = s.strip_prefix("LU").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 8)?;
        let head = digits[..6]
            .iter()
            .fold(0u32, |acc, &d| acc * 10 + u32::from(d));
        let expected = head % 89;
        let found = u32::from(digits[6]) * 10 + u32::from(digits[7]);
        if found != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Switzerland: AHV/AVS social security number and UID.
pub(super) mod switzerland {
    use super::*;

    /// Validates a Swiss social security number.
    ///
    /// Thirteen digits beginning `756` with a GS1 check digit, the same one an
    /// EAN uses.
    pub(in crate::id) fn validate_ssn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 13)?;
        if digits[..3] != [7, 5, 6] {
            return Err(Error::OutOfRange);
        }
        if checksum::gs1_check_digit(&digits[..12]) != digits[12] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Swiss business identification number.
    ///
    /// Twelve characters: the prefix `CHE`, eight digits and a check digit.
    pub(in crate::id) fn validate_uid(s: &str) -> Result<()> {
        let s = if s.len() == 9 && s.bytes().all(|b| b.is_ascii_digit()) {
            format!("CHE{s}")
        } else {
            s.to_string()
        };
        if s.len() != 12 {
            return Err(Error::WrongLength {
                expected: 12,
                found: s.len(),
            });
        }
        if !s.starts_with("CHE") {
            return Err(Error::InvalidPrefix);
        }
        let digits = checksum::digits_only(&s[3..])?;
        checksum::expect_len(&digits, 9)?;
        const WEIGHTS: [u32; 8] = [5, 4, 3, 2, 7, 6, 5, 4];
        let sum: u32 = digits[..8]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        if digits[8] != ((11 - sum % 11) % 11) as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Russia: ИНН and ОГРН.
pub(super) mod russia {
    use super::*;

    /// Validates a Russian ИНН.
    ///
    /// Ten digits for an organisation, or twelve for a person, each with its
    /// own weighted check.
    pub(in crate::id) fn validate_inn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        match digits.len() {
            10 => {
                const WEIGHTS: [u32; 9] = [2, 4, 10, 3, 5, 9, 4, 6, 8];
                let sum: u32 = digits[..9]
                    .iter()
                    .zip(WEIGHTS)
                    .map(|(&d, w)| u32::from(d) * w)
                    .sum();
                if digits[9] != (sum % 11 % 10) as u8 {
                    return Err(Error::BadChecksum);
                }
            }
            12 => {
                const FIRST: [u32; 10] = [7, 2, 4, 10, 3, 5, 9, 4, 6, 8];
                let sum: u32 = digits[..10]
                    .iter()
                    .zip(FIRST)
                    .map(|(&d, w)| u32::from(d) * w)
                    .sum();
                let d1 = (sum % 11 % 10) as u8;
                if digits[10] != d1 {
                    return Err(Error::BadChecksum);
                }
                const SECOND: [u32; 11] = [3, 7, 2, 4, 10, 3, 5, 9, 4, 6, 8];
                let sum: u32 = digits[..10]
                    .iter()
                    .copied()
                    .chain(std::iter::once(d1))
                    .zip(SECOND)
                    .map(|(d, w)| u32::from(d) * w)
                    .sum();
                if digits[11] != (sum % 11 % 10) as u8 {
                    return Err(Error::BadChecksum);
                }
            }
            found => {
                return Err(Error::WrongLength {
                    expected: 10,
                    found,
                });
            }
        }
        Ok(())
    }

    /// Validates a Russian ОГРН.
    ///
    /// Thirteen digits for an organisation, or fifteen for a person, where the
    /// check is the whole body modulo eleven (or thirteen) reduced to a digit.
    pub(in crate::id) fn validate_ogrn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        let body: u64 = match digits.len() {
            13 if digits[0] != 0 => &digits[..12],
            15 if matches!(digits[0], 3 | 4) => &digits[..14],
            13 | 15 => return Err(Error::OutOfRange),
            found => {
                return Err(Error::WrongLength {
                    expected: 13,
                    found,
                });
            }
        }
        .iter()
        .fold(0u64, |acc, &d| acc * 10 + u64::from(d));
        let expected = if digits.len() == 13 {
            body % 11 % 10
        } else {
            body % 13
        };
        if u64::from(digits[digits.len() - 1]) != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Ukraine: ЄДРПОУ, the entity code.
pub(super) mod ukraine {
    use super::*;

    /// Validates a Ukrainian EDRPOU.
    ///
    /// Eight digits whose check digit is a weighted sum modulo eleven, retried
    /// with heavier weights when the first pass lands on ten.
    pub(in crate::id) fn validate_edrpou(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 8)?;
        let base: [u32; 7] = if matches!(digits[0], 3..=5) {
            [7, 1, 2, 3, 4, 5, 6]
        } else {
            [1, 2, 3, 4, 5, 6, 7]
        };
        let total: u32 = digits[..7]
            .iter()
            .zip(base)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        let check = if total % 11 < 10 {
            total % 11
        } else {
            let total: u32 = digits[..7]
                .iter()
                .zip(base.map(|w| w + 2))
                .map(|(&d, w)| u32::from(d) * w)
                .sum();
            total % 11 % 10
        };
        if digits[7] != check as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn irish_vat_examples() {
        for value in ["6433435F", "6433435OA", "8D79739I"] {
            assert!(ireland::validate_vat(value).is_ok(), "{value}");
        }
        assert_eq!(ireland::validate_vat("6433435E"), Err(Error::BadChecksum));
        assert!(ireland::validate_vat("8?79739J").is_err());
    }

    #[test]
    fn irish_pps_examples() {
        for value in [
            "6433435F",
            "6433435FT",
            "6433435FW",
            "6433435OA",
            "6433435IH",
        ] {
            assert!(ireland::validate_pps(value).is_ok(), "{value}");
        }
        assert_eq!(ireland::validate_pps("6433435E"), Err(Error::BadChecksum));
        assert_eq!(ireland::validate_pps("6433435VH"), Err(Error::BadChecksum));
    }

    #[test]
    fn hungarian_examples() {
        assert!(hungary::validate_anum("12892312").is_ok());
        assert_eq!(hungary::validate_anum("12892313"), Err(Error::BadChecksum));
    }

    #[test]
    fn croatian_examples() {
        assert!(croatia::validate_oib("33392005961").is_ok());
        assert_eq!(
            croatia::validate_oib("33392005962"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn bulgarian_examples() {
        assert!(bulgaria::validate_egn("7523169263").is_ok());
        assert_eq!(
            bulgaria::validate_egn("7523169264"),
            Err(Error::BadChecksum)
        );
        assert!(bulgaria::validate_pnf("7111042925").is_ok());
        assert_eq!(
            bulgaria::validate_pnf("7111042922"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn czechoslovak_examples() {
        for value in ["7103192745", "991231123"] {
            assert!(czechoslovakia::validate_rc(value).is_ok(), "{value}");
        }
        assert_eq!(
            czechoslovakia::validate_rc("7103192746"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            czechoslovakia::validate_rc("1103492745"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn estonian_examples() {
        assert!(estonia::validate_ik("37605030299").is_ok());
        assert_eq!(estonia::validate_ik("37605030298"), Err(Error::BadChecksum));
    }

    #[test]
    fn lithuanian_examples() {
        assert!(lithuania::validate_asmens("33309240064").is_ok());
        assert_eq!(
            lithuania::validate_asmens("33309240065"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn luxembourg_examples() {
        assert!(luxembourg::validate_tva("15027442").is_ok());
        assert_eq!(
            luxembourg::validate_tva("15027443"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn swiss_examples() {
        assert!(switzerland::validate_ssn("7569217076985").is_ok());
        assert_eq!(
            switzerland::validate_ssn("7569217076984"),
            Err(Error::BadChecksum)
        );
        assert!(switzerland::validate_uid("CHE100155212").is_ok());
        assert_eq!(
            switzerland::validate_uid("CHE100155213"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn russian_examples() {
        for value in ["123456789047", "1234567894"] {
            assert!(russia::validate_inn(value).is_ok(), "{value}");
        }
        assert_eq!(
            russia::validate_inn("123456789037"),
            Err(Error::BadChecksum)
        );
        assert!(russia::validate_ogrn("1022200525819").is_ok());
        assert!(russia::validate_ogrn("385768585948949").is_ok());
        assert_eq!(
            russia::validate_ogrn("1022500001328"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn ukrainian_examples() {
        assert!(ukraine::validate_edrpou("32855961").is_ok());
        assert_eq!(
            ukraine::validate_edrpou("32855968"),
            Err(Error::BadChecksum)
        );
    }
}
