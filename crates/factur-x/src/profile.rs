//! The profile a document claims to follow.
//!
//! Factur-X and Order-X each define a ladder of profiles. The profile is named
//! in the specification identifier (BT-24 in EN 16931), which lives in a
//! different element per syntax: `ram:GuidelineSpecifiedDocumentContextParameter`
//! / `ram:ID` for CII and `cbc:CustomizationID` for UBL. The identifier is a
//! URN, and the profile is the last segment of it, or the segment before a
//! `#`-qualified suffix.
//!
//! The reference's `get_facturx_level` reads the same field the same way, and
//! this is a port of it.

use crate::{Error, Result};

/// The profile a document claims to follow.
///
/// The two families share the level names `basic` and `extended`; a code is
/// only ever read under the syntax the document declares, so a single enum is
/// enough. [`Profile::Comfort`] is Order-X only, and
/// [`Profile::ExtendedCtcFr`] is the French CTC extension of the Factur-X
/// extended profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Profile {
    /// Factur-X `minimum`: the fewest elements a document can carry.
    Minimum,
    /// Factur-X `basicwl`: the basic profile without the line items.
    BasicWl,
    /// Factur-X `basic`, also Order-X `basic`.
    Basic,
    /// The full EN 16931 core.
    En16931,
    /// Factur-X `extended`, also Order-X `extended`.
    Extended,
    /// The French CTC profile, `extended-ctc-fr`.
    ExtendedCtcFr,
    /// Order-X `comfort`.
    Comfort,
}

impl Profile {
    /// Every profile, for a caller that needs to enumerate them.
    pub const ALL: &'static [Self] = &[
        Self::Minimum,
        Self::BasicWl,
        Self::Basic,
        Self::En16931,
        Self::Extended,
        Self::ExtendedCtcFr,
        Self::Comfort,
    ];

    /// The level name as the specification identifier spells it.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Minimum => "minimum",
            Self::BasicWl => "basicwl",
            Self::Basic => "basic",
            Self::En16931 => "en16931",
            Self::Extended => "extended",
            Self::ExtendedCtcFr => "extended-ctc-fr",
            Self::Comfort => "comfort",
        }
    }

    /// Reads the profile from a specification identifier.
    ///
    /// The profile is the text after the last `:`; when that is not a known
    /// level, the identifier is cut at the first `#` and the segment before the
    /// last `:` of the cut is tried instead, which is how a qualified URN such
    /// as `urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:basic` still
    /// names `basic`.
    ///
    /// # Errors
    ///
    /// Returns [`Error::UnknownProfile`] when neither reading names a known
    /// level. This matches the reference, which raises rather than guessing.
    pub fn parse(guideline: &str) -> Result<Self> {
        if let Some(profile) = Self::from_level(suffix(guideline, ':')) {
            return Ok(profile);
        }
        // Ignore everything after the first `#`, then take the segment before
        // the last `:` of what remains.
        let cut = guideline.split('#').next().unwrap_or(guideline);
        if cut.contains(':')
            && let Some(profile) = Self::from_level(second_to_last(cut))
        {
            return Ok(profile);
        }
        Err(Error::UnknownProfile {
            guideline: guideline.to_string(),
        })
    }

    fn from_level(level: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|p| p.as_str() == level)
    }
}

impl std::fmt::Display for Profile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// The text after the last occurrence of `separator`, or the whole string.
fn suffix(text: &str, separator: char) -> &str {
    match text.rsplit_once(separator) {
        Some((_, tail)) => tail,
        None => text,
    }
}

/// The segment before the last `separator`.
fn second_to_last(text: &str) -> &str {
    match text.rsplit_once(':') {
        Some((head, _)) => match head.rsplit_once(':') {
            Some((_, segment)) => segment,
            None => head,
        },
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_urn_names_its_last_segment() {
        assert_eq!(
            Profile::parse("urn:factur-x.eu:1p0:minimum").unwrap(),
            Profile::Minimum
        );
        assert_eq!(
            Profile::parse("urn:factur-x.eu:1p0:basicwl").unwrap(),
            Profile::BasicWl
        );
        assert_eq!(
            Profile::parse("urn:order-x.eu:1p0:comfort").unwrap(),
            Profile::Comfort
        );
        assert_eq!(
            Profile::parse("urn:cen.eu:en16931:2017").unwrap(),
            Profile::En16931
        );
    }

    #[test]
    fn a_qualified_urn_is_cut_at_the_hash() {
        // `#compliant#urn:factur-x.eu:1p0:basic`: the last segment is `basic`.
        assert_eq!(
            Profile::parse("urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:basic").unwrap(),
            Profile::Basic
        );
        assert_eq!(
            Profile::parse(
                "urn:cen.eu:en16931:2017#conformant#urn.cpro.gouv.fr:1p0:extended-ctc-fr"
            )
            .unwrap(),
            Profile::ExtendedCtcFr
        );
    }

    #[test]
    fn an_unknown_identifier_is_reported() {
        assert!(Profile::parse("garbage").is_err());
        assert!(Profile::parse("urn:factur-x.eu:1p0:unknown").is_err());
    }

    #[test]
    fn the_string_and_display_round_trip() {
        for profile in Profile::ALL {
            assert_eq!(Profile::parse(profile.as_str()), Ok(*profile));
            assert_eq!(profile.to_string(), profile.as_str());
        }
    }
}
