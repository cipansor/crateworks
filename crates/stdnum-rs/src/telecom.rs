//! Identifiers that need no registry: a ship (IMO), a CDMA handset (MEID), an
//! audiovisual work (ISAN) and a Bitcoin address.
//!
//! These are ports of `python-stdnum`'s `imo`, `meid`, `isan` and `bitcoin`
//! modules. Each is a self-contained check over the number itself, so there is
//! no data table to keep in step with the standard and the whole rule fits in
//! this file.

use crate::{Error, Result};

/// The characters an ISAN or a MEID is drawn from, in check-digit order.
const HEX: &[u8] = b"0123456789ABCDEF";

/// The Base58 alphabet Bitcoin uses: the digits and letters with the four
/// glyphs that are easy to confuse (`0`, `O`, `I`, `l`) removed.
const BASE58: &[u8] = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz";

/// The Bech32 alphabet, which is a BCH code over five-bit groups.
const BECH32: &[u8] = b"qpzry9x8gf2tvdw0s3jn54khce6mua7l";

/// The index of `byte` in `alphabet`, or `None` if it is absent.
fn index_in(alphabet: &[u8], byte: u8) -> Option<u32> {
    alphabet.iter().position(|b| *b == byte).map(|p| p as u32)
}

/// Removes the separators a number may be grouped with.
///
/// `python-stdnum` cleans each of these types with its own separator set, so
/// this takes the set as an argument rather than sharing the crate's global
/// normalisation, which would also drop characters the reference keeps. It does
/// not change case: a Base58 address is case-sensitive, so each caller
/// upper-cases only when its reference does.
fn clean(input: &str, separators: &[char]) -> String {
    input.chars().filter(|c| !separators.contains(c)).collect()
}

/// Rejects any character that is not an ASCII digit.
fn digits_only(s: &str) -> Result<()> {
    if let Some(c) = s.chars().find(|c| !c.is_ascii_digit()) {
        return Err(Error::InvalidCharacter(c));
    }
    Ok(())
}

/// The values of `s` in `alphabet`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] for a symbol outside the alphabet.
fn values_in(alphabet: &[u8], s: &str) -> Result<Vec<u8>> {
    s.bytes()
        .map(|b| {
            index_in(alphabet, b)
                .map(|v| v as u8)
                .ok_or_else(|| Error::InvalidCharacter(char::from(b)))
        })
        .collect()
}

/// An IMO number, the permanent identifier of a ship's hull.
///
/// Seven digits, written with an `IMO` prefix. The last digit is a check digit
/// computed from the first six, weighted seven down to two.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Imo(String);

impl Imo {
    /// Validates `input` and returns the normalised digits.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        // The reference strips spaces only, then a leading `IMO`, and does not
        // treat a hyphen as a separator, so neither does this.
        let cleaned = clean(input.trim(), &[' ']).to_uppercase();
        let body = cleaned.strip_prefix("IMO").unwrap_or(&cleaned);
        if body.is_empty() {
            return Err(Error::Empty);
        }
        digits_only(body)?;
        if body.len() != 7 {
            return Err(Error::WrongLength {
                expected: 7,
                found: body.len(),
            });
        }
        if body.as_bytes()[6] != b'0' + check_digit(&body[..6])? {
            return Err(Error::BadChecksum);
        }
        Ok(Self(body.to_string()))
    }

    /// The normalised seven digits.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The form written on a hull, `IMO 9319466`.
    #[must_use]
    pub fn formatted(&self) -> String {
        format!("IMO {}", self.0)
    }

    /// The check digit for a six-digit payload.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCharacter`] if `payload` is not six digits.
    pub fn check_digit_for(payload: &str) -> Result<u8> {
        check_digit(payload)
    }
}

/// The IMO check digit: each of the six digits times its weight, seven down to
/// two, taken modulo ten.
fn check_digit(payload: &str) -> Result<u8> {
    digits_only(payload)?;
    let sum: u32 = payload
        .bytes()
        .enumerate()
        .map(|(i, b)| u32::from(b - b'0') * (7 - i as u32))
        .sum();
    Ok((sum % 10) as u8)
}

impl std::fmt::Display for Imo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A MEID, the identifier of a piece of CDMA mobile equipment.
///
/// It is stored in its fourteen-character hexadecimal form. The decimal form,
/// which is eighteen digits, is accepted and converted, and either form may
/// carry a trailing check digit.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Meid(String);

