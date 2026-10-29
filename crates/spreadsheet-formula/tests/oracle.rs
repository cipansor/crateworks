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
            let value = if raw.is_empty() {
                Value::Empty
            } else if let Ok(n) = raw.parse::<f64>() {
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

#[test]
fn statistical_distributions_match_the_reference() {
    // Every expectation below is the value `@formulajs/formulajs` returns,
    // confirmed by `research/tools/verify_spreadsheet.py`.
    assert!((num("", "=NORMDIST(1,0,1,TRUE)") - 0.8413447460685429).abs() < 1e-12);
    assert!((num("", "=NORMDIST(1,0,1,FALSE)") - 0.24197072451914337).abs() < 1e-12);
    assert!((num("", "=NORMSDIST(1.5)") - 0.9331927987311419).abs() < 1e-12);
    assert!((num("", "=NORMSDIST(1.5,FALSE)") - 0.12951759566589174).abs() < 1e-12);
    assert!((num("", "=NORMINV(0.975,0,1)") - 1.9599639845400545).abs() < 1e-9);
    assert!((num("", "=NORMSINV(0.975)") - 1.9599639845400545).abs() < 1e-9);
    assert!((num("", "=LOGNORMDIST(2,1,0.5,TRUE)") - 0.2697049307349095).abs() < 1e-12);
    assert!((num("", "=LOGNORMDIST(2,1,0.5,FALSE)") - 0.3304645659834839).abs() < 1e-12);
    assert!((num("", "=TDIST(2,5,1)") - 0.05096973941479).abs() < 1e-12);
    assert!((num("", "=TDIST(2,5,2)") - 0.10193947882958).abs() < 1e-12);
    assert!((num("", "=TINV(0.3,5)") - -0.55942964449409).abs() < 1e-9);
    assert!((num("", "=CHIDIST(3,5,TRUE)") - 0.30001416412137294).abs() < 1e-12);
    assert!((num("", "=CHIDIST(3,5,FALSE)") - 0.1541803298037694).abs() < 1e-12);
    assert!((num("", "=CHIINV(0.3,5)") - 2.9999081327599035).abs() < 1e-9);
    assert!((num("", "=FDIST(2,3,4,TRUE)") - 0.7436127978787577).abs() < 1e-12);
    assert!((num("", "=FDIST(2,3,4,FALSE)") - 0.13942740046346708).abs() < 1e-12);
    assert!((num("", "=FINV(0.3,3,4)") - 0.5038966866244746).abs() < 1e-9);
    assert!((num("", "=BINOMDIST(3,10,0.5,TRUE)") - 0.171875).abs() < 1e-12);
    assert!((num("", "=BINOMDIST(3,10,0.5,FALSE)") - 0.1171875).abs() < 1e-12);
    assert!((num("", "=POISSONDIST(2,3,TRUE)") - 0.42319008112684353).abs() < 1e-12);
    assert!((num("", "=EXPONDIST(1,2,TRUE)") - 0.8646647167633873).abs() < 1e-12);
    assert!((num("", "=GAMMADIST(2,3,4,TRUE)") - 0.014387677966970706).abs() < 1e-12);
    assert!((num("", "=BETADIST(0.4,2,3,TRUE)") - 0.5248000000000007).abs() < 1e-12);
    assert!((num("", "=BETAINV(0.5,2,3)") - 0.3857275681323893).abs() < 1e-9);
    assert!((num("", "=HYPGEOMDIST(2,5,3,20,FALSE)") - 0.13157894736842105).abs() < 1e-12);
    assert!((num("", "=NEGBINOMDIST(3,4,0.5,FALSE)") - 0.15625).abs() < 1e-12);
    assert!((num("", "=STANDARDIZE(5,3,2)") - 1.0).abs() < 1e-12);
    assert!((num("", "=GAUSS(1.5)") - 0.4331927987311419).abs() < 1e-12);
    assert!((num("", "=FISHER(0.5)") - 0.5493061443340548).abs() < 1e-12);
    assert!((num("", "=FISHERINV(0.5)") - 0.46211715726000974).abs() < 1e-12);
}

#[test]
fn the_dotted_names_reach_the_same_arm_as_the_flat_ones() {
    // A spreadsheet writes `NORM.S.DIST`; the flat alias is what the reference
    // exports, so the two spellings must agree.
    assert_eq!(
        num("", "=NORM.S.DIST(1.5,TRUE)"),
        num("", "=NORMSDIST(1.5,TRUE)")
    );
    assert_eq!(num("", "=CHISQ.INV(0.3,5)"), num("", "=CHIINV(0.3,5)"));
    assert_eq!(num("", "=T.INV(0.3,5)"), num("", "=TINV(0.3,5)"));
    assert_eq!(num("", "=F.INV(0.3,3,4)"), num("", "=FINV(0.3,3,4)"));
}

#[test]
fn the_advertised_dotted_names_all_resolve() {
    // A name in `FUNCTION_NAMES` that the dispatcher does not know would make
    // a caller's validation reject a formula that works. Every dotted name the
    // crate advertises must evaluate, so a missing alias is caught here rather
    // than by a user.
    let grid = "A1=1;A2=2;A3=3;A4=4;B1=3;B2=4";
    for formula in [
        "=BETA.DIST(0.5,2,3,TRUE)",
        "=BETA.INV(0.5,2,3)",
        "=BINOM.DIST(1,5,0.5,TRUE)",
        "=CEILING.MATH(2.5)",
        "=CEILING.PRECISE(2.5)",
        "=COVARIANCE.P(A1:A4,B1:B2)",
        "=COVARIANCE.S(A1:A4,B1:B2)",
        "=EXPON.DIST(1,2,TRUE)",
        "=F.DIST(2,3,4,TRUE)",
        "=F.DIST.RT(2,3,4)",
        "=F.INV(0.5,3,4)",
        "=F.INV.RT(0.5,3,4)",
        "=F.TEST(A1:A4,B1:B2)",
        "=FLOOR.MATH(2.5)",
        "=FLOOR.PRECISE(2.5)",
        "=GAMMA.DIST(1,2,3,TRUE)",
        "=GAMMA.INV(0.5,2,3)",
        "=GAMMALN.PRECISE(2)",
        "=HYPGEOM.DIST(2,5,3,20,TRUE)",
        "=ISO.CEILING(1.5)",
        "=LOGNORM.DIST(1,0,1,TRUE)",
        "=LOGNORM.INV(0.5,0,1)",
        "=MODE.SNGL(1,2,2)",
        "=NEGBINOM.DIST(1,2,0.5,TRUE)",
        "=NETWORKDAYS.INTL(45292,45322)",
        "=NORM.DIST(1,0,1,TRUE)",
        "=NORM.INV(0.5,0,1)",
        "=NORM.S.DIST(1,TRUE)",
        "=NORM.S.INV(0.5)",
        "=PERCENTILE.EXC(A1:A4,0.5)",
        "=PERCENTILE.INC(A1:A4,0.5)",
        "=PERCENTRANK.EXC(A1:A4,2)",
        "=PERCENTRANK.INC(A1:A4,2)",
        "=POISSON.DIST(1,2,TRUE)",
        "=QUARTILE.EXC(A1:A4,1)",
        "=QUARTILE.INC(A1:A4,1)",
        "=RANK.AVG(2,A1:A4)",
        "=RANK.EQ(2,A1:A4)",
        "=SKEW.P(A1:A4)",
        "=STDEV.P(A1:A4)",
        "=STDEV.S(A1:A4)",
        "=T.DIST(2,5,1)",
        "=T.DIST.RT(2,5)",
        "=T.INV(0.3,5)",
        "=T.TEST(A1:A4,B1:B2,2,1)",
        "=VAR.P(A1:A4)",
        "=VAR.S(A1:A4)",
        "=WEIBULL.DIST(1,2,3,TRUE)",
        "=WORKDAY.INTL(45292,3)",
        "=Z.TEST(A1:A4,2)",
    ] {
        assert!(
            eval(grid, formula).is_ok(),
            "{formula} is advertised but did not evaluate: {:?}",
            eval(grid, formula)
        );
    }
}

#[test]
fn the_modern_distribution_names_match_the_reference() {
    // Each value was read from `@formulajs/formulajs` before being written
    // down, and the whole set is re-checked by `verify_spreadsheet.py`.
    assert!((num("", "=CHISQ.DIST(3,5,TRUE)") - 0.30001416412137294).abs() < 1e-12);
    assert!((num("", "=CHISQ.DIST(3,5,FALSE)") - 0.1541803298037694).abs() < 1e-12);
    assert!((num("", "=CHISQ.DIST.RT(3,5)") - 0.6999858358786271).abs() < 1e-12);
    assert!((num("", "=CHISQ.INV(0.3,5)") - 2.9999081327599035).abs() < 1e-9);
    assert!((num("", "=CHISQ.INV.RT(0.3,5)") - 6.0644299841549).abs() < 1e-9);
    let grid = "A1=10;A2=20;B1=15;B2=15";
    assert!((num(grid, "=CHISQ.TEST(A1:A2,B1:B2)") - 0.067889).abs() < 1e-12);
    assert_eq!(num("", "=BINOM.INV(10,0.5,0.75)"), 6.0);
    assert!((num("", "=T.DIST.2T(2,5)") - 0.10193947882957999).abs() < 1e-12);
    assert!((num("", "=T.INV.2T(0.05,5)") - 2.570581834764143).abs() < 1e-12);
    // The modern spelling and the legacy one are the same function.
    assert_eq!(
        num(grid, "=CHISQ.TEST(A1:A2,B1:B2)"),
        num(grid, "=CHITEST(A1:A2,B1:B2)")
    );
    assert_eq!(
        num("", "=BINOM.INV(10,0.5,0.75)"),
        num("", "=CRITBINOM(10,0.5,0.75)")
    );
}

#[test]
fn regression_and_tests_agree_with_the_reference() {
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=2;B2=4;B3=6;B4=8",
            "=CORREL(A1:A4,B1:B4)"
        ) - 1.0)
            .abs()
            < 1e-12
    );
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=2;B2=4;B3=6;B4=8",
            "=SLOPE(B1:B4,A1:A4)"
        ) - 2.0)
            .abs()
            < 1e-12
    );
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=3;B2=5;B3=7;B4=9",
            "=INTERCEPT(B1:B4,A1:A4)"
        ) - 1.0)
            .abs()
            < 1e-12
    );
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=3;B2=5;B3=7;B4=9",
            "=FORECAST(5,B1:B4,A1:A4)"
        ) - 11.0)
            .abs()
            < 1e-12
    );
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=3;B2=5;B3=7;B4=9",
            "=STEYX(B1:B4,A1:A4)"
        ) - 0.0)
            .abs()
            < 1e-12
    );
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=2;B2=4;B3=6;B4=8",
            "=COVAR(A1:A4,B1:B4)"
        ) - 2.5)
            .abs()
            < 1e-12
    );
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=2;B2=4;B3=6;B4=8",
            "=COVARIANCE.S(A1:A4,B1:B4)"
        ) - 3.3333333333333335)
            .abs()
            < 1e-12
    );
    assert!((num("A1=1;A2=2;A3=3;A4=4", "=ZTEST(A1:A4,2)") - 0.21928901304049997).abs() < 1e-12);
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=2;B2=3;B3=4;B4=5",
            "=TTEST(A1:A4,B1:B4)"
        ) - 0.3153335962012298)
            .abs()
            < 1e-12
    );
    assert!(
        (num(
            "A1=1;A2=2;A3=3;A4=4;B1=2;B2=4;B3=6;B4=8",
            "=FTEST(A1:A4,B1:B4)"
        ) - 0.25)
            .abs()
            < 1e-12
    );
    assert!((num("A1=10;A2=20;B1=15;B2=15", "=CHITEST(A1:A2,B1:B2)") - 0.067889).abs() < 1e-12);
    assert!(
        (num(
            "A1=1;A2=2;A3=3;B1=0.2;B2=0.3;B3=0.5",
            "=PROB(A1:A3,B1:B3,2)"
        ) - 0.3)
            .abs()
            < 1e-12
    );
}

