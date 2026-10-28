//! A MAC address, the hardware identifier of a network interface.
//!
//! This is a port of `python-stdnum`'s `mac` module. The reference also
//! resolves the manufacturer from the IEEE registry, which needs a 1.2 MB
//! lookup table; this crate does not ship one, so it checks the shape and the
//! two bits that carry meaning and leaves the registry to the caller.

use crate::{Error, Result};

/// A MAC address, in the reference's canonical form `aa:bb:cc:dd:ee:ff`.
///
/// Only the shape is checked: six one- or two-digit hexadecimal groups
/// separated by colons, written with `-` or `:` in the input. The two
/// universally-administered and locally-administered bits are read by
/// [`Mac::is_multicast`] and [`Mac::is_locally_administered`].
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Mac(String);

impl Mac {
    /// Validates `input` and returns the normalised form.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Empty`], [`Error::InvalidCharacter`] or
    /// [`Error::WrongLength`].
    pub fn parse(input: &str) -> Result<Self> {
        let compact = compact(input);
        if compact.is_empty() {
            return Err(Error::Empty);
        }
        let groups: Vec<&str> = compact.split(':').collect();
        if groups.len() != 6 {
            return Err(Error::WrongLength {
                expected: 17,
                found: compact.len(),
            });
        }
        let mut normalized = String::with_capacity(17);
        for (i, group) in groups.iter().enumerate() {
            if group.is_empty() || group.len() > 2 {
                return Err(Error::WrongLength {
                    expected: 2,
                    found: group.len(),
                });
            }
            if let Some(c) = group.chars().find(|c| !c.is_ascii_hexdigit()) {
                return Err(Error::InvalidCharacter(c));
            }
            if i > 0 {
                normalized.push(':');
            }
            // The reference zero-pads a single-digit group, so `0:1b:...`
            // becomes `00:1b:...` rather than being rejected.
            if group.len() == 1 {
                normalized.push('0');
            }
            normalized.push_str(&group.to_lowercase());
        }
        Ok(Self(normalized))
    }

    /// The normalised address, `aa:bb:cc:dd:ee:ff`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The address in EUI-48 form, `AA-BB-CC-DD-EE-FF`.
    ///
    /// This is the reference's `to_eui48`.
    #[must_use]
    pub fn to_eui48(&self) -> String {
        self.0.to_uppercase().replace(':', "-")
    }

    /// The first three octets, the organisationally unique identifier, as six
    /// upper-case hexadecimal digits, such as `D05099`.
    ///
    /// The reference resolves this to a manufacturer through the IEEE
    /// registry; this crate returns the identifier and leaves the lookup to
    /// the caller.
    #[must_use]
    pub fn oui(&self) -> String {
        self.0[..8].replace(':', "").to_uppercase()
    }

    /// The last three octets, the part the registrant assigns, as six
    /// upper-case hexadecimal digits.
    #[must_use]
    pub fn nic(&self) -> String {
        self.0[9..].replace(':', "").to_uppercase()
    }

    /// The first octet as a byte.
    fn first_octet(&self) -> u8 {
        u8::from_str_radix(&self.0[..2], 16).expect("parse validated the octet")
    }

    /// Whether the address is unicast: the least significant bit of the first
    /// octet is zero.
    #[must_use]
    pub fn is_unicast(&self) -> bool {
        self.first_octet() & 1 == 0
    }

    /// Whether the address is multicast: the least significant bit of the
    /// first octet is one.
    #[must_use]
    pub fn is_multicast(&self) -> bool {
        !self.is_unicast()
    }

    /// Whether the address is the broadcast address `ff:ff:ff:ff:ff:ff`.
    #[must_use]
    pub fn is_broadcast(&self) -> bool {
        self.0 == "ff:ff:ff:ff:ff:ff"
    }

    /// Whether the address is universally administered: the second-least
    /// significant bit of the first octet is zero, so the manufacturer
    /// assigned it.
    #[must_use]
    pub fn is_universally_administered(&self) -> bool {
        self.first_octet() & 2 == 0
    }

    /// Whether the address is locally administered: the second-least
    /// significant bit of the first octet is one.
    #[must_use]
    pub fn is_locally_administered(&self) -> bool {
        !self.is_universally_administered()
    }
}

/// Removes spaces and turns `-` separators into `:`, the reference's `compact`.
///
/// The result is lower-cased, but no zero-padding happens here: that is done
/// group by group so a group of the wrong width can be reported. The dash
/// variants are folded first, because the reference's `clean` does the same
/// and an address pasted from a document may carry an en dash.
fn compact(input: &str) -> String {
    input
        .chars()
        .filter(|c| *c != ' ')
        .map(|c| match c {
            '\u{2010}'..='\u{2015}' | '\u{2212}' | '\u{fe58}' | '\u{fe63}' | '\u{ff0d}' | '-' => {
                ':'
            }
            _ => c,
        })
        .collect::<String>()
        .to_lowercase()
}

impl std::fmt::Display for Mac {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_the_published_forms() {
        for input in [
            "D0-50-99-84-A2-A0",
            "d0:50:99:84:a2:a0",
            "D0:50:99:84:A2:A0",
            "0:1b:63:84:45:e6",
        ] {
            assert!(Mac::parse(input).is_ok(), "{input} should parse");
        }
    }

    #[test]
    fn normalises_to_lower_case_colons() {
        let mac = Mac::parse("D0-50-99-84-A2-A0").unwrap();
        assert_eq!(mac.as_str(), "d0:50:99:84:a2:a0");
        assert_eq!(mac.to_string(), "d0:50:99:84:a2:a0");
        assert_eq!(mac.to_eui48(), "D0-50-99-84-A2-A0");
        assert_eq!(mac.oui(), "D05099");
        assert_eq!(mac.nic(), "84A2A0");
    }

    #[test]
    fn zero_pads_a_single_digit_group() {
        assert_eq!(
            Mac::parse("0:1b:63:84:45:e6").unwrap().as_str(),
            "00:1b:63:84:45:e6"
        );
    }

    #[test]
    fn classifies_the_first_octet() {
        let unicast = Mac::parse("d0:50:99:84:a2:a0").unwrap();
        assert!(unicast.is_unicast());
        assert!(!unicast.is_multicast());
        assert!(unicast.is_universally_administered());
        assert!(!unicast.is_locally_administered());
        assert!(!unicast.is_broadcast());

        let multicast = Mac::parse("01:00:5e:00:00:01").unwrap();
        assert!(multicast.is_multicast());
        assert!(!multicast.is_unicast());

        let local = Mac::parse("02:00:00:00:00:01").unwrap();
        assert!(local.is_locally_administered());
        assert!(!local.is_universally_administered());

        assert!(Mac::parse("ff:ff:ff:ff:ff:ff").unwrap().is_broadcast());
    }

    #[test]
    fn rejects_a_wrong_shape() {
        assert_eq!(Mac::parse(""), Err(Error::Empty));
        assert_eq!(
            Mac::parse("001b638445e6"),
            Err(Error::WrongLength {
                expected: 17,
                found: 12,
            })
        );
        assert_eq!(
            Mac::parse("00:1b:63:84:45:eg"),
            Err(Error::InvalidCharacter('g'))
        );
    }
}
