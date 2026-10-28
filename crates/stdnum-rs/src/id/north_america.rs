//! United States identifiers.
//!
//! The SSN and the ITIN share a shape: three digits, two digits, four digits,
//! written with or without the dashes. Neither carries a check digit, so the
//! check is the range of the area and group fields, and both are cross-checked
//! against `python-stdnum` by `tests/fixtures/stdnum_vectors.tsv`.

use crate::{Error, Result, checksum};

/// Splits a nine-digit number written in the three-two-four grouping.
///
/// The reference matches `\d{3}-?\d{2}-?\d{4}`, so a dash is allowed only
/// between the groups and not anywhere else. A number that merely *contains*
/// nine digits and a dash, such as `1112-23333`, is rejected: the dash sits
/// inside a group. Returning the three fields keeps the caller from repeating
/// the shape check.
fn split_grouped(s: &str) -> Option<[&str; 3]> {
    let b = s.as_bytes();
    let digits = |r: std::ops::Range<usize>| b[r].iter().all(u8::is_ascii_digit);
    match b.len() {
        9 if digits(0..9) => Some([&s[0..3], &s[3..5], &s[5..9]]),
        10 if b[3] == b'-' && digits(0..3) && digits(4..10) => {
            Some([&s[0..3], &s[4..6], &s[6..10]])
        }
        10 if b[6] == b'-' && digits(0..6) && digits(7..10) => {
            Some([&s[0..3], &s[3..5], &s[7..10]])
        }
        11 if b[3] == b'-' && b[6] == b'-' && digits(0..3) && digits(4..6) && digits(7..11) => {
            Some([&s[0..3], &s[4..6], &s[7..11]])
        }
        _ => None,
    }
}

/// United States: Social Security Number and Individual Taxpayer
/// Identification Number.
pub(super) mod united_states {
    use super::*;

    /// SSNs the Social Security Administration never issued but that circulate
    /// as examples. The reference refuses them so a number copied from a
    /// textbook is not mistaken for a real one.
    const SSN_BLACKLIST: [&str; 3] = ["078-05-1120", "457-55-5462", "219-09-9999"];

