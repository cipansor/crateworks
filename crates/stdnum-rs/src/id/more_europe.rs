//! Further European identifiers: the microstates, the Caucasus and the Baltic
//! and Central European numbers that were still missing.
//!
//! Each validator cleans the raw input with the separator set its reference
//! `compact` uses, which differs per country, and each is cross-checked against
//! `python-stdnum` by `tests/fixtures/stdnum_vectors.tsv`.

use crate::{Error, Result, checksum};

/// Removes the given separators, strips whitespace and upper-cases.
fn compact(input: &str, separators: &[char]) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !separators.contains(c))
        .flat_map(char::to_uppercase)
        .collect()
}

/// Andorra: NRT, the taxpayer registry number.
pub(super) mod andorra {
    use super::*;

    /// Validates an Andorran NRT.
    ///
    /// Eight characters: a letter from `ACDEFGLOPU`, six digits and a control
    /// letter. The reference checks the ranges the first letter implies, which
    /// is what pins the number to a person or an entity.
    pub(in crate::id) fn validate_nrt(s: &str) -> Result<()> {
        let s = compact(s, &['-', '.']);
        checksum::expect_len_str(&s, 8)?;
        let bytes = s.as_bytes();
        if !bytes[0].is_ascii_alphabetic() || !bytes[7].is_ascii_alphabetic() {
            return Err(Error::InvalidPrefix);
        }
        if let Some(bad) = bytes[1..7].iter().find(|b| !b.is_ascii_digit()) {
            return Err(Error::InvalidCharacter(char::from(*bad)));
        }
        if !b"ACDEFGLOPU".contains(&bytes[0]) {
            return Err(Error::OutOfRange);
        }
        let body = &s[1..7];
        if bytes[0] == b'F' && body > "699999" {
            return Err(Error::OutOfRange);
        }
        if matches!(bytes[0], b'A' | b'L') && !(body > "699999" && body < "800000") {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Albania: NIPT, the taxpayer identification number.
pub(super) mod albania {
    use super::*;

    /// Validates an Albanian NIPT.
    ///
    /// Ten characters: a letter `A` to `M`, eight digits and a final letter.
    pub(in crate::id) fn validate_nipt(s: &str) -> Result<()> {
        let s = s
            .strip_prefix("(AL)")
            .or_else(|| s.strip_prefix("AL"))
            .map_or_else(|| compact(s, &[]), |r| compact(r, &[]));
        checksum::expect_len_str(&s, 10)?;
        let bytes = s.as_bytes();
        if !(b'A'..=b'M').contains(&bytes[0]) {
            return Err(Error::InvalidPrefix);
        }
        if let Some(bad) = bytes[1..9].iter().find(|b| !b.is_ascii_digit()) {
            return Err(Error::InvalidCharacter(char::from(*bad)));
        }
        if !bytes[9].is_ascii_uppercase() {
            return Err(Error::InvalidCharacter(char::from(bytes[9])));
        }
        Ok(())
    }
}

/// Azerbaijan: VÖEN, the taxpayer identification number.
pub(super) mod azerbaijan {
    use super::*;

    /// Validates an Azerbaijani VÖEN.
    ///
    /// Ten digits, left-padded with a zero when only nine were given. The
    /// second-to-last digit is the check and the last is always 1 or 2.
    pub(in crate::id) fn validate_voen(s: &str) -> Result<()> {
        let mut s = compact(s, &[]);
        if s.len() == 9 {
            s.insert(0, '0');
        }
        let digits = checksum::digits_only(&s)?;
        checksum::expect_len(&digits, 10)?;
        if !matches!(digits[9], 1 | 2) {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 8] = [4, 1, 8, 6, 2, 7, 5, 3];
        let sum: u32 = WEIGHTS
            .iter()
            .zip(&digits)
            .map(|(w, &d)| w * u32::from(d))
            .sum::<u32>()
            % 11;
        if u32::from(digits[8]) != sum {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Faroe Islands: V-number.
pub(super) mod faroe_islands {
    use super::*;

    /// Validates a Faroese V-number.
    ///
    /// Six digits, optionally prefixed with `FO`. There is no check digit.
    pub(in crate::id) fn validate_vn(s: &str) -> Result<()> {
        let s = compact(s, &['-', '.']);
        let s = s.strip_prefix("FO").unwrap_or(&s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 6)?;
        Ok(())
    }
}

/// Moldova: IDNO, the state identification number of a legal entity.
pub(super) mod moldova {
    use super::*;

    /// Validates a Moldovan IDNO.
    ///
    /// Thirteen digits. The check digit is a weighted sum with the weights
    /// 7, 3, 1 repeating, taken modulo ten.
    pub(in crate::id) fn validate_idno(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 13)?;
        const WEIGHTS: [u32; 13] = [7, 3, 1, 7, 3, 1, 7, 3, 1, 7, 3, 1, 0];
        let sum: u32 = WEIGHTS
            .iter()
            .zip(&digits)
            .map(|(w, &d)| w * u32::from(d))
            .sum::<u32>()
            % 10;
        if u32::from(digits[12]) != sum {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// San Marino: COE, the economic operator code.
pub(super) mod san_marino {
    use super::*;

    /// Validates a Sammarinese COE.
    ///
    /// One to five digits, without leading zeroes. A number of two digits or
    /// fewer must be one of the registered low numbers.
    pub(in crate::id) fn validate_coe(s: &str) -> Result<()> {
        let s = compact(s, &['.']);
        let s = s.trim_start_matches('0');
        if s.is_empty() {
            return Err(Error::Empty);
        }
        if s.len() > 5 {
            return Err(Error::WrongLength {
                expected: 5,
                found: s.len(),
            });
        }
        let digits = checksum::digits_only(s)?;
        if digits.len() < 3 {
            const LOW: [u32; 63] = [
                2, 4, 6, 7, 8, 9, 10, 11, 13, 16, 18, 19, 20, 21, 25, 26, 30, 32, 33, 35, 36, 37,
                38, 39, 40, 42, 45, 47, 49, 51, 52, 55, 56, 57, 58, 59, 61, 62, 64, 65, 66, 67, 68,
                69, 70, 71, 72, 73, 74, 75, 76, 79, 80, 81, 84, 85, 87, 88, 91, 92, 94, 95, 96,
            ];
            let value: u32 = s.parse().map_err(|_| Error::InvalidPrefix)?;
            if !LOW.contains(&value) {
                return Err(Error::OutOfRange);
            }
        }
        Ok(())
    }
}

/// Liechtenstein: PEID, the business and person identifier.
pub(super) mod liechtenstein {
    use super::*;

    /// Validates a Liechtenstein PEID.
    ///
    /// Four to twelve digits after the leading zeroes are stripped.
    pub(in crate::id) fn validate_peid(s: &str) -> Result<()> {
        let s = compact(s, &['.']).trim_start_matches('0').to_owned();
        let digits = checksum::digits_only(&s)?;
        if !(4..=12).contains(&digits.len()) {
            return Err(Error::WrongLength {
                expected: 4,
                found: digits.len(),
            });
        }
        Ok(())
    }
}

/// Latvia: PVN, the VAT registration number.
pub(super) mod latvia {
    use super::*;

    /// Validates a Latvian PVN.
    ///
    /// Eleven digits, optionally prefixed with `LV`. A number whose first digit
    /// exceeds three is a legal entity checked by one weighted sum; the rest are
    /// personal codes checked by another, and unless they start with `32` their
    /// birth date must be real.
    pub(in crate::id) fn validate_pvn(s: &str) -> Result<()> {
        let s = compact(s, &['-']);
        let s = s.strip_prefix("LV").unwrap_or(&s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        if digits[0] > 3 {
            const WEIGHTS: [u32; 11] = [9, 1, 4, 8, 3, 10, 2, 5, 7, 6, 1];
            let sum: u32 = WEIGHTS
                .iter()
                .zip(&digits)
                .map(|(w, &d)| w * u32::from(d))
                .sum::<u32>()
                % 11;
            if sum != 3 {
                return Err(Error::BadChecksum);
            }
            return Ok(());
        }
        if !s.starts_with("32") {
            let day = u32::from(digits[0]) * 10 + u32::from(digits[1]);
            let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
            let year = 1800
                + u32::from(digits[6]) * 100
                + u32::from(digits[4]) * 10
                + u32::from(digits[5]);
            if !checksum::is_real_date(year, month, day) {
                return Err(Error::OutOfRange);
            }
        }
        if !personal_check_digit(&digits[..10], digits[10]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Whether the Latvian personal-code check digit matches.
    fn personal_check_digit(payload: &[u8], expected: u8) -> bool {
        const WEIGHTS: [u32; 10] = [10, 5, 8, 4, 2, 1, 6, 3, 7, 9];
        let sum: u32 = 1 + WEIGHTS
            .iter()
            .zip(payload)
            .map(|(w, &d)| w * u32::from(d))
            .sum::<u32>();
        (sum % 11) % 10 == u32::from(expected)
    }
}

/// Monaco: TVA, the VAT number, which is a French one with a fixed prefix.
pub(super) mod monaco {
    use super::*;

    /// Validates a Monégasque VAT number.
    ///
    /// Monaco uses the French scheme, so this delegates to the French check
    /// after requiring the middle three characters to be `000`.
    pub(in crate::id) fn validate_tva(s: &str) -> Result<()> {
        let body = compact(s, &['-', '.']);
        let body = body.strip_prefix("FR").unwrap_or(&body);
        let digits = checksum::digits_only(body).map_err(|_| Error::InvalidPrefix)?;
        if digits.len() != 11 {
            return Err(Error::WrongLength {
                expected: 11,
                found: digits.len(),
            });
        }
        if &body[2..5] != "000" {
            return Err(Error::OutOfRange);
        }
        crate::id::france_vat(body)
    }
}

/// Austria: UID, the VAT identification number, and TIN, the tax number.
pub(super) mod austria {
    use super::*;

    /// The tax-office prefixes the reference's `at/fa` table lists.
    const OFFICES: [&str; 40] = [
        "03", "04", "06", "07", "08", "09", "10", "12", "15", "16", "18", "22", "23", "29", "33",
        "38", "41", "46", "51", "52", "53", "54", "57", "59", "61", "65", "67", "68", "69", "71",
        "72", "81", "82", "83", "84", "90", "91", "93", "97", "98",
    ];

    /// Validates an Austrian UID.
    ///
    /// Nine characters: `U` followed by eight digits. The check digit is the
    /// Luhn checksum of the seven digits after the `U`, negated modulo ten.
    pub(in crate::id) fn validate_uid(s: &str) -> Result<()> {
        let s = compact(s, &['-', '.', '/']);
        let s = s.strip_prefix("AT").unwrap_or(&s);
        checksum::expect_len_str(s, 9)?;
        let bytes = s.as_bytes();
        if bytes[0] != b'U' || !bytes[1..].iter().all(u8::is_ascii_digit) {
            return Err(Error::InvalidPrefix);
        }
        let digits: Vec<u8> = s[1..].bytes().map(|b| b - b'0').collect();
        let luhn = checksum::luhn_checksum(&digits[..7]);
        // The reference subtracts from six and takes the result modulo ten, so
        // a checksum above six must wrap into range rather than underflow.
        let expected = (6i32 - i32::from(luhn)).rem_euclid(10) as u8;
        if digits[7] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates an Austrian TIN (Abgabenkontonummer).
    ///
    /// Nine digits. The first two must name a tax office the reference knows,
    /// and the last is a check digit over the first eight with the even
    /// positions folded through `0246813579`.
    pub(in crate::id) fn validate_tin(s: &str) -> Result<()> {
        let s = compact(s, &['-', '.', '/', ',']);
        let digits = checksum::digits_only(&s)?;
        checksum::expect_len(&digits, 9)?;
        let prefix = format!(
            "{}{}",
            char::from(b'0' + digits[0]),
            char::from(b'0' + digits[1])
        );
        if !OFFICES.contains(&prefix.as_str()) {
            return Err(Error::OutOfRange);
        }
        const FOLD: [u32; 10] = [0, 2, 4, 6, 8, 1, 3, 5, 7, 9];
        let sum: u32 = digits[..8]
            .iter()
            .enumerate()
            .map(|(i, &d)| {
                if i % 2 == 0 {
                    u32::from(d)
                } else {
                    FOLD[usize::from(d)]
                }
            })
            .sum();
        if u32::from(digits[8]) != (10 - sum % 10) % 10 {
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
    fn andorran_examples() {
        for value in ["U-132950-X", "C123456Z"] {
            assert!(andorra::validate_nrt(value).is_ok(), "{value}");
        }
        assert_eq!(
            andorra::validate_nrt("A123B"),
            Err(Error::WrongLength {
                expected: 8,
                found: 5,
            })
        );
        // The first letter must be one the reference lists.
        assert_eq!(andorra::validate_nrt("B123456Z"), Err(Error::OutOfRange));
    }

    #[test]
    fn albanian_examples() {
        for value in ["AL J 91402501 L", "K22218003V"] {
            assert!(albania::validate_nipt(value).is_ok(), "{value}");
        }
        assert_eq!(
            albania::validate_nipt("Z 22218003 V"),
            Err(Error::InvalidPrefix)
        );
    }

    #[test]
    fn azerbaijani_examples() {
        assert!(azerbaijan::validate_voen("140 155 5071").is_ok());
        assert_eq!(
            azerbaijan::validate_voen("140 155 5081"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn faroese_examples() {
        assert!(faroe_islands::validate_vn("623857").is_ok());
        assert_eq!(
            faroe_islands::validate_vn("1234"),
            Err(Error::WrongLength {
                expected: 6,
                found: 4,
            })
        );
    }

    #[test]
    fn moldovan_examples() {
        assert!(moldova::validate_idno("1008600038413").is_ok());
        assert_eq!(
            moldova::validate_idno("1008600038412"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn sammarinese_examples() {
        for value in ["51", "024165"] {
            assert!(san_marino::validate_coe(value).is_ok(), "{value}");
        }
        assert_eq!(
            san_marino::validate_coe("1124165"),
            Err(Error::WrongLength {
                expected: 5,
                found: 7,
            })
        );
    }

    #[test]
    fn liechtenstein_examples() {
        for value in ["1234567", "00001234567"] {
            assert!(liechtenstein::validate_peid(value).is_ok(), "{value}");
        }
        assert_eq!(
            liechtenstein::validate_peid("123"),
            Err(Error::WrongLength {
                expected: 4,
                found: 3,
            })
        );
    }

    #[test]
    fn latvian_examples() {
        for value in ["LV 4000 3521 600", "161175-19997"] {
            assert!(latvia::validate_pvn(value).is_ok(), "{value}");
        }
        assert_eq!(latvia::validate_pvn("40003521601"), Err(Error::BadChecksum));
        assert_eq!(latvia::validate_pvn("161375-19997"), Err(Error::OutOfRange));
    }

    #[test]
    fn monacan_examples() {
        assert!(monaco::validate_tva("53 0000 04605").is_ok());
        assert_eq!(
            monaco::validate_tva("FR 61 954 506 077"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn austrian_examples() {
        assert!(austria::validate_uid("AT U13585627").is_ok());
        assert_eq!(austria::validate_uid("U13585626"), Err(Error::BadChecksum));
        assert!(austria::validate_tin("59-119/9013").is_ok());
    }
}
