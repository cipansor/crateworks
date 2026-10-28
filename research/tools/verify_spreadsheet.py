#!/usr/bin/env python3
"""Regression gate for `crates/spreadsheet-formula` against `@formulajs/formulajs`.

formulajs is the JavaScript port of the spreadsheet function library and the
mature reference for this domain. Every function the crate implements that also
exists in formulajs is called with the same arguments on both sides and the two
answers are compared. A difference is a bug in one of the two.

The Rust side is exercised through the public `Evaluator`, not the internal
`call`, so the gate covers the parser and the evaluator as well as the function
library: each case is a formula string, and the JS side calls the same function
with the same values.

Run:  python3 research/tools/verify_spreadsheet.py [--verbose]
Needs: npm install @formulajs/formulajs
"""

import json
import os
import pathlib
import subprocess
import sys
from datetime import date

ROOT = pathlib.Path(__file__).resolve().parents[2]
CRATE = ROOT / "crates" / "spreadsheet-formula"
CARGO_ENV = {
    **os.environ,
    "PATH": "/workspace/.toolchain/cargo/bin:" + os.environ.get("PATH", ""),
    "CARGO_HOME": "/workspace/.toolchain/cargo",
}

# A literal is (rust_formula_text, js_value_literal, python_value).
# The same value must be written the two ways each side understands.
def num(v):
    return (repr(v), repr(v), v)


def text(v):
    escaped = v.replace("\\", "\\\\").replace('"', '\\"')
    js = json.dumps(v)
    return (f'"{escaped}"', js, v)


TRUE = ("TRUE", "true", True)
FALSE = ("FALSE", "false", False)


class Range:
    """A list of numbers that both sides must see as one argument.

    Rust gets a real cell range in a generated sheet, so the crate sees a
    reference rather than a list of literals; JS gets the array. That is the
    distinction the aggregation rules turn on, so the two sides must differ here.
    """

    def __init__(self, *values):
        self.values = values


def RANGE(*values):
    """A cell range on the Rust side and an array on the JS side."""
    return Range(*values)


class JsDate:
    """A calendar date that both sides must see as a date.

    The Rust side works in spreadsheet serial numbers, so it gets the serial.
    The reference's date functions call `new Date(value)`, and a serial number
    is read as milliseconds since the epoch, which is the wrong day by decades.
    So the JS side is handed a real `Date`, built by the driver. Only `ACCRINT`
    needs this, and it is why `ACCRINT` can be gated at all when the reference
    otherwise refuses serial dates.
    """

    def __init__(self, year, month, day):
        self.year = year
        self.month = month
        self.day = day
        self.serial = (date(year, month, day) - date(1899, 12, 30)).days

