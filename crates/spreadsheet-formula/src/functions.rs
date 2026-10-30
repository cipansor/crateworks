//! The built-in function library.
//!
//! Every function receives flattened arguments: a scalar argument becomes a
//! single value and a range becomes one value per cell, in row-major order.
//! An [`Arg`] also records whether it came from a reference, because a
//! spreadsheet treats the two differently: text, logicals and blanks in a cell
//! are ignored by `SUM(A1:A3)`, while the same values written directly are
//! coerced, so `SUM(TRUE)` is 1 but `SUM(A1)` is 0 when `A1` holds `TRUE`.
//!
//! `IF` and `IFERROR` are not here, because they must not evaluate every
//! argument; the evaluator handles them directly.

use crate::Error;
use crate::bessel;
use crate::complex::Complex;
use crate::database;
use crate::datetime;
use crate::financial;
use crate::lookup::{self, Axis, MatchMode};
use crate::stats;
use crate::value::{Error as ValueError, Value};

/// A flattened argument, and whether it came from a cell or range reference.
#[derive(Debug, Clone)]
pub(crate) struct Arg {
    values: Vec<Value>,
    from_ref: bool,
    /// The number of columns, when the argument is a rectangular range.
    ///
    /// The flat `values` are row-major, so the width is all a lookup needs to
    /// recover the shape. A scalar or a one-dimensional argument has no shape,
    /// which is what `None` records.
    width: Option<usize>,
    /// The cell the argument started at, as `(row, column)`.
    ///
    /// `ROW` and `COLUMN` report it, so a reference has to remember where it
    /// came from and not only what it holds.
    origin: Option<(u32, u32)>,
}

impl Arg {
    /// An argument written directly in the formula.
    pub(crate) fn scalar(value: Value) -> Self {
        Self {
            values: vec![value],
            from_ref: false,
            width: None,
            origin: None,
        }
    }

    /// An argument read from cells.
    pub(crate) fn reference(values: Vec<Value>) -> Self {
        Self {
            values,
            from_ref: true,
            width: None,
            origin: None,
        }
    }

    /// A rectangular range, given row-major and with its width.
    pub(crate) fn range(values: Vec<Value>, width: usize, origin: (u32, u32)) -> Self {
        Self {
            values,
            from_ref: true,
            width: Some(width),
            origin: Some(origin),
        }
    }

    /// The same argument, tagged with the cell it was read from.
    pub(crate) fn with_origin(mut self, origin: (u32, u32)) -> Self {
        self.origin = Some(origin);
        self
    }

    /// The shape of a rectangular range, as `(rows, columns)`.
    ///
    /// Returns `None` for anything that is not a range, which is how a lookup
    /// function knows to reject a scalar argument.
    fn shape(&self) -> Option<(usize, usize)> {
        let width = self.width?;
        if width == 0 {
            return None;
        }
        Some((self.values.len().div_ceil(width), width))
    }

    /// The numbers this argument contributes to an aggregating function.
    ///
    /// A reference contributes only its numeric cells. A literal is coerced -
    /// `TRUE` is 1, `"3"` is 3 - and a literal that is not a number is an
    /// error, so `SUM("x")` reports a type error rather than ignoring it.
    fn aggregate_numbers(&self, name: &str) -> Result<Vec<f64>, Error> {
        let mut out = Vec::with_capacity(self.values.len());
        for value in &self.values {
            match value {
                Value::Number(n) => out.push(*n),
                _ if self.from_ref => {}
                Value::Empty => out.push(0.0),
                Value::Bool(b) => out.push(if *b { 1.0 } else { 0.0 }),
                Value::Text(t) => match t.trim().parse::<f64>() {
                    Ok(n) => out.push(n),
                    Err(_) => {
                        return Err(Error::TypeError {
                            operation: name.to_string(),
                            found: value.clone(),
                        });
                    }
                },
            }
        }
        Ok(out)
    }

    /// The values `COUNT` counts: numbers, plus logicals and numeric text when
    /// they are written directly. A reference contributes only its numbers.
    fn countable(&self) -> usize {
        self.values
            .iter()
            .filter(|v| {
                if self.from_ref {
                    matches!(v, Value::Number(_))
                } else {
                    !v.is_empty() && v.as_number().is_ok()
                }
            })
            .count()
    }
}

/// Maps a function name to the single spelling the dispatcher matches on.
///
/// A spreadsheet writes the modern names with dots - `NORM.S.DIST`,
/// `CHISQ.INV.RT` - and the older flat names without them - `NORMSDIST`,
/// `CHIINVRT`. Both mean the same function, so the dots are removed and the
/// result is the flat alias the arms below are written against. The reference
/// exposes most modern names as nested namespaces, which the oracle walks, so
/// both spellings are checked against it.
fn canonical_name(name: &str) -> String {
    name.replace('.', "")
}

/// Collects the numbers from every argument of an aggregating function.
fn numbers_of(name: &str, args: &[Arg]) -> Result<Vec<f64>, Error> {
    let mut out = Vec::new();
    for arg in args {
        out.extend(arg.aggregate_numbers(name)?);
    }
    Ok(out)
}

