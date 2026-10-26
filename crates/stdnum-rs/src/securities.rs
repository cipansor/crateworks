//! Financial instrument and institution identifiers.
//!
//! These are the codes a trading, settlement or treasury system exchanges:
//! ISIN and CUSIP name a security, FIGI names an instrument across venues, LEI
//! names the legal entity that owns it, GRid names a research organisation and
//! BIC names the bank that moves the money.
//!
//! # What is checked
//!
//! Structure and check digit only. An ISIN whose check digit is right can still
//! name a security that was never listed, and a BIC whose country is assigned
//! can still name a branch that closed. Nothing here contacts a register.

use crate::iso3166;
use crate::iso7064;
use crate::{Error, Result};

/// The characters an ISIN, FIGI or CUSIP is drawn from, in check-digit order.
const ALNUM: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ";

/// CUSIP extends the alphanumeric alphabet with three placeholders that stand
/// for a letter the issuing agency had not assigned when the code was written.
const CUSIP_ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ*@#";

/// The codes an ISIN may carry that are not ISO 3166-1 country codes.
///
/// `XS` is the one that matters in practice: Eurobonds and other international
/// securities are issued under it, so rejecting it would reject most of the
/// international bond market.
const ISIN_EXTRA_CODES: &[&str] = &[
    "EU", // European Union institutions
    "QS", // Euroclear France, and temporarily assigned in Germany
    "QT", // internally used in Switzerland
    "XA", // CUSIP Global Services substitute agencies
    "XB", // NSD Russia substitute agencies
    "XC", // WM Datenservice Germany substitute agencies
    "XD", // SIX Telekurs substitute agencies
    "XF", // internally assigned, not unique
    "XK", // temporary country code for Kosovo
    "XS", // international securities
];

/// The index of `byte` in `alphabet`, or `None` if it is absent.
fn index_in(alphabet: &[u8], byte: u8) -> Option<u32> {
    alphabet.iter().position(|b| *b == byte).map(|p| p as u32)
}

/// The ISIN check digit for `payload`: the characters are expanded to their
/// alphabet values, the result is doubled from the right, and the digit sum is
/// completed to a multiple of ten.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` is not alphanumeric.
fn isin_check_digit(payload: &str) -> Result<u8> {
    let mut digits = Vec::with_capacity(payload.len() * 2);
    for byte in payload.bytes() {
        let value =
            index_in(ALNUM, byte).ok_or_else(|| Error::InvalidCharacter(char::from(byte)))?;
        digits.extend_from_slice(format!("{value}").as_bytes());
    }
    // Doubling starts at the rightmost digit of the expanded payload, because
    // the check digit that follows it sits at distance zero.
    let sum: u32 = digits
        .iter()
        .rev()
        .enumerate()
        .map(|(i, b)| {
            let value = u32::from(b - b'0');
            if i % 2 == 0 { value * 2 } else { value }
        })
        .map(|v| if v > 9 { v - 9 } else { v })
        .sum();
    Ok(((10 - sum % 10) % 10) as u8)
}

/// The CUSIP check digit for `payload`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` contains a character
/// outside the CUSIP alphabet.
fn cusip_check_digit(payload: &str) -> Result<u8> {
    let mut sum = 0u32;
    for (i, byte) in payload.bytes().enumerate() {
        let value = index_in(CUSIP_ALPHABET, byte)
            .ok_or_else(|| Error::InvalidCharacter(char::from(byte)))?;
        // CUSIP doubles from the left, so the first character is the one that
        // is left alone.
        let mut v = value * if i % 2 == 0 { 1 } else { 2 };
        if v > 9 {
            v = v / 10 + v % 10;
        }
        sum += v;
    }
    Ok(((10 - sum % 10) % 10) as u8)
}

/// An ISIN, the ISO 6166 identifier for a security.
///
/// Twelve characters: a two-letter country code, nine characters of national
/// identifier, and a check digit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Isin(String);

