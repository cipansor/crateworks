//! South-East European identifiers: Serbia, Montenegro, North Macedonia,
//! Slovenia, Romania and Belarus.
//!
//! These are the tax and company numbers of the countries between the Adriatic
//! and the Black Sea. Each is a weighted checksum over a fixed-length number,
//! so each is a small function, and each is cross-checked against
//! `python-stdnum` by the fixture in `tests/fixtures/stdnum_vectors.tsv`.

use crate::{Error, Result, checksum};

/// Serbia: PIB, the tax identification number.
pub(super) mod serbia {
    use super::*;

    /// Validates a Serbian PIB.
    ///
    /// Nine digits. The check is ISO 7064 MOD 11,10: the running value starts
    /// at five, is doubled mod 11 and the next digit added mod 10, and a valid
    /// number ends at one. A zero intermediate value is replaced by ten, which
    /// is what the scheme requires and what the reference writes as
    /// `(check or 10)`.
    pub(in crate::id) fn validate_pib(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 9)?;
        let mut check = 5u32;
        for &d in &digits {
            let base = if check == 0 { 10 } else { check };
            check = ((base * 2) % 11 + u32::from(d)) % 10;
        }
        if check != 1 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Montenegro: PIB, the tax number.
pub(super) mod montenegro {
    use super::*;

    /// Validates a Montenegrin PIB.
    ///
    /// Eight digits. The check digit is the negation of the weighted sum,
    /// reduced mod 11 and then mod 10, so a remainder of ten collapses to zero.
    pub(in crate::id) fn validate_pib(s: &str) -> Result<()> {
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 8)?;
        const WEIGHTS: [u32; 7] = [8, 7, 6, 5, 4, 3, 2];
        let sum: u32 = digits[..7]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| w * u32::from(d))
            .sum();
        if digits[7] != ((-(sum as i64)).rem_euclid(11) % 10) as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// North Macedonia: ЕДБ, the tax number.
pub(super) mod north_macedonia {
    use super::*;

    /// Validates a North Macedonian ЕДБ number.
    ///
    /// Thirteen digits, optionally carrying an `MK` prefix in either the Latin
    /// or the Cyrillic alphabet. The check digit is the negated weighted sum
    /// mod 11, then mod 10.
    pub(in crate::id) fn validate_edb(s: &str) -> Result<()> {
        let s = strip_mk_prefix(s);
        let digits = checksum::digits_only(&s)?;
        checksum::expect_len(&digits, 13)?;
        const WEIGHTS: [u32; 12] = [7, 6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
        let sum: u32 = digits[..12]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| w * u32::from(d))
            .sum();
        if digits[12] != ((-(sum as i64)).rem_euclid(11) % 10) as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Removes the optional `MK` prefix, in either alphabet.
    ///
    /// The Cyrillic form is `МК`, which is two code points and not the same
    /// bytes as the Latin one, so both are checked explicitly.
    fn strip_mk_prefix(s: &str) -> String {
        let upper = s.trim().to_uppercase();
        for prefix in ["MK", "МК"] {
            if let Some(rest) = upper.strip_prefix(prefix) {
                return rest.to_string();
            }
        }
        upper
    }
}

/// Slovenia: DDV, the VAT number, and the matična številka company register.
pub(super) mod slovenia {
    use super::*;

