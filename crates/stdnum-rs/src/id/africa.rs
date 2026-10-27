//! African and further-Middle-Eastern tax and company identifiers.
//!
//! These are the numbers `python-stdnum` covers for regions the crate already
//! reached through South Africa and the Maghreb but did not finish. Each is a
//! shape check: the reference defines no registry lookup, so a number that
//! passes may still be unissued.
//!
//! The reference's `compact` removes a different separator set per country
//! (Algeria allows only spaces, Senegal also commas and slashes), so each
//! validator cleans the raw input itself rather than taking the crate's global
//! normalisation. Getting that wrong would accept a number the reference
//! rejects.

use crate::{Error, Result, checksum, iso7064};

/// Removes the given separators, strips whitespace and upper-cases.
///
/// This is the reference's `clean()` plus `upper()`: the separator set differs
/// per country, which is why it is a parameter.
fn compact(input: &str, separators: &[char]) -> String {
    input
        .chars()
        .filter(|c| !c.is_whitespace() && !separators.contains(c))
        .flat_map(char::to_uppercase)
        .collect()
}

/// Algeria: NIF, the fiscal identification number.
pub(super) mod algeria {
    use super::*;

    /// Validates an Algerian NIF.
    ///
    /// Fifteen digits, or twenty for a branch or secondary establishment.
    /// There is no check digit; the reference checks the shape only.
    pub(in crate::id) fn validate_nif(input: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(input, &[' ']))?;
        if !matches!(digits.len(), 15 | 20) {
            return Err(Error::WrongLength {
                expected: 15,
                found: digits.len(),
            });
        }
        Ok(())
    }
}

/// Egypt: the tax registration number.
pub(super) mod egypt {
    use super::*;

    /// Validates an Egyptian tax registration number.
    ///
    /// Nine digits. The reference accepts Arabic-Indic digits too, so the input
    /// is transliterated to ASCII before the digit check.
    pub(in crate::id) fn validate_tn(input: &str) -> Result<()> {
        let latin = checksum::to_ascii_digits(&compact(input, &[' ', '-', '/']));
        checksum::expect_len(&checksum::digits_only(&latin)?, 9)
    }
}

/// Ghana: TIN, the taxpayer identification number.
pub(super) mod ghana {
    use super::*;