impl Isin {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`], [`Error::UnknownCountry`] or
    /// [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let normalized = crate::normalize(input);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        for byte in normalized.bytes() {
            if index_in(ALNUM, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        if normalized.len() != 12 {
            return Err(Error::WrongLength {
                expected: 12,
                found: normalized.len(),
            });
        }
        let country = &normalized[..2];
        if !iso3166::is_assigned(country) && !ISIN_EXTRA_CODES.contains(&country) {
            return Err(Error::UnknownCountry(country.to_string()));
        }
        if isin_check_digit(&normalized[..11])? != normalized.as_bytes()[11] - b'0' {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised ISIN.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The two-character prefix, a country code or one of the ISIN-specific
    /// codes such as `XS`.
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self.0[..2]
    }

    /// The nine characters between the prefix and the check digit.
    #[must_use]
    pub fn national_id(&self) -> &str {
        &self.0[2..11]
    }

    /// The check digit.
    #[must_use]
    pub fn check_digit(&self) -> u8 {
        self.0.as_bytes()[11] - b'0'
    }

    /// Builds an ISIN from a national security identifier, so a CUSIP or a
    /// SEDOL can be promoted to the international form.
    ///
    /// `national_id` is left-padded with zeros to nine characters, which is what
    /// the ISIN specification requires for identifiers that are shorter.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownCountry`] if `country_code` is not a code an
    /// ISIN may carry, and [`Error::InvalidCharacter`] if `national_id` is not
    /// alphanumeric or is longer than nine characters.
    ///
    /// # Example
    ///
    /// ```
    /// use stdnum_rs::securities::Isin;
    ///
    /// let isin = Isin::from_national_id("gb", "BYXJL75").unwrap();
    /// assert_eq!(isin.as_str(), "GB00BYXJL758");
    /// ```
    pub fn from_national_id(country_code: &str, national_id: &str) -> Result<Self> {
        let country = country_code.trim().to_ascii_uppercase();
        if country.len() != 2
            || (!iso3166::is_assigned(&country) && !ISIN_EXTRA_CODES.contains(&country.as_str()))
        {
            return Err(Error::UnknownCountry(country_code.to_string()));
        }
        let body = crate::normalize(national_id);
        for byte in body.bytes() {
            if index_in(ALNUM, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        if body.len() > 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: body.len(),
            });
        }
        let payload = format!("{country}{body:0>9}");
        let check = isin_check_digit(&payload)?;
        Ok(Self(format!("{payload}{check}")))
    }
}

impl std::fmt::Display for Isin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A CUSIP, the nine-character North American security identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Cusip(String);

impl Cusip {
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
        for byte in normalized.bytes() {
            if index_in(CUSIP_ALPHABET, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        if normalized.len() != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: normalized.len(),
            });
        }
        if cusip_check_digit(&normalized[..8])? != normalized.as_bytes()[8] - b'0' {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised CUSIP.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The issuer code, the first six characters.
    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.0[..6]
    }

    /// The check digit.
    #[must_use]
    pub fn check_digit(&self) -> u8 {
        self.0.as_bytes()[8] - b'0'
    }

    /// Promotes the CUSIP to an ISIN, which is how a United States or Canadian
    /// listing is named internationally.
    ///
    /// # Errors
    ///
    /// Returns the reason the constructed ISIN was rejected, which cannot
    /// happen for a CUSIP that already parsed.
    pub fn to_isin(&self) -> Result<Isin> {
        Isin::from_national_id("US", &self.0)
    }
}

impl std::fmt::Display for Cusip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A FIGI, the twelve-character Financial Instrument Global Identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Figi(String);

/// The characters a FIGI may use. The vowels and `Y` are excluded so the code
/// cannot accidentally spell a word.
const FIGI_ALPHABET: &[u8] = b"0123456789BCDFGHJKLMNPQRSTVWXYZ";