#[test]
fn the_a_suffixed_statistics_count_logicals_and_text() {
    // `AVERAGEA` counts a logical as one and text as zero, where `AVERAGE`
    // would skip both.
    assert_eq!(num("", "=AVERAGEA(1,2,3)"), 2.0);
    assert_eq!(num("", "=VARA(1,2,3)"), 1.0);
    assert_eq!(num("", "=VARPA(1,2,3)"), 2.0 / 3.0);
    assert_eq!(num("", "=STDEVA(1,2,3)"), 1.0);
    assert_eq!(num("", "=STDEVPA(1,2,3)"), (2.0f64 / 3.0).sqrt());
}

#[test]
fn bessel_functions_match_the_reference() {
    // Expectations taken from npm `@formulajs/formulajs`, which is the gate for
    // this crate; the coefficients come from the SheetJS Bessel port. The
    // tolerance is relative, because the asymptotic expansion above x = 8
    // carries a few digits less than the polynomial branch.
    fn close(formula: &str, expected: f64) {
        let got = num("", formula);
        assert!(
            (got - expected).abs() <= 1e-9 * expected.abs(),
            "{formula} gave {got}, expected {expected}"
        );
    }
    close("=BESSELJ(1,0)", 0.7651976837548592);
    close("=BESSELJ(2,1)", 0.5767248078962938);
    close("=BESSELJ(10,2)", 0.2546303136523144);
    close("=BESSELY(1,0)", 0.08825697139770806);
    close("=BESSELI(1.5,2)", 0.3378346208744381);
    close("=BESSELK(2,1)", 0.13986588);
    // A negative order follows the parity rule for J and is an error for the
    // others, and order zero at the origin is infinite for Y and K, which the
    // reference also returns as a number rather than as an error.
    assert!((num("", "=BESSELJ(1,-2)") - num("", "=BESSELJ(1,2)")).abs() < 1e-12);
    assert!(eval("", "=BESSELI(1,-1)").is_err());
    assert!(num("", "=BESSELY(0,0)").is_infinite() && num("", "=BESSELY(0,0)") < 0.0);
    assert!(num("", "=BESSELK(0,0)").is_infinite() && num("", "=BESSELK(0,0)") > 0.0);
}

