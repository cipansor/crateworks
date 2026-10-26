//! Publication, recording, device and creditor identifiers.
//!
//! These are the codes that identify a thing rather than a person: a journal
//! (ISSN), a music recording (ISRC), a printed score (ISMN), an author or
//! organisation (ISNI), a phone (IMEI), a chemical substance (CAS RN) and a
//! payment reference (ISO 11649).

use crate::iso7064;
use crate::{Error, Result};

/// The characters an identifier made of digits and a final check character is
/// drawn from.
const ALNUM: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// The index of `byte` in `alphabet`, or `None` if it is absent.
fn index_in(alphabet: &[u8], byte: u8) -> Option<u32> {
    alphabet.iter().position(|b| *b == byte).map(|p| p as u32)
}

/// Rejects any character that is not an ASCII digit.
fn digits_only(s: &str) -> Result<()> {
    if let Some(c) = s.chars().find(|c| !c.is_ascii_digit()) {
        return Err(Error::InvalidCharacter(c));
    }
    Ok(())
}

/// An ISSN, the eight-character identifier for a periodical.
///
/// The last character is a check character and may be `X`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Issn(String);

impl Issn {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        if normalized.len() != 8 {
            return Err(Error::WrongLength {
                expected: 8,
                found: normalized.len(),
            });
        }
        digits_only(&normalized[..7])?;
        let expected = issn_check_digit(&normalized[..7])?;
        if normalized.as_bytes()[7] != expected as u8 {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised ISSN, without a hyphen.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The form written on a publication, `1234-5679`.
    #[must_use]
    pub fn formatted(&self) -> String {
        format!("{}-{}", &self.0[..4], &self.0[4..])
    }

    /// The seven-digit body without the check character.
    #[must_use]
    pub fn body(&self) -> &str {
        &self.0[..7]
    }
}

/// The ISSN check character for a seven-character payload.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if the payload is not all digits.
fn issn_check_digit(payload: &str) -> Result<u8> {
    digits_only(payload)?;
    let sum: u32 = payload
        .bytes()
        .enumerate()
        .map(|(i, b)| (8 - i as u32) * u32::from(b - b'0'))
        .sum();
    let check = (11 - sum % 11) % 11;
    // A check value of ten is written as X, not as a digit.
    Ok(if check == 10 {
        b'X'
    } else {
        b'0' + check as u8
    })
}

impl std::fmt::Display for Issn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An ISRC, the twelve-character identifier for a sound or music video
/// recording.
///
/// Four parts: a country code, a registrant code, two digits for the year, and
/// five digits for the recording.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Isrc(String);

/// The codes an ISRC may carry in addition to the assigned country codes.
///
/// The agencies exhausted several national allocations and were given codes
/// that are not countries, such as `QM` and `QZ` for the United States, so a
/// check against ISO 3166-1 alone would reject valid recordings.
const ISRC_EXTRA_CODES: &[&str] = &[
    "BC", "BK", "BP", "BX", // Pro-música Brasil
    "CB", // Connect, Canada
    "CP", "DG", // reserved for further overflow
    "FX", // SCPP, France
    "GX", "UK", // PPL UK
    "KS", // KMCA, South Korea
    "QM", // United States, new registrants
    "QN", // International ISRC Agency, worldwide
    "QT", "QZ", // RIAA, United States
    "XK", // International ISRC Agency, Kosovo
    "YU", // International ISRC Agency, former Yugoslavia before 2006
    "ZB", // RISA, South Africa
    "ZZ", // International ISRC Agency, worldwide
];

impl Isrc {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`], [`Error::InvalidPrefix`] or
    /// [`Error::UnknownCountry`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        if normalized.len() != 12 {
            return Err(Error::WrongLength {
                expected: 12,
                found: normalized.len(),
            });
        }
        for byte in normalized.bytes() {
            if index_in(ALNUM, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        let bytes = normalized.as_bytes();
        let (country, registrant) = (&normalized[..2], &normalized[2..5]);
        if !bytes[..2].iter().all(u8::is_ascii_uppercase) {
            return Err(Error::InvalidPrefix);
        }
        // The registrant is letters or digits, the year and the recording are
        // digits, so the pattern is fixed even though there is no check digit.
        if !registrant.bytes().all(|b| b.is_ascii_alphanumeric())
            || !normalized[5..].bytes().all(|b| b.is_ascii_digit())
        {
            return Err(Error::InvalidPrefix);
        }
        if !crate::iso3166::is_assigned(country) && !ISRC_EXTRA_CODES.contains(&country) {
            return Err(Error::UnknownCountry(country.to_string()));
        }
        Ok(Self(normalized))
    }

    /// The normalised ISRC, without separators.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The two-character country or agency code.
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self.0[..2]
    }

    /// The three-character registrant code.
    #[must_use]
    pub fn registrant(&self) -> &str {
        &self.0[2..5]
    }

    /// The two-digit year the reference was assigned.
    #[must_use]
    pub fn year(&self) -> &str {
        &self.0[5..7]
    }

    /// The five-digit recording number.
    #[must_use]
    pub fn recording(&self) -> &str {
        &self.0[7..]
    }

    /// The form written on a release, `US-SKG-19-12345`.
    #[must_use]
    pub fn formatted(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            self.country_code(),
            self.registrant(),
            self.year(),
            self.recording()
        )
    }
}

impl std::fmt::Display for Isrc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An IMEI or IMEISV, the identifier of a mobile device.
///
/// The IMEI is fifteen digits and its last digit is a Luhn check digit. The
/// fourteen-digit form omits the check digit, and the sixteen-digit IMEISV
/// replaces it with a two-digit software version, so neither can be checked.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Imei(String);

/// Which of the three forms an [`Imei`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ImeiKind {
    /// Fourteen digits, with no check digit.
    WithoutCheckDigit,
    /// Fifteen digits, with a Luhn check digit.
    Imei,
    /// Sixteen digits, ending in a two-digit software version.
    Imeisv,
}

impl Imei {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        digits_only(&normalized)?;
        match normalized.len() {
            14 | 16 => {}
            15 => {
                // Only the fifteen-digit form carries a check digit, so only it
                // can be rejected for a wrong one.
                crate::luhn::Luhn::parse(&normalized)?;
            }
            found => {
                return Err(Error::WrongLength {
                    expected: 15,
                    found,
                });
            }
        }
        Ok(Self(normalized))
    }

    /// The normalised digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Which of the three forms this is.
    #[must_use]
    pub fn kind(&self) -> ImeiKind {
        match self.0.len() {
            14 => ImeiKind::WithoutCheckDigit,
            16 => ImeiKind::Imeisv,
            _ => ImeiKind::Imei,
        }
    }

    /// The eight-digit Type Allocation Code, which identifies the model.
    #[must_use]
    pub fn tac(&self) -> &str {
        &self.0[..8]
    }

    /// The six-digit serial number.
    #[must_use]
    pub fn serial(&self) -> &str {
        &self.0[8..14]
    }

    /// The check digit, or the two-digit software version for an IMEISV.
    #[must_use]
    pub fn suffix(&self) -> &str {
        &self.0[14..]
    }

    /// The form written on a device label, `35-417803-685978-9`.
    #[must_use]
    pub fn formatted(&self) -> String {
        let bytes = self.0.as_bytes();
        let mut parts = vec![&self.0[..2], &self.0[2..8], &self.0[8..14]];
        if bytes.len() > 14 {
            parts.push(&self.0[14..]);
        }
        parts.join("-")
    }
}

impl std::fmt::Display for Imei {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An ISMN, the identifier for printed music.
///
/// The current form is thirteen digits and begins `9790`; the legacy form is
/// ten characters and begins `M`. Both are accepted.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Ismn(String);

/// Which of the two ISMN forms an [`Ismn`] holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum IsmnKind {
    /// The legacy ten-character form beginning `M`.
    Ismn10,
    /// The current thirteen-digit form beginning `9790`.
    Ismn13,
}

impl Ismn {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`], [`Error::InvalidPrefix`] or
    /// [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        match normalized.len() {
            10 => {
                if !normalized.starts_with('M') {
                    return Err(Error::InvalidPrefix);
                }
                // The legacy form is the EAN of the same number with the `M`
                // replaced by `9790`, so its check digit is the EAN one.
                crate::ean::Ean::parse(&format!("9790{}", &normalized[1..]))?;
            }
            13 => {
                if !normalized.starts_with("9790") {
                    return Err(Error::InvalidPrefix);
                }
                crate::ean::Ean::parse(&normalized)?;
            }
            found => {
                return Err(Error::WrongLength {
                    expected: 13,
                    found,
                });
            }
        }
        Ok(Self(normalized))
    }

    /// The normalised ISMN.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Which of the two forms this is.
    #[must_use]
    pub fn kind(&self) -> IsmnKind {
        if self.0.len() == 10 {
            IsmnKind::Ismn10
        } else {
            IsmnKind::Ismn13
        }
    }

    /// The same number in the thirteen-digit form, which is what a barcode
    /// carries.
    #[must_use]
    pub fn to_ismn13(&self) -> String {
        if self.0.len() == 13 {
            self.0.clone()
        } else {
            format!("9790{}", &self.0[1..])
        }
    }
}