/// Calls a function by name.
///
/// This is [`call`], with the cell whose formula is being evaluated: `ROW` and
/// `COLUMN` report that cell when they are given no argument, so the evaluator
/// has to pass it down.
pub(crate) fn call_from(
    name: &str,
    args: &[Arg],
    origin: Option<(u32, u32)>,
) -> Result<Value, Error> {
    let canonical = canonical_name(name);
    let name = canonical.as_str();
    match name {
        "SUM" => {
            require_at_least(name, args, 1)?;
            Ok(Value::Number(numbers_of(name, args)?.iter().sum()))
        }
        "PRODUCT" => {
            require_at_least(name, args, 1)?;
            // The product of no numbers is 1, which is what Excel returns for
            // `PRODUCT` over a range with no numeric cells.
            Ok(Value::Number(
                numbers_of(name, args)?.iter().product::<f64>(),
            ))
        }
        "AVERAGE" => {
            require_at_least(name, args, 1)?;
            let values = numbers_of(name, args)?;
            if values.is_empty() {
                return Err(Error::Arithmetic(ValueError::DivisionByZero));
            }
            let count = values.len() as f64;
            Ok(Value::Number(values.iter().sum::<f64>() / count))
        }
        "MEDIAN" => {
            require_at_least(name, args, 1)?;
            let mut values = numbers_of(name, args)?;
            if values.is_empty() {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            values.sort_by(f64::total_cmp);
            let mid = values.len() / 2;
            let median = if values.len().is_multiple_of(2) {
                f64::midpoint(values[mid - 1], values[mid])
            } else {
                values[mid]
            };
            Ok(Value::Number(median))
        }
        "MIN" => {
            require_at_least(name, args, 1)?;
            Ok(min_max(name, args, true)?)
        }
        "MAX" => {
            require_at_least(name, args, 1)?;
            Ok(min_max(name, args, false)?)
        }
        "COUNT" => {
            require_at_least(name, args, 1)?;
            let n: usize = args.iter().map(Arg::countable).sum();
            Ok(Value::Number(n as f64))
        }
        "COUNTA" => {
            require_at_least(name, args, 1)?;
            let n = args
                .iter()
                .flat_map(|a| a.values.iter())
                .filter(|v| !v.is_empty())
                .count();
            Ok(Value::Number(n as f64))
        }
        // `SUBTOTAL` runs one of eleven aggregates; the `100 +` forms mean the
        // same thing here, because nothing in this evaluator is ever hidden.
        "SUBTOTAL" => {
            require_at_least(name, args, 2)?;
            let selector = first_number(name, &args[0])?;
            let rest = &args[1..];
            match financial::subtotal_index(selector) {
                Some(1) => {
                    let values = numbers_of(name, rest)?;
                    if values.is_empty() {
                        return Err(Error::Arithmetic(ValueError::DivisionByZero));
                    }
                    Ok(Value::Number(stats::mean(&values)))
                }
                Some(2) => Ok(Value::Number(
                    rest.iter().map(Arg::countable).sum::<usize>() as f64,
                )),
                Some(3) => Ok(Value::Number(
                    rest.iter()
                        .flat_map(|a| a.values.iter())
                        .filter(|v| !v.is_empty())
                        .count() as f64,
                )),
                Some(4) | Some(5) => min_max(name, rest, matches!(selector.trunc(), 5.0)),
                Some(6) => Ok(Value::Number(
                    numbers_of(name, rest)?.iter().product::<f64>(),
                )),
                Some(7) => {
                    let values = numbers_of(name, rest)?;
                    Ok(Value::Number(sample_variance(&values).sqrt()))
                }
                Some(8) => {
                    let values = numbers_of(name, rest)?;
                    Ok(Value::Number(population_variance(&values).sqrt()))
                }
                Some(9) => Ok(Value::Number(numbers_of(name, rest)?.iter().sum())),
                Some(10) => {
                    let values = numbers_of(name, rest)?;
                    Ok(Value::Number(sample_variance(&values)))
                }
                Some(11) => {
                    let values = numbers_of(name, rest)?;
                    Ok(Value::Number(population_variance(&values)))
                }
                _ => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        // `AGGREGATE` runs one of nineteen aggregates; the `options` argument
        // selects what to skip, but nothing in this evaluator is ever hidden or
        // an error value, so it only has to be accepted.
        "AGGREGATE" => {
            require_between(name, args, 3, 4)?;
            let selector = first_number(name, &args[0])?;
            if selector.fract() != 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let selector = selector as u32;
            let first = &args[2];
            let second = args.get(3);
            let values = sorted_values(first);
            match selector {
                1..=11 => {
                    let rest = &args[2..];
                    match selector {
                        1 => {
                            let values = numbers_of(name, rest)?;
                            if values.is_empty() {
                                return Err(Error::Arithmetic(ValueError::DivisionByZero));
                            }
                            Ok(Value::Number(stats::mean(&values)))
                        }
                        2 => Ok(Value::Number(
                            rest.iter().map(Arg::countable).sum::<usize>() as f64,
                        )),
                        3 => Ok(Value::Number(
                            rest.iter()
                                .flat_map(|a| a.values.iter())
                                .filter(|v| !v.is_empty())
                                .count() as f64,
                        )),
                        4 => min_max(name, rest, false),
                        5 => min_max(name, rest, true),
                        6 => Ok(Value::Number(
                            numbers_of(name, rest)?.iter().product::<f64>(),
                        )),
                        7 => {
                            let values = numbers_of(name, rest)?;
                            Ok(Value::Number(sample_variance(&values).sqrt()))
                        }
                        8 => {
                            let values = numbers_of(name, rest)?;
                            Ok(Value::Number(population_variance(&values).sqrt()))
                        }
                        9 => Ok(Value::Number(numbers_of(name, rest)?.iter().sum())),
                        10 => {
                            let values = numbers_of(name, rest)?;
                            Ok(Value::Number(sample_variance(&values)))
                        }
                        _ => {
                            let values = numbers_of(name, rest)?;
                            Ok(Value::Number(population_variance(&values)))
                        }
                    }
                }
                12 => {
                    if values.is_empty() {
                        return Err(Error::Arithmetic(ValueError::NotANumber));
                    }
                    let mid = values.len() / 2;
                    let median = if values.len().is_multiple_of(2) {
                        f64::midpoint(values[mid - 1], values[mid])
                    } else {
                        values[mid]
                    };
                    Ok(Value::Number(median))
                }
                13 => {
                    let values = numbers_of(name, &args[2..3])?;
                    Ok(Value::Number(mode_single(&values)))
                }
                14 | 15 => {
                    let k = first_number(
                        name,
                        second.ok_or(Error::WrongArity {
                            name: name.to_string(),
                            given: args.len(),
                            expected: "4",
                        })?,
                    )?;
                    Ok(Value::Number(rank_value(selector == 14, &values, k)?))
                }
                16 | 18 => {
                    let k = first_number(
                        name,
                        second.ok_or(Error::WrongArity {
                            name: name.to_string(),
                            given: args.len(),
                            expected: "4",
                        })?,
                    )?;
                    Ok(Value::Number(percentile_value(selector == 18, &values, k)?))
                }
                17 | 19 => {
                    let k = first_number(
                        name,
                        second.ok_or(Error::WrongArity {
                            name: name.to_string(),
                            given: args.len(),
                            expected: "4",
                        })?,
                    )?;
                    Ok(Value::Number(quartile_value(selector == 19, &values, k)?))
                }
                _ => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        "ABS" => unary_number(name, args, f64::abs),
        "SIGN" => unary_number(name, args, |n| {
            if n > 0.0 {
                1.0
            } else if n < 0.0 {
                -1.0
            } else {
                0.0
            }
        }),
        "SQRT" => unary_number(name, args, f64::sqrt),
        "INT" => unary_number(name, args, f64::floor),
        "SIN" => unary_number(name, args, f64::sin),
        "COS" => unary_number(name, args, f64::cos),
        "TAN" => unary_number(name, args, f64::tan),
        "ASIN" => unary_number(name, args, f64::asin),
        "ACOS" => unary_number(name, args, f64::acos),
        "ATAN" => unary_number(name, args, f64::atan),
        "SINH" => unary_number(name, args, f64::sinh),
        "COSH" => unary_number(name, args, f64::cosh),
        "TANH" => unary_number(name, args, f64::tanh),
        "ASINH" => unary_number(name, args, f64::asinh),
        "ACOSH" => unary_number(name, args, f64::acosh),
        "ATANH" => unary_number(name, args, f64::atanh),
        "DEGREES" => unary_number(name, args, f64::to_degrees),
        "RADIANS" => unary_number(name, args, f64::to_radians),
        "SQRTPI" => unary_number(name, args, |n| (n * std::f64::consts::PI).sqrt()),
        // The reciprocal and hyperbolic-reciprocal family. Each is the
        // reciprocal of the corresponding base function, and the reference
        // reports a division by zero where the reciprocal is undefined.
        "CSC" => unary_number(
            name,
            args,
            |n| {
                if n == 0.0 { f64::NAN } else { 1.0 / n.sin() }
            },
        ),
        "SEC" => unary_number(name, args, |n| 1.0 / n.cos()),
        "COT" => unary_number(
            name,
            args,
            |n| {
                if n == 0.0 { f64::NAN } else { 1.0 / n.tan() }
            },
        ),
        "CSCH" => unary_number(name, args, |n| {
            if n == 0.0 {
                f64::NAN
            } else {
                2.0 / (n.exp() - (-n).exp())
            }
        }),
        "SECH" => unary_number(name, args, |n| 2.0 / (n.exp() + (-n).exp())),
        "COTH" => unary_number(name, args, |n| {
            if n == 0.0 {
                return f64::NAN;
            }
            let e2 = (2.0 * n).exp();
            (e2 + 1.0) / (e2 - 1.0)
        }),
        // `ACOT` is `atan(1/x)`, which differs from `atan(x).recip()` in
        // quadrant: the reference uses the former, so `ACOT(-1)` is -π/4.
        "ACOT" => unary_number(name, args, |n| (1.0 / n).atan()),
        "ACOTH" => unary_number(name, args, |n| 0.5 * ((n + 1.0) / (n - 1.0)).ln()),
        "ATAN2" => {
            require_arity(name, args, 2)?;
            let x = first_number(name, &args[0])?;
            let y = first_number(name, &args[1])?;
            Ok(Value::Number(x.atan2(y)))
        }
        "GAMMA" => unary_number(name, args, |n| {
            // The reference rejects zero and negative integers, where the
            // gamma function has a pole, rather than returning an infinity.
            if n == 0.0 || (n < 0.0 && n.fract() == 0.0) {
                f64::NAN
            } else {
                gamma_fn(n)
            }
        }),
        "GAMMALN" | "GAMMALNPRECISE" => unary_number(name, args, gamma_ln),
        // --- Bessel functions ------------------------------------------------
        // The order argument is truncated, which the reference does inside each
        // implementation; the wrapper here leaves it to `bessel`.
        "BESSELI" | "BESSELJ" | "BESSELK" | "BESSELY" => {
            require_arity(name, args, 2)?;
            let x = first_number(name, &args[0])?;
            let n = first_number(name, &args[1])?;
            Ok(Value::Number(match name {
                "BESSELI" => bessel::besseli(x, n),
                "BESSELJ" => bessel::besselj(x, n),
                "BESSELK" => bessel::besselk(x, n),
                _ => bessel::bessely(x, n),
            }))
        }
        "ERF" => {
            require_between(name, args, 1, 2)?;
            Ok(Value::Number(erf(first_number(name, &args[0])?)))
        }
        "ERFC" => unary_number(name, args, |n| 1.0 - erf(n)),
        "ROUND" => {
            let (value, digits) = number_and_digits(name, args)?;
            Ok(Value::Number(round_to(value, digits, RoundMode::Half)))
        }
        "ROUNDUP" => {
            let (value, digits) = number_and_digits(name, args)?;
            Ok(Value::Number(round_to(value, digits, RoundMode::Up)))
        }
        "ROUNDDOWN" => {
            let (value, digits) = number_and_digits(name, args)?;
            Ok(Value::Number(round_to(value, digits, RoundMode::Down)))
        }
        "MOD" => {
            require_arity(name, args, 2)?;
            let a = first_number(name, &args[0])?;
            let b = first_number(name, &args[1])?;
            if b == 0.0 {
                return Err(Error::Arithmetic(crate::value::Error::DivisionByZero));
            }
            // Spreadsheet MOD follows the sign of the divisor.
            Ok(Value::Number(a - b * (a / b).floor()))
        }
        "QUOTIENT" => {
            require_arity(name, args, 2)?;
            let a = first_number(name, &args[0])?;
            let b = first_number(name, &args[1])?;
            if b == 0.0 {
                return Err(Error::Arithmetic(crate::value::Error::DivisionByZero));
            }
            // The integer part of the division, truncated towards zero.
            Ok(Value::Number((a / b).trunc()))
        }
        "CEILING" => {
            require_arity(name, args, 2)?;
            let value = first_number(name, &args[0])?;
            let significance = first_number(name, &args[1])?;
            // A positive value with a negative significance has no solution, and
            // a zero significance divides by zero. Both are errors rather than
            // a silent guess.
            if significance == 0.0 {
                return Err(Error::Arithmetic(crate::value::Error::DivisionByZero));
            }
            if value > 0.0 && significance < 0.0 {
                return Err(Error::Arithmetic(crate::value::Error::NotANumber));
            }
            Ok(Value::Number((value / significance).ceil() * significance))
        }
        "CEILINGMATH" | "CEILINGPRECISE" | "ISOCEILING" => {
            require_between(name, args, 1, 3)?;
            let value = first_number(name, &args[0])?;
            let significance = match args.get(1) {
                Some(arg) => first_number(name, arg)?,
                None => 1.0,
            };
            let mode = match args.get(2) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            Ok(Value::Number(round_multiple(
                value,
                significance,
                mode,
                true,
            )))
        }
        "FLOOR" => {
            require_arity(name, args, 2)?;
            let value = first_number(name, &args[0])?;
            let significance = first_number(name, &args[1])?;
            if significance == 0.0 {
                return Err(Error::Arithmetic(crate::value::Error::DivisionByZero));
            }
            if value > 0.0 && significance < 0.0 {
                return Err(Error::Arithmetic(crate::value::Error::NotANumber));
            }
            Ok(Value::Number((value / significance).floor() * significance))
        }
        "FLOORMATH" | "FLOORPRECISE" => {
            require_between(name, args, 1, 3)?;
            let value = first_number(name, &args[0])?;
            let significance = match args.get(1) {
                Some(arg) => first_number(name, arg)?,
                None => 1.0,
            };
            let mode = match args.get(2) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            Ok(Value::Number(round_multiple(
                value,
                significance,
                mode,
                false,
            )))
        }
        "TRUNC" => {
            require_between(name, args, 1, 2)?;
            let value = first_number(name, &args[0])?;
            let digits = match args.get(1) {
                Some(arg) => first_number(name, arg)? as i32,
                None => 0,
            };
            Ok(Value::Number(round_to(value, digits, RoundMode::Down)))
        }
        "MROUND" => {
            require_arity(name, args, 2)?;
            let value = first_number(name, &args[0])?;
            let multiple = first_number(name, &args[1])?;
            if multiple == 0.0 {
                return Ok(Value::Number(0.0));
            }
            // The value and the multiple must share a sign, otherwise there is
            // no nearest multiple to return.
            if (value > 0.0) != (multiple > 0.0) {
                return Err(Error::Arithmetic(crate::value::Error::NotANumber));
            }
            Ok(Value::Number((value / multiple).round() * multiple))
        }
        "EVEN" => unary_number(name, args, |n| {
            // Away from zero, to the next even integer.
            let rounded = n.abs().ceil();
            let even = if rounded % 2.0 == 0.0 {
                rounded
            } else {
                rounded + 1.0
            };
            n.signum() * even
        }),
        "ODD" => unary_number(name, args, |n| {
            // ODD(0) is 1, not 0: the next odd integer away from zero.
            let rounded = n.abs().ceil().max(1.0);
            let odd = if rounded % 2.0 == 1.0 {
                rounded
            } else {
                rounded + 1.0
            };
            if n < 0.0 { -odd } else { odd }
        }),
        "SUMSQ" => {
            require_at_least(name, args, 1)?;
            Ok(Value::Number(
                numbers_of(name, args)?.iter().map(|n| n * n).sum(),
            ))
        }
        "SUMPRODUCT" => {
            require_at_least(name, args, 1)?;
            // Every argument must be the same length; the products are summed
            // position by position. A shorter argument would leave trailing
            // terms undefined, so it is rejected.
            let len = args[0].values.len();
            for arg in args {
                if arg.values.len() != len {
                    return Err(Error::TypeError {
                        operation: name.to_string(),
                        found: Value::Number(arg.values.len() as f64),
                    });
                }
            }
            let mut total = 0.0;
            for index in 0..len {
                let mut product = 1.0;
                for arg in args {
                    product *= arg.values[index].as_number().unwrap_or(0.0);
                }
                total += product;
            }
            Ok(Value::Number(total))
        }
        "GCD" => integer_list(name, args, |a, b| {
            let (mut a, mut b) = (a.abs(), b.abs());
            while b != 0 {
                (a, b) = (b, a % b);
            }
            a
        }),
        "LCM" => integer_list(name, args, |a, b| {
            if a == 0 || b == 0 {
                return 0;
            }
            (a / gcd_i64(a, b) * b).abs()
        }),
        "FACT" => {
            require_arity(name, args, 1)?;
            let n = first_number(name, &args[0])?;
            if n < 0.0 {
                return Err(Error::Arithmetic(crate::value::Error::NotANumber));
            }
            // Truncate to an integer, then multiply out. 171 is the largest
            // argument whose factorial is still finite in an f64.
            let n = n.trunc() as u32;
            let mut out = 1.0;
            for i in 2..=n {
                out *= f64::from(i);
            }
            Ok(Value::Number(out))
        }
        "EXP" => unary_number(name, args, f64::exp),
        "LN" => unary_number(name, args, |n| if n <= 0.0 { f64::NAN } else { n.ln() }),
        "LOG10" => unary_number(name, args, |n| if n <= 0.0 { f64::NAN } else { n.log10() }),
        "LOG" => {
            require_between(name, args, 1, 2)?;
            let value = first_number(name, &args[0])?;
            let base = match args.get(1) {
                Some(arg) => first_number(name, arg)?,
                None => 10.0,
            };
            if value <= 0.0 || base <= 0.0 {
                return Ok(Value::Number(f64::NAN));
            }
            Ok(Value::Number(value.log(base)))
        }
        "POWER" => {
            require_arity(name, args, 2)?;
            let a = first_number(name, &args[0])?;
            let b = first_number(name, &args[1])?;
            Ok(Value::Number(a.powf(b)))
        }
        "PI" => {
            require_arity(name, args, 0)?;
            Ok(Value::Number(std::f64::consts::PI))
        }
        "COMBIN" => {
            require_arity(name, args, 2)?;
            let n = first_number(name, &args[0])?;
            let k = first_number(name, &args[1])?;
            // Choosing more than there are is undefined, not zero.
            if n < k {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(binomial(n, k)))
        }
        "COMBINA" => {
            require_arity(name, args, 2)?;
            let n = first_number(name, &args[0])?;
            let k = first_number(name, &args[1])?;
            if n < k {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            // Stars and bars: zero of each is one way, otherwise it is `COMBIN`
            // with the pool raised by one less than the number chosen.
            if n == 0.0 && k == 0.0 {
                return Ok(Value::Number(1.0));
            }
            Ok(Value::Number(binomial(n + k - 1.0, n - 1.0)))
        }
        "PERMUT" => {
            require_arity(name, args, 2)?;
            let n = first_number(name, &args[0])?;
            let k = first_number(name, &args[1])?;
            if n < 0.0 || k < 0.0 || n < k {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(factorial(n) / factorial(n - k)))
        }
        "PERMUTATIONA" => {
            require_arity(name, args, 2)?;
            let n = first_number(name, &args[0])?;
            let k = first_number(name, &args[1])?;
            Ok(Value::Number(n.powf(k)))
        }
        "MULTINOMIAL" => {
            require_at_least(name, args, 1)?;
            let numbers = numbers_of(name, args)?;
            let total: f64 = numbers.iter().sum();
            let divisor: f64 = numbers.iter().map(|n| factorial(*n)).product();
            Ok(Value::Number(factorial(total) / divisor))
        }
        "FACTDOUBLE" => unary_number(name, args, |n| {
            // The product of every second integer down to one; zero and any
            // negative input are 1, matching the reference.
            let n = n.floor();
            if n <= 0.0 {
                return 1.0;
            }
            let mut out = 1.0;
            let mut i = n;
            while i > 0.0 {
                out *= i;
                i -= 2.0;
            }
            out
        }),
        // --- descriptive statistics -----------------------------------------
        "AVEDEV" => {
            let values = numbers_of(name, args)?;
            if values.is_empty() {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            let total: f64 = values.iter().map(|v| (v - mean).abs()).sum();
            Ok(Value::Number(total / values.len() as f64))
        }
        "DEVSQ" => {
            let values = numbers_of(name, args)?;
            let mean = values.iter().sum::<f64>() / values.len() as f64;
            Ok(Value::Number(
                values.iter().map(|v| (v - mean).powi(2)).sum(),
            ))
        }
        "GEOMEAN" => {
            let values = numbers_of(name, args)?;
            if values.iter().any(|v| *v <= 0.0) {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let log_sum: f64 = values.iter().map(|v| v.ln()).sum();
            Ok(Value::Number((log_sum / values.len() as f64).exp()))
        }
        "HARMEAN" => {
            let values = numbers_of(name, args)?;
            if values.iter().any(|v| *v <= 0.0) {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let denominator: f64 = values.iter().map(|v| 1.0 / v).sum();
            Ok(Value::Number(values.len() as f64 / denominator))
        }
        "VARS" | "VAR" => {
            let values = numbers_of(name, args)?;
            Ok(Value::Number(sample_variance(&values)))
        }
        "VARP" => {
            let values = numbers_of(name, args)?;
            Ok(Value::Number(population_variance(&values)))
        }
        "STDEVS" | "STDEV" => {
            let values = numbers_of(name, args)?;
            Ok(Value::Number(sample_variance(&values).sqrt()))
        }
        "STDEVP" => {
            let values = numbers_of(name, args)?;
            Ok(Value::Number(population_variance(&values).sqrt()))
        }
        "KURT" => {
            let values = numbers_of(name, args)?;
            Ok(Value::Number(kurtosis(&values)))
        }
        "SKEW" => {
            let values = numbers_of(name, args)?;
            Ok(Value::Number(skewness(&values, true)))
        }
        "SKEWP" => {
            let values = numbers_of(name, args)?;
            Ok(Value::Number(skewness(&values, false)))
        }
        "LARGE" | "SMALL" => {
            require_arity(name, args, 2)?;
            let k = first_number(name, &args[1])?;
            let values = sorted_values(&args[0]);
            Ok(Value::Number(rank_value(name == "LARGE", &values, k)?))
        }
        "MODE" | "MODESNGL" => {
            let values = numbers_of(name, args)?;
            Ok(Value::Number(mode_single(&values)))
        }
        "TRIMMEAN" => {
            require_arity(name, args, 2)?;
            let percent = first_number(name, &args[1])?;
            let mut values = args[0]
                .values
                .iter()
                .filter_map(|v| v.as_number().ok())
                .collect::<Vec<f64>>();
            if values.is_empty() {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            // The number trimmed from each end is half the count, floored to an
            // even number, which is what keeps the two ends equal.
            let trim = ((values.len() as f64 * percent).floor() / 2.0) as usize;
            let kept = &values[trim..values.len() - trim];
            let mean = kept.iter().sum::<f64>() / kept.len() as f64;
            Ok(Value::Number(mean))
        }
        "PERCENTILE" | "PERCENTILEINC" | "PERCENTILEEXC" => {
            require_arity(name, args, 2)?;
            let k = first_number(name, &args[1])?;
            let values = sorted_values(&args[0]);
            Ok(Value::Number(percentile_value(
                name == "PERCENTILEEXC",
                &values,
                k,
            )?))
        }
        "QUARTILE" | "QUARTILEINC" | "QUARTILEEXC" => {
            require_arity(name, args, 2)?;
            let quart = first_number(name, &args[1])?;
            let values = sorted_values(&args[0]);
            Ok(Value::Number(quartile_value(
                name == "QUARTILEEXC",
                &values,
                quart,
            )?))
        }
        "RANK" | "RANKEQ" | "RANKAVG" => {
            require_between(name, args, 2, 3)?;
            let number = first_number(name, &args[0])?;
            let ascending = match args.get(2) {
                Some(arg) => first_number(name, arg)? != 0.0,
                None => false,
            };
            let mut values = args[1]
                .values
                .iter()
                .filter_map(|v| v.as_number().ok())
                .collect::<Vec<f64>>();
            values.sort_by(|a, b| {
                let order = a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal);
                if ascending { order } else { order.reverse() }
            });
            let Some(position) = values.iter().position(|v| *v == number) else {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            };
            let tied = values.iter().filter(|v| **v == number).count();
            // The average rank splits the difference across a run of equals;
            // `RANKEQ` returns the first of the run.
            let rank = if name == "RANKAVG" && tied > 1 {
                (2.0 * position as f64 + tied as f64 + 1.0) / 2.0
            } else {
                position as f64 + 1.0
            };
            Ok(Value::Number(rank))
        }
        "SERIESSUM" => {
            require_arity(name, args, 4)?;
            let x = first_number(name, &args[0])?;
            let n = first_number(name, &args[1])?;
            let m = first_number(name, &args[2])?;
            let coefficients: Vec<f64> = args[3]
                .values
                .iter()
                .filter_map(|v| v.as_number().ok())
                .collect();
            // a0*x^n + a1*x^(n+m) + a2*x^(n+2m) + ...
            let total: f64 = coefficients
                .iter()
                .enumerate()
                .map(|(i, a)| a * x.powf(n + (i as f64) * m))
                .sum();
            Ok(Value::Number(total))
        }
        "SUMX2MY2" | "SUMX2PY2" | "SUMXMY2" => {
            require_arity(name, args, 2)?;
            let xs: Vec<f64> = args[0]
                .values
                .iter()
                .filter_map(|v| v.as_number().ok())
                .collect();
            let ys: Vec<f64> = args[1]
                .values
                .iter()
                .filter_map(|v| v.as_number().ok())
                .collect();
            if xs.len() != ys.len() {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let total: f64 = xs
                .iter()
                .zip(&ys)
                .map(|(x, y)| match name {
                    "SUMX2MY2" => x.powi(2) - y.powi(2),
                    "SUMX2PY2" => x.powi(2) + y.powi(2),
                    _ => (x - y).powi(2),
                })
                .sum();
            Ok(Value::Number(total))
        }
        // `AVERAGEA`, `MAXA` and `MINA` differ from their plain forms in how
        // they read a cell: text counts as zero and a logical as one or zero,
        // rather than being skipped.
        "AVERAGEA" => {
            let values = values_of_a(name, args);
            if values.is_empty() {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let total: f64 = values.iter().sum();
            Ok(Value::Number(total / values.len() as f64))
        }
        "MAXA" => {
            let values = values_of_a(name, args);
            Ok(Value::Number(
                values.iter().fold(f64::NEG_INFINITY, |a, b| a.max(*b)),
            ))
        }
        "MINA" => {
            let values = values_of_a(name, args);
            Ok(Value::Number(
                values.iter().fold(f64::INFINITY, |a, b| a.min(*b)),
            ))
        }
        // --- database --------------------------------------------------------
        // The `D...` family aggregates one field of a database over the records
        // a criteria range selects. `database` and `criteria` are grids, so they
        // need their width to be read as rows and columns.
        "DSUM" | "DAVERAGE" | "DCOUNT" | "DCOUNTA" | "DGET" | "DMAX" | "DMIN" | "DPRODUCT"
        | "DSTDEV" | "DSTDEVP" | "DVAR" | "DVARP" => {
            require_arity(name, args, 3)?;
            let aggregate = database::Aggregate::for_name(name)
                .expect("the match arm names a database function");
            let (database_values, width) = database_grid(name, &args[0])?;
            let (criteria_values, criteria_width) = database_grid(name, &args[2])?;
            database::evaluate(
                aggregate,
                &database_values,
                width,
                &first_value(&args[1]),
                &criteria_values,
                criteria_width,
            )
        }
        "COUNTIF" => {
            require_arity(name, args, 2)?;
            let matched = args[0]
                .values
                .iter()
                .filter(|v| criterion(v, &args[1]))
                .count();
            Ok(Value::Number(matched as f64))
        }
        "SUMIF" => {
            require_between(name, args, 2, 3)?;
            let values = matching_values(args);
            Ok(Value::Number(values.iter().sum()))
        }
        "AVERAGEIF" => {
            require_between(name, args, 2, 3)?;
            let values = matching_values(args);
            if values.is_empty() {
                return Ok(Value::Empty);
            }
            let count = values.len() as f64;
            Ok(Value::Number(values.iter().sum::<f64>() / count))
        }
        "COUNTBLANK" => {
            require_arity(name, args, 1)?;
            let blanks = args[0]
                .values
                .iter()
                .filter(|v| v.is_empty() || v.to_string().trim().is_empty())
                .count();
            Ok(Value::Number(blanks as f64))
        }
        "COUNTIFS" => {
            require_at_least(name, args, 2)?;
            if !args.len().is_multiple_of(2) {
                return Err(Error::WrongArity {
                    name: name.to_string(),
                    given: args.len(),
                    expected: "an even number of arguments",
                });
            }
            let len = args[0].values.len();
            let count = (0..len)
                .filter(|&index| every_criterion_matches(args, index))
                .count();
            Ok(Value::Number(count as f64))
        }
        "SUMIFS" => {
            require_at_least(name, args, 3)?;
            // The range to add is the first argument; the criteria ranges and
            // their criteria follow in pairs, which is the opposite order from
            // `SUMIF`.
            if args.len() % 2 != 1 {
                return Err(Error::WrongArity {
                    name: name.to_string(),
                    given: args.len(),
                    expected: "a sum range followed by criteria pairs",
                });
            }
            let len = args[0].values.len();
            let mut total = 0.0;
            for index in 0..len {
                if every_criterion_matches(&args[1..], index) {
                    total += args[0].values[index].as_number().unwrap_or(0.0);
                }
            }
            Ok(Value::Number(total))
        }
        "AVERAGEIFS" => {
            require_at_least(name, args, 3)?;
            if args.len() % 2 != 1 {
                return Err(Error::WrongArity {
                    name: name.to_string(),
                    given: args.len(),
                    expected: "an average range followed by criteria pairs",
                });
            }
            let len = args[0].values.len();
            let matched: Vec<f64> = (0..len)
                .filter(|&index| every_criterion_matches(&args[1..], index))
                .filter_map(|index| args[0].values[index].as_number().ok())
                .collect();
            if matched.is_empty() {
                return Err(Error::Arithmetic(ValueError::DivisionByZero));
            }
            let count = matched.len() as f64;
            Ok(Value::Number(matched.iter().sum::<f64>() / count))
        }
        "MAXIFS" | "MINIFS" => {
            require_at_least(name, args, 3)?;
            if args.len() % 2 != 1 {
                return Err(Error::WrongArity {
                    name: name.to_string(),
                    given: args.len(),
                    expected: "a value range followed by criteria pairs",
                });
            }
            let len = args[0].values.len();
            let want_min = name == "MINIFS";
            let mut best: Option<f64> = None;
            for index in 0..len {
                if !every_criterion_matches(&args[1..], index) {
                    continue;
                }
                let Some(value) = args[0].values[index].as_number().ok() else {
                    continue;
                };
                best = Some(match best {
                    None => value,
                    Some(current) => {
                        if (want_min && value < current) || (!want_min && value > current) {
                            value
                        } else {
                            current
                        }
                    }
                });
            }
            // With no matching cell the answer is 0, as in a spreadsheet.
            Ok(Value::Number(best.unwrap_or(0.0)))
        }
        // `IF`, `IFERROR` and `SWITCH` are evaluated by the evaluator, not here,
        // because they must not evaluate every argument. These arms exist only
        // so the name is dispatched rather than reported unknown; the evaluator
        // intercepts them before this function is reached.
        "IF" => Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: "3",
        }),
        "IFERROR" => Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: "2",
        }),
        "SWITCH" => Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: "at least 3",
        }),
        "IFS" => {
            // Pairs of condition and result, then an optional final fallback.
            // Every condition is already evaluated by the caller, so this cannot
            // short-circuit the way `IF` does; it picks, it does not save work.
            require_at_least(name, args, 2)?;
            let mut i = 0;
            while i + 1 < args.len() {
                if first_value(&args[i]).as_bool() {
                    return Ok(first_value(&args[i + 1]));
                }
                i += 2;
            }
            if args.len() % 2 == 1 {
                return Ok(first_value(&args[args.len() - 1]));
            }
            Err(Error::NoMatchingCondition)
        }
        "EXACT" => {
            require_arity(name, args, 2)?;
            let a = first_text(&args[0]);
            let b = first_text(&args[1]);
            Ok(Value::Bool(a == b))
        }
        "FIND" => {
            require_between(name, args, 2, 3)?;
            let needle = first_text(&args[0]);
            let haystack = first_text(&args[1]);
            let start = match args.get(2) {
                Some(arg) => first_number(name, arg)?.max(1.0) as usize,
                None => 1,
            };
            // FIND is case-sensitive and has no wildcards.
            let chars: Vec<char> = haystack.chars().collect();
            let from = start.saturating_sub(1).min(chars.len());
            let window: String = chars[from..].iter().collect();
            match window.find(&needle) {
                Some(byte_index) => {
                    let before = window[..byte_index].chars().count();
                    Ok(Value::Number((from + before + 1) as f64))
                }
                None => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        "SEARCH" => {
            require_between(name, args, 2, 3)?;
            let needle = first_text(&args[0]).to_lowercase();
            let haystack = first_text(&args[1]).to_lowercase();
            let start = match args.get(2) {
                Some(arg) => first_number(name, arg)?.max(1.0) as usize,
                None => 1,
            };
            // SEARCH ignores case and understands `*` and `?`.
            let chars: Vec<char> = haystack.chars().collect();
            let from = start.saturating_sub(1).min(chars.len());
            let window: String = chars[from..].iter().collect();
            match search_index(&window, &needle) {
                Some(offset) => Ok(Value::Number((from + offset + 1) as f64)),
                None => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        "PROPER" => unary_text(name, args, proper_case),
        "CHAR" => {
            require_arity(name, args, 1)?;
            let code = first_number(name, &args[0])? as u32;
            let ch = char::from_u32(code).unwrap_or('\u{fffd}');
            Ok(Value::Text(ch.to_string()))
        }
        "CODE" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            match text.chars().next() {
                Some(ch) => Ok(Value::Number(f64::from(ch as u32))),
                None => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        "UNICHAR" => {
            require_arity(name, args, 1)?;
            let code = first_number(name, &args[0])? as u32;
            let ch = char::from_u32(code).unwrap_or('\u{fffd}');
            Ok(Value::Text(ch.to_string()))
        }
        "UNICODE" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            match text.chars().next() {
                Some(ch) => Ok(Value::Number(f64::from(ch as u32))),
                None => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        "CLEAN" => unary_text(name, args, |text| {
            // The C0 controls are the ones a spreadsheet strips; the C1 range
            // is left alone, as in Excel.
            text.chars()
                .filter(|c| !c.is_control() || *c > '\u{1f}')
                .collect()
        }),
        "T" => {
            require_arity(name, args, 1)?;
            // `T` returns text unchanged and everything else as an empty string.
            Ok(match first_value(&args[0]) {
                Value::Text(t) => Value::Text(t),
                _ => Value::Text(String::new()),
            })
        }
        "N" => {
            require_arity(name, args, 1)?;
            // `N` converts a number or a logical to a number, and anything else
            // to zero.
            Ok(Value::Number(match first_value(&args[0]) {
                Value::Number(n) => n,
                Value::Bool(b) if b => 1.0,
                _ => 0.0,
            }))
        }
        "TEXT" => {
            require_arity(name, args, 2)?;
            let value = first_number(name, &args[0])?;
            let format = first_text(&args[1]);
            match format_number(value, &format) {
                Some(text) => Ok(Value::Text(text)),
                None => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        "VALUE" => {
            require_arity(name, args, 1)?;
            let raw = first_text(&args[0]);
            // Strip currency symbols, thousands separators and the accounting
            // parentheses around a negative, then parse what is left.
            let mut cleaned: String = raw
                .chars()
                .filter(|c| !matches!(c, '$' | '€' | '£' | '¥' | ',' | ' '))
                .collect();
            let negative = cleaned.starts_with('(') && cleaned.ends_with(')');
            if negative {
                cleaned = cleaned[1..cleaned.len() - 1].to_string();
            }
            let percent = cleaned.ends_with('%');
            if percent {
                cleaned.pop();
            }
            let mut number = match cleaned.parse::<f64>() {
                Ok(n) => n,
                Err(_) => return Err(Error::Arithmetic(ValueError::NotANumber)),
            };
            if negative {
                number = -number;
            }
            if percent {
                number /= 100.0;
            }
            Ok(Value::Number(number))
        }
        "REPT" => {
            require_arity(name, args, 2)?;
            let text = first_text(&args[0]);
            let times = args[1]
                .values
                .first()
                .map_or(0.0, |v| v.as_number().unwrap_or(0.0));
            if times < 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Text(text.repeat(times as usize)))
        }
        "SUBSTITUTE" => {
            require_arity(name, args, 3)?;
            let text = first_text(&args[0]);
            let from = first_text(&args[1]);
            let to = first_text(&args[2]);
            // An empty needle would loop forever; a spreadsheet leaves the text
            // alone rather than hanging, and so does this.
            if from.is_empty() {
                return Ok(Value::Text(text));
            }
            Ok(Value::Text(text.replace(&from, &to)))
        }
        "LEN" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            Ok(Value::Number(text.chars().count() as f64))
        }
        "UPPER" => unary_text(name, args, str::to_uppercase),
        "LOWER" => unary_text(name, args, str::to_lowercase),
        "TRIM" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            Ok(Value::Text(
                text.split_whitespace().collect::<Vec<_>>().join(" "),
            ))
        }
        "CONCAT" | "CONCATENATE" => {
            require_at_least(name, args, 1)?;
            let mut out = String::new();
            for arg in args {
                for v in &arg.values {
                    out.push_str(&v.to_string());
                }
            }
            Ok(Value::Text(out))
        }
        "CONVERT" => {
            require_arity(name, args, 3)?;
            let number = first_number(name, &args[0])?;
            let from = first_text(&args[1]);
            let to = first_text(&args[2]);
            Ok(Value::Number(crate::convert::convert(number, &from, &to)?))
        }
        "LEFT" => {
            require_arity(name, args, 2)?;
            let text = first_text(&args[0]);
            let n = first_number(name, &args[1])? as usize;
            Ok(Value::Text(text.chars().take(n).collect()))
        }
        "RIGHT" => {
            require_arity(name, args, 2)?;
            let text = first_text(&args[0]);
            let n = first_number(name, &args[1])? as usize;
            let chars: Vec<char> = text.chars().collect();
            let start = chars.len().saturating_sub(n);
            Ok(Value::Text(chars[start..].iter().collect()))
        }
        "MID" => {
            require_arity(name, args, 3)?;
            let text = first_text(&args[0]);
            let start = first_number(name, &args[1])? as usize;
            let len = first_number(name, &args[2])? as usize;
            // MID is one-based, like a spreadsheet.
            let chars: Vec<char> = text.chars().collect();
            let from = start.saturating_sub(1).min(chars.len());
            let to = from.saturating_add(len).min(chars.len());
            Ok(Value::Text(chars[from..to].iter().collect()))
        }
        "AND" => {
            require_at_least(name, args, 1)?;
            Ok(Value::Bool(
                args.iter()
                    .flat_map(|a| a.values.iter())
                    .all(Value::as_bool),
            ))
        }
        "OR" => {
            require_at_least(name, args, 1)?;
            Ok(Value::Bool(
                args.iter()
                    .flat_map(|a| a.values.iter())
                    .any(Value::as_bool),
            ))
        }
        "NOT" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(
                !args[0].values.first().unwrap_or(&Value::Empty).as_bool(),
            ))
        }
        "TRUE" => {
            require_arity(name, args, 0)?;
            Ok(Value::Bool(true))
        }
        "FALSE" => {
            require_arity(name, args, 0)?;
            Ok(Value::Bool(false))
        }
        "IFNA" => {
            require_arity(name, args, 2)?;
            let value = first_value(&args[0]);
            if matches!(value, Value::Empty) {
                Ok(first_value(&args[1]))
            } else {
                Ok(value)
            }
        }
        "ISERROR" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(matches!(first_value(&args[0]), Value::Empty)))
        }
        "ISERR" => {
            // The reference has no error value to inspect, so this mirrors
            // `ISERROR`; both exist so a formula written with either parses.
            require_arity(name, args, 1)?;
            Ok(Value::Bool(matches!(first_value(&args[0]), Value::Empty)))
        }
        "ISNA" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(matches!(first_value(&args[0]), Value::Empty)))
        }
        "NA" => {
            require_arity(name, args, 0)?;
            Err(Error::Arithmetic(ValueError::NotANumber))
        }
        // --- serial dates ----------------------------------------------------
        // A date is a number of days from the 1899-12-30 epoch and the time of
        // day is its fraction; `datetime` holds the arithmetic. Every function
        // below takes and returns that number.
        "DATE" => {
            require_arity(name, args, 3)?;
            let y = first_number(name, &args[0])?;
            let m = first_number(name, &args[1])?;
            let d = first_number(name, &args[2])?;
            Ok(Value::Number(datetime::date_serial(y, m, d) as f64))
        }
        "TIME" => {
            require_arity(name, args, 3)?;
            let h = first_number(name, &args[0])?.trunc();
            let m = first_number(name, &args[1])?.trunc();
            let s = first_number(name, &args[2])?.trunc();
            let seconds = h * 3600.0 + m * 60.0 + s;
            // A spreadsheet wraps a time past midnight rather than overflowing
            // into the next day.
            Ok(Value::Number(
                seconds.rem_euclid(datetime::SECONDS_PER_DAY) / datetime::SECONDS_PER_DAY,
            ))
        }
        "DAY" | "MONTH" | "YEAR" => {
            require_arity(name, args, 1)?;
            let serial = first_number(name, &args[0])?;
            let (y, m, d) = datetime::ymd_from_serial(datetime::date_part(serial));
            Ok(Value::Number(match name {
                "DAY" => d,
                "MONTH" => m,
                _ => y,
            } as f64))
        }
        "HOUR" | "MINUTE" | "SECOND" => {
            require_arity(name, args, 1)?;
            let serial = first_number(name, &args[0])?;
            let seconds = (datetime::time_part(serial) * datetime::SECONDS_PER_DAY).round() as i64;
            Ok(Value::Number(match name {
                "HOUR" => seconds / 3600,
                "MINUTE" => (seconds / 60) % 60,
                _ => seconds % 60,
            } as f64))
        }
        "EDATE" | "EOMONTH" => {
            require_arity(name, args, 2)?;
            let start = datetime::date_part(first_number(name, &args[0])?);
            let months = first_number(name, &args[1])?.trunc() as i64;
            let result = if name == "EDATE" {
                datetime::edate(start, months)
            } else {
                datetime::eomonth(start, months)
            };
            Ok(Value::Number(result as f64))
        }
        "DAYS" => {
            require_arity(name, args, 2)?;
            let end = datetime::date_part(first_number(name, &args[0])?);
            let start = datetime::date_part(first_number(name, &args[1])?);
            Ok(Value::Number((end - start) as f64))
        }
        "DAYS360" => {
            require_between(name, args, 2, 3)?;
            let start = datetime::date_part(first_number(name, &args[0])?);
            let end = datetime::date_part(first_number(name, &args[1])?);
            let european = args.get(2).is_some_and(|a| first_value(a).as_bool());
            let days = if european {
                datetime::days360_european(start, end)
            } else {
                datetime::days360_us(start, end)
            };
            Ok(Value::Number(days as f64))
        }
        "YEARFRAC" => {
            require_between(name, args, 2, 3)?;
            let start = datetime::date_part(first_number(name, &args[0])?);
            let end = datetime::date_part(first_number(name, &args[1])?);
            let basis = match args.get(2) {
                Some(arg) => first_number(name, arg)?.trunc() as i64,
                None => 0,
            };
            Ok(Value::Number(datetime::yearfrac(start, end, basis)))
        }
        "DATEDIF" => {
            require_arity(name, args, 3)?;
            let start = datetime::date_part(first_number(name, &args[0])?);
            let end = datetime::date_part(first_number(name, &args[1])?);
            let unit = first_text(&args[2]).to_ascii_uppercase();
            datetime::datedif(start, end, &unit)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "WEEKDAY" => {
            require_between(name, args, 1, 2)?;
            let serial = datetime::date_part(first_number(name, &args[0])?);
            let kind = match args.get(1) {
                Some(arg) => first_number(name, arg)?.trunc() as i64,
                None => 1,
            };
            datetime::weekday(serial, kind)
                .map(|n| Value::Number(n as f64))
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "WEEKNUM" => {
            require_between(name, args, 1, 2)?;
            let serial = datetime::date_part(first_number(name, &args[0])?);
            let kind = match args.get(1) {
                Some(arg) => first_number(name, arg)?.trunc() as i64,
                None => 1,
            };
            datetime::weeknum(serial, kind)
                .map(|n| Value::Number(n as f64))
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "ISOWEEKNUM" => {
            require_arity(name, args, 1)?;
            let serial = datetime::date_part(first_number(name, &args[0])?);
            Ok(Value::Number(datetime::iso_week(serial) as f64))
        }
        "DATEVALUE" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            datetime::parse_date(&text)
                .map(|s| Value::Number(s as f64))
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "TIMEVALUE" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            datetime::parse_time(&text)
                .map(|s| Value::Number(s / datetime::SECONDS_PER_DAY))
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "NETWORKDAYSINTL" | "WORKDAYINTL" => {
            require_between(name, args, 2, 4)?;
            let first = datetime::date_part(first_number(name, &args[0])?);
            let second = first_number(name, &args[1])?;
            // The third argument is either a weekend code or a seven-character
            // mask read as text, and the mask wins when it looks like one.
            let weekend = match args.get(2) {
                Some(arg) => match &arg.values[0] {
                    Value::Text(mask) => datetime::weekend_mask(mask)
                        .ok_or(Error::Arithmetic(ValueError::NotANumber))?,
                    other => datetime::weekend_code(other.as_number()? as i64)
                        .ok_or(Error::Arithmetic(ValueError::NotANumber))?,
                },
                None => datetime::weekend_code(1).expect("code 1 is a weekend"),
            };
            let holidays: Vec<i64> = match args.get(3) {
                Some(arg) => arg
                    .values
                    .iter()
                    .filter_map(|v| v.as_number().ok())
                    .map(datetime::date_part)
                    .collect(),
                None => Vec::new(),
            };
            if name == "NETWORKDAYSINTL" {
                let end = datetime::date_part(second);
                Ok(Value::Number(
                    datetime::networkdays_intl(first, end, &weekend, &holidays) as f64,
                ))
            } else {
                let days = second.trunc() as i64;
                Ok(Value::Number(
                    datetime::workday_intl(first, days, &weekend, &holidays) as f64,
                ))
            }
        }
        "NETWORKDAYS" | "WORKDAY" => {
            require_between(name, args, 2, 3)?;
            let first = datetime::date_part(first_number(name, &args[0])?);
            let second = first_number(name, &args[1])?;
            let holidays: Vec<i64> = match args.get(2) {
                Some(arg) => arg
                    .values
                    .iter()
                    .filter_map(|v| v.as_number().ok())
                    .map(datetime::date_part)
                    .collect(),
                None => Vec::new(),
            };
            if name == "NETWORKDAYS" {
                let end = datetime::date_part(second);
                Ok(Value::Number(
                    datetime::networkdays(first, end, &holidays) as f64
                ))
            } else {
                let days = second.trunc() as i64;
                Ok(Value::Number(
                    datetime::workday(first, days, &holidays) as f64
                ))
            }
        }
        // --- lookup and reference --------------------------------------------
        // These need the shape of a range, not just its values, so the argument
        // carries its width and the module walks rows and columns.
        "VLOOKUP" | "HLOOKUP" => {
            require_between(name, args, 3, 4)?;
            let vertical = name == "VLOOKUP";
            // The signature is `(lookup_value, table, index, range_lookup)`, so
            // the table is the second argument.
            let table = &args[1];
            let width = table.width.unwrap_or(table.values.len().max(1));
            let target = first_value(&args[0]);
            let approximate = args.get(3).is_none_or(|arg| first_value(arg).as_bool());
            let mode = if approximate {
                MatchMode::Approximate
            } else {
                MatchMode::Exact
            };
            // The index is one-based and must stay inside the table.
            let index = first_number(name, &args[2])?.trunc();
            if index < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let axis = if vertical {
                Axis::vertical(0, 0, index as i64 - 1)
            } else {
                Axis::horizontal(0, 0, index as i64 - 1)
            };
            lookup::table_lookup(&table.values, width, &target, mode, vertical, axis)
                .map(Ok)
                .unwrap_or(Err(Error::Arithmetic(ValueError::NotANumber)))
        }
        "MATCH" => {
            require_between(name, args, 2, 3)?;
            let target = first_value(&args[0]);
            let flag = match args.get(2) {
                Some(arg) => first_number(name, arg)?,
                None => 1.0,
            };
            let mode = lookup::match_mode(flag);
            lookup::find(&args[1].values, &target, mode, true)
                .map(|index| Value::Number(index as f64 + 1.0))
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "INDEX" => {
            require_between(name, args, 2, 4)?;
            let width = args[0].width.unwrap_or(1);
            let row = first_number(name, &args[1])?.trunc() as i64;
            let column = match args.get(2) {
                Some(arg) => first_number(name, arg)?.trunc() as i64,
                // A one-column range needs no second index.
                None => 1,
            };
            lookup::index(&args[0].values, width, row, column)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "LOOKUP" => {
            require_between(name, args, 2, 3)?;
            let target = first_value(&args[0]);
            // The three-argument form searches the second range and returns the
            // value at the same position of the third.
            if let Some(result) = args.get(2) {
                let found = lookup::find(&args[1].values, &target, MatchMode::Approximate, false)
                    .ok_or(Error::Arithmetic(ValueError::NotANumber))?;
                return result
                    .values
                    .get(found)
                    .cloned()
                    .ok_or(Error::Arithmetic(ValueError::NotANumber));
            }
            let width = args[1].width.unwrap_or(1);
            lookup::lookup(&args[1].values, width, &target)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "CHOOSE" => {
            require_at_least(name, args, 2)?;
            let index = first_number(name, &args[0])?.trunc();
            if index < 1.0 || index as usize >= args.len() {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(first_value(&args[index as usize]))
        }
        "ROWS" => {
            require_arity(name, args, 1)?;
            let (rows, _) = args[0].shape().unwrap_or((args[0].values.len().max(1), 1));
            Ok(Value::Number(rows as f64))
        }
        "COLUMNS" => {
            require_arity(name, args, 1)?;
            let (_, columns) = args[0].shape().unwrap_or((1, 1));
            Ok(Value::Number(columns as f64))
        }
        "ROW" => {
            require_between(name, args, 0, 1)?;
            // With no argument the row of the calling cell, which the evaluator
            // supplies as the origin.
            let row = match args.first().and_then(|arg| arg.origin) {
                Some((row, _)) => row,
                None => origin.ok_or(Error::Arithmetic(ValueError::NotANumber))?.0,
            };
            Ok(Value::Number(f64::from(row) + 1.0))
        }
        "COLUMN" => {
            require_between(name, args, 0, 1)?;
            let column = match args.first().and_then(|arg| arg.origin) {
                Some((_, column)) => column,
                None => origin.ok_or(Error::Arithmetic(ValueError::NotANumber))?.1,
            };
            Ok(Value::Number(f64::from(column) + 1.0))
        }
        // --- complex numbers -------------------------------------------------
        // A complex number is text such as `3+4i`; `complex` parses and formats
        // it and these arms only marshal the arguments.
        "COMPLEX" => {
            require_between(name, args, 2, 3)?;
            let re = first_number(name, &args[0])?;
            let im = first_number(name, &args[1])?;
            let unit = match args.get(2) {
                Some(arg) => match first_text(arg).to_ascii_lowercase().as_str() {
                    "i" => 'i',
                    "j" => 'j',
                    _ => {
                        return Err(Error::TypeError {
                            operation: name.to_string(),
                            found: first_value(arg),
                        });
                    }
                },
                None => 'i',
            };
            Ok(Value::Text(Complex::new(re, im).format(unit)))
        }
        "IMREAL" => {
            require_arity(name, args, 1)?;
            Ok(Value::Number(complex_arg(name, &args[0])?.re))
        }
        "IMAGINARY" => {
            require_arity(name, args, 1)?;
            Ok(Value::Number(complex_arg(name, &args[0])?.im))
        }
        "IMABS" => {
            require_arity(name, args, 1)?;
            Ok(Value::Number(complex_arg(name, &args[0])?.modulus()))
        }
        "IMARGUMENT" => {
            require_arity(name, args, 1)?;
            let z = complex_arg(name, &args[0])?;
            if z.re == 0.0 && z.im == 0.0 {
                return Err(Error::Arithmetic(ValueError::DivisionByZero));
            }
            Ok(Value::Number(z.argument()))
        }
        "IMCONJUGATE" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            Ok(Value::Text(z.conjugate().format(unit)))
        }
        "IMSQRT" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            Ok(Value::Text(z.sqrt().format(unit)))
        }
        "IMEXP" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            Ok(Value::Text(z.exp().format(unit)))
        }
        "IMLN" | "IMLOG10" | "IMLOG2" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            if z.re == 0.0 && z.im == 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let ln = z.ln().expect("non-zero checked above");
            // The other bases are the natural logarithm divided by the base's
            // logarithm. Dividing rather than multiplying by a reciprocal
            // matters: the two round differently in the last digit.
            let scaled = match name {
                "IMLN" => ln,
                "IMLOG10" => Complex::new(ln.re / 10f64.ln(), ln.im / 10f64.ln()),
                _ => Complex::new(ln.re / 2f64.ln(), ln.im / 2f64.ln()),
            };
            Ok(Value::Text(scaled.format(unit)))
        }
        "IMSIN" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            Ok(Value::Text(z.sin().format(unit)))
        }
        "IMCOS" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            Ok(Value::Text(z.cos().format(unit)))
        }
        "IMTAN" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            let t = z
                .tan()
                .ok_or(Error::Arithmetic(ValueError::DivisionByZero))?;
            Ok(Value::Text(t.format(unit)))
        }
        // The reciprocal functions are the identity over the corresponding
        // direct one; `IMCOT`, for instance, is `cos / sin`.
        "IMCOT" | "IMSEC" | "IMCSC" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            let one = Complex::new(1.0, 0.0);
            let (numerator, denominator) = match name {
                "IMCOT" => (z.cos(), z.sin()),
                "IMSEC" => (one, z.cos()),
                _ => (one, z.sin()),
            };
            let value = numerator
                .div(denominator)
                .ok_or(Error::Arithmetic(ValueError::DivisionByZero))?;
            Ok(Value::Text(value.format(unit)))
        }
        "IMSINH" | "IMCOSH" | "IMCSCH" | "IMSECH" => {
            require_arity(name, args, 1)?;
            let (z, unit) = complex_and_unit(name, &args[0])?;
            let i = Complex::new(0.0, 1.0);
            let value = match name {
                // `sinh z = -i sin(iz)` and `cosh z = cos(iz)`.
                "IMSINH" => z.mul(i).sin().mul(Complex::new(0.0, -1.0)),
                "IMCOSH" => z.mul(i).cos(),
                "IMCSCH" => Complex::new(1.0, 0.0)
                    .div(z.mul(i).sin().mul(Complex::new(0.0, -1.0)))
                    .ok_or(Error::Arithmetic(ValueError::DivisionByZero))?,
                _ => Complex::new(1.0, 0.0)
                    .div(z.mul(i).cos())
                    .ok_or(Error::Arithmetic(ValueError::DivisionByZero))?,
            };
            Ok(Value::Text(value.format(unit)))
        }
        "IMSUM" | "IMSUB" | "IMPRODUCT" => {
            require_at_least(name, args, 1)?;
            let unit = complex_unit_of(args)?;
            let mut values = Vec::with_capacity(args.len());
            for arg in args {
                values.push(Complex::parse(&first_text(arg)).ok_or(Error::TypeError {
                    operation: name.to_string(),
                    found: first_value(arg),
                })?);
            }
            let mut acc = values[0];
            match name {
                "IMSUM" => {
                    for z in &values[1..] {
                        acc = Complex::new(acc.re + z.re, acc.im + z.im);
                    }
                }
                "IMSUB" => {
                    for z in &values[1..] {
                        acc = Complex::new(acc.re - z.re, acc.im - z.im);
                    }
                }
                _ => {
                    for z in &values[1..] {
                        acc = acc.mul(*z);
                    }
                }
            }
            Ok(Value::Text(acc.format(unit)))
        }
        "IMDIV" | "IMPOWER" => {
            require_arity(name, args, 2)?;
            let (a, unit) = complex_and_unit(name, &args[0])?;
            let result = if name == "IMDIV" {
                let b = Complex::parse(&first_text(&args[1])).ok_or(Error::TypeError {
                    operation: name.to_string(),
                    found: first_value(&args[1]),
                })?;
                a.div(b)
                    .ok_or(Error::Arithmetic(ValueError::DivisionByZero))?
            } else {
                let n = first_number(name, &args[1])?;
                a.pow(n).ok_or(Error::Arithmetic(ValueError::NotANumber))?
            };
            Ok(Value::Text(result.format(unit)))
        }
        // --- financial -------------------------------------------------------
        "FV" | "PV" | "PMT" => {
            require_between(name, args, 3, 5)?;
            let rate = first_number(name, &args[0])?;
            let nper = first_number(name, &args[1])?;
            let third = first_number(name, &args[2])?;
            let fourth = match args.get(3) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            let kind = kind_arg(args.get(4));
            let result = match name {
                "FV" => financial::fv(rate, nper, third, fourth, kind),
                "PV" => financial::pv(rate, nper, third, fourth, kind),
                _ => financial::pmt(rate, nper, third, fourth, kind),
            };
            result
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        // `NPER` takes its arguments in a different order from the other three:
        // the payment comes second and the present value third.
        "NPER" => {
            require_between(name, args, 3, 5)?;
            let rate = first_number(name, &args[0])?;
            let payment = first_number(name, &args[1])?;
            let pv_value = first_number(name, &args[2])?;
            let fv_value = match args.get(3) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            let kind = kind_arg(args.get(4));
            financial::nper(rate, payment, pv_value, fv_value, kind)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "IPMT" | "PPMT" => {
            require_between(name, args, 4, 6)?;
            let rate = first_number(name, &args[0])?;
            let per = first_number(name, &args[1])?;
            let nper = first_number(name, &args[2])?;
            let pv_value = first_number(name, &args[3])?;
            let fv_value = match args.get(4) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            let kind = match args.get(5) {
                Some(arg) => first_value(arg).as_bool().into(),
                None => 0.0,
            };
            let result = if name == "IPMT" {
                financial::ipmt(rate, per, nper, pv_value, fv_value, kind)
            } else {
                financial::ppmt(rate, per, nper, pv_value, fv_value, kind)
            };
            result
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "CUMIPMT" | "CUMPRINC" => {
            require_arity(name, args, 6)?;
            let rate = first_number(name, &args[0])?;
            let nper = first_number(name, &args[1])?;
            let pv_value = first_number(name, &args[2])?;
            let start = first_number(name, &args[3])?;
            let end = first_number(name, &args[4])?;
            let kind = first_value(&args[5]).as_bool().into();
            let result = if name == "CUMIPMT" {
                financial::cumipmt(rate, nper, pv_value, start, end, kind)
            } else {
                financial::cumprinc(rate, nper, pv_value, start, end, kind)
            };
            result
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "ISPMT" => {
            require_arity(name, args, 4)?;
            let rate = first_number(name, &args[0])?;
            let per = first_number(name, &args[1])?;
            let nper = first_number(name, &args[2])?;
            let pv_value = first_number(name, &args[3])?;
            financial::ispmt(rate, per, nper, pv_value)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "FVSCHEDULE" => {
            require_arity(name, args, 2)?;
            let principal = first_number(name, &args[0])?;
            let rates = numbers_of(name, &args[1..])?;
            Ok(Value::Number(financial::fvschedule(principal, &rates)))
        }
        "NPV" => {
            require_at_least(name, args, 2)?;
            let rate = first_number(name, &args[0])?;
            let amounts = numbers_of(name, &args[1..])?;
            financial::npv(rate, &amounts)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "IRR" => {
            require_between(name, args, 1, 2)?;
            let amounts = numbers_of(name, &args[0..1])?;
            let guess = match args.get(1) {
                Some(arg) => first_number(name, arg)?,
                None => 0.1,
            };
            financial::CashFlows { amounts: &amounts }
                .irr(guess)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "MIRR" => {
            require_arity(name, args, 3)?;
            let amounts = numbers_of(name, &args[0..1])?;
            let finance_rate = first_number(name, &args[1])?;
            let reinvest_rate = first_number(name, &args[2])?;
            financial::mirr(&amounts, finance_rate, reinvest_rate)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "XNPV" => {
            require_arity(name, args, 3)?;
            let rate = first_number(name, &args[0])?;
            let amounts = numbers_of(name, &args[1..2])?;
            let dates = numbers_of(name, &args[2..3])?;
            financial::xnpv(rate, &amounts, &dates)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "XIRR" => {
            require_between(name, args, 2, 3)?;
            let amounts = numbers_of(name, &args[0..1])?;
            let dates = numbers_of(name, &args[1..2])?;
            let guess = match args.get(2) {
                Some(arg) => first_number(name, arg)?,
                None => 0.1,
            };
            financial::xirr(&amounts, &dates, guess)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "SLN" => {
            require_arity(name, args, 3)?;
            let cost = first_number(name, &args[0])?;
            let salvage = first_number(name, &args[1])?;
            let life = first_number(name, &args[2])?;
            financial::sln(cost, salvage, life)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::DivisionByZero))
        }
        "SYD" => {
            require_arity(name, args, 4)?;
            let cost = first_number(name, &args[0])?;
            let salvage = first_number(name, &args[1])?;
            let life = first_number(name, &args[2])?;
            let per = first_number(name, &args[3])?;
            financial::syd(cost, salvage, life, per)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "DDB" => {
            require_between(name, args, 4, 5)?;
            let cost = first_number(name, &args[0])?;
            let salvage = first_number(name, &args[1])?;
            let life = first_number(name, &args[2])?;
            let per = first_number(name, &args[3])?;
            let factor = match args.get(4) {
                Some(arg) => first_number(name, arg)?,
                None => 2.0,
            };
            financial::ddb(cost, salvage, life, per, factor)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "DB" => {
            require_between(name, args, 4, 5)?;
            let cost = first_number(name, &args[0])?;
            let salvage = first_number(name, &args[1])?;
            let life = first_number(name, &args[2])?;
            let per = first_number(name, &args[3])?;
            let month = match args.get(4) {
                Some(arg) => first_number(name, arg)?,
                None => 12.0,
            };
            financial::db(cost, salvage, life, per, month)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "EFFECT" => {
            require_arity(name, args, 2)?;
            financial::effect(first_number(name, &args[0])?, first_number(name, &args[1])?)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "NOMINAL" => {
            require_arity(name, args, 2)?;
            financial::nominal(first_number(name, &args[0])?, first_number(name, &args[1])?)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "RATE" => {
            require_between(name, args, 3, 6)?;
            let nper = first_number(name, &args[0])?;
            let payment = first_number(name, &args[1])?;
            let pv_value = first_number(name, &args[2])?;
            let fv_value = match args.get(3) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            let kind = match args.get(4) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            let guess = match args.get(5) {
                Some(arg) => first_number(name, arg)?,
                None => 0.1,
            };
            financial::rate(nper, payment, pv_value, fv_value, kind, guess)
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "RRI" => {
            require_arity(name, args, 3)?;
            financial::rri(
                first_number(name, &args[0])?,
                first_number(name, &args[1])?,
                first_number(name, &args[2])?,
            )
            .map(Value::Number)
            .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "PDURATION" => {
            require_arity(name, args, 3)?;
            financial::pduration(
                first_number(name, &args[0])?,
                first_number(name, &args[1])?,
                first_number(name, &args[2])?,
            )
            .map(Value::Number)
            .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "DOLLARDE" | "DOLLARFR" => {
            require_arity(name, args, 2)?;
            let value = first_number(name, &args[0])?;
            let fraction = first_number(name, &args[1])?;
            let result = if name == "DOLLARDE" {
                financial::dollarde(value, fraction)
            } else {
                financial::dollarfr(value, fraction)
            };
            result
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "COUPDAYS" => {
            require_between(name, args, 3, 4)?;
            let basis = args
                .get(3)
                .map_or(0, |a| first_value(a).as_number().unwrap_or(0.0) as i64);
            financial::coupdays(
                first_number(name, &args[0])?,
                first_number(name, &args[1])?,
                first_number(name, &args[2])? as i64,
                basis,
            )
            .map(Value::Number)
            .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "DISC" => {
            require_between(name, args, 4, 5)?;
            let basis = args
                .get(4)
                .map_or(0, |a| first_value(a).as_number().unwrap_or(0.0) as i64);
            financial::disc(
                first_number(name, &args[0])?,
                first_number(name, &args[1])?,
                first_number(name, &args[2])?,
                first_number(name, &args[3])?,
                basis,
            )
            .map(Value::Number)
            .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "PRICEDISC" => {
            require_between(name, args, 4, 5)?;
            let basis = args
                .get(4)
                .map_or(0, |a| first_value(a).as_number().unwrap_or(0.0) as i64);
            financial::pricedisc(
                first_number(name, &args[0])?,
                first_number(name, &args[1])?,
                first_number(name, &args[2])?,
                first_number(name, &args[3])?,
                basis,
            )
            .map(Value::Number)
            .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "TBILLPRICE" | "TBILLYIELD" | "TBILLEQ" => {
            require_arity(name, args, 3)?;
            let settlement = first_number(name, &args[0])?;
            let maturity = first_number(name, &args[1])?;
            let third = first_number(name, &args[2])?;
            let result = match name {
                "TBILLPRICE" => financial::tbillprice(settlement, maturity, third),
                "TBILLYIELD" => financial::tbillyield(settlement, maturity, third),
                _ => financial::tbilleq(settlement, maturity, third),
            };
            result
                .map(Value::Number)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        "ACCRINT" => {
            require_between(name, args, 6, 7)?;
            let basis = args
                .get(6)
                .map_or(0.0, |a| first_value(a).as_number().unwrap_or(0.0));
            financial::accrint(
                first_number(name, &args[0])?,
                first_number(name, &args[1])?,
                first_number(name, &args[2])?,
                first_number(name, &args[3])?,
                first_number(name, &args[4])?,
                first_number(name, &args[5])?,
                basis,
            )
            .map(Value::Number)
            .ok_or(Error::Arithmetic(ValueError::NotANumber))
        }
        // --- base conversion -------------------------------------------------
        // Every base function works on a fixed ten-digit field: a leading digit
        // above the base's half is a negative number, which is how a spreadsheet
        // stores one in these functions.
        "BIN2DEC" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            let value = parse_radix(&text, 2).ok_or(Error::Arithmetic(ValueError::NotANumber))?;
            Ok(Value::Number(signed_field(value, &text, 2, 10)))
        }
        "HEX2DEC" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            let value = parse_radix(&text, 16).ok_or(Error::Arithmetic(ValueError::NotANumber))?;
            Ok(Value::Number(signed_field(value, &text, 16, 10)))
        }
        "OCT2DEC" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            let value = parse_radix(&text, 8).ok_or(Error::Arithmetic(ValueError::NotANumber))?;
            Ok(Value::Number(signed_field(value, &text, 8, 10)))
        }
        "DEC2BIN" => decimal_to_radix(name, args, 2, 512.0, 511.0),
        "DEC2OCT" => decimal_to_radix(name, args, 8, 536_870_912.0, 536_870_911.0),
        "DEC2HEX" => decimal_to_radix(name, args, 16, 549_755_813_888.0, 549_755_813_887.0),
        // The cross-base conversions. A source field whose leading digit is at
        // or above half its base is negative, and the two that land in binary or
        // octal are bounded by that narrower field's range.
        "BIN2HEX" => convert_radix_field(name, args, 2, 16, None),
        "BIN2OCT" => convert_radix_field(name, args, 2, 8, None),
        "HEX2BIN" => convert_radix_field(name, args, 16, 2, Some((-512.0, 511.0))),
        "HEX2OCT" => convert_radix_field(name, args, 16, 8, Some((-536_870_912.0, 536_870_911.0))),
        "OCT2BIN" => convert_radix_field(name, args, 8, 2, Some((-512.0, 511.0))),
        "OCT2HEX" => convert_radix_field(name, args, 8, 16, None),
        "BASE" => {
            require_between(name, args, 2, 3)?;
            let number = first_number(name, &args[0])?;
            let radix = first_number(name, &args[1])? as u32;
            if !(2..=36).contains(&radix) {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let digits = radix_digits(number, radix);
            let min_length = match args.get(2) {
                Some(arg) => first_number(name, arg)? as usize,
                None => 0,
            };
            Ok(Value::Text(format!(
                "{digits:0>width$}",
                width = min_length.max(digits.len())
            )))
        }
        "DECIMAL" => {
            require_arity(name, args, 2)?;
            let text = first_text(&args[0]);
            let radix = first_number(name, &args[1])? as u32;
            if !(2..=36).contains(&radix) {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let value =
                parse_radix(&text, radix).ok_or(Error::Arithmetic(ValueError::NotANumber))?;
            Ok(Value::Number(value as f64))
        }
        // --- bitwise ---------------------------------------------------------
        "BITAND" | "BITOR" | "BITXOR" => {
            require_arity(name, args, 2)?;
            let a = bit_operand(name, &args[0])?;
            let b = bit_operand(name, &args[1])?;
            Ok(Value::Number(match name {
                "BITAND" => (a & b) as f64,
                "BITOR" => (a | b) as f64,
                _ => (a ^ b) as f64,
            }))
        }
        "BITLSHIFT" | "BITRSHIFT" => {
            require_arity(name, args, 2)?;
            let value = bit_operand(name, &args[0])?;
            let shift = first_number(name, &args[1])?;
            if !(0.0..=53.0).contains(&shift) || shift.fract() != 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let shift = shift as u32;
            let out = if name == "BITLSHIFT" {
                value.checked_shl(shift)
            } else {
                value.checked_shr(shift)
            };
            // The reference rejects a shift that would leave the 48-bit range
            // rather than truncating it.
            let out = out
                .filter(|v| *v <= MAX_BIT_VALUE)
                .ok_or(Error::Arithmetic(ValueError::NotANumber))?;
            Ok(Value::Number(out as f64))
        }
        // --- roman numerals --------------------------------------------------
        "ROMAN" => {
            require_arity(name, args, 1)?;
            let number = first_number(name, &args[0])?;
            Ok(Value::Text(to_roman(number)))
        }
        "ARABIC" => {
            require_arity(name, args, 1)?;
            let text = first_text(&args[0]);
            match from_roman(&text) {
                Some(value) => Ok(Value::Number(value as f64)),
                None => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        // --- information -----------------------------------------------------
        "ISBLANK" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(first_value(&args[0]).is_empty()))
        }
        "ISNUMBER" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(matches!(
                first_value(&args[0]),
                Value::Number(_)
            )))
        }
        "ISTEXT" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(matches!(first_value(&args[0]), Value::Text(_))))
        }
        "ISNONTEXT" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(!matches!(
                first_value(&args[0]),
                Value::Text(_)
            )))
        }
        "ISLOGICAL" => {
            require_arity(name, args, 1)?;
            Ok(Value::Bool(matches!(first_value(&args[0]), Value::Bool(_))))
        }
        "ISEVEN" | "ISODD" => {
            require_arity(name, args, 1)?;
            // The integer part is what is tested, and the sign is ignored.
            let n = first_number(name, &args[0])?.abs().floor() as i64;
            let even = n % 2 == 0;
            Ok(Value::Bool(if name == "ISEVEN" { even } else { !even }))
        }
        "TYPE" => {
            require_arity(name, args, 1)?;
            Ok(Value::Number(match first_value(&args[0]) {
                Value::Number(_) => 1.0,
                Value::Text(_) => 2.0,
                Value::Bool(_) => 4.0,
                Value::Empty => 1.0,
            }))
        }
        "DELTA" => {
            require_between(name, args, 1, 2)?;
            let a = first_number(name, &args[0])?;
            let b = match args.get(1) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            Ok(Value::Number(if a == b { 1.0 } else { 0.0 }))
        }
        "GESTEP" => {
            require_between(name, args, 1, 2)?;
            let number = first_number(name, &args[0])?;
            let step = match args.get(1) {
                Some(arg) => first_number(name, arg)?,
                None => 0.0,
            };
            Ok(Value::Number(if number >= step { 1.0 } else { 0.0 }))
        }
        "XOR" => {
            require_at_least(name, args, 1)?;
            // Only the logicals and numbers count, and the result is true when
            // an odd number of them are true.
            let truthy = args
                .iter()
                .flat_map(|a| a.values.iter())
                .filter(|v| !v.is_empty())
                .filter(|v| v.as_number().is_ok())
                .filter(|v| v.as_bool())
                .count();
            Ok(Value::Bool(truthy % 2 == 1))
        }
        // --- text ------------------------------------------------------------
        "REPLACE" => {
            require_arity(name, args, 4)?;
            let text = first_text(&args[0]);
            let start = first_number(name, &args[1])? as usize;
            let count = first_number(name, &args[2])? as usize;
            let new = first_text(&args[3]);
            let chars: Vec<char> = text.chars().collect();
            // One-based, and a start past the end appends.
            let from = start.saturating_sub(1).min(chars.len());
            let to = from.saturating_add(count).min(chars.len());
            let mut out: String = chars[..from].iter().collect();
            out.push_str(&new);
            out.extend(&chars[to..]);
            Ok(Value::Text(out))
        }
        "TEXTJOIN" => {
            require_at_least(name, args, 3)?;
            let delimiter = first_text(&args[0]);
            let ignore_empty = first_value(&args[1]).as_bool();
            let pieces: Vec<String> = args[2..]
                .iter()
                .flat_map(|a| a.values.iter())
                .filter(|v| !(ignore_empty && v.to_string().is_empty()))
                .map(ToString::to_string)
                .collect();
            Ok(Value::Text(pieces.join(&delimiter)))
        }
        "DOLLAR" => {
            require_between(name, args, 1, 2)?;
            let value = first_number(name, &args[0])?;
            let digits = match args.get(1) {
                Some(arg) => first_number(name, arg)? as i32,
                None => 2,
            };
            // The reference rounds to the requested number of decimals before
            // formatting, and never shows a negative count of them.
            let rounded = round_to(value, digits, RoundMode::Half);
            Ok(Value::Text(format_currency(rounded, digits.max(0))))
        }
        "FIXED" => {
            require_between(name, args, 1, 3)?;
            let value = first_number(name, &args[0])?;
            let digits = match args.get(1) {
                Some(arg) => first_number(name, arg)? as i32,
                None => 2,
            };
            let no_commas = match args.get(2) {
                Some(arg) => first_value(arg).as_bool(),
                None => false,
            };
            Ok(Value::Text(format_fixed(value, digits, no_commas)))
        }
        "NUMBERVALUE" => {
            require_between(name, args, 1, 3)?;
            let text = first_text(&args[0]);
            let decimal_sep = args.get(1).map(first_text).unwrap_or_else(|| ".".into());
            let group_sep = args.get(2).map(first_text).unwrap_or_else(|| ",".into());
            match parse_number_value(&text, &decimal_sep, &group_sep) {
                Some(value) => Ok(Value::Number(value)),
                None => Err(Error::Arithmetic(ValueError::NotANumber)),
            }
        }
        // --- Statistical distributions, ported from jStat through formulajs. ---
        // The dispatch normalises a dotted name to its flat alias first, so
        // `NORM.S.DIST` and `NORMSDIST` reach the same arm; the parser accepts
        // dots in a function name for exactly this reason.
        "NORMDIST" => {
            require_arity(name, args, 4)?;
            let x = first_number(name, &args[0])?;
            let mean = first_number(name, &args[1])?;
            let std = first_number(name, &args[2])?;
            if std <= 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let cumulative = first_value(&args[3]).as_bool();
            Ok(Value::Number(if cumulative {
                stats::normal_cdf(x, mean, std)
            } else {
                stats::normal_pdf(x, mean, std)
            }))
        }
        "NORMSDIST" => {
            require_between(name, args, 1, 2)?;
            let x = first_number(name, &args[0])?;
            // The one-argument legacy form is cumulative; the modern
            // `NORM.S.DIST` takes the flag explicitly.
            let cumulative = args.get(1).is_none_or(|a| first_value(a).as_bool());
            Ok(Value::Number(if cumulative {
                stats::normal_cdf(x, 0.0, 1.0)
            } else {
                stats::normal_pdf(x, 0.0, 1.0)
            }))
        }
        "NORMINV" => {
            require_arity(name, args, 3)?;
            let p = first_number(name, &args[0])?;
            let mean = first_number(name, &args[1])?;
            let std = first_number(name, &args[2])?;
            if !(0.0..=1.0).contains(&p) || std <= 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::normal_inv(p, mean, std)))
        }
        "NORMSINV" => {
            require_arity(name, args, 1)?;
            let p = first_number(name, &args[0])?;
            if !(0.0..=1.0).contains(&p) {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::normal_inv(p, 0.0, 1.0)))
        }
        // The half-width of a confidence interval: the two-tailed critical
        // value times the standard error. `CONFIDENCE` is the pre-2010 name of
        // `CONFIDENCE.NORM`; both canonicalise to the same arm.
        "CONFIDENCE" | "CONFIDENCENORM" | "CONFIDENCET" => {
            require_arity(name, args, 3)?;
            let alpha = first_number(name, &args[0])?;
            let std = first_number(name, &args[1])?;
            let size = first_number(name, &args[2])?;
            // `CONFIDENCE.T` needs at least two observations, since it divides
            // by `n - 1` degrees of freedom; the normal form needs one.
            let least = if name == "CONFIDENCET" { 2.0 } else { 1.0 };
            if !(alpha > 0.0 && alpha < 1.0) || std <= 0.0 || size < least {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let critical = if name == "CONFIDENCET" {
                // `T.INV.2T(alpha, n - 1)`, the two-tailed inverse. The crate's
                // `studentt_inv` is the one-tailed form, so the lower tail is
                // taken and made positive.
                stats::studentt_inv(alpha / 2.0, size - 1.0).abs()
            } else {
                stats::normal_inv(1.0 - alpha / 2.0, 0.0, 1.0)
            };
            Ok(Value::Number(critical * std / size.sqrt()))
        }
        "LOGNORMDIST" => {
            require_between(name, args, 3, 4)?;
            let x = first_number(name, &args[0])?;
            let mean = first_number(name, &args[1])?;
            let std = first_number(name, &args[2])?;
            if std <= 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            // The three-argument legacy form is the cumulative one.
            let cumulative = args.get(3).is_none_or(|a| first_value(a).as_bool());
            Ok(Value::Number(if cumulative {
                stats::lognormal_cdf(x, mean, std)
            } else {
                stats::lognormal_pdf(x, mean, std)
            }))
        }
        "LOGINV" => {
            // The pre-2010 spelling of `LOGNORMINV`, with the same arguments.
            require_arity(name, args, 3)?;
            let p = first_number(name, &args[0])?;
            let mean = first_number(name, &args[1])?;
            let std = first_number(name, &args[2])?;
            if !(0.0..=1.0).contains(&p) || std <= 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::lognormal_inv(p, mean, std)))
        }
        "LOGNORMINV" => {
            require_arity(name, args, 3)?;
            let p = first_number(name, &args[0])?;
            let mean = first_number(name, &args[1])?;
            let std = first_number(name, &args[2])?;
            if !(0.0..=1.0).contains(&p) || std <= 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::lognormal_inv(p, mean, std)))
        }
        "TDIST" => {
            require_arity(name, args, 3)?;
            let x = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            let tails = first_number(name, &args[2])?;
            if x < 0.0 || dof < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let cdf = stats::studentt_cdf(x, dof);
            Ok(Value::Number(if tails == 1.0 {
                1.0 - cdf
            } else {
                (1.0 - cdf) * 2.0
            }))
        }
        "TDISTRT" => {
            require_arity(name, args, 2)?;
            let x = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            if x < 0.0 || dof < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(1.0 - stats::studentt_cdf(x, dof)))
        }
        // `T.DIST.2T`: the two-tailed probability, which is the right tail
        // doubled. A negative `x` is rejected, as in `TDIST`.
        "TDIST2T" => {
            require_arity(name, args, 2)?;
            let x = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            if x < 0.0 || dof < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number((1.0 - stats::studentt_cdf(x, dof)) * 2.0))
        }
        "TINV" => {
            require_arity(name, args, 2)?;
            let p = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            if dof < 1.0 || p <= 0.0 || p >= 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            // `T.INV` is the one-tailed inverse.
            Ok(Value::Number(stats::studentt_inv(p, dof)))
        }
        // `T.INV.2T`: the two-tailed inverse, which Excel defines as the
        // positive `x` with a two-tailed probability of `p`, so it is the
        // one-tailed inverse of `p / 2` taken in the upper tail.
        "TINV2T" => {
            require_arity(name, args, 2)?;
            let p = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            if dof < 1.0 || p <= 0.0 || p >= 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::studentt_inv(1.0 - p / 2.0, dof)))
        }
        // `CHIDIST` and its modern spelling `CHISQ.DIST` reach the same arm:
        // the optional cumulative flag, when false or absent, is the density.
        "CHIDIST" | "CHISQDIST" => {
            require_between(name, args, 2, 3)?;
            let x = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            match args.get(2) {
                Some(flag) if first_value(flag).as_bool() => {
                    Ok(Value::Number(stats::chisq_cdf(x, dof)))
                }
                // A missing or false flag is the density, which is what the
                // reference returns when `cumulative` is undefined.
                _ => Ok(Value::Number(stats::chisq_pdf(x, dof))),
            }
        }
        "CHIDISTRT" | "CHISQDISTRT" => {
            require_arity(name, args, 2)?;
            let x = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            Ok(Value::Number(1.0 - stats::chisq_cdf(x, dof)))
        }
        "CHIINV" => {
            require_between(name, args, 2, 2)?;
            let p = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            if !(0.0..=1.0).contains(&p) || dof < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::chisq_inv(p, dof)))
        }
        "CHIINVRT" => {
            require_arity(name, args, 2)?;
            let p = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            if !(0.0..=1.0).contains(&p) || dof < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::chisq_inv(1.0 - p, dof)))
        }
        // `CHISQ.INV` is the left-tail inverse, the same as `CHIINV`.
        "CHISQINV" => {
            require_arity(name, args, 2)?;
            let p = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            if !(0.0..=1.0).contains(&p) || dof < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::chisq_inv(p, dof)))
        }
        // `CHISQ.INV.RT` is the right-tail inverse, the same as `CHIINVRT`.
        "CHISQINVRT" => {
            require_arity(name, args, 2)?;
            let p = first_number(name, &args[0])?;
            let dof = first_number(name, &args[1])?;
            if !(0.0..=1.0).contains(&p) || dof < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::chisq_inv(1.0 - p, dof)))
        }
        "FDIST" => {
            require_between(name, args, 3, 4)?;
            let x = first_number(name, &args[0])?;
            let df1 = first_number(name, &args[1])?;
            let df2 = first_number(name, &args[2])?;
            match args.get(3) {
                Some(flag) if first_value(flag).as_bool() => {
                    Ok(Value::Number(stats::f_cdf(x, df1, df2)))
                }
                _ => Ok(Value::Number(stats::f_pdf(x, df1, df2))),
            }
        }
        "FDISTRT" => {
            require_arity(name, args, 3)?;
            let x = first_number(name, &args[0])?;
            let df1 = first_number(name, &args[1])?;
            let df2 = first_number(name, &args[2])?;
            Ok(Value::Number(1.0 - stats::f_cdf(x, df1, df2)))
        }
        "FINV" => {
            require_arity(name, args, 3)?;
            let p = first_number(name, &args[0])?;
            let df1 = first_number(name, &args[1])?;
            let df2 = first_number(name, &args[2])?;
            if p <= 0.0 || p > 1.0 || df1 < 1.0 || df2 < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::f_inv(p, df1, df2)))
        }
        "FINVRT" => {
            require_arity(name, args, 3)?;
            let p = first_number(name, &args[0])?;
            let df1 = first_number(name, &args[1])?;
            let df2 = first_number(name, &args[2])?;
            if !(0.0..=1.0).contains(&p) || df1 < 1.0 || df2 < 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::f_inv(1.0 - p, df1, df2)))
        }
        "BINOMDIST" => {
            require_arity(name, args, 4)?;
            let k = first_number(name, &args[0])?;
            let n = first_number(name, &args[1])?;
            let p = first_number(name, &args[2])?;
            let cumulative = first_value(&args[3]).as_bool();
            Ok(Value::Number(if cumulative {
                stats::binomial_cdf(k, n, p)
            } else {
                stats::binomial_pdf(k, n, p)
            }))
        }
        // `BINOM.DIST.RANGE`: the sum of the binomial density over an inclusive
        // interval of successes. The fourth argument is optional and defaults
        // to the third, so the three-argument form is a single term.
        "BINOMDISTRANGE" => {
            require_between(name, args, 3, 4)?;
            let trials = first_number(name, &args[0])?;
            let p = first_number(name, &args[1])?;
            let lower = first_number(name, &args[2])?;
            let upper = match args.get(3) {
                Some(a) => first_number(name, a)?,
                None => lower,
            };
            if trials < 0.0 || !(0.0..=1.0).contains(&p) || lower < 0.0 || upper < lower {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let mut result = 0.0;
            let mut k = lower;
            while k <= upper {
                result += stats::binomial_pdf(k, trials, p);
                k += 1.0;
            }
            Ok(Value::Number(result))
        }
        // `BINOM.INV` is the modern name for `CRITBINOM`.
        "CRITBINOM" | "BINOMINV" => {
            require_arity(name, args, 3)?;
            let n = first_number(name, &args[0])?;
            let p = first_number(name, &args[1])?;
            let alpha = first_number(name, &args[2])?;
            let mut x = 0.0;
            while x <= n {
                if stats::binomial_cdf(x, n, p) >= alpha {
                    return Ok(Value::Number(x));
                }
                x += 1.0;
            }
            Ok(Value::Number(x))
        }
        "POISSONDIST" => {
            require_arity(name, args, 3)?;
            let x = first_number(name, &args[0])?;
            let mean = first_number(name, &args[1])?;
            let cumulative = first_value(&args[2]).as_bool();
            Ok(Value::Number(if cumulative {
                stats::poisson_cdf(x, mean)
            } else {
                stats::poisson_pdf(x, mean)
            }))
        }
        "EXPONDIST" => {
            require_arity(name, args, 3)?;
            let x = first_number(name, &args[0])?;
            let lambda = first_number(name, &args[1])?;
            let cumulative = first_value(&args[2]).as_bool();
            Ok(Value::Number(if cumulative {
                stats::exponential_cdf(x, lambda)
            } else {
                stats::exponential_pdf(x, lambda)
            }))
        }
        "WEIBULLDIST" => {
            require_arity(name, args, 4)?;
            let x = first_number(name, &args[0])?;
            let alpha = first_number(name, &args[1])?;
            let beta = first_number(name, &args[2])?;
            let cumulative = first_value(&args[3]).as_bool();
            Ok(Value::Number(if cumulative {
                stats::weibull_cdf(x, beta, alpha)
            } else {
                stats::weibull_pdf(x, beta, alpha)
            }))
        }
        "GAMMADIST" => {
            require_arity(name, args, 4)?;
            let x = first_number(name, &args[0])?;
            let alpha = first_number(name, &args[1])?;
            let beta = first_number(name, &args[2])?;
            if x < 0.0 || alpha <= 0.0 || beta <= 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            let cumulative = first_value(&args[3]).as_bool();
            Ok(Value::Number(if cumulative {
                stats::gamma_cdf(x, alpha, beta)
            } else {
                stats::gamma_pdf(x, alpha, beta)
            }))
        }
        "GAMMAINV" => {
            require_arity(name, args, 3)?;
            let p = first_number(name, &args[0])?;
            let alpha = first_number(name, &args[1])?;
            let beta = first_number(name, &args[2])?;
            if !(0.0..=1.0).contains(&p) || alpha <= 0.0 || beta <= 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(stats::gamma_inv(p, alpha, beta)))
        }
        "BETADIST" => {
            require_between(name, args, 3, 6)?;
            let x = first_number(name, &args[0])?;
            let alpha = first_number(name, &args[1])?;
            let beta = first_number(name, &args[2])?;
            // The reference order is `cumulative, a, b`; a three-argument call
            // is the legacy cumulative form over the unit interval.
            let (cumulative, a, b) = if args.len() >= 4 {
                (
                    first_value(&args[3]).as_bool(),
                    args.get(4).map_or(Ok(0.0), |a| first_number(name, a))?,
                    args.get(5).map_or(Ok(1.0), |a| first_number(name, a))?,
                )
            } else {
                (true, 0.0, 1.0)
            };
            let scaled = (x - a) / (b - a);
            Ok(Value::Number(if cumulative {
                stats::beta_cdf(scaled, alpha, beta)
            } else {
                stats::beta_pdf(scaled, alpha, beta)
            }))
        }
        "BETAINV" => {
            require_between(name, args, 3, 5)?;
            let p = first_number(name, &args[0])?;
            let alpha = first_number(name, &args[1])?;
            let beta = first_number(name, &args[2])?;
            let a = args.get(3).map_or(Ok(0.0), |a| first_number(name, a))?;
            let b = args.get(4).map_or(Ok(1.0), |a| first_number(name, a))?;
            Ok(Value::Number(stats::beta_inv(p, alpha, beta) * (b - a) + a))
        }
        "HYPGEOMDIST" => {
            require_between(name, args, 4, 5)?;
            let k = first_number(name, &args[0])?;
            let n = first_number(name, &args[1])?;
            let m = first_number(name, &args[2])?;
            let big_n = first_number(name, &args[3])?;
            let cumulative = args.get(4).is_some_and(|a| first_value(a).as_bool());
            Ok(Value::Number(if cumulative {
                stats::hypgeom_cdf(k, n, m, big_n)
            } else {
                stats::hypgeom_pdf(k, n, m, big_n)
            }))
        }
        "NEGBINOMDIST" => {
            require_arity(name, args, 4)?;
            let f = first_number(name, &args[0])?;
            let s = first_number(name, &args[1])?;
            let p = first_number(name, &args[2])?;
            let cumulative = first_value(&args[3]).as_bool();
            Ok(Value::Number(if cumulative {
                stats::negbinom_cdf(f, s, p)
            } else {
                stats::negbinom_pdf(f, s, p)
            }))
        }
        "STANDARDIZE" => {
            require_arity(name, args, 3)?;
            let x = first_number(name, &args[0])?;
            let mean = first_number(name, &args[1])?;
            let std = first_number(name, &args[2])?;
            if std <= 0.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number((x - mean) / std))
        }
        "GAUSS" => {
            require_arity(name, args, 1)?;
            let z = first_number(name, &args[0])?;
            Ok(Value::Number(stats::normal_cdf(z, 0.0, 1.0) - 0.5))
        }
        "PHI" => {
            require_arity(name, args, 1)?;
            let x = first_number(name, &args[0])?;
            Ok(Value::Number(
                (-0.5 * x * x).exp() / 2.506_628_274_631_000_2,
            ))
        }
        "FISHER" => {
            require_arity(name, args, 1)?;
            let x = first_number(name, &args[0])?;
            if x <= -1.0 || x >= 1.0 {
                return Err(Error::Arithmetic(ValueError::NotANumber));
            }
            Ok(Value::Number(((1.0 + x) / (1.0 - x)).ln() / 2.0))
        }
        "FISHERINV" => {
            require_arity(name, args, 1)?;
            let y = first_number(name, &args[0])?;
            let e2y = (2.0 * y).exp();
            Ok(Value::Number((e2y - 1.0) / (e2y + 1.0)))
        }
        "CORREL" => {
            require_arity(name, args, 2)?;
            let a = numbers_of_arg(name, &args[0])?;
            let b = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::correl(&a, &b)))
        }
        "PEARSON" => {
            require_arity(name, args, 2)?;
            let a = numbers_of_arg(name, &args[0])?;
            let b = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::pearson(&a, &b)))
        }
        "RSQ" => {
            require_arity(name, args, 2)?;
            let ys = numbers_of_arg(name, &args[0])?;
            let xs = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::pearson(&ys, &xs).powi(2)))
        }
        "SLOPE" => {
            require_arity(name, args, 2)?;
            let ys = numbers_of_arg(name, &args[0])?;
            let xs = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::slope(&ys, &xs)))
        }
        "INTERCEPT" => {
            require_arity(name, args, 2)?;
            let ys = numbers_of_arg(name, &args[0])?;
            let xs = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::forecast(0.0, &ys, &xs)))
        }
        "STEYX" => {
            require_arity(name, args, 2)?;
            let ys = numbers_of_arg(name, &args[0])?;
            let xs = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::steyx(&ys, &xs)))
        }
        "FORECAST" => {
            require_arity(name, args, 3)?;
            let x = first_number(name, &args[0])?;
            let ys = numbers_of_arg(name, &args[1])?;
            let xs = numbers_of_arg(name, &args[2])?;
            Ok(Value::Number(stats::forecast(x, &ys, &xs)))
        }
        "COVAR" | "COVARIANCEP" | "COVARIANCE" => {
            require_arity(name, args, 2)?;
            let a = numbers_of_arg(name, &args[0])?;
            let b = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::covariance_population(&a, &b)))
        }
        "COVARIANCES" => {
            require_arity(name, args, 2)?;
            let a = numbers_of_arg(name, &args[0])?;
            let b = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::covariance_sample(&a, &b)))
        }
        "PERCENTRANKINC" | "PERCENTRANKEXC" | "PERCENTRANK" => {
            require_between(name, args, 2, 3)?;
            let mut array = numbers_of_arg(name, &args[0])?;
            let x = first_number(name, &args[1])?;
            let significance = args.get(2).map_or(Ok(3.0), |a| first_number(name, a))?;
            array.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let exclusive = name == "PERCENTRANKEXC";
            Ok(Value::Number(percent_rank(
                &array,
                x,
                significance,
                exclusive,
            )))
        }
        "VARA" | "VARPA" | "STDEVA" | "STDEVPA" => {
            let values = values_of(args);
            let defined: Vec<&Value> = values
                .iter()
                .filter(|v| !matches!(v, Value::Empty))
                .collect();
            if defined.is_empty() {
                return Err(Error::Arithmetic(ValueError::DivisionByZero));
            }
            let mean =
                defined.iter().map(|v| a_value_number(v)).sum::<f64>() / defined.len() as f64;
            let sigma: f64 = defined
                .iter()
                .map(|v| (a_value_number(v) - mean).powi(2))
                .sum();
            let population = name == "VARPA" || name == "STDEVPA";
            let variance = if population {
                sigma / defined.len() as f64
            } else {
                sigma / (defined.len() as f64 - 1.0)
            };
            Ok(Value::Number(if name == "STDEVA" || name == "STDEVPA" {
                variance.sqrt()
            } else {
                variance
            }))
        }
        "ZTEST" => {
            require_between(name, args, 2, 3)?;
            let values = numbers_of_arg(name, &args[0])?;
            let x = first_number(name, &args[1])?;
            let sigma = match args.get(2) {
                Some(arg) => Some(first_number(name, arg)?),
                None => None,
            };
            Ok(Value::Number(stats::ztest(&values, x, sigma)))
        }
        "TTEST" => {
            require_between(name, args, 2, 4)?;
            let a = numbers_of_arg(name, &args[0])?;
            let b = numbers_of_arg(name, &args[1])?;
            // The tails and type arguments are accepted but not used: the
            // reference ignores them, and this crate reproduces the reference.
            Ok(Value::Number(stats::ttest(&a, &b)))
        }
        "FTEST" => {
            require_arity(name, args, 2)?;
            let a = numbers_of_arg(name, &args[0])?;
            let b = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::ftest(&a, &b)))
        }
        // `CHISQ.TEST` is the modern name for `CHITEST`.
        "CHITEST" | "CHISQTEST" => {
            require_arity(name, args, 2)?;
            let a = numbers_of_arg(name, &args[0])?;
            let b = numbers_of_arg(name, &args[1])?;
            Ok(Value::Number(stats::chitest(&a, &b)))
        }
        "PROB" => {
            require_between(name, args, 3, 4)?;
            let xs = numbers_of_arg(name, &args[0])?;
            let probs = numbers_of_arg(name, &args[1])?;
            let lower = first_number(name, &args[2])?;
            let upper = args.get(3).map_or(Ok(lower), |a| first_number(name, a))?;
            Ok(Value::Number(stats::prob(&xs, &probs, lower, upper)))
        }
        other => Err(Error::UnknownFunction(other.to_string())),
    }
}

