//! Statistical distributions and regression.
//!
//! The reference for this domain is the `jStat` library, which the JavaScript
//! spreadsheet-function library calls for every distribution. The kernels here
//! reproduce its algorithms term for term - the same continued fraction, the
//! same 12-step Newton iteration, the same guard values - because a spreadsheet
//! formula has to return the number the reference returns, not merely a number
//! that is mathematically correct. Two correct implementations differ in the
//! last digits, and that is enough to fail a comparison.
//!
//! Every kernel takes and returns `f64`. Argument handling - which cells
//! contribute, and what a text or empty cell means - stays in the dispatcher,
//! so this module is pure arithmetic and is tested against the reference
//! directly.

use crate::functions::{erf, gamma_ln};

/// The value `jStat.betafn` returns for a non-positive argument.
///
/// The reference returns `undefined` there, which the caller turns into an
/// error; a NaN carries the same meaning through the arithmetic.
const BETA_UNDEFINED: f64 = f64::NAN;

/// `n` choose `k`, as the reference computes it.
///
/// Above 170 the factorials overflow a `f64`, so the reference switches to the
/// exponential of the log form; the threshold and the switch are part of the
/// observable behaviour.
fn combination(n: f64, m: f64) -> f64 {
    if n > 170.0 || m > 170.0 {
        (combination_ln(n, m)).exp()
    } else {
        (factorial(n) / factorial(m)) / factorial(n - m)
    }
}

fn combination_ln(n: f64, m: f64) -> f64 {
    factorial_ln(n) - factorial_ln(m) - factorial_ln(n - m)
}

fn factorial(n: f64) -> f64 {
    if n < 0.0 { f64::NAN } else { gamma_fn(n + 1.0) }
}

fn factorial_ln(n: f64) -> f64 {
    if n < 0.0 { f64::NAN } else { gamma_ln(n + 1.0) }
}

/// The gamma function, matching `jStat.gammafn` (a rational approximation).
fn gamma_fn(x: f64) -> f64 {
    const P: [f64; 8] = [
        -1.716_185_138_865_495,
        24.765_650_805_575_92,
        -379.804_256_470_945_63,
        629.331_155_312_818_4,
        866.966_202_790_413_3,
        -31_451.272_968_848_367,
        -36_144.413_418_691_176,
        66_456.143_820_240_54,
    ];
    const Q: [f64; 8] = [
        -30.840_230_011_973_9,
        315.350_626_979_604_16,
        -1_015.156_367_490_219_2,
        -3_107.771_671_572_311,
        22_538.118_420_980_15,
        4_755.846_277_527_881,
        -134_659.959_864_969_3,
        -115_132.259_675_553_5,
    ];
    if x > 171.624_376_953_607_6 {
        return f64::INFINITY;
    }
    let mut y = x;
    let mut fact = 0.0;
    if y <= 0.0 {
        let res = y % 1.0 + 3.6e-16;
        if res != 0.0 {
            let sign = if ((y as i64) & 1) == 0 { 1.0 } else { -1.0 };
            fact = sign * std::f64::consts::PI / (std::f64::consts::PI * res).sin();
            y = 1.0 - y;
        } else {
            return f64::INFINITY;
        }
    }
    let yi = y;
    let mut n = 0.0;
    let z;
    if y < 1.0 {
        z = y;
        y += 1.0;
    } else {
        n = (y as i64) as f64 - 1.0;
        y -= n;
        z = y - 1.0;
    }
    let mut xnum = 0.0;
    let mut xden = 0.0;
    for i in 0..8 {
        xnum = (xnum + P[i]) * z;
        xden = xden * z + Q[i];
    }
    let mut res = xnum / xden + 1.0;
    if yi < y {
        res /= yi;
    } else if yi > y {
        let mut i = 0.0;
        while i < n {
            res *= y;
            y += 1.0;
            i += 1.0;
        }
    }
    if fact != 0.0 {
        res = fact / res;
    }
    res
}

