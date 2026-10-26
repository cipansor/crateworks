//! Calculation methods, schools, and the parameters that define a prayer time.

use crate::astronomy::{self, SolarTime};

/// Which shadow length defines Asr.
///
/// The majority of schools take the time when a shadow equals the object's
/// height plus the shadow at noon; the Hanafi school takes twice that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Madhab {
    /// A shadow one times the object's height, plus the noon shadow.
    #[default]
    Shafi,
    /// A shadow two times the object's height, plus the noon shadow.
    Hanafi,
}

impl Madhab {
    /// The multiplier the shadow calculation uses.
    #[must_use]
    pub fn shadow_length(self) -> f64 {
        match self {
            Self::Shafi => 1.0,
            Self::Hanafi => 2.0,
        }
    }
}

/// How Fajr and Isha are bounded at latitudes where twilight never ends.
///
/// Above roughly 48 degrees the sun may not reach the twilight angle at all in
/// summer, so the prayers have to be defined some other way. Each rule picks a
/// fraction of the night.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HighLatitudeRule {
    /// Half the night for Fajr, half for Isha.
    #[default]
    MiddleOfTheNight,
    /// One seventh of the night, the earliest and latest a prayer is allowed.
    SeventhOfTheNight,
    /// A fraction of the night given by the twilight angle over sixty.
    TwilightAngle,
}

impl HighLatitudeRule {
    /// The night portions for Fajr and Isha.
    ///
    /// The twilight rule needs the angles, so the method is passed in.
    fn night_portions(self, fajr_angle: f64, isha_angle: f64) -> (f64, f64) {
        match self {
            Self::MiddleOfTheNight => (1.0 / 2.0, 1.0 / 2.0),
            Self::SeventhOfTheNight => (1.0 / 7.0, 1.0 / 7.0),
            Self::TwilightAngle => (fajr_angle / 60.0, isha_angle / 60.0),
        }
    }

    /// The rule to use at a latitude, when the caller has no preference.
    ///
    /// Above 48 degrees the seventh-of-the-night rule is chosen, because the
    /// angle-based times become unreasonable there.
    #[must_use]
    pub fn recommended(latitude: f64) -> Self {
        if latitude > 48.0 {
            Self::SeventhOfTheNight
        } else {
            Self::MiddleOfTheNight
        }
    }
}

/// A named calculation method.
///
/// The names and angles follow the conventions published by each authority. Two
/// methods define Isha as a fixed interval after sunset rather than an angle:
/// Umm al-Qura uses ninety minutes, and Qatar does the same.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Method {
    /// Muslim World League: 18 and 17 degrees.
    #[default]
    MuslimWorldLeague,
    /// Egyptian General Authority of Survey: 19.5 and 17.5 degrees.
    Egyptian,
    /// University of Islamic Sciences, Karachi: 18 and 18 degrees.
    Karachi,
    /// Umm al-Qura, Makkah: 18.5 degrees, Isha 90 minutes after sunset.
    UmmAlQura,
    /// Dubai: 18.2 and 18.2 degrees, with offsets.
    Dubai,
    /// Moonsighting Committee: 18 and 18 degrees, seasonal twilight at high
    /// latitudes.
    MoonsightingCommittee,
    /// Islamic Society of North America: 15 and 15 degrees.
    NorthAmerica,
    /// Kuwait: 18 and 17.5 degrees.
    Kuwait,
    /// Qatar: 18 degrees, Isha 90 minutes after sunset.
    Qatar,
    /// Singapore: 20 and 18 degrees, rounded up.
    Singapore,
    /// Institute of Geophysics, University of Tehran: 17.7 and 14 degrees,
    /// Maghrib 4.5 degrees.
    Tehran,
    /// Dianet, Turkey: 18 and 17 degrees, with offsets.
    Turkey,
    /// Angles supplied by the caller rather than a named authority.
    Other,
}

impl Method {
    /// The name the reference implementation uses, for tests and logging.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::MuslimWorldLeague => "MuslimWorldLeague",
            Self::Egyptian => "Egyptian",
            Self::Karachi => "Karachi",
            Self::UmmAlQura => "UmmAlQura",
            Self::Dubai => "Dubai",
            Self::MoonsightingCommittee => "MoonsightingCommittee",
            Self::NorthAmerica => "NorthAmerica",
            Self::Kuwait => "Kuwait",
            Self::Qatar => "Qatar",
            Self::Singapore => "Singapore",
            Self::Tehran => "Tehran",
            Self::Turkey => "Turkey",
            Self::Other => "Other",
        }
    }

    /// The parameters this method defines.
    #[must_use]
    pub fn parameters(self) -> Parameters {
        let mut parameters = Parameters {
            method: self,
            ..Parameters::default()
        };
        match self {
            Self::MuslimWorldLeague => {
                parameters.fajr_angle = 18.0;
                parameters.isha_angle = 17.0;
                parameters.method_minutes[DHUHR] = 1.0;
            }
            Self::Egyptian => {
                parameters.fajr_angle = 19.5;
                parameters.isha_angle = 17.5;
                parameters.method_minutes[DHUHR] = 1.0;
            }
            Self::Karachi => {
                parameters.fajr_angle = 18.0;
                parameters.isha_angle = 18.0;
                parameters.method_minutes[DHUHR] = 1.0;
            }
            Self::UmmAlQura => {
                parameters.fajr_angle = 18.5;
                parameters.isha_interval = 90.0;
            }
            Self::Dubai => {
                parameters.fajr_angle = 18.2;
                parameters.isha_angle = 18.2;
                parameters.method_minutes[SUNRISE] = -3.0;
                parameters.method_minutes[DHUHR] = 3.0;
                parameters.method_minutes[ASR] = 3.0;
                parameters.method_minutes[MAGHRIB] = 3.0;
            }
            Self::MoonsightingCommittee => {
                parameters.fajr_angle = 18.0;
                parameters.isha_angle = 18.0;
                parameters.method_minutes[DHUHR] = 5.0;
                parameters.method_minutes[MAGHRIB] = 3.0;
            }
            Self::NorthAmerica => {
                parameters.fajr_angle = 15.0;
                parameters.isha_angle = 15.0;
                parameters.method_minutes[DHUHR] = 1.0;
            }
            Self::Kuwait => {
                parameters.fajr_angle = 18.0;
                parameters.isha_angle = 17.5;
            }
            Self::Qatar => {
                parameters.fajr_angle = 18.0;
                parameters.isha_interval = 90.0;
            }
            Self::Singapore => {
                parameters.fajr_angle = 20.0;
                parameters.isha_angle = 18.0;
                parameters.method_minutes[DHUHR] = 1.0;
                parameters.rounding = Rounding::Up;
            }
            Self::Tehran => {
                parameters.fajr_angle = 17.7;
                parameters.isha_angle = 14.0;
                parameters.maghrib_angle = 4.5;
            }
            Self::Turkey => {
                parameters.fajr_angle = 18.0;
                parameters.isha_angle = 17.0;
                parameters.method_minutes[SUNRISE] = -7.0;
                parameters.method_minutes[DHUHR] = 5.0;
                parameters.method_minutes[ASR] = 4.0;
                parameters.method_minutes[MAGHRIB] = 7.0;
            }
            Self::Other => {}
        }
        parameters
    }
}