impl std::fmt::Display for Ismn {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An ISNI, the sixteen-character identifier for a contributor to media
/// content.
///
/// The last character is a MOD 11-2 check character and may be `X`.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Isni(String);

impl Isni {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        if normalized.len() != 16 {
            return Err(Error::WrongLength {
                expected: 16,
                found: normalized.len(),
            });
        }
        digits_only(&normalized[..15])?;
        if iso7064::mod_11_2_checksum(&normalized) != Some(1) {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised ISNI.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The form written by the registration agency, in four groups.
    #[must_use]
    pub fn formatted(&self) -> String {
        format!(
            "{} {} {} {}",
            &self.0[..4],
            &self.0[4..8],
            &self.0[8..12],
            &self.0[12..]
        )
    }
}

impl std::fmt::Display for Isni {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A CAS Registry Number, the identifier for a chemical substance.
///
/// It is written in three hyphen-separated groups, such as `7732-18-5`, and the
/// last digit is a check digit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CasNumber(String);

impl CasNumber {
    /// Validates `input` and returns the normalised form, with hyphens.
    ///
    /// The hyphens must be where the specification puts them. A number that
    /// arrives as bare digits is accepted and hyphenated, but a number whose
    /// hyphens are anywhere else is rejected rather than repaired, because a
    /// misplaced hyphen is a sign the value was transcribed wrongly.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`], [`Error::InvalidPrefix`] or
    /// [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        // Only spaces are separators here; a hyphen is part of the number.
        let stripped: String = input.chars().filter(|c| *c != ' ').collect();
        let stripped = stripped.trim();
        if stripped.is_empty() {
            return Err(Error::Empty);
        }
        let normalized = if stripped.contains('-') {
            stripped.to_string()
        } else {
            // Two to seven digits, then the two-digit group, then the check
            // digit.
            if !(5..=10).contains(&stripped.len()) {
                return Err(Error::WrongLength {
                    expected: 7,
                    found: stripped.len() + 2,
                });
            }
            let (head, rest) = stripped.split_at(stripped.len() - 3);
            format!("{head}-{}-{}", &rest[..2], &rest[2..])
        };
        if !(7..=12).contains(&normalized.len()) {
            return Err(Error::WrongLength {
                expected: 7,
                found: normalized.len(),
            });
        }
        // The shape is fixed: one to seven leading digits, a hyphen, two
        // digits, a hyphen, the check digit.
        let bytes = normalized.as_bytes();
        let last_hyphen = normalized.len() - 2;
        let first_hyphen = normalized.len() - 5;
        if bytes[last_hyphen] != b'-' || bytes[first_hyphen] != b'-' {
            // A hyphen that is not in one of the two required positions.
            return Err(Error::InvalidCharacter('-'));
        }
        for (i, byte) in bytes.iter().enumerate() {
            if i == last_hyphen || i == first_hyphen {
                continue;
            }
            if !byte.is_ascii_digit() {
                return Err(Error::InvalidCharacter(char::from(*byte)));
            }
        }
        if bytes[0] == b'0' {
            return Err(Error::InvalidPrefix);
        }
        let digits: String = normalized.chars().filter(char::is_ascii_digit).collect();
        let payload = &digits[..digits.len() - 1];
        if cas_check_digit(payload)? != digits.as_bytes()[digits.len() - 1] - b'0' {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised number, with hyphens.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The digits without the hyphens.
    #[must_use]
    pub fn digits(&self) -> String {
        self.0.chars().filter(char::is_ascii_digit).collect()
    }
}

/// The CAS check digit: each digit is weighted by its distance from the check
/// digit, counting from one.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` is not all digits.
fn cas_check_digit(payload: &str) -> Result<u8> {
    digits_only(payload)?;
    let sum: u32 = payload
        .bytes()
        .rev()
        .enumerate()
        .map(|(i, b)| (i as u32 + 1) * u32::from(b - b'0'))
        .sum();
    Ok((sum % 10) as u8)
}

impl std::fmt::Display for CasNumber {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An ISO 11649 structured creditor reference, used to match a payment to an
/// invoice.
///
/// It begins `RF`, ends in two MOD 97-10 check digits, and is at most 25
/// characters.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CreditorReference(String);

impl CreditorReference {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`], [`Error::InvalidPrefix`] or
    /// [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        if !(5..=25).contains(&normalized.len()) {
            return Err(Error::WrongLength {
                expected: 25,
                found: normalized.len(),
            });
        }
        if !normalized.starts_with("RF") {
            return Err(Error::InvalidPrefix);
        }
        for byte in normalized.bytes() {
            if index_in(ALNUM, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        // The check digits cover the body followed by the prefix, which is the
        // same rearrangement MOD 97-10 uses for an IBAN.
        let rearranged = format!("{}{}", &normalized[4..], &normalized[..4]);
        if iso7064::mod_97_10_checksum(&rearranged) != Some(1) {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised reference, without spaces.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The form written on an invoice, in groups of four.
    #[must_use]
    pub fn formatted(&self) -> String {
        self.0
            .as_bytes()
            .chunks(4)
            .map(|c| std::str::from_utf8(c).expect("ASCII input"))
            .collect::<Vec<_>>()
            .join(" ")
    }
}

impl std::fmt::Display for CreditorReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_published_issn() {
        let issn = Issn::parse("0024-9319").unwrap();
        assert_eq!(issn.as_str(), "00249319");
        assert_eq!(issn.formatted(), "0024-9319");
        assert_eq!(issn.body(), "0024931");
    }

    #[test]
    fn issn_check_character_may_be_x() {
        // An ISSN whose check character is X, which is what the algorithm
        // produces when the weighted sum leaves ten.
        assert!(Issn::parse("0000-006X").is_ok());
        assert_eq!(Issn::parse("0032147X"), Err(Error::BadChecksum));
    }

    #[test]
    fn accepts_a_published_isrc() {
        let isrc = Isrc::parse("US-SKG-19-12345").unwrap();
        assert_eq!(isrc.as_str(), "USSKG1912345");
        assert_eq!(isrc.country_code(), "US");
        assert_eq!(isrc.registrant(), "SKG");
        assert_eq!(isrc.year(), "19");
        assert_eq!(isrc.recording(), "12345");
        assert_eq!(isrc.formatted(), "US-SKG-19-12345");
    }

    #[test]
    fn accepts_an_isrc_agency_code_that_is_not_a_country() {
        // The United States exhausted its national allocation, so recordings
        // are issued under QM, QT and QZ.
        assert!(Isrc::parse("QM-SKG-19-12345").is_ok());
    }

    #[test]
    fn rejects_an_isrc_with_an_unassigned_country() {
        assert_eq!(
            Isrc::parse("XX-SKG-19-12345"),
            Err(Error::UnknownCountry("XX".to_string()))
        );
    }

    #[test]
    fn accepts_the_three_imei_forms() {
        let imei = Imei::parse("35-417803-685978-9").unwrap();
        assert_eq!(imei.as_str(), "354178036859789");
        assert_eq!(imei.kind(), ImeiKind::Imei);
        assert_eq!(imei.tac(), "35417803");
        assert_eq!(imei.serial(), "685978");
        assert_eq!(imei.formatted(), "35-417803-685978-9");

        let bare = Imei::parse("35686800004141").unwrap();
        assert_eq!(bare.kind(), ImeiKind::WithoutCheckDigit);
        assert_eq!(bare.suffix(), "");

        let sv = Imei::parse("35686800-004141-20").unwrap();
        assert_eq!(sv.kind(), ImeiKind::Imeisv);
        assert_eq!(sv.suffix(), "20");
    }

    #[test]
    fn rejects_an_imei_with_a_bad_check_digit() {
        assert_eq!(Imei::parse("35-417803-685978-1"), Err(Error::BadChecksum));
    }

    #[test]
    fn accepts_both_ismn_forms() {
        let modern = Ismn::parse("979-0-3452-4680-5").unwrap();
        assert_eq!(modern.as_str(), "9790345246805");
        assert_eq!(modern.kind(), IsmnKind::Ismn13);
        assert_eq!(modern.to_ismn13(), "9790345246805");

        let legacy = Ismn::parse("M230671187").unwrap();
        assert_eq!(legacy.as_str(), "M230671187");
        assert_eq!(legacy.kind(), IsmnKind::Ismn10);
        assert_eq!(legacy.to_ismn13(), "9790230671187");
    }

    #[test]
    fn rejects_an_ismn_with_a_bad_check_digit() {
        assert_eq!(Ismn::parse("9790060115614"), Err(Error::BadChecksum));
    }

    #[test]
    fn accepts_a_published_isni() {
        let isni = Isni::parse("0000 0001 2281 955X").unwrap();
        assert_eq!(isni.as_str(), "000000012281955X");
        assert_eq!(isni.formatted(), "0000 0001 2281 955X");
    }

    #[test]
    fn rejects_an_isni_with_a_bad_check_character() {
        assert_eq!(Isni::parse("0000 0001 1111 955X"), Err(Error::BadChecksum));
    }

    #[test]
    fn accepts_published_cas_numbers() {
        for (input, digits) in [
            ("7732-18-5", "7732185"),
            ("50-00-0", "50000"),
            ("107-05-1", "107051"),
        ] {
            let cas = CasNumber::parse(input).unwrap();
            assert_eq!(cas.as_str(), input);
            assert_eq!(cas.digits(), digits);
        }
    }

    #[test]
    fn accepts_a_cas_number_without_its_hyphens() {
        assert_eq!(CasNumber::parse("7732185").unwrap().as_str(), "7732-18-5");
    }

    #[test]
    fn rejects_a_cas_number_with_a_bad_check_digit() {
        assert_eq!(CasNumber::parse("7732-18-6"), Err(Error::BadChecksum));
    }

    #[test]
    fn rejects_a_cas_number_whose_hyphens_are_misplaced() {
        // The reference rejects these rather than repairing them, and the
        // fixture shows it: a hyphen in the wrong place is a transcription
        // error, not a formatting preference.
        assert_eq!(
            CasNumber::parse("87586-5"),
            Err(Error::InvalidCharacter('-'))
        );
        assert_eq!(
            CasNumber::parse("87-8615"),
            Err(Error::InvalidCharacter('-'))
        );
    }

    #[test]
    fn accepts_a_published_creditor_reference() {
        let reference = CreditorReference::parse("RF18539007547034").unwrap();
        assert_eq!(reference.as_str(), "RF18539007547034");
        assert_eq!(reference.formatted(), "RF18 5390 0754 7034");
        assert!(CreditorReference::parse("RF712348231").is_ok());
    }

    #[test]
    fn rejects_a_creditor_reference_with_a_bad_check_digit() {
        assert_eq!(
            CreditorReference::parse("RF19539007547034"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            CreditorReference::parse("GB18539007547034"),
            Err(Error::InvalidPrefix)
        );
    }
}
