//! Financial functions.
//!
//! Every function here follows the spreadsheet sign convention: money that
//! leaves the account is negative and money that arrives is positive. That is
//! why `PMT` on a positive present value returns a negative payment - the
//! borrower pays out.
//!
//! The present-value family all derive from one identity, that the present
//! value of a level payment stream is `pmt * (1 - (1+r)^-n) / r`, and the
//! functions are that identity solved for a different unknown.

/// A cash flow series, used by `IRR`, `MIRR`, `XNPV` and `XIRR`.
pub(crate) struct CashFlows<'a> {
    /// The amounts, in time order.
    pub(crate) amounts: &'a [f64],
}

impl CashFlows<'_> {
    /// The net present value at a given rate.
    fn npv(&self, rate: f64) -> f64 {
        self.amounts
            .iter()
            .enumerate()
            .map(|(i, amount)| amount / (1.0 + rate).powi(i as i32))
            .sum()
    }

    /// The internal rate of return, by Newton's method with a bisection
    /// fallback.
    ///
    /// Newton converges in a handful of steps on a well-behaved series but can
    /// wander off; the bisection guarantees a bracket is not lost. The result
    /// is `None` when no rate in the search range zeroes the net present value.
    pub(crate) fn irr(&self, guess: f64) -> Option<f64> {
        if self.amounts.len() < 2 {
            return None;
        }
        // A series with no sign change never crosses zero.
        let has_positive = self.amounts.iter().any(|a| *a > 0.0);
        let has_negative = self.amounts.iter().any(|a| *a < 0.0);
        if !(has_positive && has_negative) {
            return None;
        }
        let mut rate = guess;
        for _ in 0..100 {
            let value = self.npv(rate);
            if value.abs() < 1e-9 {
                return Some(rate);
            }
            // The derivative of the net present value with respect to the
            // rate, which Newton needs.
            let slope: f64 = self
                .amounts
                .iter()
                .enumerate()
                .skip(1)
                .map(|(i, amount)| -(i as f64) * amount / (1.0 + rate).powi(i as i32 + 1))
                .sum();
            if slope.abs() < f64::EPSILON {
                break;
            }
            let next = rate - value / slope;
            if !next.is_finite() || next <= -1.0 {
                break;
            }
            if (next - rate).abs() < 1e-12 {
                return Some(next);
            }
            rate = next;
        }
        // Bisection over a wide bracket.
        let (mut lo, mut hi) = (-0.999_999, 10.0);
        let mut f_lo = self.npv(lo);
        if f_lo * self.npv(hi) > 0.0 {
            return None;
        }
        for _ in 0..200 {
            let mid = f64::midpoint(lo, hi);
            let f_mid = self.npv(mid);
            if f_mid.abs() < 1e-12 {
                return Some(mid);
            }
            if f_lo * f_mid < 0.0 {
                hi = mid;
            } else {
                lo = mid;
                f_lo = f_mid;
            }
        }
        Some(f64::midpoint(lo, hi))
    }
}

/// The compound growth factor `(1 + rate)^periods`.
fn growth(rate: f64, periods: f64) -> f64 {
    (1.0 + rate).powf(periods)
}

/// `FV`: the future value of an investment with level payments.
pub(crate) fn fv(rate: f64, nper: f64, pmt: f64, pv: f64, kind: f64) -> Option<f64> {
    if rate == 0.0 {
        return Some(-(pv + pmt * nper));
    }
    let g = growth(rate, nper);
    Some(-(pv * g + pmt * (1.0 + rate * kind) * (g - 1.0) / rate))
}

/// `PV`: the present value of an investment with level payments.
pub(crate) fn pv(rate: f64, nper: f64, pmt: f64, fv_value: f64, kind: f64) -> Option<f64> {
    if rate == 0.0 {
        return Some(-(fv_value + pmt * nper));
    }
    let g = growth(rate, nper);
    Some(-(fv_value + pmt * (1.0 + rate * kind) * (g - 1.0) / rate) / g)
}