/// The positions in `method_minutes` and the caller's adjustments, in order.
const FAJR: usize = 0;
const SUNRISE: usize = 1;
const DHUHR: usize = 2;
const ASR: usize = 3;
const MAGHRIB: usize = 4;
const ISHA: usize = 5;

/// How seconds are handled when a time is reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Rounding {
    /// To the nearest minute.
    #[default]
    Nearest,
    /// Up to the next minute.
    Up,
    /// No rounding; seconds are kept.
    None,
}

/// Which twilight marks the end of Isha, for the Moonsighting Committee method.
///
/// Shafaq is the light in the western sky after sunset. The three definitions
/// come from the classical schools: the red afterglow, the white one that
/// follows it, and the general case. The choice only matters for the
/// Moonsighting Committee, which is the only method that reads it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Shafaq {
    /// The general twilight, the reference's default.
    #[default]
    General,
    /// The red afterglow.
    Ahmer,
    /// The white twilight that follows the red.
    Abyad,
}

/// How a day inside the polar circle is resolved.
///
/// Above about 66 degrees the sun may not rise or set for weeks around the
/// solstices, so the ordinary solar times do not exist and there is no night to
/// divide. The reference resolves the day to another latitude or another date
/// whose sun does rise and set, and computes the times there. Which of the two
/// is used, and whether either is, is this choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PolarCircleResolution {
    /// Leave the times that cannot be computed as `None`. The reference's
    /// default, and the honest answer: a caller in Tromso has to decide for
    /// itself what to do in June.
    #[default]
    Unresolved,
    /// Borrow the times from a nearby day that has a sunrise and a sunset.
    AqrabYaum,
    /// Borrow the times from a nearby latitude that has a sunrise and a sunset.
    AqrabBalad,
}

/// Everything that decides a prayer time.
///
/// The method fields set the angles; the `*_minutes` fields are adjustments the
/// authority applies on top, and `isha_interval` replaces the Isha angle when it
/// is not zero.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Parameters {
    /// The method these parameters came from.
    pub method: Method,
    /// The angle below the horizon for Fajr, in degrees.
    pub fajr_angle: f64,
    /// The angle below the horizon for Isha, in degrees.
    pub isha_angle: f64,
    /// Minutes after sunset for Isha, when the method defines it that way.
    pub isha_interval: f64,
    /// The angle for Maghrib, used only by the methods that set it.
    pub maghrib_angle: f64,
    /// Which school defines Asr.
    pub madhab: Madhab,
    /// How Fajr and Isha are bounded at high latitude.
    pub high_latitude_rule: HighLatitudeRule,
    /// How seconds are rounded.
    pub rounding: Rounding,
    /// Which twilight ends Isha, for the Moonsighting Committee.
    pub shafaq: Shafaq,
    /// How a day inside the polar circle is resolved.
    pub polar_circle_resolution: PolarCircleResolution,
    /// Minutes added to Fajr by the caller, on top of whatever the method
    /// applies. Zero unless the caller sets it.
    pub fajr_minutes: f64,
    /// Minutes added to sunrise.
    pub sunrise_minutes: f64,
    /// Minutes added to Dhuhr.
    pub dhuhr_minutes: f64,
    /// Minutes added to Asr.
    pub asr_minutes: f64,
    /// Minutes added to Maghrib.
    pub maghrib_minutes: f64,
    /// Minutes added to Isha.
    pub isha_minutes: f64,
    /// The minutes each method applies on its own.
    ///
    /// Kept apart from the `*_minutes` fields above because the reference keeps
    /// them apart: its `methodAdjustments` come from the method and its
    /// `adjustments` from the caller, and a caller adding two minutes to Dhuhr
    /// under a method that already adds five gets seven, not two. Folding them
    /// into one field would make that impossible to express.
    method_minutes: [f64; 6],
}