/// The beta function, matching `jStat.betafn`.
fn betafn(x: f64, y: f64) -> f64 {
    if x <= 0.0 || y <= 0.0 {
        return BETA_UNDEFINED;
    }
    if x + y > 170.0 {
        (betaln(x, y)).exp()
    } else {
        gamma_fn(x) * gamma_fn(y) / gamma_fn(x + y)
    }
}

fn betaln(x: f64, y: f64) -> f64 {
    gamma_ln(x) + gamma_ln(y) - gamma_ln(x + y)
}

/// The lower regularized incomplete gamma function `P(a, x)`, matching
/// `jStat.lowRegGamma`. The iteration count is derived from `a` exactly as the
/// reference derives it, so the truncation agrees.
pub(crate) fn low_reg_gamma(a: f64, x: f64) -> f64 {
    let aln = gamma_ln(a);
    let mut ap = a;
    let mut sum = 1.0 / a;
    let mut del = sum;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / 1.0e-30;
    let mut d = 1.0 / b;
    let mut h = d;
    let mut i = 1.0;

    let itmax = ((a.max(1.0 / a).ln() * 8.5 + a * 0.4 + 17.0).ceil()).max(0.0);

    if x < 0.0 || a <= 0.0 {
        return f64::NAN;
    }
    if x < a + 1.0 {
        while i <= itmax {
            ap += 1.0;
            del *= x / ap;
            sum += del;
            i += 1.0;
        }
        return sum * (-x + a * x.ln() - aln).exp();
    }

    while i <= itmax {
        let an = -i * (i - a);
        b += 2.0;
        d = an * d + b;
        c = b + an / c;
        d = 1.0 / d;
        h *= d * c;
        i += 1.0;
    }
    1.0 - h * (-x + a * x.ln() - aln).exp()
}

