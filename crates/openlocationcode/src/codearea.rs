//! The rectangle a code stands for.

use crate::consts::{LATITUDE_MAX, LONGITUDE_MAX};

/// The area a full code identifies.
///
/// A code names a rectangle, not a point. The rectangle is the one that
/// contains every location which encodes to the same code, and the fields are
/// its corners in degrees. A longer code gives a smaller rectangle: ten
/// characters is about 13 by 13 metres at the equator, and eleven about 3 by 4.
///
/// The latitude and longitude of the center are given directly, because that is
/// the point a caller normally wants; a point near the center is safer than a
/// corner, whose neighbours encode to a different code.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CodeArea {
    /// The latitude of the south edge, in degrees.
    pub latitude_low: f64,
    /// The longitude of the west edge, in degrees.
    pub longitude_low: f64,
    /// The latitude of the north edge, in degrees.
    pub latitude_high: f64,
    /// The longitude of the east edge, in degrees.
    pub longitude_high: f64,
    /// The latitude of the center, in degrees.
    pub latitude_center: f64,
    /// The longitude of the center, in degrees.
    pub longitude_center: f64,
    /// The number of significant characters in the code, separator excluded.
    pub code_length: usize,
}

impl CodeArea {
    /// Builds an area from its corners, clamping the center to the valid range.
    ///
    /// The clamp matters at the poles and the antimeridian, where the arithmetic
    /// midpoint of an edge can fall just outside the legal range; the reference
    /// clamps there for the same reason.
    pub(crate) fn new(
        latitude_low: f64,
        longitude_low: f64,
        latitude_high: f64,
        longitude_high: f64,
        code_length: usize,
    ) -> Self {
        let latitude_center =
            (latitude_low + (latitude_high - latitude_low) / 2.0).min(LATITUDE_MAX);
        let longitude_center =
            (longitude_low + (longitude_high - longitude_low) / 2.0).min(LONGITUDE_MAX);
        Self {
            latitude_low,
            longitude_low,
            latitude_high,
            longitude_high,
            latitude_center,
            longitude_center,
            code_length,
        }
    }

    /// The height of the area in degrees.
    #[must_use]
    pub fn latitude_height(&self) -> f64 {
        self.latitude_high - self.latitude_low
    }

    /// The width of the area in degrees.
    #[must_use]
    pub fn longitude_width(&self) -> f64 {
        self.longitude_high - self.longitude_low
    }
}