impl Default for Parameters {
    fn default() -> Self {
        Self {
            method: Method::Other,
            fajr_angle: 0.0,
            isha_angle: 0.0,
            isha_interval: 0.0,
            maghrib_angle: 0.0,
            madhab: Madhab::Shafi,
            high_latitude_rule: HighLatitudeRule::MiddleOfTheNight,
            rounding: Rounding::Nearest,
            shafaq: Shafaq::General,
            polar_circle_resolution: PolarCircleResolution::Unresolved,
            fajr_minutes: 0.0,
            sunrise_minutes: 0.0,
            dhuhr_minutes: 0.0,
            asr_minutes: 0.0,
            maghrib_minutes: 0.0,
            isha_minutes: 0.0,
            method_minutes: [0.0; 6],
        }
    }
}

impl Parameters {
    /// The parameters for a method.
    #[must_use]
    pub fn for_method(method: Method) -> Self {
        method.parameters()
    }

    /// Sets the Asr school.
    #[must_use]
    pub fn with_madhab(mut self, madhab: Madhab) -> Self {
        self.madhab = madhab;
        self
    }

    /// Sets the high latitude rule.
    #[must_use]
    pub fn with_high_latitude_rule(mut self, rule: HighLatitudeRule) -> Self {
        self.high_latitude_rule = rule;
        self
    }

    /// The minutes to add to a time: the caller's own, plus the method's.
    fn added_minutes(&self, prayer: usize) -> f64 {
        let caller = match prayer {
            FAJR => self.fajr_minutes,
            SUNRISE => self.sunrise_minutes,
            DHUHR => self.dhuhr_minutes,
            ASR => self.asr_minutes,
            MAGHRIB => self.maghrib_minutes,
            _ => self.isha_minutes,
        };
        caller + self.method_minutes[prayer]
    }
}

/// A time on the day being computed, held as whole seconds.
///
/// The reference floors a floating-point hour to whole seconds before doing
/// anything else, then rounds to the minute at the very end. Working in whole
/// seconds reproduces that exactly; carrying a float through the rounding would
/// disagree on the boundary cases.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DayTime {
    seconds: f64,
}

impl DayTime {
    /// From hours as a float, floored to whole seconds.
    pub(crate) fn from_hours(hours: f64) -> Self {
        Self::from_seconds(hours * 3600.0)
    }

    /// From whole seconds.
    ///
    /// The seasonal twilight helpers already work in seconds, and passing their
    /// result to `from_hours` would multiply it by another sixty squared.
    pub(crate) fn from_seconds(seconds: f64) -> Self {
        if !seconds.is_finite() {
            return Self { seconds: f64::NAN };
        }
        Self {
            seconds: seconds.floor(),
        }
    }

    /// Whether the time is a real number.
    ///
    /// Inside the polar circles the sun may not reach the twilight angle at all,
    /// and the arc cosine that produces the hour angle then has no answer.
    pub(crate) fn is_valid(self) -> bool {
        self.seconds.is_finite()
    }

    /// Adds minutes.
    pub(crate) fn plus_minutes(self, minutes: f64) -> Self {
        Self {
            seconds: self.seconds + minutes * 60.0,
        }
    }

    /// Adds seconds.
    pub(crate) fn plus_seconds(self, seconds: f64) -> Self {
        Self {
            seconds: self.seconds + seconds,
        }
    }

    /// The number of seconds since midnight, as a float.
    pub(crate) fn as_seconds(self) -> f64 {
        self.seconds
    }

    /// Rounds to the minute according to `rounding`.
    pub(crate) fn rounded(self, rounding: Rounding) -> Self {
        if !self.is_valid() {
            return self;
        }
        let second_of_minute = self.seconds.rem_euclid(60.0);
        let offset = match rounding {
            Rounding::None => 0.0,
            Rounding::Up => 60.0 - second_of_minute,
            Rounding::Nearest => {
                if second_of_minute >= 30.0 {
                    60.0 - second_of_minute
                } else {
                    -second_of_minute
                }
            }
        };
        Self {
            seconds: self.seconds + offset,
        }
    }

    /// The hour, minute and second on a 24-hour clock.
    ///
    /// A time before midnight or after the next one is reduced into the day, so
    /// a caller reading the clock sees the right reading; the reference does the
    /// same, and a time that crosses midnight is expected to be offset by the
    /// caller's own timezone anyway.
    ///
    /// The seconds are always whole: the reference floors the floating-point
    /// hour to whole seconds before it starts, and nothing after that
    /// reintroduces a fraction. With `Rounding::None` the second is the real one,
    /// which is the only way to see the reference's own answer exactly.
    pub(crate) fn components(self) -> Option<(i64, u32, u32)> {
        if !self.is_valid() {
            return None;
        }
        // Floored, not truncated toward zero. The reference keeps whole
        // milliseconds of an absolute instant and reads the clock off that, so a
        // time before midnight - a negative reading here - lands on the second
        // below, not the one above. Truncating instead is a second out, and only
        // shows with `Rounding::None` at high latitude.
        let total_seconds = self.seconds.floor() as i64;
        let day_seconds = 24 * 60 * 60;
        let normalized = total_seconds.rem_euclid(day_seconds);
        Some((
            normalized / 3600,
            ((normalized % 3600) / 60) as u32,
            (normalized % 60) as u32,
        ))
    }
}