impl Figi {
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
        for byte in normalized.bytes() {
            if index_in(FIGI_ALPHABET, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        if normalized.len() != 12 {
            return Err(Error::WrongLength {
                expected: 12,
                found: normalized.len(),
            });
        }
        let bytes = normalized.as_bytes();
        // The first two characters are the provider, and both are letters; the
        // third is always `G` for "global".
        if bytes[0].is_ascii_digit() || bytes[1].is_ascii_digit() {
            return Err(Error::InvalidPrefix);
        }
        if matches!(&normalized[..2], "BS" | "BM" | "GG" | "GB" | "VG") {
            return Err(Error::InvalidPrefix);
        }
        if bytes[2] != b'G' {
            return Err(Error::InvalidPrefix);
        }
        if figi_check_digit(&normalized[..11])? != bytes[11] - b'0' {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised FIGI.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The check digit.
    #[must_use]
    pub fn check_digit(&self) -> u8 {
        self.0.as_bytes()[11] - b'0'
    }
}

/// The FIGI check digit for `payload`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` is not in the FIGI
/// alphabet.
fn figi_check_digit(payload: &str) -> Result<u8> {
    let mut sum = 0u32;
    for (i, byte) in payload.bytes().enumerate() {
        let value =
            index_in(ALNUM, byte).ok_or_else(|| Error::InvalidCharacter(char::from(byte)))?;
        // FIGI doubles from the left, like CUSIP.
        let mut v = value * if i % 2 == 0 { 1 } else { 2 };
        if v > 9 {
            v = v / 10 + v % 10;
        }
        sum += v;
    }
    Ok(((10 - sum % 10) % 10) as u8)
}

impl std::fmt::Display for Figi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An LEI, the twenty-character Legal Entity Identifier.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Lei(String);

impl Lei {
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
        for byte in normalized.bytes() {
            if index_in(ALNUM, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        if normalized.len() != 20 {
            return Err(Error::WrongLength {
                expected: 20,
                found: normalized.len(),
            });
        }
        if iso7064::mod_97_10_checksum(&normalized) != Some(1) {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised LEI.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The four-character code of the Local Operating Unit that issued it.
    #[must_use]
    pub fn lou(&self) -> &str {
        &self.0[..4]
    }

    /// The two check digits at the end.
    #[must_use]
    pub fn check_digits(&self) -> &str {
        &self.0[18..]
    }
}

impl std::fmt::Display for Lei {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A GRid, the eighteen-character identifier for a research organisation.
///
/// It is written with the `GRID:` prefix in citations; the prefix is accepted
/// and stripped.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Grid(String);

impl Grid {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let upper = input.trim().to_ascii_uppercase();
        let stripped = upper.strip_prefix("GRID:").unwrap_or(&upper);
        let normalized = crate::normalize(stripped);
        if normalized.is_empty() {
            return Err(Error::Empty);
        }
        for byte in normalized.bytes() {
            if index_in(ALNUM, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        if normalized.len() != 18 {
            return Err(Error::WrongLength {
                expected: 18,
                found: normalized.len(),
            });
        }
        if iso7064::mod_37_36_checksum(&normalized) != Some(1) {
            return Err(Error::BadChecksum);
        }
        Ok(Self(normalized))
    }

    /// The normalised GRid, without the `GRID:` prefix.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The form written in citations, `GRID:` followed by the code.
    #[must_use]
    pub fn prefixed(&self) -> String {
        format!("GRID:{}", self.0)
    }

    /// The form with the code split into its four parts by hyphens.
    #[must_use]
    pub fn formatted(&self) -> String {
        format!(
            "{}-{}-{}-{}",
            &self.0[0..2],
            &self.0[2..7],
            &self.0[7..17],
            &self.0[17..]
        )
    }
}

impl std::fmt::Display for Grid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A BIC, the ISO 9362 code that identifies a bank or one of its branches.
///
/// Eight characters identify the institution and its country and city;
/// eleven characters add a three-character branch code. The eight-character
/// form is the primary office, so an eight-character BIC and the same code with
/// `XXX` appended mean the same branch.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Bic(String);

impl Bic {
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
        if !matches!(normalized.len(), 8 | 11) {
            return Err(Error::WrongLength {
                expected: 8,
                found: normalized.len(),
            });
        }
        for byte in normalized.bytes() {
            if index_in(ALNUM, byte).is_none() {
                return Err(Error::InvalidCharacter(char::from(byte)));
            }
        }
        let bytes = normalized.as_bytes();
        // The institution code and the country code are letters; the location
        // code and the branch code are alphanumeric.
        if !bytes[..4].iter().all(u8::is_ascii_uppercase)
            || !bytes[4..6].iter().all(u8::is_ascii_uppercase)
        {
            return Err(Error::InvalidPrefix);
        }
        let country = &normalized[4..6];
        // `AN` and `CS` were withdrawn from ISO 3166-1 and no BIC carries them,
        // so they are rejected even though ISIN still accepts them on numbers
        // that are already in circulation.
        if !iso3166::is_assigned(country) || matches!(country, "AN" | "CS") {
            return Err(Error::UnknownCountry(country.to_string()));
        }
        Ok(Self(normalized))
    }

    /// The normalised BIC.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The four-character institution code.
    #[must_use]
    pub fn institution(&self) -> &str {
        &self.0[..4]
    }

    /// The two-character country code.
    #[must_use]
    pub fn country_code(&self) -> &str {
        &self.0[4..6]
    }

    /// The two-character location code.
    #[must_use]
    pub fn location(&self) -> &str {
        &self.0[6..8]
    }

    /// The three-character branch code, or `None` for the eight-character form.
    ///
    /// The eight-character form and a branch code of `XXX` both mean the
    /// primary office, which [`Bic::is_primary_office`] reports.
    #[must_use]
    pub fn branch(&self) -> Option<&str> {
        self.0.get(8..)
    }

    /// Whether this names the institution's primary office rather than a
    /// branch.
    #[must_use]
    pub fn is_primary_office(&self) -> bool {
        // `get(8..)` on an eight-character code is `Some("")`, not `None`, so
        // the empty branch is the eight-character form.
        matches!(self.0.get(8..), None | Some("") | Some("XXX"))
    }
}

impl std::fmt::Display for Bic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_published_isins() {
        // Apple, the example in the ISIN article; a United Kingdom listing; an
        // international Eurobond under XS; and a Netherlands Antilles code that
        // is still in circulation.
        for n in [
            "US0378331005",
            "GB00BYXJL758",
            "XS1234567896",
            "AN8068571086",
        ] {
            assert!(Isin::parse(n).is_ok(), "{n} should be valid");
        }
    }

    #[test]
    fn rejects_a_bad_isin_check_digit() {
        assert_eq!(Isin::parse("US0378331006"), Err(Error::BadChecksum));
        assert_eq!(
            Isin::parse("US0378331005".repeat(2).as_str()),
            Err(Error::WrongLength {
                expected: 12,
                found: 24
            })
        );
    }

    #[test]
    fn builds_an_isin_from_a_national_id() {
        // The worked example in the reference's docstring.
        assert_eq!(
            Isin::from_national_id("gb", "BYXJL75").unwrap().as_str(),
            "GB00BYXJL758"
        );
        assert_eq!(
            Isin::from_national_id("US", "037833100").unwrap().as_str(),
            "US0378331005"
        );
    }

    #[test]
    fn isin_accepts_an_international_security_code() {
        assert!(Isin::parse("XS1234567896").is_ok());
    }

    #[test]
    fn accepts_published_cusips() {
        for n in ["037833100", "594918104"] {
            assert!(Cusip::parse(n).is_ok(), "{n} should be valid");
        }
        assert_eq!(Cusip::parse("123456789"), Err(Error::BadChecksum));
    }

    #[test]
    fn promotes_a_cusip_to_an_isin() {
        let cusip = Cusip::parse("037833100").unwrap();
        assert_eq!(cusip.to_isin().unwrap().as_str(), "US0378331005");
        assert_eq!(cusip.issuer(), "037833");
    }

    #[test]
    fn accepts_published_figis() {
        for n in ["BBG000BLNNH6", "BBG000B9XRY4"] {
            assert!(Figi::parse(n).is_ok(), "{n} should be valid");
        }
    }

    #[test]
    fn rejects_a_figi_that_does_not_start_with_the_global_marker() {
        assert_eq!(Figi::parse("BB0000BLNNH6"), Err(Error::InvalidPrefix));
    }

    #[test]
    fn accepts_published_leis() {
        for n in ["5493001KJTIIGC8Y1R12", "213800KUD8LAJWSQ9D15"] {
            let lei = Lei::parse(n).unwrap();
            assert_eq!(lei.check_digits().len(), 2);
        }
        assert_eq!(Lei::parse("5493001KJTIIGC8Y1R13"), Err(Error::BadChecksum));
    }

    #[test]
    fn accepts_a_grid_with_and_without_its_prefix() {
        let bare = Grid::parse("GRID419475A000000N").unwrap();
        let prefixed = Grid::parse("GRID:grid419475a000000n").unwrap();
        assert_eq!(bare, prefixed);
        assert_eq!(bare.prefixed(), "GRID:GRID419475A000000N");
        assert_eq!(bare.formatted(), "GR-ID419-475A000000-N");
    }

    #[test]
    fn accepts_published_bics() {
        for n in ["ABNANL2A", "ABNANL2AXXX", "DEUTDEFF500"] {
            assert!(Bic::parse(n).is_ok(), "{n} should be valid");
        }
    }

    #[test]
    fn reports_a_bic_primary_office() {
        assert!(Bic::parse("ABNANL2A").unwrap().is_primary_office());
        assert!(Bic::parse("ABNANL2AXXX").unwrap().is_primary_office());
        let branch = Bic::parse("DEUTDEFF500").unwrap();
        assert!(!branch.is_primary_office());
        assert_eq!(branch.branch(), Some("500"));
        assert_eq!(branch.country_code(), "DE");
        assert_eq!(branch.location(), "FF");
    }

    #[test]
    fn rejects_a_bic_with_a_withdrawn_country() {
        assert_eq!(
            Bic::parse("ABNAAN22"),
            Err(Error::UnknownCountry("AN".to_string()))
        );
    }

    #[test]
    fn rejects_a_bic_of_the_wrong_length() {
        assert_eq!(
            Bic::parse("ABNANL2AXX"),
            Err(Error::WrongLength {
                expected: 8,
                found: 10
            })
        );
    }
}