    /// Validates a Slovenian VAT number.
    ///
    /// Eight digits, optionally prefixed `SI`. The check digit is `11 - sum
    /// mod 11`, and a result of ten is written as zero. A leading zero is
    /// invalid, which the reference rejects as a format error.
    pub(in crate::id) fn validate_ddv(s: &str) -> Result<()> {
        let s = s.strip_prefix("SI").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 8)?;
        if digits[0] == 0 {
            return Err(Error::InvalidCharacter('0'));
        }
        const WEIGHTS: [u32; 7] = [8, 7, 6, 5, 4, 3, 2];
        let sum: u32 = digits[..7]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| w * u32::from(d))
            .sum();
        let check = 11 - sum % 11;
        let expected = if check == 10 { 0 } else { check };
        if u32::from(digits[7]) != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Slovenian matična številka, the business register number.
    ///
    /// Seven digits whose last is a check digit, optionally followed by a
    /// three-character unit suffix. A ten-character number that ends in `000`
    /// is the main unit and is treated as the seven-digit form; any other
    /// suffix must be one alphanumeric character and two digits. The check
    /// digit is eleven minus the weighted sum modulo eleven, and a remainder of
    /// zero leaves no digit, so such a number cannot be valid.
    pub(in crate::id) fn validate_maticna(s: &str) -> Result<()> {
        let s: String = s
            .chars()
            .filter(|c| !matches!(c, '.' | ' '))
            .flat_map(char::to_uppercase)
            .collect();
        let s = if s.len() == 10 && s.ends_with("000") {
            &s[..7]
        } else {
            &s[..]
        };
        if !matches!(s.len(), 7 | 10) {
            return Err(Error::WrongLength {
                expected: 7,
                found: s.len(),
            });
        }
        let bytes = s.as_bytes();
        if !bytes[..6].iter().all(u8::is_ascii_digit) {
            let bad = bytes
                .iter()
                .find(|b| !b.is_ascii_digit())
                .copied()
                .unwrap_or(bytes[0]);
            return Err(Error::InvalidCharacter(char::from(bad)));
        }
        // A ten-character number carries a unit suffix after the check digit.
        if s.len() == 10 {
            let tail = &bytes[7..];
            if !tail[0].is_ascii_alphanumeric()
                || !tail[1].is_ascii_digit()
                || !tail[2].is_ascii_digit()
            {
                return Err(Error::InvalidCharacter(char::from(tail[0])));
            }
        }
        const WEIGHTS: [u32; 6] = [7, 6, 5, 4, 3, 2];
        let sum: u32 = bytes[..6]
            .iter()
            .zip(WEIGHTS)
            .map(|(&b, w)| u32::from(b - b'0') * w)
            .sum();
        let remainder = (11 - sum % 11) % 11;
        // A remainder of zero has no single-digit form; a check position that
        // is not a digit also fails the comparison rather than the shape.
        if remainder == 0 || Some(remainder % 10) != bytes[6].checked_sub(b'0').map(u32::from) {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Validates a Slovenian EMŠO (enotna matična številka občana), the unique
    /// master citizen number.
    ///
    /// Thirteen digits: seven encode the date of birth, three the region and
    /// serial, one the sex, and the last is a check digit. The date's year is
    /// completed by the same 800 rule the reference uses, and the check digit
    /// is a weighted sum negated modulo eleven, then modulo ten.
    pub(in crate::id) fn validate_emso(s: &str) -> Result<()> {
        let cleaned = crate::id::clean(s, &[' ']);
        let digits = checksum::digits_only(&cleaned)?;
        checksum::expect_len(&digits, 13)?;
        let day = u32::from(digits[0]) * 10 + u32::from(digits[1]);
        let month = u32::from(digits[2]) * 10 + u32::from(digits[3]);
        let year = u32::from(digits[4]) * 100 + u32::from(digits[5]) * 10 + u32::from(digits[6]);
        let year = if year < 800 { year + 2000 } else { year + 1000 };
        if !checksum::is_real_date(year, month, day) {
            return Err(Error::OutOfRange);
        }
        const WEIGHTS: [u32; 12] = [7, 6, 5, 4, 3, 2, 7, 6, 5, 4, 3, 2];
        let total: u32 = digits[..12]
            .iter()
            .zip(WEIGHTS)
            .map(|(&d, w)| u32::from(d) * w)
            .sum();
        let expected = (-(i64::from(total))).rem_euclid(11) % 10;
        if u32::from(digits[12]) != expected as u32 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Romania: CF, the VAT number, which may also be a CNP.
pub(super) mod romania {
    use super::*;

    /// Validates a Romanian VAT number.
    ///
    /// Two shapes: a CUI/CIF of two to ten digits with a weighted check digit,
    /// or a thirteen-digit CNP personal code, which is checked as a birth
    /// number. The reference accepts the `RO` prefix, and it is stripped here
    /// too.
    pub(in crate::id) fn validate_cf(s: &str) -> Result<()> {
        let s = s.strip_prefix("RO").unwrap_or(s);
        let digits = checksum::digits_only(s)?;
        if digits.len() == 13 {
            return crate::id::europe::romania::validate_cnp(s);
        }
        validate_cui_digits(&digits)
    }

    /// Validates a Romanian CUI/CIF, the company identifier.
    ///
    /// The same rule as the VAT form without the `RO` prefix.
    pub(in crate::id) fn validate_cui(s: &str) -> Result<()> {
        let cleaned = crate::id::clean(s, &[' ', '-']);
        let s = cleaned.strip_prefix("RO").unwrap_or(&cleaned);
        let digits = checksum::digits_only(s)?;
        validate_cui_digits(&digits)
    }

    /// Validates the digits of a Romanian CUI/CIF, the company identifier.
    ///
    /// Two to ten digits with a weighted check digit, shared by the VAT form
    /// and the plain company identifier.
    pub(in crate::id) fn validate_cui_digits(digits: &[u8]) -> Result<()> {
        if !(2..=10).contains(&digits.len()) {
            return Err(Error::WrongLength {
                expected: 10,
                found: digits.len(),
            });
        }
        // The CUI payload is left-padded with zeroes to nine digits, then
        // weighted, and the check digit is that sum times ten mod 11 mod 10. A
        // leading zero is not a valid CUI, which the reference rejects.
        if digits[0] == 0 {
            return Err(Error::InvalidCharacter('0'));
        }
        const WEIGHTS: [u32; 9] = [7, 5, 3, 2, 1, 7, 5, 3, 2];
        let payload = &digits[..digits.len() - 1];
        let pad = 9usize.saturating_sub(payload.len());
        let sum: u32 = std::iter::repeat_n(0u32, pad)
            .chain(payload.iter().map(|&d| u32::from(d)))
            .zip(WEIGHTS)
            .map(|(d, w)| d * w)
            .sum();
        let expected = sum * 10 % 11 % 10;
        if u32::from(digits[digits.len() - 1]) != expected {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The counties a Romanian trade-register number may name, as the reference
    /// lists them.
    const ONRC_COUNTIES: &[u32] = &[
        1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25,
        26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 51, 52,
    ];

    /// Validates a Romanian ONRC number, the trade-register identifier.
    ///
    /// Two forms share the leading letter `J`, `F` or `C` for the kind of
    /// company. The old form is `YNN/XXXX/AAAA`: a county, a serial and a year,
    /// with the year between 1990 and 2024. The new form is fourteen digits
    /// `YAAAAXXXXXXJJC`, where the year decides which counties are allowed and
    /// the last digit is a check over the rest. The reference chooses the form
    /// by the presence of a slash.
    pub(in crate::id) fn validate_onrc(input: &str) -> Result<()> {
        let compacted = compact_onrc(input);
        let Some(&kind) = compacted.as_bytes().first() else {
            return Err(Error::Empty);
        };
        if !matches!(kind, b'J' | b'F' | b'C') {
            return Err(Error::InvalidCharacter(char::from(kind)));
        }
        if compacted.contains('/') {
            return validate_onrc_old(&compacted);
        }
        validate_onrc_new(&compacted)
    }

    /// Reduces an ONRC to the reference's canonical form: separators become
    /// slashes, the slash after the leading letter and the one after a
    /// single-digit county are removed, and a trailing full date is cut back to
    /// its year.
    fn compact_onrc(input: &str) -> String {
        let mut out = String::with_capacity(input.len());
        for c in input.chars() {
            let c = match c {
                '\u{2010}'..='\u{2015}' | '\u{2212}' | '\u{fe58}' | '\u{fe63}' | '\u{ff0d}' => '-',
                '\u{2000}'..='\u{200b}' | '\u{202f}' | '\u{205f}' | '\u{3000}' | '\u{a0}' => ' ',
                _ => c,
            };
            if matches!(c, ' ' | '/' | '\\' | '-') {
                // A run of separators collapses to a single slash.
                if !out.ends_with('/') {
                    out.push('/');
                }
            } else {
                out.extend(c.to_uppercase());
            }
        }
        // Strip a slash left at either end, which came from surrounding space.
        let trimmed = out.trim_matches('/').to_string();
        let mut number = trimmed;
        // Remove the slash between the leading letter and the county digits.
        if number.as_bytes().get(1) == Some(&b'/') {
            number.remove(1);
        }
        // Normalise a single-digit county to two digits.
        if number.as_bytes().get(2) == Some(&b'/') {
            number.insert(1, '0');
        }
        // A trailing day and month before the year are dropped.
        if let Some(year_only) = strip_onrc_full_date(&number) {
            number = year_only;
        }
        number
    }

    /// If `number` ends `.../DD.MM.YYYY`, returns it with the day and month
    /// removed, as the reference does for the old form.
    fn strip_onrc_full_date(number: &str) -> Option<String> {
        let (head, tail) = number.rsplit_once('/')?;
        let mut parts = tail.split('.');
        let day = parts.next()?;
        let month = parts.next()?;
        let year = parts.next()?;
        if parts.next().is_some()
            || day.len() != 2
            || month.len() != 2
            || year.len() != 4
            || !day.bytes().all(|b| b.is_ascii_digit())
            || !month.bytes().all(|b| b.is_ascii_digit())
            || !year.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        // The head must itself look like `LETTER digits / digits`.
        let (letter, rest) = head.split_at(1);
        let (num1, num2) = rest.split_once('/')?;
        if !letter.bytes().all(|b| b.is_ascii_uppercase())
            || num1.is_empty()
            || num2.is_empty()
            || !num1.bytes().all(|b| b.is_ascii_digit())
            || !num2.bytes().all(|b| b.is_ascii_digit())
        {
            return None;
        }
        Some(format!("{head}/{year}"))
    }

    /// Validates the old `YNN/XXXX/AAAA` ONRC form.
    fn validate_onrc_old(number: &str) -> Result<()> {
        let mut parts = number[1..].split('/');
        let (Some(county), Some(serial), Some(year)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(Error::InvalidPrefix);
        };
        if parts.next().is_some() || serial.is_empty() || county.is_empty() {
            return Err(Error::InvalidPrefix);
        }
        if !county.bytes().all(|b| b.is_ascii_digit())
            || !serial.bytes().all(|b| b.is_ascii_digit())
            || !year.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(Error::InvalidPrefix);
        }
        if serial.len() > 5 || year.len() != 4 {
            return Err(Error::WrongLength {
                expected: 4,
                found: year.len(),
            });
        }
        if !(1..=2).contains(&county.len()) {
            return Err(Error::OutOfRange);
        }
        let county: u32 = county.parse().map_err(|_| Error::OutOfRange)?;
        if !ONRC_COUNTIES.contains(&county) {
            return Err(Error::OutOfRange);
        }
        let year: u32 = year.parse().map_err(|_| Error::OutOfRange)?;
        // The old form stopped being issued after 2024.
        if !(1990..=2024).contains(&year) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Validates the new fourteen-digit `YAAAAXXXXXXJJC` ONRC form.
    fn validate_onrc_new(number: &str) -> Result<()> {
        if number.len() != 14 {
            return Err(Error::WrongLength {
                expected: 14,
                found: number.len(),
            });
        }
        if !number.as_bytes()[1..].iter().all(u8::is_ascii_digit) {
            return Err(Error::InvalidPrefix);
        }
        let year: u32 = number[1..5].parse().map_err(|_| Error::OutOfRange)?;
        if !(1990..=checksum::current_year()).contains(&year) {
            return Err(Error::OutOfRange);
        }
        // The registration year decides which county codes are allowed: any
        // county before 2024, a county or `00` in 2024, and only `00` after.
        let county: u32 = number[11..13].parse().map_err(|_| Error::OutOfRange)?;
        let county_ok = if year < 2024 {
            ONRC_COUNTIES.contains(&county)
        } else if year == 2024 {
            county == 0 || ONRC_COUNTIES.contains(&county)
        } else {
            county == 0
        };
        if !county_ok {
            return Err(Error::OutOfRange);
        }
        // The check digit replaces the leading letter with its code point
        // modulo ten, then sums every digit but the last.
        let code = u32::from(number.as_bytes()[0]) % 10;
        let sum: u32 = std::iter::once(code)
            .chain(number[1..13].bytes().map(|b| u32::from(b - b'0')))
            .sum();
        if u32::from(number.as_bytes()[13] - b'0') != sum % 10 {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }
}

/// Belarus: УНП, the payer account number.
pub(super) mod belarus {
    use super::*;

    /// The Cyrillic letters that stand in for Latin ones in a УНП, in the
    /// order the reference maps them.
    const CYRILLIC: [char; 10] = ['А', 'В', 'Е', 'К', 'М', 'Н', 'О', 'Р', 'С', 'Т'];
    /// Their Latin equivalents, position for position.
    const LATIN: [char; 10] = ['A', 'B', 'E', 'K', 'M', 'H', 'O', 'P', 'C', 'T'];
    /// The letters the second character may be drawn from.
    const SECOND_LETTERS: &str = "ABCEHKMOPT";

    /// Validates a Belarusian УНП.
    ///
    /// Nine characters: two identifying characters (digits, or letters that
    /// encode as their index in the Latin alphabet) followed by seven digits,
    /// with a weighted check digit over the first eight.
    pub(in crate::id) fn validate_unp(s: &str) -> Result<()> {
        // The prefix is removed before transliteration, because it is written
        // in the Cyrillic alphabet and those letters are not in the map below.
        // The reference does the same, and it is what makes `УНП MA1953684`
        // valid rather than a length error.
        let s = s.trim().to_uppercase();
        let s = ["УНП", "UNP"]
            .iter()
            .find_map(|p| s.strip_prefix(p))
            .unwrap_or(&s)
            .to_string();
        let s = transliterate(&s);
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: chars.len(),
            });
        }
        if !chars[2..].iter().all(char::is_ascii_digit) {
            return Err(Error::InvalidCharacter(
                *chars[2..]
                    .iter()
                    .find(|c| !c.is_ascii_digit())
                    .expect("a non-digit was found"),
            ));
        }
        let head_is_digits = chars[..2].iter().all(char::is_ascii_digit);
        if !head_is_digits && !chars[..2].iter().all(|c| SECOND_LETTERS.contains(*c)) {
            return Err(Error::InvalidCharacter(chars[0]));
        }
        if !"1234567ABCEHKM".contains(chars[0]) {
            return Err(Error::OutOfRange);
        }
        // The first two characters are read as values, then the next six
        // digits, and the weighted sum over those eight gives the check digit.
        // The first character is read in the full digit-then-letter alphabet,
        // but the second is read in the narrower set of letters a УНП may use,
        // which is not the same numbering: `M` is 22 in the first and 6 in the
        // second. The reference makes exactly this distinction.
        const WEIGHTS: [u32; 8] = [29, 23, 19, 17, 13, 7, 5, 3];
        let values = [
            chars[0]
                .to_digit(36)
                .ok_or(Error::InvalidCharacter(chars[0]))?,
            char_value(chars[1])?,
            chars[2].to_digit(10).expect("checked above"),
            chars[3].to_digit(10).expect("checked above"),
            chars[4].to_digit(10).expect("checked above"),
            chars[5].to_digit(10).expect("checked above"),
            chars[6].to_digit(10).expect("checked above"),
            chars[7].to_digit(10).expect("checked above"),
        ];
        let sum: u32 = values.iter().zip(WEIGHTS).map(|(&v, w)| v * w).sum();
        let check = sum % 11;
        // A two-digit check is rejected rather than reduced, which is what the
        // reference does.
        if check > 9 {
            return Err(Error::BadChecksum);
        }
        if chars[8] != char::from_digit(check, 10).expect("check is below ten") {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// Replaces the Cyrillic letters a УНП may be written with by their Latin
    /// equivalents.
    fn transliterate(s: &str) -> String {
        s.trim()
            .to_uppercase()
            .chars()
            .map(|c| {
                CYRILLIC
                    .iter()
                    .position(|&x| x == c)
                    .map_or(c, |i| LATIN[i])
            })
            .collect()
    }

    /// The numeric value of one identifying character: a digit as itself, a
    /// letter as its index in the alphabet.
    fn char_value(c: char) -> Result<u32> {
        if let Some(d) = c.to_digit(10) {
            return Ok(d);
        }
        SECOND_LETTERS
            .find(c)
            .map(|i| i as u32)
            .ok_or(Error::InvalidCharacter(c))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::normalize;

    #[test]
    fn serbian_example() {
        // From the reference's own doctest.
        assert!(serbia::validate_pib("101134702").is_ok());
        assert_eq!(serbia::validate_pib("101134703"), Err(Error::BadChecksum));
    }

    #[test]
    fn montenegrin_example() {
        assert!(montenegro::validate_pib("02655284").is_ok());
        assert_eq!(
            montenegro::validate_pib("02655283"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn north_macedonian_example_accepts_both_alphabets() {
        assert!(north_macedonia::validate_edb("4030000375897").is_ok());
        assert!(
            north_macedonia::validate_edb(&normalize("MK4057009501106")).is_ok(),
            "the Latin prefix"
        );
        assert!(
            north_macedonia::validate_edb(&normalize("\u{41c}\u{41a} 4020990116747")).is_ok(),
            "the Cyrillic prefix is a different code point, not the same bytes"
        );
        assert_eq!(
            north_macedonia::validate_edb("4030000375890"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn slovenian_example() {
        assert!(slovenia::validate_ddv(&normalize("SI 5022 3054")).is_ok());
        assert_eq!(
            slovenia::validate_ddv(&normalize("SI 50223055")),
            Err(Error::BadChecksum)
        );
        assert!(
            slovenia::validate_ddv("05022305").is_err(),
            "a leading zero is not a valid number"
        );
    }

    #[test]
    fn romanian_example_and_cnp_fallback() {
        assert!(romania::validate_cf(&normalize("RO 185 472 90")).is_ok());
        assert!(romania::validate_cf("18547290").is_ok());
        // A thirteen-digit CF is a CNP and is checked as one.
        assert!(romania::validate_cf("1630615123457").is_ok());
        assert_eq!(
            romania::validate_cf("1630615123458"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn belarusian_example_and_cyrillic_prefix() {
        assert!(belarus::validate_unp("MA1953684").is_ok());
        assert!(
            belarus::validate_unp(&normalize("\u{423}\u{41d}\u{41f} MA1953684")).is_ok(),
            "the prefix is stripped before transliteration"
        );
        assert_eq!(belarus::validate_unp("MA1953685"), Err(Error::BadChecksum));
    }

    #[test]
    fn the_first_and_second_belarusian_characters_use_different_alphabets() {
        // `M` is 22 in the digit-then-letter alphabet the first character is
        // read in, but 6 in the letter set the second one uses. Reading both
        // the same way is what the fixture caught.
        assert!(belarus::validate_unp("MA1953684").is_ok());
        assert_eq!(
            belarus::validate_unp("AM1953684"),
            Err(Error::BadChecksum),
            "read as the second character's table, A is 0, not 10"
        );
        assert_eq!(belarus::validate_unp("AA1953684"), Err(Error::BadChecksum));
    }
}
