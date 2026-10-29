//! Constants of the Open Location Code algorithm.
//!
//! The values are the reference's, in its own names, so that the port can be
//! read against it line by line. The two a caller needs are re-exported from the
//! crate root.

/// The characters a code is built from, in value order.
///
/// The set omits the vowels and the easily confused letters, so a code is
/// unlikely to spell a word and a misread letter is likely to be outside the
/// set rather than inside it.
pub(crate) const CODE_ALPHABET: [char; 20] = [
    '2', '3', '4', '5', '6', '7', '8', '9', 'C', 'F', 'G', 'H', 'J', 'M', 'P', 'Q', 'R', 'V', 'W',
    'X',
];

/// The separator that splits a code into two parts for legibility.
pub(crate) const SEPARATOR: char = '+';

/// How many characters sit before the separator in a full code.
pub(crate) const SEPARATOR_POSITION: usize = 8;

/// The character that pads a code shorter than the separator position.
pub(crate) const PADDING_CHARACTER: char = '0';

/// The base the pair section is written in.
pub(crate) const ENCODING_BASE: i64 = 20;

/// The largest latitude, in degrees.
pub(crate) const LATITUDE_MAX: f64 = 90.0;

/// The largest longitude, in degrees.
pub(crate) const LONGITUDE_MAX: f64 = 180.0;

/// The fewest significant characters a code may carry.
pub(crate) const MIN_CODE_LENGTH: usize = 2;

/// The most significant characters a code may carry. Digits past this are
/// ignored, so a code of any length is truncated rather than rejected.
pub const MAX_CODE_LENGTH: usize = 15;

/// The length of the pair section, before the grid refinement begins.
pub const PAIR_CODE_LENGTH: usize = 10;

/// The place value of the most significant pair.
pub(crate) const PAIR_FIRST_PLACE_VALUE: i64 = 160_000; // 20^4

/// The inverse precision of the pair section.
pub(crate) const PAIR_PRECISION: i64 = 8_000; // 20^3

/// The height and width, in degrees, of each pair position.
pub(crate) const PAIR_RESOLUTIONS: [f64; 5] = [20.0, 1.0, 0.05, 0.0025, 0.000125];

/// The number of grid-refinement digits.
pub(crate) const GRID_CODE_LENGTH: usize = 5;

/// The number of grid columns; a grid digit's column is its value modulo this.
pub(crate) const GRID_COLUMNS: i64 = 4;

/// The number of grid rows; a grid digit's row is its value divided by
/// [`GRID_COLUMNS`].
pub(crate) const GRID_ROWS: i64 = 5;

/// The place value of the most significant grid row.
pub(crate) const GRID_LAT_FIRST_PLACE_VALUE: i64 = 625; // 5^4

/// The place value of the most significant grid column.
pub(crate) const GRID_LNG_FIRST_PLACE_VALUE: i64 = 256; // 4^4

/// What to multiply a latitude by to reach an integer value at full precision.
pub(crate) const FINAL_LAT_PRECISION: i64 = 25_000_000; // 8000 * 5^5

/// What to multiply a longitude by to reach an integer value at full precision.
pub(crate) const FINAL_LNG_PRECISION: i64 = 8_192_000; // 8000 * 4^5

/// The shortest code that can be shortened.
pub(crate) const MIN_TRIMMABLE_CODE_LEN: usize = 6;
