//! The ISO 7064 check-digit algorithms, as public building blocks.
//!
//! ISO 7064 defines a family of check-digit schemes; the three that identifiers
//! in this crate use are MOD 11-2 (ISNI, ISBN-10), MOD 37-36 (GRid, ISAN) and
//! MOD 97-10 (LEI, the ISO 11649 creditor reference). Each is a pure function
//! over the payload, so it is exposed here rather than hidden inside the
//! identifier that happens to need it.

use crate::{Error, Result};

/// MOD 11-2: the check is valid when the whole number is congruent to 1 mod 11.
///
/// An `X` counts as ten, which is why the algorithm works over characters and
/// not digits. ISBN-10 uses it, and so does ISNI.
pub fn mod_11_2_checksum(number: &str) -> Option<u32> {
    let mut check = 0u32;
    for c in number.chars() {
        let value = if c == 'X' { 10 } else { c.to_digit(10)? };
        check = (2 * check + value) % 11;
    }
    Some(check)
}

/// The MOD 11-2 check character for `payload`, so a number can be generated.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` contains anything other
/// than a digit or an `X`.
pub fn mod_11_2_check_digit(payload: &str) -> Result<char> {
    let checksum = mod_11_2_checksum(payload)
        .ok_or_else(|| Error::InvalidCharacter(first_invalid(payload)))?;
    let c = (1 + 11 - (2 * checksum) % 11) % 11;
    Ok(if c == 10 {
        'X'
    } else {
        char::from(b'0' + c as u8)
    })
}

/// MOD 37-36: the alphabet is the digits then the letters, and a valid number
/// is congruent to 1 mod 37.
pub fn mod_37_36_checksum(number: &str) -> Option<u32> {
    const MODULUS: u32 = 36;
    let mut check = MODULUS / 2;
    for c in number.chars() {
        let value = c.to_digit(36)?;
        // A zero intermediate value is replaced by the modulus. The scheme's
        // definition says so, and without it the arithmetic would stick at
        // zero, which is a real difference for inputs such as "0000".
        let base = if check == 0 { MODULUS } else { check };
        check = ((base * 2) % (MODULUS + 1) + value) % MODULUS;
    }
    Some(check)
}

/// The MOD 37-36 check character for `payload`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` is not alphanumeric.
pub fn mod_37_36_check_digit(payload: &str) -> Result<char> {
    const MODULUS: u32 = 36;
    let checksum = mod_37_36_checksum(payload)
        .ok_or_else(|| Error::InvalidCharacter(first_invalid(payload)))?;
    let base = if checksum == 0 { MODULUS } else { checksum };
    let index = (1 + MODULUS - (base * 2) % (MODULUS + 1)) % MODULUS;
    Ok(std::char::from_digit(index, 36)
        .expect("index is below the alphabet size")
        .to_ascii_uppercase())
}

/// MOD 97-10: the number is read as a base-36 integer, letters expanded to two
/// decimal digits, and a valid number is congruent to 1 mod 97.
pub fn mod_97_10_checksum(number: &str) -> Option<u32> {
    let mut remainder = 0u32;
    for c in number.chars() {
        let value = c.to_digit(36)?;
        // A letter expands to two decimal digits, so it shifts by 100.
        let base = if value >= 10 { 100 } else { 10 };
        remainder = (remainder * base + value) % 97;
    }
    Some(remainder)
}

/// The two MOD 97-10 check digits for `payload`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` is not alphanumeric.
pub fn mod_97_10_check_digits(payload: &str) -> Result<String> {
    let with_zeros = format!("{payload}00");
    let checksum = mod_97_10_checksum(&with_zeros)
        .ok_or_else(|| Error::InvalidCharacter(first_invalid(payload)))?;
    Ok(format!("{:02}", 98 - checksum))
}

/// MOD 11-10: a valid number is congruent to 1 mod 11.
///
/// The running value is doubled modulo eleven, a zero is replaced by the
/// modulus, the digit is added, and the result is taken modulo ten. It is the
/// check that the German tax identifier (IdNr) uses.
pub fn mod_11_10_checksum(number: &str) -> Option<u32> {
    let mut check = 5u32;
    for c in number.chars() {
        let digit = c.to_digit(10)?;
        check = (((if check == 0 { 10 } else { check }) * 2) % 11 + digit) % 10;
    }
    Some(check)
}

/// The MOD 11-10 check digit for `payload`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` is not all digits.
pub fn mod_11_10_check_digit(payload: &str) -> Result<char> {
    let checksum = mod_11_10_checksum(payload)
        .ok_or_else(|| Error::InvalidCharacter(first_invalid(payload)))?;
    let base = if checksum == 0 { 10 } else { checksum };
    Ok(char::from(b'0' + ((1 + 10 - (base * 2) % 11) % 10) as u8))
}