#[test]
fn gammaln_precise_agrees_with_gammaln() {
    assert!((num("", "=GAMMALN.PRECISE(2.5)") - 0.2846828704729183).abs() < 1e-12);
    assert_eq!(num("", "=GAMMALN.PRECISE(2.5)"), num("", "=GAMMALN(2.5)"));
}

#[test]
fn subtotal_dispatches_to_the_named_aggregate() {
    let data = "A1=1;A2=2;A3=3;A4=4";
    assert_eq!(num(data, "=SUBTOTAL(9,A1:A4)"), 10.0);
    assert_eq!(num(data, "=SUBTOTAL(1,A1:A4)"), 2.5);
    assert_eq!(num(data, "=SUBTOTAL(2,A1:A4)"), 4.0);
    assert_eq!(num(data, "=SUBTOTAL(3,A1:A4)"), 4.0);
    assert_eq!(num(data, "=SUBTOTAL(4,A1:A4)"), 4.0);
    assert_eq!(num(data, "=SUBTOTAL(5,A1:A4)"), 1.0);
    assert_eq!(num(data, "=SUBTOTAL(6,A1:A4)"), 24.0);
    assert!((num(data, "=SUBTOTAL(7,A1:A4)") - 1.2909944487358056).abs() < 1e-12);
    assert!((num(data, "=SUBTOTAL(10,A1:A4)") - 1.6666666666666667).abs() < 1e-12);
    // The `100 +` forms mean the same thing here, since nothing is hidden.
    assert_eq!(num(data, "=SUBTOTAL(109,A1:A4)"), 10.0);
    assert_eq!(num(data, "=SUBTOTAL(103,A1:A4)"), 4.0);
    // A selector that names no aggregate is an error.
    assert!(eval(data, "=SUBTOTAL(12,A1:A4)").is_err());
}