# function name -> list of argument tuples.
CASES = {
    "ABS": [(-3.5,), (0.0,), (7.0,)],
    "CONVERT": [
        (1, text("m"), text("cm")),
        (1, text("kg"), text("lbm")),
        (1, text("day"), text("hr")),
        (1, text("ft"), text("m")),
        (1, text("mi"), text("km")),
        (1, text("gal"), text("l")),
        (1, text("Wh"), text("J")),
        (1, text("km"), text("m")),
        (1, text("kibyte"), text("bit")),
        (1, text("Pa"), text("mmHg")),
        (2.5, text("m"), text("ft")),
        # `C` is the coulomb and `K` the kelvin, two categories, so both sides
        # report `#N/A`; the reference does not implement the Celsius scale.
        (100, text("C"), text("K")),
        (1, text("furlong"), text("m")),
    ],
    "CEILING": [(2.5, 1), (-2.5, 1), (-2.5, -1), (1.5, 0.1), (10, 3)],
    "FLOOR": [(2.5, 1), (-2.5, -1), (-2.5, 1), (10, 3)],
    "TRUNC": [(2.567, 2), (-2.567, 1), (2.567,), (2.567, 0)],
    "MROUND": [(10, 3), (-10, -3), (1.3, 0.2), (7, 2)],
    "EVEN": [(1.5,), (-1.5,), (3.0,), (0.0,), (2.0,)],
    "ODD": [(1.5,), (-1.5,), (3.0,), (0.0,), (2.0,)],
    "QUOTIENT": [(5, 2), (-5, 2), (5, -2), (10, 3)],
    "MOD": [(3, 2), (-3, 2), (3, -2), (5.5, 2)],
    "INT": [(2.5,), (-2.5,), (7.0,)],
    "SIGN": [(-5.0,), (0.0,), (3.0,)],
    "SQRT": [(16.0,), (2.0,), (-1.0,)],
    "POWER": [(2, 10), (9, 0.5), (2, -1)],
    "EXP": [(1.0,), (2.0,), (0.0,)],
    "LN": [(10.0,), (1.0,), (2.718281828459045,), (0.0,)],
    "LOG10": [(1000.0,), (100.0,), (1.0,), (0.0,)],
    "LOG": [(8, 2), (100,), (16, 4), (0,)],
    "FACT": [(5.0,), (0.0,), (1.0,), (10.0,)],
    "GCD": [(24, 36), (0, 5), (7, 13)],
    "LCM": [(4, 6), (0, 5), (21, 6)],
    "SUMSQ": [(1, 2, 3), (4, 5)],
    "SUM": [(1, 2, 3), (1.5, 2.5)],
    "PRODUCT": [(2, 3, 4), (1.5, 2)],
    "AVERAGE": [(1, 2, 3, 4), (10, 20)],
    "MEDIAN": [(1, 2, 3, 4), (1, 2, 3), (5, 1, 9)],
    "MIN": [(3, 1, 2), (5, 5)],
    "MAX": [(3, 1, 2), (5, 5)],
    "ROUND": [(2.567, 2), (2.5, 0), (-2.5, 0), (1234.5678, 2), (2.567, -1)],
    "ROUNDUP": [(2.1, 0), (-2.1, 0), (2.567, 2)],
    "ROUNDDOWN": [(2.9, 0), (-2.9, 0), (2.567, 2)],
    "PI": [()],
    "AND": [(TRUE, FALSE), (TRUE, TRUE)],
    "OR": [(TRUE, FALSE), (FALSE, FALSE)],
    "NOT": [(TRUE,), (FALSE,)],
    "EXACT": [(text("abc"), text("abc")), (text("abc"), text("ABC"))],
    "LEN": [(text("hello"),), (text(""),)],
    "UPPER": [(text("abc"),), (text("aBc"),)],
    "LOWER": [(text("ABC"),), (text("aBc"),)],
    "TRIM": [(text("  a   b  "),), (text("abc"),)],
    "LEFT": [(text("abcdef"), 3), (text("ab"), 5)],
    "RIGHT": [(text("abcdef"), 3), (text("ab"), 5)],
    "MID": [(text("abcdef"), 2, 3), (text("abcdef"), 1, 2), (text("ab"), 1, 5)],
    "REPT": [(text("ab"), 3), (text("x"), 0)],
    "SUBSTITUTE": [
        (text("a-b-c"), text("-"), text("+")),
        (text("abc"), text("z"), text("y")),
    ],
    "CONCAT": [(text("a"), text("b"), text("c")), (text("x"),)],
    "CONCATENATE": [(text("a"), text("b")), (text("x"), text("y"), text("z"))],
    "PROPER": [(text("hello world"),), (text("HELLO WORLD"),), (text("o brien"),)],
    "CHAR": [(65.0,), (97.0,), (955.0,)],
    "CODE": [(text("A"),), (text("abc"),)],
    "UNICHAR": [(65.0,), (955.0,)],
    "UNICODE": [(text("A"),), (text("\u03bb"),)],
    "CLEAN": [(text("ab"),)],
    "T": [(text("x"),), (5.0,)],
    "N": [(5.0,), (text("x"),), (TRUE,)],
    "FIND": [
        (text("b"), text("abcb")),
        (text("b"), text("abcb"), 3),
        (text("a"), text("banana")),
    ],
    "SEARCH": [
        (text("B"), text("abcb")),
        (text("b"), text("abcb"), 3),
        (text("a"), text("banana")),
    ],
    "TEXT": [
        (1234.567, text("#,##0.00")),
        (1234.567, text("0")),
        (1234.567, text("#,##0")),
        (0.5, text("0%")),
        (0.125, text("0.0%")),
        (-1234.5, text("#,##0.00")),
        (1234567, text("#,##0")),
    ],
    "VALUE": [
        (text("1234.5"),),
        (text("1,234.5"),),
        (text("12%"),),
    ],
    "COUNT": [(1, 2, 3), (text("a"), 1)],
    "COUNTA": [(1, 2, 3), (text("a"), 1)],
    # --- trigonometric and hyperbolic ------------------------------------
    "SIN": [(0.0,), (1.0,), (-1.0,)],
    "COS": [(0.0,), (1.0,), (2.0,)],
    "TAN": [(0.0,), (1.0,), (0.5,)],
    "ASIN": [(0.5,), (0.0,), (1.0,)],
    "ACOS": [(0.5,), (1.0,), (0.0,)],
    "ATAN": [(1.0,), (0.0,), (-1.0,)],
    "ATAN2": [(1, 1), (0, 1), (-1, -1)],
    "SINH": [(1.0,), (0.0,), (-1.0,)],
    "COSH": [(1.0,), (0.0,)],
    "TANH": [(1.0,), (0.0,)],
    "ASINH": [(1.0,), (0.0,), (-1.0,)],
    "ACOSH": [(1.0,), (2.0,)],
    "ATANH": [(0.5,), (0.0,)],
    "DEGREES": [(1.0,), (3.141592653589793,)],
    "RADIANS": [(1.0,), (180.0,)],
    "SQRTPI": [(2.0,), (0.0,), (4.0,)],
    "CSC": [(1.0,), (2.0,)],
    "SEC": [(1.0,), (2.0,), (0.0,)],
    "COT": [(1.0,), (2.0,)],
    "CSCH": [(1.0,), (2.0,)],
    "SECH": [(1.0,), (2.0,)],
    "COTH": [(1.0,), (2.0,)],
    "ACOT": [(1.0,), (2.0,), (-1.0,)],
    "ACOTH": [(2.0,), (-2.0,)],
    "GAMMA": [(5.0,), (0.5,), (1.0,)],
    "GAMMALN": [(5.0,), (0.5,), (1.0,)],
    "GAMMALNPRECISE": [(5.0,), (0.5,), (1.0,)],
    "LOGINV": [(0.5, 0, 1), (0.9, 0, 1), (0.1, 1, 0.5)],
    # --- Bessel ----------------------------------------------------------
    "BESSELJ": [(1, 0), (2, 1), (1, 2), (2, 3), (-2, 3), (10, 2)],
    "BESSELY": [(1, 0), (2, 1), (5, 2), (10, 3)],
    "BESSELI": [(1, 0), (1.5, 2), (5, 1), (-1, 1)],
    "BESSELK": [(1, 0), (2, 1), (5, 2)],
    "ERF": [(1.0,), (0.5,), (-1.0,)],
    "ERFC": [(1.0,), (0.5,), (2.0,)],
    # --- combinatorics ---------------------------------------------------
    "COMBIN": [(5, 2), (10, 3), (0, 0)],
    "COMBINA": [(5, 2), (3, 3), (0, 0)],
    "PERMUT": [(5, 2), (10, 3)],
    "PERMUTATIONA": [(5, 2), (3, 3)],
    "MULTINOMIAL": [(1, 2, 3), (3, 4)],
    "FACTDOUBLE": [(6.0,), (7.0,), (0.0,)],
    # --- descriptive statistics ------------------------------------------
    "AVEDEV": [(1, 2, 3, 4), (1, 2, 3)],
    "DEVSQ": [(1, 2, 3, 4), (2, 4, 6)],
    "GEOMEAN": [(1, 2, 3, 4), (2, 8)],
    "HARMEAN": [(1, 2, 3, 4), (2, 4)],
    "VARS": [(1, 2, 3, 4), (1, 2, 3, 4, 5)],
    "VARP": [(1, 2, 3, 4), (1, 2, 3, 4, 5)],
    "STDEVS": [(1, 2, 3, 4), (1, 2, 3, 4, 5)],
    "STDEVP": [(1, 2, 3, 4), (1, 2, 3, 4, 5)],
    "KURT": [(1, 2, 3, 4, 5), (2, 4, 6, 8)],
    "SKEW": [(1, 2, 3, 4, 5), (1, 2, 3, 4, 10)],
    "SKEWP": [(1, 2, 3, 4, 5), (1, 2, 3, 4, 10)],
    "LARGE": [(RANGE(1, 2, 3, 4), 2), (RANGE(5, 3, 9, 1), 1)],
    "SMALL": [(RANGE(1, 2, 3, 4), 2), (RANGE(5, 3, 9, 1), 1)],
    "MODESNGL": [(1, 2, 2, 3), (1, 1, 2, 2)],
    "TRIMMEAN": [(RANGE(1, 2, 3, 4, 5, 6, 7, 8, 9, 10), 0.2), (RANGE(1, 2, 3, 4), 0.5)],
    "PERCENTILEINC": [(RANGE(1, 2, 3, 4), 0.5), (RANGE(1, 2, 3, 4), 0), (RANGE(1, 2, 3, 4), 1)],
    "PERCENTILEEXC": [(RANGE(1, 2, 3, 4), 0.5), (RANGE(1, 2, 3, 4), 0.25)],
    "QUARTILEINC": [(RANGE(1, 2, 3, 4), 2), (RANGE(1, 2, 3, 4), 1), (RANGE(1, 2, 3, 4), 3)],
    "QUARTILEEXC": [(RANGE(1, 2, 3, 4), 2), (RANGE(1, 2, 3, 4), 1), (RANGE(1, 2, 3, 4), 3)],
    "RANKEQ": [(2, RANGE(1, 2, 3)), (4, RANGE(1, 2, 3, 4), 1), (2, RANGE(1, 2, 2, 3))],
    "RANKAVG": [(2, RANGE(1, 2, 3)), (2, RANGE(1, 2, 2, 3))],
    "SERIESSUM": [(1, 1, 1, RANGE(1, 2, 3)), (2, 1, 2, RANGE(1, 2, 3))],
    "SUMX2MY2": [(RANGE(1, 2, 3), RANGE(4, 5, 6))],
    "SUMX2PY2": [(RANGE(1, 2, 3), RANGE(4, 5, 6))],
    "SUMXMY2": [(RANGE(1, 2, 3), RANGE(4, 5, 6))],
    "AVERAGEA": [(1, 2, 3), (1, 2)],
    "MAXA": [(1, 2, 3), (1, 2)],
    "MINA": [(1, 2, 3), (1, 2)],
    # --- base conversion -------------------------------------------------
    "BIN2DEC": [(text("1010"),), (text("1111111111"),)],
    "HEX2DEC": [(text("FF"),), (text("ffffffff"),)],
    "OCT2DEC": [(text("17"),), (text("7777777777"),)],
    "DEC2BIN": [(10,), (5, 8), (-5,)],
    "DEC2HEX": [(255,), (255, 4), (-1,)],
    "DEC2OCT": [(8,), (-8,)],
    "BIN2HEX": [(text("1010"),), (text("1010"), 4), (text("1111111111"),)],
    "BIN2OCT": [(text("1010"),), (text("1111111111"),), (text("1010"), 6)],
    "HEX2BIN": [(text("FF"),), (text("F"), 8), (text("FFFFFFFF"),)],
    "HEX2OCT": [(text("FF"),), (text("1F"), 4), (text("FFFFFFFFFF"),)],
    "OCT2BIN": [(text("17"),), (text("17"), 8), (text("7777777777"),)],
    "OCT2HEX": [(text("17"),), (text("7777777777"),), (text("17"), 4)],
    "BASE": [(255, 16), (255, 2), (7, 2, 4)],
    "DECIMAL": [(text("FF"), 16), (text("zz"), 36)],
    # --- rounding variants -----------------------------------------------
    "CEILINGMATH": [(5.5,), (-5.5,), (5.5, 2), (-5.5, 2), (-5.5, 2, 1), (5.5, -2), (5.5, 0)],
    "FLOORMATH": [(5.5,), (-5.5,), (5.5, 2), (-5.5, 2), (-5.5, 2, 1), (5.5, -2), (5.5, 0)],
    "CEILINGPRECISE": [(5.5,), (-5.5,), (5.5, 2), (-5.5, 2), (5.5, -2)],
    "FLOORPRECISE": [(5.5,), (-5.5,), (5.5, 2), (-5.5, 2), (5.5, -2)],
    "ISOCEILING": [(5.5, 2), (-5.5, 2), (2.5, 1)],
    # --- bitwise ---------------------------------------------------------
    "BITAND": [(5, 3), (13, 25)],
    "BITOR": [(5, 3), (13, 25)],
    "BITXOR": [(5, 3), (13, 25)],
    "BITLSHIFT": [(5, 2), (5, 0)],
    "BITRSHIFT": [(5, 2), (5, 0)],
    # --- roman numerals --------------------------------------------------
    "ROMAN": [(2024,), (4,), (1994,), (45,)],
    # --- serial dates ----------------------------------------------------
    # The reference disagrees with Excel inside the 1900 phantom-leap window
    # (serials 59 and 60) and returns a `Date` from `WORKDAY` regardless of
    # serial mode, so those cases live in the crate's own unit tests instead.
    "DATE": [(2024, 1, 15), (1900, 1, 1), (1900, 2, 28), (2000, 2, 29)],
    "DAY": [(45306,), (1,), (61,)],
    "MONTH": [(45306,), (61,)],
    "YEAR": [(45306,), (60,), (1,)],
    "TIME": [(12, 30, 15), (0, 0, 0), (23, 59, 59)],
    "HOUR": [(0.5,), (0.25,), (0.75,)],
    "MINUTE": [(0.5,), (0.25,), (0.75,)],
    "SECOND": [(0.5,), (0.25,), (0.75,)],
    "EDATE": [(45306, 1), (45306, -2), (45292, 12)],
    "EOMONTH": [(45306, 0), (45306, 1), (45306, -1)],
    "DAYS": [(45306, 45292), (45292, 45306)],
    "DAYS360": [(45306, 45473), (45292, 45473), (45292, 45473, TRUE)],
    "YEARFRAC": [(45292, 45839), (45292, 45839, 1), (45292, 45839, 2), (45292, 45839, 3)],
    "DATEDIF": [(43875, 45503, text("Y")), (43875, 45503, text("M")), (43875, 45503, text("D")), (43875, 45503, text("MD")), (43875, 45503, text("YM")), (43875, 45503, text("YD"))],
    "WEEKDAY": [(45306,), (45306, 2), (45306, 3), (45306, 11), (45306, 16)],
    "WEEKNUM": [(45306,), (45306, 2), (45306, 21)],
    "ISOWEEKNUM": [(45306,), (44218,)],
    "DATEVALUE": [(text("2024-01-15"),), (text("1900-01-01"),)],
    "NETWORKDAYS": [(45292, 45322), (45292, 45322, RANGE(45301, 45302))],
    "NETWORKDAYSINTL": [
        (45292, 45322),
        (45292, 45322, 1),
        (45292, 45322, 2),
        (45292, 45322, text("0000011")),
        (45292, 45322, 11),
        (45292, 45322, 7, RANGE(45301, 45302)),
    ],
    # `WORKDAY` returns a JS `Date` regardless of serial mode, so its cases
    # live in the crate's own unit tests rather than here.
    # --- database --------------------------------------------------------
    "RATE": [(60, -100, 1000), (12, -100, 1000, 0), (24, -1000, 10000)],
    # `ACCRINT` takes dates, so it is gated with `JsDate` values: the reference
    # needs a real `Date` where this crate takes a serial. See the class.
    "ACCRINT": [
        (JsDate(2020, 1, 15), JsDate(2020, 7, 15), JsDate(2021, 1, 15), 0.05, 1000, 2),
        (JsDate(2020, 1, 15), JsDate(2020, 7, 15), JsDate(2021, 1, 15), 0.05, 1000, 1),
        (JsDate(2020, 1, 15), JsDate(2020, 7, 15), JsDate(2021, 1, 15), 0.05, 1000, 4),
        (JsDate(2020, 1, 15), JsDate(2020, 7, 15), JsDate(2021, 1, 15), 0.05, 1000, 2, 1),
        (JsDate(2020, 1, 15), JsDate(2020, 7, 15), JsDate(2021, 1, 15), 0.05, 1000, 2, 2),
        (JsDate(2020, 1, 15), JsDate(2020, 7, 15), JsDate(2021, 1, 15), 0.05, 1000, 2, 3),
        (JsDate(2020, 1, 15), JsDate(2020, 7, 15), JsDate(2021, 1, 15), 0.05, 1000, 2, 4),
        # A partial first year: the term is under twelve months.
        (JsDate(2020, 3, 1), JsDate(2020, 9, 1), JsDate(2020, 9, 1), 0.06, 5000, 2),
        # A rate that produces a round number under 30/360.
        (JsDate(2019, 2, 15), JsDate(2019, 8, 15), JsDate(2020, 2, 15), 0.04, 1000, 2),
    ],
    "COUPDAYS": [
        (45292, 45839, 2),
        (45292, 45839, 2, 0),
        (45292, 45839, 2, 1),
        (45292, 45839, 2, 3),
        (45292, 45839, 4),
        (45292, 45839, 1),
    ],
    "AGGREGATE": [
        (1, 0, RANGE(1, 2, 3)),
        (9, 0, RANGE(1, 2, 3)),
        (4, 0, RANGE(1, 2, 3)),
        (5, 0, RANGE(1, 2, 3)),
        (12, 0, RANGE(1, 2, 3)),
        (13, 0, RANGE(1, 2, 2, 3)),
        (14, 0, RANGE(1, 2, 3), 1),
        (15, 0, RANGE(1, 2, 3), 1),
        (16, 0, RANGE(1, 2, 3), 0.5),
        (17, 0, RANGE(1, 2, 3), 2),
        (6, 0, RANGE(1, 2, 3)),
        (7, 0, RANGE(1, 2, 3)),
        (8, 0, RANGE(1, 2, 3)),
        (10, 0, RANGE(1, 2, 3)),
        (11, 0, RANGE(1, 2, 3)),
    ],
    "SUBTOTAL": [
        (9, RANGE(1, 2, 3, 4)),
        (1, RANGE(1, 2, 3, 4)),
        (2, RANGE(1, 2, 3, 4)),
        (4, RANGE(1, 2, 3, 4)),
        (5, RANGE(1, 2, 3, 4)),
        (6, RANGE(1, 2, 3, 4)),
        (109, RANGE(1, 2, 3, 4)),
        (103, RANGE(1, 2, 3, 4)),
    ],
    # --- complex numbers -------------------------------------------------
    "COMPLEX": [(3, 4), (3, 4, text("j")), (0, 1), (1, 0), (-3, -4)],
    # The reference returns a bare real number unparsed from `IMREAL`, as the
    # string it was given, so that case lives in the crate's own tests.
    "IMREAL": [(text("3+4i"),)],
    "IMAGINARY": [(text("3+4i"),), (text("5"),)],
    "IMABS": [(text("3+4i"),), (text("-5i"),)],
    "IMARGUMENT": [(text("3+4i"),), (text("1+i"),)],
    "IMCONJUGATE": [(text("3+4i"),), (text("3+4j"),)],
    "IMSQRT": [(text("3+4i"),), (text("1+i"),)],
    "IMEXP": [(text("1+i"),)],
    "IMLN": [(text("1+i"),)],
    "IMLOG10": [(text("1+i"),)],
    "IMLOG2": [(text("1+i"),)],
    "IMSIN": [(text("1+i"),)],
    "IMCOS": [(text("1+i"),)],
    "IMTAN": [(text("1+i"),)],
    "IMSINH": [(text("1+i"),)],
    "IMCOSH": [(text("1+i"),)],
    "IMCOT": [(text("1+i"),)],
    "IMCSC": [(text("1+i"),)],
    "IMCSCH": [(text("1+i"),)],
    "IMSEC": [(text("1+i"),)],
    "IMSECH": [(text("1+i"),)],
    "IMSUM": [(text("3+4i"), text("1-2i"))],
    "IMSUB": [(text("3+4i"), text("1-2i"))],
    "IMPRODUCT": [(text("3+4i"), text("1-2i"))],
    "IMDIV": [(text("3+4i"), text("1-2i"))],
    "IMPOWER": [(text("3+4i"), 2)],
    # --- financial -------------------------------------------------------
    "FV": [(0.05, 10, -100), (0.05, 10, -100, -500), (0.05, 10, -100, -500, TRUE)],
    "PV": [(0.05, 10, -100), (0.05, 10, -100, 500)],
    "PMT": [(0.05, 10, 1000), (0.05, 10, 1000, 500)],
    "NPER": [(0.05, -100, 1000), (0.05, -100, 1000, 500)],
    "IPMT": [(0.05, 1, 10, 1000), (0.05, 2, 10, 1000), (0.05, 1, 10, 1000, 0, TRUE)],
    "PPMT": [(0.05, 1, 10, 1000), (0.05, 2, 10, 1000)],
    "CUMIPMT": [(0.05, 10, 1000, 1, 3, 0)],
    "CUMPRINC": [(0.05, 10, 1000, 1, 3, 0)],
    "ISPMT": [(0.05, 1, 10, 1000), (0.05, 5, 10, 1000)],
    "NPV": [(0.1, -10000, 3000, 4200, 6800), (0.05, 100, 100)],
    "IRR": [(RANGE(-1000, 100, 200, 300),), (RANGE(-1000, 100, 200, 300), 0.2)],
    "MIRR": [(RANGE(-1000, 100, 200, 300), 0.1, 0.12)],
    "SLN": [(1000, 100, 5)],
    "SYD": [(1000, 100, 5, 1), (1000, 100, 5, 3)],
    "DDB": [(1000, 100, 5, 1), (1000, 100, 5, 2), (1000, 100, 5, 1, 1.5)],
    "EFFECT": [(0.05, 4)],
    "NOMINAL": [(0.05, 4)],
    "RRI": [(10, 100, 200)],
    "PDURATION": [(0.05, 1000, 2000)],
    "DOLLARDE": [(1.02, 16), (1.1, 32)],
    "DOLLARFR": [(1.125, 16), (1.125, 32)],
    "FVSCHEDULE": [(100, RANGE(0.05, 0.05, 0.05))],
    "TBILLPRICE": [(45306, 45671, 0.05)],
    "TBILLYIELD": [(45306, 45671, 94.93055555555556)],
    "TBILLEQ": [(45306, 45671, 0.05)],
    # --- lookup and reference --------------------------------------------
    # The lookup family is exercised through formulas in the crate's own tests,
    # because the gate's `RANGE` helper flattens a range without its shape.
    "CHOOSE": [(2, 10, 20, 30), (1, text("a"), text("b"))],
    "ROWS": [(RANGE(1, 2, 3),)],
    "COLUMNS": [(RANGE(1, 2, 3),)],
    "ARABIC": [(text("MMXXIV"),), (text("MCMXC"),), (text("iv"),)],
    # --- information -----------------------------------------------------
    "ISNUMBER": [(5.0,), (text("5"),)],
    "ISTEXT": [(text("x"),), (5.0,)],
    "ISNONTEXT": [(5.0,), (text("x"),)],
    "ISLOGICAL": [(TRUE,), (5.0,)],
    "ISEVEN": [(2.0,), (3.0,), (-2.0,)],
    "ISODD": [(3.0,), (2.0,), (-3.0,)],
    "TYPE": [(5.0,), (text("x"),), (TRUE,)],
    "DELTA": [(1, 1), (1, 2), (5,)],
    "GESTEP": [(5, 4), (4, 4), (5,)],
    "XOR": [(TRUE, FALSE), (TRUE, TRUE)],
    # --- text ------------------------------------------------------------
    "REPLACE": [
        (text("abcdef"), 2, 3, text("XY")),
        (text("abcdef"), 1, 0, text("X")),
    ],
    "TEXTJOIN": [(text(","), TRUE, text("a"), text("b"))],
    "DOLLAR": [(1234.5, 2), (1234.5, 0), (-1234.5, 2)],
    "FIXED": [(1234.5, 1), (1234.567, 2, TRUE)],
    "NUMBERVALUE": [(text("1.5"),), (text("1,5"), text(","))],
    # --- statistical distributions ---------------------------------------
    # These follow the flat legacy names, which is the only spelling the
    # reference exports. The crate also accepts the dotted modern names, but
    # the reference cannot be asked for those.
    "NORMDIST": [
        (1, 0, 1, TRUE),
        (1, 0, 1, FALSE),
        (0, 5, 2, TRUE),
        (2.5, 1, 1.5, FALSE),
        (1, 0, 0, TRUE),
    ],
    "NORMSDIST": [(1.5, TRUE), (1.5, FALSE), (0, TRUE), (-2, TRUE)],
    "NORMINV": [(0.3, 5, 2), (0.975, 0, 1), (0.5, 0, 1)],
    "NORMSINV": [(0.3,), (0.975,), (0.5,)],
    "LOGNORMDIST": [
        (2, 1, 0.5, TRUE),
        (2, 1, 0.5, FALSE),
        (1, 0, 1, TRUE),
    ],
    "LOGNORMINV": [(0.5, 1, 0.5), (0.3, 0, 1)],
    "TDIST": [(2, 5, 1), (2, 5, 2), (1.5, 10, 1), (0.5, 3, 2)],
    "TINV": [(0.3, 5), (0.05, 10), (0.5, 4)],
    "CHIDIST": [
        (3, 5, TRUE),
        (3, 5, FALSE),
        (7.5, 3, TRUE),
        (2, 1, FALSE),
    ],
    "CHIINV": [(0.3, 5), (0.05, 10), (0.95, 3)],
    "FDIST": [
        (2, 3, 4, TRUE),
        (2, 3, 4, FALSE),
        (1.5, 5, 7, TRUE),
        (0.5, 2, 3, FALSE),
    ],
    "FINV": [(0.3, 3, 4), (0.05, 5, 10), (0.9, 2, 3)],
    "BINOMDIST": [
        (3, 10, 0.5, TRUE),
        (3, 10, 0.5, FALSE),
        (2, 8, 0.25, TRUE),
        (5, 20, 0.3, FALSE),
    ],
    "CRITBINOM": [(10, 0.5, 0.75), (20, 0.3, 0.9), (6, 0.5, 0.5)],
    "POISSONDIST": [(2, 3, TRUE), (2, 3, FALSE), (5, 2.5, TRUE)],
    "EXPONDIST": [(1, 2, TRUE), (1, 2, FALSE), (0.5, 0.5, TRUE)],
    "WEIBULLDIST": [
        (2, 3, 4, TRUE),
        (2, 3, 4, FALSE),
        (1, 2, 5, TRUE),
    ],
    "GAMMADIST": [
        (2, 3, 4, TRUE),
        (2, 3, 4, FALSE),
        (1, 2, 2, TRUE),
    ],
    "GAMMAINV": [(0.5, 3, 4), (0.3, 2, 1), (0.9, 5, 2)],
    "BETADIST": [
        (0.4, 2, 3, TRUE),
        (0.4, 2, 3, FALSE),
        (0.25, 1, 1, TRUE),
        (0.5, 2, 2, TRUE),
    ],
    "BETAINV": [(0.5, 2, 3), (0.3, 1, 1), (0.8, 3, 2)],
    "HYPGEOMDIST": [
        (2, 5, 3, 20, FALSE),
        (2, 5, 3, 20, TRUE),
        (1, 3, 5, 15, TRUE),
    ],
    "NEGBINOMDIST": [
        (3, 4, 0.5, FALSE),
        (3, 4, 0.5, TRUE),
        (5, 2, 0.4, TRUE),
    ],
    "STANDARDIZE": [(5, 3, 2), (1, 0, 1), (10, 20, 4)],
    "GAUSS": [(1.5,), (0,), (-1.5,)],
    "PHI": [(1.5,), (0,), (-2,)],
    "FISHER": [(0.5,), (0,), (-0.25,)],
    "FISHERINV": [(0.5,), (0,), (-1,)],
    "CORREL": [(RANGE(1, 2, 3, 4), RANGE(2, 4, 6, 8)), (RANGE(3, 1, 2), RANGE(5, 2, 9))],
    "PEARSON": [(RANGE(1, 2, 3, 4), RANGE(2, 4, 6, 8)), (RANGE(3, 1, 2), RANGE(5, 2, 9))],
    "RSQ": [(RANGE(1, 2, 3, 4), RANGE(2, 4, 6, 8)), (RANGE(3, 1, 2), RANGE(5, 2, 9))],
    "SLOPE": [(RANGE(3, 5, 7, 9), RANGE(1, 2, 3, 4)), (RANGE(1, 2, 3), RANGE(2, 4, 6))],
    "INTERCEPT": [(RANGE(3, 5, 7, 9), RANGE(1, 2, 3, 4)), (RANGE(1, 2, 3), RANGE(2, 4, 6))],
    "STEYX": [(RANGE(3, 5, 7, 9), RANGE(1, 2, 3, 4)), (RANGE(1, 2, 4), RANGE(2, 4, 6))],
    "FORECAST": [
        (5, RANGE(3, 5, 7, 9), RANGE(1, 2, 3, 4)),
        (0, RANGE(1, 2, 3), RANGE(2, 4, 6)),
    ],
    "COVAR": [(RANGE(1, 2, 3, 4), RANGE(2, 4, 6, 8)), (RANGE(3, 1, 2), RANGE(5, 2, 9))],
    "COVARIANCES": [
        (RANGE(1, 2, 3, 4), RANGE(2, 4, 6, 8)),
        (RANGE(3, 1, 2), RANGE(5, 2, 9)),
    ],
    "PERCENTRANKINC": [
        (RANGE(1, 2, 3, 4), 2),
        (RANGE(1, 2, 3, 4), 2.5),
        (RANGE(1, 2, 3, 4), 3),
    ],
    "PERCENTRANKEXC": [
        (RANGE(1, 2, 3, 4), 2),
        (RANGE(1, 2, 3, 4), 2.5),
        (RANGE(1, 2, 3, 4), 3),
    ],
    "MODESNGL": [(RANGE(1, 2, 2, 3),), (RANGE(1, 1, 2, 2),), (RANGE(5, 5, 5),)],
    "PERCENTILEINC": [(RANGE(1, 2, 3, 4), 0.5), (RANGE(1, 2, 3, 4), 0), (RANGE(1, 2, 3, 4), 1)],
    # Only the interior quartiles are checked: the reference's `QUARTILE.INC`
    # falls through to `#NUM!` for 0 and 4, which Excel accepts as the minimum
    # and maximum. That is a reference oversight, not the documented behaviour,
    # so the crate returns the minimum and maximum and the gate cannot ask for
    # the extremes.
    "QUARTILEINC": [(RANGE(1, 2, 3, 4), 1), (RANGE(1, 2, 3, 4), 2), (RANGE(1, 2, 3, 4), 3)],
    "RANKEQ": [(2, RANGE(1, 2, 3, 4)), (4, RANGE(1, 2, 3, 4)), (1, RANGE(1, 2, 3, 4), 1)],
    "AVERAGEA": [(1, 2, 3), (RANGE(1, 2, 3),), (TRUE, 2, 3)],
    "VARA": [(1, 2, 3), (RANGE(1, 2, 3),), (1, 2, 3, 4)],
    "VARPA": [(1, 2, 3), (RANGE(1, 2, 3),), (1, 2, 3, 4)],
    "STDEVA": [(1, 2, 3), (RANGE(1, 2, 3),), (1, 2, 3, 4)],
    "STDEVPA": [(1, 2, 3), (RANGE(1, 2, 3),), (1, 2, 3, 4)],
    "ZTEST": [
        (RANGE(1, 2, 3, 4), 2),
        (RANGE(1, 2, 3, 4), 2, 1),
        (RANGE(2, 4, 6, 8), 3),
    ],
    "TTEST": [
        (RANGE(1, 2, 3, 4), RANGE(2, 3, 4, 5)),
        (RANGE(1, 2, 3, 4), RANGE(5, 6, 7, 8)),
        (RANGE(1, 2, 3, 4), RANGE(2, 4, 6, 8)),
    ],
    "FTEST": [
        (RANGE(1, 2, 3, 4), RANGE(2, 4, 6, 8)),
        (RANGE(1, 2, 3, 4), RANGE(1, 2, 3, 5)),
    ],
    "CHITEST": [
        (RANGE(10, 20), RANGE(15, 15)),
        (RANGE(5, 10, 15), RANGE(10, 10, 10)),
    ],
    "PROB": [
        (RANGE(1, 2, 3), RANGE(0.2, 0.3, 0.5), 2),
        (RANGE(1, 2, 3), RANGE(0.2, 0.3, 0.5), 1, 2),
        (RANGE(1, 2, 3), RANGE(0.2, 0.3, 0.5), 0, 2),
    ],
}

