//! Complex numbers in the textual form a spreadsheet uses.
//!
//! A spreadsheet has no complex type: `COMPLEX` builds a string such as
//! `3+4i` and every `IM*` function parses one back. The parsing rules are
//! narrow on purpose, because the strings are also what a user types:
//!
//! * the imaginary unit is `i` or `j`, and it may be written with an explicit
//!   coefficient (`1i`) or without one (`i`, `-i`);
//! * a coefficient of one is omitted on output, so `1+1i` prints as `1+i`;
//! * a zero part is dropped, so `3+0i` prints as `3` and `0+1i` as `i`.

/// A complex number.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Complex {
    /// The real part.
    pub(crate) re: f64,
    /// The imaginary part.
    pub(crate) im: f64,
}

/// Whether a float is close enough to zero to be treated as zero.
///
/// The trigonometric identities go through `exp` and `ln`, so a part that
/// should be zero can come back as 1e-17; printing that would be wrong.
fn is_zero(x: f64) -> bool {
    x.abs() < 1e-12
}

impl Complex {
    /// Creates a complex number.
    pub(crate) const fn new(re: f64, im: f64) -> Self {
        Self { re, im }
    }

    /// Parses `a+bi` or `a+bj`.
    ///
    /// A string that is a bare number is read as real, which is what a
    /// spreadsheet does when a real number reaches an `IM*` function.
    pub(crate) fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        if text.is_empty() {
            return None;
        }
        // A bare real number has no unit at all.
        let has_unit = text.ends_with('i') || text.ends_with('j');
        if !has_unit {
            return text.parse::<f64>().ok().map(|re| Self::new(re, 0.0));
        }
        let body = &text[..text.len() - 1];
        // Find the sign that separates the two parts, ignoring a sign that
        // belongs to an exponent and the leading sign of the whole number.
        let mut split = None;
        for (i, c) in body.char_indices() {
            if i == 0 {
                continue;
            }
            if c == '+' || c == '-' {
                let prev = body[..i].chars().next_back();
                if !matches!(prev, Some('e') | Some('E')) {
                    split = Some(i);
                }
            }
        }
        match split {
            Some(i) => {
                let re: f64 = body[..i].trim().parse().ok()?;
                let im: f64 = parse_imaginary(&body[i..])?;
                Some(Self::new(re, im))
            }
            None => {
                let im = parse_imaginary(body)?;
                Some(Self::new(0.0, im))
            }
        }
    }

    /// Formats the number the way `COMPLEX` does, using the given unit.
    pub(crate) fn format(self, unit: char) -> String {
        let real_zero = is_zero(self.re);
        let imag_zero = is_zero(self.im);
        if imag_zero {
            return format_real(self.re);
        }
        if real_zero {
            return format_imaginary(self.im, unit);
        }
        let sign = if self.im < 0.0 { '-' } else { '+' };
        format!(
            "{}{}{}",
            format_real(self.re),
            sign,
            format_imaginary(self.im.abs(), unit)
        )
    }

    /// The absolute value, `IMABS`.
    pub(crate) fn modulus(self) -> f64 {
        self.re.hypot(self.im)
    }

    /// The argument in radians, `IMARGUMENT`.
    pub(crate) fn argument(self) -> f64 {
        self.im.atan2(self.re)
    }

    /// The complex conjugate.
    pub(crate) fn conjugate(self) -> Self {
        Self::new(self.re, -self.im)
    }

    /// Complex multiplication.
    pub(crate) fn mul(self, other: Self) -> Self {
        Self::new(
            self.re * other.re - self.im * other.im,
            self.re * other.im + self.im * other.re,
        )
    }

    /// Complex division.
    pub(crate) fn div(self, other: Self) -> Option<Self> {
        let d = other.re * other.re + other.im * other.im;
        if is_zero(d) {
            return None;
        }
        Some(Self::new(
            (self.re * other.re + self.im * other.im) / d,
            (self.im * other.re - self.re * other.im) / d,
        ))
    }

    /// `r * e^(i*theta)`, used to come back from polar form.
    fn from_polar(r: f64, theta: f64) -> Self {
        Self::new(r * theta.cos(), r * theta.sin())
    }

    /// The natural logarithm.
    pub(crate) fn ln(self) -> Option<Self> {
        if self.re == 0.0 && self.im == 0.0 {
            return None;
        }
        Some(Self::new(self.modulus().ln(), self.argument()))
    }

    /// The principal square root.
    pub(crate) fn sqrt(self) -> Self {
        Self::from_polar(self.modulus().sqrt(), self.argument() / 2.0)
    }

    /// The exponential.
    pub(crate) fn exp(self) -> Self {
        Self::from_polar(self.re.exp(), self.im)
    }

    /// A complex power, `IMPOWER`.
    ///
    /// The reference goes through polar form - raise the modulus and multiply
    /// the argument - which rounds differently from repeated multiplication,
    /// and the difference is visible in the last digit. Matching it is the
    /// point of a port, so the polar route is the one taken.
    pub(crate) fn pow(self, n: f64) -> Option<Self> {
        if self.re == 0.0 && self.im == 0.0 {
            return None;
        }
        let p = self.modulus().powf(n);
        let t = self.argument();
        Some(Self::new(p * (n * t).cos(), p * (n * t).sin()))
    }

    /// The sine, from `sin(a+bi) = sin a cosh b + i cos a sinh b`.
    pub(crate) fn sin(self) -> Self {
        Self::new(
            self.re.sin() * self.im.cosh(),
            self.re.cos() * self.im.sinh(),
        )
    }

    /// The cosine, from `cos(a+bi) = cos a cosh b - i sin a sinh b`.
    pub(crate) fn cos(self) -> Self {
        Self::new(
            self.re.cos() * self.im.cosh(),
            -self.re.sin() * self.im.sinh(),
        )
    }

    /// The tangent, `sin / cos`.
    pub(crate) fn tan(self) -> Option<Self> {
        self.sin().div(self.cos())
    }
}