/// The six times of the day, in UTC.
///
/// A time is `None` when the sun does not reach the position it needs, which
/// happens inside the polar circles: in midsummer the sun never sets, so there
/// is no sunrise, sunset, or twilight. The times that do not depend on those are
/// still given. This is the same answer the reference library gives, and it is
/// the honest one: a caller in Tromso has to decide for itself what to do in
/// June, and a made-up number would hide that decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PrayerTimes {
    /// Fajr, before sunrise.
    pub fajr: Option<(i64, u32, u32)>,
    /// Sunrise.
    pub sunrise: Option<(i64, u32, u32)>,
    /// Dhuhr, after the sun passes the meridian.
    pub dhuhr: Option<(i64, u32, u32)>,
    /// Asr.
    pub asr: Option<(i64, u32, u32)>,
    /// Maghrib, at sunset.
    pub maghrib: Option<(i64, u32, u32)>,
    /// Isha, after twilight.
    pub isha: Option<(i64, u32, u32)>,
}

/// One of the times of the day, for asking which one comes next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Prayer {
    /// Fajr, before sunrise.
    Fajr,
    /// Sunrise.
    Sunrise,
    /// Dhuhr, after the sun passes the meridian.
    Dhuhr,
    /// Asr.
    Asr,
    /// Maghrib, at sunset.
    Maghrib,
    /// Isha, after twilight.
    Isha,
}

impl PrayerTimes {
    /// The clock reading of one time, or `None` when the sun never reached it.
    ///
    /// # Example
    ///
    /// ```
    /// use adhan::{prayer_times, Method, Parameters, Prayer};
    ///
    /// let times = prayer_times(2026, 1, 1, -6.2, 106.8, &Parameters::for_method(Method::MuslimWorldLeague));
    /// assert_eq!(times.time_for(Prayer::Sunrise), times.sunrise);
    /// ```
    #[must_use]
    pub fn time_for(&self, prayer: Prayer) -> Option<(i64, u32, u32)> {
        match prayer {
            Prayer::Fajr => self.fajr,
            Prayer::Sunrise => self.sunrise,
            Prayer::Dhuhr => self.dhuhr,
            Prayer::Asr => self.asr,
            Prayer::Maghrib => self.maghrib,
            Prayer::Isha => self.isha,
        }
    }

    /// The time that has most recently begun at the given clock reading.
    ///
    /// `now` is a UTC clock reading on the same date the times were computed for,
    /// which is the same convention as the fields. The reference library compares
    /// whole instants, but because it too reduces every time into the day, the
    /// comparison is the same one.
    ///
    /// A time the sun never reached is skipped rather than treated as absent, so
    /// a caller inside the polar circle still gets an answer for the times that
    /// do exist.
    #[must_use]
    pub fn current_prayer(&self, now: (i64, u32, u32)) -> Option<Prayer> {
        let reached = |time: Option<(i64, u32, u32)>| time.is_some_and(|time| now >= time);
        if reached(self.isha) {
            Some(Prayer::Isha)
        } else if reached(self.maghrib) {
            Some(Prayer::Maghrib)
        } else if reached(self.asr) {
            Some(Prayer::Asr)
        } else if reached(self.dhuhr) {
            Some(Prayer::Dhuhr)
        } else if reached(self.sunrise) {
            Some(Prayer::Sunrise)
        } else if reached(self.fajr) {
            Some(Prayer::Fajr)
        } else {
            None
        }
    }

    /// The next time to begin after the given clock reading, or `None` when the
    /// day's last time has already begun.
    ///
    /// `now` is a UTC clock reading on the same date the times were computed for.
    /// Sunrise is returned like any other time, because the reference does the
    /// same and a caller filtering it out can ask for the one after it.
    #[must_use]
    pub fn next_prayer(&self, now: (i64, u32, u32)) -> Option<Prayer> {
        let reached = |time: Option<(i64, u32, u32)>| time.is_some_and(|time| now >= time);
        if reached(self.isha) {
            None
        } else if reached(self.maghrib) {
            Some(Prayer::Isha)
        } else if reached(self.asr) {
            Some(Prayer::Maghrib)
        } else if reached(self.dhuhr) {
            Some(Prayer::Asr)
        } else if reached(self.sunrise) {
            Some(Prayer::Dhuhr)
        } else if reached(self.fajr) {
            Some(Prayer::Sunrise)
        } else {
            Some(Prayer::Fajr)
        }
    }
}