/// `PMT`: the level payment that clears a loan.
pub(crate) fn pmt(rate: f64, nper: f64, pv_value: f64, fv_value: f64, kind: f64) -> Option<f64> {
    if nper == 0.0 {
        return None;
    }
    if rate == 0.0 {
        return Some(-(pv_value + fv_value) / nper);
    }
    let g = growth(rate, nper);
    Some(-(pv_value * g + fv_value) * rate / ((1.0 + rate * kind) * (g - 1.0)))
}

/// `NPER`: how many payments a loan needs.
pub(crate) fn nper(
    rate: f64,
    pmt_value: f64,
    pv_value: f64,
    fv_value: f64,
    kind: f64,
) -> Option<f64> {
    if rate == 0.0 {
        if pmt_value == 0.0 {
            return None;
        }
        return Some(-(pv_value + fv_value) / pmt_value);
    }
    let adjusted = pmt_value * (1.0 + rate * kind);
    let numerator = adjusted - fv_value * rate;
    let denominator = pv_value * rate + adjusted;
    // The logarithm needs a positive argument, so the two must share a sign.
    if numerator * denominator <= 0.0 {
        return None;
    }
    let n = (numerator / denominator).ln() / (1.0 + rate).ln();
    if n.is_finite() { Some(n) } else { None }
}

/// `IPMT`: the interest part of one payment.
pub(crate) fn ipmt(
    rate: f64,
    per: f64,
    nper: f64,
    pv_value: f64,
    fv_value: f64,
    kind: f64,
) -> Option<f64> {
    if per < 1.0 || per > nper {
        return None;
    }
    let payment = pmt(rate, nper, pv_value, fv_value, kind)?;
    let interest = if per == 1.0 {
        // No interest has accrued yet on the first payment; when it is due at
        // the start of the period the balance has not been outstanding at all.
        if kind == 1.0 { 0.0 } else { -pv_value }
    } else if kind == 1.0 {
        // The balance before this payment, which is one period further along
        // than the end-of-period case.
        fv(rate, per - 2.0, payment, pv_value, 1.0)? - payment
    } else {
        fv(rate, per - 1.0, payment, pv_value, 0.0)?
    };
    Some(interest * rate)
}

/// `PPMT`: the principal part of one payment, which is the payment less the
/// interest.
pub(crate) fn ppmt(
    rate: f64,
    per: f64,
    nper: f64,
    pv_value: f64,
    fv_value: f64,
    kind: f64,
) -> Option<f64> {
    let payment = pmt(rate, nper, pv_value, fv_value, kind)?;
    Some(payment - ipmt(rate, per, nper, pv_value, fv_value, kind)?)
}

/// `CUMIPMT`: the total interest paid over a range of periods.
pub(crate) fn cumipmt(
    rate: f64,
    nper: f64,
    pv_value: f64,
    start: f64,
    end: f64,
    kind: f64,
) -> Option<f64> {
    if rate <= 0.0 || nper <= 0.0 || start < 1.0 || end < start || end > nper {
        return None;
    }
    let mut total = 0.0;
    let mut per = start;
    while per <= end {
        total += ipmt(rate, per, nper, pv_value, 0.0, kind)?;
        per += 1.0;
    }
    Some(total)
}

/// `CUMPRINC`: the total principal repaid over a range of periods.
pub(crate) fn cumprinc(
    rate: f64,
    nper: f64,
    pv_value: f64,
    start: f64,
    end: f64,
    kind: f64,
) -> Option<f64> {
    if rate <= 0.0 || nper <= 0.0 || start < 1.0 || end < start || end > nper {
        return None;
    }
    let mut total = 0.0;
    let mut per = start;
    while per <= end {
        total += ppmt(rate, per, nper, pv_value, 0.0, kind)?;
        per += 1.0;
    }
    Some(total)
}

