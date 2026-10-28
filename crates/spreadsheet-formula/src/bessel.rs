//! The Bessel functions `BESSELI`, `BESSELJ`, `BESSELK` and `BESSELY`.
//!
//! These are the polynomial and recurrence approximations a spreadsheet uses,
//! not a series evaluated to full precision. The crate reproduces them exactly
//! so that a value matches what the reference library returns, which is the
//! behaviour a workbook already depends on.
//!
//! Ported from the `bessel` package (SheetJS, Apache-2.0); see `NOTICE`.

/// Evaluates a polynomial in `v` with Horner's method.
fn horner(coefficients: &[f64], v: f64) -> f64 {
    let mut z = 0.0;
    for &c in coefficients {
        z = v * z + c;
    }
    z
}

/// Recurrence for the integer-order variants: `f2 = f1 * o * 2/x + sign * f0`.
fn bessel_iter(x: f64, n: i32, f0: f64, f1: f64, sign: f64) -> f64 {
    if n == 0 {
        return f0;
    }
    if n == 1 {
        return f1;
    }
    let tdx = 2.0 / x;
    let mut f0 = f0;
    let mut f1 = f1;
    let mut f2 = f1;
    for o in 1..n {
        f2 = f1 * f64::from(o) * tdx + sign * f0;
        f0 = f1;
        f1 = f2;
    }
    f2
}

/// Shared wrapper for `Y` and `K`: the order is floored, and a non-positive `x`
/// is an infinity or a NaN rather than a value. `at_zero` is the infinity the
/// reference returns for `x == 0`, which differs between the two.
fn bessel_wrap(
    bessel0: fn(f64) -> f64,
    bessel1: fn(f64) -> f64,
    x: f64,
    n: f64,
    sign: f64,
    at_zero: f64,
) -> f64 {
    if x == 0.0 {
        return at_zero;
    }
    if x < 0.0 {
        return f64::NAN;
    }
    if n == 0.0 {
        return bessel0(x);
    }
    if n == 1.0 {
        return bessel1(x);
    }
    if n < 0.0 {
        return f64::NAN;
    }
    let order = n as i32;
    let b0 = bessel0(x);
    let b1 = bessel1(x);
    bessel_iter(x, order, b0, b1, sign)
}

