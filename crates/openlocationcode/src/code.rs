//! The Open Location Code algorithm.
//!
//! A port of the reference implementation in `google/open-location-code`
//! (Apache-2.0). The function names are the reference's so the two can be read
//! side by side, but they are Rust snake_case here.

use crate::consts::{
    CODE_ALPHABET, ENCODING_BASE, FINAL_LAT_PRECISION, FINAL_LNG_PRECISION, GRID_CODE_LENGTH,
    GRID_COLUMNS, GRID_LAT_FIRST_PLACE_VALUE, GRID_LNG_FIRST_PLACE_VALUE, GRID_ROWS, LATITUDE_MAX,
    LONGITUDE_MAX, MAX_CODE_LENGTH, MIN_CODE_LENGTH, MIN_TRIMMABLE_CODE_LEN, PADDING_CHARACTER,
    PAIR_CODE_LENGTH, PAIR_FIRST_PLACE_VALUE, PAIR_PRECISION, PAIR_RESOLUTIONS, SEPARATOR,
    SEPARATOR_POSITION,
};
use crate::{CodeArea, Error, Result};

/// The value a character stands for, or `None` if it is not in the alphabet.
///
/// The reference calls this `CODE_ALPHABET.find`; every caller has already
/// checked the character is valid, so the `None` arm is unreachable there.
fn code_value(c: char) -> Option<i64> {
    CODE_ALPHABET.iter().position(|&a| a == c).map(|i| i as i64)
}

/// Clips a latitude to the range the codes cover, -90 to 90.
#[must_use]
pub fn clip_latitude(latitude: f64) -> f64 {
    latitude.clamp(-LATITUDE_MAX, LATITUDE_MAX)
}

/// Folds a longitude into the range -180 to 180, 180 itself excluded.
#[must_use]
pub fn normalize_longitude(longitude: f64) -> f64 {
    let mut value = longitude;
    while value < -LONGITUDE_MAX {
        value += LONGITUDE_MAX * 2.0;
    }
    while value >= LONGITUDE_MAX {
        value -= LONGITUDE_MAX * 2.0;
    }
    value
}

/// The latitude precision, in degrees, of a code of the given length.
///
/// Lengths up to ten have the same precision on both axes; past ten the grid
/// method has fewer columns than rows, so the two diverge.
#[must_use]
pub fn compute_latitude_precision(code_length: usize) -> f64 {
    if code_length <= PAIR_CODE_LENGTH {
        (ENCODING_BASE as f64).powf((code_length as f64 / -2.0 + 2.0).floor())
    } else {
        (ENCODING_BASE as f64).powf(-3.0)
            / GRID_ROWS.pow((code_length - PAIR_CODE_LENGTH) as u32) as f64
    }
}

/// Whether `code` is a well-formed code of any length.
///
/// A code must contain exactly one separator, at an even position no later than
/// the eighth character; any padding must be a single run of an even number of
/// zeros that ends the code; and every other character must be in the alphabet.
#[must_use]
pub fn is_valid(code: &str) -> bool {
    let separators = code.matches(SEPARATOR).count();
    let Some(separator) = code.find(SEPARATOR) else {
        return false;
    };
    if code.len() == 1 || separators > 1 {
        return false;
    }
    if separator > SEPARATOR_POSITION || separator % 2 == 1 {
        return false;
    }
    if let Some(pad) = code.find(PADDING_CHARACTER) {
        // Padding is only legal in a full code, so a short code that carries it
        // is rejected, and it may never start the code.
        if separator < SEPARATOR_POSITION || pad == 0 {
            return false;
        }
        let end = code.rfind(PADDING_CHARACTER).unwrap() + 1;
        let padding = &code[pad..end];
        if padding.len() % 2 == 1 || padding.chars().any(|c| c != PADDING_CHARACTER) {
            return false;
        }
        // A code long enough to end with the separator must do so.
        if !code.ends_with(SEPARATOR) {
            return false;
        }
    }
    // A single character after the separator is not a legal refinement.
    if code.len() - separator - 1 == 1 {
        return false;
    }
    code.chars().all(|c| {
        let upper = c.to_ascii_uppercase();
        upper == SEPARATOR || upper == PADDING_CHARACTER || code_value(upper).is_some()
    })
}

/// Whether `code` is a valid short code.
///
/// A short code is a full code with four or more leading characters removed, so
/// its separator sits before the eighth position.
#[must_use]
pub fn is_short(code: &str) -> bool {
    is_valid(code) && code.find(SEPARATOR).is_some_and(|s| s < SEPARATOR_POSITION)
}