/// Parses the coefficient of a bare imaginary unit, where the `1` may be
/// omitted.
fn parse_imaginary(body: &str) -> Option<f64> {
    let body = body.trim();
    match body {
        "" | "+" => Some(1.0),
        "-" => Some(-1.0),
        other => other.parse::<f64>().ok(),
    }
}

/// Formats a real number without a trailing `.0`.
fn format_real(x: f64) -> String {
    if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{x}")
    }
}

/// Formats the imaginary part, omitting a coefficient of one.
fn format_imaginary(im: f64, unit: char) -> String {
    if (im - 1.0).abs() < f64::EPSILON {
        return unit.to_string();
    }
    if (im + 1.0).abs() < f64::EPSILON {
        return format!("-{unit}");
    }
    format!("{}{unit}", format_real(im))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_the_forms_a_user_writes() {
        assert_eq!(Complex::parse("3+4i"), Some(Complex::new(3.0, 4.0)));
        assert_eq!(Complex::parse("3-4j"), Some(Complex::new(3.0, -4.0)));
        assert_eq!(Complex::parse("i"), Some(Complex::new(0.0, 1.0)));
        assert_eq!(Complex::parse("-i"), Some(Complex::new(0.0, -1.0)));
        assert_eq!(Complex::parse("4i"), Some(Complex::new(0.0, 4.0)));
        // A bare number is real.
        assert_eq!(Complex::parse("5"), Some(Complex::new(5.0, 0.0)));
        assert_eq!(Complex::parse("2.5"), Some(Complex::new(2.5, 0.0)));
        // An exponent's sign must not be mistaken for the separator.
        assert_eq!(Complex::parse("1e-3+2i"), Some(Complex::new(0.001, 2.0)));
        assert_eq!(Complex::parse("nonsense"), None);
    }

    #[test]
    fn formats_the_way_the_reference_does() {
        assert_eq!(Complex::new(3.0, 4.0).format('i'), "3+4i");
        assert_eq!(Complex::new(3.0, 4.0).format('j'), "3+4j");
        assert_eq!(Complex::new(0.0, 1.0).format('i'), "i");
        assert_eq!(Complex::new(1.0, 0.0).format('i'), "1");
        assert_eq!(Complex::new(-3.0, -4.0).format('i'), "-3-4i");
        assert_eq!(Complex::new(0.0, -1.0).format('i'), "-i");
    }

    #[test]
    fn round_trips_through_text() {
        for value in [Complex::new(3.0, 4.0), Complex::new(-1.5, 0.25)] {
            let text = value.format('i');
            assert_eq!(Complex::parse(&text), Some(value));
        }
    }

    #[test]
    fn arithmetic_matches_the_definitions() {
        let a = Complex::new(3.0, 4.0);
        let b = Complex::new(1.0, -2.0);
        assert_eq!(a.modulus(), 5.0);
        assert_eq!(a.conjugate(), Complex::new(3.0, -4.0));
        assert_eq!(a.mul(b), Complex::new(11.0, -2.0));
        assert_eq!(a.div(b), Some(Complex::new(-1.0, 2.0)));
        assert_eq!(Complex::new(0.0, 0.0).div(a), Some(Complex::new(0.0, 0.0)));
        // Division by zero has no answer.
        assert_eq!(a.div(Complex::new(0.0, 0.0)), None);
        // `sqrt(3+4i)` is exactly `2+i`.
        let root = a.sqrt();
        assert!((root.re - 2.0).abs() < 1e-12);
        assert!((root.im - 1.0).abs() < 1e-12);
    }
}