/// Every value in every argument, flattened, for the `*A` functions.
///
/// Unlike `numbers_of`, this keeps text, logicals and blanks, because the
/// `*A` family counts them: a logical `TRUE` contributes one and any other
/// non-number contributes zero to the total.
fn values_of(args: &[Arg]) -> Vec<Value> {
    let mut out = Vec::new();
    for arg in args {
        out.extend(arg.values.iter().cloned());
    }
    out
}

/// One value's contribution to an `AVERAGEA` sum, where a logical is one and
/// text is zero.
fn a_value_number(value: &Value) -> f64 {
    match value {
        Value::Number(n) => *n,
        Value::Bool(true) => 1.0,
        _ => 0.0,
    }
}

/// The numbers in one argument, for the regression and covariance functions.
///
/// A reference contributes its numeric cells; a scalar contributes itself.
/// Text and empty cells are skipped, which is what the reference does when it
/// flattens an array through `parseNumberArray`.
fn numbers_of_arg(name: &str, arg: &Arg) -> Result<Vec<f64>, Error> {
    let mut out = Vec::new();
    for value in &arg.values {
        match value {
            Value::Number(n) => out.push(*n),
            Value::Bool(b) => out.push(if *b { 1.0 } else { 0.0 }),
            Value::Empty => {}
            Value::Text(t) => {
                if let Ok(n) = t.trim().parse::<f64>() {
                    out.push(n);
                }
            }
        }
    }
    if out.is_empty() {
        return Err(Error::TypeError {
            operation: name.to_string(),
            found: Value::Empty,
        });
    }
    Ok(out)
}