/// `BESSELJ`: the Bessel function of the first kind, `Jn(x)`.
pub(crate) fn besselj(x: f64, n: f64) -> f64 {
    const TWO_OVER_PI: f64 = std::f64::consts::FRAC_2_PI;

    fn bessel0(x: f64) -> f64 {
        const A1A: &[f64] = &[
            -184.905_245_6,
            77_392.330_17,
            -11_214_424.18,
            651_619_640.7,
            -13_362_590_354.0,
            57_568_490_574.0,
        ];
        const A2A: &[f64] = &[
            1.0,
            267.853_271_2,
            59_272.648_53,
            9_494_680.718,
            1_029_532_985.0,
            57_568_490_411.0,
        ];
        const A1B: &[f64] = &[
            0.209_388_721_1e-6,
            -0.207_337_063_9e-5,
            0.273_451_040_7e-4,
            -0.109_862_862_7e-2,
            1.0,
        ];
        const A2B: &[f64] = &[
            -0.934_935_152e-7,
            0.762_109_516_1e-6,
            -0.691_114_765_1e-5,
            0.143_048_876_5e-3,
            -0.156_249_999_5e-1,
        ];
        let mut y = x * x;
        if x < 8.0 {
            let a1 = horner(A1A, y);
            let a2 = horner(A2A, y);
            a1 / a2
        } else {
            let xx = x - 0.785_398_164;
            y = 64.0 / y;
            let a1 = horner(A1B, y);
            let a2 = horner(A2B, y);
            (TWO_OVER_PI / x).sqrt() * (xx.cos() * a1 - xx.sin() * a2 * 8.0 / x)
        }
    }

    fn bessel1(x: f64) -> f64 {
        const A1A: &[f64] = &[
            -30.160_366_06,
            15_704.482_60,
            -2_972_611.439,
            242_396_853.1,
            -7_895_059_235.0,
            72_362_614_232.0,
        ];
        const A2A: &[f64] = &[
            1.0,
            376.999_139_7,
            99_447.433_94,
            18_583_304.74,
            2_300_535_178.0,
            144_725_228_442.0,
        ];
        const A1B: &[f64] = &[
            -0.240_337_019e-6,
            0.245_752_017_4e-5,
            -0.351_639_649_6e-4,
            0.183_105e-2,
            1.0,
        ];
        const A2B: &[f64] = &[
            0.105_787_412e-6,
            -0.882_289_87e-6,
            0.844_919_909_6e-5,
            -0.200_269_087_3e-3,
            0.046_874_999_95,
        ];
        let y = x * x;
        let xx = x.abs() - 2.356_194_491;
        if x.abs() < 8.0 {
            let a1 = x * horner(A1A, y);
            let a2 = horner(A2A, y);
            a1 / a2
        } else {
            let y = 64.0 / y;
            let a1 = horner(A1B, y);
            let a2 = horner(A2B, y);
            let a =
                (TWO_OVER_PI / x.abs()).sqrt() * (xx.cos() * a1 - xx.sin() * a2 * 8.0 / x.abs());
            if x < 0.0 { -a } else { a }
        }
    }

    let n = n.round();
    if !x.is_finite() {
        return if x.is_nan() { x } else { 0.0 };
    }
    if n < 0.0 {
        return if (n as i64) % 2 != 0 { -1.0 } else { 1.0 } * besselj(x, -n);
    }
    if x < 0.0 {
        return if (n as i64) % 2 != 0 { -1.0 } else { 1.0 } * besselj(-x, n);
    }
    if n == 0.0 {
        return bessel0(x);
    }
    if n == 1.0 {
        return bessel1(x);
    }
    if x == 0.0 {
        return 0.0;
    }

    let mut ret = 0.0;
    if x > n {
        ret = bessel_iter(x, n as i32, bessel0(x), bessel1(x), -1.0);
    } else {
        let m = 2 * ((n + (40.0 * n).sqrt().floor()) / 2.0).floor() as i64;
        let mut jsum = false;
        let mut bjp = 0.0;
        let mut sum = 0.0;
        let mut bj = 1.0;
        let tox = 2.0 / x;
        let mut j = m;
        while j > 0 {
            let bjm = j as f64 * tox * bj - bjp;
            bjp = bj;
            bj = bjm;
            if bj.abs() > 1e10 {
                bj *= 1e-10;
                bjp *= 1e-10;
                ret *= 1e-10;
                sum *= 1e-10;
            }
            if jsum {
                sum += bj;
            }
            jsum = !jsum;
            if j as f64 == n {
                ret = bjp;
            }
            j -= 1;
        }
        sum = 2.0 * sum - bj;
        ret /= sum;
    }
    ret
}