/// MOD 37-2: the alphabet is the digits, the letters and `*`, and a valid
/// number is congruent to 1 mod 37.
///
/// A `*` counts as 36, the one symbol that makes the alphabet 37 long.
pub fn mod_37_2_checksum(number: &str) -> Option<u32> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ*";
    let mut check = 0u32;
    for c in number.chars() {
        let value = ALPHABET.iter().position(|&a| a == c as u8)? as u32;
        check = (2 * check + value) % 37;
    }
    Some(check)
}

/// The MOD 37-2 check character for `payload`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` carries a symbol outside
/// the digit, letter and `*` alphabet.
pub fn mod_37_2_check_digit(payload: &str) -> Result<char> {
    const ALPHABET: &[u8] = b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ*";
    let checksum = mod_37_2_checksum(payload)
        .ok_or_else(|| Error::InvalidCharacter(first_invalid(payload)))?;
    let index = ((1 + 37 - (2 * checksum) % 37) % 37) as usize;
    Ok(char::from(ALPHABET[index]))
}

/// The Damm operation table, a totally anti-symmetric quasigroup of order ten.
const DAMM_TABLE: [[u8; 10]; 10] = [
    [0, 3, 1, 7, 5, 9, 8, 6, 4, 2],
    [7, 0, 9, 2, 1, 5, 4, 8, 6, 3],
    [4, 2, 0, 6, 8, 7, 1, 3, 5, 9],
    [1, 7, 5, 0, 9, 8, 3, 4, 2, 6],
    [6, 1, 2, 3, 0, 4, 5, 9, 7, 8],
    [3, 6, 7, 4, 2, 0, 9, 5, 8, 1],
    [5, 8, 6, 9, 7, 2, 0, 1, 3, 4],
    [8, 9, 4, 5, 3, 6, 2, 0, 1, 7],
    [9, 4, 3, 8, 6, 1, 7, 2, 0, 5],
    [2, 5, 8, 1, 4, 3, 6, 7, 9, 0],
];

/// The Damm check over `number`; valid when the result is zero.
///
/// Damm catches every single-digit error and every adjacent transposition,
/// which Luhn does not.
#[must_use]
pub fn damm_checksum(number: &str) -> Option<u8> {
    let mut interim = 0usize;
    for c in number.chars() {
        let digit = c.to_digit(10)? as usize;
        interim = usize::from(DAMM_TABLE[interim][digit]);
    }
    Some(interim as u8)
}

/// The Damm check digit for `payload`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` is not all digits.
pub fn damm_check_digit(payload: &str) -> Result<u8> {
    damm_checksum(payload).ok_or_else(|| Error::InvalidCharacter(first_invalid(payload)))
}

/// The Verhoeff multiplication and permutation tables.
const VERHOEFF_MULTIPLY: [[u8; 10]; 10] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 2, 3, 4, 0, 6, 7, 8, 9, 5],
    [2, 3, 4, 0, 1, 7, 8, 9, 5, 6],
    [3, 4, 0, 1, 2, 8, 9, 5, 6, 7],
    [4, 0, 1, 2, 3, 9, 5, 6, 7, 8],
    [5, 9, 8, 7, 6, 0, 4, 3, 2, 1],
    [6, 5, 9, 8, 7, 1, 0, 4, 3, 2],
    [7, 6, 5, 9, 8, 2, 1, 0, 4, 3],
    [8, 7, 6, 5, 9, 3, 2, 1, 0, 4],
    [9, 8, 7, 6, 5, 4, 3, 2, 1, 0],
];

const VERHOEFF_PERMUTE: [[u8; 10]; 8] = [
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9],
    [1, 5, 7, 6, 2, 8, 3, 0, 9, 4],
    [5, 8, 0, 3, 7, 9, 6, 1, 4, 2],
    [8, 9, 1, 6, 0, 4, 3, 5, 2, 7],
    [9, 4, 5, 3, 1, 2, 6, 8, 7, 0],
    [4, 2, 8, 6, 5, 7, 3, 9, 0, 1],
    [2, 7, 9, 3, 8, 0, 6, 4, 1, 5],
    [7, 0, 4, 6, 9, 1, 3, 2, 5, 8],
];

/// The Verhoeff check over `number`; valid when the result is zero.
#[must_use]
pub fn verhoeff_checksum(number: &str) -> Option<u8> {
    let mut check = 0u8;
    for (i, c) in number.chars().rev().enumerate() {
        let digit = c.to_digit(10)? as usize;
        check = VERHOEFF_MULTIPLY[usize::from(check)][usize::from(VERHOEFF_PERMUTE[i % 8][digit])];
    }
    Some(check)
}