EXAMPLE = """//! Generated by research/tools/verify_spreadsheet.py. Do not edit.
use spreadsheet_formula::{CellRef, Evaluator, Sheet, Value};

/// A sheet backed by a list of cells, used for the range cases.
struct Grid(&'static [(u32, u32, f64)]);

impl Sheet for Grid {
    fn value(&self, cell: CellRef) -> Value {
        for (column, row, value) in self.0 {
            if *column == cell.column && *row == cell.row {
                return Value::Number(*value);
            }
        }
        Value::Empty
    }
}

fn main() {
    let cases: &[(&str, &[(u32, u32, f64)])] = __CASES__;
    for (formula, cells) in cases {
        let grid = Grid(cells);
        let mut evaluator = Evaluator::new(&grid);
        match evaluator.eval(formula) {
            Ok(Value::Number(n)) => println!("num\\t{n}"),
            Ok(Value::Text(t)) => println!("text\\t{t}"),
            Ok(Value::Bool(b)) => println!("bool\\t{b}"),
            Ok(Value::Empty) => println!("empty"),
            Err(_) => println!("error"),
        }
    }
}
"""


def rust_formulas():
    """Each case as (formula, sheet cells), in order.

    A range becomes a column of the generated sheet, so a formula with two
    ranges uses two columns. The cells are laid out in the order the ranges
    appear in the case.
    """
    out = []
    for name, arg_sets in CASES.items():
        for args in arg_sets:
            cells = []
            rendered_args = []
            for arg in args:
                if isinstance(arg, Range):
                    column = len({c for c, _, _ in cells})
                    for row, value in enumerate(arg.values):
                        cells.append((column, row, float(value)))
                    rendered_args.append(
                        f"{COLUMN_NAMES[column]}1:{COLUMN_NAMES[column]}{len(arg.values)}"
                    )
                else:
                    rendered_args.append(render_rust(arg))
            out.append((f"={name}({', '.join(rendered_args)})", cells))
    return out


