//! European Union identifiers that are not tied to one member state.
//!
//! These wrap the national rules: an excise number is a member state's
//! identifier under an EU-wide prefix, and an OSS number carries an ISO 3166-1
//! numeric member-state code. The rules live here rather than in a country
//! module because the reference groups them the same way.

use crate::{Error, Result};

/// The two-letter codes of the member states an excise number may name, in the
/// lower case the reference compares against.
const MEMBER_STATES: &[&str] = &[
    "at", "be", "bg", "cy", "cz", "de", "dk", "ee", "es", "fi", "fr", "gr", "hr", "hu", "ie", "it",
    "lt", "lu", "lv", "mt", "nl", "pl", "pt", "ro", "se", "si", "sk", "xi",
];

/// The ISO 3166-1 numeric codes of the member states an OSS number may name.
///
/// The reference keeps these as three-digit strings with a leading zero where
/// the number is below one hundred, so they are compared as text.
const OSS_MEMBER_STATES: &[&str] = &[
    "040", "056", "100", "191", "196", "203", "208", "233", "246", "250", "276", "300", "348",
    "372", "380", "428", "440", "442", "470", "528", "616", "620", "642", "703", "705", "724",
    "752", "900",
];

/// European excise numbers.
pub(super) mod excise {
    use super::*;

    /// Validates an EU excise number, the reference a business authorised to
    /// trade excise goods carries.
    ///
    /// A two-letter member-state code followed by up to eleven alphanumeric
    /// characters, zero-padded on the right of the code to thirteen. The
    /// country code must name a member state; the reference then hands the
    /// whole number to that country's own excise module, and only France
    /// defines one, so the French shape is enforced and the rest are accepted
    /// on the country code alone.
    pub(in crate::id) fn validate(s: &str) -> Result<()> {
        let cleaned = crate::id::clean(s, &[' ']);
        if cleaned.len() < 2 {
            return Err(Error::WrongLength {
                expected: 13,
                found: cleaned.len(),
            });
        }
        let mut number = cleaned;
        if number.len() < 13 {
            number = format!("{}{:0>11}", &number[..2], &number[2..]);
        }
        if number.len() != 13 {
            return Err(Error::WrongLength {
                expected: 13,
                found: number.len(),
            });
        }
        let cc = number[..2].to_ascii_lowercase();
        if !MEMBER_STATES.contains(&cc.as_str()) {
            return Err(Error::OutOfRange);
        }
        if cc == "fr" {
            super::super::europe::france::validate_accise(&number)?;
        }
        Ok(())
    }
}

/// European One Stop Shop VAT numbers.
pub(super) mod oss {
    use super::*;

    /// Validates an EU OSS number, the VAT reference issued under the one stop
    /// shop for e-commerce.
    ///
    /// The non-union scheme uses `EU` and eleven characters, the import scheme
    /// `IM` and twelve; an intermediary's `IN` number is not a VAT number and
    /// is rejected. The three digits after the prefix are an ISO 3166-1
    /// numeric member-state code. The reference notes that a check digit
    /// exists but that its algorithm is not published, so only the shape is
    /// checked.
    pub(in crate::id) fn validate(s: &str) -> Result<()> {
        let cleaned = crate::id::clean(s, &[' ', '-']);
        let expected = match cleaned.get(..2) {
            Some("EU") => 11,
            Some("IM") => 12,
            _ => return Err(Error::OutOfRange),
        };
        if cleaned.len() != expected {
            return Err(Error::WrongLength {
                expected,
                found: cleaned.len(),
            });
        }
        if !cleaned[2..].bytes().all(|b| b.is_ascii_digit()) {
            return Err(Error::InvalidCharacter(' '));
        }
        if !OSS_MEMBER_STATES.contains(&&cleaned[2..5]) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn excise_examples() {
        // The reference example, in its short and full forms.
        assert!(excise::validate("LU 987ABC").is_ok());
        assert!(excise::validate("LU00000987ABC").is_ok());
        // France is the one member state with its own excise rule.
        assert!(excise::validate("FR023004N9448").is_ok());
        // The excise category letter is constrained, so a wrong one is
        // rejected rather than repaired.
        assert_eq!(excise::validate("FR023004A9448"), Err(Error::OutOfRange));
        // A country that is not a member state is rejected.
        assert_eq!(excise::validate("XX00000987ABC"), Err(Error::OutOfRange));
        assert_eq!(
            excise::validate("DE1234567890123"),
            Err(Error::WrongLength {
                expected: 13,
                found: 15
            })
        );
    }

    #[test]
    fn oss_examples() {
        assert!(oss::validate("EU 372022452").is_ok());
        assert!(oss::validate("EU372022452").is_ok());
        // The import scheme is one character longer.
        assert!(oss::validate("IM3720224523").is_ok());
        assert_eq!(
            oss::validate("IM372022452"),
            Err(Error::WrongLength {
                expected: 12,
                found: 11
            })
        );
        // An intermediary number is not a VAT number.
        assert_eq!(oss::validate("IN372022452"), Err(Error::OutOfRange));
        // A code that is not a member state is rejected.
        assert_eq!(oss::validate("EU999022452"), Err(Error::OutOfRange));
    }
}