/// The inclusive or exclusive percentage rank, as `PERCENTRANK` reports it.
///
/// `array` is sorted ascending. The exclusive form ranks against `n + 1`
/// positions so neither end reaches zero or one; the inclusive form uses
/// `n - 1`. Both truncate toward zero at the requested significance.
fn percent_rank(array: &[f64], x: f64, significance: f64, exclusive: bool) -> f64 {
    let n = array.len();
    // `array` is sorted ascending, so equal values are adjacent: a value is
    // new when it differs from the last kept one, and its run starts at the
    // index where it was first seen. Recording that index once keeps the
    // lookup below linear instead of re-scanning `array` per unique value.
    let mut uniques: Vec<f64> = Vec::new();
    let mut first_index: Vec<usize> = Vec::new();
    for (index, value) in array.iter().enumerate() {
        if uniques.last() != Some(value) {
            uniques.push(*value);
            first_index.push(index);
        }
    }
    let m = uniques.len();
    let power = 10f64.powf(significance);
    let mut result = 0.0;
    let mut i = 0;
    while i < m {
        if x == uniques[i] {
            let index = first_index[i];
            result = if exclusive {
                (index + 1) as f64 / (n + 1) as f64
            } else {
                index as f64 / (n - 1) as f64
            };
            break;
        } else if x >= uniques[i] && (i + 1 >= m || x < uniques[i + 1]) {
            let index = first_index[i];
            let next = if i + 1 < m {
                uniques[i + 1]
            } else {
                uniques[i]
            };
            let fraction = (x - uniques[i]) / (next - uniques[i]);
            result = if exclusive {
                (index + 1) as f64 + fraction
            } else {
                index as f64 + fraction
            } / if exclusive {
                (n + 1) as f64
            } else {
                (n - 1) as f64
            };
            break;
        }
        i += 1;
    }
    (result * power).floor() / power
}