/// The continued fraction for the incomplete beta function, by modified
/// Lentz's method, matching `jStat.betacf`.
fn betacf(x: f64, a: f64, b: f64) -> f64 {
    let fpmin = 1e-30;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < fpmin {
        d = fpmin;
    }
    d = 1.0 / d;
    let mut h = d;

    for m in 1..=100 {
        let m = m as f64;
        let m2 = 2.0 * m;
        let mut aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < fpmin {
            d = fpmin;
        }
        c = 1.0 + aa / c;
        if c.abs() < fpmin {
            c = fpmin;
        }
        d = 1.0 / d;
        h *= d * c;
        aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < fpmin {
            d = fpmin;
        }
        c = 1.0 + aa / c;
        if c.abs() < fpmin {
            c = fpmin;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 3e-7 {
            break;
        }
    }
    h
}

/// The incomplete beta function `I_x(a, b)`, matching `jStat.ibeta`.
pub(crate) fn ibeta(x: f64, a: f64, b: f64) -> f64 {
    let bt = if x == 0.0 || x == 1.0 {
        0.0
    } else {
        (gamma_ln(a + b) - gamma_ln(a) - gamma_ln(b) + a * x.ln() + b * (1.0 - x).ln()).exp()
    };
    if !(0.0..=1.0).contains(&x) {
        return f64::NAN;
    }
    if x < (a + 1.0) / (a + b + 2.0) {
        bt * betacf(x, a, b) / a
    } else {
        1.0 - bt * betacf(1.0 - x, b, a) / b
    }
}

/// The inverse of the incomplete beta function, matching `jStat.ibetainv`.
pub(crate) fn ibetainv(p: f64, a: f64, b: f64) -> f64 {
    let eps = 1e-8;
    let a1 = a - 1.0;
    let b1 = b - 1.0;
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let mut x;
    if a >= 1.0 && b >= 1.0 {
        let pp = if p < 0.5 { p } else { 1.0 - p };
        let t = (-2.0 * pp.ln()).sqrt();
        x = (2.30753 + t * 0.27061) / (1.0 + t * (0.99229 + t * 0.04481)) - t;
        if p < 0.5 {
            x = -x;
        }
        let al = (x * x - 3.0) / 6.0;
        let h = 2.0 / (1.0 / (2.0 * a - 1.0) + 1.0 / (2.0 * b - 1.0));
        let w = (x * (al + h).sqrt() / h)
            - (1.0 / (2.0 * b - 1.0) - 1.0 / (2.0 * a - 1.0)) * (al + 5.0 / 6.0 - 2.0 / (3.0 * h));
        x = a / (a + b * (2.0 * w).exp());
    } else {
        let lna = (a / (a + b)).ln();
        let lnb = (b / (a + b)).ln();
        let t = (a * lna).exp() / a;
        let u = (b * lnb).exp() / b;
        let w = t + u;
        if p < t / w {
            x = (a * w * p).powf(1.0 / a);
        } else {
            x = 1.0 - (b * w * (1.0 - p)).powf(1.0 / b);
        }
    }
    let afac = -gamma_ln(a) - gamma_ln(b) + gamma_ln(a + b);
    for j in 0..10 {
        if x == 0.0 || x == 1.0 {
            return x;
        }
        let err = ibeta(x, a, b) - p;
        let t = (a1 * x.ln() + b1 * (1.0 - x).ln() + afac).exp();
        let u = err / t;
        let step = u / (1.0 - 0.5 * (1.0f64).min(u * (a1 / x - b1 / (1.0 - x))));
        x -= step;
        if x <= 0.0 {
            x = 0.5 * (x + step);
        }
        if x >= 1.0 {
            x = 0.5 * (x + step + 1.0);
        }
        if step.abs() < eps * x && j > 0 {
            break;
        }
    }
    x
}

/// The inverse of the lower regularized incomplete gamma function, matching
/// `jStat.gammapinv`.
pub(crate) fn gammapinv(p: f64, a: f64) -> f64 {
    let eps = 1e-8;
    let a1 = a - 1.0;
    let gln = gamma_ln(a);
    if p >= 1.0 {
        return 100.0f64.max(a + 100.0 * a.sqrt());
    }
    if p <= 0.0 {
        return 0.0;
    }
    let mut x;
    let mut afac = 0.0;
    if a > 1.0 {
        let lna1 = a1.ln();
        afac = (a1 * (lna1 - 1.0) - gln).exp();
        let pp = if p < 0.5 { p } else { 1.0 - p };
        let t = (-2.0 * pp.ln()).sqrt();
        x = (2.30753 + t * 0.27061) / (1.0 + t * (0.99229 + t * 0.04481)) - t;
        if p < 0.5 {
            x = -x;
        }
        x = 1e-3f64.max(a * (1.0 - 1.0 / (9.0 * a) - x / (3.0 * a.sqrt())).powi(3));
    } else {
        let t = 1.0 - a * (0.253 + a * 0.12);
        if p < t {
            x = (p / t).powf(1.0 / a);
        } else {
            x = 1.0 - (1.0 - (p - t) / (1.0 - t)).ln();
        }
    }
    for _ in 0..12 {
        if x <= 0.0 {
            return 0.0;
        }
        let err = low_reg_gamma(a, x) - p;
        let t = if a > 1.0 {
            afac * (-(x - a1) + a1 * (x.ln() - a1.ln())).exp()
        } else {
            (-x + a1 * x.ln() - gln).exp()
        };
        let u = err / t;
        let step = u / (1.0 - 0.5 * (1.0f64).min(u * ((a - 1.0) / x - 1.0)));
        x -= step;
        if x <= 0.0 {
            x = 0.5 * (x + step);
        }
        if step.abs() < eps * x {
            break;
        }
    }
    x
}

/// The inverse of the complementary error function, matching `jStat.erfcinv`.
pub(crate) fn erfcinv(p: f64) -> f64 {
    if p >= 2.0 {
        return -100.0;
    }
    if p <= 0.0 {
        return 100.0;
    }
    let pp = if p < 1.0 { p } else { 2.0 - p };
    let t = (-2.0 * (pp / 2.0).ln()).sqrt();
    let mut x = -std::f64::consts::FRAC_1_SQRT_2
        * ((2.30753 + t * 0.27061) / (1.0 + t * (0.99229 + t * 0.04481)) - t);
    for _ in 0..2 {
        let err = erfc(x) - pp;
        x += err / (std::f64::consts::FRAC_2_SQRT_PI * (-x * x).exp() - x * err);
    }
    if p < 1.0 { x } else { -x }
}

fn erfc(x: f64) -> f64 {
    1.0 - erf(x)
}

// --- Normal and lognormal ---

pub(crate) fn normal_pdf(x: f64, mean: f64, std: f64) -> f64 {
    (-0.5 * (2.0 * std::f64::consts::PI).ln() - std.ln() - (x - mean).powi(2) / (2.0 * std * std))
        .exp()
}

pub(crate) fn normal_cdf(x: f64, mean: f64, std: f64) -> f64 {
    0.5 * (1.0 + erf((x - mean) / (2.0 * std * std).sqrt()))
}

pub(crate) fn normal_inv(p: f64, mean: f64, std: f64) -> f64 {
    -std::f64::consts::SQRT_2 * std * erfcinv(2.0 * p) + mean
}

pub(crate) fn lognormal_pdf(x: f64, mu: f64, sigma: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    (-x.ln()
        - 0.5 * (2.0 * std::f64::consts::PI).ln()
        - sigma.ln()
        - (x.ln() - mu).powi(2) / (2.0 * sigma * sigma))
        .exp()
}

pub(crate) fn lognormal_cdf(x: f64, mu: f64, sigma: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    0.5 + 0.5 * erf((x.ln() - mu) / (2.0 * sigma * sigma).sqrt())
}

pub(crate) fn lognormal_inv(p: f64, mu: f64, sigma: f64) -> f64 {
    (-std::f64::consts::SQRT_2 * sigma * erfcinv(2.0 * p) + mu).exp()
}

// --- Chi-square ---

pub(crate) fn chisq_pdf(x: f64, dof: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    if x == 0.0 && dof == 2.0 {
        return 0.5;
    }
    ((dof / 2.0 - 1.0) * x.ln() - x / 2.0 - (dof / 2.0) * 2.0f64.ln() - gamma_ln(dof / 2.0)).exp()
}

pub(crate) fn chisq_cdf(x: f64, dof: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    low_reg_gamma(dof / 2.0, x / 2.0)
}

pub(crate) fn chisq_inv(p: f64, dof: f64) -> f64 {
    2.0 * gammapinv(p, 0.5 * dof)
}

// --- Student t ---

pub(crate) fn studentt_cdf(x: f64, dof: f64) -> f64 {
    let dof2 = dof / 2.0;
    ibeta(
        (x + (x * x + dof).sqrt()) / (2.0 * (x * x + dof).sqrt()),
        dof2,
        dof2,
    )
}

pub(crate) fn studentt_inv(p: f64, dof: f64) -> f64 {
    let x = ibetainv(2.0 * p.min(1.0 - p), 0.5 * dof, 0.5);
    let x = (dof * (1.0 - x) / x).sqrt();
    if p > 0.5 { x } else { -x }
}

// --- F ---

pub(crate) fn f_cdf(x: f64, df1: f64, df2: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    ibeta((df1 * x) / (df1 * x + df2), df1 / 2.0, df2 / 2.0)
}

pub(crate) fn f_pdf(x: f64, df1: f64, df2: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    if df1 <= 2.0 {
        if x == 0.0 && df1 < 2.0 {
            return f64::INFINITY;
        }
        if x == 0.0 && df1 == 2.0 {
            return 1.0;
        }
        return (1.0 / betafn(df1 / 2.0, df2 / 2.0))
            * (df1 / df2).powf(df1 / 2.0)
            * x.powf(df1 / 2.0 - 1.0)
            * (1.0 + (df1 / df2) * x).powf(-(df1 + df2) / 2.0);
    }
    let p = (df1 * x) / (df2 + x * df1);
    let q = df2 / (df2 + x * df1);
    let f = df1 * q / 2.0;
    f * binomial_pdf((df1 - 2.0) / 2.0, (df1 + df2 - 2.0) / 2.0, p)
}

pub(crate) fn f_inv(p: f64, df1: f64, df2: f64) -> f64 {
    df2 / (df1 * (1.0 / ibetainv(p, df1 / 2.0, df2 / 2.0) - 1.0))
}

// --- Beta and gamma ---

pub(crate) fn beta_pdf(x: f64, alpha: f64, beta: f64) -> f64 {
    if x > 1.0 || x < 0.0 {
        return 0.0;
    }
    if alpha == 1.0 && beta == 1.0 {
        return 1.0;
    }
    if alpha < 512.0 && beta < 512.0 {
        (x.powf(alpha - 1.0) * (1.0 - x).powf(beta - 1.0)) / betafn(alpha, beta)
    } else {
        ((alpha - 1.0) * x.ln() + (beta - 1.0) * (1.0 - x).ln() - betaln(alpha, beta)).exp()
    }
}

pub(crate) fn beta_cdf(x: f64, alpha: f64, beta: f64) -> f64 {
    if x > 1.0 {
        return 1.0;
    }
    if x < 0.0 {
        return 0.0;
    }
    ibeta(x, alpha, beta)
}

pub(crate) fn beta_inv(p: f64, alpha: f64, beta: f64) -> f64 {
    ibetainv(p, alpha, beta)
}

pub(crate) fn gamma_pdf(x: f64, shape: f64, scale: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    if x == 0.0 && shape == 1.0 {
        return 1.0 / scale;
    }
    ((shape - 1.0) * x.ln() - x / scale - gamma_ln(shape) - shape * scale.ln()).exp()
}

pub(crate) fn gamma_cdf(x: f64, shape: f64, scale: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    low_reg_gamma(shape, x / scale)
}

pub(crate) fn gamma_inv(p: f64, shape: f64, scale: f64) -> f64 {
    gammapinv(p, shape) * scale
}

// --- Binomial, Poisson, exponential, Weibull ---

pub(crate) fn binomial_pdf(k: f64, n: f64, p: f64) -> f64 {
    if p == 0.0 || p == 1.0 {
        return if (n * p) == k { 1.0 } else { 0.0 };
    }
    combination(n, k) * p.powf(k) * (1.0 - p).powf(n - k)
}

/// The continued fraction `betinc` the reference uses inside `binomial.cdf`.
///
/// It is a different continued fraction from `betacf` above and converges by a
/// relative-difference test rather than a fixed step count, so it is kept
/// separate rather than folded into the shared one.
fn betinc(x: f64, a: f64, b: f64, eps: f64) -> f64 {
    let mut a0 = 0.0f64;
    let mut b0 = 1.0f64;
    let mut a1 = 1.0f64;
    let mut b1 = 1.0f64;
    let mut m9 = 0.0f64;
    let mut a2 = 0.0f64;

    while ((a1 - a2) / a1).abs() > eps {
        a2 = a1;
        let c9 = -(a + m9) * (a + b + m9) * x / (a + 2.0 * m9) / (a + 2.0 * m9 + 1.0);
        a0 = a1 + c9 * a0;
        b0 = b1 + c9 * b0;
        m9 += 1.0;
        let c9 = m9 * (b - m9) * x / (a + 2.0 * m9 - 1.0) / (a + 2.0 * m9);
        a1 = a0 + c9 * a1;
        b1 = b0 + c9 * b1;
        a0 /= b1;
        b0 /= b1;
        a1 /= b1;
        b1 = 1.0;
    }
    a1 / a
}

pub(crate) fn binomial_cdf(x: f64, n: f64, p: f64) -> f64 {
    let eps = 1e-10;
    if x < 0.0 {
        return 0.0;
    }
    if x >= n {
        return 1.0;
    }
    if p < 0.0 || p > 1.0 || n <= 0.0 {
        return f64::NAN;
    }
    let x = x.floor();
    let z = p;
    let a = x + 1.0;
    let b = n - x;
    let s = a + b;
    let bt = (gamma_ln(s) - gamma_ln(b) - gamma_ln(a) + a * z.ln() + b * (1.0 - z).ln()).exp();
    let betacdf = if z < (a + 1.0) / (s + 2.0) {
        bt * betinc(z, a, b, eps)
    } else {
        1.0 - bt * betinc(1.0 - z, b, a, eps)
    };
    ((1.0 - betacdf) * (1.0 / eps)).round() / (1.0 / eps)
}

pub(crate) fn poisson_pdf(k: f64, l: f64) -> f64 {
    if l < 0.0 || (k % 1.0) != 0.0 || k < 0.0 {
        return 0.0;
    }
    l.powf(k) * (-l).exp() / factorial(k)
}

pub(crate) fn poisson_cdf(x: f64, l: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut k = 0.0;
    while k <= x {
        sum += poisson_pdf(k, l);
        k += 1.0;
    }
    sum
}

pub(crate) fn exponential_pdf(x: f64, rate: f64) -> f64 {
    if x < 0.0 {
        0.0
    } else {
        rate * (-rate * x).exp()
    }
}

pub(crate) fn exponential_cdf(x: f64, rate: f64) -> f64 {
    if x < 0.0 {
        0.0
    } else {
        1.0 - (-rate * x).exp()
    }
}

pub(crate) fn weibull_pdf(x: f64, scale: f64, shape: f64) -> f64 {
    if x < 0.0 || scale < 0.0 || shape < 0.0 {
        return 0.0;
    }
    (shape / scale) * (x / scale).powf(shape - 1.0) * (-((x / scale).powf(shape))).exp()
}

pub(crate) fn weibull_cdf(x: f64, scale: f64, shape: f64) -> f64 {
    if x < 0.0 {
        0.0
    } else {
        1.0 - (-((x / scale).powf(shape))).exp()
    }
}

// --- Hypergeometric and negative binomial ---

/// The hypergeometric probability mass, as the reference computes it directly
/// from binomial coefficients rather than through the recursive form.
pub(crate) fn hypgeom_pdf(k: f64, n: f64, m: f64, big_n: f64) -> f64 {
    (combination(m, k) * combination(big_n - m, n - k)) / combination(big_n, n)
}

pub(crate) fn hypgeom_cdf(x: f64, n: f64, m: f64, big_n: f64) -> f64 {
    let mut result = 0.0;
    let mut i = 0.0;
    while i <= x {
        result += hypgeom_pdf(i, n, m, big_n);
        i += 1.0;
    }
    result
}

pub(crate) fn negbinom_pdf(k: f64, r: f64, p: f64) -> f64 {
    if k < 0.0 {
        return 0.0;
    }
    combination(k + r - 1.0, r - 1.0) * (1.0 - p).powf(k) * p.powf(r)
}

pub(crate) fn negbinom_cdf(x: f64, r: f64, p: f64) -> f64 {
    if x < 0.0 {
        return 0.0;
    }
    let mut sum = 0.0;
    let mut k = 0.0;
    while k <= x {
        sum += negbinom_pdf(k, r, p);
        k += 1.0;
    }
    sum
}

// --- Descriptive statistics and regression ---

pub(crate) fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

/// The sum of squared errors from the mean, as `jStat.sumsqerr` computes it.
fn sum_sq_err(values: &[f64]) -> f64 {
    let m = mean(values);
    values.iter().map(|v| (v - m) * (v - m)).sum()
}

/// The sample standard deviation, which the correlation coefficient divides by.
fn stdev_sample(values: &[f64]) -> f64 {
    (sum_sq_err(values) / (values.len() as f64 - 1.0)).sqrt()
}

/// The covariance of two equally long samples, dividing by `n - 1`.
pub(crate) fn covariance_sample(a: &[f64], b: &[f64]) -> f64 {
    let u = mean(a);
    let v = mean(b);
    let n = a.len().min(b.len());
    let mut sum = 0.0;
    for i in 0..n {
        sum += (a[i] - u) * (b[i] - v);
    }
    sum / (n as f64 - 1.0)
}

/// The covariance of two equally long series, dividing by `n`, which is what
/// `COVARIANCE.P` reports.
pub(crate) fn covariance_population(a: &[f64], b: &[f64]) -> f64 {
    let u = mean(a);
    let v = mean(b);
    let n = a.len().min(b.len());
    let mut sum = 0.0;
    for i in 0..n {
        sum += (a[i] - u) * (b[i] - v);
    }
    sum / n as f64
}

/// The Pearson correlation coefficient, matching `jStat.corrcoeff`.
pub(crate) fn correl(a: &[f64], b: &[f64]) -> f64 {
    covariance_sample(a, b) / stdev_sample(a) / stdev_sample(b)
}

/// The Pearson product-moment coefficient as `PEARSON` computes it, which uses
/// population sums; the `n` in each term cancels.
pub(crate) fn pearson(a: &[f64], b: &[f64]) -> f64 {
    let xmean = mean(a);
    let ymean = mean(b);
    let n = a.len().min(b.len());
    let mut num = 0.0;
    let mut den1 = 0.0;
    let mut den2 = 0.0;
    for i in 0..n {
        num += (a[i] - xmean) * (b[i] - ymean);
        den1 += (a[i] - xmean).powi(2);
        den2 += (b[i] - ymean).powi(2);
    }
    num / (den1 * den2).sqrt()
}

/// The least-squares slope of `ys` against `xs`, matching `SLOPE`.
pub(crate) fn slope(ys: &[f64], xs: &[f64]) -> f64 {
    let xmean = mean(xs);
    let ymean = mean(ys);
    let n = xs.len().min(ys.len());
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        num += (xs[i] - xmean) * (ys[i] - ymean);
        den += (xs[i] - xmean).powi(2);
    }
    num / den
}