/// `BESSELY`: the Bessel function of the second kind, `Yn(x)`.
pub(crate) fn bessely(x: f64, n: f64) -> f64 {
    const TWO_OVER_PI: f64 = std::f64::consts::FRAC_2_PI;

    fn bessel0(x: f64) -> f64 {
        const A1A: &[f64] = &[
            228.462_273_3,
            -86_327.927_57,
            10_879_881.29,
            -512_359_803.6,
            7_062_834_065.0,
            -2_957_821_389.0,
        ];
        const A2A: &[f64] = &[
            1.0,
            226.103_024_4,
            47_447.264_70,
            7_189_466.438,
            745_249_964.8,
            40_076_544_269.0,
        ];
        const A1B: &[f64] = &[
            0.209_388_721_1e-6,
            -0.207_337_063_9e-5,
            0.273_451_040_7e-4,
            -0.109_862_862_7e-2,
            1.0,
        ];
        const A2B: &[f64] = &[
            -0.934_945_152e-7,
            0.762_109_516_1e-6,
            -0.691_114_765_1e-5,
            0.143_048_876_5e-3,
            -0.156_249_999_5e-1,
        ];
        let y = x * x;
        let xx = x - 0.785_398_164;
        if x < 8.0 {
            let a1 = horner(A1A, y);
            let a2 = horner(A2A, y);
            a1 / a2 + TWO_OVER_PI * besselj(x, 0.0) * x.ln()
        } else {
            let y = 64.0 / y;
            let a1 = horner(A1B, y);
            let a2 = horner(A2B, y);
            (TWO_OVER_PI / x).sqrt() * (xx.sin() * a1 + xx.cos() * a2 * 8.0 / x)
        }
    }

    fn bessel1(x: f64) -> f64 {
        const A1A: &[f64] = &[
            8_511.937_935,
            -4_237_922.726,
            734_926_455.1,
            -51_534_381_390.0,
            1_275_274_390_000.0,
            -4_900_604_943_000.0,
        ];
        const A2A: &[f64] = &[
            1.0,
            354.963_288_5,
            102_042.605,
            22_459_040.02,
            3_733_650_367.0,
            424_441_966_400.0,
            24_995_805_700_000.0,
        ];
        const A1B: &[f64] = &[
            -0.240_337_019e-6,
            0.245_752_017_4e-5,
            -0.351_639_649_6e-4,
            0.183_105e-2,
            1.0,
        ];
        const A2B: &[f64] = &[
            0.105_787_412e-6,
            -0.882_289_87e-6,
            0.844_919_909_6e-5,
            -0.200_269_087_3e-3,
            0.046_874_999_95,
        ];
        let y = x * x;
        let xx = x - 2.356_194_491;
        if x < 8.0 {
            let a1 = x * horner(A1A, y);
            let a2 = horner(A2A, y);
            a1 / a2 + TWO_OVER_PI * (besselj(x, 1.0) * x.ln() - 1.0 / x)
        } else {
            let y = 64.0 / y;
            let a1 = horner(A1B, y);
            let a2 = horner(A2B, y);
            (TWO_OVER_PI / x).sqrt() * (xx.sin() * a1 + xx.cos() * a2 * 8.0 / x)
        }
    }

    bessel_wrap(bessel0, bessel1, x, n, -1.0, f64::NEG_INFINITY)
}

/// `BESSELI`: the modified Bessel function, `In(x)`.
pub(crate) fn besseli(x: f64, n: f64) -> f64 {
    fn bessel0(x: f64) -> f64 {
        const A: &[f64] = &[
            0.458_13e-2,
            0.360_768e-1,
            0.265_973_2,
            1.206_749_2,
            3.089_942_4,
            3.515_622_9,
            1.0,
        ];
        const B: &[f64] = &[
            0.392_377e-2,
            -0.164_763_3e-1,
            0.263_553_7e-1,
            -0.205_770_6e-1,
            0.916_281e-2,
            -0.157_565e-2,
            0.225_319e-2,
            0.132_859_2e-1,
            0.398_942_28,
        ];
        if x <= 3.75 {
            horner(A, x * x / (3.75 * 3.75))
        } else {
            x.abs().exp() / x.abs().sqrt() * horner(B, 3.75 / x.abs())
        }
    }

    fn bessel1(x: f64) -> f64 {
        const A: &[f64] = &[
            0.324_11e-3,
            0.301_532e-2,
            0.265_873_3e-1,
            0.150_849_34,
            0.514_988_69,
            0.878_905_94,
            0.5,
        ];
        const B: &[f64] = &[
            -0.420_059e-2,
            0.178_765_4e-1,
            -0.289_531_2e-1,
            0.228_296_7e-1,
            -0.103_155_5e-1,
            0.163_801e-2,
            -0.362_018e-2,
            -0.398_802_4e-1,
            0.398_942_28,
        ];
        if x < 3.75 {
            x * horner(A, x * x / (3.75 * 3.75))
        } else {
            let sign = if x < 0.0 { -1.0 } else { 1.0 };
            sign * x.abs().exp() / x.abs().sqrt() * horner(B, 3.75 / x.abs())
        }
    }

    let n = n.round();
    if n == 0.0 {
        return bessel0(x);
    }
    if n == 1.0 {
        return bessel1(x);
    }
    if n < 0.0 {
        return f64::NAN;
    }
    if x.abs() == 0.0 {
        return 0.0;
    }
    if x == f64::INFINITY {
        return f64::INFINITY;
    }

    let mut ret = 0.0;
    let tox = 2.0 / x.abs();
    let mut bip = 0.0;
    let mut bi = 1.0;
    let m = 2 * ((n + (40.0 * n).sqrt().round()) / 2.0).round() as i64;
    let mut j = m;
    while j > 0 {
        let bim = j as f64 * tox * bi + bip;
        bip = bi;
        bi = bim;
        if bi.abs() > 1e10 {
            bi *= 1e-10;
            bip *= 1e-10;
            ret *= 1e-10;
        }
        if j as f64 == n {
            ret = bip;
        }
        j -= 1;
    }
    ret *= besseli(x, 0.0) / bi;
    if x < 0.0 && (n as i64) % 2 != 0 {
        -ret
    } else {
        ret
    }
}