/// `ISPMT`: the interest paid in one period of an interest-only schedule,
/// where the principal is repaid in equal instalments and interest is charged
/// on the declining balance.
pub(crate) fn ispmt(rate: f64, per: f64, nper: f64, pv_value: f64) -> Option<f64> {
    if nper == 0.0 {
        return None;
    }
    Some(pv_value * rate * (per / nper - 1.0))
}

/// `FVSCHEDULE`: the future value under a series of different rates.
pub(crate) fn fvschedule(principal: f64, rates: &[f64]) -> f64 {
    rates.iter().fold(principal, |acc, r| acc * (1.0 + r))
}

/// `NPV`: the present value of a series of future cash flows.
///
/// The first amount is discounted one period, not zero - a spreadsheet's `NPV`
/// is not the same as the net present value of the initial outlay, which is
/// why `NPV` is combined with a negative flow at time zero.
pub(crate) fn npv(rate: f64, amounts: &[f64]) -> Option<f64> {
    if rate == -1.0 {
        return None;
    }
    Some(
        amounts
            .iter()
            .enumerate()
            .map(|(i, a)| a / (1.0 + rate).powi(i as i32 + 1))
            .sum(),
    )
}

/// `MIRR`: the modified internal rate of return.
pub(crate) fn mirr(amounts: &[f64], finance_rate: f64, reinvest_rate: f64) -> Option<f64> {
    let n = amounts.len();
    if n < 2 {
        return None;
    }
    let negatives: f64 = amounts
        .iter()
        .enumerate()
        .filter(|(_, a)| **a < 0.0)
        .map(|(i, a)| a / (1.0 + finance_rate).powi(i as i32))
        .sum();
    let positives: f64 = amounts
        .iter()
        .enumerate()
        .map(|(i, a)| {
            if *a > 0.0 {
                a * (1.0 + reinvest_rate).powi((n - 1 - i) as i32)
            } else {
                0.0
            }
        })
        .sum();
    if negatives == 0.0 || positives == 0.0 {
        return None;
    }
    Some((positives / -negatives).powf(1.0 / (n as f64 - 1.0)) - 1.0)
}

/// `XNPV`: the net present value of cash flows on arbitrary dates.
///
/// Each flow is discounted by its own distance from the first date, measured
/// in days over a 365-day year.
pub(crate) fn xnpv(rate: f64, amounts: &[f64], dates: &[f64]) -> Option<f64> {
    if amounts.len() != dates.len() || amounts.is_empty() || rate <= -1.0 {
        return None;
    }
    let base = dates[0];
    Some(
        amounts
            .iter()
            .zip(dates)
            .map(|(a, d)| a / (1.0 + rate).powf((d - base) / 365.0))
            .sum(),
    )
}

/// `XIRR`: the rate that zeroes [`xnpv`].
pub(crate) fn xirr(amounts: &[f64], dates: &[f64], guess: f64) -> Option<f64> {
    if amounts.len() != dates.len() || amounts.len() < 2 {
        return None;
    }
    let mut rate = guess;
    for _ in 0..100 {
        let value = xnpv(rate, amounts, dates)?;
        if value.abs() < 1e-7 {
            return Some(rate);
        }
        let base = dates[0];
        let slope: f64 = amounts
            .iter()
            .zip(dates)
            .map(|(a, d)| {
                let t = (d - base) / 365.0;
                -t * a / (1.0 + rate).powf(t + 1.0)
            })
            .sum();
        if slope.abs() < f64::EPSILON {
            return None;
        }
        let next = rate - value / slope;
        if !next.is_finite() || next <= -1.0 {
            return None;
        }
        if (next - rate).abs() < 1e-12 {
            return Some(next);
        }
        rate = next;
    }
    Some(rate)
}

/// `SLN`: straight-line depreciation for one period.
pub(crate) fn sln(cost: f64, salvage: f64, life: f64) -> Option<f64> {
    if life == 0.0 {
        return None;
    }
    Some((cost - salvage) / life)
}