/// Computes the prayer times for a date and location.
///
/// `year`, `month` and `day` are a Gregorian date, `latitude` is positive to the
/// north, and the result is in UTC. The caller adds its own offset for a local
/// clock.
///
/// # Example
///
/// ```
/// use adhan::{prayer_times, Method, Parameters};
///
/// // Jakarta, 1 January 2026. The result is in UTC, so add the local offset
/// // (UTC+7) to read it as a wall clock.
/// let times = prayer_times(
///     2026, 1, 1, -6.2, 106.8,
///     &Parameters::for_method(Method::MuslimWorldLeague),
/// );
///
/// // Fajr comes before sunrise, which is the one ordering that always holds.
/// let fajr = times.fajr.expect("Jakarta is far from the polar circles");
/// let sunrise = times.sunrise.expect("Jakarta is far from the polar circles");
/// assert!(fajr < sunrise);
/// ```
#[must_use]
pub fn prayer_times(
    year: i32,
    month: u32,
    day: u32,
    latitude: f64,
    longitude: f64,
    parameters: &Parameters,
) -> PrayerTimes {
    let solar = SolarTime::new(year, month, day, latitude, longitude);
    let (next_year, next_month, next_day) = next_day(year, month, day);
    let tomorrow = SolarTime::new(next_year, next_month, next_day, latitude, longitude);

    // Inside the polar circle the sun may not rise or set at all, and the
    // reference then borrows the solar times of a nearby day or latitude. The
    // latitude used for the seasonal twilight below stays the caller's, because
    // the reference keeps `coordinates` and only swaps the solar times.
    let (solar, tomorrow) = resolve_polar_circle(
        parameters.polar_circle_resolution,
        year,
        month,
        day,
        latitude,
        longitude,
        solar,
        tomorrow,
    );

    let transit = DayTime::from_hours(solar.transit);
    let sunrise = DayTime::from_hours(solar.sunrise);
    let sunset = DayTime::from_hours(solar.sunset);
    let tomorrow_sunrise = DayTime::from_hours(tomorrow.sunrise);

    // The night runs from sunset to tomorrow's sunrise. `DayTime` holds a clock
    // reading with no day attached, and a sunrise before midnight UTC comes out
    // negative, so a full day is added back before the two are subtracted. Skip
    // that and the night is negative for the whole eastern hemisphere.
    let night_seconds = tomorrow_sunrise.as_seconds() + 86_400.0 - sunset.as_seconds();
    let day_of_year = astronomy::day_of_year(year, month, day);
    let moonsighting = parameters.method == Method::MoonsightingCommittee;

    let asr = DayTime::from_hours(solar.afternoon(parameters.madhab.shadow_length()));

    let fajr = fajr_time(
        solar.hour_angle(-parameters.fajr_angle, false),
        sunrise,
        sunset,
        night_seconds,
        parameters,
        moonsighting,
        latitude,
        day_of_year,
        year,
    );

    let isha = isha_time(
        solar.hour_angle(-parameters.isha_angle, true),
        sunset,
        night_seconds,
        parameters,
        moonsighting,
        latitude,
        day_of_year,
        year,
    );

    let mut maghrib = sunset;
    if parameters.maghrib_angle != 0.0 {
        // Only the methods that define Maghrib by an angle use this, and then
        // only when the angle lands between sunset and Isha.
        let by_angle = DayTime::from_hours(solar.hour_angle(-parameters.maghrib_angle, true));
        if sunset.as_seconds() < by_angle.as_seconds() && isha.as_seconds() > by_angle.as_seconds()
        {
            maghrib = by_angle;
        }
    }

    PrayerTimes {
        fajr: fajr
            .plus_minutes(parameters.added_minutes(FAJR))
            .rounded(parameters.rounding)
            .components(),
        sunrise: sunrise
            .plus_minutes(parameters.added_minutes(SUNRISE))
            .rounded(parameters.rounding)
            .components(),
        dhuhr: transit
            .plus_minutes(parameters.added_minutes(DHUHR))
            .rounded(parameters.rounding)
            .components(),
        asr: asr
            .plus_minutes(parameters.added_minutes(ASR))
            .rounded(parameters.rounding)
            .components(),
        maghrib: maghrib
            .plus_minutes(parameters.added_minutes(MAGHRIB))
            .rounded(parameters.rounding)
            .components(),
        isha: isha
            .plus_minutes(parameters.added_minutes(ISHA))
            .rounded(parameters.rounding)
            .components(),
    }
}

/// The Qibla: the direction to the Kaaba, in degrees clockwise from true north.
///
/// The answer is a bearing, not a heading: a compass in most of the world reads
/// a little off true north, and the difference is the caller's to apply.
///
/// The formula is the spherical one from "Spherical Trigonometry for the use of
/// colleges and schools", which is the one the reference library uses, so the two
/// agree to the fourth decimal. Makkah's coordinates are the reference's own.
///
/// # Example
///
/// ```
/// use adhan::qibla;
///
/// // From Jakarta the Kaaba lies to the west-northwest.
/// let bearing = qibla(-6.2, 106.8);
/// assert!((bearing - 295.15).abs() < 0.1);
/// ```
#[must_use]
pub fn qibla(latitude: f64, longitude: f64) -> f64 {
    let makkah_latitude = 21.422_524_1_f64;
    let makkah_longitude = 39.826_181_8_f64;

    let delta_longitude = (makkah_longitude - longitude).to_radians();
    let term1 = delta_longitude.sin();
    let term2 = latitude.to_radians().cos() * makkah_latitude.to_radians().tan();
    let term3 = latitude.to_radians().sin() * delta_longitude.cos();
    let angle = (term1).atan2(term2 - term3);

    unwind_angle(angle.to_degrees())
}

/// Reduces an angle into `[0, 360)`.
///
/// The reference subtracts `360 * floor(angle / 360)`, which keeps a negative
/// angle positive. `rem_euclid` does the same thing and says so.
fn unwind_angle(angle: f64) -> f64 {
    angle.rem_euclid(360.0)
}

/// The middle and last third of the night, for the night that follows a Maghrib.
///
/// The night runs from Maghrib to the next day's Fajr, so both are on the day
/// after the prayer times they were computed from, and both can fall after
/// midnight. They are what the voluntary night prayer is timed by: the last third
/// is the part of the night most often recommended for Qiyam.
///
/// The day crosses a month and a year boundary like any other, and the next day's
/// Fajr is computed with the same parameters, so a caller who changed the method
/// or the high latitude rule gets the night that goes with it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SunnahTimes {
    /// The middle of the night: Maghrib plus half the night.
    pub middle_of_the_night: Option<(i64, u32, u32)>,
    /// The last third of the night: Maghrib plus two thirds of it.
    pub last_third_of_the_night: Option<(i64, u32, u32)>,
}