impl Meid {
    /// Validates `input` and returns the fourteen-character hexadecimal form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`], [`Error::OutOfRange`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let (body, check) = parse_parts(input)?;
        let hex = match body.len() {
            18 => {
                // The eighteen-digit decimal form is checked with the decimal
                // Luhn before it is converted.
                if let Some(cd) = check {
                    crate::luhn::Luhn::parse(&format!("{body}{cd}"))?;
                }
                decimal_to_hex(&body)?
            }
            _ => {
                // Fourteen hexadecimal characters. When they are all decimal
                // digits the number is an IMEI in disguise, so the check digit
                // is the decimal Luhn one; otherwise the hexadecimal Luhn is
                // used.
                if body.bytes().all(|b| b.is_ascii_digit()) {
                    if let Some(cd) = check {
                        crate::luhn::Luhn::parse(&format!("{body}{cd}"))?;
                    }
                } else if let Some(cd) = check {
                    validate_hex_luhn(&body, cd)?;
                }
                body
            }
        };
        Ok(Self(hex))
    }

    /// The normalised fourteen-character hexadecimal form, without a check
    /// digit.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The check digit for the fourteen-character body.
    #[must_use]
    pub fn check_digit(&self) -> char {
        let value = if self.0.bytes().all(|b| b.is_ascii_digit()) {
            let digits = crate::checksum::digits_only(&self.0).unwrap_or_default();
            crate::checksum::luhn_check_digit(&digits).unwrap_or(0)
        } else {
            let values = values_in(HEX, &self.0).unwrap_or_default();
            crate::checksum::luhn_mod_n_check_digit(&values, 16)
        };
        char::from(HEX[value as usize])
    }

    /// The decimal form, which is how a CDMA network writes the number.
    #[must_use]
    pub fn to_decimal(&self) -> String {
        let manufacturer = u32::from_str_radix(&self.0[..8], 16).unwrap_or(0);
        let serial = u32::from_str_radix(&self.0[8..], 16).unwrap_or(0);
        format!("{manufacturer:010}{serial:08}")
    }
}

/// Splits a MEID into its fourteen- or eighteen-character body and the check
/// character that follows it, if any.
fn parse_parts(input: &str) -> Result<(String, Option<char>)> {
    let cleaned = clean(input, &[' ', '-']).to_uppercase();
    if cleaned.is_empty() {
        return Err(Error::Empty);
    }
    match cleaned.len() {
        14 | 15 => {
            values_in(HEX, &cleaned)?;
            let (body, check) = cleaned.split_at(14);
            Ok((body.to_string(), check.chars().next()))
        }
        18 | 19 => {
            digits_only(&cleaned)?;
            let (body, check) = cleaned.split_at(18);
            Ok((body.to_string(), check.chars().next()))
        }
        found => Err(Error::WrongLength {
            expected: 14,
            found,
        }),
    }
}

/// Converts the eighteen-digit decimal body to fourteen hexadecimal
/// characters, rejecting a manufacturer or serial that overflows its field.
fn decimal_to_hex(body: &str) -> Result<String> {
    let manufacturer: u64 = body[..10].parse().map_err(|_| Error::OutOfRange)?;
    let serial: u64 = body[10..].parse().map_err(|_| Error::OutOfRange)?;
    // The reference rejects a manufacturer that needs more than thirty-two bits
    // or a serial that needs more than twenty-four, which a ten- and eight-digit
    // field can exceed.
    if manufacturer > u64::from(u32::MAX) || serial > 0x00ff_ffff {
        return Err(Error::OutOfRange);
    }
    Ok(format!("{manufacturer:08X}{serial:06X}"))
}