/// `SYD`: sum-of-years-digits depreciation for one period.
pub(crate) fn syd(cost: f64, salvage: f64, life: f64, per: f64) -> Option<f64> {
    if life <= 0.0 || per <= 0.0 || per > life {
        return None;
    }
    Some((cost - salvage) * (life - per + 1.0) * 2.0 / (life * (life + 1.0)))
}

/// `DDB`: double-declining-balance depreciation for one period.
///
/// The book value is walked forward from the first period because each
/// period's charge depends on the one before it, and the charge is never
/// allowed to take the book value below the salvage value.
pub(crate) fn ddb(cost: f64, salvage: f64, life: f64, per: f64, factor: f64) -> Option<f64> {
    if cost < 0.0 || salvage < 0.0 || life <= 0.0 || per <= 0.0 || per > life || factor <= 0.0 {
        return None;
    }
    let mut book = cost;
    let mut charge = 0.0;
    let mut period = 1.0;
    while period <= per {
        charge = (book * factor / life).min(book - salvage);
        if charge < 0.0 {
            charge = 0.0;
        }
        book -= charge;
        period += 1.0;
    }
    Some(charge)
}

/// `DB`: fixed-declining-balance depreciation for one period.
pub(crate) fn db(cost: f64, salvage: f64, life: f64, per: f64, month: f64) -> Option<f64> {
    if cost < 0.0 || salvage < 0.0 || life <= 0.0 || per <= 0.0 || per > life + 1.0 {
        return None;
    }
    if cost == 0.0 {
        return Some(0.0);
    }
    // The rate that brings the book value down to the salvage value over the
    // asset's life, rounded to three decimals the way a spreadsheet does.
    let rate = ((1.0 - (salvage / cost).powf(1.0 / life)) * 1000.0).round() / 1000.0;
    let first = cost * rate * month / 12.0;
    if per == 1.0 {
        return Some(first);
    }
    let mut book = cost - first;
    let mut charge = 0.0;
    let mut period = 2.0;
    while period <= per {
        charge = book * rate;
        book -= charge;
        period += 1.0;
    }
    Some(charge)
}

/// `EFFECT`: the effective annual rate from a nominal rate.
pub(crate) fn effect(nominal: f64, npery: f64) -> Option<f64> {
    if nominal <= 0.0 || npery < 1.0 {
        return None;
    }
    Some((1.0 + nominal / npery).powf(npery) - 1.0)
}

/// `NOMINAL`: the nominal annual rate from an effective rate.
pub(crate) fn nominal(effect_rate: f64, npery: f64) -> Option<f64> {
    if effect_rate <= 0.0 || npery < 1.0 {
        return None;
    }
    Some(npery * ((1.0 + effect_rate).powf(1.0 / npery) - 1.0))
}

/// `RRI`: the equivalent interest rate of a growth over a number of periods.
pub(crate) fn rri(nper: f64, pv_value: f64, fv_value: f64) -> Option<f64> {
    if nper <= 0.0 || pv_value == 0.0 {
        return None;
    }
    Some((fv_value / pv_value).powf(1.0 / nper) - 1.0)
}

/// `PDURATION`: how many periods a present value needs to reach a future one.
pub(crate) fn pduration(rate: f64, pv_value: f64, fv_value: f64) -> Option<f64> {
    if rate <= 0.0 || pv_value <= 0.0 || fv_value <= 0.0 {
        return None;
    }
    Some((fv_value / pv_value).ln() / (1.0 + rate).ln())
}