/// The Verhoeff check digit for `payload`.
///
/// # Errors
///
/// Returns [`Error::InvalidCharacter`] if `payload` is not all digits.
pub fn verhoeff_check_digit(payload: &str) -> Result<u8> {
    let with_zero = format!("{payload}0");
    let checksum = verhoeff_checksum(&with_zero)
        .ok_or_else(|| Error::InvalidCharacter(first_invalid(payload)))?;
    let position = VERHOEFF_MULTIPLY[usize::from(checksum)]
        .iter()
        .position(|v| *v == 0)
        .expect("the multiplication table has a zero in every row");
    Ok(position as u8)
}

/// The first character of `s` that is not an ASCII letter or digit, for the
/// error message. Returns `'?'` when every character is acceptable, which can
/// only happen if the caller already rejected the input.
fn first_invalid(s: &str) -> char {
    s.chars()
        .find(|c| !c.is_ascii_alphanumeric())
        .unwrap_or('?')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_11_2_matches_the_reference() {
        // python-stdnum: mod_11_2.checksum('12345') == 2, and the check
        // character that makes the number valid is '8'.
        assert_eq!(mod_11_2_checksum("12345"), Some(2));
        assert_eq!(mod_11_2_check_digit("12345").unwrap(), '8');
        assert_eq!(mod_11_2_checksum("X"), Some(10), "X counts as ten");
    }

    #[test]
    fn mod_97_10_matches_the_published_examples() {
        // The worked examples in the python-stdnum module docstring.
        assert_eq!(
            mod_97_10_check_digits("99991234567890121414").unwrap(),
            "90"
        );
        assert_eq!(mod_97_10_check_digits("4354111611551114").unwrap(), "31");
        assert_eq!(
            mod_97_10_check_digits("22181321402534321446701611").unwrap(),
            "35"
        );
        assert_eq!(
            mod_97_10_checksum("9999123456789012141490"),
            Some(1),
            "a number carrying its own check digits is congruent to 1"
        );
    }

    #[test]
    fn mod_37_36_matches_the_reference() {
        assert_eq!(mod_37_36_checksum("A1B2C3D4E5F6G7H8"), Some(5));
        assert_eq!(mod_37_36_check_digit("A1B2C3D4E5F6G7H8").unwrap(), 'R');
    }

    #[test]
    fn mod_37_36_treats_a_zero_intermediate_as_the_modulus() {
        // The reference writes `(check or modulus)`, so an intermediate value
        // of zero becomes the modulus rather than doubling to zero. Without
        // that the running value would stick at zero, so an input that reaches
        // zero, such as "0000", would give a different answer.
        assert_eq!(mod_37_36_checksum("0000"), Some(29));
        assert_eq!(mod_37_36_check_digit("0000").unwrap(), 'G');
    }

    #[test]
    fn damm_matches_the_reference() {
        assert_eq!(damm_checksum("572"), Some(4));
        assert_eq!(damm_check_digit("572").unwrap(), 4);
        assert_eq!(damm_checksum("5724"), Some(0), "a valid number is zero");
        assert_eq!(damm_checksum("816"), Some(4));
    }

    #[test]
    fn verhoeff_matches_the_reference() {
        assert_eq!(verhoeff_checksum("1234"), Some(1));
        assert_eq!(verhoeff_check_digit("1234").unwrap(), 0);
        assert_eq!(
            verhoeff_checksum("12340"),
            Some(0),
            "a valid number is zero"
        );
    }

    #[test]
    fn mod_11_10_matches_the_reference() {
        // The worked examples in the python-stdnum module docstring.
        assert_eq!(mod_11_10_checksum("794623"), Some(1));
        assert_eq!(mod_11_10_check_digit("79462").unwrap(), '3');
        assert_eq!(mod_11_10_check_digit("00200667308").unwrap(), '5');
    }

    #[test]
    fn mod_37_2_matches_the_reference() {
        // The worked examples in the python-stdnum module docstring, including
        // the `*` that makes the alphabet 37 long.
        assert_eq!(mod_37_2_checksum("G123489654321Y"), Some(1));
        assert_eq!(mod_37_2_check_digit("G123489654321").unwrap(), 'Y');
        assert_eq!(mod_37_2_check_digit("079").unwrap(), 'T');
    }

    #[test]
    fn rejects_non_alphanumeric_input() {
        assert_eq!(mod_97_10_checksum("12-34"), None);
        assert_eq!(damm_checksum("12a"), None);
        assert_eq!(verhoeff_checksum("12a"), None);
        assert_eq!(mod_11_2_checksum("12a"), None);
        assert_eq!(mod_37_36_checksum("12-34"), None);
    }
}
