//! IBAN validation.
//!
//! An IBAN is validated in three independent ways: the country-specific
//! length and structure, the ISO 7064 MOD-97-10 check over the rearranged
//! characters, and the national check digit a few countries carry inside the
//! BBAN. The MOD-97 check alone would accept a number of the wrong length for
//! its country, or with a letter where the registry requires a digit.

use crate::checksum;
use crate::{Error, Result};

/// An IBAN.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Iban(String);

/// Country code and expected IBAN length, from the ISO 13616 registry.
///
/// Only members of the registry appear here. An IBAN whose country is absent
/// is rejected as [`Error::UnknownCountry`] rather than guessed at.
static IBAN_LENGTHS: &[(&str, usize)] = &[
    ("AD", 24),
    ("AE", 23),
    ("AL", 28),
    ("AT", 20),
    ("AZ", 28),
    ("BA", 20),
    ("BE", 16),
    ("BG", 22),
    ("BH", 22),
    ("BI", 27),
    ("BR", 29),
    ("BY", 28),
    ("CH", 21),
    ("CR", 22),
    ("CY", 28),
    ("CZ", 24),
    ("DE", 22),
    ("DK", 18),
    ("DJ", 27),
    ("DO", 28),
    ("EE", 20),
    ("EG", 29),
    ("ES", 24),
    ("FI", 18),
    ("FK", 18),
    ("FO", 18),
    ("FR", 27),
    ("GB", 22),
    ("GE", 22),
    ("GI", 23),
    ("GL", 18),
    ("GR", 27),
    ("GT", 28),
    ("HN", 28),
    ("HR", 21),
    ("HU", 28),
    ("IE", 22),
    ("IL", 23),
    ("IQ", 23),
    ("IS", 26),
    ("IT", 27),
    ("JO", 30),
    ("KW", 30),
    ("KZ", 20),
    ("LB", 28),
    ("LC", 32),
    ("LI", 21),
    ("LT", 20),
    ("LU", 20),
    ("LV", 21),
    ("LY", 25),
    ("MC", 27),
    ("MD", 24),
    ("ME", 22),
    ("MK", 19),
    ("MN", 20),
    ("MR", 27),
    ("MT", 31),
    ("MU", 30),
    ("NI", 28),
    ("NL", 18),
    ("NO", 15),
    ("OM", 23),
    ("PK", 24),
    ("PL", 28),
    ("PS", 29),
    ("PT", 25),
    ("QA", 29),
    ("RO", 24),
    ("RS", 22),
    ("RU", 33),
    ("SA", 24),
    ("SC", 31),
    ("SD", 18),
    ("SE", 24),
    ("SI", 19),
    ("SK", 24),
    ("SM", 27),
    ("SO", 23),
    ("ST", 25),
    ("SV", 28),
    ("TL", 23),
    ("TN", 24),
    ("TR", 26),
    ("UA", 29),
    ("VA", 22),
    ("VG", 24),
    ("XK", 20),
    ("YE", 30),
];

fn expected_length(country: &str) -> Option<usize> {
    IBAN_LENGTHS
        .iter()
        .find(|(code, _)| *code == country)
        .map(|(_, len)| *len)
}