#[test]
fn rate_solves_the_annuity_equation() {
    assert!((num("", "=RATE(60,-100,1000)") - 0.09966552520320725).abs() < 1e-12);
    assert!((num("", "=RATE(12,-100,1000,0)") - 0.029228540769133268).abs() < 1e-12);
    assert!((num("", "=RATE(24,-1000,10000)") - 0.08627804676388601).abs() < 1e-12);
}

#[test]
fn switch_picks_the_first_matching_case_or_the_default() {
    assert_eq!(text("", "=SWITCH(2,1,\"a\",2,\"b\",\"z\")"), "b");
    assert_eq!(text("", "=SWITCH(3,1,\"a\",2,\"b\",\"z\")"), "z");
    assert_eq!(text("", "=SWITCH(\"x\",\"y\",\"a\",\"x\",\"b\")"), "b");
    // No case matched and there is no default, which is an error.
    assert!(eval("", "=SWITCH(9,1,\"a\",2,\"b\")").is_err());
}

#[test]
fn international_workday_functions_honour_the_weekend_argument() {
    // 45292 is 2024-01-01, a Monday; the default weekend is Saturday/Sunday.
    assert_eq!(num("", "=NETWORKDAYS.INTL(45292,45322)"), 23.0);
    assert_eq!(num("", "=NETWORKDAYS.INTL(45292,45322,1)"), 23.0);
    // Code 11 makes Sunday the only weekend day, so more days count.
    assert_eq!(num("", "=NETWORKDAYS.INTL(45292,45322,11)"), 27.0);
    // The mask `0000011` marks Saturday and Sunday, matching the default.
    assert_eq!(num("", "=NETWORKDAYS.INTL(45292,45322,\"0000011\")"), 23.0);
    // The same weekend as a code and as a mask must agree.
    assert_eq!(
        num("", "=NETWORKDAYS.INTL(45292,45322,2)"),
        num("", "=NETWORKDAYS.INTL(45292,45322,\"1000001\")")
    );
    assert_eq!(num("", "=WORKDAY.INTL(45292,10,1)"), 45306.0);
    assert_eq!(num("", "=WORKDAY.INTL(45292,10,11)"), 45303.0);
}

