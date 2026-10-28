//! Further Asian, Middle Eastern and African identifiers.

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

/// Israel: ת.ז., the identity number.
pub(super) mod israel {
    use super::*;

    /// Validates an Israeli identity number.
    ///
    /// Up to nine digits, left-padded with zeroes, checked by Luhn.
    pub(in crate::id) fn validate_idnr(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        if digits.is_empty() || digits.len() > 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits.len(),
            });
        }
        let mut padded = vec![0u8; 9 - digits.len()];
        padded.extend_from_slice(&digits);
        if padded.iter().all(|&d| d == 0) {
            return Err(Error::OutOfRange);
        }
        if !checksum::luhn_matches(&padded[..8], padded[8]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates an Israeli company number (מספר חברה).
    ///
    /// Nine digits whose first is a `5` and whose last is a Luhn check digit.
    /// The reference requires the whole number to be positive, so an all-zero
    /// number is rejected.
    pub(in crate::id) fn validate_company_number(s: &str) -> Result<()> {
        let cleaned = compact(s, &[' ', '-']);
        let digits = checksum::digits_only(&cleaned)?;
        checksum::expect_len(&digits, 9)?;
        if digits[0] != 5 || digits.iter().all(|&d| d == 0) {
            return Err(Error::OutOfRange);
        }
        if !checksum::luhn_matches(&digits[..8], digits[8]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Thailand: เลขประจำตัวประชาชน, the personal identification number.
pub(super) mod thailand {
    use super::*;

    /// Validates a Thai personal identification number.
    ///
    /// Thirteen digits whose check digit is a weighted sum with weights
    /// counting down from two.
    pub(in crate::id) fn validate_pin(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 13)?;
        if matches!(digits[0], 0 | 9) {
            return Err(Error::OutOfRange);
        }
        // The weight at position `i` is `2 - i`, which goes negative past the
        // second digit; the reference reduces it modulo eleven, so this does
        // the same with a signed sum.
        let sum: i64 = digits[..12]
            .iter()
            .enumerate()
            .map(|(i, &d)| (2 - i as i64) * i64::from(d))
            .sum::<i64>()
            .rem_euclid(11);
        let expected = (1 - sum).rem_euclid(10) as u8;
        if digits[12] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Thai MOA (memorandum of association) number.
    ///
    /// Thirteen digits like the PIN, always beginning with zero, and using the
    /// same check digit: the weighted sum over the first twelve digits.
    pub(in crate::id) fn validate_moa(s: &str) -> Result<()> {
        let compacted: String = s.chars().filter(|c| !matches!(c, ' ' | '-')).collect();
        let digits = checksum::digits_only(&compacted)?;
        checksum::expect_len(&digits, 13)?;
        if digits[0] != 0 {
            return Err(Error::OutOfRange);
        }
        let sum: i64 = digits[..12]
            .iter()
            .enumerate()
            .map(|(i, &d)| (2 - i as i64) * i64::from(d))
            .sum::<i64>()
            .rem_euclid(11);
        let expected = (1 - sum).rem_euclid(10) as u8;
        if digits[12] != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Thai Taxpayer Identification Number in either of its two
    /// forms.
    ///
    /// The reference tries the personal number and the company number in turn,
    /// so a thirteen-digit number is valid when it satisfies either check. The
    /// two disagree on the leading digit and on the weight sequence, so the
    /// distinction is real and not cosmetic.
    pub(in crate::id) fn validate_tin(s: &str) -> Result<()> {
        let cleaned = crate::id::clean(s, &[' ', '-']);
        validate_pin(&cleaned)
            .or_else(|_| validate_moa(&cleaned))
            .map_err(|_| Error::BadChecksum)
    }
}

/// Singapore: UEN, the Unique Entity Number.
pub(super) mod singapore {
    use super::*;

    /// The two-letter entity types a ten-character UEN may carry.
    const ENTITY_TYPES: &[&str] = &[
        "CC", "CD", "CH", "CL", "CM", "CP", "CS", "CX", "DP", "FB", "FC", "FM", "FN", "GA", "GB",
        "GS", "HS", "LL", "LP", "MB", "MC", "MD", "MH", "MM", "MQ", "NB", "NR", "PA", "PB", "PF",
        "RF", "RP", "SM", "SS", "TC", "TU", "VH", "XL",
    ];

    /// Validates a Singapore UEN in any of its three shapes.
    pub(in crate::id) fn validate_uen(s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        match bytes.len() {
            9 => validate_business(s),
            10 if bytes[0].is_ascii_digit() => validate_local_company(s),
            10 => validate_other(s),
            found => Err(Error::WrongLength { expected: 9, found }),
        }
    }

    /// The eight-digit business (ROB) shape.
    fn validate_business(s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        if !bytes[..8].iter().all(u8::is_ascii_digit) || !bytes[8].is_ascii_alphabetic() {
            return Err(Error::InvalidCharacter(char::from(bytes[0])));
        }
        const WEIGHTS: [u32; 8] = [10, 4, 9, 3, 8, 2, 7, 1];
        let sum: u32 = bytes[..8]
            .iter()
            .zip(WEIGHTS)
            .map(|(&b, w)| u32::from(b - b'0') * w)
            .sum();
        if bytes[8] != b"XMKECAWLJDB"[(sum % 11) as usize] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The nine-digit local company (ROC) shape, whose first four digits are
    /// the year of issue.
    fn validate_local_company(s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        if !bytes[..9].iter().all(u8::is_ascii_digit) || !bytes[9].is_ascii_alphabetic() {
            return Err(Error::InvalidCharacter(char::from(bytes[0])));
        }
        const WEIGHTS: [u32; 9] = [10, 8, 6, 4, 9, 7, 5, 3, 1];
        let sum: u32 = bytes[..9]
            .iter()
            .zip(WEIGHTS)
            .map(|(&b, w)| u32::from(b - b'0') * w)
            .sum();
        if bytes[9] != b"ZKCMDNERGWH"[(sum % 11) as usize] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The ten-character "other entity" shape.
    fn validate_other(s: &str) -> Result<()> {
        let bytes = s.as_bytes();
        if !matches!(bytes[0], b'R' | b'S' | b'T') {
            return Err(Error::InvalidCharacter(char::from(bytes[0])));
        }
        if !bytes[1..3].iter().all(u8::is_ascii_digit)
            || !bytes[5..9].iter().all(u8::is_ascii_digit)
        {
            return Err(Error::InvalidCharacter(char::from(bytes[1])));
        }
        let entity = std::str::from_utf8(&bytes[3..5]).unwrap_or_default();
        if !ENTITY_TYPES.contains(&entity) {
            return Err(Error::InvalidPrefix);
        }
        const ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWX0123456789";
        const WEIGHTS: [u32; 9] = [4, 3, 5, 3, 10, 2, 2, 5, 7];
        let mut sum: i64 = -5;
        for (&b, w) in bytes[..9].iter().zip(WEIGHTS) {
            let value = ALPHABET
                .iter()
                .position(|&a| a == b)
                .ok_or(Error::InvalidCharacter(char::from(b)))?;
            sum += value as i64 * i64::from(w);
        }
        let index = sum.rem_euclid(11) as usize;
        if bytes[9] != ALPHABET[index] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Malaysia: NRIC, the identity card number.
pub(super) mod malaysia {
    use super::*;

    /// Validates a Malaysian identity card number.
    ///
    /// Twelve digits: a six-digit birth date and a two-digit place of birth.
    pub(in crate::id) fn validate_nric(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 12)?;
        // The date may use either century, and the reference accepts whichever
        // one is a real date.
        let year = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        if !checksum::is_real_date(1900 + year, month, day)
            && !checksum::is_real_date(2000 + year, month, day)
        {
            return Err(Error::OutOfRange);
        }
        let place = u32::from(digits[6]) * 10 + u32::from(digits[7]);
        if !is_birth_place(place) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Whether a two-digit code is a Malaysian state or a known foreign
    /// country. The set is the one `python-stdnum` ships in its `my/bp`
    /// database.
    fn is_birth_place(place: u32) -> bool {
        matches!(
            place,
            1..=16
                | 21..=39
                | 40..=68
                | 71..=72
                | 74..=79
                | 82..=93
                | 98..=99
        )
    }
}

/// South Africa: the identity number.
pub(super) mod south_africa {
    use super::*;

    /// Validates a South African identity number.
    ///
    /// Thirteen digits: a birth date, a serial whose first digit encodes sex,
    /// a citizenship digit, and a Luhn check digit.
    pub(in crate::id) fn validate_idnr(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 13)?;
        if !matches!(digits[10], 0 | 1) {
            return Err(Error::OutOfRange);
        }
        // The number writes only two year digits and the century is inferred
        // from the present, so the full year is reconstructed the way the
        // reference does before the date is checked.
        let year = u32::from(digits[0]) * 10
            + u32::from(digits[1])
            + 100 * (checksum::current_year() / 100);
        let year = if year > checksum::current_year() {
            year - 100
        } else {
            year
        };
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        if !checksum::luhn_matches(&digits[..12], digits[12]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a South African tax reference number.
    ///
    /// Ten digits with a Luhn check; the first digit is one of `01239`.
    pub(in crate::id) fn validate_tin(s: &str) -> Result<()> {
        let s: String = s
            .chars()
            .filter(|c| !matches!(c, ' ' | '-' | '/'))
            .flat_map(char::to_uppercase)
            .collect();
        if s.chars().count() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: s.chars().count(),
            });
        }
        let digits = checksum::digits_only(&s)?;
        if !matches!(digits[0], 0..=3 | 9) {
            return Err(Error::OutOfRange);
        }
        if !checksum::luhn_matches(&digits[..9], digits[9]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Greece: ΑΜΚΑ, the social security number.
pub(super) mod greece {
    use super::*;

    /// Validates a Greek AMKA.
    ///
    /// Eleven digits whose first six are a birth date and whose whole is
    /// checked by Luhn.
    pub(in crate::id) fn validate_amka(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        if !checksum::luhn_matches(&digits[..10], digits[10]) {
            return Err(Error::BadChecksum);
        }
        let day = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let year = 1900 + u32::from(digits[4]) * 10 + u32::from(digits[5]);
        if !checksum::is_real_date(year, month, day)
            && !checksum::is_real_date(year + 100, month, day)
        {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Japan: the corporate number and the My Number individual number.
pub(super) mod japan {
    use super::*;

    /// Validates a Japanese corporate number (法人番号).
    ///
    /// Thirteen digits. The check digit is the first digit, derived from the
    /// remaining twelve with weights that alternate 1 and 2 from the right;
    /// the reference takes the sum modulo nine and subtracts it from nine.
    pub(in crate::id) fn validate_cn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(s, &['-']))?;
        checksum::expect_len(&digits, 13)?;
        const WEIGHTS: [u32; 12] = [1, 2, 1, 2, 1, 2, 1, 2, 1, 2, 1, 2];
        let sum: u32 = WEIGHTS
            .iter()
            .zip(digits[1..].iter().rev())
            .map(|(w, &d)| w * u32::from(d))
            .sum::<u32>()
            % 9;
        if digits[0] != (9 - sum) as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Japanese My Number (マイナンバー).
    ///
    /// Twelve digits. The check digit is the last, a weighted sum of the first
    /// eleven with weights 6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2 taken modulo
    /// eleven and then modulo ten.
    pub(in crate::id) fn validate_in(s: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(s, &['-']))?;
        checksum::expect_len(&digits, 12)?;
        const WEIGHTS: [u32; 11] = [6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
        let sum: u32 = WEIGHTS
            .iter()
            .zip(&digits[..11])
            .map(|(w, &d)| w * u32::from(d))
            .sum();
        let expected = ((-(sum as i64)) % 11).rem_euclid(11) % 10;
        if u32::from(digits[11]) != expected as u32 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Pakistan: CNIC, the national identity card number.
pub(super) mod pakistan {
    use super::*;

    /// Validates a Pakistani CNIC.
    ///
    /// Thirteen digits. There is no check digit: the last digit encodes gender
    /// (odd for male, even for female) and the first encodes the province, so
    /// both must be in range.
    pub(in crate::id) fn validate_cnic(s: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(s, &['-']))?;
        checksum::expect_len(&digits, 13)?;
        // 1..7 identify a province; 0, 8 and 9 do not.
        if !(1..=7).contains(&digits[0]) {
            return Err(Error::OutOfRange);
        }
        // A gender digit of 0 is unassigned.
        if digits[12] == 0 {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// Taiwan: UBN, the unified business number.
pub(super) mod taiwan {
    use super::*;

    /// Validates a Taiwanese unified business number.
    ///
    /// Eight digits. Each digit is multiplied by weights 1, 2, 1, 2, 1, 2, 4,
    /// 1, the products' decimal digits summed, and the total must be a multiple
    /// of ten. The reference also accepts a total ending in nine when the
    /// seventh digit is seven, because that case is a known rounding quirk.
    pub(in crate::id) fn validate_ubn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(&compact(s, &['-']))?;
        checksum::expect_len(&digits, 8)?;
        const WEIGHTS: [u32; 8] = [1, 2, 1, 2, 1, 2, 4, 1];
        let sum: u32 = WEIGHTS
            .iter()
            .zip(&digits)
            .map(|(w, &d)| w * u32::from(d))
            .map(|product| product / 10 + product % 10)
            .sum();
        let checksum = sum % 10;
        if checksum != 0 && !(checksum == 9 && digits[6] == 7) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn israeli_examples() {
        assert!(israel::validate_idnr("39337423").is_ok());
        assert_eq!(israel::validate_idnr("39337422"), Err(Error::BadChecksum));
    }

    #[test]
    fn thai_examples() {
        assert!(thailand::validate_pin("3100600445635").is_ok());
        assert_eq!(
            thailand::validate_pin("3100600445634"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn thai_moa_examples() {
        assert!(thailand::validate_moa("0994000617721").is_ok());
        assert!(thailand::validate_moa("0-99-4-000-61772-1").is_ok());
        assert_eq!(
            thailand::validate_moa("0-99-4-000-61772-3"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn singapore_examples() {
        for value in ["00192200M", "197401143C", "S16FC0121D", "T01FC6132D"] {
            assert!(singapore::validate_uen(value).is_ok(), "{value}");
        }
        assert_eq!(
            singapore::validate_uen("123456"),
            Err(Error::WrongLength {
                expected: 9,
                found: 6,
            })
        );
    }

    #[test]
    fn malaysian_examples() {
        assert!(malaysia::validate_nric("770305021234").is_ok());
        // A day that does not exist in the month is rejected.
        assert_eq!(
            malaysia::validate_nric("771305021234"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn south_african_examples() {
        assert!(south_africa::validate_idnr("8001015009087").is_ok());
        assert_eq!(
            south_africa::validate_idnr("8001015009088"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn greek_examples() {
        assert!(greece::validate_amka("01013099997").is_ok());
        assert_eq!(
            greece::validate_amka("01013099998"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn japanese_examples() {
        assert!(japan::validate_cn("5-8356-7825-6246").is_ok());
        assert_eq!(
            japan::validate_cn("2-8356-7825-6246"),
            Err(Error::BadChecksum)
        );
        assert!(japan::validate_in("6214 9832 0257").is_ok());
        assert_eq!(
            japan::validate_in("6214 9832 0258"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            japan::validate_in("6214 9832 025X"),
            Err(Error::InvalidCharacter('X'))
        );
    }

    #[test]
    fn pakistani_examples() {
        for value in ["34201-0891231-8", "42201-0397640-8"] {
            assert!(pakistan::validate_cnic(value).is_ok(), "{value}");
        }
        // Province 0 is unassigned, and gender digit 0 is unassigned.
        assert_eq!(
            pakistan::validate_cnic("04201-0397640-8"),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            pakistan::validate_cnic("42201-0397640-0"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn taiwanese_examples() {
        assert!(taiwan::validate_ubn("00501503").is_ok());
        assert_eq!(taiwan::validate_ubn("00501502"), Err(Error::BadChecksum));
        assert_eq!(
            taiwan::validate_ubn("12345"),
            Err(Error::WrongLength {
                expected: 8,
                found: 5,
            })
        );
    }
}