/// The predicted value at `x`, matching `FORECAST` (and so `INTERCEPT`, which
/// is `FORECAST(0, ...)`).
pub(crate) fn forecast(x: f64, ys: &[f64], xs: &[f64]) -> f64 {
    let xmean = mean(xs);
    let ymean = mean(ys);
    let n = xs.len().min(ys.len());
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        num += (xs[i] - xmean) * (ys[i] - ymean);
        den += (xs[i] - xmean).powi(2);
    }
    let b = num / den;
    let a = ymean - b * xmean;
    a + b * x
}

/// The standard error of the predicted `y`, matching `STEYX`.
pub(crate) fn steyx(ys: &[f64], xs: &[f64]) -> f64 {
    let xmean = mean(xs);
    let ymean = mean(ys);
    let n = xs.len().min(ys.len());
    let mut lft = 0.0;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..n {
        lft += (ys[i] - ymean).powi(2);
        num += (xs[i] - xmean) * (ys[i] - ymean);
        den += (xs[i] - xmean).powi(2);
    }
    ((lft - (num * num) / den) / (n as f64 - 2.0)).sqrt()
}

/// The probability of a Student's `t`-test, as the reference's `T.TEST`
/// reports it.
///
/// The JavaScript reference takes only the two arrays: it ignores the tail and
/// type arguments, always computes the unequal-variance statistic, and always
/// uses `n1 + n2 - 2` degrees of freedom. That is not what the Excel
/// documentation describes, but it is what the oracle returns, so the port
/// follows it rather than inventing the documented behaviour.
pub(crate) fn ttest(a: &[f64], b: &[f64]) -> f64 {
    let m1 = mean(a);
    let m2 = mean(b);
    let s1 = sum_sq_err(a) / (a.len() as f64 - 1.0);
    let s2 = sum_sq_err(b) / (b.len() as f64 - 1.0);
    let t = (m1 - m2).abs() / (s1 / a.len() as f64 + s2 / b.len() as f64).sqrt();
    let dof = a.len() as f64 + b.len() as f64 - 2.0;
    (1.0 - studentt_cdf(t, dof)) * 2.0
}