/// `RATE`: the interest rate per period, found by Newton's method.
///
/// The reference uses a fixed 100 iterations and a 1e-10 tolerance, and returns
/// an error when the estimate leaves the domain or the iteration does not
/// converge. Reproducing that exactly is what makes a value agree with a
/// workbook that already contains the formula.
pub(crate) fn rate(
    nper: f64,
    pmt_value: f64,
    pv_value: f64,
    fv_value: f64,
    kind: f64,
    guess: f64,
) -> Option<f64> {
    const EPS_MAX: f64 = 1e-10;
    const ITER_MAX: u32 = 100;

    let kind = if kind != 0.0 { 1.0 } else { 0.0 };
    let mut rate = guess;
    for _ in 0..ITER_MAX {
        if rate <= -1.0 {
            return None;
        }
        let (y, dy);
        if rate.abs() < EPS_MAX {
            y = pv_value * (1.0 + nper * rate) + pmt_value * (1.0 + rate * kind) * nper + fv_value;
            dy = pv_value * nper + pmt_value * kind * nper;
        } else {
            let f = (1.0 + rate).powf(nper);
            y = pv_value * f + pmt_value * (1.0 / rate + kind) * (f - 1.0) + fv_value;
            let df = nper * (1.0 + rate).powf(nper - 1.0);
            dy = pv_value * df
                + pmt_value * (1.0 / rate + kind) * df
                + pmt_value * (-1.0 / (rate * rate)) * (f - 1.0);
        }
        if y.abs() < EPS_MAX {
            return Some(rate);
        }
        rate -= y / dy;
    }
    None
}

/// `SUBTOTAL`: maps `function_num` to the aggregate it names.
///
/// The `100 +` forms mean "ignore hidden rows" in a real spreadsheet; nothing
/// here is ever hidden, so they are the same aggregate as the plain form. A
/// value that names no aggregate returns `None`, which the caller reports.
pub(crate) fn subtotal_index(function_num: f64) -> Option<u32> {
    if function_num.fract() != 0.0 {
        return None;
    }
    let base = if (1.0..=11.0).contains(&function_num) {
        function_num
    } else if (101.0..=111.0).contains(&function_num) {
        function_num - 100.0
    } else {
        return None;
    };
    Some(base as u32)
}

/// `DOLLARDE`: a fractional dollar price to a decimal one.
pub(crate) fn dollarde(fractional: f64, fraction: f64) -> Option<f64> {
    if fraction < 0.0 {
        return None;
    }
    let fraction = fraction.trunc();
    if fraction == 0.0 {
        return Some(fractional);
    }
    // The digits after the point are the numerator over the fraction.
    let denominator = 10f64.powf(fraction.log10().ceil());
    let whole = fractional.trunc();
    let numerator = (fractional - whole) * denominator;
    Some(whole + numerator / fraction)
}

/// `DOLLARFR`: a decimal dollar price back to a fractional one.
pub(crate) fn dollarfr(decimal: f64, fraction: f64) -> Option<f64> {
    if fraction < 0.0 {
        return None;
    }
    let fraction = fraction.trunc();
    if fraction == 0.0 {
        return Some(decimal);
    }
    let denominator = 10f64.powf(fraction.log10().ceil());
    let whole = decimal.trunc();
    Some(whole + (decimal - whole) * fraction / denominator)
}

/// `DISC`: the discount rate of a security.
pub(crate) fn disc(
    settlement: f64,
    maturity: f64,
    price: f64,
    redemption: f64,
    basis: i64,
) -> Option<f64> {
    let days = year_days_between(settlement, maturity, basis)?;
    if days <= 0.0 || price <= 0.0 || redemption <= 0.0 {
        return None;
    }
    Some((redemption - price) / redemption / days)
}

/// `PRICEDISC`: the price of a discounted security.
pub(crate) fn pricedisc(
    settlement: f64,
    maturity: f64,
    discount: f64,
    redemption: f64,
    basis: i64,
) -> Option<f64> {
    let days = year_days_between(settlement, maturity, basis)?;
    if discount <= 0.0 || redemption <= 0.0 {
        return None;
    }
    Some(redemption * (1.0 - discount * days))
}

/// `TBILLPRICE`: the price of a treasury bill per 100 face value.
///
/// The reference measures the term with the 30/360 day count, not the actual
/// number of days, so a bill spanning a 31-day month is priced as though that
/// month had 30 days.
pub(crate) fn tbillprice(settlement: f64, maturity: f64, discount: f64) -> Option<f64> {
    let days = tbill_term(settlement, maturity, discount)?;
    Some(100.0 * (1.0 - discount * days / 360.0))
}