enum RoundMode {
    Half,
    Up,
    Down,
}

fn round_to(value: f64, digits: i32, mode: RoundMode) -> f64 {
    let factor = 10f64.powi(digits);
    let scaled = value * factor;
    let rounded = match mode {
        RoundMode::Half => scaled.round(),
        RoundMode::Up => {
            if scaled >= 0.0 {
                scaled.ceil()
            } else {
                scaled.floor()
            }
        }
        RoundMode::Down => {
            if scaled >= 0.0 {
                scaled.floor()
            } else {
                scaled.ceil()
            }
        }
    };
    rounded / factor
}

/// `CEILING.MATH` and `FLOOR.MATH`: round to a multiple of `significance`.
///
/// The sign of `significance` is ignored, so unlike the bare `CEILING`/`FLOOR`
/// there is no "positive value with a negative significance" error. `mode`
/// decides the direction for a negative value: zero rounds away from zero,
/// anything else rounds toward zero. `up` selects which of the two functions
/// this is.
fn round_multiple(value: f64, significance: f64, mode: f64, up: bool) -> f64 {
    if significance == 0.0 {
        return 0.0;
    }
    let step = significance.abs();
    let away = mode == 0.0;
    let rounds_up = if value >= 0.0 || away { up } else { !up };
    let quotient = value / step;
    let n = if rounds_up {
        quotient.ceil()
    } else {
        quotient.floor()
    };
    n * step
}

fn min_max(name: &str, args: &[Arg], want_min: bool) -> Result<Value, Error> {
    let values = numbers_of(name, args)?;
    // With no numbers Excel returns 0 rather than an error or a blank.
    if values.is_empty() {
        return Ok(Value::Number(0.0));
    }
    let mut best = values[0];
    for v in &values[1..] {
        if (want_min && *v < best) || (!want_min && *v > best) {
            best = *v;
        }
    }
    Ok(Value::Number(best))
}

fn unary_number(name: &str, args: &[Arg], f: impl Fn(f64) -> f64) -> Result<Value, Error> {
    require_arity(name, args, 1)?;
    Ok(Value::Number(f(first_number(name, &args[0])?)))
}

fn unary_text(name: &str, args: &[Arg], f: impl Fn(&str) -> String) -> Result<Value, Error> {
    require_arity(name, args, 1)?;
    Ok(Value::Text(f(&first_text(&args[0]))))
}

fn number_and_digits(name: &str, args: &[Arg]) -> Result<(f64, i32), Error> {
    require_arity(name, args, 2)?;
    let value = first_number(name, &args[0])?;
    let digits = first_number(name, &args[1])?;
    Ok((value, digits as i32))
}

fn first_number(name: &str, arg: &Arg) -> Result<f64, Error> {
    let value = arg.values.first().cloned().unwrap_or(Value::Empty);
    value.as_number().map_err(|_| Error::TypeError {
        operation: name.to_string(),
        found: value,
    })
}

/// Whether every criteria range matches at one index.
///
/// `args` is the flat list of criteria ranges and their criteria in pairs:
/// range, criterion, range, criterion. All of the ranges are read at the same
/// index, which is how a spreadsheet aligns them.
fn every_criterion_matches(args: &[Arg], index: usize) -> bool {
    let mut pair = 0;
    while pair + 1 < args.len() {
        let range = &args[pair];
        let criterion_arg = &args[pair + 1];
        let Some(value) = range.values.get(index) else {
            return false;
        };
        if !criterion(value, criterion_arg) {
            return false;
        }
        pair += 2;
    }
    true
}

/// The position of `needle` in `haystack`, treating `*` as any run of
/// characters and `?` as any single character.
///
/// Returns a zero-based character offset. The needle is already lowercased by
/// the caller, and so is the haystack.
fn search_index(haystack: &str, needle: &str) -> Option<usize> {
    let text: Vec<char> = haystack.chars().collect();
    let pattern: Vec<char> = needle.chars().collect();
    // An empty needle is found at the start.
    if pattern.is_empty() {
        return Some(0);
    }
    (0..=text.len().saturating_sub(1)).find(|&start| {
        let mut ti = start;
        let mut pi = 0;
        while pi < pattern.len() {
            match pattern[pi] {
                '*' => {
                    // Try every length for the star, shortest first, so the
                    // earliest match wins.
                    if pi + 1 == pattern.len() {
                        return true;
                    }
                    let mut skip = ti;
                    while skip <= text.len() {
                        if wildcard_match_from(&text, skip, &pattern[pi + 1..]) {
                            return true;
                        }
                        skip += 1;
                    }
                    return false;
                }
                '?' => {
                    if ti >= text.len() {
                        return false;
                    }
                    ti += 1;
                    pi += 1;
                }
                c => {
                    if ti >= text.len() || text[ti] != c {
                        return false;
                    }
                    ti += 1;
                    pi += 1;
                }
            }
        }
        true
    })
}

/// Whether the pattern matches the text starting at `from`.
fn wildcard_match_from(text: &[char], from: usize, pattern: &[char]) -> bool {
    let mut ti = from;
    let mut pi = 0;
    while pi < pattern.len() {
        match pattern[pi] {
            '*' => {
                if pi + 1 == pattern.len() {
                    return true;
                }
                let mut skip = ti;
                while skip <= text.len() {
                    if wildcard_match_from(text, skip, &pattern[pi + 1..]) {
                        return true;
                    }
                    skip += 1;
                }
                return false;
            }
            '?' => {
                if ti >= text.len() {
                    return false;
                }
                ti += 1;
                pi += 1;
            }
            c => {
                if ti >= text.len() || text[ti] != c {
                    return false;
                }
                ti += 1;
                pi += 1;
            }
        }
    }
    ti == text.len()
}

/// Upper-cases the first letter of every word and lower-cases the rest.
///
/// A word begins at the start of the string and after any character that is
/// neither a letter nor a digit, so `"o'brien"` becomes `"O'Brien"`.
fn proper_case(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut at_word_start = true;
    for ch in text.chars() {
        if at_word_start {
            out.extend(ch.to_uppercase());
        } else {
            out.extend(ch.to_lowercase());
        }
        at_word_start = !ch.is_alphanumeric();
    }
    out
}

/// The largest value the bitwise functions accept, `2^48 - 1`.
const MAX_BIT_VALUE: u64 = (1 << 48) - 1;

/// The factorial of a non-negative integer, truncated, as a float.
///
/// Shared by `FACT`, `PERMUT`, `COMBIN` and `MULTINOMIAL`, which all need it.
fn factorial(n: f64) -> f64 {
    let n = n.trunc();
    if n < 0.0 {
        return f64::NAN;
    }
    let mut out = 1.0;
    let mut i = 2.0;
    while i <= n {
        out *= i;
        i += 1.0;
    }
    out
}