COLUMN_NAMES = [chr(ord("A") + i) for i in range(26)]


def render_rust(arg):
    if isinstance(arg, JsDate):
        return repr(arg.serial)
    if isinstance(arg, tuple) and len(arg) == 3 and isinstance(arg[1], str):
        return arg[0]
    return repr(arg)


def js_cases():
    """The JS call expressions and the name they belong to, in case order."""
    out = []
    for name, arg_sets in CASES.items():
        for args in arg_sets:
            rendered = ", ".join(render_js(a) for a in args)
            # `ISO.CEILING` is a nested namespace, so the flat name the CASES
            # table uses has to be spelled the way the reference exposes it.
            dotted = "ISO.CEILING" if name == "ISOCEILING" else name
            out.append((name, f"f.{dotted}({rendered})"))
    return out


def render_js(arg):
    if isinstance(arg, JsDate):
        # Local time, which is the month the crate's serial also names.
        return f"new Date({arg.year}, {arg.month - 1}, {arg.day})"
    if isinstance(arg, tuple) and len(arg) == 3 and isinstance(arg[1], str):
        return arg[1]
    if isinstance(arg, Range):
        return json.dumps(list(arg.values))
    if isinstance(arg, str):
        return json.dumps(arg)
    return repr(arg)


JS_DRIVER = """const f = require('@formulajs/formulajs');
// Dates are compared as serial numbers, which is what this crate returns; the
// reference defaults to JS `Date` objects, which would compare as 'error'.
f.utils.date.useSerial();
const cases = __CASES__;
for (const expr of cases) {
  let out;
  try {
    const value = eval(expr);
    if (value === undefined || value === null) out = 'empty';
    else if (typeof value === 'number') out = Number.isNaN(value) ? 'nan' : 'num\\t' + value;
    else if (typeof value === 'string') out = 'text\\t' + value;
    else if (typeof value === 'boolean') out = 'bool\\t' + value;
    else if (typeof value === 'object') out = 'error';
    else out = 'other';
  } catch (e) {
    out = 'error';
  }
  console.log(out);
}
"""


