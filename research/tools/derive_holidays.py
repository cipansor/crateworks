#!/usr/bin/env python3
"""Derive a holidays-rs country table from the reference and verify it.

Given a country code, this reads every holiday the reference `holidays` package
reports for 2000-2050, fits each one to the crate's `Rule` vocabulary, emits the
Rust table, and then re-resolves that table in Python and compares the date set
to the reference. It refuses to emit a table that does not match, so the output
can be pasted into `countries.rs` with confidence.

Usage:
    python3 derive_holidays.py SV          # verify only
    python3 derive_holidays.py SV --rust   # print the Rust table
"""

import collections
import datetime
import sys

import holidays as pyholidays

FIRST, LAST = 2000, 2050
STYLE = {
    "none": "SubstituteStyle::None",
    "nearest": "SubstituteStyle::NearestWeekday",
    "next_monday": "SubstituteStyle::NextMonday",
}


def easter(y):
    a, b, c = y % 19, y // 100, y % 100
    d, e = b // 4, b % 4
    f = (b + 8) // 25
    g = (b - f + 1) // 3
    h = (19 * a + b - d - g + 15) % 30
    i, k = c // 4, c % 4
    lo = (32 + 2 * e + 2 * i - h - k) % 7
    m = (a + 11 * h + 22 * lo) // 451
    mo = (h + lo - 7 * m + 114) // 31
    da = ((h + lo - 7 * m + 114) % 31) + 1
    return datetime.date(y, mo, da)


def nth_weekday(y, month, weekday, nth):
    """`weekday` is in the crate's numbering (Sunday is 0)."""
    first = datetime.date(y, month, 1)
    py_weekday = (weekday - 1) % 7
    day = 1 + (py_weekday - first.weekday()) % 7 + (nth - 1) * 7
    if nth == 5:
        while True:
            try:
                datetime.date(y, month, day + 7)
            except ValueError:
                break
            day += 7
    try:
        return datetime.date(y, month, day)
    except ValueError:
        return None


def base_shape(dates):
    """The base rule that produces `dates`, or None.

    The crate numbers weekdays with Sunday as 0, while `datetime.weekday()`
    numbers them with Monday as 0, so the weekday is converted here. A rule that
    lands on the last occurrence of its weekday in the month is emitted as
    `nth: 5`, which is how the crate spells "last".
    """
    if len({(d.month, d.day) for d in dates}) == 1:
        m, day = next(iter({(d.month, d.day) for d in dates}))
        return ("fixed", m, day)
    if (
        len({d.month for d in dates}) == 1
        and len({d.weekday() for d in dates}) == 1
        and len({(d.day - 1) // 7 for d in dates}) == 1
    ):
        m = dates[0].month
        wd = (dates[0].weekday() + 1) % 7
        nth = (dates[0].day - 1) // 7 + 1
        return ("nth", m, wd, nth)
    if len({(d - easter(d.year)).days for d in dates}) == 1:
        return ("easter", next(iter({(d - easter(d.year)).days for d in dates})))
    return None


def resolve(shape, y):
    if shape is None:
        return None
    if shape[0] == "fixed":
        return datetime.date(y, shape[1], shape[2])
    if shape[0] == "nth":
        return nth_weekday(y, shape[1], shape[2], shape[3])
    return easter(y) + datetime.timedelta(days=shape[1])


def contiguous(years):
    """Split a set of years into inclusive (from, to) runs."""
    years = sorted(years)
    runs = []
    start = prev = years[0]
    for y in years[1:]:
        if y == prev + 1:
            prev = y
            continue
        runs.append((start, prev))
        start = prev = y
    runs.append((start, prev))
    return runs


def rust_rule(shape, frm, to):
    """Render a rule with its year gate."""
    if shape[0] == "fixed":
        core = f"Rule::Fixed {{ month: {shape[1]}, day: {shape[2]} }}"
    elif shape[0] == "nth":
        core = (
            f"Rule::NthWeekday {{ month: {shape[1]}, weekday: {shape[2]}, "
            f"nth: {shape[3]} }}"
        )
    else:
        core = f"Rule::EasterOffset({shape[1]})"
    if frm == FIRST and to == LAST:
        return core
    if frm == FIRST:
        return f"Rule::Until {{ year: {to}, base: &{core} }}"
    if to == LAST:
        return f"Rule::Since {{ year: {frm}, base: &{core} }}"
    return f"Rule::Between {{ from: {frm}, to: {to}, base: &{core} }}"


def derive(code):
    ref = pyholidays.country_holidays(code, years=range(FIRST, LAST + 1))
    byname = collections.defaultdict(dict)
    for d, n in ref.items():
        byname[n][d.year] = d
    entries = []
    for name, byyear in byname.items():
        dates = list(byyear.values())
        shape = base_shape(dates)
        if shape is None:
            return None, f"unexpressible: {name}"
        years = set(byyear)
        for frm, to in contiguous(years):
            # Guard against a holiday whose date shape changes inside a run.
            if any(resolve(shape, y) != byyear[y] for y in range(frm, to + 1)):
                return None, f"shape shifts inside a run: {name} {frm}-{to}"
            entries.append((frm, name, shape, frm, to))
    return entries, None


def resolve_table(entries, style):
    out = set()
    for y in range(FIRST, LAST + 1):
        falls = []
        for _, name, shape, frm, to in entries:
            if frm <= y <= to:
                d = resolve(shape, y)
                if d is not None:
                    falls.append((d, name))
        falls.sort()
        if style == "none":
            out |= {d for d, _ in falls}
            continue
        taken = set()
        for d, _ in falls:
            moved = d
            if style == "nearest":
                if d.weekday() == 5:
                    moved = d - datetime.timedelta(days=1)
                elif d.weekday() == 6:
                    moved = d + datetime.timedelta(days=1)
            elif style == "next_monday":
                if d.weekday() == 5:
                    moved = d + datetime.timedelta(days=2)
                elif d.weekday() == 6:
                    moved = d + datetime.timedelta(days=1)
            while moved in taken:
                moved += datetime.timedelta(days=1)
            taken.add(moved)
            if moved.year == FIRST or moved.year == LAST + 1 or FIRST < moved.year < LAST:
                out.add(moved)
    return {d.isoformat() for d in out}


def main():
    code = sys.argv[1]
    show = "--rust" in sys.argv
    style = "none"
    for s in STYLE:
        if s in sys.argv:
            style = s
    entries, err = derive(code)
    if err:
        print(f"FAIL {code}: {err}")
        return 1
    ours = resolve_table(entries, style)
    ref = {d.isoformat() for d in pyholidays.country_holidays(code, years=range(FIRST, LAST + 1))}
    if ours != ref:
        print(f"FAIL {code}: missing={len(ref - ours)} invented={len(ours - ref)}")
        for d in sorted(ref - ours)[:6]:
            print("      missing ", d)
        for d in sorted(ours - ref)[:6]:
            print("      invented", d)
        return 1
    print(f"ok  {code}  ({len(ours)} dates, style={style})")
    if show:
        print()
        for _, name, shape, frm, to in entries:
            print(f'    h("{name}", {rust_rule(shape, frm, to)}),')
    return 0


if __name__ == "__main__":
    sys.exit(main())
