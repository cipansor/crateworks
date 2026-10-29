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

427 function names - 361 flat and 66 dotted modern spellings that share an
implementation with their flat alias - plus `IF` and `IFERROR`, which are
evaluated lazily along with
`SWITCH`. Cell and range references, absolute references, sheet qualifiers, text
concatenation, and comparison operators. The library covers arithmetic and
rounding, trigonometry and its hyperbolic and reciprocal forms, logarithms, the
gamma and error functions, integer number theory and combinatorics, descriptive statistics and
the statistical distributions, regression and correlation, base conversion and
bitwise operations, roman numerals, text case, search and formatting, date and
time, complex numbers, financial and depreciation calculations, lookup and
reference, the conditional aggregates, and the database functions.

The crate is deliberately independent of any file format. It takes a formula
string and a way to resolve cell references, so it can be used with `xlsx`,
`ods`, or a database of cells without pulling in a spreadsheet reader.

## Function groups

* Arithmetic and rounding - `SUM`, `PRODUCT`, `ABS`, `SIGN`, `MOD`, `QUOTIENT`,
  `INT`, `TRUNC`, `ROUND`, `ROUNDUP`, `ROUNDDOWN`, `MROUND`, `CEILING`, `FLOOR`,
  `CEILING.MATH`, `FLOOR.MATH`, `CEILING.PRECISE`, `FLOOR.PRECISE`, `ISO.CEILING`,
  `EVEN`, `ODD`.
* Trigonometry - `SIN`, `COS`, `TAN`, `ASIN`, `ACOS`, `ATAN`, `ATAN2`, `DEGREES`,
  `RADIANS`, `SQRTPI`, and the reciprocal and hyperbolic forms `CSC`, `SEC`,
  `COT`, `ACOT`, `SINH`, `COSH`, `TANH`, `ASINH`, `ACOSH`, `ATANH`, `CSCH`,
  `SECH`, `COTH`, `ACOTH`.
* Logarithms, powers and special functions - `EXP`, `LN`, `LOG`, `LOG10`,
  `POWER`, `SQRT`, `PI`, `GAMMA`, `GAMMALN`, `GAMMALN.PRECISE`, `ERF`, `ERFC`,
  `BESSELI`, `BESSELJ`, `BESSELK`, `BESSELY`.
* Number theory and combinatorics - `GCD`, `LCM`, `FACT`, `FACTDOUBLE`, `COMBIN`,
  `COMBINA`, `PERMUT`, `PERMUTATIONA`, `MULTINOMIAL`.
* Descriptive statistics - `AVERAGE`, `AVERAGEA`, `MEDIAN`, `MIN`, `MAX`, `MINA`,
  `MAXA`, `AVEDEV`, `DEVSQ`, `GEOMEAN`, `HARMEAN`, `VARS`, `VARP`, `STDEVS`,
  `STDEVP`, `STDEVA`, `STDEVPA`, `VARA`, `VARPA`, `KURT`, `SKEW`, `SKEWP`,
  `LARGE`, `SMALL`, `MODE`, `MODESNGL`, `TRIMMEAN`, `PERCENTILE`, `PERCENTILEINC`,
  `PERCENTILEEXC`, `QUARTILE`, `QUARTILEINC`, `QUARTILEEXC`, `RANK`, `RANKEQ`,
  `RANKAVG`, `PERCENTRANKINC`, `PERCENTRANKEXC`, `SERIESSUM`, `SUMX2MY2`,
  `SUMX2PY2`, `SUMXMY2`, `PROB`. The legacy flat spellings Excel still accepts
  are aliases of the modern ones: `STDEV` (`STDEV.S`), `VAR` (`VAR.S`),
  `PERCENTRANK` (`PERCENTRANK.INC`) and `COVARIANCE` (`COVARIANCE.P`).
* Statistical distributions - `NORMDIST`, `NORMSDIST`, `NORMINV`, `NORMSINV`,
  `LOGNORMDIST`, `LOGNORMINV`, `LOGINV`, `TDIST`, `TINV`, `CHIDIST`, `CHIINV`, `FDIST`,
  `FINV`, `BINOMDIST`, `CRITBINOM`, `POISSONDIST`, `EXPONDIST`, `WEIBULLDIST`,
  `GAMMADIST`, `GAMMAINV`, `BETADIST`, `BETAINV`, `HYPGEOMDIST`, `NEGBINOMDIST`,
  `STANDARDIZE`, `GAUSS`, `PHI`, `FISHER`, `FISHERINV`, `CONFIDENCE`,
  `CONFIDENCE.NORM`, `CONFIDENCE.T`, and the modern dotted spellings
  `CHISQ.DIST`, `CHISQ.DIST.RT`, `CHISQ.INV`, `CHISQ.INV.RT`, `T.DIST.2T`,
  `T.INV.2T`, `BINOM.INV`.
