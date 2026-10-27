//! Lookup and reference functions.
//!
//! The functions here work on the shape of a range, not just its values: a
//! `VLOOKUP` needs to know where one column ends and the next begins, so the
//! caller passes the width alongside the row-major values.
//!
//! Two comparison rules run through the whole family:
//!
//! * a text comparison is case-insensitive, because that is what a user sees;
//! * an exact match may use `*` and `?` wildcards, but only when the lookup
//!   value is itself text.

use crate::value::Value;

/// How a lookup decides whether a value matches.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MatchMode {
    /// The first value greater than the target, on a sorted list.
    Approximate,
    /// The first value equal to the target.
    Exact,
    /// The first value less than the target, on a sorted list.
    ApproximateDescending,
}

/// `MATCH`'s third argument: 1, 0 or -1.
pub(crate) fn match_mode(flag: f64) -> MatchMode {
    if flag > 0.0 {
        MatchMode::Approximate
    } else if flag < 0.0 {
        MatchMode::ApproximateDescending
    } else {
        MatchMode::Exact
    }
}

/// Whether `value` matches `target` under the given mode.
///
/// Wildcards are only honoured in exact mode and only when the target is text,
/// matching a spreadsheet's behaviour where `"a*"` is a pattern but the number
/// next to it is not.
pub(crate) fn matches(value: &Value, target: &Value, mode: MatchMode, wildcards: bool) -> bool {
    match (value, target) {
        (Value::Text(haystack), Value::Text(needle)) if mode == MatchMode::Exact => {
            if wildcards {
                crate::functions::wildcard_match(haystack, needle)
            } else {
                haystack.eq_ignore_ascii_case(needle)
            }
        }
        (Value::Text(haystack), Value::Text(needle)) => haystack.eq_ignore_ascii_case(needle),
        _ => match (value.as_number(), target.as_number()) {
            (Ok(a), Ok(b)) => a == b,
            _ => value.to_string() == target.to_string(),
        },
    }
}

/// The position of `target` in `values`, as a zero-based index.
///
/// The two approximate modes differ in what they do when there is no exact
/// match: they settle on the nearest neighbour in the sorted order, and the
/// caller is responsible for the list actually being sorted.
pub(crate) fn find(
    values: &[Value],
    target: &Value,
    mode: MatchMode,
    wildcards: bool,
) -> Option<usize> {
    match mode {
        MatchMode::Exact => values
            .iter()
            .position(|value| matches(value, target, mode, wildcards)),
        MatchMode::Approximate => {
            let mut best = None;
            for (index, value) in values.iter().enumerate() {
                if matches(value, target, MatchMode::Exact, wildcards) {
                    return Some(index);
                }
                // The last value that is still at or below the target.
                if let (Ok(v), Ok(t)) = (value.as_number(), target.as_number())
                    && v <= t
                {
                    best = Some(index);
                }
            }
            best
        }
        MatchMode::ApproximateDescending => {
            let mut best = None;
            for (index, value) in values.iter().enumerate() {
                if matches(value, target, MatchMode::Exact, wildcards) {
                    return Some(index);
                }
                // The first value that is at or above the target.
                if let (Ok(v), Ok(t)) = (value.as_number(), target.as_number())
                    && v >= t
                    && best.is_none()
                {
                    best = Some(index);
                }
            }
            best
        }
    }
}

/// One axis of a lookup: which direction the table is searched in, and how many
/// steps to take afterwards.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Axis {
    /// The row to start at, when the table is searched by column.
    pub(crate) row: usize,
    /// The column to start at, when the table is searched by row.
    pub(crate) column: usize,
    /// How far to walk after the match, as a signed count.
    pub(crate) offset: i64,
}

impl Axis {
    /// A horizontal lookup, `VLOOKUP`: search a column, return from a column to
    /// its right.
    pub(crate) const fn vertical(row: usize, column: usize, offset: i64) -> Self {
        Self {
            row,
            column,
            offset,
        }
    }

    /// A vertical lookup, `HLOOKUP`: search a row, return from a row below it.
    pub(crate) const fn horizontal(row: usize, column: usize, offset: i64) -> Self {
        Self {
            row,
            column,
            offset,
        }
    }
}