def run_rust(cases):
    example = CRATE / "examples" / "_oracle_formulas.rs"
    example.parent.mkdir(exist_ok=True)
    lines = []
    for formula, cells in cases:
        rendered_cells = ", ".join(f"({c}, {r}, {v!r})" for c, r, v in cells)
        lines.append(f'    ({json.dumps(formula, ensure_ascii=False)}, &[{rendered_cells}]),')
    body = "\n".join(lines)
    example.write_text(EXAMPLE.replace("__CASES__", f"&[\n{body}\n]"))
    try:
        r = subprocess.run(
            ["cargo", "run", "--quiet", "--example", "_oracle_formulas"],
            cwd=CRATE, capture_output=True, text=True, env=CARGO_ENV,
        )
        if r.returncode != 0:
            raise SystemExit(r.stderr[-4000:])
        return r.stdout.splitlines()
    finally:
        example.unlink()


def run_js(cases):
    driver = CRATE / "_oracle_cases.js"
    exprs = [expr for _, expr in cases]
    driver.write_text(JS_DRIVER.replace("__CASES__", json.dumps(exprs, ensure_ascii=False)))
    try:
        r = subprocess.run(
            ["node", str(driver)], cwd=ROOT, capture_output=True, text=True,
        )
        if r.returncode != 0:
            raise SystemExit(r.stderr[-4000:])
        return r.stdout.splitlines()
    finally:
        driver.unlink()


