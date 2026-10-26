#![doc = include_str!("../README.md")]
mod astronomy;
mod calculation;

pub use calculation::{
    HighLatitudeRule, Madhab, Method, Parameters, PolarCircleResolution, Prayer, PrayerTimes,
    Rounding, Shafaq, SunnahTimes, prayer_times, qibla, sunnah_times,
};