/// The line of a table a lookup searches along.
fn search_line(values: &[Value], width: usize, vertical: bool, axis: Axis) -> Vec<&Value> {
    if vertical {
        (axis.row..)
            .take_while(|row| row * width < values.len())
            .filter_map(|row| values.get(row * width + axis.column))
            .collect()
    } else {
        let start = axis.row * width + axis.column;
        values
            .get(start..(start + width).min(values.len()))
            .map(<[Value]>::iter)
            .map(Iterator::collect)
            .unwrap_or_default()
    }
}

/// The value a lookup returns once it knows where the match is.
fn return_value(
    values: &[Value],
    width: usize,
    vertical: bool,
    axis: Axis,
    found: usize,
) -> Option<Value> {
    let rows = values.len().div_ceil(width);
    let offset = axis.offset;
    if vertical {
        // The returned column must stay inside the table; without this check a
        // wide index would wrap into the next row.
        let column = axis.column as i64 + offset;
        if column < 0 || column as usize >= width {
            return None;
        }
        values
            .get((axis.row + found) * width + column as usize)
            .cloned()
    } else {
        let row = axis.row as i64 + offset;
        if row < 0 || row as usize >= rows {
            return None;
        }
        let column = axis.column + found;
        if column >= width {
            return None;
        }
        values.get(row as usize * width + column).cloned()
    }
}

/// A `VLOOKUP` or `HLOOKUP`, sharing one implementation.
pub(crate) fn table_lookup(
    values: &[Value],
    width: usize,
    target: &Value,
    mode: MatchMode,
    vertical: bool,
    axis: Axis,
) -> Option<Value> {
    let line = search_line(values, width, vertical, axis);
    let found = find(
        &line.iter().map(|v| (*v).clone()).collect::<Vec<Value>>(),
        target,
        mode,
        true,
    )?;
    return_value(values, width, vertical, axis, found)
}

/// `INDEX`: the value at a one-based position in a range.
///
/// A range that is a single row or a single column needs only one index, and
/// that index then counts along the row or down the column. A rectangle needs
/// both. The array forms - an index of zero, meaning a whole row or column -
/// have no scalar answer, so they are reported as out of range.
pub(crate) fn index(values: &[Value], width: usize, row: i64, column: i64) -> Option<Value> {
    let rows = values.len().div_ceil(width);
    if width == 0 || rows == 0 {
        return None;
    }
    if rows == 1 {
        let column = usize::try_from(row).ok()?;
        return values.get(column.checked_sub(1)?).cloned();
    }
    if width == 1 {
        let row = usize::try_from(row).ok()?;
        return values.get(row.checked_sub(1)?).cloned();
    }
    let row = usize::try_from(row).ok()?;
    let column = usize::try_from(column).ok()?;
    if row < 1 || row > rows || column < 1 || column > width {
        return None;
    }
    values.get((row - 1) * width + (column - 1)).cloned()
}