/// Computes the middle and last third of the night after the given day.
///
/// The night is measured from this day's Maghrib to the next day's Fajr. A
/// `PrayerTimes` with no Maghrib or no Fajr - inside the polar circles, where the
/// sun may not set or may not rise - has no night to divide, and both fields come
/// back `None`.
#[must_use]
pub fn sunnah_times(
    year: i32,
    month: u32,
    day: u32,
    latitude: f64,
    longitude: f64,
    parameters: &Parameters,
) -> SunnahTimes {
    let today = prayer_times(year, month, day, latitude, longitude, parameters);
    let (next_year, next_month, next_day) = next_day(year, month, day);
    let tomorrow = prayer_times(
        next_year, next_month, next_day, latitude, longitude, parameters,
    );

    let (Some(maghrib), Some(fajr)) = (today.maghrib, tomorrow.fajr) else {
        return SunnahTimes {
            middle_of_the_night: None,
            last_third_of_the_night: None,
        };
    };

    // Maghrib and the next Fajr are clock readings with no day attached. The
    // night is the gap between them, which is always less than a day: a
    // negative difference means the Fajr falls on the following day, so a day is
    // added back. Adding a day unconditionally is wrong, because in an eastern
    // timezone Maghrib itself is already past midnight UTC.
    let maghrib_seconds = seconds_of(maghrib);
    let fajr_seconds = seconds_of(fajr);
    let mut night_seconds = fajr_seconds - maghrib_seconds;
    if night_seconds < 0.0 {
        night_seconds += 86_400.0;
    }

    let divide = |fraction: f64| {
        let seconds = maghrib_seconds + night_seconds * fraction;
        DayTime::from_seconds(seconds)
            .rounded(parameters.rounding)
            .components()
    };

    SunnahTimes {
        middle_of_the_night: divide(0.5),
        last_third_of_the_night: divide(2.0 / 3.0),
    }
}

/// A clock reading as seconds since midnight.
fn seconds_of((hour, minute, second): (i64, u32, u32)) -> f64 {
    (hour * 3600 + i64::from(minute) * 60 + i64::from(second)) as f64
}

/// Whether a day has both a sunrise and a sunset at all.
fn has_sunrise_and_sunset(solar: &SolarTime) -> bool {
    solar.sunrise.is_finite() && solar.sunset.is_finite()
}

/// The date after the given one.
fn next_day(year: i32, month: u32, day: u32) -> (i32, u32, u32) {
    let lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
    let last = if month == 2 && astronomy::is_leap_year(year) {
        29
    } else {
        lengths[month as usize - 1]
    };
    if day < last {
        (year, month, day + 1)
    } else if month < 12 {
        (year, month + 1, 1)
    } else {
        (year + 1, 1, 1)
    }
}

/// The date before the given one.
fn previous_day(year: i32, month: u32, day: u32) -> (i32, u32, u32) {
    if day > 1 {
        return (year, month, day - 1);
    }
    if month > 1 {
        let lengths = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
        let last = if month == 3 && astronomy::is_leap_year(year) {
            29
        } else {
            lengths[month as usize - 2]
        };
        return (year, month - 1, last);
    }
    (year - 1, 12, 31)
}

/// A date a whole number of days away, forwards or backwards.
fn add_days(year: i32, month: u32, day: u32, days: i64) -> (i32, u32, u32) {
    let mut date = (year, month, day);
    for _ in 0..days.unsigned_abs() {
        date = if days > 0 {
            next_day(date.0, date.1, date.2)
        } else {
            previous_day(date.0, date.1, date.2)
        };
    }
    date
}

/// Swaps in solar times from a nearby day or latitude when the caller's own day
/// has no sunrise or sunset.
///
/// The reference does this before computing anything, and the borrowed times
/// then flow through the rest of the calculation unchanged; the latitude used
/// for the seasonal twilight is still the caller's.
#[allow(clippy::too_many_arguments)]
fn resolve_polar_circle(
    resolution: PolarCircleResolution,
    year: i32,
    month: u32,
    day: u32,
    latitude: f64,
    longitude: f64,
    solar: SolarTime,
    tomorrow: SolarTime,
) -> (SolarTime, SolarTime) {
    if resolution == PolarCircleResolution::Unresolved {
        return (solar, tomorrow);
    }
    let usable = has_sunrise_and_sunset(&solar) && tomorrow.sunrise.is_finite();
    if usable {
        return (solar, tomorrow);
    }

    match resolution {
        PolarCircleResolution::Unresolved => (solar, tomorrow),
        PolarCircleResolution::AqrabYaum => {
            match aqrab_yaum(latitude, longitude, year, month, day, 1, 1) {
                Some(test) => {
                    let next = next_day(test.0, test.1, test.2);
                    (
                        SolarTime::new(test.0, test.1, test.2, latitude, longitude),
                        SolarTime::new(next.0, next.1, next.2, latitude, longitude),
                    )
                }
                None => (solar, tomorrow),
            }
        }
        PolarCircleResolution::AqrabBalad => {
            // The first attempt already steps the latitude inwards, as the
            // reference does.
            let start = latitude - latitude.signum() * 0.5;
            match aqrab_balad(longitude, year, month, day, start) {
                Some((test_solar, test_tomorrow)) => (test_solar, test_tomorrow),
                None => (solar, tomorrow),
            }
        }
    }
}

/// The nearest day, walking outwards, whose sun rises and sets.
///
/// The walk alternates direction and stops after half a year, which is as far as
/// a day can be from a solstice.
#[allow(clippy::too_many_arguments)]
fn aqrab_yaum(
    latitude: f64,
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    days_added: i64,
    direction: i64,
) -> Option<(i32, u32, u32)> {
    if days_added > 183 {
        return None;
    }
    let test = add_days(year, month, day, direction * days_added);
    let next = next_day(test.0, test.1, test.2);
    let solar = SolarTime::new(test.0, test.1, test.2, latitude, longitude);
    let tomorrow = SolarTime::new(next.0, next.1, next.2, latitude, longitude);
    if !has_sunrise_and_sunset(&solar) || !has_sunrise_and_sunset(&tomorrow) {
        let step = days_added + if direction > 0 { 0 } else { 1 };
        return aqrab_yaum(latitude, longitude, year, month, day, step, -direction);
    }
    Some(test)
}