/// `n` choose `k`, computed as a product of ratios so it does not overflow the
/// factorial of a moderately large `n`.
fn binomial(n: f64, k: f64) -> f64 {
    let n = n.trunc();
    let k = k.trunc().min(n - k.trunc());
    if k < 0.0 {
        return f64::NAN;
    }
    let mut out = 1.0;
    let mut i = 0.0;
    while i < k {
        out = out * (n - i) / (i + 1.0);
        i += 1.0;
    }
    out.round()
}

/// The sample variance, dividing by `n - 1`. An empty or single-value input has
/// no sample variance, which the reference reports as zero.
pub(crate) fn sample_variance(values: &[f64]) -> f64 {
    if values.len() < 2 {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let sum: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
    sum / (values.len() as f64 - 1.0)
}

/// The population variance, dividing by `n`.
pub(crate) fn population_variance(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let sum: f64 = values.iter().map(|v| (v - mean).powi(2)).sum();
    sum / values.len() as f64
}

/// The excess kurtosis, using the same sample-based formula as the reference:
/// the fourth central moment is divided by the sample standard deviation to the
/// fourth, not the population one.
fn kurtosis(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    let sigma: f64 =
        values.iter().map(|v| (v - mean).powi(4)).sum::<f64>() / sample_variance(values).powi(2);
    n * (n + 1.0) / ((n - 1.0) * (n - 2.0) * (n - 3.0)) * sigma
        - 3.0 * (n - 1.0).powi(2) / ((n - 2.0) * (n - 3.0))
}

/// The skewness of a sample. The sample form (`SKEW`) divides by the sample
/// standard deviation and corrects for the sample size; the population form
/// (`SKEWP`) uses the population moments.
fn skewness(values: &[f64], sample: bool) -> f64 {
    let n = values.len() as f64;
    let mean = values.iter().sum::<f64>() / n;
    if sample {
        let sigma: f64 = values.iter().map(|v| (v - mean).powi(3)).sum::<f64>();
        n * sigma / ((n - 1.0) * (n - 2.0) * sample_variance(values).powf(1.5))
    } else {
        let m2: f64 = values.iter().map(|v| (v - mean).powi(2)).sum::<f64>() / n;
        let m3: f64 = values.iter().map(|v| (v - mean).powi(3)).sum::<f64>() / n;
        m3 / m2.powf(1.5)
    }
}

/// The most common value, breaking a tie towards the smallest, which is what
/// `MODESNGL` does. An empty input has no mode and is reported as an error by
/// the caller through the returned NaN.
fn mode_single(values: &[f64]) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    let mut best = values[0];
    let mut best_count = 0usize;
    for candidate in values {
        let count = values.iter().filter(|v| *v == candidate).count();
        if count > best_count || (count == best_count && *candidate < best) {
            best = *candidate;
            best_count = count;
        }
    }
    best
}

/// The numbers of one argument, sorted ascending.
fn sorted_values(arg: &Arg) -> Vec<f64> {
    let mut values = arg
        .values
        .iter()
        .filter_map(|v| v.as_number().ok())
        .collect::<Vec<f64>>();
    values.sort_by(f64::total_cmp);
    values
}

/// The `k`th value from the top (`LARGE`) or the bottom (`SMALL`) of a sorted
/// list.
///
/// The reference returns an error for a rank below one or beyond the number of
/// values, rather than a blank.
fn rank_value(from_top: bool, sorted: &[f64], k: f64) -> Result<f64, Error> {
    if k < 1.0 || k > sorted.len() as f64 {
        return Err(Error::Arithmetic(ValueError::NotANumber));
    }
    let index = if from_top {
        sorted.len() - k as usize
    } else {
        k as usize - 1
    };
    Ok(sorted[index])
}

/// `PERCENTILE`, with the inclusive and exclusive forms selected by `exclusive`.
fn percentile_value(exclusive: bool, sorted: &[f64], k: f64) -> Result<f64, Error> {
    if sorted.is_empty() {
        return Err(Error::Arithmetic(ValueError::NotANumber));
    }
    let n = sorted.len() as f64;
    let l = if exclusive {
        // The exclusive form shrinks the range by one at each end, so the
        // extreme percentiles have no answer.
        if k < 1.0 / (n + 1.0) || k > 1.0 - 1.0 / (n + 1.0) {
            return Err(Error::Arithmetic(ValueError::NotANumber));
        }
        k * (n + 1.0) - 1.0
    } else {
        if !(0.0..=1.0).contains(&k) {
            return Err(Error::Arithmetic(ValueError::NotANumber));
        }
        k * (n - 1.0)
    };
    Ok(interpolate(sorted, l))
}

/// `QUARTILE`, with the inclusive and exclusive forms selected by `exclusive`.
fn quartile_value(exclusive: bool, sorted: &[f64], quart: f64) -> Result<f64, Error> {
    let fraction = match quart {
        0.0 => 0.0,
        1.0 => 0.25,
        2.0 => 0.5,
        3.0 => 0.75,
        4.0 => 1.0,
        _ => return Err(Error::Arithmetic(ValueError::NotANumber)),
    };
    // The exclusive form has no answer at the extremes.
    if exclusive && (quart == 0.0 || quart == 4.0) {
        return Err(Error::Arithmetic(ValueError::NotANumber));
    }
    percentile_value(exclusive, sorted, fraction)
}

/// The linear interpolation used by the percentile and quartile functions.
fn interpolate(sorted: &[f64], l: f64) -> f64 {
    let floor = l.floor();
    if floor < 0.0 {
        return sorted[0];
    }
    let index = floor as usize;
    if l == floor || index + 1 >= sorted.len() {
        return sorted[index.min(sorted.len() - 1)];
    }
    sorted[index] + (l - floor) * (sorted[index + 1] - sorted[index])
}

/// The numbers `AVERAGEA`, `MAXA` and `MINA` act on: a reference contributes
/// text as zero and a logical as one or zero, where the plain forms skip them.
fn values_of_a(name: &str, args: &[Arg]) -> Vec<f64> {
    let _ = name;
    let mut out = Vec::new();
    for arg in args {
        for value in &arg.values {
            match value {
                Value::Number(n) => out.push(*n),
                Value::Bool(b) => out.push(if *b { 1.0 } else { 0.0 }),
                Value::Empty => {}
                // Text is zero when read from a cell; written directly it is
                // parsed like a number, which is how a spreadsheet coerces it.
                Value::Text(t) => {
                    if arg.from_ref {
                        out.push(0.0);
                    } else if let Ok(n) = t.trim().parse::<f64>() {
                        out.push(n);
                    }
                }
            }
        }
    }
    out
}

/// The gamma function, using the same rational approximation the reference's
/// `jStat.gammafn` uses, so the two agree to the last digit.
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

/// The natural logarithm of the gamma function, using the same series the
/// reference's `jStat.gammaln` uses.
pub(crate) fn gamma_ln(x: f64) -> f64 {
    const COF: [f64; 6] = [
        76.180_091_729_471_46,
        -86.505_320_329_416_77,
        24.014_098_240_830_91,
        -1.231_739_572_450_155,
        1.208_650_973_866_179e-3,
        -0.539_523_938_495_3e-5,
    ];
    let mut ser = 1.000_000_000_190_015;
    let xx = x;
    let mut y = x;
    let mut tmp = xx + 5.5;
    tmp -= (xx + 0.5) * tmp.ln();
    for c in COF {
        y += 1.0;
        ser += c / y;
    }
    (2.506_628_274_631_000_5 * ser / xx).ln() - tmp
}

/// The error function, using the same Chebyshev expansion the reference's
/// `jStat.erf` uses, so the two agree to the last digit.
pub(crate) fn erf(x: f64) -> f64 {
    const COF: [f64; 28] = [
        -1.302_653_719_781_709_4,
        6.419_697_923_564_903e-1,
        1.947_647_320_418_583_6e-2,
        -9.561_514_786_808_63e-3,
        -9.465_953_444_820_36e-4,
        3.668_394_978_527_61e-4,
        4.252_332_480_690_7e-5,
        -2.027_857_811_253_4e-5,
        -1.624_290_004_647e-6,
        1.303_655_835_58e-6,
        1.562_644_172_2e-8,
        -8.523_809_591_5e-8,
        6.529_054_439e-9,
        5.059_343_495e-9,
        -9.913_641_56e-10,
        -2.273_651_22e-10,
        9.646_791_1e-11,
        2.394_038e-12,
        -6.886_027e-12,
        8.944_87e-13,
        3.130_92e-13,
        -1.127_08e-13,
        3.81e-16,
        7.106e-15,
        -1.523e-15,
        -9.4e-17,
        1.21e-16,
        -2.8e-17,
    ];
    let (x, is_neg) = if x < 0.0 { (-x, true) } else { (x, false) };
    let t = 2.0 / (2.0 + x);
    let ty = 4.0 * t - 2.0;
    let mut d = 0.0;
    let mut dd = 0.0;
    for c in COF.iter().skip(1).rev() {
        let tmp = d;
        d = ty * d - dd + c;
        dd = tmp;
    }
    let res = t * (-x * x + 0.5 * (COF[0] + ty * d) - dd).exp();
    if is_neg { res - 1.0 } else { 1.0 - res }
}

/// A roman numeral for a number, using the subtractive forms. The reference
/// rounds toward zero and repeats `M` past 3999 rather than refusing.
fn to_roman(number: f64) -> String {
    let n = number.trunc() as i64;
    if n <= 0 {
        return String::new();
    }
    let table: [(i64, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut rest = n;
    let mut out = String::new();
    for (value, symbol) in table {
        while rest >= value {
            out.push_str(symbol);
            rest -= value;
        }
    }
    out
}

/// Parses a roman numeral, rejecting a string that is not one. The pattern is
/// the reference's: only the standard subtractive forms are accepted.
fn from_roman(text: &str) -> Option<i64> {
    // The reference matches its pattern against the text as written, so a
    // lower-case numeral is rejected. The canonical form is upper-case.
    let upper = text.to_string();
    if upper.is_empty() {
        return Some(0);
    }
    let chars: Vec<char> = upper.chars().collect();
    if !chars
        .iter()
        .all(|c| matches!(c, 'M' | 'D' | 'C' | 'L' | 'X' | 'V' | 'I'))
    {
        return None;
    }
    let value = |c: char| match c {
        'M' => 1000,
        'D' => 500,
        'C' => 100,
        'L' => 50,
        'X' => 10,
        'V' => 5,
        _ => 1,
    };
    // A numeral is subtractive where a smaller symbol precedes a larger one.
    let mut total = 0;
    let mut i = 0;
    while i < chars.len() {
        let current = value(chars[i]);
        let next = chars.get(i + 1).map(|c| value(*c));
        if let Some(next) = next
            && current < next
        {
            total += next - current;
            i += 2;
            continue;
        }
        total += current;
        i += 1;
    }
    // Round-trip check: only a canonical numeral is accepted, so `IIII` and
    // `MMMM` are rejected the way the reference's pattern rejects them.
    if to_roman(total as f64) == upper {
        Some(total)
    } else {
        None
    }
}

/// Parses a string in a base from 2 to 36 into an integer, or `None` when a
/// character is not a digit of that base.
fn parse_radix(text: &str, radix: u32) -> Option<u64> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Some(0);
    }
    let mut out: u64 = 0;
    for c in trimmed.chars() {
        let digit = c.to_digit(radix)?;
        out = out
            .checked_mul(u64::from(radix))?
            .checked_add(u64::from(digit))?;
    }
    Some(out)
}

/// Renders a number in a base from 2 to 36, ignoring any fractional part.
fn radix_digits(number: f64, radix: u32) -> String {
    let mut n = number.trunc() as i64;
    if n == 0 {
        return "0".to_string();
    }
    let negative = n < 0;
    n = n.abs();
    let mut digits = Vec::new();
    while n > 0 {
        let digit = (n % i64::from(radix)) as u32;
        digits.push(std::char::from_digit(digit, radix).unwrap_or('0'));
        n /= i64::from(radix);
    }
    if negative {
        digits.push('-');
    }
    digits.iter().rev().collect()
}

/// Interprets a fixed-width field as a signed number: the reference treats a
/// ten-character string whose leading digit is at or above half the base as a
/// negative number in two's complement.
fn signed_field(value: u64, text: &str, radix: u32, width: usize) -> f64 {
    let chars = text.trim().chars().count();
    let leading = text.trim().chars().next().and_then(|c| c.to_digit(radix));
    let modulus = u64::from(radix).pow(width as u32);
    if chars == width && leading.is_some_and(|d| u64::from(d) >= u64::from(radix) / 2) {
        value as f64 - modulus as f64
    } else {
        value as f64
    }
}

/// Reads a fixed-width field in a base and returns it as a signed number.
///
/// The reference rejects a field that is empty, longer than ten characters or
/// outside the base's alphabet, and the caller turns `None` into an error.
fn parse_radix_field(text: &str, radix: u32) -> Option<f64> {
    let trimmed = text.trim();
    let length = trimmed.chars().count();
    if length == 0 || length > 10 || !trimmed.chars().all(|c| c.is_digit(radix)) {
        return None;
    }
    let unsigned = parse_radix(trimmed, radix)?;
    Some(signed_field(unsigned, trimmed, radix, 10))
}

/// `BIN2HEX` and its siblings: the value of a source-base field, written out in
/// a target base.
///
/// A negative result keeps the ten-character field shape, so its sign bit is
/// still visible; the optional `places` argument only pads a non-negative
/// result, and a field wider than `places` is an error rather than a truncation.
/// `range` is the interval the target base can hold, which the reference checks
/// for the conversions into binary and octal.
fn convert_radix_field(
    name: &str,
    args: &[Arg],
    source: u32,
    target: u32,
    range: Option<(f64, f64)>,
) -> Result<Value, Error> {
    require_between(name, args, 1, 2)?;
    let text = first_text(&args[0]);
    let value =
        parse_radix_field(&text, source).ok_or(Error::Arithmetic(ValueError::NotANumber))?;
    if let Some((low, high)) = range
        && (value < low || value > high)
    {
        return Err(Error::Arithmetic(ValueError::NotANumber));
    }
    if value < 0.0 {
        let modulus = f64::from(target).powi(10);
        let digits = radix_digits(modulus + value, target);
        return Ok(Value::Text(format!("{digits:0>10}")));
    }
    let digits = radix_digits(value, target);
    let Some(arg) = args.get(1) else {
        return Ok(Value::Text(digits));
    };
    let places = first_number(name, arg)?;
    if places < 0.0 {
        return Err(Error::Arithmetic(ValueError::NotANumber));
    }
    let places = places.trunc() as usize;
    if places < digits.len() {
        return Err(Error::Arithmetic(ValueError::NotANumber));
    }
    Ok(Value::Text(format!("{digits:0>places$}")))
}

/// `DEC2BIN` and its siblings: a decimal number in a target base, as a
/// ten-character field when negative. The two limits are the base's own.
fn decimal_to_radix(
    name: &str,
    args: &[Arg],
    radix: u32,
    low: f64,
    high: f64,
) -> Result<Value, Error> {
    require_between(name, args, 1, 2)?;
    let number = first_number(name, &args[0])?;
    if number < -low || number > high {
        return Err(Error::Arithmetic(ValueError::NotANumber));
    }
    let text = if number < 0.0 {
        // A negative value is written as the base's modulus plus the number.
        let modulus = f64::from(radix).powi(10);
        let complement = (modulus + number).trunc();
        let digits = radix_digits(complement, radix);
        format!("{digits:0>10}")
    } else {
        radix_digits(number, radix)
    };
    let places = match args.get(1) {
        Some(arg) => first_number(name, arg)? as usize,
        None => 0,
    };
    Ok(Value::Text(format!(
        "{text:0>width$}",
        width = places.max(text.len())
    )))
}

/// A bitwise operand: a non-negative integer at most `2^48 - 1`.
fn bit_operand(name: &str, arg: &Arg) -> Result<u64, Error> {
    let value = first_number(name, arg)?;
    if value < 0.0 || value.fract() != 0.0 || value > MAX_BIT_VALUE as f64 {
        return Err(Error::Arithmetic(ValueError::NotANumber));
    }
    Ok(value as u64)
}

/// Formats a number as currency: a leading dollar sign, thousands separators
/// and the given number of decimals, with a negative wrapped in parentheses.
fn format_currency(value: f64, digits: i32) -> String {
    let body = format!("{:.digits$}", value.abs(), digits = digits.max(0) as usize);
    let (whole, fraction) = match body.split_once('.') {
        Some((w, f)) => (group_thousands(w), format!(".{f}")),
        None => (group_thousands(&body), String::new()),
    };
    if value < 0.0 {
        format!("$({whole}{fraction})")
    } else {
        format!("${whole}{fraction}")
    }
}

/// Formats a number the way `FIXED` does: a fixed number of decimals, with
/// thousands separators unless `no_commas`.
fn format_fixed(value: f64, digits: i32, no_commas: bool) -> String {
    let body = format!("{:.digits$}", value, digits = digits.max(0) as usize);
    let (sign, rest) = match body.strip_prefix('-') {
        Some(r) => ("-", r.to_string()),
        None => ("", body),
    };
    let (whole, fraction) = match rest.split_once('.') {
        Some((w, f)) => (w.to_string(), format!(".{f}")),
        None => (rest, String::new()),
    };
    let whole = if no_commas {
        whole
    } else {
        group_thousands(&whole)
    };
    format!("{sign}{whole}{fraction}")
}

/// Parses a number written with an explicit decimal and group separator, the
/// way `NUMBERVALUE` does. Each separator is replaced once, matching the
/// reference, which uses `String.replace` with a string argument.
fn parse_number_value(text: &str, decimal_sep: &str, group_sep: &str) -> Option<f64> {
    let mut cleaned = if decimal_sep.is_empty() {
        text.to_string()
    } else {
        text.replacen(decimal_sep, ".", 1)
    };
    if !group_sep.is_empty() {
        cleaned = cleaned.replacen(group_sep, "", 1);
    }
    cleaned.trim().parse::<f64>().ok()
}

/// The greatest common divisor of two integers, always non-negative.
fn gcd_i64(a: i64, b: i64) -> i64 {
    let (mut a, mut b) = (a.abs(), b.abs());
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

/// Applies an integer operation pairwise across every number in the arguments.
fn integer_list(
    name: &str,
    args: &[Arg],
    combine: impl Fn(i64, i64) -> i64,
) -> Result<Value, Error> {
    require_at_least(name, args, 1)?;
    let numbers = numbers_of(name, args)?;
    if numbers.is_empty() {
        return Ok(Value::Number(0.0));
    }
    let mut acc = numbers[0].trunc() as i64;
    for value in &numbers[1..] {
        acc = combine(acc, value.trunc() as i64);
    }
    Ok(Value::Number(acc as f64))
}

/// Formats a number with a spreadsheet number format.
///
/// Supports the subset that covers the common cases: a `%` suffix, a thousands
/// separator, a fixed count of decimal places from the `0` placeholders after
/// the decimal point, and literal text. Returns `None` for a format this does
/// not understand, so the caller can report an error rather than guess.
fn format_number(value: f64, format: &str) -> Option<String> {
    let percent = format.contains('%');
    let scaled = if percent { value * 100.0 } else { value };

    let integer_part = format.split('.').next().unwrap_or(format);
    let use_thousands = integer_part.contains(',');

    let decimals = match format.split_once('.') {
        Some((_, after)) => after.chars().take_while(|c| *c == '0' || *c == '#').count(),
        None => 0,
    };

    let body = format!("{scaled:.decimals$}");
    let (sign, digits) = match body.strip_prefix('-') {
        Some(rest) => ("-", rest.to_string()),
        None => ("", body),
    };
    let (whole, fraction) = match digits.split_once('.') {
        Some((w, f)) => (w.to_string(), f.to_string()),
        None => (digits, String::new()),
    };

    let whole = if use_thousands {
        group_thousands(&whole)
    } else {
        whole
    };

    let mut out = String::new();
    out.push_str(sign);
    out.push_str(&whole);
    if !fraction.is_empty() {
        out.push('.');
        out.push_str(&fraction);
    }
    if percent {
        out.push('%');
    }
    Some(out)
}

/// Inserts thousands separators into a run of digits.
fn group_thousands(digits: &str) -> String {
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let len = digits.len();
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (len - index).is_multiple_of(3) {
            out.push(',');
        }
        out.push(ch);
    }
    out
}

/// The numbers `SUMIF` and `AVERAGEIF` should act on.
///
/// The first argument is tested against the criterion and the third holds the
/// numbers, and the two are aligned by position: the value summed for a match is
/// the one at the same index in the sum range. When the third argument is left
/// out the tested range is summed, which is the spreadsheet default and not the
/// same as summing the criterion.
fn matching_values(args: &[Arg]) -> Vec<f64> {
    let tested = &args[0];
    let sum_range = args.get(2).unwrap_or(&args[0]);
    tested
        .values
        .iter()
        .enumerate()
        .filter(|(_, value)| criterion(value, &args[1]))
        .filter_map(|(index, _)| sum_range.values.get(index).and_then(|v| v.as_number().ok()))
        .collect()
}

/// Requires an argument count within an inclusive range.
fn require_between(name: &str, args: &[Arg], minimum: usize, maximum: usize) -> Result<(), Error> {
    if args.len() < minimum || args.len() > maximum {
        return Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: "2 or 3",
        });
    }
    Ok(())
}

/// The first value of an argument, or `Empty`.
fn first_value(arg: &Arg) -> Value {
    arg.values.first().cloned().unwrap_or(Value::Empty)
}

/// A database or criteria argument as a row-major grid and its column count.
///
/// The `D...` functions need the shape of both grids, and a scalar argument has
/// none, so one is a type error rather than a one-cell grid.
fn database_grid(name: &str, arg: &Arg) -> Result<(Vec<Value>, usize), Error> {
    match arg.shape() {
        Some((_, width)) => Ok((arg.values.clone(), width)),
        None => Err(Error::TypeError {
            operation: name.to_string(),
            found: first_value(arg),
        }),
    }
}