/// `LOOKUP`: the value in the last row or column of a range.
pub(crate) fn lookup(values: &[Value], width: usize, target: &Value) -> Option<Value> {
    let rows = values.len().div_ceil(width);
    // A one-dimensional range searches itself; a rectangle searches its first
    // column and returns from its last.
    let (search, result) = if rows == 1 || width == 1 {
        (values.to_vec(), values.to_vec())
    } else {
        let first: Vec<Value> = (0..rows)
            .filter_map(|r| values.get(r * width))
            .cloned()
            .collect();
        let last: Vec<Value> = (0..rows)
            .filter_map(|r| values.get(r * width + width - 1))
            .cloned()
            .collect();
        (first, last)
    };
    let found = find(&search, target, MatchMode::Approximate, false)?;
    result.get(found).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers(values: &[f64]) -> Vec<Value> {
        values.iter().copied().map(Value::Number).collect()
    }

    #[test]
    fn exact_matching_is_case_insensitive() {
        let values = [Value::Text("apple".into()), Value::Text("Banana".into())];
        assert_eq!(
            find(
                &values,
                &Value::Text("APPLE".into()),
                MatchMode::Exact,
                false
            ),
            Some(0)
        );
        assert_eq!(
            find(
                &values,
                &Value::Text("banana".into()),
                MatchMode::Exact,
                false
            ),
            Some(1)
        );
        assert_eq!(
            find(
                &values,
                &Value::Text("cherry".into()),
                MatchMode::Exact,
                false
            ),
            None
        );
    }

    #[test]
    fn wildcards_only_apply_in_exact_mode_on_text() {
        let values = [Value::Text("alpha".into()), Value::Text("beta".into())];
        assert_eq!(
            find(&values, &Value::Text("a*".into()), MatchMode::Exact, true),
            Some(0)
        );
        // Without the wildcard flag the pattern is literal text.
        assert_eq!(
            find(&values, &Value::Text("a*".into()), MatchMode::Exact, false),
            None
        );
    }

    #[test]
    fn approximate_modes_pick_the_nearest_neighbour() {
        let values = numbers(&[10.0, 20.0, 30.0]);
        assert_eq!(
            find(&values, &Value::Number(25.0), MatchMode::Approximate, false),
            Some(1)
        );
        assert_eq!(
            find(&values, &Value::Number(5.0), MatchMode::Approximate, false),
            None
        );
        assert_eq!(
            find(
                &values,
                &Value::Number(25.0),
                MatchMode::ApproximateDescending,
                false
            ),
            Some(2)
        );
        assert_eq!(
            find(
                &values,
                &Value::Number(35.0),
                MatchMode::ApproximateDescending,
                false
            ),
            None
        );
    }

    #[test]
    fn match_mode_reads_the_flag() {
        assert_eq!(match_mode(1.0), MatchMode::Approximate);
        assert_eq!(match_mode(0.0), MatchMode::Exact);
        assert_eq!(match_mode(-1.0), MatchMode::ApproximateDescending);
    }

    #[test]
    fn vertical_lookup_returns_from_a_later_column() {
        // A 3x2 table whose first column is the key.
        let values = numbers(&[1.0, 10.0, 2.0, 20.0, 3.0, 30.0]);
        let got = table_lookup(
            &values,
            2,
            &Value::Number(2.0),
            MatchMode::Exact,
            true,
            Axis::vertical(0, 0, 1),
        );
        assert_eq!(got, Some(Value::Number(20.0)));
        // The offset may point left of the search column as well.
        let got = table_lookup(
            &values,
            2,
            &Value::Number(20.0),
            MatchMode::Exact,
            true,
            Axis::vertical(0, 1, -1),
        );
        assert_eq!(got, Some(Value::Number(2.0)));
    }

    #[test]
    fn horizontal_lookup_returns_from_a_lower_row() {
        let values = numbers(&[1.0, 2.0, 3.0, 10.0, 20.0, 30.0]);
        let got = table_lookup(
            &values,
            3,
            &Value::Number(2.0),
            MatchMode::Exact,
            false,
            Axis::horizontal(0, 0, 1),
        );
        assert_eq!(got, Some(Value::Number(20.0)));
    }

    #[test]
    fn index_reads_a_position() {
        // A 2x3 rectangle.
        let values = numbers(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
        assert_eq!(index(&values, 3, 2, 2), Some(Value::Number(5.0)));
        assert_eq!(index(&values, 3, 1, 3), Some(Value::Number(3.0)));
        // A single row needs only one index, which counts along the row.
        let row = numbers(&[1.0, 2.0, 3.0]);
        assert_eq!(index(&row, 3, 2, 1), Some(Value::Number(2.0)));
        // As does a single column.
        let column = numbers(&[1.0, 2.0, 3.0]);
        assert_eq!(index(&column, 1, 2, 1), Some(Value::Number(2.0)));
        // Out of bounds has no value.
        assert_eq!(index(&values, 3, 3, 1), None);
        assert_eq!(index(&values, 3, 1, 4), None);
        assert_eq!(index(&values, 3, 0, 0), None);
    }

    #[test]
    fn lookup_searches_the_first_column_and_returns_the_last() {
        let values = numbers(&[1.0, 100.0, 2.0, 200.0, 3.0, 300.0]);
        assert_eq!(
            lookup(&values, 2, &Value::Number(2.0)),
            Some(Value::Number(200.0))
        );
        // A value between two keys settles on the lower one.
        assert_eq!(
            lookup(&values, 2, &Value::Number(2.5)),
            Some(Value::Number(200.0))
        );
    }
}
