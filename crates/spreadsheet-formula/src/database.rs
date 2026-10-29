//! The database functions: `DSUM`, `DAVERAGE` and the rest of the family.
//!
//! A database is a rectangular range in which the first cell of every row names
//! the field, so one field's values run across a row rather than down a column.
//! A criteria range has the same shape: the first cell of a criteria row names a
//! field and the remaining cells are its tests. A record matches when every
//! criteria row that names one of its fields holds, which makes two criteria
//! rows a conjunction and two cells within one row a disjunction.
//!
//! Both the matching rule and the aggregations follow `@formulajs/formulajs`,
//! this crate's oracle, quirks included: a text criterion is compared exactly
//! and case-sensitively, a numeric criterion matches only numeric cells, and a
//! criterion that is `*` or blank matches everything.

use crate::Error;
use crate::functions::{population_variance, sample_variance, split_operator};
use crate::stats;
use crate::value::{Error as ValueError, Value};

/// The aggregate a `D...` function computes over the records its criteria
/// range selects.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Aggregate {
    /// The sum of the field's values.
    Sum,
    /// The mean of the field's values.
    Average,
    /// How many of the field's values are numbers.
    Count,
    /// How many of the field's values are not blank.
    CountA,
    /// The single value the criteria select.
    Get,
    /// The largest of the field's values.
    Max,
    /// The smallest of the field's values.
    Min,
    /// The product of the field's values.
    Product,
    /// The sample standard deviation of the field's values.
    StDev,
    /// The population standard deviation of the field's values.
    StDevP,
    /// The sample variance of the field's values.
    Var,
    /// The population variance of the field's values.
    VarP,
}

impl Aggregate {
    /// The aggregate a function name selects, or `None` for any other name.
    pub(crate) fn for_name(name: &str) -> Option<Self> {
        Some(match name {
            "DSUM" => Self::Sum,
            "DAVERAGE" => Self::Average,
            "DCOUNT" => Self::Count,
            "DCOUNTA" => Self::CountA,
            "DGET" => Self::Get,
            "DMAX" => Self::Max,
            "DMIN" => Self::Min,
            "DPRODUCT" => Self::Product,
            "DSTDEV" => Self::StDev,
            "DSTDEVP" => Self::StDevP,
            "DVAR" => Self::Var,
            "DVARP" => Self::VarP,
            _ => return None,
        })
    }
}

/// Evaluates one database function over the given grids.
///
/// `field` is the field argument's first value: the name of the row to
/// aggregate, or a zero-based row index. `database` and `criteria` are
/// row-major grids given with their column counts.
pub(crate) fn evaluate(
    aggregate: Aggregate,
    database: &[Value],
    width: usize,
    field: &Value,
    criteria: &[Value],
    criteria_width: usize,
) -> Result<Value, Error> {
    let rows = database.len() / width;
    let field_row = field_row(field, database, width, rows)?;
    let matched = matching_records(database, width, rows, criteria, criteria_width);
    let target = &database[field_row * width + 1..field_row * width + width];
    let numbers: Vec<f64> = matched
        .iter()
        .filter_map(|&i| target.get(i).and_then(|v| v.as_number().ok()))
        .collect();

    Ok(match aggregate {
        Aggregate::Sum => Value::Number(numbers.iter().sum()),
        // `DCOUNT` counts the numeric cells, so a blank or text value in the
        // field is not counted even though the reference's own `SUM` treats a
        // blank as zero.
        Aggregate::Count => Value::Number(
            matched
                .iter()
                .filter(|&&i| matches!(target.get(i), Some(Value::Number(_))))
                .count() as f64,
        ),
        Aggregate::CountA => Value::Number(
            matched
                .iter()
                .filter(|&&i| target.get(i).is_some_and(|v| !v.is_empty()))
                .count() as f64,
        ),
        Aggregate::Average => {
            if numbers.is_empty() {
                return Err(Error::Arithmetic(ValueError::DivisionByZero));
            }
            Value::Number(stats::mean(&numbers))
        }
        // `DGET` is the one matching value, and an error when the criteria
        // select none or more than one. The reference distinguishes `#VALUE!`
        // from `#NUM!`; this crate reports one arithmetic error for both.
        Aggregate::Get => match matched.as_slice() {
            [one] => target[*one].clone(),
            _ => return Err(Error::Arithmetic(ValueError::NotANumber)),
        },
        Aggregate::Max => numbers
            .iter()
            .copied()
            .reduce(f64::max)
            .map_or(Value::Empty, Value::Number),
        Aggregate::Min => numbers
            .iter()
            .copied()
            .reduce(f64::min)
            .map_or(Value::Empty, Value::Number),
        Aggregate::Product => Value::Number(numbers.iter().product()),
        Aggregate::StDev => Value::Number(sample_variance(&numbers).sqrt()),
        Aggregate::StDevP => Value::Number(population_variance(&numbers).sqrt()),
        Aggregate::Var => Value::Number(sample_variance(&numbers)),
        Aggregate::VarP => Value::Number(population_variance(&numbers)),
    })
}