/// The nearest latitude, walking inwards, whose sun rises and sets.
#[allow(clippy::too_many_arguments)]
fn aqrab_balad(
    longitude: f64,
    year: i32,
    month: u32,
    day: u32,
    test_latitude: f64,
) -> Option<(SolarTime, SolarTime)> {
    let solar = SolarTime::new(year, month, day, test_latitude, longitude);
    let next = next_day(year, month, day);
    let tomorrow = SolarTime::new(next.0, next.1, next.2, test_latitude, longitude);
    if !has_sunrise_and_sunset(&solar) || !has_sunrise_and_sunset(&tomorrow) {
        if test_latitude.abs() >= 65.0 {
            let step = test_latitude - test_latitude.signum() * 0.5;
            return aqrab_balad(longitude, year, month, day, step);
        }
        return None;
    }
    Some((solar, tomorrow))
}

/// Fajr, bounded by the high latitude rule when the angle gives no answer or an
/// answer later than the rule allows.
#[allow(clippy::too_many_arguments)]
fn fajr_time(
    by_angle: f64,
    sunrise: DayTime,
    sunset: DayTime,
    night_seconds: f64,
    parameters: &Parameters,
    moonsighting: bool,
    latitude: f64,
    day_of_year: u32,
    year: i32,
) -> DayTime {
    let mut by_angle = DayTime::from_hours(by_angle);

    // The Moonsighting Committee substitutes a seventh of the night above 55
    // degrees, before the safety bound is applied.
    if moonsighting && latitude >= 55.0 {
        by_angle = sunrise.plus_seconds(-(night_seconds / 7.0));
    }

    let safe = if moonsighting {
        DayTime::from_seconds(astronomy::season_adjusted_morning_twilight(
            latitude,
            day_of_year,
            year,
            sunrise.as_seconds(),
        ))
    } else {
        let (portion, _) = parameters
            .high_latitude_rule
            .night_portions(parameters.fajr_angle, parameters.isha_angle);
        let _ = sunset;
        sunrise.plus_seconds(-portion * night_seconds)
    };

    if !by_angle.is_valid() || safe.as_seconds() > by_angle.as_seconds() {
        safe
    } else {
        by_angle
    }
}

