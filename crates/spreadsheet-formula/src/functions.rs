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
use crate::value::{Error as ValueError, Value};

/// A flattened argument, and whether it came from a cell or range reference.
#[derive(Debug, Clone)]
pub(crate) struct Arg {
    values: Vec<Value>,
    from_ref: bool,
}

impl Arg {
    /// An argument written directly in the formula.
    pub(crate) fn scalar(value: Value) -> Self {
        Self {
            values: vec![value],
            from_ref: false,
        }
    }

    /// An argument read from cells.
    pub(crate) fn reference(values: Vec<Value>) -> Self {
        Self {
            values,
            from_ref: true,
        }
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

/// Collects the numbers from every argument of an aggregating function.
fn numbers_of(name: &str, args: &[Arg]) -> Result<Vec<f64>, Error> {
    let mut out = Vec::new();
    for arg in args {
        out.extend(arg.aggregate_numbers(name)?);
    }
    Ok(out)
}

/// Calls a function by name.
pub(crate) fn call(name: &str, args: &[Arg]) -> Result<Value, Error> {
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
        "CEILING" | "CEILING.MATH" => {
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
        "FLOOR" | "FLOOR.MATH" => {
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
        other => Err(Error::UnknownFunction(other.to_string())),
    }
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
fn split_operator(raw: &str) -> (&str, &str) {
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
fn wildcard_match(text: &str, pattern: &str) -> bool {
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
/// against the supported set.
pub const FUNCTION_NAMES: &[&str] = &[
    "ABS",
    "AND",
    "AVERAGE",
    "AVERAGEIF",
    "AVERAGEIFS",
    "CEILING",
    "CHAR",
    "CLEAN",
    "CODE",
    "CONCAT",
    "CONCATENATE",
    "COUNT",
    "COUNTA",
    "COUNTBLANK",
    "COUNTIF",
    "COUNTIFS",
    "EVEN",
    "EXACT",
    "EXP",
    "FACT",
    "FIND",
    "FLOOR",
    "GCD",
    "IFS",
    "INT",
    "LCM",
    "LEFT",
    "LEN",
    "LN",
    "LOG",
    "LOG10",
    "LOWER",
    "MAX",
    "MAXIFS",
    "MEDIAN",
    "MID",
    "MIN",
    "MINIFS",
    "MOD",
    "MROUND",
    "N",
    "NOT",
    "ODD",
    "OR",
    "PI",
    "POWER",
    "PRODUCT",
    "PROPER",
    "QUOTIENT",
    "REPT",
    "RIGHT",
    "ROUND",
    "ROUNDDOWN",
    "ROUNDUP",
    "SEARCH",
    "SIGN",
    "SQRT",
    "SUBSTITUTE",
    "SUM",
    "SUMIF",
    "SUMIFS",
    "SUMPRODUCT",
    "SUMSQ",
    "T",
    "TEXT",
    "TRIM",
    "TRUNC",
    "UNICHAR",
    "UNICODE",
    "UPPER",
    "VALUE",
];

#[cfg(test)]
mod tests {
    use super::*;

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
