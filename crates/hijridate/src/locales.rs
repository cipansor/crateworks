//! Month, day and era names in the languages the reference publishes.
//!
//! The English names are the ones the crate exposes as `month_name` and
//! `day_name`. The other three languages are here so that a caller can present
//! a date in the language the user reads, without carrying its own tables.
//!
//! The tables are transcribed from the `hijridate` Python package, which is the
//! reference for this crate, so a name that differs there is a defect here.

/// A language the month, day and era names are available in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Language {
    /// English.
    English,
    /// Arabic.
    Arabic,
    /// Bengali.
    Bengali,
    /// Turkish.
    Turkish,
}

impl Language {
    /// Every language this crate knows.
    pub const ALL: &'static [Self] = &[Self::English, Self::Arabic, Self::Bengali, Self::Turkish];

    /// The two-letter language tag, such as `"ar"`.
    #[must_use]
    pub const fn tag(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Arabic => "ar",
            Self::Bengali => "bn",
            Self::Turkish => "tr",
        }
    }

    /// The language for a tag such as `"ar"` or `"ar-SA"`.
    ///
    /// Only the first two characters are read, so a full BCP 47 tag works. The
    /// match is case-insensitive.
    #[must_use]
    pub fn from_tag(tag: &str) -> Option<Self> {
        let head: String = tag.chars().take(2).collect::<String>().to_lowercase();
        Self::ALL.iter().copied().find(|l| l.tag() == head)
    }

    /// The twelve Hijri month names, Muharram first.
    #[must_use]
    pub const fn hijri_month_names(self) -> &'static [&'static str; 12] {
        match self {
            Self::English => &EN_HIJRI_MONTHS,
            Self::Arabic => &AR_HIJRI_MONTHS,
            Self::Bengali => &BN_HIJRI_MONTHS,
            Self::Turkish => &TR_HIJRI_MONTHS,
        }
    }

    /// The twelve Gregorian month names, January first.
    #[must_use]
    pub const fn gregorian_month_names(self) -> &'static [&'static str; 12] {
        match self {
            Self::English => &EN_GREGORIAN_MONTHS,
            Self::Arabic => &AR_GREGORIAN_MONTHS,
            Self::Bengali => &BN_GREGORIAN_MONTHS,
            Self::Turkish => &TR_GREGORIAN_MONTHS,
        }
    }

    /// The seven day names, Monday first.
    #[must_use]
    pub const fn day_names(self) -> &'static [&'static str; 7] {
        match self {
            Self::English => &EN_DAYS,
            Self::Arabic => &AR_DAYS,
            Self::Bengali => &BN_DAYS,
            Self::Turkish => &TR_DAYS,
        }
    }

    /// The era notation for a Hijri year, such as `"AH"` or `"هـ"`.
    #[must_use]
    pub const fn hijri_notation(self) -> &'static str {
        match self {
            Self::English => "AH",
            Self::Arabic => "هـ",
            Self::Bengali => "হিজরি",
            Self::Turkish => "Hicri",
        }
    }

    /// The era notation for a Gregorian year, such as `"CE"` or `"م"`.
    #[must_use]
    pub const fn gregorian_notation(self) -> &'static str {
        match self {
            Self::English => "CE",
            Self::Arabic => "م",
            Self::Bengali => "খ্রিস্টাব্দ",
            Self::Turkish => "Miladi",
        }
    }
}

const EN_HIJRI_MONTHS: [&str; 12] = [
    "Muharram",
    "Safar",
    "Rabi' al-Awwal",
    "Rabi' al-Thani",
    "Jumada al-Ula",
    "Jumada al-Akhirah",
    "Rajab",
    "Sha'ban",
    "Ramadan",
    "Shawwal",
    "Dhu al-Qa'dah",
    "Dhu al-Hijjah",
];

const AR_HIJRI_MONTHS: [&str; 12] = [
    "محرم",
    "صفر",
    "ربيع الأول",
    "ربيع الثاني",
    "جمادى الأولى",
    "جمادى الآخرة",
    "رجب",
    "شعبان",
    "رمضان",
    "شوال",
    "ذو القعدة",
    "ذو الحجة",
];