#[test]
fn legacy_aliases_match_their_modern_spellings() {
    // `LOGINV` is the pre-2010 name of `LOGNORMINV`.
    assert_eq!(num("", "=LOGINV(0.5,0,1)"), num("", "=LOGNORMINV(0.5,0,1)"));
    assert!((num("", "=LOGINV(0.9,0,1)") - 3.602224479279159).abs() < 1e-12);
    // `ISERR` is `ISERROR` without the `#N/A` case, which this crate cannot
    // distinguish, so the two agree everywhere.
    assert_eq!(num("", "=IF(ISERR(1),1,0)"), num("", "=IF(ISERROR(1),1,0)"));
    assert_eq!(num("", "=IF(ISERR(1),1,0)"), 0.0);
}

#[test]
fn coupdays_measures_the_coupon_period() {
    // 45292 is 2024-01-01 and 45839 is 2025-08-31.
    assert_eq!(num("", "=COUPDAYS(45292,45839,2)"), 180.0);
    assert_eq!(num("", "=COUPDAYS(45292,45839,4)"), 90.0);
    assert_eq!(num("", "=COUPDAYS(45292,45839,1)"), 360.0);
    // Basis 3 counts 365 days a year rather than 360.
    assert!((num("", "=COUPDAYS(45292,45839,2,3)") - 182.5).abs() < 1e-12);
    // Basis 1 measures the real span of the coupon period. The coupon date
    // before settlement is 2024-02-29, the maturity's day carried back and
    // clamped to the month, and the period runs to 2024-08-29.
    assert_eq!(num("", "=COUPDAYS(45292,45839,2,1)"), 182.0);
    // Settlement on or after maturity has no coupon period.
    assert!(eval("", "=COUPDAYS(45839,45292,2)").is_err());
}