/// The row a field argument names.
///
/// A number is a zero-based row index, which is how the reference reads it;
/// anything else is the first row whose leading cell equals it.
fn field_row(field: &Value, database: &[Value], width: usize, rows: usize) -> Result<usize, Error> {
    if let Value::Number(n) = field {
        let index = *n as usize;
        if *n < 0.0 || index >= rows {
            return Err(Error::Arithmetic(ValueError::NotANumber));
        }
        return Ok(index);
    }
    for row in 0..rows {
        if &database[row * width] == field {
            return Ok(row);
        }
    }
    // The reference yields an error value when no field row carries the name.
    Err(Error::Arithmetic(ValueError::NotANumber))
}

/// The data columns whose record every applicable criteria row accepts.
///
/// The returned index counts from the first data column, so index `i` is the
/// cell at grid column `i + 1` of the field's row.
fn matching_records(
    database: &[Value],
    width: usize,
    rows: usize,
    criteria: &[Value],
    criteria_width: usize,
) -> Vec<usize> {
    let columns = width.saturating_sub(1);
    if criteria_width == 0 {
        return (0..columns).collect();
    }
    let criteria_rows = criteria.len() / criteria_width;
    // Every record column starts as a match; a criteria row that names one of
    // its fields and does not hold clears it.
    let mut matched = vec![true; columns];
    for field_row in 1..rows {
        let field_name = &database[field_row * width];
        for record in 0..columns {
            let cell = &database[field_row * width + record + 1];
            let mut named = false;
            let mut holds = false;
            for criteria_row in 0..criteria_rows {
                let base = criteria_row * criteria_width;
                if &criteria[base] != field_name {
                    continue;
                }
                named = true;
                for column in 1..criteria_width {
                    if holds {
                        break;
                    }
                    let test = &criteria[base + column];
                    if is_wildcard(test) || criterion_holds(cell, test) {
                        holds = true;
                    }
                }
            }
            if named {
                matched[record] &= holds;
            }
        }
    }
    matched
        .iter()
        .enumerate()
        .filter(|&(_, &m)| m)
        .map(|(i, _)| i)
        .collect()
}

/// Whether a criterion cell matches every value: blank, or a lone `*`.
fn is_wildcard(test: &Value) -> bool {
    matches!(test, Value::Empty) || matches!(test, Value::Text(t) if t == "*")
}

/// Whether one cell satisfies one criterion.
///
/// The criterion is written as text, with an optional leading comparison
/// operator. A numeric target matches only a numeric cell, and the reference
/// compares a non-numeric cell to it as `NaN`, so `<>` holds and the others do
/// not. Any other target is compared to the cell's text exactly.
fn criterion_holds(cell: &Value, test: &Value) -> bool {
    let text = test.to_string();
    let (operator, rest) = split_operator(&text);
    if let Ok(target) = rest.parse::<f64>() {
        return match cell {
            Value::Number(n) => compare(*n, operator, target),
            _ => operator == "<>",
        };
    }
    let value = cell.to_string();
    match operator {
        "=" => value == rest,
        "<>" => value != rest,
        _ => false,
    }
}

/// Applies a comparison operator to two numbers.
fn compare(left: f64, operator: &str, right: f64) -> bool {
    match operator {
        "=" => left == right,
        "<>" => left != right,
        "<" => left < right,
        "<=" => left <= right,
        ">" => left > right,
        ">=" => left >= right,
        _ => false,
    }
}
