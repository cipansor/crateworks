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