    /// Validates a Ghanaian TIN.
    ///
    /// Eleven characters: a one-letter prefix from `PCGQV`, the literal `00`,
    /// then eight alphanumerics of which the last is the check character. The
    /// check is a weighted sum of positions one to nine with weights one to
    /// nine, modulo eleven, where ten is written `X`.
    pub(in crate::id) fn validate_tin(input: &str) -> Result<()> {
        let s = compact(input, &[' ']);
        checksum::expect_len_str(&s, 11)?;
        let bytes = s.as_bytes();
        if !matches!(bytes[0], b'P' | b'C' | b'G' | b'Q' | b'V') || &bytes[1..3] != b"00" {
            return Err(Error::InvalidPrefix);
        }
        // The reference's pattern is `[PCGQV]00[A-Z0-9]{8}`, but its
        // `calc_check_digit` then calls `int()` on positions one to nine, so a
        // letter there raises and the number is reported invalid. Rejecting it
        // here gives the same verdict without the crash.
        if let Some(bad) = bytes[1..10].iter().find(|b| !b.is_ascii_digit()) {
            return Err(Error::InvalidCharacter(char::from(*bad)));
        }
        if !bytes[10].is_ascii_alphanumeric() {
            return Err(Error::InvalidCharacter(char::from(bytes[10])));
        }
        let sum: u32 = bytes[1..10]
            .iter()
            .enumerate()
            .map(|(i, b)| (i as u32 + 1) * u32::from(b - b'0'))
            .sum();
        let expected = match sum % 11 {
            10 => b'X',
            v => b'0' + v as u8,
        };
        if bytes[10] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Kenya: PIN, the KRA personal identification number.
pub(super) mod kenya {
    use super::*;

    /// Validates a Kenyan KRA PIN.
    ///
    /// Eleven characters: `A` for an individual or `P` for a non-individual,
    /// nine digits and a final letter. There is no check digit.
    pub(in crate::id) fn validate_pin(input: &str) -> Result<()> {
        let s = compact(input, &[' ', '-']);
        checksum::expect_len_str(&s, 11)?;
        let bytes = s.as_bytes();
        if !matches!(bytes[0], b'A' | b'P') {
            return Err(Error::InvalidPrefix);
        }
        if let Some(bad) = bytes[1..10].iter().find(|b| !b.is_ascii_digit()) {
            return Err(Error::InvalidCharacter(char::from(*bad)));
        }
        if !bytes[10].is_ascii_uppercase() {
            return Err(Error::InvalidCharacter(char::from(bytes[10])));
        }
        Ok(())
    }
}

/// Morocco: ICE, the common enterprise identifier.
pub(super) mod morocco {
    use super::*;

    /// Validates a Moroccan ICE.
    ///
    /// Fifteen digits whose MOD 97-10 checksum is zero.
    pub(in crate::id) fn validate_ice(input: &str) -> Result<()> {
        let s = compact(input, &[' ']);
        checksum::expect_len(&checksum::digits_only(&s)?, 15)?;
        if iso7064::mod_97_10_checksum(&s) != Some(0) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Mozambique: NUIT, the single tax identification number.
pub(super) mod mozambique {
    use super::*;

    /// Validates a Mozambican NUIT.
    ///
    /// Nine digits; the last is a weighted sum of the first eight with weights
    /// 8, 9, 4, 5, 6, 7, 8, 9 modulo eleven. The reference indexes its digit
    /// alphabet as `01234567891`, so a remainder of ten maps to `1` and never
    /// to `10`.
    pub(in crate::id) fn validate_nuit(input: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(input, &[' ', '-', '.']))?;
        checksum::expect_len(&digits, 9)?;
        const WEIGHTS: [u32; 8] = [8, 9, 4, 5, 6, 7, 8, 9];
        let sum: u32 = WEIGHTS
            .iter()
            .zip(&digits)
            .map(|(w, &d)| w * u32::from(d))
            .sum();
        let expected = match sum % 11 {
            10 => 1,
            v => v as u8,
        };
        if digits[8] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Senegal: NINEA, the national business and association number.
pub(super) mod senegal {
    use super::*;

    /// Validates a Senegalese NINEA.
    ///
    /// Seven or nine digits, optionally followed by a three-character COFI
    /// code. The digit part has a Luhn-like checksum: weights one and two
    /// alternate from the left, and the total modulo ten is zero.
    pub(in crate::id) fn validate_ninea(input: &str) -> Result<()> {
        let s = compact(input, &[' ', '-', '/', ',']);
        let chars: Vec<char> = s.chars().collect();
        let (payload, cofi) = if chars.len() > 9 {
            let split = chars.len() - 3;
            (
                chars[..split].iter().collect::<String>(),
                Some(chars[split..].iter().collect::<String>()),
            )
        } else {
            (s.clone(), None)
        };
        if !matches!(payload.chars().count(), 7 | 9) {
            return Err(Error::WrongLength {
                expected: 7,
                found: payload.chars().count(),
            });
        }
        if let Some(cofi) = cofi {
            validate_cofi(&cofi)?;
        }
        let digits = checksum::digits_only(&payload)?;
        // The reference left-pads to nine before applying the weights, so a
        // seven-digit payload behaves as if it had two leading zeroes.
        let pad = 9 - digits.len();
        let mut sum = 0u32;
        for i in 0..9 {
            let d = if i < pad {
                0
            } else {
                u32::from(digits[i - pad])
            };
            sum += if i.is_multiple_of(2) { d } else { 2 * d };
        }
        if !sum.is_multiple_of(10) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates the three-character COFI suffix.
    ///
    /// The first character is the tax status, the second the tax centre and the
    /// third the legal form.
    fn validate_cofi(cofi: &str) -> Result<()> {
        let bytes: Vec<u8> = cofi.bytes().collect();
        if bytes.len() != 3 {
            return Err(Error::WrongLength {
                expected: 3,
                found: bytes.len(),
            });
        }
        if !matches!(bytes[0], b'0'..=b'2') {
            return Err(Error::OutOfRange);
        }
        // The reference's alphabet omits I and O.
        const CENTRES: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWZ";
        if !CENTRES.contains(&bytes[1]) {
            return Err(Error::OutOfRange);
        }
        if !bytes[2].is_ascii_digit() {
            return Err(Error::InvalidCharacter(char::from(bytes[2])));
        }
        Ok(())
    }
}

/// Tunisia: MF, the matricule fiscal.
pub(super) mod tunisia {
    use super::*;

    /// Validates a Tunisian matricule fiscal.
    ///
    /// Either eight characters (the fiscal identifier and its control key) or
    /// thirteen (those plus a VAT code, a category code and a three-digit
    /// secondary-establishment number). There is no arithmetic check; the
    /// reference validates each field against its alphabet.
    ///
    /// The reference left-pads the leading digit run to seven before checking,
    /// so `121J` is the same number as `0000121J`.
    pub(in crate::id) fn validate_mf(input: &str) -> Result<()> {
        let s = pad_serial(&compact(input, &[' ', '/', '.', '-']));
        let chars: Vec<char> = s.chars().collect();
        if !matches!(chars.len(), 8 | 13) {
            return Err(Error::WrongLength {
                expected: 8,
                found: chars.len(),
            });
        }
        if let Some(bad) = chars[..7].iter().find(|c| !c.is_ascii_digit()) {
            return Err(Error::InvalidCharacter(*bad));
        }
        // The control key excludes I, O and U.
        const CONTROL_KEYS: &[char] = &[
            'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'J', 'K', 'L', 'M', 'N', 'P', 'Q', 'R', 'S',
            'T', 'V', 'W', 'X', 'Y', 'Z',
        ];
        if !CONTROL_KEYS.contains(&chars[7]) {
            return Err(Error::OutOfRange);
        }
        if chars.len() == 8 {
            return Ok(());
        }
        const TVA_CODES: &[char] = &['A', 'P', 'B', 'D', 'N'];
        if !TVA_CODES.contains(&chars[8]) {
            return Err(Error::OutOfRange);
        }
        const CATEGORY_CODES: &[char] = &['M', 'P', 'C', 'N', 'E'];
        if !CATEGORY_CODES.contains(&chars[9]) {
            return Err(Error::OutOfRange);
        }
        if chars[10..].iter().any(|c| !c.is_ascii_digit()) {
            return Err(Error::OutOfRange);
        }
        // A secondary-establishment number other than `000` requires the
        // category code `E`.
        if chars[10..] != ['0', '0', '0'] && chars[9] != 'E' {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Zero-pads the leading run of digits to seven, as the reference does.
    fn pad_serial(s: &str) -> String {
        let digits = s.chars().take_while(char::is_ascii_digit).count();
        if digits == 0 || digits >= 7 {
            return s.to_owned();
        }
        let mut out = "0".repeat(7 - digits);
        out.push_str(s);
        out
    }
}

/// Vietnam: MST, the tax code.
pub(super) mod vietnam {
    use super::*;

    /// Validates a Vietnamese MST.
    ///
    /// Ten digits, or thirteen for a branch where the last three are a sequence
    /// number. The tenth digit is a weighted sum of the first nine with weights
    /// 31, 29, 23, 19, 17, 13, 7, 5, 3, and the check is `10 - (total mod 11)`.
    pub(in crate::id) fn validate_mst(input: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(input, &[' ', '-', '.']))?;
        if !matches!(digits.len(), 10 | 13) {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        // A sequential block of all zeroes is not issued, and neither is a
        // branch sequence of `000`.
        if digits[2..9].iter().all(|&d| d == 0) {
            return Err(Error::OutOfRange);
        }
        if digits.len() == 13 && digits[10..].iter().all(|&d| d == 0) {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 9] = [31, 29, 23, 19, 17, 13, 7, 5, 3];
        let total: u32 = WEIGHTS
            .iter()
            .zip(&digits)
            .map(|(w, &d)| w * u32::from(d))
            .sum();
        if u32::from(digits[9]) != 10 - (total % 11) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Every accepted value below is a doctest of the corresponding
    // python-stdnum module; every rejected one is a `Traceback` doctest there.

    #[test]
    fn algerian_examples() {
        for value in [
            "416001000000007",
            "408 020 000 150 039",
            "41201600000606600001",
            "000 216 001 808 337 13010",
        ] {
            assert!(algeria::validate_nif(value).is_ok(), "{value}");
        }
        assert!(algeria::validate_nif("12345").is_err());
        assert!(algeria::validate_nif("X1600100000000V").is_err());
    }

    #[test]
    fn egyptian_examples() {
        assert!(egypt::validate_tn("100-531-385").is_ok());
        // The reference accepts Arabic-Indic digits.
        assert!(egypt::validate_tn("٣٣١-١٠٥-٢٦٨").is_ok());
        assert!(egypt::validate_tn("12345").is_err());
        assert!(egypt::validate_tn("VV3456789").is_err());
    }

    #[test]
    fn ghanaian_examples() {
        assert!(ghana::validate_tin("C0000803561").is_ok());
        assert_eq!(ghana::validate_tin("C0000803562"), Err(Error::BadChecksum));
        assert_eq!(
            ghana::validate_tin("X0012345678"),
            Err(Error::InvalidPrefix)
        );
    }

    #[test]
    fn kenyan_examples() {
        assert!(kenya::validate_pin("P051365947M").is_ok());
        assert!(kenya::validate_pin("A004416331M").is_ok());
        assert!(kenya::validate_pin("a004416331m").is_ok());
        assert!(kenya::validate_pin("12345").is_err());
        assert!(kenya::validate_pin("V1234567890").is_err());
    }

    #[test]
    fn moroccan_examples() {
        assert!(morocco::validate_ice("001561191000066").is_ok());
        assert!(morocco::validate_ice("00 21 36 09 30 00 040").is_ok());
        assert!(morocco::validate_ice("12345").is_err());
        assert_eq!(
            morocco::validate_ice("001561191000065"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn mozambican_examples() {
        assert!(mozambique::validate_nuit("400339910").is_ok());
        assert!(mozambique::validate_nuit("400 005 834").is_ok());
        assert!(mozambique::validate_nuit("12345").is_err());
        assert_eq!(
            mozambique::validate_nuit("400339911"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn senegalese_examples() {
        assert!(senegal::validate_ninea("306 7221").is_ok());
        assert!(senegal::validate_ninea("30672212G2").is_ok());
        assert!(senegal::validate_ninea("3067221 2G2").is_ok());
        assert_eq!(senegal::validate_ninea("3067222"), Err(Error::BadChecksum));
        assert!(senegal::validate_ninea("1234567 0AZ").is_err());
    }

    #[test]
    fn tunisian_examples() {
        assert!(tunisia::validate_mf("1234567/M/A/E/001").is_ok());
        assert!(tunisia::validate_mf("1282182 W").is_ok());
        // The serial is zero-padded to seven.
        assert!(tunisia::validate_mf("121J").is_ok());
        assert!(tunisia::validate_mf("1496298 T P N 000").is_ok());
        // `U` is not a control key, and a non-`000` establishment needs `E`.
        assert!(tunisia::validate_mf("1219773U").is_err());
        assert!(tunisia::validate_mf("1234567/M/A/X/000").is_err());
    }

    #[test]
    fn vietnamese_examples() {
        assert!(vietnam::validate_mst("0100233488").is_ok());
        assert!(vietnam::validate_mst("0314409058-002").is_ok());
        assert!(vietnam::validate_mst("12345").is_err());
        assert_eq!(vietnam::validate_mst("0100233480"), Err(Error::BadChecksum));
    }
}