* Regression and hypothesis tests - `CORREL`, `PEARSON`, `RSQ`, `SLOPE`,
  `INTERCEPT`, `STEYX`, `FORECAST`, `COVAR`, `COVARIANCEP`, `COVARIANCES`,
  `ZTEST`, `TTEST`, `FTEST`, `CHITEST`, `CHISQ.TEST`.
* Base conversion and bitwise - `BIN2DEC`, `BIN2HEX`, `BIN2OCT`, `HEX2BIN`,
  `HEX2DEC`, `HEX2OCT`, `OCT2BIN`, `OCT2DEC`, `OCT2HEX`, `DEC2BIN`, `DEC2HEX`,
  `DEC2OCT`, `BASE`, `DECIMAL`, `BITAND`, `BITOR`, `BITXOR`, `BITLSHIFT`,
  `BITRSHIFT`, `ROMAN`, `ARABIC`.
* Unit conversion - `CONVERT`, between 146 units across mass, length, time,
  pressure, energy, power, information, magnetism and more, with the decimal SI
  and the binary prefixes. See the section on `CONVERT` below.
* Information - `ISBLANK`, `ISNUMBER`, `ISTEXT`, `ISNONTEXT`, `ISLOGICAL`,
  `ISERROR`, `ISERR`, `ISEVEN`, `ISODD`, `TYPE`, `DELTA`, `GESTEP`, `N`, `T`,
  `XOR`.
* Date and time - `DATE`, `DATEVALUE`, `DAY`, `MONTH`, `YEAR`, `TIME`,
  `TIMEVALUE`, `HOUR`, `MINUTE`, `SECOND`, `EDATE`, `EOMONTH`, `DAYS`,
  `DAYS360`, `DATEDIF`, `YEARFRAC`, `WEEKDAY`, `WEEKNUM`, `ISOWEEKNUM`,
  `NETWORKDAYS`, `WORKDAY`, `NETWORKDAYS.INTL`, `WORKDAY.INTL`.
* Complex numbers - `COMPLEX`, `IMREAL`, `IMAGINARY`, `IMABS`, `IMARGUMENT`,
  `IMCONJUGATE`, `IMSUM`, `IMSUB`, `IMPRODUCT`, `IMDIV`, `IMPOWER`, `IMSQRT`,
  `IMEXP`, `IMLN`, `IMLOG10`, `IMLOG2`, `IMSIN`, `IMCOS`, `IMTAN`, `IMCOT`,
  `IMSEC`, `IMCSC`, `IMSINH`, `IMCOSH`, `IMSECH`, `IMCSCH`.
* Financial - `FV`, `PV`, `PMT`, `NPER`, `RATE`, `IPMT`, `PPMT`, `CUMIPMT`, `CUMPRINC`,
  `ISPMT`, `NPV`, `IRR`, `MIRR`, `XNPV`, `XIRR`, `SLN`, `SYD`, `DDB`, `DB`,
  `EFFECT`, `NOMINAL`, `RRI`, `PDURATION`, `DOLLARDE`, `DOLLARFR`, `DISC`,
  `PRICEDISC`, `TBILLPRICE`, `TBILLYIELD`, `TBILLEQ`, `ACCRINT`, `FVSCHEDULE`, `COUPDAYS`.
* Lookup and reference - `VLOOKUP`, `HLOOKUP`, `LOOKUP`, `MATCH`, `INDEX`,
  `CHOOSE`, `ROW`, `COLUMN`, `ROWS`, `COLUMNS`.