/// Isha, from an interval after sunset or an angle, bounded the same way.
#[allow(clippy::too_many_arguments)]
fn isha_time(
    by_angle: f64,
    sunset: DayTime,
    night_seconds: f64,
    parameters: &Parameters,
    moonsighting: bool,
    latitude: f64,
    day_of_year: u32,
    year: i32,
) -> DayTime {
    if parameters.isha_interval > 0.0 {
        return sunset.plus_minutes(parameters.isha_interval);
    }
    let mut by_angle = DayTime::from_hours(by_angle);

    if moonsighting && latitude >= 55.0 {
        by_angle = sunset.plus_seconds(night_seconds / 7.0);
    }

    let safe = if moonsighting {
        DayTime::from_seconds(astronomy::season_adjusted_evening_twilight(
            latitude,
            day_of_year,
            year,
            sunset.as_seconds(),
            parameters.shafaq,
        ))
    } else {
        let (_, portion) = parameters
            .high_latitude_rule
            .night_portions(parameters.fajr_angle, parameters.isha_angle);
        sunset.plus_seconds(portion * night_seconds)
    };

    if !by_angle.is_valid() || safe.as_seconds() < by_angle.as_seconds() {
        safe
    } else {
        by_angle
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_methods_carry_their_published_angles() {
        assert_eq!(Method::MuslimWorldLeague.parameters().fajr_angle, 18.0);
        assert_eq!(Method::MuslimWorldLeague.parameters().isha_angle, 17.0);
        assert_eq!(Method::Egyptian.parameters().fajr_angle, 19.5);
        assert_eq!(Method::NorthAmerica.parameters().fajr_angle, 15.0);
        assert_eq!(Method::UmmAlQura.parameters().isha_interval, 90.0);
        assert_eq!(Method::Tehran.parameters().maghrib_angle, 4.5);
        assert_eq!(Method::Singapore.parameters().rounding, Rounding::Up);
    }

    #[test]
    fn the_hanafi_shadow_is_twice_the_other() {
        assert_eq!(Madhab::Shafi.shadow_length(), 1.0);
        assert_eq!(Madhab::Hanafi.shadow_length(), 2.0);
    }

    #[test]
    fn a_day_without_a_sunset_is_left_unresolved_by_default() {
        // Tromso in June: the sun does not set. The default is the reference's
        // own, and it reports nothing rather than a made-up time.
        let times = prayer_times(
            2026,
            6,
            21,
            69.6492,
            18.9553,
            &Parameters::for_method(Method::MuslimWorldLeague),
        );
        assert!(times.sunrise.is_none());
        assert!(times.maghrib.is_none());
        assert!(times.fajr.is_none());
        assert!(times.dhuhr.is_some());
        assert!(times.asr.is_some());
    }

    #[test]
    fn aqrab_yaum_borrows_from_a_day_that_has_a_sunset() {
        // The nearest day to 21 June whose sun rises and sets at Tromso is 26
        // July, thirty-five days later; the reference walks outwards and finds
        // the same one.
        let mut parameters = Parameters::for_method(Method::MuslimWorldLeague);
        parameters.polar_circle_resolution = PolarCircleResolution::AqrabYaum;
        let times = prayer_times(2026, 6, 21, 69.6492, 18.9553, &parameters);
        // The values are the reference's, to the minute.
        assert_eq!(times.sunrise, Some((23, 5, 0)));
        assert_eq!(times.maghrib, Some((21, 58, 0)));
        assert_eq!(times.fajr, Some((22, 19, 0)));
    }

    #[test]
    fn aqrab_balad_borrows_from_a_latitude_that_has_a_sunset() {
        // Four degrees south of Tromso, the sun does set on 21 June.
        let mut parameters = Parameters::for_method(Method::MuslimWorldLeague);
        parameters.polar_circle_resolution = PolarCircleResolution::AqrabBalad;
        let times = prayer_times(2026, 6, 21, 69.6492, 18.9553, &parameters);
        assert_eq!(times.sunrise, Some((23, 6, 0)));
        assert_eq!(times.maghrib, Some((22, 26, 0)));
    }

    #[test]
    fn the_polar_resolvers_do_not_touch_a_day_that_has_a_sunset() {
        // Jakarta is nowhere near the polar circles, so every resolution gives
        // the same answer as the default.
        let base = prayer_times(
            2026,
            6,
            21,
            -6.2,
            106.8,
            &Parameters::for_method(Method::MuslimWorldLeague),
        );
        for resolution in [
            PolarCircleResolution::AqrabYaum,
            PolarCircleResolution::AqrabBalad,
        ] {
            let mut parameters = Parameters::for_method(Method::MuslimWorldLeague);
            parameters.polar_circle_resolution = resolution;
            let times = prayer_times(2026, 6, 21, -6.2, 106.8, &parameters);
            assert_eq!(times, base);
        }
    }

    #[test]
    fn the_shafaq_changes_isha_only_for_the_moonsighting_committee() {
        let at = |shafaq, method| {
            let mut parameters = Parameters::for_method(method);
            parameters.shafaq = shafaq;
            parameters.rounding = Rounding::None;
            prayer_times(2026, 1, 15, 55.0, 10.0, &parameters).isha
        };
        // Above 48 degrees the Moonsighting Committee's seasonal curve sets
        // Isha, and the three shafaqs give three different times.
        let general = at(Shafaq::General, Method::MoonsightingCommittee);
        let ahmer = at(Shafaq::Ahmer, Method::MoonsightingCommittee);
        let abyad = at(Shafaq::Abyad, Method::MoonsightingCommittee);
        assert_ne!(general, ahmer);
        assert_ne!(general, abyad);
        assert!(ahmer < general, "the red twilight ends earliest");
        assert!(general < abyad, "the white twilight ends latest");
        // Under any other method the field is not read.
        assert_eq!(
            at(Shafaq::General, Method::MuslimWorldLeague),
            at(Shafaq::Ahmer, Method::MuslimWorldLeague)
        );
    }

    #[test]
    fn the_date_arithmetic_crosses_months_and_years() {
        assert_eq!(next_day(2026, 1, 31), (2026, 2, 1));
        assert_eq!(next_day(2026, 12, 31), (2027, 1, 1));
        assert_eq!(next_day(2024, 2, 28), (2024, 2, 29), "leap year");
        assert_eq!(previous_day(2026, 1, 1), (2025, 12, 31));
        assert_eq!(previous_day(2026, 3, 1), (2026, 2, 28));
        assert_eq!(previous_day(2024, 3, 1), (2024, 2, 29), "leap year");
        assert_eq!(add_days(2026, 6, 21, 35), (2026, 7, 26));
        assert_eq!(add_days(2026, 1, 1, -1), (2025, 12, 31));
    }

    #[test]
    fn the_recommended_rule_changes_above_forty_eight_degrees() {
        assert_eq!(
            HighLatitudeRule::recommended(51.5),
            HighLatitudeRule::SeventhOfTheNight
        );
        assert_eq!(
            HighLatitudeRule::recommended(-6.2),
            HighLatitudeRule::MiddleOfTheNight
        );
    }

    #[test]
    fn rounding_follows_the_rule() {
        let time = DayTime::from_hours(12.0 + 20.0 / 3600.0);
        assert_eq!(
            time.rounded(Rounding::Nearest).components(),
            Some((12, 0, 0))
        );
        assert_eq!(time.rounded(Rounding::Up).components(), Some((12, 1, 0)));
        assert_eq!(time.rounded(Rounding::None).components(), Some((12, 0, 20)));
    }

    #[test]
    fn a_time_outside_the_day_keeps_its_clock_but_moves_the_day() {
        // A negative hour is the previous day, 24 or more the next; the clock
        // reading is what the caller sees, so both reduce.
        assert_eq!(DayTime::from_hours(-0.5).components(), Some((23, 30, 0)));
        assert_eq!(DayTime::from_hours(25.5).components(), Some((1, 30, 0)));
        assert_eq!(DayTime::from_hours(12.0).components(), Some((12, 0, 0)));
        // A time that does not exist, such as a sunrise the sun never has, is
        // reported as absent rather than as an arbitrary reading.
        assert_eq!(DayTime::from_hours(f64::NAN).components(), None);
    }

    #[test]
    fn the_next_day_wraps_months_and_years() {
        assert_eq!(next_day(2026, 10, 21), (2026, 10, 22));
        assert_eq!(next_day(2026, 10, 31), (2026, 11, 1));
        assert_eq!(next_day(2026, 12, 31), (2027, 1, 1));
        assert_eq!(next_day(2028, 2, 28), (2028, 2, 29));
        assert_eq!(next_day(2026, 2, 28), (2026, 3, 1));
    }
}