/// The `F`-test ratio of two sample variances, matching `F.TEST`.
pub(crate) fn ftest(a: &[f64], b: &[f64]) -> f64 {
    let v1 = sum_sq_err(a) / (a.len() as f64 - 1.0);
    let v2 = sum_sq_err(b) / (b.len() as f64 - 1.0);
    v1 / v2
}

/// The `z`-test probability, matching `Z.TEST`.
pub(crate) fn ztest(values: &[f64], x: f64, sigma: Option<f64>) -> f64 {
    let sigma = sigma.unwrap_or_else(|| stdev_sample(values));
    let n = values.len() as f64;
    1.0 - normal_cdf((mean(values) - x) / (sigma / n.sqrt()), 0.0, 1.0)
}

/// The chi-squared test for independence, matching `CHISQ.TEST` for the
/// one-dimensional case the reference handles: the statistic is summed and the
/// survival function of the chi-square with `rows - 1` degrees of freedom is
/// returned through the same series the reference uses.
pub(crate) fn chitest(actual: &[f64], expected: &[f64]) -> f64 {
    let dof = actual.len() as f64 - 1.0;
    let mut xsqr = 0.0;
    for i in 0..actual.len().min(expected.len()) {
        xsqr += (actual[i] - expected[i]).powi(2) / expected[i];
    }
    chisq_survival_series(xsqr, dof)
}