/// `TBILLYIELD`: the yield of a treasury bill.
pub(crate) fn tbillyield(settlement: f64, maturity: f64, price: f64) -> Option<f64> {
    let days = tbill_term(settlement, maturity, price)?;
    Some((100.0 - price) * 360.0 / (price * days))
}

/// `TBILLEQ`: the bond-equivalent yield of a treasury bill.
pub(crate) fn tbilleq(settlement: f64, maturity: f64, discount: f64) -> Option<f64> {
    let days = tbill_term(settlement, maturity, discount)?;
    Some(365.0 * discount / (360.0 - discount * days))
}

/// `ACCRINT`: the accrued interest on a security that pays periodic interest.
///
/// The term runs from the issue date to the settlement date, and the interest
/// is the par value times the annual rate times that term as a fraction of a
/// year under the day-count basis. The first interest date does not enter the
/// formula - the reference accepts it and ignores it - but the argument is kept
/// so the call has the shape a spreadsheet gives it.
///
/// The basis is restricted to 0 through 4 and the frequency to 1, 2 or 4, which
/// is what the reference does, so an out-of-range value is an error rather than
/// a silently different day count. The reference also checks that the first
/// interest date parses, even though the value is unused, so a malformed one is
/// rejected here too.
pub(crate) fn accrint(
    issue: f64,
    first_interest: f64,
    settlement: f64,
    rate: f64,
    par: f64,
    frequency: f64,
    basis: f64,
) -> Option<f64> {
    if !issue.is_finite() || !first_interest.is_finite() || !settlement.is_finite() {
        return None;
    }
    if rate <= 0.0 || par <= 0.0 {
        return None;
    }
    if !matches!(frequency, 1.0 | 2.0 | 4.0) {
        return None;
    }
    if !(0.0..=4.0).contains(&basis) || basis.fract() != 0.0 {
        return None;
    }
    let (start, end) = (
        crate::datetime::date_part(issue),
        crate::datetime::date_part(settlement),
    );
    if end <= start {
        return None;
    }
    Some(par * rate * crate::datetime::yearfrac(start, end, basis as i64))
}

/// The 30/360 term of a treasury bill, rejecting a bill that runs longer than a
/// year or a non-positive rate.
fn tbill_term(settlement: f64, maturity: f64, rate: f64) -> Option<f64> {
    let (lo, hi) = (
        crate::datetime::date_part(settlement),
        crate::datetime::date_part(maturity),
    );
    if rate <= 0.0 || hi < lo || hi - lo > 365 {
        return None;
    }
    Some(crate::datetime::days360_us(lo, hi) as f64)
}

/// The fraction of a year between two dates under a day-count basis, which is
/// what the security functions measure their term with.
fn year_days_between(settlement: f64, maturity: f64, basis: i64) -> Option<f64> {
    let (lo, hi) = (
        crate::datetime::date_part(settlement),
        crate::datetime::date_part(maturity),
    );
    if hi < lo {
        return None;
    }
    let fraction = crate::datetime::yearfrac(lo, hi, basis);
    if fraction == 0.0 {
        None
    } else {
        Some(fraction)
    }
}

/// `COUPDAYS`: the days in the coupon period that contains `settlement`.
///
/// For every basis but actual/actual the answer is a fixed fraction of the
/// year, `360 / frequency` or `365 / frequency`. Basis 1 measures the actual
/// span between the coupon date before settlement and the next one.
pub(crate) fn coupdays(settlement: f64, maturity: f64, frequency: i64, basis: i64) -> Option<f64> {
    let start = crate::datetime::date_part(settlement);
    let end = crate::datetime::date_part(maturity);
    if start >= end {
        return None;
    }
    match basis {
        1 => {
            let previous = last_coupon_before(start, end, frequency)?;
            let months = 12 / frequency;
            let next = crate::datetime::edate(previous, months);
            Some((next - previous) as f64)
        }
        0 | 2 | 4 => Some(360.0 / frequency as f64),
        3 => Some(365.0 / frequency as f64),
        _ => None,
    }
}