/// `BESSELK`: the modified Bessel function of the second kind, `Kn(x)`.
pub(crate) fn besselk(x: f64, n: f64) -> f64 {
    fn bessel0(x: f64) -> f64 {
        const A: &[f64] = &[
            0.74e-5,
            0.107_50e-3,
            0.262_698e-2,
            0.348_859_0e-1,
            0.230_697_56,
            0.422_784_20,
            -0.577_215_66,
        ];
        const B: &[f64] = &[
            0.532_08e-3,
            -0.251_540e-2,
            0.587_872e-2,
            -0.106_244_6e-1,
            0.218_956_8e-1,
            -0.783_235_8e-1,
            1.253_314_14,
        ];
        if x <= 2.0 {
            -(x / 2.0).ln() * besseli(x, 0.0) + horner(A, x * x / 4.0)
        } else {
            (-x).exp() / x.sqrt() * horner(B, 2.0 / x)
        }
    }

    fn bessel1(x: f64) -> f64 {
        const A: &[f64] = &[
            -0.4686e-4,
            -0.110_404e-2,
            -0.191_940_2e-1,
            -0.181_568_97,
            -0.672_785_79,
            0.154_431_44,
            1.0,
        ];
        const B: &[f64] = &[
            -0.682_45e-3,
            0.325_614e-2,
            -0.780_353e-2,
            0.150_426_8e-1,
            -0.365_562_0e-1,
            0.234_986_19,
            1.253_314_14,
        ];
        if x <= 2.0 {
            (x / 2.0).ln() * besseli(x, 1.0) + (1.0 / x) * horner(A, x * x / 4.0)
        } else {
            (-x).exp() / x.sqrt() * horner(B, 2.0 / x)
        }
    }

    bessel_wrap(bessel0, bessel1, x, n, 1.0, f64::INFINITY)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every value here came from the `bessel` package the reference library
    /// wraps, so the port is checked against the implementation it copies.
    #[test]
    fn matches_the_reference_approximations() {
        let close = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
        assert!(close(besselj(1.0, 0.0), 0.765_197_683_754_859_2));
        assert!(close(besselj(2.0, 1.0), 0.576_724_807_896_293_8));
        assert!(close(besselj(1.0, 2.0), 0.114_903_484_992_469_38));
        assert!(close(bessely(1.0, 0.0), 0.088_256_971_397_708_06));
        assert!(close(bessely(2.0, 1.0), -0.107_032_431_648_038_1));
        assert!(close(besseli(1.0, 0.0), 1.266_065_848_034_260_1));
        assert!(close(besseli(1.5, 2.0), 0.337_834_620_874_438_1));
        assert!(close(besselk(1.0, 0.0), 0.421_024_421_083_418));
        assert!(close(besselk(2.0, 1.0), 0.139_865_88));
        assert!(close(besselj(-2.0, 3.0), -0.128_943_249_975_627_17));
        assert!(close(besseli(-1.0, 1.0), -0.565_159_097_581_943_5));
    }

    #[test]
    fn non_positive_arguments_follow_the_reference() {
        assert_eq!(bessely(0.0, 0.0), f64::NEG_INFINITY);
        assert!(bessely(-1.0, 0.0).is_nan());
        assert!(besseli(1.0, -1.0).is_nan());
        assert_eq!(besselk(0.0, 0.0), f64::INFINITY);
    }
}
