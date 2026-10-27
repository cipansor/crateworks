# spreadsheet-formula

[![crates.io](https://img.shields.io/crates/v/spreadsheet-formula.svg)](https://crates.io/crates/spreadsheet-formula)
[![docs.rs](https://docs.rs/spreadsheet-formula/badge.svg)](https://docs.rs/spreadsheet-formula)

A spreadsheet formula parser and evaluator, independent of any file format.

```rust
use spreadsheet_formula::{CellRef, Evaluator, Sheet, Value};

struct Grid;
impl Sheet for Grid {
    fn value(&self, cell: CellRef) -> Value {
        match (cell.column, cell.row) {
            (0, 0) => Value::Number(2.0),
            (0, 1) => Value::Number(3.0),
            _ => Value::Empty,
        }
    }
}

assert_eq!(Evaluator::new(&Grid).eval("=SUM(A1:A2)*2").unwrap(), Value::Number(10.0));
```

256 functions plus `IF` and `IFERROR`, which are evaluated lazily. Cell and range
references, absolute references, sheet qualifiers, text concatenation, and
comparison operators. The library covers arithmetic and rounding, trigonometry
and its hyperbolic and reciprocal forms, logarithms, the gamma and error
functions, integer number theory and combinatorics, descriptive statistics,
base conversion and bitwise operations, roman numerals, text case, search and
formatting, date and time, complex numbers, financial and depreciation
calculations, lookup and reference, and the conditional aggregates.

The crate is deliberately independent of any file format. It takes a formula
string and a way to resolve cell references, so it can be used with `xlsx`,
`ods`, or a database of cells without pulling in a spreadsheet reader.

## Function groups

* Arithmetic and rounding - `SUM`, `PRODUCT`, `ABS`, `SIGN`, `MOD`, `QUOTIENT`,
  `INT`, `TRUNC`, `ROUND`, `ROUNDUP`, `ROUNDDOWN`, `MROUND`, `CEILING`, `FLOOR`,
  `EVEN`, `ODD`.
* Trigonometry - `SIN`, `COS`, `TAN`, `ASIN`, `ACOS`, `ATAN`, `ATAN2`, `DEGREES`,
  `RADIANS`, `SQRTPI`, and the reciprocal and hyperbolic forms `CSC`, `SEC`,
  `COT`, `ACOT`, `SINH`, `COSH`, `TANH`, `ASINH`, `ACOSH`, `ATANH`, `CSCH`,
  `SECH`, `COTH`, `ACOTH`.
* Logarithms, powers and special functions - `EXP`, `LN`, `LOG`, `LOG10`,
  `POWER`, `SQRT`, `PI`, `GAMMA`, `GAMMALN`, `ERF`, `ERFC`.
* Number theory and combinatorics - `GCD`, `LCM`, `FACT`, `FACTDOUBLE`, `COMBIN`,
  `COMBINA`, `PERMUT`, `PERMUTATIONA`, `MULTINOMIAL`.
* Descriptive statistics - `AVERAGE`, `AVERAGEA`, `MEDIAN`, `MIN`, `MAX`, `MINA`,
  `MAXA`, `AVEDEV`, `DEVSQ`, `GEOMEAN`, `HARMEAN`, `VARS`, `VARP`, `STDEVS`,
  `STDEVP`, `KURT`, `SKEW`, `SKEWP`, `LARGE`, `SMALL`, `MODESNGL`, `TRIMMEAN`,
  `PERCENTILEINC`, `PERCENTILEEXC`, `QUARTILEINC`, `QUARTILEEXC`, `RANKEQ`,
  `RANKAVG`, `SERIESSUM`, `SUMX2MY2`, `SUMX2PY2`, `SUMXMY2`.
* Base conversion and bitwise - `BIN2DEC`, `HEX2DEC`, `OCT2DEC`, `DEC2BIN`,
  `DEC2HEX`, `DEC2OCT`, `BASE`, `DECIMAL`, `BITAND`, `BITOR`, `BITXOR`,
  `BITLSHIFT`, `BITRSHIFT`, `ROMAN`, `ARABIC`.
* Information - `ISBLANK`, `ISNUMBER`, `ISTEXT`, `ISNONTEXT`, `ISLOGICAL`,
  `ISEVEN`, `ISODD`, `TYPE`, `DELTA`, `GESTEP`, `N`, `T`, `XOR`.
* Date and time - `DATE`, `DATEVALUE`, `DAY`, `MONTH`, `YEAR`, `TIME`,
  `TIMEVALUE`, `HOUR`, `MINUTE`, `SECOND`, `EDATE`, `EOMONTH`, `DAYS`,
  `DAYS360`, `DATEDIF`, `YEARFRAC`, `WEEKDAY`, `WEEKNUM`, `ISOWEEKNUM`,
  `NETWORKDAYS`, `WORKDAY`.
* Complex numbers - `COMPLEX`, `IMREAL`, `IMAGINARY`, `IMABS`, `IMARGUMENT`,
  `IMCONJUGATE`, `IMSUM`, `IMSUB`, `IMPRODUCT`, `IMDIV`, `IMPOWER`, `IMSQRT`,
  `IMEXP`, `IMLN`, `IMLOG10`, `IMLOG2`, `IMSIN`, `IMCOS`, `IMTAN`, `IMCOT`,
  `IMSEC`, `IMCSC`, `IMSINH`, `IMCOSH`, `IMSECH`, `IMCSCH`.
* Financial - `FV`, `PV`, `PMT`, `NPER`, `IPMT`, `PPMT`, `CUMIPMT`, `CUMPRINC`,
  `ISPMT`, `NPV`, `IRR`, `MIRR`, `XNPV`, `XIRR`, `SLN`, `SYD`, `DDB`, `DB`,
  `EFFECT`, `NOMINAL`, `RRI`, `PDURATION`, `DOLLARDE`, `DOLLARFR`, `DISC`,
  `PRICEDISC`, `TBILLPRICE`, `TBILLYIELD`, `TBILLEQ`, `FVSCHEDULE`.
* Lookup and reference - `VLOOKUP`, `HLOOKUP`, `LOOKUP`, `MATCH`, `INDEX`,
  `CHOOSE`, `ROW`, `COLUMN`, `ROWS`, `COLUMNS`.
* Text - `CONCAT`, `CONCATENATE`, `LEFT`, `RIGHT`, `MID`, `LEN`, `LOWER`,
  `UPPER`, `PROPER`, `TRIM`, `REPT`, `SUBSTITUTE`, `REPLACE`, `FIND`, `SEARCH`,
  `EXACT`, `TEXTJOIN`, `CHAR`, `CODE`, `UNICHAR`, `UNICODE`, `CLEAN`, `TEXT`,
  `VALUE`, `NUMBERVALUE`, `DOLLAR`, `FIXED`.

## Scope

The evaluator covers arithmetic, comparisons, text concatenation, cell and
range references, and a growing set of functions. It does not implement
array formulas, volatile functions or iterative recalculation; a cyclic
reference is reported as an error rather than resolved.

The base-conversion and bitwise functions follow the spreadsheet convention of
a fixed ten-digit field, so a ten-character binary, octal or hexadecimal string
whose leading digit is at or above half the base is a negative number. `BASE`
accepts a radix from 2 to 36, and `DECIMAL` parses one.

`FUNCTION_NAMES` is the list of every function the crate implements, and a caller
can validate a formula against it. The conditional ones are `IF` and `IFERROR`,
which the evaluator handles itself so that they do not evaluate every argument,
plus `IFS`, `COUNTIF`, `COUNTIFS`, `SUMIF`, `SUMIFS`, `AVERAGEIF`, `AVERAGEIFS`,
`MAXIFS`, `MINIFS` and `COUNTBLANK`. They take the spreadsheet criteria of a bare
value or a comparison such as `">3"`, and match text case-insensitively. `SUMIF`
and `AVERAGEIF` take the optional third argument as the range to add up, aligned
by position with the range being tested; the `*IFS` forms take the range first and
then criteria range and criterion pairs.

`TEXT` supports the common number formats - a `%` suffix, a thousands separator
and a fixed number of decimal places - and `VALUE` parses a number back out of
text, including a thousands separator, currency symbol or trailing percent.

### Aggregation rules

Aggregating functions follow the spreadsheet rule that a value read from a cell
is treated differently from the same value written directly. In a reference,
text, logicals and blanks are ignored; written directly, they are coerced:

```rust
# use spreadsheet_formula::{CellRef, Evaluator, Sheet, Value};
# struct Grid;
# impl Sheet for Grid {
#     fn value(&self, cell: CellRef) -> Value {
#         if (cell.column, cell.row) == (0, 0) { Value::Bool(true) } else { Value::Empty }
#     }
# }
// A1 holds TRUE, but a logical in a cell is not a number.
assert_eq!(Evaluator::new(&Grid).eval("=SUM(A1)").unwrap(), Value::Number(0.0));
// Written directly, TRUE is coerced to 1.
assert_eq!(Evaluator::new(&Grid).eval("=SUM(TRUE)").unwrap(), Value::Number(1.0));
```

When a range holds no numbers, `MIN` and `MAX` return `0`, `PRODUCT` returns the
empty product `1`, and `AVERAGE` and `MEDIAN` report an error, matching Excel's
`#DIV/0!` and `#NUM!`.

Criteria follow the same type rules: a numeric criterion such as `">2"` matches
numbers only, a text criterion never matches a number, and `COUNTIF`, `SUMIF` and
`AVERAGEIF` understand the `*` and `?` wildcards, case-insensitively.


Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