/// The last coupon date at or before `settlement`.
///
/// The coupon dates are the maturity date carried back by whole coupon periods,
/// so the search starts from the maturity date's month and day and steps back.
fn last_coupon_before(settlement: i64, maturity: i64, frequency: i64) -> Option<i64> {
    if frequency != 1 && frequency != 2 && frequency != 4 {
        return None;
    }
    let (_, month, day) = crate::datetime::ymd_from_serial(maturity);
    let (year, _, _) = crate::datetime::ymd_from_serial(settlement);
    let months = 12 / frequency;
    // Try the coupon date in the settlement year, then step back one period at
    // a time until it is at or before the settlement date.
    let mut date = crate::datetime::date_serial(year as f64, month as f64, day as f64);
    while date > settlement {
        date = crate::datetime::edate(date, -months);
    }
    Some(date)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The reference values, taken from a differential run against
    /// `@formulajs/formulajs`.
    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * a.abs().max(b.abs()).max(1.0)
    }

    #[test]
    fn present_value_family_matches_the_reference() {
        assert!(close(
            fv(0.05, 10.0, -100.0, 0.0, 0.0).unwrap(),
            1_257.789_253_554_884
        ));
        assert!(close(
            pv(0.05, 10.0, -100.0, 0.0, 0.0).unwrap(),
            772.173_492_918_481_7
        ));
        assert!(close(
            pmt(0.05, 10.0, 1000.0, 0.0, 0.0).unwrap(),
            -129.504_574_965_456_6
        ));
        assert!(close(
            nper(0.05, -100.0, 1000.0, 0.0, 0.0).unwrap(),
            14.206_699_082_890_461
        ));
        // A zero rate is the plain sum, with no compounding.
        assert_eq!(fv(0.0, 10.0, -100.0, 0.0, 0.0), Some(1000.0));
        assert_eq!(pmt(0.0, 10.0, 1000.0, 0.0, 0.0), Some(-100.0));
    }

    #[test]
    fn payment_breaks_into_interest_and_principal() {
        assert!(close(
            ipmt(0.05, 1.0, 10.0, 1000.0, 0.0, 0.0).unwrap(),
            -50.0
        ));
        assert!(close(
            ppmt(0.05, 1.0, 10.0, 1000.0, 0.0, 0.0).unwrap(),
            -79.504_574_965_456_61
        ));
        // A period outside the schedule has no payment.
        assert_eq!(ipmt(0.05, 0.0, 10.0, 1000.0, 0.0, 0.0), None);
        assert_eq!(ipmt(0.05, 11.0, 10.0, 1000.0, 0.0, 0.0), None);
    }

    #[test]
    fn net_present_value_discounts_from_the_first_period() {
        // The reference discounts the first amount by one period, so the full
        // series including the outlay is what matches it.
        let flows = [-10000.0, 3000.0, 4200.0, 6800.0];
        assert!(close(npv(0.1, &flows).unwrap(), 1_188.443_412_335_221_6));
        // A rate of -100% is a division by zero, not a number.
        assert_eq!(npv(-1.0, &flows), None);
    }

    #[test]
    fn irr_finds_the_rate_that_zeroes_the_value() {
        let flows = [-1000.0, 100.0, 200.0, 300.0];
        let rate = CashFlows { amounts: &flows }.irr(0.1).unwrap();
        assert!(close(rate, -0.192_393_283_986_829_76));
        // A series that never changes sign has no rate.
        let all_positive = [1.0, 2.0, 3.0];
        assert_eq!(
            CashFlows {
                amounts: &all_positive
            }
            .irr(0.1),
            None
        );
    }

    #[test]
    fn depreciation_methods_agree_on_the_first_period() {
        assert_eq!(sln(1000.0, 100.0, 5.0), Some(180.0));
        assert_eq!(syd(1000.0, 100.0, 5.0, 1.0), Some(300.0));
        assert_eq!(ddb(1000.0, 100.0, 5.0, 1.0, 2.0), Some(400.0));
        // A life of zero has no depreciation to divide by.
        assert_eq!(sln(1000.0, 100.0, 0.0), None);
        assert_eq!(syd(1000.0, 100.0, 5.0, 6.0), None);
    }

    #[test]
    fn rates_convert_between_nominal_and_effective() {
        assert!(close(effect(0.05, 4.0).unwrap(), 0.050_945_336_914_062_22));
        assert!(close(nominal(0.05, 4.0).unwrap(), 0.049_088_937_716_157_41));
        assert_eq!(effect(0.0, 4.0), None);
        assert_eq!(nominal(0.05, 0.0), None);
    }

    #[test]
    fn growth_measures() {
        assert!(close(
            rri(10.0, 100.0, 200.0).unwrap(),
            0.071_773_462_536_293_13
        ));
        assert!(close(
            pduration(0.05, 1000.0, 2000.0).unwrap(),
            14.206_699_082_890_463
        ));
        assert_eq!(rri(0.0, 100.0, 200.0), None);
        assert_eq!(pduration(0.0, 100.0, 200.0), None);
    }

    #[test]
    fn fractional_dollar_prices_round_trip() {
        assert!((dollarde(1.02, 16.0).unwrap() - 1.125).abs() < 1e-12);
        assert!((dollarfr(1.125, 16.0).unwrap() - 1.02).abs() < 1e-12);
        // A fraction of zero leaves the price untouched.
        assert_eq!(dollarde(1.5, 0.0), Some(1.5));
        assert_eq!(dollarfr(1.5, 0.0), Some(1.5));
    }

    #[test]
    fn accrued_interest_matches_the_reference() {
        // 2020-01-15 to 2021-01-15 is exactly one year under 30/360, so the
        // interest is par times the rate. 43845 and 44211 are those serials.
        let (issue, first, settlement) = (43_845.0, 44_027.0, 44_211.0);
        assert!(close(
            accrint(issue, first, settlement, 0.05, 1000.0, 2.0, 0.0).unwrap(),
            50.0
        ));
        // Actual/360 over 366 days gives a little more than a year's interest.
        assert!(close(
            accrint(issue, first, settlement, 0.05, 1000.0, 2.0, 2.0).unwrap(),
            50.833_333_333_333_33
        ));
        // A rate, a par, a frequency or a basis out of range is refused, as is a
        // settlement that is not after the issue date.
        assert_eq!(
            accrint(issue, first, settlement, 0.0, 1000.0, 2.0, 0.0),
            None
        );
        assert_eq!(accrint(issue, first, settlement, 0.05, 0.0, 2.0, 0.0), None);
        assert_eq!(
            accrint(issue, first, settlement, 0.05, 1000.0, 3.0, 0.0),
            None
        );
        assert_eq!(
            accrint(issue, first, settlement, 0.05, 1000.0, 2.0, 5.0),
            None
        );
        assert_eq!(accrint(issue, first, issue, 0.05, 1000.0, 2.0, 0.0), None);
        // The first interest date is unused but still has to be a date.
        assert_eq!(
            accrint(issue, f64::NAN, settlement, 0.05, 1000.0, 2.0, 0.0),
            None
        );
    }

    #[test]
    fn treasury_bill_functions_agree_with_each_other() {
        let (settlement, maturity) = (45_306.0, 45_671.0);
        // The term is 365 days of actual time but 360 under the 30/360 count
        // the reference uses.
        let price = tbillprice(settlement, maturity, 0.05).unwrap();
        assert!(close(price, 95.013_888_888_888_89));
        let yield_ = tbillyield(settlement, maturity, price).unwrap();
        assert!(yield_ > 0.0);
        assert!(tbilleq(settlement, maturity, 0.05).is_some());
        // A term beyond a year is not a treasury bill.
        assert_eq!(tbillprice(settlement, 46_000.0, 0.05), None);
        // Nor is a non-positive rate.
        assert_eq!(tbillprice(settlement, maturity, 0.0), None);
    }
}