/// Whether `code` is a valid full code.
///
/// Not every sequence of valid characters decodes to a legal location: the
/// first character decides a latitude and the second a longitude, and a code
/// whose leading pair falls outside the globe is rejected here.
#[must_use]
pub fn is_full(code: &str) -> bool {
    if !is_valid(code) || is_short(code) {
        return false;
    }
    // The first character's value times the base is twice the latitude it can
    // reach; the same for the second and the longitude.
    let first_lat = code_value(code.chars().next().unwrap().to_ascii_uppercase()).unwrap_or(0);
    if first_lat * ENCODING_BASE >= (LATITUDE_MAX as i64) * 2 {
        return false;
    }
    if let Some(second) = code.chars().nth(1) {
        let first_lng = code_value(second.to_ascii_uppercase()).unwrap_or(0);
        if first_lng * ENCODING_BASE >= (LONGITUDE_MAX as i64) * 2 {
            return false;
        }
    }
    true
}

/// Converts a latitude and longitude into the integer representation.
///
/// This is exposed because the reference exposes it for testing; an ordinary
/// caller wants [`encode`]. Latitude is clipped to the poles and longitude is
/// normalised, so a value outside the globe is folded in rather than refused.
#[must_use]
pub fn location_to_integers(latitude: f64, longitude: f64) -> (i64, i64) {
    let mut lat = (latitude * FINAL_LAT_PRECISION as f64).floor() as i64;
    lat += (LATITUDE_MAX as i64) * FINAL_LAT_PRECISION;
    if lat < 0 {
        lat = 0;
    } else if lat >= 2 * (LATITUDE_MAX as i64) * FINAL_LAT_PRECISION {
        lat = 2 * (LATITUDE_MAX as i64) * FINAL_LAT_PRECISION - 1;
    }

    let mut lng = (longitude * FINAL_LNG_PRECISION as f64).floor() as i64;
    lng += (LONGITUDE_MAX as i64) * FINAL_LNG_PRECISION;
    let span = 2 * (LONGITUDE_MAX as i64) * FINAL_LNG_PRECISION;
    if !(0..span).contains(&lng) {
        // Rust's `%` keeps the sign of the dividend, so a negative value needs
        // one more span added to land back in range.
        lng = lng.rem_euclid(span);
    }
    (lat, lng)
}

/// Encodes a latitude and longitude into a code of the requested length.
///
/// The length is the number of significant characters, the separator excluded.
/// Ten is the usual choice, about 13 by 13 metres; eleven or twelve refine it
/// further, and past fifteen the extra characters are dropped.
///
/// # Errors
///
/// Returns [`Error::InvalidCodeLength`] if the length is below two, or is odd
/// and below ten (an odd pair section would give a cell twenty times taller
/// than it is wide).
pub fn encode(latitude: f64, longitude: f64, code_length: usize) -> Result<String> {
    let (lat, lng) = location_to_integers(latitude, longitude);
    encode_integers(lat, lng, code_length)
}

/// Encodes the integer representation of a location into a code.
///
/// Exposed for testing, as in the reference; most callers want [`encode`]. The
/// integers come from [`location_to_integers`].
///
/// # Errors
///
/// As [`encode`].
pub fn encode_integers(mut lat: i64, mut lng: i64, code_length: usize) -> Result<String> {
    if code_length < MIN_CODE_LENGTH || (code_length < PAIR_CODE_LENGTH && code_length % 2 == 1) {
        return Err(Error::InvalidCodeLength(code_length));
    }
    let code_length = code_length.min(MAX_CODE_LENGTH);
    let mut code = String::with_capacity(code_length + 1);

    if code_length > PAIR_CODE_LENGTH {
        // The grid refinement, least significant digit first.
        for _ in 0..GRID_CODE_LENGTH {
            let lat_digit = lat % GRID_ROWS;
            let lng_digit = lng % GRID_COLUMNS;
            let index = (lat_digit * GRID_COLUMNS + lng_digit) as usize;
            code.push(CODE_ALPHABET[index]);
            lat /= GRID_ROWS;
            lng /= GRID_COLUMNS;
        }
    } else {
        // Drop the grid digits so the pair section starts at the right place.
        lat /= GRID_ROWS.pow(GRID_CODE_LENGTH as u32);
        lng /= GRID_COLUMNS.pow(GRID_CODE_LENGTH as u32);
    }
    for _ in 0..PAIR_CODE_LENGTH / 2 {
        code.push(CODE_ALPHABET[(lng % ENCODING_BASE) as usize]);
        lng /= ENCODING_BASE;
        code.push(CODE_ALPHABET[(lat % ENCODING_BASE) as usize]);
        lat /= ENCODING_BASE;
    }
    // `code` was built least-significant first, so reverse it and insert the
    // separator at its position.
    let mut digits: Vec<char> = code.chars().rev().collect();
    digits.insert(SEPARATOR_POSITION, SEPARATOR);
    let full: String = digits.into_iter().collect();

    if code_length >= SEPARATOR_POSITION {
        Ok(full.chars().take(code_length + 1).collect())
    } else {
        // Pad the pair section out to the separator, then re-add it.
        let mut padded: String = full.chars().take(code_length).collect();
        padded.push_str(
            &PADDING_CHARACTER
                .to_string()
                .repeat(SEPARATOR_POSITION - code_length),
        );
        padded.push(SEPARATOR);
        Ok(padded)
    }
}