/// The chi-square survival function by the reference's explicit series.
///
/// `jStat`'s `CHISQ.TEST` does not use `chisquare.cdf`; it sums the terms
/// itself and rounds to six decimals, and a spreadsheet test is compared to
/// that rounded value, so the series is reproduced rather than the closed form.
fn chisq_survival_series(xsqr: f64, dof: f64) -> f64 {
    let mut p = (-0.5 * xsqr).exp();
    if dof % 2.0 == 1.0 {
        p *= (2.0 * xsqr / std::f64::consts::PI).sqrt();
    }
    let mut k = dof;
    while k >= 2.0 {
        p = (p * xsqr) / k;
        k -= 2.0;
    }
    let mut t = p;
    let mut a = dof;
    let mut pp = p;
    while t > 0.0000000001 * pp {
        a += 2.0;
        t = (t * xsqr) / a;
        pp += t;
    }
    let survival = 1.0 - pp;
    (survival * 1_000_000.0).round() / 1_000_000.0
}

/// The probability that values fall between two limits, matching `PROB`.
///
/// The reference sorts a copy to walk the values but looks each probability up
/// in the original order, so a repeated value takes the probability of its
/// first occurrence.
pub(crate) fn prob(xs: &[f64], probs: &[f64], lower: f64, upper: f64) -> f64 {
    if lower == upper {
        return xs
            .iter()
            .position(|v| *v == lower)
            .map_or(0.0, |i| probs[i]);
    }
    let mut sorted = xs.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let mut result = 0.0;
    for value in &sorted {
        if *value >= lower
            && *value <= upper
            && let Some(index) = xs.iter().position(|v| v == value)
        {
            result += probs[index];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    #[test]
    fn normal_matches_published_values() {
        assert!(close(normal_cdf(1.96, 0.0, 1.0), 0.9750021048517796));
        assert!(close(normal_cdf(0.0, 0.0, 1.0), 0.5));
        assert!(close(normal_inv(0.975, 0.0, 1.0), 1.9599639845400545));
        assert!(close(normal_pdf(0.0, 0.0, 1.0), 0.3989422804014327));
    }

    #[test]
    fn chi_square_and_t_match_published_values() {
        assert!(close(chisq_cdf(3.0, 5.0), 0.30001416412137294));
        assert!(close(studentt_cdf(2.0, 5.0), 0.94903026058521));
        assert!(close(studentt_inv(0.975, 5.0), 2.5705818347641416));
    }

    #[test]
    fn regression_recovers_a_known_line() {
        let xs = [1.0, 2.0, 3.0, 4.0];
        let ys = [3.0, 5.0, 7.0, 9.0];
        assert!(close(slope(&ys, &xs), 2.0));
        assert!(close(forecast(0.0, &ys, &xs), 1.0));
        assert!(close(pearson(&xs, &ys), 1.0));
        assert!(close(steyx(&ys, &xs), 0.0));
    }
}