const BN_HIJRI_MONTHS: [&str; 12] = [
    "মুহাররম",
    "সফর",
    "রবিউল আউয়াল",
    "রবিউস সানী",
    "জুমাদাল উলা",
    "জুমাদাস সানী",
    "রজব",
    "শাবান",
    "রমজান",
    "শাওয়াল",
    "জিলক্বদ",
    "জিলহজ",
];

const TR_HIJRI_MONTHS: [&str; 12] = [
    "Muharrem",
    "Safer",
    "Rebiülevvel",
    "Rebiülahir",
    "Cemaziyelevvel",
    "Cemaziyelahir",
    "Recep",
    "Şaban",
    "Ramazan",
    "Şevval",
    "Zilkade",
    "Zilhicce",
];

const EN_GREGORIAN_MONTHS: [&str; 12] = [
    "January",
    "February",
    "March",
    "April",
    "May",
    "June",
    "July",
    "August",
    "September",
    "October",
    "November",
    "December",
];

const AR_GREGORIAN_MONTHS: [&str; 12] = [
    "يناير",
    "فبراير",
    "مارس",
    "أبريل",
    "مايو",
    "يونيو",
    "يوليو",
    "أغسطس",
    "سبتمبر",
    "أكتوبر",
    "نوفمبر",
    "ديسمبر",
];

const BN_GREGORIAN_MONTHS: [&str; 12] = [
    "জানুয়ারি",
    "ফেব্রুয়ারি",
    "মার্চ",
    "এপ্রিল",
    "মে",
    "জুন",
    "জুলাই",
    "আগস্ট",
    "সেপ্টেম্বর",
    "অক্টোবর",
    "নভেম্বর",
    "ডিসেম্বর",
];

const TR_GREGORIAN_MONTHS: [&str; 12] = [
    "Ocak", "Şubat", "Mart", "Nisan", "Mayıs", "Haziran", "Temmuz", "Ağustos", "Eylül", "Ekim",
    "Kasım", "Aralık",
];

const EN_DAYS: [&str; 7] = [
    "Monday",
    "Tuesday",
    "Wednesday",
    "Thursday",
    "Friday",
    "Saturday",
    "Sunday",
];

const AR_DAYS: [&str; 7] = [
    "الإثنين",
    "الثلاثاء",
    "الأربعاء",
    "الخميس",
    "الجمعة",
    "السبت",
    "الأحد",
];

const BN_DAYS: [&str; 7] = [
    "সোমবার",
    "মঙ্গলবার",
    "বুধবার",
    "বৃহস্পতিবার",
    "শুক্রবার",
    "শনিবার",
    "রবিবার",
];

const TR_DAYS: [&str; 7] = [
    "Pazartesi",
    "Salı",
    "Çarşamba",
    "Perşembe",
    "Cuma",
    "Cumartesi",
    "Pazar",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_a_full_set_of_names() {
        for language in Language::ALL {
            assert_eq!(language.hijri_month_names().len(), 12);
            assert_eq!(language.gregorian_month_names().len(), 12);
            assert_eq!(language.day_names().len(), 7);
            assert!(!language.hijri_notation().is_empty());
            assert!(!language.gregorian_notation().is_empty());
            // No name may be empty, which would be a transcription slip.
            for name in language
                .hijri_month_names()
                .iter()
                .chain(language.gregorian_month_names())
                .chain(language.day_names())
            {
                assert!(!name.is_empty(), "{language:?} has an empty name");
            }
        }
    }

    #[test]
    fn language_tags_round_trip_and_accept_a_region() {
        for language in Language::ALL {
            assert_eq!(Language::from_tag(language.tag()), Some(*language));
        }
        assert_eq!(Language::from_tag("AR"), Some(Language::Arabic));
        assert_eq!(Language::from_tag("ar-SA"), Some(Language::Arabic));
        assert_eq!(Language::from_tag("zh"), None);
    }

    #[test]
    fn english_tables_match_the_crate_constants() {
        // The English tables must agree with the names the crate already
        // exposed, or `month_name` and `month_name_in(English)` would differ.
        assert_eq!(Language::English.hijri_month_names()[8], "Ramadan");
        assert_eq!(Language::English.gregorian_month_names()[1], "February");
        assert_eq!(Language::English.day_names()[2], "Wednesday");
        assert_eq!(Language::English.hijri_notation(), crate::HIJRI_NOTATION);
        assert_eq!(
            Language::English.gregorian_notation(),
            crate::GREGORIAN_NOTATION
        );
    }
}