/// Whether a value satisfies a criterion that may be text, a number, or a
/// comparison such as `">3"`.
///
/// The operators are the spreadsheet ones: `=`, `<>`, `<`, `<=`, `>`, `>=`. A
/// bare value means equality. A numeric criterion is matched against numbers
/// only, so `">2"` does not match a text cell. A text criterion is matched
/// case-insensitively and understands the `*` and `?` wildcards, with `~`
/// escaping the next character.
fn criterion(value: &Value, criterion_arg: &Arg) -> bool {
    // A logical criterion, written as `TRUE`, matches logical cells.
    if let Some(Value::Bool(want)) = criterion_arg.values.first() {
        return matches!(value, Value::Bool(have) if have == want);
    }

    let raw = first_text(criterion_arg);
    let (operator, rest) = split_operator(&raw);
    let rest = rest.trim();

    // A numeric target compares numerically. Only numeric cells can satisfy it:
    // text and blanks are never "greater than 2".
    if let Ok(target) = rest.parse::<f64>() {
        let number = match value {
            Value::Number(n) => Some(*n),
            _ => None,
        };
        return match operator {
            "=" => number == Some(target),
            "<>" => number != Some(target),
            "<" => number.is_some_and(|n| n < target),
            "<=" => number.is_some_and(|n| n <= target),
            ">" => number.is_some_and(|n| n > target),
            ">=" => number.is_some_and(|n| n >= target),
            _ => false,
        };
    }

    // A text target matches text cells only, so `COUNTIF(range,"*2*")` does not
    // count the number 2. Matching is case-insensitive and wildcards apply.
    let text = match value {
        Value::Text(t) => Some(t.as_str()),
        _ => None,
    };
    match operator {
        "=" => text.is_some_and(|t| wildcard_match(t, rest)),
        "<>" => !text.is_some_and(|t| wildcard_match(t, rest)),
        "<" => text.is_some_and(|t| t.to_lowercase() < rest.to_lowercase()),
        "<=" => text.is_some_and(|t| t.to_lowercase() <= rest.to_lowercase()),
        ">" => text.is_some_and(|t| t.to_lowercase() > rest.to_lowercase()),
        ">=" => text.is_some_and(|t| t.to_lowercase() >= rest.to_lowercase()),
        _ => false,
    }
}

/// Splits a criterion into its comparison operator and the value to compare
/// against. A criterion with no operator is an equality test.
pub(crate) fn split_operator(raw: &str) -> (&str, &str) {
    match raw.as_bytes().first() {
        Some(b'<') if raw.starts_with("<=") => ("<=", &raw[2..]),
        Some(b'>') if raw.starts_with(">=") => (">=", &raw[2..]),
        Some(b'<') if raw.starts_with("<>") => ("<>", &raw[2..]),
        Some(b'<') => ("<", &raw[1..]),
        Some(b'>') => (">", &raw[1..]),
        Some(b'=') => ("=", &raw[1..]),
        _ => ("=", raw),
    }
}

/// Case-insensitive `*`/`?` matching, where `*` is any run of characters, `?`
/// is one character, and `~` escapes the following wildcard.
pub(crate) fn wildcard_match(text: &str, pattern: &str) -> bool {
    let text: Vec<char> = text.to_lowercase().chars().collect();
    let pattern: Vec<char> = pattern.to_lowercase().chars().collect();
    // Iterative backtracking: remember the last `*` so a failed match can
    // resume there instead of recursing.
    let (mut t, mut p) = (0usize, 0usize);
    let (mut star, mut resume) = (None, 0usize);
    while t < text.len() {
        let literal = |i: usize| -> Option<char> {
            match pattern.get(i) {
                Some('~') => pattern.get(i + 1).copied(),
                other => other.copied(),
            }
        };
        let is_escaped = matches!(pattern.get(p), Some('~'));
        let matches_here = match pattern.get(p) {
            Some('*') => {
                star = Some(p);
                resume = t;
                p += 1;
                continue;
            }
            Some('?') => true,
            Some(_) => literal(p) == Some(text[t]),
            None => false,
        };
        if matches_here {
            p += if is_escaped { 2 } else { 1 };
            t += 1;
        } else if let Some(s) = star {
            p = s + 1;
            resume += 1;
            t = resume;
        } else {
            return false;
        }
    }
    while matches!(pattern.get(p), Some('*')) {
        p += 1;
    }
    p == pattern.len()
}

fn first_text(arg: &Arg) -> String {
    arg.values
        .first()
        .cloned()
        .unwrap_or(Value::Empty)
        .to_string()
}

/// The complex number an `IM*` function was given, or a type error.
fn complex_arg(name: &str, arg: &Arg) -> Result<Complex, Error> {
    Complex::parse(&first_text(arg)).ok_or(Error::TypeError {
        operation: name.to_string(),
        found: first_value(arg),
    })
}

/// The unit letter a complex argument is written with.
///
/// A spreadsheet keeps whichever of `i` and `j` the user typed, so the result
/// of an `IM*` function is formatted with the same letter it was given.
fn unit_of(text: &str) -> char {
    if text.trim().ends_with('j') { 'j' } else { 'i' }
}

/// The complex argument and the unit letter to format the result with.
fn complex_and_unit(name: &str, arg: &Arg) -> Result<(Complex, char), Error> {
    let text = first_text(arg);
    let z = Complex::parse(&text).ok_or(Error::TypeError {
        operation: name.to_string(),
        found: first_value(arg),
    })?;
    Ok((z, unit_of(&text)))
}

/// The unit letter shared by a list of complex arguments, taken from the first.
fn complex_unit_of(args: &[Arg]) -> Result<char, Error> {
    Ok(args
        .first()
        .map(|arg| unit_of(&first_text(arg)))
        .unwrap_or('i'))
}

/// The payment-timing flag the financial functions share.
///
/// A spreadsheet accepts `0` or `1` as well as `TRUE` or `FALSE`, and treats a
/// missing argument as `0` - payment at the end of the period.
fn kind_arg(arg: Option<&Arg>) -> f64 {
    match arg {
        Some(arg) => first_value(arg).as_number().unwrap_or(0.0).trunc(),
        None => 0.0,
    }
}

fn require_arity(name: &str, args: &[Arg], expected: usize) -> Result<(), Error> {
    if args.len() != expected {
        return Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: match expected {
                0 => "no",
                1 => "1",
                2 => "2",
                3 => "3",
                _ => "that many",
            },
        });
    }
    Ok(())
}

fn require_at_least(name: &str, args: &[Arg], minimum: usize) -> Result<(), Error> {
    if args.len() < minimum {
        return Err(Error::WrongArity {
            name: name.to_string(),
            given: args.len(),
            expected: "1 or more",
        });
    }
    Ok(())
}

/// Names of every function the crate implements.
///
/// This is public so that a caller can offer completion or validate a formula
/// against the supported set. Both the dotted modern spelling and the flat alias
/// of a function are listed, so a caller offering completion sees the name a
/// modern spreadsheet writes as well as the one an older one does; the two
/// dispatch to the same implementation.
pub const FUNCTION_NAMES: &[&str] = &[
    "ABS",
    "ACCRINT",
    "AGGREGATE",
    "ACOS",
    "ACOSH",
    "ACOT",
    "ACOTH",
    "AND",
    "ARABIC",
    "ASIN",
    "ASINH",
    "ATAN",
    "ATAN2",
    "ATANH",
    "AVEDEV",
    "AVERAGE",
    "AVERAGEA",
    "AVERAGEIF",
    "AVERAGEIFS",
    "BASE",
    "BESSELI",
    "BESSELJ",
    "BESSELK",
    "BESSELY",
    "BETADIST",
    "BETA.DIST",
    "BETAINV",
    "BETA.INV",
    "BIN2DEC",
    "BIN2HEX",
    "BIN2OCT",
    "BINOM.INV",
    "BINOMDIST",
    "BINOM.DIST",
    "BINOM.DIST.RANGE",
    "BITAND",
    "BITLSHIFT",
    "BITOR",
    "BITRSHIFT",
    "BITXOR",
    "CEILING",
    "CEILINGMATH",
    "CEILING.MATH",
    "CEILINGPRECISE",
    "CEILING.PRECISE",
    "CHAR",
    "CHIDIST",
    "CHIDISTRT",
    "CHIDIST.RT",
    "CHIINV",
    "CHIINVRT",
    "CHIINV.RT",
    "CHISQ.DIST",
    "CHISQ.DIST.RT",
    "CHISQ.INV",
    "CHISQ.INV.RT",
    "CHISQ.TEST",
    "CHITEST",
    "CHOOSE",
    "CLEAN",
    "CODE",
    "COLUMN",
    "COLUMNS",
    "COMBIN",
    "COMBINA",
    "COMPLEX",
    "CONCAT",
    "CONCATENATE",
    "CONFIDENCE",
    "CONFIDENCE.NORM",
    "CONFIDENCE.T",
    "CONVERT",
    "CORREL",
    "COS",
    "COSH",
    "COT",
    "COTH",
    "COUNT",
    "COUNTA",
    "COUNTBLANK",
    "SUBTOTAL",
    "COUNTIF",
    "COUNTIFS",
    "COUPDAYS",
    "COVAR",
    "COVARIANCE",
    "COVARIANCEP",
    "COVARIANCE.P",
    "COVARIANCES",
    "COVARIANCE.S",
    "CRITBINOM",
    "CSC",
    "CSCH",
    "CUMIPMT",
    "CUMPRINC",
    "DATE",
    "DATEDIF",
    "DATEVALUE",
    "DAY",
    "DAYS",
    "DAYS360",
    "DAVERAGE",
    "DCOUNT",
    "DCOUNTA",
    "DGET",
    "DMAX",
    "DMIN",
    "DPRODUCT",
    "DSTDEV",
    "DSTDEVP",
    "DSUM",
    "DVAR",
    "DVARP",
    "DB",
    "DDB",
    "DEC2BIN",
    "DEC2HEX",
    "DEC2OCT",
    "DECIMAL",
    "DEGREES",
    "DELTA",
    "DEVSQ",
    "DISC",
    "DOLLAR",
    "DOLLARDE",
    "DOLLARFR",
    "EDATE",
    "EFFECT",
    "EOMONTH",
    "ERF",
    "ERFC",
    "EVEN",
    "EXACT",
    "EXP",
    "EXPONDIST",
    "EXPON.DIST",
    "FACT",
    "FACTDOUBLE",
    "FALSE",
    "FDIST",
    "F.DIST",
    "FDISTRT",
    "F.DIST.RT",
    "FDIST.RT",
    "FIND",
    "FINV",
    "F.INV",
    "FINVRT",
    "F.INV.RT",
    "FINV.RT",
    "FISHER",
    "FISHERINV",
    "FIXED",
    "FLOOR",
    "FLOORMATH",
    "FLOOR.MATH",
    "FLOORPRECISE",
    "FLOOR.PRECISE",
    "FORECAST",
    "FTEST",
    "F.TEST",
    "FV",
    "FVSCHEDULE",
    "GAMMA",
    "GAMMADIST",
    "GAMMA.DIST",
    "GAMMAINV",
    "GAMMA.INV",
    "GAMMALN",
    "GAMMALNPRECISE",
    "GAMMALN.PRECISE",
    "GAUSS",
    "GCD",
    "GEOMEAN",
    "GESTEP",
    "HARMEAN",
    "HEX2BIN",
    "HEX2DEC",
    "HEX2OCT",
    "HLOOKUP",
    "HOUR",
    "HYPGEOMDIST",
    "HYPGEOM.DIST",
    "IF",
    "IFERROR",
    "IFNA",
    "IFS",
    "IMABS",
    "IMAGINARY",
    "IMARGUMENT",
    "IMCONJUGATE",
    "IMCOS",
    "IMCOSH",
    "IMCOT",
    "IMCSC",
    "IMCSCH",
    "IMDIV",
    "IMEXP",
    "IMLN",
    "IMLOG10",
    "IMLOG2",
    "IMPOWER",
    "IMPRODUCT",
    "IMREAL",
    "IMSEC",
    "IMSECH",
    "IMSIN",
    "IMSINH",
    "IMSQRT",
    "IMSUB",
    "IMSUM",
    "IMTAN",
    "INDEX",
    "INT",
    "INTERCEPT",
    "IPMT",
    "IRR",
    "ISBLANK",
    "ISERROR",
    "ISERR",
    "ISEVEN",
    "ISLOGICAL",
    "ISNA",
    "ISNONTEXT",
    "ISNUMBER",
    "ISOCEILING",
    "ISO.CEILING",
    "ISODD",
    "ISOWEEKNUM",
    "ISPMT",
    "ISTEXT",
    "KURT",
    "LARGE",
    "LCM",
    "LEFT",
    "LEN",
    "LN",
    "LOG",
    "LOG10",
    "LOGINV",
    "LOGNORMDIST",
    "LOGNORM.DIST",
    "LOGNORMINV",
    "LOGNORM.INV",
    "LOOKUP",
    "LOWER",
    "MATCH",
    "MAX",
    "MAXA",
    "MAXIFS",
    "MEDIAN",
    "MID",
    "MIN",
    "MINA",
    "MINIFS",
    "MINUTE",
    "MIRR",
    "MOD",
    "MODE",
    "MODESNGL",
    "MODE.SNGL",
    "MONTH",
    "MROUND",
    "MULTINOMIAL",
    "N",
    "NA",
    "NEGBINOMDIST",
    "NEGBINOM.DIST",
    "NETWORKDAYS",
    "NETWORKDAYSINTL",
    "NETWORKDAYS.INTL",
    "NOMINAL",
    "NORMDIST",
    "NORM.DIST",
    "NORMINV",
    "NORM.INV",
    "NORMSDIST",
    "NORM.S.DIST",
    "NORMSINV",
    "NORM.S.INV",
    "NOT",
    "NPER",
    "NPV",
    "NUMBERVALUE",
    "OCT2BIN",
    "OCT2DEC",
    "OCT2HEX",
    "ODD",
    "OR",
    "PDURATION",
    "PEARSON",
    "PERCENTILE",
    "PERCENTILEEXC",
    "PERCENTILE.EXC",
    "PERCENTILEINC",
    "PERCENTILE.INC",
    "PERCENTRANK",
    "PERCENTRANKEXC",
    "PERCENTRANK.EXC",
    "PERCENTRANKINC",
    "PERCENTRANK.INC",
    "PERMUT",
    "PERMUTATIONA",
    "PHI",
    "PI",
    "PMT",
    "POISSONDIST",
    "POISSON.DIST",
    "POWER",
    "PPMT",
    "PRICEDISC",
    "PROB",
    "PRODUCT",
    "PROPER",
    "PV",
    "QUARTILE",
    "QUARTILEEXC",
    "QUARTILE.EXC",
    "QUARTILEINC",
    "QUARTILE.INC",
    "QUOTIENT",
    "RADIANS",
    "RANK",
    "RANKAVG",
    "RANK.AVG",
    "RANKEQ",
    "RANK.EQ",
    "REPLACE",
    "REPT",
    "RIGHT",
    "ROMAN",
    "ROUND",
    "ROUNDDOWN",
    "ROUNDUP",
    "ROW",
    "ROWS",
    "RATE",
    "RRI",
    "RSQ",
    "SEARCH",
    "SEC",
    "SECH",
    "SECOND",
    "SERIESSUM",
    "SIGN",
    "SIN",
    "SINH",
    "SKEW",
    "SKEWP",
    "SKEW.P",
    "SLN",
    "SLOPE",
    "SMALL",
    "SQRT",
    "SQRTPI",
    "STANDARDIZE",
    "STDEV",
    "STDEVA",
    "STDEVP",
    "STDEV.P",
    "STDEVPA",
    "STDEVS",
    "STDEV.S",
    "STEYX",
    "SUBSTITUTE",
    "SUM",
    "SUMIF",
    "SUMIFS",
    "SUMPRODUCT",
    "SWITCH",
    "SUMSQ",
    "SUMX2MY2",
    "SUMX2PY2",
    "SUMXMY2",
    "SYD",
    "T",
    "TAN",
    "TANH",
    "TBILLEQ",
    "TBILLPRICE",
    "TBILLYIELD",
    "TDIST",
    "T.DIST",
    "T.DIST.2T",
    "TDISTRT",
    "T.DIST.RT",
    "TDIST.RT",
    "TEXT",
    "TEXTJOIN",
    "TIME",
    "TIMEVALUE",
    "TINV",
    "T.INV",
    "T.INV.2T",
    "TRIM",
    "TRIMMEAN",
    "TRUE",
    "TRUNC",
    "TTEST",
    "T.TEST",
    "TYPE",
    "UNICHAR",
    "UNICODE",
    "UPPER",
    "VALUE",
    "VAR",
    "VARA",
    "VARP",
    "VAR.P",
    "VARPA",
    "VARS",
    "VAR.S",
    "VLOOKUP",
    "WEEKDAY",
    "WEEKNUM",
    "WEIBULLDIST",
    "WEIBULL.DIST",
    "WORKDAY",
    "WORKDAYINTL",
    "WORKDAY.INTL",
    "XIRR",
    "XNPV",
    "XOR",
    "YEAR",
    "YEARFRAC",
    "ZTEST",
    "Z.TEST",
];

#[cfg(test)]
mod tests {
    use super::*;

    /// [`call_from`] without an origin, for functions that do not need one.
    fn call(name: &str, args: &[Arg]) -> Result<Value, Error> {
        call_from(name, args, None)
    }

    fn n(v: f64) -> Arg {
        Arg::scalar(Value::Number(v))
    }

    /// A range of numbers, as a reference.
    fn nums(vs: &[f64]) -> Arg {
        Arg::reference(vs.iter().map(|v| Value::Number(*v)).collect())
    }

    fn t(s: &str) -> Arg {
        Arg::scalar(Value::Text(s.into()))
    }

    #[test]
    fn every_registered_name_has_an_implementation() {
        // `MID` was implemented but missing from `FUNCTION_NAMES`, so a caller
        // validating a formula against the published list rejected a function
        // that worked. This catches the next one.
        for name in FUNCTION_NAMES {
            let result = call(name, &[n(1.0)]);
            assert!(
                !matches!(result, Err(Error::UnknownFunction(_))),
                "{name} is advertised but not implemented"
            );
        }
    }

    #[test]
    fn trigonometry_and_hyperbolics_match_the_reference() {
        assert_eq!(call("SIN", &[n(0.0)]).unwrap(), Value::Number(0.0));
        assert_eq!(call("COS", &[n(0.0)]).unwrap(), Value::Number(1.0));
        assert!(close(
            call("ATAN", &[n(1.0)]).unwrap(),
            std::f64::consts::FRAC_PI_4,
        ));
        assert!(close(
            call("DEGREES", &[n(std::f64::consts::PI)]).unwrap(),
            180.0
        ));
        assert!(close(
            call("RADIANS", &[n(180.0)]).unwrap(),
            std::f64::consts::PI
        ));
        // `ACOT` follows `atan(1/x)`, so a negative argument is negative.
        assert!(close(
            call("ACOT", &[n(-1.0)]).unwrap(),
            -std::f64::consts::FRAC_PI_4,
        ));
        assert!(close(
            call("SQRTPI", &[n(4.0)]).unwrap(),
            3.544_907_701_811_031_8
        ));
    }

    #[test]
    fn combinatorics_and_roman_numerals_match_the_reference() {
        assert_eq!(
            call("COMBIN", &[n(5.0), n(2.0)]).unwrap(),
            Value::Number(10.0)
        );
        assert_eq!(
            call("COMBINA", &[n(5.0), n(2.0)]).unwrap(),
            Value::Number(15.0)
        );
        assert_eq!(
            call("PERMUT", &[n(5.0), n(2.0)]).unwrap(),
            Value::Number(20.0)
        );
        assert_eq!(call("FACTDOUBLE", &[n(6.0)]).unwrap(), Value::Number(48.0));
        assert_eq!(
            call("MULTINOMIAL", &[n(1.0), n(2.0), n(3.0)]).unwrap(),
            Value::Number(60.0)
        );
        assert_eq!(
            call("ROMAN", &[n(2024.0)]).unwrap(),
            Value::Text("MMXXIV".into())
        );
        assert_eq!(
            call("ARABIC", &[t("MCMXC")]).unwrap(),
            Value::Number(1990.0)
        );
        // A numeral that is not the canonical form is rejected, as is lower case.
        assert!(call("ARABIC", &[t("IIII")]).is_err());
        assert!(call("ARABIC", &[t("iv")]).is_err());
    }

    #[test]
    fn base_conversion_uses_the_ten_digit_field() {
        assert_eq!(call("BIN2DEC", &[t("1010")]).unwrap(), Value::Number(10.0));
        // Ten digits with a leading 1 is a negative number.
        assert_eq!(
            call("BIN2DEC", &[t("1111111111")]).unwrap(),
            Value::Number(-1.0)
        );
        assert_eq!(call("HEX2DEC", &[t("FF")]).unwrap(), Value::Number(255.0));
        assert_eq!(
            call("DEC2BIN", &[n(10.0)]).unwrap(),
            Value::Text("1010".into())
        );
        assert_eq!(
            call("DEC2BIN", &[n(-5.0)]).unwrap(),
            Value::Text("1111111011".into())
        );
        assert_eq!(
            call("DEC2HEX", &[n(255.0)]).unwrap(),
            Value::Text("ff".into())
        );
        assert_eq!(
            call("BASE", &[n(255.0), n(16.0)]).unwrap(),
            Value::Text("ff".into())
        );
        assert_eq!(
            call("DECIMAL", &[t("zz"), n(36.0)]).unwrap(),
            Value::Number(1295.0)
        );
        // Cross-base fields keep the ten-character shape when negative, and the
        // optional places only pads a non-negative result.
        assert_eq!(
            call("BIN2HEX", &[t("1010")]).unwrap(),
            Value::Text("a".into())
        );
        assert_eq!(
            call("BIN2HEX", &[t("1111111111")]).unwrap(),
            Value::Text("ffffffffff".into())
        );
        assert_eq!(
            call("BIN2OCT", &[t("1010"), n(6.0)]).unwrap(),
            Value::Text("000012".into())
        );
        assert_eq!(
            call("HEX2BIN", &[t("F"), n(8.0)]).unwrap(),
            Value::Text("00001111".into())
        );
        assert_eq!(
            call("HEX2OCT", &[t("FF")]).unwrap(),
            Value::Text("377".into())
        );
        assert_eq!(
            call("OCT2BIN", &[t("17")]).unwrap(),
            Value::Text("1111".into())
        );
        assert_eq!(
            call("OCT2HEX", &[t("7777777777")]).unwrap(),
            Value::Text("ffffffffff".into())
        );
        // `places` pads a short result but never truncates a long one, which is
        // what the reference does too.
        assert_eq!(
            call("BIN2HEX", &[t("1010"), n(2.0)]).unwrap(),
            Value::Text("0a".into())
        );
        // A value outside the binary field's range, and a digit the source base
        // does not have, are errors rather than silent truncation.
        assert!(call("HEX2BIN", &[t("FFFFFFFF")]).is_err());
        assert!(call("BIN2HEX", &[t("1012")]).is_err());
    }