* Database and aggregation - `SUBTOTAL`, which runs one of eleven aggregates
  (`AVERAGE`, `COUNT`, `COUNTA`, `MAX`, `MIN`, `PRODUCT`, `STDEV.S`, `STDEV.P`,
  `SUM`, `VAR.S`, `VAR.P`) selected by number, and accepts the `100 +` forms
  that mean "ignore hidden rows" elsewhere. `AGGREGATE` is the larger version of
  the same idea: nineteen aggregates - the eleven `SUBTOTAL` names plus `MEDIAN`,
  `MODE.SNGL`, `LARGE`, `SMALL`, `PERCENTILE.INC`, `QUARTILE.INC`,
  `PERCENTILE.EXC` and `QUARTILE.EXC` - selected by number, with an `options`
  argument that says what to skip. Nothing in this evaluator is hidden or an
  error value, so `options` is accepted rather than acted on. The `D`-prefixed
  database functions - `DSUM`, `DAVERAGE`, `DCOUNT`, `DCOUNTA`, `DGET`, `DMAX`,
  `DMIN`, `DPRODUCT`, `DSTDEV`, `DSTDEVP`, `DVAR`, `DVARP` - are described under
  [Database functions](#database-functions).
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

### CONVERT

`CONVERT(value, from, to)` expresses a value in one unit as another. The units
are the ones Excel knows, 146 of them, and the table is generated from
`@formulajs/formulajs` by `research/tools/gen_convert_units.py` rather than
hand-written. A unit is written with its symbol or any of its aliases, so
`CONVERT(1, "L", "gal")` and `CONVERT(1, "lt", "gal")` agree.

A unit may carry a decimal SI prefix (`k`, `m`, `M`) or a binary prefix (`ki`,
`Mi`), applied only when the bare name is not itself a unit: `km` is a kilometre,
but `min` is a minute and `mmHg` a millimetre of mercury. `kibyte` is 1024 bits.
A prefix is tried only after the whole string fails to match a unit, so a
multi-letter symbol always wins.

Converting between two units of different categories, or naming a unit the
crate does not know, is an error, which is how a spreadsheet reports `#N/A`.
The reference instead returns its `{}` object for both; this crate reports the
error so a caller can tell the two apart.

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

### Statistical functions and dotted names

A modern function name may be dotted - `NORM.S.DIST`, `CHISQ.INV.RT`,
`T.INV.2T` - and the tokeniser accepts one wherever a call follows. Both the
dotted spelling and the flat alias a spreadsheet also accepts (`NORMSDIST`,
`CHIINV`) reach the same implementation, so a formula written either way
evaluates the same. The dots are only stripped before dispatch, so the two
spellings cannot drift apart, and every dotted name is listed in
`FUNCTION_NAMES` alongside its flat alias. `BINOM.DIST.RANGE` is the one that is
not a rename: it sums the binomial density over an inclusive interval and has no
flat counterpart.

The distributions, the regression and correlation functions and the hypothesis
tests were ported from the JavaScript `@formulajs/formulajs` library, which
delegates the arithmetic to `jStat`. The kernels reproduce that reference
algorithm for algorithm, including the continued fraction, the Newton iteration
and the guard values, because a formula has to return the number the reference
returns rather than a mathematically equivalent one. The cases in
[`tests/oracle.rs`](https://github.com/cipansor/crateworks/blob/main/crates/spreadsheet-formula/tests/oracle.rs)
are the values the reference produced, and
`research/tools/verify_spreadsheet.py` re-derives them.

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

### Database functions

The `D`-prefixed functions - `DSUM`, `DAVERAGE`, `DCOUNT`, `DCOUNTA`, `DGET`,
`DMAX`, `DMIN`, `DPRODUCT`, `DSTDEV`, `DSTDEVP`, `DVAR`, `DVARP` - aggregate one
field of a database over the records a criteria range selects. A database is a
rectangular range in which the first cell of every row names the field, so a
field's values run across a row rather than down a column; a criteria range has
the same shape, with a field name in the first cell of each row followed by its
tests. A record matches when every criteria row that names one of its fields
holds, so two criteria rows are a conjunction and two tests in one row are a
disjunction:

```rust
# use spreadsheet_formula::{CellRef, Evaluator, Sheet, Value};
# struct Grid;
# impl Sheet for Grid {
#     fn value(&self, cell: CellRef) -> Value {
#         match (cell.column, cell.row) {
#             (0, 0) => Value::Text("Tree".into()),
#             (1, 0) => Value::Text("Apple".into()),
#             (0, 1) => Value::Text("Yield".into()),
#             (1, 1) => Value::Number(14.0),
#             (0, 2) => Value::Text("Height".into()),
#             (1, 2) => Value::Number(18.0),
#             (2, 0) => Value::Text("Height".into()),
#             (3, 0) => Value::Text(">10".into()),
#             _ => Value::Empty,
#         }
#     }
# }
// The database is `A1:B3`, with `Tree` and `Yield` and `Height` naming the
// rows. The field is `Yield` and the criteria range `C1:D1` asks for
// `Height > 10`, which the single record satisfies.
let grid = Grid;
assert_eq!(Evaluator::new(&grid).eval("=DSUM(A1:B3,\"Yield\",C1:D1)").unwrap(), Value::Number(14.0));
```

A field is named by the text in the first cell of its row, or by its zero-based
row index. The matching rule and the aggregations follow the JavaScript
`@formulajs/formulajs` library, this crate's reference for the spreadsheet
functions: a numeric criterion matches only numeric cells and a text criterion is
compared exactly, a criterion of `*` or a blank cell matches every record, and
`DGET` is the one matching value, reporting an error when the criteria select
none or more than one. `DCOUNT` counts the numeric cells of the field, while
`DCOUNTA` counts the non-blank ones, so a blank field value is counted by the
second and not the first.


Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