    /// Validates a United States Social Security Number.
    ///
    /// Nine digits in the three-two-four grouping. There is no check digit, so
    /// only the field ranges can be checked: the area is never `000` or `666`
    /// and never starts with `9`, the group is never `00` and the serial is
    /// never `0000`.
    pub(in crate::id) fn validate_ssn(input: &str) -> Result<()> {
        let s = input.trim();
        let Some([area, group, serial]) = split_grouped(s) else {
            return Err(Error::InvalidPrefix);
        };
        if area == "000" || area == "666" || area.starts_with('9') {
            return Err(Error::OutOfRange);
        }
        if group == "00" || serial == "0000" {
            return Err(Error::OutOfRange);
        }
        // The reference compares the number in its formatted shape, so a
        // dashless input is grouped before the blacklist lookup.
        if s.len() == 9 {
            let grouped = format!("{area}-{group}-{serial}");
            if SSN_BLACKLIST.contains(&grouped.as_str()) {
                return Err(Error::OutOfRange);
            }
        } else if SSN_BLACKLIST.contains(&s) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Validates a United States Individual Taxpayer Identification Number.
    ///
    /// Same shape as an SSN, but the area always starts with `9` and the group
    /// is one of 70 to 99 except 89 and 93, which are reserved.
    pub(in crate::id) fn validate_itin(input: &str) -> Result<()> {
        let s = input.trim();
        let Some([area, group, _serial]) = split_grouped(s) else {
            return Err(Error::InvalidPrefix);
        };
        if !area.starts_with('9') {
            return Err(Error::OutOfRange);
        }
        let group: u32 = group.parse().map_err(|_| Error::InvalidPrefix)?;
        if !(70..=99).contains(&group) || matches!(group, 89 | 93) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Validates a United States ABA routing transit number.
    ///
    /// Nine digits whose last is a checksum over the first eight: the first,
    /// fourth and seventh digits are weighted seven, the second, fifth and
    /// eighth three, and the third and sixth nine. The reference deletes no
    /// separators, so a dashed number is rejected rather than compacted.
    pub(in crate::id) fn validate_rtn(input: &str) -> Result<()> {
        let s = input.trim();
        let digits = checksum::digits_only(s)?;
        checksum::expect_len(&digits, 9)?;
        let weighted = 7 * (u32::from(digits[0]) + u32::from(digits[3]) + u32::from(digits[6]))
            + 3 * (u32::from(digits[1]) + u32::from(digits[4]) + u32::from(digits[7]))
            + 9 * (u32::from(digits[2]) + u32::from(digits[5]));
        if (weighted % 10) as u8 != digits[8] {
            return Err(Error::BadChecksum);
        }
        Ok(())
    }

    /// The area numbers an EIN prefix may carry, taken from the IRS list the
    /// reference ships. An area outside it was never assigned.
    const EIN_AREAS: &[&str] = &[
        "01", "02", "03", "04", "05", "06", "10", "11", "12", "13", "14", "15", "16", "20", "21",
        "22", "23", "24", "25", "26", "27", "30", "31", "32", "33", "34", "35", "36", "37", "38",
        "39", "40", "41", "42", "43", "44", "45", "46", "47", "48", "50", "51", "52", "53", "54",
        "55", "56", "57", "58", "59", "60", "61", "62", "63", "64", "65", "66", "67", "68", "71",
        "72", "73", "74", "75", "76", "77", "80", "81", "82", "83", "84", "85", "86", "87", "88",
        "90", "91", "92", "93", "94", "95", "98", "99",
    ];

    /// Validates a United States Employer Identification Number.
    ///
    /// Nine digits in the two-seven grouping, optionally written with the
    /// hyphen after the area. There is no check digit; the only test is that
    /// the two-digit area is one the IRS has assigned.
    pub(in crate::id) fn validate_ein(input: &str) -> Result<()> {
        let s = input.trim();
        let area: String = match s.len() {
            9 if s.bytes().all(|b| b.is_ascii_digit()) => s[..2].to_string(),
            10 if s.as_bytes()[2] == b'-'
                && s.bytes()
                    .enumerate()
                    .all(|(i, b)| i == 2 || b.is_ascii_digit()) =>
            {
                s[..2].to_string()
            }
            other => {
                return Err(Error::WrongLength {
                    expected: 9,
                    found: other,
                });
            }
        };
        if !EIN_AREAS.contains(&area.as_str()) {
            return Err(Error::OutOfRange);
        }
        Ok(())
    }

    /// Validates a United States Preparer Tax Identification Number.
    ///
    /// A capital `P` followed by eight digits, optionally written with a
    /// hyphen after the `P`. There is no check digit.
    pub(in crate::id) fn validate_ptin(input: &str) -> Result<()> {
        let s = input.trim();
        let cleaned = s.replace('-', "");
        let bytes = cleaned.as_bytes();
        if bytes.len() != 9 || bytes[0] != b'P' || !bytes[1..].iter().all(u8::is_ascii_digit) {
            return Err(Error::InvalidPrefix);
        }
        Ok(())
    }

    /// Validates a United States Adoption Taxpayer Identification Number.
    ///
    /// The same nine-digit shape as a Social Security number. The reference
    /// accepts the hyphen after the third and after the fifth digit
    /// independently, so `123456789`, `123-45-6789` and `123-456789` are all
    /// well formed, while a hyphen anywhere else is not. There is no check
    /// digit and no other rule.
    pub(in crate::id) fn validate_atin(input: &str) -> Result<()> {
        let s = input.trim();
        let mut digits = 0;
        for (i, c) in s.chars().enumerate() {
            if c.is_ascii_digit() {
                digits += 1;
            } else if c != '-' || (i != 3 && i != 6) {
                return Err(Error::InvalidPrefix);
            }
        }
        if digits != 9 {
            return Err(Error::WrongLength {
                expected: 9,
                found: digits,
            });
        }
        Ok(())
    }

    /// Validates a United States Taxpayer Identification Number in any of its
    /// five forms.
    ///
    /// The reference tries the Social Security, Individual Taxpayer, Employer
    /// Identification, Preparer Tax Identification and Adoption Taxpayer
    /// numbers in turn and accepts the number when any of them does. The order
    /// matters only for the error reported, not the verdict, so this returns
    /// the first success and reports the last failure otherwise.
    pub(in crate::id) fn validate_tin(input: &str) -> Result<()> {
        let s = input.trim();
        validate_ssn(s)
            .or_else(|_| validate_itin(s))
            .or_else(|_| validate_ein(s))
            .or_else(|_| validate_ptin(s))
            .or_else(|_| validate_atin(s))
            .map_err(|_| Error::InvalidPrefix)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Error;

    #[test]
    fn social_security_number_examples() {
        assert!(united_states::validate_ssn("536-90-4399").is_ok());
        assert!(united_states::validate_ssn("536904399").is_ok());
        // A dash inside a group is not a valid shape.
        assert_eq!(
            united_states::validate_ssn("1112-23333"),
            Err(Error::InvalidPrefix)
        );
        // An area of 000 or 666, or one starting with 9, is never issued.
        assert_eq!(
            united_states::validate_ssn("666-00-0000"),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            united_states::validate_ssn("900-12-3456"),
            Err(Error::OutOfRange)
        );
        // The advertising-range example is blacklisted in both shapes.
        assert_eq!(
            united_states::validate_ssn("078-05-1120"),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            united_states::validate_ssn("078051120"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn taxpayer_identification_number_examples() {
        assert!(united_states::validate_itin("912-90-3456").is_ok());
        // The area must start with 9 and the group must be in 70..=99.
        assert_eq!(
            united_states::validate_itin("123-45-6789"),
            Err(Error::OutOfRange)
        );
        assert_eq!(
            united_states::validate_itin("912-93-4567"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn routing_transit_number_examples() {
        assert!(united_states::validate_rtn("111000025").is_ok());
        // Nine digits only: the reference strips no separators.
        assert!(united_states::validate_rtn("11100002").is_err());
        assert_eq!(
            united_states::validate_rtn("112000025"),
            Err(Error::BadChecksum)
        );
    }
}
