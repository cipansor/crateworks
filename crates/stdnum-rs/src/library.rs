//! An ISIL, the international identifier for a library or a related
//! organisation.
//!
//! This is a port of `python-stdnum`'s `isil` module (ISO 15511). The
//! identifier is two parts joined by a hyphen: a prefix that names the
//! registering agency, which is either an ISO 3166-1 country code or a
//! registered non-national code, and the identifier the agency issued. Only the
//! prefix can be checked, because it is the part that is registered globally;
//! the agency's own numbering is not published in a common form.
//!
//! Unlike the rest of the crate the separators are significant - a colon or a
//! slash is part of the identifier - so this module does its own cleaning
//! rather than sharing the crate's global normalisation.

use crate::{Error, Result};

/// The characters an ISIL may contain.
fn is_alphabet(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | ':' | '/')
}

/// Folds the Unicode look-alikes of `-`, `:` and `/` to their ASCII form.
///
/// The reference's `clean` does the same, so an identifier copied out of a
/// document that used an en dash or a full-width colon is accepted rather than
/// rejected on the character.
fn fold(c: char) -> char {
    match c {
        '\u{2010}'..='\u{2015}'
        | '\u{2212}'
        | '\u{fe58}'
        | '\u{fe63}'
        | '\u{ff0d}'
        | '\u{2e3a}'
        | '\u{2e3b}' => '-',
        '\u{ff1a}' | '\u{a789}' | '\u{2236}' => ':',
        '\u{ff0f}' | '\u{2215}' | '\u{2044}' => '/',
        _ => c,
    }
}

/// The agency prefixes registered with the ISIL Registration Authority.
///
/// The first thirty-eight are ISO 3166-1 country codes; the last five
/// (`EUR`, `GTB`, `O`, `OCLC`, `ZDB`) are the non-national codes the authority
/// assigned to agencies that act across countries. The list is a snapshot of
/// `python-stdnum`'s `isil.dat`, which is itself generated from the authority's
/// register; a prefix that is added upstream is rejected here until this table
/// is regenerated.
static AGENCIES: &[&str] = &[
    "AR", "AT", "AU", "BE", "BG", "BO", "BY", "CA", "CH", "CY", "CZ", "DE", "DK", "EG", "EUR",
    "FI", "FR", "GB", "GL", "GTB", "HR", "HU", "IL", "IR", "IT", "JP", "KR", "KZ", "LU", "MD",
    "NL", "NO", "NP", "NZ", "O", "OCLC", "QA", "RO", "RU", "SI", "SK", "US", "ZDB",
];

/// An ISIL, the identifier of a library or a related organisation.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Isil(String);

impl Isil {
    /// Validates `input` and returns the normalised form.
    ///
    /// The normalised form is the input with surrounding whitespace removed;
    /// the agency prefix keeps whatever case it was written in, which
    /// [`Isil::formatted`] fixes.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidCharacter`] for a character outside the ISIL
    /// alphabet, [`Error::WrongLength`] for an identifier longer than fifteen
    /// characters, and [`Error::InvalidPrefix`] when the agency prefix is not
    /// one that is registered.
    pub fn parse(input: &str) -> Result<Self> {
        let compact: String = input.chars().map(fold).collect();
        let compact = compact.trim();
        if let Some(c) = compact.chars().find(|c| !is_alphabet(*c)) {
            return Err(Error::InvalidCharacter(c));
        }
        if compact.len() > 15 {
            return Err(Error::WrongLength {
                expected: 15,
                found: compact.len(),
            });
        }
        let agency = compact.split('-').next().unwrap_or("");
        if !is_known_agency(agency) {
            return Err(Error::InvalidPrefix);
        }
        Ok(Self(compact.to_string()))
    }

    /// The normalised identifier, without a surrounding space.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The agency prefix, the part before the first hyphen.
    #[must_use]
    pub fn agency(&self) -> &str {
        self.0.split('-').next().unwrap_or("")
    }

    /// The identifier the agency issued, the part after the first hyphen, or
    /// the empty string when the identifier is only an agency code.
    #[must_use]
    pub fn local(&self) -> &str {
        match self.0.split_once('-') {
            Some((_, rest)) => rest,
            None => "",
        }
    }

    /// The identifier in its standard presentation form.
    ///
    /// This is the reference's `format`: the agency prefix is upper-cased when
    /// it is a known one, and the local part is left as it was written.
    #[must_use]
    pub fn formatted(&self) -> String {
        match self.0.split_once('-') {
            Some((agency, rest)) if is_known_agency(agency) => {
                format!("{}-{rest}", agency.to_uppercase())
            }
            _ => self.0.clone(),
        }
    }
}

/// Whether `agency` names a registered agency, case-insensitively.
fn is_known_agency(agency: &str) -> bool {
    let upper = agency.to_uppercase();
    AGENCIES.binary_search(&upper.as_str()).is_ok()
}

impl std::fmt::Display for Isil {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_published_isils() {
        for input in ["IT-RM0267", "OCLC-DLC", "ZDB-1-NOL", "US-InU-Mu"] {
            assert!(Isil::parse(input).is_ok(), "{input} should parse");
        }
    }

    #[test]
    fn splits_into_agency_and_local_parts() {
        let isil = Isil::parse("IT-RM0267").unwrap();
        assert_eq!(isil.agency(), "IT");
        assert_eq!(isil.local(), "RM0267");
        // The reference accepts a bare agency code as an ISIL.
        let bare = Isil::parse("OCLC").unwrap();
        assert_eq!(bare.agency(), "OCLC");
        assert_eq!(bare.local(), "");
    }

    #[test]
    fn upper_cases_a_known_agency_in_the_presentation_form() {
        assert_eq!(Isil::parse("it-RM0267").unwrap().formatted(), "IT-RM0267");
        // The local part keeps its case.
        assert_eq!(Isil::parse("US-InU-Mu").unwrap().formatted(), "US-InU-Mu");
    }

    #[test]
    fn rejects_an_unregistered_agency() {
        assert_eq!(Isil::parse("WW-RM0267"), Err(Error::InvalidPrefix));
        assert_eq!(Isil::parse("XX-ABC"), Err(Error::InvalidPrefix));
        assert_eq!(Isil::parse("A-B"), Err(Error::InvalidPrefix));
        assert_eq!(Isil::parse(""), Err(Error::InvalidPrefix));
    }

    #[test]
    fn rejects_a_colon_in_place_of_the_hyphen() {
        // The reference rejects this as an unknown component: `IT:RM0267` has
        // no hyphen, so the whole string is read as the agency and it is not
        // one that is registered.
        assert_eq!(Isil::parse("IT:RM0267"), Err(Error::InvalidPrefix));
    }

    #[test]
    fn folds_a_unicode_dash_to_a_hyphen() {
        // A document may write the separator as an en dash; the reference's
        // `clean` folds it, so this crate does too.
        assert_eq!(
            Isil::parse("IT\u{2013}RM0267").unwrap().as_str(),
            "IT-RM0267"
        );
    }

    #[test]
    fn rejects_an_over_long_identifier() {
        assert_eq!(
            Isil::parse("IT-AAAAAAAAAAAAAA"),
            Err(Error::WrongLength {
                expected: 15,
                found: 17,
            })
        );
    }
}
