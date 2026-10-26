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

fn text(specs: &str, formula: &str) -> String {
    match eval(specs, formula) {
        Ok(Value::Text(t)) => t,
        other => panic!("{formula} on {specs:?} gave {other:?}, expected text"),
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

#[test]
fn rounding_family_matches_the_reference() {
    // CEILING and FLOOR follow the sign of the significance; MROUND and the
    // EVEN/ODD pair round away from zero.
    assert_eq!(num("", "=CEILING(2.5,1)"), 3.0);
    assert_eq!(num("", "=CEILING(-2.5,1)"), -2.0);
    assert_eq!(num("", "=CEILING(-2.5,-1)"), -3.0);
    assert_eq!(num("", "=FLOOR(2.5,1)"), 2.0);
    assert_eq!(num("", "=FLOOR(-2.5,-1)"), -2.0);
    assert_eq!(num("", "=FLOOR(-2.5,1)"), -3.0);
    assert_eq!(num("", "=MROUND(10,3)"), 9.0);
    assert_eq!(num("", "=MROUND(-10,-3)"), -9.0);
    assert_eq!(num("", "=EVEN(1.5)"), 2.0);
    assert_eq!(num("", "=EVEN(-1.5)"), -2.0);
    assert_eq!(num("", "=ODD(1.5)"), 3.0);
    assert_eq!(num("", "=ODD(-1.5)"), -3.0);
    // ODD(0) is 1 and EVEN(0) is 0.
    assert_eq!(num("", "=ODD(0)"), 1.0);
    assert_eq!(num("", "=EVEN(0)"), 0.0);
    // TRUNC drops towards zero; ROUNDDOWN with a positive value agrees.
    assert_eq!(num("", "=TRUNC(2.567,2)"), 2.56);
    assert_eq!(num("", "=TRUNC(-2.567,1)"), -2.5);
    assert_eq!(num("", "=QUOTIENT(-5,2)"), -2.0);
    // A zero significance is a division by zero, not a silent zero.
    assert!(eval("", "=CEILING(1,0)").is_err());
    // A positive value with a negative significance has no answer.
    assert!(eval("", "=CEILING(1,-1)").is_err());
    assert!(eval("", "=FLOOR(1,-1)").is_err());
}

#[test]
fn logarithms_and_powers_match_the_reference() {
    assert_eq!(num("", "=LOG(8,2)"), 3.0);
    assert_eq!(num("", "=LOG(100)"), 2.0);
    assert_eq!(num("", "=LOG10(1000)"), 3.0);
    assert_eq!(num("", "=EXP(0)"), 1.0);
    assert_eq!(num("", "=FACT(5)"), 120.0);
    assert_eq!(num("", "=FACT(0)"), 1.0);
    assert_eq!(num("", "=GCD(24,36)"), 12.0);
    assert_eq!(num("", "=LCM(4,6)"), 12.0);
    assert_eq!(num("", "=SUMSQ(1,2,3)"), 14.0);
    // A non-positive argument has no real logarithm; the reference reports a
    // number error rather than a NaN.
    assert!(eval("", "=LN(0)").is_err());
    assert!(eval("", "=LOG10(-1)").is_err());
    assert!(eval("", "=FACT(-1)").is_err());
}

#[test]
fn suproduct_multiplies_position_by_position() {
    assert_eq!(
        num("A1=1;A2=2;A3=3;B1=4;B2=5;B3=6", "=SUMPRODUCT(A1:A3,B1:B3)"),
        32.0
    );
    // Mismatched lengths are an error rather than a partial product.
    assert!(eval("A1=1;A2=2;B1=4", "=SUMPRODUCT(A1:A2,B1)").is_err());
}

#[test]
fn conditional_aggregates_align_ranges_by_position() {
    let data = "A1=1;A2=2;A3=3;A4=4;B1=10;B2=20;B3=30;B4=40";
    assert_eq!(num(data, "=SUMIFS(B1:B4,A1:A4,\">2\")"), 70.0);
    assert_eq!(num(data, "=COUNTIFS(A1:A4,\">2\")"), 2.0);
    assert_eq!(num(data, "=AVERAGEIFS(B1:B4,A1:A4,\">2\")"), 35.0);
    assert_eq!(num(data, "=MAXIFS(B1:B4,A1:A4,\">2\")"), 40.0);
    assert_eq!(num(data, "=MINIFS(B1:B4,A1:A4,\">2\")"), 30.0);
    // Two criteria must both hold.
    let labels = "A1=1;B1=a;A2=2;B2=b;A3=3;B3=a;A4=4;B4=b;C1=10;C2=20;C3=30;C4=40";
    assert_eq!(num(labels, "=SUMIFS(C1:C4,A1:A4,\">1\",B1:B4,\"a\")"), 30.0);
    assert_eq!(num(labels, "=COUNTIFS(A1:A4,\">1\",B1:B4,\"a\")"), 1.0);
    // No match is zero for the sums and the extrema, and an error for the
    // average, because there is nothing to divide by.
    assert_eq!(num(data, "=SUMIFS(B1:B4,A1:A4,\">99\")"), 0.0);
    assert_eq!(num(data, "=COUNTIFS(A1:A4,\">99\")"), 0.0);
    assert_eq!(num(data, "=MAXIFS(B1:B4,A1:A4,\">99\")"), 0.0);
    assert!(eval(data, "=AVERAGEIFS(B1:B4,A1:A4,\">99\")").is_err());
}

#[test]
fn text_search_and_case_functions_match_the_reference() {
    assert_eq!(num("", "=FIND(\"b\",\"abcb\")"), 2.0);
    assert_eq!(num("", "=FIND(\"b\",\"abcb\",3)"), 4.0);
    // FIND is case-sensitive; SEARCH is not.
    assert!(eval("", "=FIND(\"B\",\"abc\")").is_err());
    assert_eq!(num("", "=SEARCH(\"B\",\"abcb\")"), 2.0);
    // A missing needle is a number error, not a zero.
    assert!(eval("", "=FIND(\"z\",\"abc\")").is_err());
    assert_eq!(num("", "=CODE(\"A\")"), 65.0);
    assert_eq!(num("", "=UNICODE(\"\u{3bb}\")"), 955.0);
    assert_eq!(text("", "=CHAR(65)"), "A");
    assert_eq!(text("", "=UNICHAR(955)"), "\u{3bb}");
    // PROPER treats a non-alphanumeric character as a word boundary.
    assert_eq!(text("", "=PROPER(\"o brien\")"), "O Brien");
    assert_eq!(text("", "=T(\"x\")"), "x");
    assert_eq!(text("", "=T(5)"), "");
    assert_eq!(num("", "=N(TRUE)"), 1.0);
    assert_eq!(num("", "=N(\"x\")"), 0.0);
}

#[test]
fn text_and_value_round_trip_numbers() {
    assert_eq!(text("", "=TEXT(1234.567,\"#,##0.00\")"), "1,234.57");
    assert_eq!(text("", "=TEXT(1234.567,\"#,##0\")"), "1,235");
    assert_eq!(text("", "=TEXT(0.5,\"0%\")"), "50%");
    assert_eq!(text("", "=TEXT(0.125,\"0.0%\")"), "12.5%");
    assert_eq!(num("", "=VALUE(\"1,234.5\")"), 1234.5);
    assert_eq!(num("", "=VALUE(\"12%\")"), 0.12);
    // Text that is not a number is a number error, not zero.
    assert!(eval("", "=VALUE(\"abc\")").is_err());
}