/// Decodes a full code into the area it stands for.
///
/// # Errors
///
/// Returns [`Error::NotFullCode`] if `code` is not a valid full code.
pub fn decode(code: &str) -> Result<CodeArea> {
    if !is_full(code) {
        return Err(Error::NotFullCode);
    }
    let cleaned: String = code
        .chars()
        .filter(|&c| c != SEPARATOR && c != PADDING_CHARACTER)
        .map(|c| c.to_ascii_uppercase())
        .take(MAX_CODE_LENGTH)
        .collect();

    let mut normal_lat: f64 = -(LATITUDE_MAX * PAIR_PRECISION as f64);
    let mut normal_lng: f64 = -(LONGITUDE_MAX * PAIR_PRECISION as f64);
    let mut grid_lat: f64 = 0.0;
    let mut grid_lng: f64 = 0.0;

    let digits = cleaned.len().min(PAIR_CODE_LENGTH);
    let mut place_value = PAIR_FIRST_PLACE_VALUE;
    let chars: Vec<char> = cleaned.chars().collect();
    let mut i = 0;
    while i < digits {
        normal_lat += code_value(chars[i]).unwrap() as f64 * place_value as f64;
        normal_lng += code_value(chars[i + 1]).unwrap() as f64 * place_value as f64;
        if i < digits - 2 {
            place_value /= ENCODING_BASE;
        }
        i += 2;
    }
    let mut lat_precision = place_value as f64 / PAIR_PRECISION as f64;
    let mut lng_precision = place_value as f64 / PAIR_PRECISION as f64;

    if cleaned.len() > PAIR_CODE_LENGTH {
        let mut row_value = GRID_LAT_FIRST_PLACE_VALUE;
        let mut column_value = GRID_LNG_FIRST_PLACE_VALUE;
        let digits = cleaned.len().min(MAX_CODE_LENGTH);
        for (i, &ch) in chars.iter().enumerate().take(digits).skip(PAIR_CODE_LENGTH) {
            let digit = code_value(ch).unwrap();
            grid_lat += (digit / GRID_COLUMNS) as f64 * row_value as f64;
            grid_lng += (digit % GRID_COLUMNS) as f64 * column_value as f64;
            if i < digits - 1 {
                row_value /= GRID_ROWS;
                column_value /= GRID_COLUMNS;
            }
        }
        lat_precision = row_value as f64 / FINAL_LAT_PRECISION as f64;
        lng_precision = column_value as f64 / FINAL_LNG_PRECISION as f64;
    }

    let lat = normal_lat / PAIR_PRECISION as f64 + grid_lat / FINAL_LAT_PRECISION as f64;
    let lng = normal_lng / PAIR_PRECISION as f64 + grid_lng / FINAL_LNG_PRECISION as f64;
    Ok(CodeArea::new(
        lat,
        lng,
        lat + lat_precision,
        lng + lng_precision,
        cleaned.len().min(MAX_CODE_LENGTH),
    ))
}

/// Removes leading characters from a full code, using a nearby reference point.
///
/// The number of characters removed depends on how close the reference point is
/// to the code's center: at least four are removed, and up to eight, with the
/// gap filled by padding. The reference point must be within half the cell, so
/// that a slightly different point still recovers the same code.
///
/// # Errors
///
/// Returns [`Error::NotFullCode`] if `code` is not a valid full code,
/// [`Error::CannotShortenPadded`] if it already carries padding, and
/// [`Error::CodeTooShort`] if it is shorter than six characters.
pub fn shorten(code: &str, latitude: f64, longitude: f64) -> Result<String> {
    if !is_full(code) {
        return Err(Error::NotFullCode);
    }
    if code.contains(PADDING_CHARACTER) {
        return Err(Error::CannotShortenPadded);
    }
    let code = code.to_ascii_uppercase();
    let area = decode(&code)?;
    if area.code_length < MIN_TRIMMABLE_CODE_LEN {
        return Err(Error::CodeTooShort);
    }
    let latitude = clip_latitude(latitude);
    let longitude = normalize_longitude(longitude);
    let range = (area.latitude_center - latitude)
        .abs()
        .max((area.longitude_center - longitude).abs());
    // Walk from the coarsest removable pair to the finest; 0.3 rather than 0.5
    // leaves a margin so a nearby reference still recovers the same code.
    for i in (1..PAIR_RESOLUTIONS.len() - 1).rev() {
        if range < PAIR_RESOLUTIONS[i] * 0.3 {
            return Ok(code.chars().skip((i + 1) * 2).collect());
        }
    }
    Ok(code)
}

