//! Asian identifiers: India, Turkey, China, Korea and Mexico.

use crate::{Error, Result, checksum};

/// India: Aadhaar, PAN and GSTIN.
pub(super) mod india {
    use super::*;

    /// Validates an Indian Aadhaar number.
    pub(in crate::id) fn validate_aadhaar(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 12)?;
        // Aadhaar never starts with 0 or 1.
        if digits[0] < 2 {
            return Err(Error::OutOfRange);
        }
        if !verhoeff_is_valid(&digits) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The holder types a PAN's fourth character may name.
    ///
    /// The fourth character says who holds the number: an individual, a firm, a
    /// trust and so on. A character outside this set is not a PAN at all.
    const PAN_HOLDER_TYPES: &[u8] = b"ABCFGHLJPTK";

    /// Validates an Indian PAN (Permanent Account Number).
    ///
    /// Ten characters: five letters, four digits and a final letter. The fourth
    /// letter names the holder type and the last four digits are a serial that
    /// may not be all zero. The reference documents no check-digit algorithm,
    /// so the final letter is only required to be a letter.
    pub(in crate::id) fn validate_pan(s: &str) -> Result<()> {
        let s: String = s
            .chars()
            .filter(|c| !matches!(c, ' ' | '-'))
            .flat_map(char::to_uppercase)
            .collect();
        if s.chars().count() != 10 {
            return Err(Error::WrongLength {
                expected: 10,
                found: s.chars().count(),
            });
        }
        let bytes = s.as_bytes();
        // The shape is fixed: letters, then four digits, then a letter.
        if !bytes[..5].iter().all(u8::is_ascii_uppercase)
            || !bytes[5..9].iter().all(u8::is_ascii_digit)
            || !bytes[9].is_ascii_uppercase()
        {
            let bad = bytes
                .iter()
                .find(|b| !b.is_ascii_alphanumeric())
                .copied()
                .unwrap_or(bytes[0]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        if !PAN_HOLDER_TYPES.contains(&bytes[3]) {
            return Err(Error::OutOfRange);
        }
        if &s[5..9] == "0000" {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// The Indian state codes a GSTIN may begin with.
    const GSTIN_STATE_CODES: [&str; 37] = [
        "01", "02", "03", "04", "05", "06", "07", "08", "09", "10", "11", "12", "13", "14", "15",
        "16", "17", "18", "19", "20", "21", "22", "23", "24", "25", "26", "27", "28", "29", "30",
        "31", "32", "33", "34", "35", "36", "37",
    ];

    /// The alphanumeric alphabet the GSTIN check digit is computed over.
    const ALNUM: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

    /// Validates an Indian GSTIN, the goods-and-services-tax number.
    ///
    /// Fifteen characters: a two-digit state code, the holder's PAN, a
    /// registration count, a literal `Z` and a Luhn-mod-36 check digit. The
    /// registration count may not be zero, and the PAN must be valid on its own.
    pub(in crate::id) fn validate_gstin(s: &str) -> Result<()> {
        let s = s.replace(" -", "");
        if s.chars().count() != 15 {
            return Err(Error::WrongLength {
                expected: 15,
                found: s.chars().count(),
            });
        }
        let bytes = s.as_bytes();
        let shaped = bytes[..2].iter().all(u8::is_ascii_digit)
            && bytes[2..7].iter().all(u8::is_ascii_uppercase)
            && bytes[7..11].iter().all(u8::is_ascii_digit)
            && bytes[11].is_ascii_uppercase()
            && bytes[12..].iter().all(u8::is_ascii_alphanumeric);
        if !shaped {
            let bad = bytes
                .iter()
                .find(|b| !b.is_ascii_alphanumeric())
                .copied()
                .unwrap_or(bytes[0]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        if !GSTIN_STATE_CODES.contains(&&s[..2]) || bytes[12] == b'0' || bytes[13] != b'Z' {
            return Err(Error::OutOfRange);
        }
        validate_pan(&s[2..12])?;
        // The check digit is Luhn over the whole number read in base 36.
        let values: Vec<u8> = bytes
            .iter()
            .map(|b| ALNUM.iter().position(|a| a == b).unwrap_or(0) as u8)
            .collect();
        if checksum::luhn_mod_n_checksum(&values, 36) != 0 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Turkey: T.C. Kimlik No.
pub(super) mod turkey {
    use super::*;

    /// Validates a Turkish identification number.
    pub(in crate::id) fn validate_tckn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 11)?;
        if digits[0] == 0 {
            return Err(Error::OutOfRange);
        }
        // The tenth digit is derived from the odd and even positions of the
        // first nine, and the eleventh from the sum of the first ten.
        let odd: u32 = digits[..9].iter().step_by(2).map(|&d| u32::from(d)).sum();
        let even: u32 = digits[1..8].iter().step_by(2).map(|&d| u32::from(d)).sum();
        let tenth = (odd * 7).wrapping_sub(even) % 10;
        if tenth != u32::from(digits[9]) {
            return Err(Error::BadChecksum);
        }
        let first_ten: u32 = digits[..10].iter().map(|&d| u32::from(d)).sum();
        if first_ten % 10 != u32::from(digits[10]) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// China: resident identity card.
pub(super) mod china {
    use super::*;

    /// Validates a Chinese resident identity card number.
    pub(in crate::id) fn validate_ric(s: &str) -> Result<()> {
        checksum::expect_len_str(s, 18)?;
        let bytes = s.as_bytes();
        if !bytes[..17].iter().all(u8::is_ascii_digit) {
            let bad = bytes
                .iter()
                .find(|b| !b.is_ascii_digit())
                .copied()
                .unwrap_or(bytes[0]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        let check = bytes[17];
        if !check.is_ascii_digit() && check != b'X' {
            return Err(Error::InvalidCharacter(char::from(check)));
        }
        // The check character is ISO 7064 MOD 11-2 over the first seventeen
        // digits, using the weights 7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5,
        // 8, 4, 2 and the remainder table 1, 0, X, 9, 8, 7, 6, 5, 4, 3, 2.
        const WEIGHTS: [u32; 17] = [7, 9, 10, 5, 8, 4, 2, 1, 6, 3, 7, 9, 10, 5, 8, 4, 2];
        const TABLE: &[u8] = b"10X98765432";
        let sum: u32 = bytes[..17]
            .iter()
            .zip(WEIGHTS)
            .map(|(&b, w)| u32::from(b - b'0') * w)
            .sum();
        let expected = TABLE[(sum % 11) as usize];
        if expected != check {
            return Err(Error::BadChecksum);
        }
        // The digits in positions seven to fourteen are the birth date. The
        // issuing place is also encoded in the first six digits, but resolving
        // it needs a location table this crate does not carry, so it is left
        // unchecked.
        let year = u32::from(bytes[6] - b'0') * 1000
            + u32::from(bytes[7] - b'0') * 100
            + u32::from(bytes[8] - b'0') * 10
            + u32::from(bytes[9] - b'0');
        let month = u32::from(bytes[10] - b'0') * 10 + u32::from(bytes[11] - b'0');
        let day = u32::from(bytes[12] - b'0') * 10 + u32::from(bytes[13] - b'0');
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

/// South Korea: resident registration number.
pub(super) mod korea {
    use super::*;

    /// Validates a South Korean resident registration number.
    ///
    /// The seventh digit selects the century, and the eighth and ninth are a
    /// place-of-birth code that must not exceed 96.
    pub(in crate::id) fn validate_rrn(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 13)?;
        let yy = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let day = u32::from(digits[4]) * 10 + u32::from(digits[5]);
        let year = match digits[6] {
            1 | 2 | 5 | 6 => yy + 1900,
            3 | 4 | 7 | 8 => yy + 2000,
            _ => yy + 1800,
        };
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        let place = u32::from(digits[7]) * 10 + u32::from(digits[8]);
        if place > 96 {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 12] = [2, 3, 4, 5, 6, 7, 8, 9, 2, 3, 4, 5];
        let sum: u32 = digits[..12]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        let check = (11 - sum % 11) % 10;
        if check as u8 != digits[12] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Mexico: RFC.
pub(super) mod mexico {
    use super::*;

    /// The alphabet used for the RFC check digit, in value order. The enye is a
    /// separate entry after the blank, so the table is over characters rather
    /// than bytes.
    const ALPHABET: [char; 39] = [
        '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H',
        'I', 'J', 'K', 'L', 'M', 'N', '&', 'O', 'P', 'Q', 'R', 'S', 'T', 'U', 'V', 'W', 'X', 'Y',
        'Z', ' ', 'Ñ',
    ];

    /// Validates a Mexican RFC.
    ///
    /// The company form has twelve characters, of which the last three are the
    /// homoclave, and the personal form has thirteen, where the last character
    /// is a check character over the preceding twelve. Only the longer form
    /// carries a verifiable check, so the shorter one is validated structurally.
    pub(in crate::id) fn validate_rfc(s: &str) -> Result<()> {
        // The RFC is written with dashes, underscores or spaces between its
        // parts, and none of them carry meaning.
        let chars: Vec<char> = s
            .chars()
            .filter(|c| !matches!(c, '-' | '_' | ' '))
            .collect();
        // Three shapes exist: a ten-character number assigned to a person
        // before the homoclave was added, a thirteen-character personal number,
        // and a twelve-character company number. All three begin with letters
        // and carry the same six digits of date, but the date starts after four
        // letters for a person and after three for a company.
        let (letters, homoclave) = match chars.len() {
            10 | 13 => (4usize, chars.len() - 10),
            12 => (3usize, 3usize),
            found => {
                return Err(Error::WrongLength {
                    expected: 13,
                    found,
                });
            }
        };
        // The leading letters may include the ampersand and the enye, both of
        // which appear in real names.
        let name_ok = |c: &char| c.is_ascii_uppercase() || *c == '&' || *c == 'Ñ';
        if let Some(bad) = chars[..letters].iter().find(|c| !name_ok(c)) {
            return Err(Error::InvalidCharacter(*bad));
        }
        if let Some(bad) = chars[letters..letters + 6]
            .iter()
            .find(|c| !c.is_ascii_digit())
        {
            return Err(Error::InvalidCharacter(*bad));
        }
        if let Some(bad) = chars[letters + 6..]
            .iter()
            .find(|c| !c.is_ascii_alphanumeric())
        {
            return Err(Error::InvalidCharacter(*bad));
        }
        let _ = homoclave;
        // The six digits are a date. The century is not stored, so it is taken
        // as 2000. Unlike the Indonesian NIK this is a real calendar check: a
        // date that does not exist is not a valid RFC.
        let digit = |i: usize| chars[i] as u32 - '0' as u32;
        let year = 2000 + digit(letters) * 10 + digit(letters + 1);
        let month = digit(letters + 2) * 10 + digit(letters + 3);
        let day = digit(letters + 4) * 10 + digit(letters + 5);
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        // The four letters of a personal RFC are a name, and the tax office
        // refuses a short list of them because they read as profanity.
        if letters == 4 {
            let name: String = chars[..4].iter().collect();
            if RFC_NAME_BLACKLIST.contains(&name.as_str()) {
                return Err(Error::OutOfRange);
            }
        }
        // The ten-character form predates the homoclave, so it has no check
        // character to verify.
        if chars.len() == 10 {
            return Ok(());
        }
        // The check character is computed over twelve characters. A company
        // number has only eleven, so it is padded on the left with a blank,
        // which is itself a character in the alphabet.
        let mut window = [' '; 12];
        let offset = 12 - (chars.len() - 1);
        window[offset..].copy_from_slice(&chars[..chars.len() - 1]);
        let mut sum = 0u32;
        for (i, c) in window.iter().enumerate() {
            let Some(index) = ALPHABET.iter().position(|a| a == c) else {
                return Err(Error::InvalidCharacter(*c));
            };
            sum += index as u32 * (13 - i as u32);
        }
        let expected = ALPHABET[((11 - sum % 11) % 11) as usize];
        if expected != chars[chars.len() - 1] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Four-letter names the Mexican tax office does not issue.
    const RFC_NAME_BLACKLIST: [&str; 41] = [
        "BUEI", "BUEY", "CACA", "CACO", "CAGA", "CAGO", "CAKA", "CAKO", "COGE", "COJA", "COJE",
        "COJI", "COJO", "CULO", "FETO", "GUEY", "JOTO", "KACA", "KACO", "KAGA", "KAGO", "KAKA",
        "KOGE", "KOJO", "KULO", "MAME", "MAMO", "MEAR", "MEAS", "MEON", "MION", "MOCO", "MULA",
        "PEDA", "PEDO", "PENE", "PUTA", "PUTO", "QULO", "RATA", "RUIN",
    ];

    /// The two-letter codes of the Mexican states a CURP may name.
    const CURP_STATES: [&str; 33] = [
        "AS", "BC", "BS", "CC", "CH", "CL", "CM", "CS", "DF", "DG", "GR", "GT", "HG", "JC", "MC",
        "MN", "MS", "NE", "NL", "NT", "OC", "PL", "QR", "QT", "SL", "SP", "SR", "TC", "TL", "TS",
        "VZ", "YN", "ZS",
    ];

    /// Four-letter name fragments the CURP refuses, the same list the RFC
    /// uses.
    const CURP_NAME_BLACKLIST: [&str; 81] = [
        "BACA", "BAKA", "BUEI", "BUEY", "CACA", "CACO", "CAGA", "CAGO", "CAKA", "CAKO", "COGE",
        "COGI", "COJA", "COJE", "COJI", "COJO", "COLA", "CULO", "FALO", "FETO", "GETA", "GUEI",
        "GUEY", "JETA", "JOTO", "KACA", "KACO", "KAGA", "KAGO", "KAKA", "KAKO", "KOGE", "KOGI",
        "KOJA", "KOJE", "KOJI", "KOJO", "KOLA", "KULO", "LILO", "LOCA", "LOCO", "LOKA", "LOKO",
        "MAME", "MAMO", "MEAR", "MEAS", "MEON", "MIAR", "MION", "MOCO", "MOKO", "MULA", "MULO",
        "NACA", "NACO", "PEDA", "PEDO", "PENE", "PIPI", "PITO", "POPO", "PUTA", "PUTO", "QULO",
        "RATA", "ROBA", "ROBE", "ROBO", "RUIN", "SENO", "TETA", "VACA", "VAGA", "VAGO", "VAKA",
        "VUEI", "VUEY", "WUEI", "WUEY",
    ];

    /// The alphabet the CURP check digit is computed over, with the enye
    /// between the ampersand and the O.
    const CURP_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMN&OPQRSTUVWXYZ";

    /// Validates a Mexican CURP, the population registry code.
    ///
    /// Eighteen characters: four name letters, six digits of birth date, a sex
    /// letter, a two-letter state, three consonants and a check digit. The
    /// century is taken from the seventeenth character, which is a digit before
    /// 2000 and a letter after.
    pub(in crate::id) fn validate_curp(s: &str) -> Result<()> {
        let s: String = s
            .chars()
            .filter(|c| !matches!(c, '-' | '_' | ' '))
            .flat_map(char::to_uppercase)
            .collect();
        if s.chars().count() != 18 {
            return Err(Error::WrongLength {
                expected: 18,
                found: s.chars().count(),
            });
        }
        let bytes = s.as_bytes();
        let shaped = bytes[..4].iter().all(u8::is_ascii_uppercase)
            && bytes[4..10].iter().all(u8::is_ascii_digit)
            && bytes[10..16].iter().all(u8::is_ascii_uppercase)
            && bytes[16].is_ascii_alphanumeric()
            && bytes[17].is_ascii_digit();
        if !shaped {
            let bad = bytes
                .iter()
                .find(|b| !b.is_ascii_alphanumeric())
                .copied()
                .unwrap_or(bytes[0]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        if CURP_NAME_BLACKLIST.contains(&&s[..4]) {
            return Err(Error::OutOfRange);
        }
        // The birth date, with the century read off the seventeenth character.
        let digit = |i: usize| u32::from(bytes[i] - b'0');
        let yy = digit(4) * 10 + digit(5);
        let year = if bytes[16].is_ascii_digit() {
            yy + 1900
        } else {
            yy + 2000
        };
        let month = digit(6) * 10 + digit(7);
        let day = digit(8) * 10 + digit(9);
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        // The eleventh character is the sex: H for a man, M for a woman.
        if !matches!(bytes[10], b'H' | b'M') {
            return Err(Error::OutOfRange);
        }
        if !CURP_STATES.contains(&&s[11..13]) {
            return Err(Error::OutOfRange);
        }
        // The check digit is a weighted sum over the first seventeen characters.
        let sum: u32 = bytes[..17]
            .iter()
            .enumerate()
            .map(|(i, b)| {
                let value = CURP_ALPHABET.iter().position(|a| a == b).unwrap_or(0) as u32;
                value * (18 - i as u32)
            })
            .sum();
        if ((10 - sum % 10) % 10) as u8 != bytes[17] - b'0' {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// The Verhoeff tables, shared by Aadhaar and the public check-digit helper.
pub(super) const VERHOEFF_D: [[u8; 10]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
    [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
    [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
    [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
    [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
    [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
    [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
    [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
    [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
];

/// The Verhoeff permutation table.
pub(super) const VERHOEFF_P: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
    [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
    [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
    [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];

/// The Verhoeff multiplicative inverse table.
pub(super) const VERHOEFF_INV: [u8; 10] = [0, 4, 3, 2, 1, 5, 6, 7, 8, 9];

/// Whether the Verhoeff check over `digits` succeeds.
pub(super) fn verhoeff_is_valid(digits: &[u8]) -> bool {
    let mut c = 0u8;
    for (i, &d) in digits.iter().rev().enumerate() {
        c = VERHOEFF_D[c as usize][VERHOEFF_P[i % 8][d as usize] as usize];
    }
    c == 0
}

/// Appends the Verhoeff check digit to a payload.
///
/// Exposed so callers can generate numbers, and used by the tests.
#[must_use]
pub fn verhoeff_check_digit(payload: &[u8]) -> u8 {
    let mut c = 0u8;
    for (i, &d) in payload.iter().rev().enumerate() {
        c = VERHOEFF_D[c as usize][VERHOEFF_P[(i + 1) % 8][d as usize] as usize];
    }
    VERHOEFF_INV[c as usize]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aadhaar_examples() {
        // Verhoeff check digits computed with the crate's own routine.
        let payload = [2, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2];
        let check = crate::id::verhoeff_check_digit(&payload);
        let mut digits = payload.to_vec();
        digits.push(check);
        let text: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
        assert!(india::validate_aadhaar(&text).is_ok(), "{text}");
    }

    #[test]
    fn aadhaar_rejects_a_leading_zero_or_one() {
        let payload = [1, 3, 4, 5, 6, 7, 8, 9, 0, 1, 2];
        let check = crate::id::verhoeff_check_digit(&payload);
        let mut digits = payload.to_vec();
        digits.push(check);
        let text: String = digits.iter().map(|d| char::from(b'0' + d)).collect();
        assert_eq!(india::validate_aadhaar(&text), Err(Error::OutOfRange));
    }

    #[test]
    fn turkish_example() {
        assert!(turkey::validate_tckn("17291716060").is_ok());
        assert_eq!(
            turkey::validate_tckn("17291716050"),
            Err(Error::BadChecksum)
        );
        assert!(turkey::validate_tckn("07291716092").is_err());
    }

    #[test]
    fn chinese_example() {
        assert!(china::validate_ric("360426199101010071").is_ok());
        // A wrong final digit is rejected.
        assert_eq!(
            china::validate_ric("360426199101010072"),
            Err(Error::BadChecksum)
        );
        // An X check character is accepted where the table calls for it.
        assert!(china::validate_ric("11010519491231002X").is_ok());
    }

    #[test]
    fn korean_examples() {
        assert!(korea::validate_rrn("9710139019902").is_ok());
        assert_eq!(
            korea::validate_rrn("9710139019903"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn mexican_examples() {
        // The thirteen-character personal form carries a check character.
        for value in ["GODE561231GR8", "VACE460910SX5"] {
            assert!(mexico::validate_rfc(value).is_ok(), "{value}");
        }
        // The twelve-character company form also carries a check character,
        // computed over one fewer character.
        assert!(mexico::validate_rfc("MAB9307148T4").is_ok());
        // The company form is padded on the left with a blank before the
        // weights are applied, which is not the same as starting the weights
        // one position later.
        assert!(mexico::validate_rfc("COMG-600703").is_ok());
        assert!(mexico::validate_rfc("GODE 561231 GR8").is_ok());
        // The strict mode of the reference rejects this one, and so do we.
        assert_eq!(
            mexico::validate_rfc("VACE-460910-SX6"),
            Err(Error::BadChecksum)
        );
        // A wrong check character on either form is rejected.
        assert_eq!(
            mexico::validate_rfc("GODE561231GR9"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            mexico::validate_rfc("MAB9307148T5"),
            Err(Error::BadChecksum)
        );
        // A month of 13 is impossible.
        assert_eq!(
            mexico::validate_rfc("GODE561331GR8"),
            Err(Error::OutOfRange)
        );
    }

    /// The enye is a character in the RFC name alphabet, not a byte of a
    /// two-byte sequence. Reading the number byte by byte refused every name
    /// that contains one.
    #[test]
    fn mexican_names_may_contain_an_enye() {
        for value in [
            "MUÑ110218LJ6",
            "AÑE9902224Z4",
            "BAÑ930616R66",
            "NIÑ7409256U1",
        ] {
            assert!(mexico::validate_rfc(value).is_ok(), "{value}");
        }
        // The enye is only allowed in the name, not in the homoclave.
        assert!(mexico::validate_rfc("GODE561231GÑ8").is_err());
    }
}