def close_enough(a, b):
    if a == b:
        return True
    try:
        fa, fb = float(a), float(b)
    except ValueError:
        return False
    return abs(fa - fb) <= 1e-9 * max(1.0, abs(fa), abs(fb))


def main() -> int:
    verbose = "--verbose" in sys.argv
    formulas = rust_formulas()
    cases = js_cases()
    ours = run_rust(formulas)
    theirs = run_js(cases)
    if len(ours) != len(theirs):
        raise SystemExit(f"case count differs: {len(ours)} rust, {len(theirs)} js")

    mismatches = 0
    for (name, expr), mine, reference in zip(cases, ours, theirs):
        mine_kind, _, mine_value = mine.partition("\t")
        ref_kind, _, ref_value = reference.partition("\t")
        if mine_kind == "error" or ref_kind == "error":
            # Both refusing is agreement; one refusing is a defect.
            if mine_kind == ref_kind:
                continue
            mismatches += 1
            print(f"MISMATCH {name} {expr}\n  rust={mine}\n  js  ={reference}")
            continue
        if mine_kind != ref_kind:
            mismatches += 1
            print(f"MISMATCH {name} {expr}\n  rust={mine}\n  js  ={reference}")
            continue
        if mine_kind == "num":
            if not close_enough(mine_value, ref_value):
                mismatches += 1
                print(f"MISMATCH {name} {expr}\n  rust={mine}\n  js  ={reference}")
        elif mine_value != ref_value:
            mismatches += 1
            print(f"MISMATCH {name} {expr}\n  rust={mine}\n  js  ={reference}")
        elif verbose:
            print(f"ok  {name} {expr} -> {mine}")

    total = len(cases)
    print(f"\n{total - mismatches}/{total} cases agree with formulajs")
    return 1 if mismatches else 0


if __name__ == "__main__":
    raise SystemExit(main())
