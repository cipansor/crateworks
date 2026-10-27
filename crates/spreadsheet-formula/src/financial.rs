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