/// The BBAN structure for each country, from the ISO 13616 registry.
///
/// A structure is a sequence of `<count>!<class>` groups, where the class is
/// `n` (digits), `a` (upper-case letters) or `c` (alphanumeric). The registry
/// fixes both the length and the character class of every position, so a BBAN
/// of the right length with a letter where a digit belongs is not a valid IBAN
/// even though its check digits close.
///
static IBAN_STRUCTURES: &[(&str, &str)] = &[
    // BEGIN GENERATED IBAN STRUCTURES
    ("AD", "4!n4!n12!c"),
    ("AE", "3!n16!n"),
    ("AL", "8!n16!c"),
    ("AT", "5!n11!n"),
    ("AZ", "4!a20!c"),
    ("BA", "3!n3!n8!n2!n"),
    ("BE", "3!n7!n2!n"),
    ("BG", "4!a4!n2!n8!c"),
    ("BH", "4!a14!c"),
    ("BI", "5!n5!n11!n2!n"),
    ("BR", "8!n5!n10!n1!a1!c"),
    ("BY", "4!c4!n16!c"),
    ("CH", "5!n12!c"),
    ("CR", "4!n14!n"),
    ("CY", "3!n5!n16!c"),
    ("CZ", "4!n16!n"),
    ("DE", "8!n10!n"),
    ("DJ", "5!n5!n11!n2!n"),
    ("DK", "4!n9!n1!n"),
    ("DO", "4!c20!n"),
    ("EE", "2!n14!n"),
    ("EG", "4!n4!n17!n"),
    ("ES", "4!n4!n1!n1!n10!n"),
    ("FI", "3!n11!n"),
    ("FK", "2!a12!n"),
    ("FO", "4!n9!n1!n"),
    ("FR", "5!n5!n11!c2!n"),
    ("GB", "4!a6!n8!n"),
    ("GE", "2!a16!n"),
    ("GI", "4!a15!c"),
    ("GL", "4!n9!n1!n"),
    ("GR", "3!n4!n16!c"),
    ("GT", "4!c20!c"),
    ("HN", "4!a20!n"),
    ("HR", "7!n10!n"),
    ("HU", "3!n4!n1!n15!n1!n"),
    ("IE", "4!a6!n8!n"),
    ("IL", "3!n3!n13!n"),
    ("IQ", "4!a3!n12!n"),
    ("IS", "4!n2!n6!n10!n"),
    ("IT", "1!a5!n5!n12!c"),
    ("JO", "4!a4!n18!c"),
    ("KW", "4!a22!c"),
    ("KZ", "3!n13!c"),
    ("LB", "4!n20!c"),
    ("LC", "4!a24!c"),
    ("LI", "5!n12!c"),
    ("LT", "5!n11!n"),
    ("LU", "3!n13!c"),
    ("LV", "4!a13!c"),
    ("LY", "3!n3!n15!n"),
    ("MC", "5!n5!n11!c2!n"),
    ("MD", "2!c18!c"),
    ("ME", "3!n13!n2!n"),
    ("MK", "3!n10!c2!n"),
    ("MN", "4!n12!n"),
    ("MR", "5!n5!n11!n2!n"),
    ("MT", "4!a5!n18!c"),
    ("MU", "4!a2!n2!n12!n3!n3!a"),
    ("NI", "4!a20!n"),
    ("NL", "4!a10!n"),
    ("NO", "4!n6!n1!n"),
    ("OM", "3!n16!c"),
    ("PK", "4!a16!c"),
    ("PL", "8!n16!n"),
    ("PS", "4!a21!c"),
    ("PT", "4!n4!n11!n2!n"),
    ("QA", "4!a21!c"),
    ("RO", "4!a16!c"),
    ("RS", "3!n13!n2!n"),
    ("RU", "9!n5!n15!c"),
    ("SA", "2!n18!c"),
    ("SC", "4!a2!n2!n16!n3!a"),
    ("SD", "2!n12!n"),
    ("SE", "3!n16!n1!n"),
    ("SI", "5!n8!n2!n"),
    ("SK", "4!n6!n10!n"),
    ("SM", "1!a5!n5!n12!c"),
    ("SO", "4!n3!n12!n"),
    ("ST", "4!n4!n11!n2!n"),
    ("SV", "4!a20!n"),
    ("TL", "3!n14!n2!n"),
    ("TN", "2!n3!n13!n2!n"),
    ("TR", "5!n1!n16!c"),
    ("UA", "6!n19!c"),
    ("VA", "3!n15!n"),
    ("VG", "4!a16!n"),
    ("XK", "4!n10!n2!n"),
    ("YE", "4!a4!n18!c"),
    // END GENERATED IBAN STRUCTURES
];

/// Whether `bban` matches a structure of repeated `<count>!<class>` groups,
/// where the class is `n` (digits), `a` (upper-case letters) or `c`
/// (alphanumeric). Returns the first character that breaks the structure, so
/// the error names the position that is wrong rather than the whole number.
fn bban_matches(bban: &str, structure: &str) -> Option<char> {
    let bytes = bban.as_bytes();
    let mut pos = 0usize;
    let mut rest = structure;
    while !rest.is_empty() {
        let Some((count, tail)) = rest.split_once('!') else {
            return bban.chars().next();
        };
        let Ok(count) = count.parse::<usize>() else {
            return bban.chars().next();
        };
        let Some(class) = tail.chars().next() else {
            return bban.chars().next();
        };
        let end = pos + count;
        if end > bytes.len() {
            return bban.chars().nth(pos);
        }
        if let Some(bad) = bytes[pos..end].iter().find(|b| !match class {
            'n' => b.is_ascii_digit(),
            'a' => b.is_ascii_uppercase(),
            'c' => b.is_ascii_uppercase() || b.is_ascii_digit(),
            _ => false,
        }) {
            return Some(char::from(*bad));
        }
        pos = end;
        rest = &tail[class.len_utf8()..];
    }
    if pos == bytes.len() {
        None
    } else {
        bban.chars().nth(pos)
    }
}