/// The ten-character code a reference point belongs to, truncated to a length.
///
/// Used by [`recover_nearest`] to rebuild the missing prefix of a short code.
fn prefix_by_reference(latitude: f64, longitude: f64, code_length: usize) -> String {
    let precision = compute_latitude_precision(code_length);
    let lat = (latitude / precision).floor() * precision;
    let lng = (longitude / precision).floor() * precision;
    let encoded = encode(lat, lng, PAIR_CODE_LENGTH).unwrap();
    encoded.chars().take(code_length).collect()
}

/// Recovers the nearest full code that matches a short code.
///
/// A short code omits its leading characters, which is what makes a Plus Code
/// usable as a street address. This rebuilds them from a reference point that
/// must be near the intended place; the result is the full code whose cell is
/// closest to that point, which is not always the cell the point is inside.
///
/// A full code is returned upper-cased and otherwise unchanged.
///
/// # Errors
///
/// Returns [`Error::InvalidCode`] if `code` is neither a short nor a full code.
pub fn recover_nearest(code: &str, latitude: f64, longitude: f64) -> Result<String> {
    if is_full(code) {
        return Ok(code.to_ascii_uppercase());
    }
    if !is_short(code) {
        return Err(Error::InvalidCode);
    }
    let latitude = clip_latitude(latitude);
    let longitude = normalize_longitude(longitude);
    let code = code.to_ascii_uppercase();
    let separator = code.find(SEPARATOR).unwrap();
    let padding_length = SEPARATOR_POSITION - separator;
    // The cell is 20^(2 - padding_length/2) degrees across; the exponent is a
    // whole number because the separator sits at an even position.
    let resolution = (ENCODING_BASE as f64).powf(2.0 - (padding_length as f64) / 2.0);
    let half_resolution = resolution / 2.0;

    let prefix = prefix_by_reference(latitude, longitude, padding_length);
    let area = decode(&format!("{prefix}{code}"))?;
    let mut center_lat = area.latitude_center;
    let mut center_lng = area.longitude_center;

    if latitude + half_resolution < center_lat && center_lat - resolution >= -LATITUDE_MAX {
        center_lat -= resolution;
    } else if latitude - half_resolution > center_lat && center_lat + resolution <= LATITUDE_MAX {
        center_lat += resolution;
    }
    if longitude + half_resolution < center_lng {
        center_lng -= resolution;
    } else if longitude - half_resolution > center_lng {
        center_lng += resolution;
    }
    encode(center_lat, center_lng, area.code_length)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_and_normalize_fold_out_of_range_values() {
        assert_eq!(clip_latitude(100.0), 90.0);
        assert_eq!(clip_latitude(-100.0), -90.0);
        assert_eq!(clip_latitude(45.0), 45.0);
        assert_eq!(normalize_longitude(190.0), -170.0);
        assert_eq!(normalize_longitude(-190.0), 170.0);
        assert_eq!(normalize_longitude(180.0), -180.0);
        assert_eq!(normalize_longitude(0.0), 0.0);
    }

    #[test]
    fn an_odd_short_length_is_rejected() {
        assert!(encode(0.0, 0.0, 1).is_err());
        assert!(encode(0.0, 0.0, 3).is_err());
        assert!(encode(0.0, 0.0, 2).is_ok());
        assert!(encode(0.0, 0.0, 4).is_ok());
        // An odd length at or above ten is legal: the grid handles the last one.
        assert!(encode(0.0, 0.0, 11).is_ok());
    }

    #[test]
    fn a_longer_code_is_a_smaller_area() {
        let ten = decode(&encode(47.365590, 8.524997, 10).unwrap()).unwrap();
        let eleven = decode(&encode(47.365590, 8.524997, 11).unwrap()).unwrap();
        assert!(eleven.latitude_height() < ten.latitude_height());
        assert!(eleven.longitude_width() < ten.longitude_width());
    }

    #[test]
    fn a_full_code_survives_a_shorten_and_recover_round_trip() {
        let code = encode(51.5074, -0.1278, 10).unwrap();
        let short = shorten(&code, 51.5074, -0.1278).unwrap();
        assert_eq!(recover_nearest(&short, 51.5074, -0.1278).unwrap(), code);
    }

    #[test]
    fn decode_rejects_a_short_code() {
        assert_eq!(decode("9G8F+6X"), Err(Error::NotFullCode));
    }
}
