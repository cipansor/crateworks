//! Differential regression tests.
//!
//! Each expectation here was confirmed against two independent engines
//! (Microsoft Excel semantics via HyperFormula and the Python `formulas`
//! package) before being written down, so the test records what a spreadsheet
//! actually does rather than what the implementation happened to do.
//!
//! The cases cover the reference-versus-literal rule: a value read from a cell
//! is treated differently from the same value written directly in a formula.
//! `SUM(A1)` ignores a logical or text cell, while `SUM(TRUE)` coerces it.

use std::collections::HashMap;

use spreadsheet_formula::{CellRef, Evaluator, Sheet, Value};

/// A grid built from `"A1=1;B2=text"` style specs.
struct Grid(HashMap<(u32, u32), Value>);

impl Grid {
    fn new(specs: &str) -> Self {
        let mut map = HashMap::new();
        for spec in specs.split(';').filter(|s| !s.is_empty()) {
            let (addr, raw) = spec.split_once('=').expect("addr=value");
            let letters: String = addr.chars().take_while(char::is_ascii_alphabetic).collect();
            let row: u32 = addr[letters.len()..].parse().expect("row number");
            let column = letters
                .bytes()
                .fold(0u32, |acc, b| acc * 26 + u32::from(b - b'A' + 1))
                - 1;
            let value = if let Ok(n) = raw.parse::<f64>() {
                Value::Number(n)
            } else if raw == "TRUE" {
                Value::Bool(true)
            } else if raw == "FALSE" {
                Value::Bool(false)
            } else {
                Value::Text(raw.to_string())
            };
            map.insert((column, row - 1), value);
        }
        Self(map)
    }
}

impl Sheet for Grid {
    fn value(&self, cell: CellRef) -> Value {
        self.0
            .get(&(cell.column, cell.row))
            .cloned()
            .unwrap_or(Value::Empty)
    }
}

fn eval(specs: &str, formula: &str) -> Result<Value, String> {
    Evaluator::new(&Grid::new(specs))
        .eval(formula)
        .map_err(|e| e.to_string())
}

fn num(specs: &str, formula: &str) -> f64 {
    match eval(specs, formula) {
        Ok(Value::Number(n)) => n,
        other => panic!("{formula} on {specs:?} gave {other:?}, expected a number"),
    }
}

#[test]
fn aggregation_over_a_range_ignores_text_logicals_and_blanks() {
    // Text and blanks contribute nothing, and a logical cell is not a number.
    assert_eq!(num("A1=1;A2=2;A3=3", "=SUM(A1:A3)"), 6.0);
    assert_eq!(num("A1=apple;A2=banana;A3=cherry", "=SUM(A1:A3)"), 0.0);
    assert_eq!(num("A1=TRUE;A2=1;A3=2", "=SUM(A1:A3)"), 3.0);
    assert_eq!(num("A1=TRUE;A2=1;A3=2", "=COUNT(A1:A3)"), 2.0);
    assert_eq!(num("A1=1;A2=;A3=3", "=SUM(A1:A3)"), 4.0);
    // The same values written directly are coerced instead.
    assert_eq!(num("", "=SUM(TRUE,1)"), 2.0);
    assert_eq!(num("", "=SUM(1,\"2\")"), 3.0);
}

#[test]
fn min_and_max_ignore_text_but_a_range_with_no_numbers_is_zero() {
    assert_eq!(num("A1=1;A2=banana;A3=3", "=MIN(A1:A3)"), 1.0);
    assert_eq!(num("A1=1;A2=banana;A3=3", "=MAX(A1:A3)"), 3.0);
    // Excel returns 0, not a blank, when a range holds no numbers.
    assert_eq!(num("A1=apple;A2=banana", "=MIN(A1:A2)"), 0.0);
    assert_eq!(num("A1=apple;A2=banana", "=MAX(A1:A2)"), 0.0);
}

#[test]
fn empty_aggregates_report_errors_or_the_empty_product() {
    // Both engines report #DIV/0! for AVERAGE and #NUM! for MEDIAN, and 1 for
    // the empty product.
    assert!(eval("A1=;A2=;A3=", "=AVERAGE(A1:A3)").is_err());
    assert!(eval("A1=;A2=;A3=", "=MEDIAN(A1:A3)").is_err());
    assert_eq!(num("A1=;A2=;A3=", "=PRODUCT(A1:A3)"), 1.0);
    assert_eq!(num("A1=;A2=;A3=", "=SUM(A1:A3)"), 0.0);
    // A literal that is not a number is a type error rather than ignored.
    assert!(eval("", "=AVERAGE(\"x\")").is_err());
}

#[test]
fn criteria_match_only_the_matching_type() {
    // `>1` counts numbers only, so the text cell is not "greater than 1".
    assert_eq!(num("A1=1;A2=apple;A3=3", "=COUNTIF(A1:A3,\">1\")"), 1.0);
    // A text criterion never matches a number.
    assert_eq!(num("A1=1;A2=2;A3=3", "=COUNTIF(A1:A3,\"*2*\")"), 0.0);
}

#[test]
fn criteria_support_wildcards() {
    let fruits = "C1=apple;C2=banana;C3=APPLE";
    assert_eq!(num(fruits, "=COUNTIF(C1:C3,\"apple\")"), 2.0);
    assert_eq!(num(fruits, "=COUNTIF(C1:C3,\"a*\")"), 2.0);
    assert_eq!(num(fruits, "=COUNTIF(C1:C3,\"*a*\")"), 3.0);
    assert_eq!(num(fruits, "=COUNTIF(C1:C3,\"?pple\")"), 2.0);
    assert_eq!(num(fruits, "=COUNTIF(C1:C3,\"*e\")"), 2.0);
}