fn expected_structure(country: &str) -> Option<&'static str> {
    IBAN_STRUCTURES
        .iter()
        .find(|(code, _)| *code == country)
        .map(|(_, structure)| *structure)
}

/// The national check digit that a few countries carry inside their BBAN, on
/// top of the MOD-97 check the whole IBAN closes.
///
/// These are the countries whose national check is arithmetic. Belgium also
/// validates the bank-code register on top of its check digit, and this crate
/// ships no register (the same boundary the MAC address and the BIC registry
/// observe), so a Belgian IBAN is checked for its check digit only.
///
/// Returns `Ok(())` for a country with no national check.
fn national_check(country: &str, bban: &str) -> Result<()> {
    match country {
        // Belgium: the last two digits are the first ten modulo 97, with a
        // remainder of zero written as 97.
        "BE" => {
            let digits = checksum::digits_only(bban)?;
            let first: u64 = digits[..10]
                .iter()
                .fold(0, |acc, d| acc * 10 + u64::from(*d));
            let expected = (first % 97) as u32;
            let expected = if expected == 0 { 97 } else { expected };
            let found = u32::from(digits[10]) * 10 + u32::from(digits[11]);
            if found != expected {
                return Err(Error::BadChecksum);
            }
            Ok(())
        }
        // Spain: a 20-digit CCC whose two check digits sit at positions 8-9.
        "ES" => {
            let digits = checksum::digits_only(bban)?;
            let expected = ccc_check_digits(&digits);
            if digits[8] != expected[0] || digits[9] != expected[1] {
                return Err(Error::BadChecksum);
            }
            Ok(())
        }
        // Norway: an 11-digit account number whose last digit closes a
        // modulus-11 sum over the first ten. A number that starts with the
        // postgiro bank code `0000` is a seven-digit account instead, and the
        // reference validates that one with Luhn.
        "NO" => {
            let digits = checksum::digits_only(bban)?;
            if digits.starts_with(&[0, 0, 0, 0]) {
                let account = &digits[4..];
                let (payload, check) = account.split_at(account.len() - 1);
                if !checksum::luhn_matches(payload, check[0]) {
                    return Err(Error::BadChecksum);
                }
                return Ok(());
            }
            let weights = [6u32, 7, 8, 9, 4, 5, 6, 7, 8, 9];
            let sum: u32 = weights
                .iter()
                .zip(&digits[..10])
                .map(|(w, d)| w * u32::from(*d))
                .sum();
            if (sum % 11) as u8 != digits[10] {
                return Err(Error::BadChecksum);
            }
            Ok(())
        }
        // Montenegro: the BBAN is congruent to 1 modulo 97.
        "ME" => {
            let mut remainder = 0u32;
            for b in bban.bytes() {
                if !b.is_ascii_digit() {
                    return Err(Error::InvalidCharacter(b as char));
                }
                remainder = (remainder * 10 + u32::from(b - b'0')) % 97;
            }
            if remainder != 1 {
                return Err(Error::BadChecksum);
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// The two check digits of a Spanish CCC, from the reference's own routine:
/// each half is a weighted sum of powers of two modulo eleven. The first half
/// is taken over two leading zeros and the first eight digits.
fn ccc_check_digits(digits: &[u8]) -> [u8; 2] {
    let half = |slice: &[u8]| -> u8 {
        let sum: u32 = slice
            .iter()
            .enumerate()
            .map(|(i, d)| u32::from(*d) * 2u32.pow(i as u32))
            .sum();
        let r = (sum % 11) as u8;
        if r < 2 { r } else { 11 - r }
    };
    let mut first = [0u8; 10];
    first[2..].copy_from_slice(&digits[..8]);
    [half(&first), half(&digits[10..])]
}

impl Iban {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::WrongLength`],
    /// [`Error::UnknownCountry`], [`Error::InvalidCharacter`] (a character that
    /// breaks the country's registry structure, or a non-digit in a national
    /// check) or [`Error::BadChecksum`] (the MOD-97-10 check, or a country's
    /// national check digit).
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        if normalized.len() < 4 {
            return Err(Error::WrongLength {
                expected: 4,
                found: normalized.len(),
            });
        }
        if !normalized.chars().all(|c| c.is_ascii_alphanumeric()) {
            let bad = normalized
                .chars()
                .find(|c| !c.is_ascii_alphanumeric())
                .unwrap_or('?');
            return Err(Error::InvalidCharacter(bad));
        }
        let country = &normalized[..2];
        if !country.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(Error::UnknownCountry(country.to_string()));
        }
        let Some(expected) = expected_length(country) else {
            return Err(Error::UnknownCountry(country.to_string()));
        };
        if normalized.len() != expected {
            return Err(Error::WrongLength {
                expected,
                found: normalized.len(),
            });
        }
        if !normalized[2..4].chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::BadChecksum);
        }
        let bban = &normalized[4..];
        if !Self::mod97_is_zero(&normalized) {
            return Err(Error::BadChecksum);
        }
        if let Some(structure) = expected_structure(country)
            && let Some(bad) = bban_matches(bban, structure)
        {
            return Err(Error::InvalidCharacter(bad));
        }
        national_check(country, bban)?;
        Ok(Self(normalized))
    }

    /// Rearranges the IBAN, converts letters to numbers and checks that the
    /// result is congruent to 1 modulo 97.
    fn mod97_is_zero(iban: &str) -> bool {
        // Move the first four characters to the end, then interpret the whole
        // string as a base-36 number and reduce it modulo 97 digit by digit.
        let rearranged = format!("{}{}", &iban[4..], &iban[..4]);
        let mut remainder = 0u32;
        for c in rearranged.chars() {
            let value = if let Some(d) = c.to_digit(10) {
                d
            } else if c.is_ascii_uppercase() {
                c as u32 - 'A' as u32 + 10
            } else {
                return false;
            };
            // A letter expands to two decimal digits, so it shifts by 100.
            let base = if value >= 10 { 100 } else { 10 };
            remainder = (remainder * base + value) % 97;
        }
        remainder == 1
    }

    /// The normalised IBAN.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The two-letter country code.
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self.0[..2]
    }

    /// The two check digits.
    #[must_use]
    pub fn check_digits(&self) -> &str {
        &self.0[2..4]
    }

    /// The country-specific part after the check digits.
    #[must_use]
    pub fn bban(&self) -> &str {
        &self.0[4..]
    }

    /// Formats the IBAN in groups of four characters, the usual printed form.
    #[must_use]
    pub fn format_grouped(&self) -> String {
        let mut out = String::with_capacity(self.0.len() + self.0.len() / 4);
        for (i, c) in self.0.chars().enumerate() {
            if i > 0 && i % 4 == 0 {
                out.push(' ');
            }
            out.push(c);
        }
        out
    }

    /// Computes the check digits for a country code and BBAN, so callers can
    /// build an IBAN from a national account number.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownCountry`] for a country outside the registry, or
    /// [`Error::InvalidCharacter`] for a non-alphanumeric BBAN.
    pub fn check_digits_for(country_code: &str, bban: &str) -> Result<String> {
        let country = country_code.to_uppercase();
        if country.len() != 2 || !country.chars().all(|c| c.is_ascii_alphabetic()) {
            return Err(Error::UnknownCountry(country_code.to_string()));
        }
        let bban = crate::normalize(bban);
        if !bban.chars().all(|c| c.is_ascii_alphanumeric()) {
            let bad = bban
                .chars()
                .find(|c| !c.is_ascii_alphanumeric())
                .unwrap_or('?');
            return Err(Error::InvalidCharacter(bad));
        }
        if let Some(expected) = expected_length(&country) {
            let found = country.len() + 2 + bban.len();
            if found != expected {
                return Err(Error::WrongLength { expected, found });
            }
        }
        // With check digits "00", find the value that makes the whole thing
        // congruent to 1 modulo 97.
        let probe = format!("{bban}{country}00");
        let mut remainder = 0u32;
        for c in probe.chars() {
            let value = if let Some(d) = c.to_digit(10) {
                d
            } else {
                c as u32 - 'A' as u32 + 10
            };
            let base = if value >= 10 { 100 } else { 10 };
            remainder = (remainder * base + value) % 97;
        }
        let check = 98 - remainder;
        Ok(format!("{check:02}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_published_examples() {
        // Examples from the ISO 13616 registry and common test data.
        for iban in [
            "DE89370400440532013000",
            "GB82WEST12345698765432",
            "FR1420041010050500013M02606",
            "NL91ABNA0417164300",
            "ES9121000418450200051332",
            "IT60X0542811101000000123456",
            "BE68539007547034",
            "CH9300762011623852957",
            "AT611904300234573201",
            "PL61109010140000071219812874",
            "SA0380000000608010167519",
            "AE070331234567890123456",
        ] {
            assert!(Iban::parse(iban).is_ok(), "{iban} should be valid");
        }
    }

    #[test]
    fn accepts_the_full_registry_country_list() {
        // Every country in the SWIFT IBAN registry, with a BBAN that matches
        // the registry's structure and correct check digits. The list comes
        // from `iban.dat` in python-stdnum, which is generated from the
        // registry, and the structures come from the same file.
        for (country, structure) in IBAN_STRUCTURES {
            let bban = structure_bban(country, structure);
            let check = Iban::check_digits_for(country, &bban).unwrap();
            let iban = format!("{country}{check}{bban}");
            assert!(Iban::parse(&iban).is_ok(), "{iban} should be valid");
        }
    }

    /// Builds a BBAN that satisfies a structure, choosing a conforming
    /// character per class (`a` a letter, otherwise a digit) and then fixing up
    /// the national check digits so the national check passes too.
    fn structure_bban(country: &str, structure: &str) -> String {
        let mut out = String::new();
        let mut rest = structure;
        while !rest.is_empty() {
            let (count, tail) = rest.split_once('!').unwrap();
            let count: usize = count.parse().unwrap();
            let class = tail.chars().next().unwrap();
            let fill = if class == 'a' { 'A' } else { '1' };
            out.extend(std::iter::repeat_n(fill, count));
            rest = &tail[class.len_utf8()..];
        }
        match country {
            "BE" => {
                let first: u64 = out[..10]
                    .bytes()
                    .fold(0, |acc, b| acc * 10 + u64::from(b - b'0'));
                let check = first % 97;
                let check = if check == 0 { 97 } else { check };
                format!("{}{:02}", &out[..10], check)
            }
            "ES" => {
                let digits: Vec<u8> = out.bytes().map(|b| b - b'0').collect();
                let check = ccc_check_digits(&digits);
                let mut bytes = out.into_bytes();
                bytes[8] = b'0' + check[0];
                bytes[9] = b'0' + check[1];
                String::from_utf8(bytes).unwrap()
            }
            "NO" => {
                let digits: Vec<u8> = out.bytes().map(|b| b - b'0').collect();
                let weights = [6u32, 7, 8, 9, 4, 5, 6, 7, 8, 9];
                let sum: u32 = weights
                    .iter()
                    .zip(&digits[..10])
                    .map(|(w, d)| w * u32::from(*d))
                    .sum();
                let mut bytes = out.into_bytes();
                bytes[10] = b'0' + (sum % 11) as u8;
                String::from_utf8(bytes).unwrap()
            }
            "ME" => {
                // Choose the last two digits so the whole BBAN is congruent to
                // one modulo 97, which is Montenegro's national check.
                let prefix = &out[..out.len() - 2];
                let p = prefix
                    .bytes()
                    .fold(0u32, |acc, b| (acc * 10 + u32::from(b - b'0')) % 97);
                let suffix = (0u32..97)
                    .find(|s| (p * 100 + s) % 97 == 1)
                    .expect("a two-digit suffix always exists");
                format!("{prefix}{suffix:02}")
            }
            _ => out,
        }
    }

    #[test]
    fn rejects_wrong_check_digits() {
        assert_eq!(
            Iban::parse("DE89370400440532013001"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            Iban::parse("GB83WEST12345698765432"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn rejects_wrong_length_for_country() {
        // Germany requires 22 characters.
        let err = Iban::parse("DE8937040044053201300").unwrap_err();
        assert_eq!(
            err,
            Error::WrongLength {
                expected: 22,
                found: 21
            }
        );
    }

    #[test]
    fn rejects_unknown_country() {
        assert_eq!(
            Iban::parse("ZZ89370400440532013000"),
            Err(Error::UnknownCountry("ZZ".into()))
        );
    }

    #[test]
    fn rejects_a_letter_where_the_registry_requires_a_digit() {
        // `DE171OCTNFFVAV1D4OWFYE` closes its MOD-97 check, but a German BBAN
        // is digits only; the reference rejects it as an invalid format and the
        // old length-only check accepted it. A Spanish BBAN is likewise all
        // digits, so a letter anywhere in it is a structure error too.
        for iban in ["DE171OCTNFFVAV1D4OWFYE", "ES11GHPN7MX0LMI0OJGSH0VY"] {
            assert!(
                matches!(Iban::parse(iban), Err(Error::InvalidCharacter(_))),
                "{iban} should fail the structure check"
            );
        }
        // A Greek BBAN is `3!n4!n16!c`, so a letter in the last group is fine.
        assert!(Iban::parse("GR1601101050000010547023795").is_ok());
    }

    #[test]
    fn rejects_a_national_check_digit_that_does_not_close() {
        // A Spanish CCC whose two national check digits are wrong, which the
        // MOD-97 check alone lets through. The reference raises InvalidChecksum.
        assert_eq!(
            Iban::parse("ES6011111111111111111111").unwrap_err(),
            Error::BadChecksum
        );
        // A Norwegian account number whose modulus-11 check digit is wrong.
        assert_eq!(
            Iban::parse("NO9486011117947").unwrap_err(),
            Error::BadChecksum
        );
        // The published examples pass both checks.
        assert!(Iban::parse("ES9121000418450200051332").is_ok());
        assert!(Iban::parse("NO9386011117947").is_ok());
    }

    #[test]
    fn a_belgian_iban_is_checked_for_structure_and_its_check_digit() {
        // Belgium's national check digit closes, but the reference additionally
        // looks the bank code up in a register this crate does not ship (the
        // same boundary the MAC address and the BIC registry observe), so a
        // Belgian IBAN whose bank code is unregistered is still accepted here.
        assert!(Iban::parse("BE68539007547034").is_ok());
        assert!(Iban::parse("BE32123456789002").is_ok());
        // A wrong Belgian check digit is rejected, as the reference does.
        assert_eq!(
            Iban::parse("BE41091811735141").unwrap_err(),
            Error::BadChecksum
        );
    }

    #[test]
    fn rejects_non_alphanumeric() {
        assert!(matches!(
            Iban::parse("DE89 3704 0044 0532 0130 0#"),
            Err(Error::InvalidCharacter('#'))
        ));
    }

    #[test]
    fn rejects_too_short() {
        assert!(matches!(Iban::parse("DE8"), Err(Error::WrongLength { .. })));
    }

    #[test]
    fn accessors_split_the_iban() {
        let i = Iban::parse("DE89370400440532013000").unwrap();
        assert_eq!(i.country_code(), "DE");
        assert_eq!(i.check_digits(), "89");
        assert_eq!(i.bban(), "370400440532013000");
    }

    #[test]
    fn groups_of_four_for_display() {
        let i = Iban::parse("DE89370400440532013000").unwrap();
        assert_eq!(i.format_grouped(), "DE89 3704 0044 0532 0130 00");
    }

    #[test]
    fn separators_and_lowercase_are_tolerated() {
        let i = Iban::parse("de89 3704 0044 0532 0130 00").unwrap();
        assert_eq!(i.as_str(), "DE89370400440532013000");
    }

    #[test]
    fn computes_check_digits() {
        // Rebuilding the published German example must reproduce "89".
        assert_eq!(
            Iban::check_digits_for("DE", "370400440532013000").unwrap(),
            "89"
        );
        assert_eq!(
            Iban::check_digits_for("GB", "WEST12345698765432").unwrap(),
            "82"
        );
        assert_eq!(
            Iban::check_digits_for("NL", "ABNA0417164300").unwrap(),
            "91"
        );
    }

    #[test]
    fn check_digits_for_rejects_bad_input() {
        assert!(matches!(
            Iban::check_digits_for("DE", "37040044053201300"),
            Err(Error::WrongLength { .. })
        ));
        assert!(matches!(
            Iban::check_digits_for("1X", "1234"),
            Err(Error::UnknownCountry(_))
        ));
    }
}
