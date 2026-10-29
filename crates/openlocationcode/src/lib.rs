//! Open Location Codes, also called Plus Codes.
//!
//! A Plus Code is a short sequence of characters that names a place on Earth,
//! for example `8FVC9G8F+6X` for a spot in Zurich. The code is derived from the
//! latitude and longitude, needs no registry, and can be written on a signpost
//! or read over the phone. The full ten-character form covers about 13 by 13
//! metres; eleven or twelve characters narrow it further.
//!
//! A code can also be *shortened* against a reference point that is already
//! known, dropping the leading characters two places share, so that only the
//! part that differs is written: `9G8F+6X` rather than the whole code. The
//! missing characters are rebuilt with [`recover_nearest`].
//!
//! ```
//! use openlocationcode::{encode, decode, recover_nearest, shorten};
//!
//! // Encode a point near Zurich to the default ten characters.
//! let code = encode(47.365590, 8.524997, 10).unwrap();
//! assert_eq!(code, "8FVC9G8F+6X");
//!
//! // Decode it back to the area it stands for.
//! let area = decode("8FVC9G8F+6X").unwrap();
//! assert!((area.latitude_center - 47.3655625).abs() < 1e-6);
//!
//! // Shorten it against a nearby point, then recover the full code.
//! let short = shorten("8FVC9G8F+6X", 47.365590, 8.524997).unwrap();
//! assert_eq!(short, "+6X");
//! assert_eq!(recover_nearest(&short, 47.365590, 8.524997).unwrap(), "8FVC9G8F+6X");
//! ```
//!
//! # What this crate is
//!
//! An independent Rust implementation of the Open Location Code algorithm
//! published by Google at
//! <https://github.com/google/open-location-code>, which is the reference. It
//! shares no code with it and has no dependencies, and every case in the
//! reference's own test data is replayed in `tests/`, so the two agree.

#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod code;
mod codearea;
mod consts;

pub use code::{
    clip_latitude, compute_latitude_precision, decode, encode, encode_integers, is_full, is_short,
    is_valid, location_to_integers, normalize_longitude, recover_nearest, shorten,
};
pub use codearea::CodeArea;
pub use consts::{MAX_CODE_LENGTH, PAIR_CODE_LENGTH};

use std::fmt;

/// Why a code or a coordinate was rejected.
///
/// The variants name the specific failure, so a caller can tell a malformed
/// code from one that is merely short, and both from a length the algorithm
/// cannot produce.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The code is not a valid full code.
    NotFullCode,
    /// The code is neither a valid short code nor a valid full code.
    InvalidCode,
    /// A code with padding cannot be shortened.
    CannotShortenPadded,
    /// A code shorter than six characters cannot be shortened.
    CodeTooShort,
    /// The requested code length is not one the algorithm can produce.
    ///
    /// Carries the length that was asked for. A length below two, or an odd
    /// length below ten, is rejected; a length above fifteen is truncated
    /// rather than refused.
    InvalidCodeLength(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFullCode => f.write_str("not a valid full Open Location Code"),
            Self::InvalidCode => f.write_str("not a valid short or full Open Location Code"),
            Self::CannotShortenPadded => f.write_str("a padded code cannot be shortened"),
            Self::CodeTooShort => f.write_str("a code must be at least six characters to shorten"),
            Self::InvalidCodeLength(len) => write!(f, "invalid Open Location Code length {len}"),
        }
    }
}

impl std::error::Error for Error {}

/// The result type used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