/// The hexadecimal Luhn check over a fourteen-character body and its digit.
fn validate_hex_luhn(body: &str, check: char) -> Result<()> {
    let mut values = values_in(HEX, body)?;
    let expected = index_in(HEX, check as u8).ok_or(Error::InvalidCharacter(check))? as u8;
    values.push(expected);
    if crate::checksum::luhn_mod_n_checksum(&values, 16) != 0 {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

impl std::fmt::Display for Meid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// An ISAN, the identifier of an audiovisual work.
///
/// A twelve-hex-digit root, a four-digit episode, an optional check character,
/// an optional eight-digit version and a second optional check character. Each
/// check character is a MOD 37-36 digit over the part it follows.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Isan {
    root: String,
    episode: String,
    version: String,
    check1: String,
    check2: String,
}

impl Isan {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`] or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let cleaned = clean(input, &[' ', '-']).to_uppercase();
        if cleaned.is_empty() {
            return Err(Error::Empty);
        }
        let (root, episode, check1, version, check2) = split(&cleaned);
        // The reference requires these component lengths and lets the check
        // characters be empty or, in the check-1 slot, one character. A version
        // that is present must be eight characters.
        if root.len() != 12
            || episode.len() != 4
            || check1.len() > 1
            || !matches!(version.len(), 0 | 8)
        {
            return Err(Error::WrongLength {
                expected: 16,
                found: cleaned.len(),
            });
        }
        // The root, episode and version are hexadecimal. The check characters
        // are not checked here; the MOD 37-36 step rejects one that is not
        // alphanumeric.
        values_in(HEX, &format!("{root}{episode}{version}"))?;
        // A check character that is present makes the whole number congruent to
        // one. The check is over the concatenation, not over the single
        // character the reference would compute, so that a number carrying more
        // than the usual one check character is judged the same way.
        if !check1.is_empty()
            && crate::iso7064::mod_37_36_checksum(&format!("{root}{episode}{check1}")) != Some(1)
        {
            return Err(Error::BadChecksum);
        }
        if !check2.is_empty()
            && crate::iso7064::mod_37_36_checksum(&format!("{root}{episode}{version}{check2}"))
                != Some(1)
        {
            return Err(Error::BadChecksum);
        }
        Ok(Self {
            root,
            episode,
            version,
            check1,
            check2,
        })
    }

    /// The form written in a catalogue, hyphen-separated, with a check
    /// character added where one is missing.
    #[must_use]
    pub fn formatted(&self) -> String {
        let mut parts = vec![
            self.root[..4].to_string(),
            self.root[4..8].to_string(),
            self.root[8..].to_string(),
            self.episode.clone(),
        ];
        let check1 = if self.check1.is_empty() {
            crate::iso7064::mod_37_36_check_digit(&format!("{}{}", self.root, self.episode))
                .map(|c| c.to_string())
                .unwrap_or_default()
        } else {
            self.check1.clone()
        };
        parts.push(check1);
        if !self.version.is_empty() {
            parts.push(self.version[..4].to_string());
            parts.push(self.version[4..].to_string());
            let check2 = if self.check2.is_empty() {
                crate::iso7064::mod_37_36_check_digit(&format!(
                    "{}{}{}",
                    self.root, self.episode, self.version
                ))
                .map(|c| c.to_string())
                .unwrap_or_default()
            } else {
                self.check2.clone()
            };
            parts.push(check2);
        }
        parts.join("-")
    }
}

/// Splits a compact ISAN into its five parts, following the reference's length
/// rules.
///
/// The reference slices by fixed position and lets a short input yield short
/// parts, which the caller then rejects, so this clamps the slices rather than
/// panicking on a length it did not expect. A check character may be more than
/// one character when the input is longer than the standard form, which is why
/// the two are strings.
fn split(number: &str) -> (String, String, String, String, String) {
    let chars: Vec<char> = number.chars().collect();
    let slice = |a: usize, z: usize| -> String {
        chars[a.min(chars.len())..z.min(chars.len())]
            .iter()
            .collect()
    };
    match chars.len() {
        17 | 26 => (
            slice(0, 12),
            slice(12, 16),
            slice(16, 17),
            slice(17, 25),
            slice(25, chars.len()),
        ),
        n if n > 16 => (
            slice(0, 12),
            slice(12, 16),
            String::new(),
            slice(16, 24),
            slice(24, chars.len()),
        ),
        _ => (
            slice(0, 12),
            slice(12, 16),
            slice(16, chars.len()),
            String::new(),
            String::new(),
        ),
    }
}

impl std::fmt::Display for Isan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.formatted())
    }
}

/// A Bitcoin address.
///
/// Three forms are accepted: the Base58 `1…` and `3…` addresses, whose last
/// four bytes are a double-SHA-256 check, and the Bech32 `bc1…` addresses,
/// whose trailing six symbols are a BCH checksum.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BitcoinAddress(String);

impl BitcoinAddress {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`],
    /// [`Error::WrongLength`], [`Error::InvalidPrefix`], [`Error::OutOfRange`]
    /// or [`Error::BadChecksum`].
    pub fn parse(input: &str) -> Result<Self> {
        let cleaned = clean(input.trim(), &[' ']);
        if cleaned.is_empty() {
            return Err(Error::Empty);
        }
        let lowered = if cleaned.len() >= 3 && cleaned[..3].eq_ignore_ascii_case("bc1") {
            cleaned.to_ascii_lowercase()
        } else {
            cleaned.clone()
        };
        if lowered.starts_with('1') || lowered.starts_with('3') {
            validate_base58(&lowered)?;
        } else if lowered.starts_with("bc1") {
            validate_bech32(&lowered)?;
        } else {
            return Err(Error::InvalidPrefix);
        }
        Ok(Self(lowered))
    }

    /// The normalised address.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Checks a Base58 address: 25 decoded bytes whose last four are the double
/// SHA-256 of the first twenty-one.
fn validate_base58(address: &str) -> Result<()> {
    if let Some(c) = address.bytes().find(|b| index_in(BASE58, *b).is_none()) {
        return Err(Error::InvalidCharacter(char::from(c)));
    }
    let decoded = base58_decode(address);
    if decoded.len() != 25 {
        return Err(Error::WrongLength {
            expected: 25,
            found: decoded.len(),
        });
    }
    let (payload, check) = decoded.split_at(21);
    let digest = crate::sha256::sha256(&crate::sha256::sha256(payload));
    if digest[..4] != *check {
        return Err(Error::BadChecksum);
    }
    Ok(())
}

/// Decodes a Base58 string, keeping the leading-zero bytes that the alphabet's
/// `1` stands for.
fn base58_decode(s: &str) -> Vec<u8> {
    // The value is accumulated as a big-endian byte string, starting empty so
    // that the result has no spurious leading zero.
    let mut bytes: Vec<u8> = Vec::new();
    for c in s.bytes() {
        let mut carry = index_in(BASE58, c).unwrap_or(0);
        for byte in bytes.iter_mut().rev() {
            let acc = u32::from(*byte) * 58 + carry;
            *byte = (acc & 0xff) as u8;
            carry = acc >> 8;
        }
        while carry > 0 {
            bytes.insert(0, (carry & 0xff) as u8);
            carry >>= 8;
        }
    }
    // One zero byte for each leading `1`, then the accumulated value.
    let leading = s.bytes().take_while(|b| *b == b'1').count();
    let mut result = vec![0u8; leading];
    result.extend_from_slice(&bytes);
    result
}

/// Checks a Bech32 address: the `bc` human-readable part, a checksum of one
/// over the expanded prefix and data, and a witness program of a valid length.
fn validate_bech32(address: &str) -> Result<()> {
    let data_part = &address[3..];
    if let Some(c) = data_part.bytes().find(|b| index_in(BECH32, *b).is_none()) {
        return Err(Error::InvalidCharacter(char::from(c)));
    }
    if address.len() < 11 || address.len() > 90 {
        return Err(Error::WrongLength {
            expected: 42,
            found: address.len(),
        });
    }
    let data = values_in(BECH32, data_part)?;
    let mut values = vec![b'b' >> 5, b'c' >> 5, 0, b'b' & 31, b'c' & 31];
    values.extend_from_slice(&data);
    if bech32_checksum(&values) != 1 {
        return Err(Error::BadChecksum);
    }
    let witness_version = data[0];
    let program = base32_decode(&data[1..data.len() - 6])?;
    if witness_version > 16 {
        return Err(Error::OutOfRange);
    }
    if !(2..=40).contains(&program.len()) {
        return Err(Error::WrongLength {
            expected: 20,
            found: program.len(),
        });
    }
    if witness_version == 0 && !matches!(program.len(), 20 | 32) {
        return Err(Error::WrongLength {
            expected: 20,
            found: program.len(),
        });
    }
    Ok(())
}

/// The Bech32 checksum over a sequence of five-bit values.
fn bech32_checksum(values: &[u8]) -> u32 {
    const GENERATOR: [(u32, u32); 5] = [
        (1 << 0, 0x3b6a_57b2),
        (1 << 1, 0x2650_8e6d),
        (1 << 2, 0x1ea1_19fa),
        (1 << 3, 0x3d42_33dd),
        (1 << 4, 0x2a14_62b3),
    ];
    let mut chk = 1u32;
    for &value in values {
        let top = chk >> 25;
        chk = (chk & 0x01ff_ffff) << 5 | u32::from(value);
        for (test, val) in GENERATOR {
            if top & test != 0 {
                chk ^= val;
            }
        }
    }
    chk
}

/// Decodes a sequence of five-bit values to bytes, rejecting a partial group.
fn base32_decode(data: &[u8]) -> Result<Vec<u8>> {
    let mut acc = 0u32;
    let mut bits = 0u32;
    let mut out = Vec::new();
    for &value in data {
        acc = ((acc << 5) | u32::from(value)) & 0xfff;
        bits += 5;
        if bits >= 8 {
            bits -= 8;
            out.push(((acc >> bits) & 0xff) as u8);
        }
    }
    if bits >= 5 || acc & ((1 << bits) - 1) != 0 {
        return Err(Error::OutOfRange);
    }
    Ok(out)
}

impl std::fmt::Display for BitcoinAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_a_published_imo() {
        let imo = Imo::parse("IMO 9319466").unwrap();
        assert_eq!(imo.as_str(), "9319466");
        assert_eq!(imo.formatted(), "IMO 9319466");
        assert_eq!(Imo::parse("8814275").unwrap().as_str(), "8814275");
    }

    #[test]
    fn rejects_an_imo_with_a_bad_check_digit() {
        assert_eq!(Imo::parse("8814274"), Err(Error::BadChecksum));
        assert_eq!(
            Imo::parse("IMO 931946"),
            Err(Error::WrongLength {
                expected: 7,
                found: 6,
            })
        );
    }

    #[test]
    fn accepts_a_hex_and_a_decimal_meid() {
        // Both spellings of the same number, from the reference's doctest.
        let hex = Meid::parse("AF 01 23 45 0A BC DE C").unwrap();
        assert_eq!(hex.as_str(), "AF0123450ABCDE");
        let decimal = Meid::parse("29360 87365 0070 3710 0").unwrap();
        assert_eq!(decimal.as_str(), "AF0123450ABCDE");
        assert_eq!(decimal.to_decimal(), "293608736500703710");
    }

    #[test]
    fn rejects_a_meid_with_a_bad_check_digit() {
        assert!(Meid::parse("29360 87365 0070 3710 1").is_err());
    }

    #[test]
    fn accepts_a_published_isan() {
        let isan = Isan::parse("0000-0001-8CFA-0000-I-0000-0000-K").unwrap();
        assert_eq!(isan.formatted(), "0000-0001-8CFA-0000-I-0000-0000-K");
        // The check characters are recomputed when they are absent.
        let without = Isan::parse("000000018CFA000000000000").unwrap();
        assert_eq!(without.formatted(), "0000-0001-8CFA-0000-I-0000-0000-K");
    }

    #[test]
    fn rejects_an_isan_with_a_bad_check_character() {
        assert_eq!(
            Isan::parse("0000-0001-8CFA-0000-A-0000-0000-K"),
            Err(Error::BadChecksum)
        );
    }

    #[test]
    fn accepts_base58_and_bech32_bitcoin_addresses() {
        assert!(BitcoinAddress::parse("1NEDqZPvTWRaoho48qXuLLsrYomMXPABfD").is_ok());
        let bech32 = BitcoinAddress::parse("BC1QARDV855YJNGSPVXUTTQ897AQCA3LXJU2Y69JCE").unwrap();
        assert_eq!(
            bech32.as_str(),
            "bc1qardv855yjngspvxuttq897aqca3lxju2y69jce"
        );
    }

    #[test]
    fn rejects_a_bitcoin_address_with_a_bad_check() {
        assert_eq!(
            BitcoinAddress::parse("1NEDqZPvTWRaoho48qXuLLsrYomMXPABfX"),
            Err(Error::BadChecksum)
        );
        assert_eq!(
            BitcoinAddress::parse("2NEDqZPvTWRaoho48qXuLLsrYomMXPABfD"),
            Err(Error::InvalidPrefix)
        );
    }
}