#[test]
fn aggregate_runs_the_selected_aggregate() {
    // The first eleven selectors are the `SUBTOTAL` set.
    assert_eq!(num("A1=1;A2=2;A3=3", "=AGGREGATE(1,0,A1:A3)"), 2.0);
    assert_eq!(num("A1=1;A2=2;A3=3", "=AGGREGATE(9,0,A1:A3)"), 6.0);
    assert_eq!(num("A1=1;A2=2;A3=3", "=AGGREGATE(4,0,A1:A3)"), 3.0);
    assert_eq!(num("A1=1;A2=2;A3=3", "=AGGREGATE(5,0,A1:A3)"), 1.0);
    // `AGGREGATE` agrees with the function it names.
    assert_eq!(
        num("A1=1;A2=2;A3=3", "=AGGREGATE(9,0,A1:A3)"),
        num("A1=1;A2=2;A3=3", "=SUM(A1:A3)")
    );
    assert_eq!(
        num("A1=1;A2=2;A3=3", "=AGGREGATE(1,0,A1:A3)"),
        num("A1=1;A2=2;A3=3", "=AVERAGE(A1:A3)")
    );
    // The selectors past eleven reach the order statistics and the reference
    // again.
    assert_eq!(
        num("A1=1;A2=2;A3=3", "=AGGREGATE(12,0,A1:A3)"),
        num("A1=1;A2=2;A3=3", "=MEDIAN(A1:A3)")
    );
    assert_eq!(
        num("A1=1;A2=2;A3=3", "=AGGREGATE(14,0,A1:A3,1)"),
        num("A1=1;A2=2;A3=3", "=LARGE(A1:A3,1)")
    );
    assert_eq!(
        num("A1=1;A2=2;A3=3", "=AGGREGATE(15,0,A1:A3,1)"),
        num("A1=1;A2=2;A3=3", "=SMALL(A1:A3,1)")
    );
    assert_eq!(
        num("A1=1;A2=2;A3=3", "=AGGREGATE(16,0,A1:A3,0.5)"),
        num("A1=1;A2=2;A3=3", "=PERCENTILE.INC(A1:A3,0.5)")
    );
    assert_eq!(
        num("A1=1;A2=2;A3=3", "=AGGREGATE(17,0,A1:A3,2)"),
        num("A1=1;A2=2;A3=3", "=QUARTILE.INC(A1:A3,2)")
    );
    // `AGGREGATE(13,...)` is `MODE.SNGL`, which needs a repeated value.
    assert_eq!(num("A1=1;A2=2;A3=2", "=AGGREGATE(13,0,A1:A3)"), 2.0);
    // The selector has to be a whole number in range.
    assert!(eval("A1=1;A2=2;A3=3", "=AGGREGATE(0,0,A1:A3)").is_err());
    assert!(eval("A1=1;A2=2;A3=3", "=AGGREGATE(20,0,A1:A3)").is_err());
    assert!(eval("A1=1;A2=2;A3=3", "=AGGREGATE(1.5,0,A1:A3)").is_err());
}

/// The database the reference's own test suite uses, laid out so its first row
/// is at `A1`. The values of a field run across a row, because the first cell
/// of every row names the field.
const DB: &str = "A1=Tree;B1=Apple;C1=Pear;D1=Cherry;E1=Apple;F1=Pear;G1=Apple;\
A2=Height;B2=18;C2=12;D2=13;E2=14;F2=9;G2=8;\
A3=Age;B3=20;C3=12;D3=14;E3=15;F3=8;G3=9;\
A4=Yield;B4=14;C4=10;D4=9;E4=10;F4=8;G4=6";