    #[test]
    fn bit_functions_reject_out_of_range_operands() {
        assert_eq!(
            call("BITAND", &[n(5.0), n(3.0)]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("BITOR", &[n(5.0), n(3.0)]).unwrap(),
            Value::Number(7.0)
        );
        assert_eq!(
            call("BITXOR", &[n(5.0), n(3.0)]).unwrap(),
            Value::Number(6.0)
        );
        assert_eq!(
            call("BITLSHIFT", &[n(5.0), n(2.0)]).unwrap(),
            Value::Number(20.0)
        );
        assert_eq!(
            call("BITRSHIFT", &[n(5.0), n(2.0)]).unwrap(),
            Value::Number(1.0)
        );
        assert!(call("BITAND", &[n(-1.0), n(1.0)]).is_err());
        // A shift past the 48-bit range has no answer.
        assert!(call("BITLSHIFT", &[n(5.0), n(54.0)]).is_err());
    }

    #[test]
    fn information_functions_read_the_value_kind() {
        assert_eq!(call("ISNUMBER", &[n(5.0)]).unwrap(), Value::Bool(true));
        assert_eq!(call("ISNUMBER", &[t("5")]).unwrap(), Value::Bool(false));
        assert_eq!(call("ISTEXT", &[t("x")]).unwrap(), Value::Bool(true));
        assert_eq!(
            call("ISLOGICAL", &[Arg::scalar(Value::Bool(true))]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(call("ISEVEN", &[n(2.0)]).unwrap(), Value::Bool(true));
        assert_eq!(call("ISODD", &[n(2.0)]).unwrap(), Value::Bool(false));
        assert_eq!(call("TYPE", &[n(5.0)]).unwrap(), Value::Number(1.0));
        assert_eq!(call("TYPE", &[t("x")]).unwrap(), Value::Number(2.0));
        assert_eq!(
            call("DELTA", &[n(1.0), n(1.0)]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("GESTEP", &[n(5.0), n(4.0)]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call(
                "XOR",
                &[
                    Arg::scalar(Value::Bool(true)),
                    Arg::scalar(Value::Bool(true))
                ]
            )
            .unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn statistics_use_the_sample_forms_the_reference_uses() {
        let range = Arg::reference(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(4.0),
        ]);
        assert!(close(
            call("VARS", std::slice::from_ref(&range)).unwrap(),
            1.666_666_666_666_666_7
        ));
        assert!(close(
            call("VARP", std::slice::from_ref(&range)).unwrap(),
            1.25
        ));
        assert!(close(
            call("STDEVS", std::slice::from_ref(&range)).unwrap(),
            1.290_994_448_735_805_6
        ));
        assert!(close(
            call("AVEDEV", std::slice::from_ref(&range)).unwrap(),
            1.0
        ));
        assert!(close(
            call("DEVSQ", std::slice::from_ref(&range)).unwrap(),
            5.0
        ));
        assert!(close(
            call("GEOMEAN", std::slice::from_ref(&range)).unwrap(),
            2.213_363_839_400_643
        ));
        assert!(close(
            call("HARMEAN", std::slice::from_ref(&range)).unwrap(),
            1.920_000_000_000_000_4
        ));
        assert_eq!(
            call("LARGE", &[range.clone(), n(2.0)]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            call("SMALL", &[range.clone(), n(2.0)]).unwrap(),
            Value::Number(2.0)
        );
        assert!(close(
            call("PERCENTILEINC", &[range.clone(), n(0.5)]).unwrap(),
            2.5
        ));
        assert!(close(
            call("TRIMMEAN", &[range.clone(), n(0.5)]).unwrap(),
            2.5
        ));
        assert!(close(
            call(
                "KURT",
                &[Arg::reference(vec![
                    Value::Number(1.0),
                    Value::Number(2.0),
                    Value::Number(3.0),
                    Value::Number(4.0),
                    Value::Number(5.0),
                ])]
            )
            .unwrap(),
            -1.200_000_000_000_002
        ));
        // The legacy flat names Excel still accepts are the modern ones; the
        // reference exposes both spellings and they must agree.
        assert!(close(
            call("STDEV", std::slice::from_ref(&range)).unwrap(),
            1.290_994_448_735_805_6
        ));
        assert!(close(
            call("VAR", std::slice::from_ref(&range)).unwrap(),
            1.666_666_666_666_666_7
        ));
        assert!(close(
            call("PERCENTRANK", &[range.clone(), n(2.0)]).unwrap(),
            0.333
        ));
        assert!(close(
            call("PERCENTRANK", &[range.clone(), n(2.5)]).unwrap(),
            0.5
        ));
        assert!(close(
            call(
                "COVARIANCE",
                &[
                    Arg::reference(vec![
                        Value::Number(3.0),
                        Value::Number(2.0),
                        Value::Number(4.0),
                        Value::Number(5.0),
                        Value::Number(6.0),
                    ]),
                    Arg::reference(vec![
                        Value::Number(9.0),
                        Value::Number(7.0),
                        Value::Number(12.0),
                        Value::Number(15.0),
                        Value::Number(17.0),
                    ]),
                ]
            )
            .unwrap(),
            5.2
        ));
    }

    #[test]
    fn special_functions_match_the_reference() {
        assert!(close(call("GAMMA", &[n(5.0)]).unwrap(), 24.0));
        assert!(close(
            call("GAMMA", &[n(0.5)]).unwrap(),
            1.772_453_855_880_139
        ));
        assert!(close(
            call("ERF", &[n(1.0)]).unwrap(),
            0.842_700_792_949_714_9
        ));
        assert!(close(
            call("ERFC", &[n(2.0)]).unwrap(),
            0.004_677_734_981_047_288
        ));
        // A pole yields a NaN, which the evaluator turns into a number error.
        assert!(matches!(call("GAMMA", &[n(0.0)]), Ok(Value::Number(n)) if n.is_nan()));
    }

    #[test]
    fn confidence_intervals_match_the_reference() {
        // Values from `@formulajs/formulajs`; the dotted spelling and the flat
        // one reach the same arm.
        assert!(close(
            call("CONFIDENCE", &[n(0.05), n(2.5), n(50.0)]).unwrap(),
            0.692_951_912_174_839_1
        ));
        assert!(close(
            call("CONFIDENCE.NORM", &[n(0.05), n(2.5), n(50.0)]).unwrap(),
            0.692_951_912_174_839_1
        ));
        assert!(close(
            call("CONFIDENCE.T", &[n(0.05), n(1.0), n(50.0)]).unwrap(),
            0.284_196_850_152_904_6
        ));
        // An out-of-range alpha, a non-positive standard deviation and a
        // sample smaller than two for the t form are all errors.
        assert!(call("CONFIDENCE.NORM", &[n(0.0), n(1.0), n(10.0)]).is_err());
        assert!(call("CONFIDENCE.NORM", &[n(0.05), n(0.0), n(10.0)]).is_err());
        assert!(call("CONFIDENCE.T", &[n(0.05), n(1.0), n(1.0)]).is_err());
    }

    #[test]
    fn text_helpers_match_the_reference() {
        assert_eq!(
            call("REPLACE", &[t("abcdef"), n(2.0), n(3.0), t("XY")]).unwrap(),
            Value::Text("aXYef".into())
        );
        assert_eq!(
            call("DOLLAR", &[n(1234.5), n(2.0)]).unwrap(),
            Value::Text("$1,234.50".into())
        );
        assert_eq!(
            call("DOLLAR", &[n(-1234.5), n(2.0)]).unwrap(),
            Value::Text("$(1,234.50)".into())
        );
        assert_eq!(
            call(
                "FIXED",
                &[n(1234.567), n(2.0), Arg::scalar(Value::Bool(true))]
            )
            .unwrap(),
            Value::Text("1234.57".into())
        );
        assert_eq!(
            call("NUMBERVALUE", &[t("1,5"), t(",")]).unwrap(),
            Value::Number(1.5)
        );
        assert_eq!(
            call(
                "TEXTJOIN",
                &[t(","), Arg::scalar(Value::Bool(true)), t("a"), t("b")]
            )
            .unwrap(),
            Value::Text("a,b".into())
        );
    }

    /// Whether a numeric result is within a rounding step of the expected one.
    fn close(value: Value, expected: f64) -> bool {
        match value {
            Value::Number(n) => (n - expected).abs() <= 1e-9 * expected.abs().max(1.0),
            _ => false,
        }
    }

    #[test]
    fn countif_accepts_a_bare_value_and_a_comparison() {
        let range = Arg::reference(vec![
            Value::Number(1.0),
            Value::Number(2.0),
            Value::Number(3.0),
            Value::Number(4.0),
        ]);
        assert_eq!(
            call("COUNTIF", &[range.clone(), n(3.0)]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("COUNTIF", &[range.clone(), t(">2")]).unwrap(),
            Value::Number(2.0)
        );
        assert_eq!(
            call("COUNTIF", &[range, t("<=2")]).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn criteria_match_only_the_matching_type() {
        // A numeric criterion never matches text, and a text criterion never
        // matches a number: `COUNTIF(range,">2")` over text is 0, and
        // `COUNTIF(range,"*2*")` does not count the number 2.
        let mixed = Arg::reference(vec![
            Value::Number(2.0),
            Value::Text("abc".into()),
            Value::Number(5.0),
        ]);
        assert_eq!(
            call("COUNTIF", &[mixed.clone(), t(">2")]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("COUNTIF", &[mixed.clone(), t("*2*")]).unwrap(),
            Value::Number(0.0)
        );
        assert_eq!(
            call("COUNTIF", &[mixed, t("<z")]).unwrap(),
            Value::Number(1.0)
        );
    }

    #[test]
    fn criteria_support_wildcards() {
        let fruits = Arg::reference(vec![
            Value::Text("apple".into()),
            Value::Text("banana".into()),
            Value::Text("APPLE".into()),
            Value::Text("cherry".into()),
        ]);
        let count = |c: &str| call("COUNTIF", &[fruits.clone(), t(c)]).unwrap();
        assert_eq!(count("apple"), Value::Number(2.0));
        assert_eq!(count("a*"), Value::Number(2.0));
        assert_eq!(count("*e"), Value::Number(2.0));
        assert_eq!(count("*a*"), Value::Number(3.0));
        assert_eq!(count("?anana"), Value::Number(1.0));
        assert_eq!(count("a?ple"), Value::Number(2.0));
    }

    #[test]
    fn a_logical_criterion_matches_only_logicals() {
        let range = Arg::reference(vec![
            Value::Bool(true),
            Value::Number(1.0),
            Value::Bool(false),
        ]);
        assert_eq!(
            call("COUNTIF", &[range, Arg::scalar(Value::Bool(true))]).unwrap(),
            Value::Number(1.0)
        );
    }

    #[test]
    fn countif_is_case_insensitive_over_text() {
        let range = Arg::reference(vec![
            Value::Text("Yes".into()),
            Value::Text("yes".into()),
            Value::Text("no".into()),
        ]);
        assert_eq!(
            call("COUNTIF", &[range, t("YES")]).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn countblank_counts_empty_cells_and_empty_text() {
        let range = Arg::reference(vec![
            Value::Empty,
            Value::Text(String::new()),
            Value::Number(0.0),
        ]);
        assert_eq!(call("COUNTBLANK", &[range]).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn sumif_uses_the_third_argument_as_the_sum_range() {
        // The bug this pins: summing the tested range instead of the sum range.
        // The criteria match the first two cells, whose values in the sum range
        // are 100 and 200, not 1 and 2.
        let tested = nums(&[1.0, 2.0, 3.0]);
        let sum_range = nums(&[100.0, 200.0, 300.0]);
        assert_eq!(
            call("SUMIF", &[tested.clone(), t("<=2"), sum_range.clone()]).unwrap(),
            Value::Number(300.0)
        );
        assert_eq!(
            call("AVERAGEIF", &[tested.clone(), t("<=2"), sum_range]).unwrap(),
            Value::Number(150.0)
        );
        // Without the third argument the tested range is summed.
        assert_eq!(
            call("SUMIF", &[tested, t("<=2")]).unwrap(),
            Value::Number(3.0)
        );
    }

    #[test]
    fn ifs_returns_the_first_holding_condition() {
        let arg = |v: Value| Arg::reference(vec![v]);
        assert_eq!(
            call(
                "IFS",
                &[
                    arg(Value::Bool(false)),
                    n(1.0),
                    arg(Value::Bool(true)),
                    n(2.0)
                ]
            )
            .unwrap(),
            Value::Number(2.0)
        );
        // With no condition holding, an odd count means the last argument is the
        // fallback and is returned.
        assert_eq!(
            call("IFS", &[arg(Value::Bool(false)), n(1.0), n(99.0)]).unwrap(),
            Value::Number(99.0)
        );
        // Without a fallback there is nothing to return, which is an error.
        assert_eq!(
            call("IFS", &[arg(Value::Bool(false)), n(1.0)]),
            Err(Error::NoMatchingCondition)
        );
    }

    #[test]
    fn exact_is_case_sensitive_where_a_comparison_is_not() {
        assert_eq!(
            call("EXACT", &[t("abc"), t("abc")]).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            call("EXACT", &[t("abc"), t("ABC")]).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn mid_counts_from_one() {
        assert_eq!(
            call("MID", &[t("hello"), n(2.0), n(3.0)]).unwrap(),
            Value::Text("ell".into())
        );
        // Past the end is empty, not an error.
        assert_eq!(
            call("MID", &[t("hello"), n(10.0), n(3.0)]).unwrap(),
            Value::Text(String::new())
        );
    }

    #[test]
    fn rept_and_substitute_behave_on_the_edges() {
        assert_eq!(
            call("REPT", &[t("ab"), n(3.0)]).unwrap(),
            Value::Text("ababab".into())
        );
        // A zero count is empty rather than an error.
        assert_eq!(
            call("REPT", &[t("ab"), n(0.0)]).unwrap(),
            Value::Text(String::new())
        );
        assert_eq!(
            call("SUBSTITUTE", &[t("a-b-c"), t("-"), t("+")]).unwrap(),
            Value::Text("a+b+c".into())
        );
        // An empty needle must not hang.
        assert_eq!(
            call("SUBSTITUTE", &[t("abc"), t(""), t("x")]).unwrap(),
            Value::Text("abc".into())
        );
    }

    #[test]
    fn sum_over_scalars_and_ranges() {
        assert_eq!(call("SUM", &[n(1.0), n(2.0)]).unwrap(), Value::Number(3.0));
        assert_eq!(
            call("SUM", &[nums(&[1.0, 2.0, 3.0])]).unwrap(),
            Value::Number(6.0)
        );
        assert_eq!(
            call("SUM", &[nums(&[1.0, 2.0]), n(3.0)]).unwrap(),
            Value::Number(6.0)
        );
    }

    #[test]
    fn sum_ignores_text_and_empty() {
        let arg = Arg::reference(vec![
            Value::Number(1.0),
            Value::Text("x".into()),
            Value::Empty,
        ]);
        assert_eq!(call("SUM", &[arg]).unwrap(), Value::Number(1.0));
    }

    #[test]
    fn average_median_min_max() {
        let arg = nums(&[1.0, 2.0, 3.0, 4.0]);
        assert_eq!(
            call("AVERAGE", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(2.5)
        );
        assert_eq!(
            call("MEDIAN", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(2.5)
        );
        assert_eq!(
            call("MIN", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("MAX", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(4.0)
        );
        let odd = nums(&[3.0, 1.0, 2.0]);
        assert_eq!(call("MEDIAN", &[odd]).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn empty_aggregates_follow_the_spreadsheet_rules() {
        // An empty range has no numbers. AVERAGE and MEDIAN cannot produce a
        // value and report an error, MIN and MAX return 0, and PRODUCT returns
        // the empty product 1. Verified against Excel.
        let empty = Arg::reference(vec![Value::Empty; 3]);
        assert!(matches!(
            call("AVERAGE", std::slice::from_ref(&empty)),
            Err(Error::Arithmetic(ValueError::DivisionByZero))
        ));
        assert!(matches!(
            call("MEDIAN", std::slice::from_ref(&empty)),
            Err(Error::Arithmetic(ValueError::NotANumber))
        ));
        assert_eq!(
            call("MIN", std::slice::from_ref(&empty)).unwrap(),
            Value::Number(0.0)
        );
        assert_eq!(
            call("MAX", std::slice::from_ref(&empty)).unwrap(),
            Value::Number(0.0)
        );
        assert_eq!(
            call("PRODUCT", std::slice::from_ref(&empty)).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("SUM", std::slice::from_ref(&empty)).unwrap(),
            Value::Number(0.0)
        );
        // A literal that is not a number is a type error, not an ignored value.
        assert!(matches!(
            call("AVERAGE", &[t("x")]),
            Err(Error::TypeError { .. })
        ));
    }

    #[test]
    fn references_ignore_text_logicals_and_blanks() {
        // In a cell, `TRUE` and text are ignored by the aggregators; written
        // directly they are coerced. This is the difference between SUM(A1) and
        // SUM(TRUE).
        let mixed = Arg::reference(vec![
            Value::Number(1.0),
            Value::Bool(true),
            Value::Text("2".into()),
            Value::Empty,
        ]);
        assert_eq!(
            call("SUM", std::slice::from_ref(&mixed)).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("COUNT", std::slice::from_ref(&mixed)).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(
            call("MIN", std::slice::from_ref(&mixed)).unwrap(),
            Value::Number(1.0)
        );
        // A literal `TRUE` counts as 1, and literal numeric text as its number.
        assert_eq!(
            call("SUM", &[Arg::scalar(Value::Bool(true))]).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(call("SUM", &[t("2")]).unwrap(), Value::Number(2.0));
        assert_eq!(
            call("COUNT", &[Arg::scalar(Value::Bool(true))]).unwrap(),
            Value::Number(1.0)
        );
    }

    #[test]
    fn counting_functions_differ() {
        let arg = Arg::reference(vec![
            Value::Number(1.0),
            Value::Text("x".into()),
            Value::Empty,
        ]);
        assert_eq!(
            call("COUNT", std::slice::from_ref(&arg)).unwrap(),
            Value::Number(1.0)
        );
        assert_eq!(call("COUNTA", &[arg]).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn rounding_modes() {
        assert_eq!(
            call("ROUND", &[n(2.5), n(0.0)]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            call("ROUND", &[n(-2.5), n(0.0)]).unwrap(),
            Value::Number(-3.0)
        );
        assert_eq!(
            call("ROUND", &[n(2.345), n(2.0)]).unwrap(),
            Value::Number(2.35)
        );
        assert_eq!(
            call("ROUNDUP", &[n(2.1), n(0.0)]).unwrap(),
            Value::Number(3.0)
        );
        assert_eq!(
            call("ROUNDDOWN", &[n(2.9), n(0.0)]).unwrap(),
            Value::Number(2.0)
        );
    }

    #[test]
    fn mod_follows_divisor_sign() {
        assert_eq!(call("MOD", &[n(7.0), n(3.0)]).unwrap(), Value::Number(1.0));
        assert_eq!(call("MOD", &[n(-7.0), n(3.0)]).unwrap(), Value::Number(2.0));
        assert!(matches!(
            call("MOD", &[n(1.0), n(0.0)]),
            Err(Error::Arithmetic(crate::value::Error::DivisionByZero))
        ));
    }

    #[test]
    fn text_functions() {
        assert_eq!(call("LEN", &[t("hello")]).unwrap(), Value::Number(5.0));
        assert_eq!(
            call("UPPER", &[t("abc")]).unwrap(),
            Value::Text("ABC".into())
        );
        assert_eq!(
            call("LOWER", &[t("ABC")]).unwrap(),
            Value::Text("abc".into())
        );
        assert_eq!(
            call("TRIM", &[t("  a   b  ")]).unwrap(),
            Value::Text("a b".into())
        );
        assert_eq!(
            call("LEFT", &[t("hello"), n(2.0)]).unwrap(),
            Value::Text("he".into())
        );
        assert_eq!(
            call("RIGHT", &[t("hello"), n(2.0)]).unwrap(),
            Value::Text("lo".into())
        );
        assert_eq!(
            call("MID", &[t("hello"), n(2.0), n(3.0)]).unwrap(),
            Value::Text("ell".into())
        );
    }

    #[test]
    fn len_counts_characters_not_bytes() {
        assert_eq!(call("LEN", &[t("éé")]).unwrap(), Value::Number(2.0));
    }

    #[test]
    fn concat_converts_numbers() {
        let arg = Arg::reference(vec![Value::Text("n=".into()), Value::Number(18.0)]);
        assert_eq!(call("CONCAT", &[arg]).unwrap(), Value::Text("n=18".into()));
    }

    #[test]
    fn logical_functions() {
        let arg = Arg::reference(vec![Value::Bool(true), Value::Bool(true)]);
        assert_eq!(
            call("AND", std::slice::from_ref(&arg)).unwrap(),
            Value::Bool(true)
        );
        let mixed = Arg::reference(vec![Value::Bool(true), Value::Bool(false)]);
        assert_eq!(call("AND", &[mixed]).unwrap(), Value::Bool(false));
        let mixed = Arg::reference(vec![Value::Bool(true), Value::Bool(false)]);
        assert_eq!(call("OR", &[mixed]).unwrap(), Value::Bool(true));
        assert_eq!(
            call("NOT", &[Arg::reference(vec![Value::Bool(false)])]).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn arity_errors_are_reported() {
        assert!(matches!(call("ABS", &[]), Err(Error::WrongArity { .. })));
        assert!(matches!(
            call("ABS", &[n(1.0), n(2.0)]),
            Err(Error::WrongArity { .. })
        ));
        assert!(matches!(call("SUM", &[]), Err(Error::WrongArity { .. })));
        assert!(matches!(call("NOPE", &[]), Err(Error::UnknownFunction(_))));
    }

    #[test]
    fn every_advertised_function_is_callable() {
        for name in FUNCTION_NAMES {
            // A wrong-arity or type error still proves the name is dispatched;
            // only an unknown-function error would mean the list is wrong.
            if let Err(Error::UnknownFunction(n)) = call(name, &[]) {
                panic!("{n} is listed in FUNCTION_NAMES but not implemented");
            }
        }
    }

    #[test]
    fn pi_and_sqrt_and_power() {
        assert_eq!(
            call("PI", &[]).unwrap(),
            Value::Number(std::f64::consts::PI)
        );
        assert_eq!(call("SQRT", &[n(9.0)]).unwrap(), Value::Number(3.0));
        assert_eq!(
            call("POWER", &[n(2.0), n(10.0)]).unwrap(),
            Value::Number(1024.0)
        );
        assert_eq!(
            call("PRODUCT", &[n(2.0), n(3.0), n(4.0)]).unwrap(),
            Value::Number(24.0)
        );
    }
}