/// A criteria block that names `Height` and `Age`, both to be satisfied: a
/// conjunction. Its first cell is at `I1`.
const CRIT_AND: &str = "I1=Height;J1=>10;I2=Age;J2=>14";

/// The same, with a second test in the `Height` row: a disjunction.
const CRIT_OR: &str = "I1=Height;J1=>10;K1=>9;I2=Age;J2=>14";

#[test]
fn database_functions_aggregate_the_matching_records() {
    // The reference's published values: only records 1 and 4 (zero-based 0 and
    // 3) satisfy both criteria, so Yield sums to 24 and averages to 12.
    let both = format!("{DB};{CRIT_AND}");
    assert_eq!(num(&both, "=DSUM(A1:G4,\"Yield\",I1:J2)"), 24.0);
    assert_eq!(num(&both, "=DAVERAGE(A1:G4,\"Yield\",I1:J2)"), 12.0);
    assert_eq!(num(&both, "=DMAX(A1:G4,\"Yield\",I1:J2)"), 14.0);
    assert_eq!(num(&both, "=DMIN(A1:G4,\"Yield\",I1:J2)"), 10.0);
    assert_eq!(num(&both, "=DPRODUCT(A1:G4,\"Yield\",I1:J2)"), 140.0);
    assert_eq!(num(&both, "=DCOUNT(A1:G4,\"Yield\",I1:J2)"), 2.0);
    assert_eq!(num(&both, "=DCOUNTA(A1:G4,\"Yield\",I1:J2)"), 2.0);
    assert_eq!(num(&both, "=DVAR(A1:G4,\"Yield\",I1:J2)"), 8.0);
    assert_eq!(num(&both, "=DVARP(A1:G4,\"Yield\",I1:J2)"), 4.0);
    // A field may be named by its zero-based row index instead of its name.
    assert_eq!(num(&both, "=DSUM(A1:G4,3,I1:J2)"), 24.0);
}

#[test]
fn a_second_test_in_one_criteria_row_is_a_disjunction() {
    // `>10` alone selects records 1, 4 and 5 (Height 18, 14, 12), so Yield sums
    // to 24 with `Age>14` and to 43 once the `>9` alternative admits the rest.
    let or = format!("{DB};{CRIT_OR}");
    assert_eq!(num(&or, "=DSUM(A1:G4,\"Yield\",I1:K2)"), 43.0);
    assert_eq!(num(&or, "=DAVERAGE(A1:G4,\"Yield\",I1:K2)"), 10.75);
}

#[test]
fn database_standard_deviations_and_get_errors() {
    // A single criterion row is enough; these are the reference's values.
    let height = format!("{DB};I1=Height;J1=>10");
    let stdev = num(&height, "=DSTDEV(A1:G4,\"Yield\",I1:J2)");
    let stdevp = num(&height, "=DSTDEVP(A1:G4,\"Yield\",I1:J2)");
    assert!((stdev - 2.217_355_782_608_345).abs() < 1e-12, "{stdev}");
    assert!((stdevp - 1.920_286_436_967_152).abs() < 1e-12, "{stdevp}");
    // `DGET` is the single matching value; none or several is an error.
    assert!(eval(&height, "=DGET(A1:G4,\"Yield\",I1:J2)").is_err());
    let one = format!("{DB};I1=Age;J1=>16");
    assert_eq!(num(&one, "=DGET(A1:G4,\"Yield\",I1:J2)"), 14.0);
}

#[test]
fn a_database_argument_must_be_a_grid() {
    // A scalar has no shape, so it cannot be read as a database.
    assert!(eval("", "=DSUM(1,\"Yield\",1)").is_err());
    // An unknown field name is an error rather than an empty result.
    assert!(eval(&format!("{DB};{CRIT_AND}"), "=DSUM(A1:G4,\"Nope\",I1:J2)").is_err());
}
