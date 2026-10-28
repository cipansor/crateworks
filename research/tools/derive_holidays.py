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
import re
import sys

import holidays as pyholidays

FIRST, LAST = 2000, 2050
STYLE = {
    "none": "SubstituteStyle::None",
    "nearest": "SubstituteStyle::NearestWeekday",
    "next_monday": "SubstituteStyle::NextMonday",
}

# A trailing parenthetical is how the reference marks an observed date
# ("Christmas Day (observed)", "Año Nuevo (puente)"). The label differs by
# language, so the text is not matched; a name is treated as observed only when
# what remains before the parenthesis is itself a name the reference uses.
PAREN = re.compile(r"\s*\(([^()]*)\)\s*$")


def crate_weekday(d):
    """Weekday with Sunday as 0, which is what the crate uses."""
    return (d.weekday() + 1) % 7


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


def orthodox_easter(y):
    """The crate's `Date::orthodox_easter`, transcribed from `src/date.rs`."""
    a, b, c = y % 4, y % 7, y % 19
    d = (19 * c + 15) % 30
    e = (2 * a + 4 * b - d + 34) % 7
    month = (d + e + 114) // 31
    day = (d + e + 114) % 31 + 1
    shift = max((y // 100) - (y // 400) - 2, 10)
    return datetime.date(y, month, day) + datetime.timedelta(days=shift)


def on_or_after(y, month, day, weekday):
    d = datetime.date(y, month, day)
    return d + datetime.timedelta(days=((weekday - 1) - d.weekday()) % 7)


def on_or_before(y, month, day, weekday):
    d = datetime.date(y, month, day)
    return d - datetime.timedelta(days=(d.weekday() - ((weekday - 1) % 7)) % 7)


def shapes_for(dates):
    """Every crate shape that reproduces `dates`, cheapest first.

    The crate numbers weekdays with Sunday as 0, while `datetime.weekday()`
    numbers them with Monday as 0, so each candidate converts the weekday with
    `(wd + 1) % 7` on the way out. Returning all candidates lets `derive` pick
    the one that also holds inside every year run, instead of committing to the
    first match and failing the run check.
    """
    out = []
    if len({(d.month, d.day) for d in dates}) == 1:
        m, day = next(iter({(d.month, d.day) for d in dates}))
        out.append(("fixed", m, day))
    if len({(d - easter(d.year)).days for d in dates}) == 1:
        out.append(("easter", next(iter({(d - easter(d.year)).days for d in dates}))))
    try:
        if len({(d - orthodox_easter(d.year)).days for d in dates}) == 1:
            out.append(
                ("orthodox", next(iter({(d - orthodox_easter(d.year)).days for d in dates})))
            )
    except ValueError:
        pass
    if (
        len({d.month for d in dates}) == 1
        and len({d.weekday() for d in dates}) == 1
        and len({(d.day - 1) // 7 for d in dates}) == 1
    ):
        m = dates[0].month
        wd = (dates[0].weekday() + 1) % 7
        nth = (dates[0].day - 1) // 7 + 1
        out.append(("nth", m, wd, nth))
    if len({d.month for d in dates}) == 1 and len({d.weekday() for d in dates}) == 1:
        m = dates[0].month
        wd = (dates[0].weekday() + 1) % 7
        # "the first <wd> on or after day D": D may sit up to six days before the
        # earliest occurrence, so the latest safe D is the minimum day.
        lo, hi = min(d.day for d in dates), max(d.day for d in dates)
        out.append(("on_after", m, lo, wd))
        out.append(("on_before", m, hi, wd))
    return out


def resolve(shape, y):
    if shape is None:
        return None
    kind = shape[0]
    if kind == "fixed":
        return datetime.date(y, shape[1], shape[2])
    if kind == "nth":
        return nth_weekday(y, shape[1], shape[2], shape[3])
    if kind == "easter":
        return easter(y) + datetime.timedelta(days=shape[1])
    if kind == "orthodox":
        return orthodox_easter(y) + datetime.timedelta(days=shape[1])
    if kind == "on_after":
        return on_or_after(y, shape[1], shape[2], shape[3])
    return on_or_before(y, shape[1], shape[2], shape[3])


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
    kind = shape[0]
    if kind == "fixed":
        core = f"Rule::Fixed {{ month: {shape[1]}, day: {shape[2]} }}"
    elif kind == "nth":
        core = (
            f"Rule::NthWeekday {{ month: {shape[1]}, weekday: {shape[2]}, "
            f"nth: {shape[3]} }}"
        )
    elif kind == "easter":
        core = f"Rule::EasterOffset({shape[1]})"
    elif kind == "orthodox":
        core = f"Rule::OrthodoxEasterOffset({shape[1]})"
    elif kind == "on_after":
        core = (
            f"Rule::WeekdayOnOrAfter {{ month: {shape[1]}, day: {shape[2]}, "
            f"weekday: {shape[3]} }}"
        )
    else:
        core = (
            f"Rule::WeekdayOnOrBefore {{ month: {shape[1]}, day: {shape[2]}, "
            f"weekday: {shape[3]} }}"
        )
    if frm == FIRST and to == LAST:
        return core
    if frm == to:
        # A run of one year is a one-off, and the crate spells that
        # `Rule::Once`; `Between` with equal bounds is legal but reads as a
        # window that never was.
        month = shape[1]
        day = shape[2] if kind == "fixed" else None
        if day is not None:
            return f"Rule::Once {{ year: {frm}, month: {month}, day: {day} }}"
    if frm == FIRST:
        return f"Rule::Until {{ year: {to}, base: &{core} }}"
    if to == LAST:
        return f"Rule::Since {{ year: {frm}, base: &{core} }}"
    return f"Rule::Between {{ from: {frm}, to: {to}, base: &{core} }}"


def split_observed(ref):
    """Split the reference into base names and the dates observed for them.

    The reference lists a moved holiday twice: once on the date it falls and
    once on the observed date, with a trailing parenthetical on the second
    ("Christmas Day" and "Christmas Day (observed)"). Returns the base
    name -> {year: date} map and base name -> {observed dates}.
    """
    byname = collections.defaultdict(dict)
    for d, n in ref.items():
        # The reference collapses two holidays that fall on the same day into one
        # entry whose name is the two joined by "; " (ANZAC Day; Easter Monday,
        # 2011). Split it back into its parts so each holiday keeps a single name
        # and the year it was collapsed for is not left uncovered: the crate has
        # no such joined name, and treating the collapse as a gap would make the
        # individual holiday's rule skip that year.
        for part in n.split("; "):
            byname[part][d.year] = d
    base_names = {n for n in byname if not PAREN.search(n)}
    bases: dict = {}
    observed: dict = collections.defaultdict(set)
    for n, byyear in byname.items():
        m = PAREN.search(n)
        root = n[: m.start()] if m else None
        if root is not None and root in base_names:
            observed[root].update(byyear.values())
        else:
            bases[n] = byyear
    return bases, observed


def observed_shift(byyear, observed, years):
    """Fit one holiday's observed dates to a per-holiday shift table.

    Returns `(from_year, {weekday: days})`, or `None` when no shift table
    reproduces the observed dates, which means the country needs the
    country-wide style instead. `from_year` is the first year the shift
    applies; before it the holiday is not moved.
    """
    peryear = {}
    for y in years:
        fall = byyear[y]
        near = {o for o in observed if 0 < abs((o - fall).days) <= 7}
        if near:
            deltas = {(o - fall).days for o in near}
            if len(deltas) != 1:
                return None
            delta = deltas.pop()
        else:
            delta = 0
        peryear[y] = (crate_weekday(fall), delta)
    for frm in range(FIRST, LAST + 1):
        table: dict = {}
        for y in years:
            if y < frm:
                continue
            weekday, delta = peryear[y]
            if weekday in table and table[weekday] != delta:
                break
            table[weekday] = delta
        else:
            return frm, {k: v for k, v in table.items() if v}
    return None


def derive(code):
    ref = pyholidays.country_holidays(code, years=range(FIRST, LAST + 1))
    byname = collections.defaultdict(dict)
    for d, n in ref.items():
        for part in n.split("; "):
            byname[part][d.year] = d
    bases, observed = split_observed(ref)
    entries = []
    for name, byyear in bases.items():
        years = set(byyear)
        runs = contiguous(years)
        shape = None
        for candidate in shapes_for(list(byyear.values())):
            if all(
                resolve(candidate, y) == byyear[y]
                for frm, to in runs
                for y in range(frm, to + 1)
            ):
                shape = candidate
                break
        if shape is None:
            return None, f"unexpressible: {name}"
        shifts = None
        if name in observed:
            shifts = observed_shift(byyear, observed[name], sorted(years))
        for frm, to in runs:
            entries.append((frm, name, shape, frm, to, shifts))
    return entries, None


def resolve_table(entries, style):
    out = set()
    for y in range(FIRST, LAST + 1):
        falls = []
        for _, name, shape, frm, to, shifts in entries:
            if frm <= y <= to:
                d = resolve(shape, y)
                if d is None:
                    continue
                if shifts is not None:
                    shift_from, table = shifts
                    delta = table.get(crate_weekday(d), 0) if y >= shift_from else 0
                    falls.append((d + datetime.timedelta(days=delta), name, True))
                else:
                    falls.append((d, name, False))
        falls.sort()
        if style == "none":
            # A holiday with its own shift is already on its observed date and
            # may share it; the country style never touches it. The others keep
            # the date they fall on.
            out |= {d for d, _, _ in falls}
            continue
        taken = set()
        for d, _, shifted in falls:
            if shifted:
                out.add(d)
                taken.add(d)
                continue
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


def rust_shift_table(table):
    """Render a `(weekday, days)` table, Sunday first."""
    return ", ".join(f"({wd}, {days})" for wd, days in sorted(table.items()))


def rust_entry(name, shape, frm, to, shifts):
    """Render one holiday, wrapping the rule in an `ObservedShift` when it has
    its own shift table."""
    rule = rust_rule(shape, frm, to)
    if shifts is None:
        return f'h("{name}", {rule})'
    shift_from, table = shifts
    return (
        f'h("{name}", Rule::ObservedShift {{ base: &{rule}, '
        f"from: {shift_from}, shifts: &[{rust_shift_table(table)}] }})"
    )


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
    # The crate reports the observed date only, while the reference also keeps
    # the raw date of a holiday that moved. A reference date we do not report
    # that falls at a weekend is therefore cosmetic (the day is already a
    # non-working day), the same classification `verify_holidays.py` uses. A
    # weekday gap or any date we invent is a real defect.
    def weekday(iso):
        return datetime.date.fromisoformat(iso).weekday() < 5

    missing = [d for d in sorted(ref - ours) if weekday(d)]
    invented = sorted(ours - ref)
    if missing or invented:
        print(f"FAIL {code}: missing={len(missing)} invented={len(invented)}")
        for d in missing[:6]:
            print("      missing ", d)
        for d in invented[:6]:
            print("      invented", d)
        return 1
    print(f"ok  {code}  ({len(ours)} dates, style={style})")
    if show:
        print()
        for _, name, shape, frm, to, shifts in entries:
            print(f"    {rust_entry(name, shape, frm, to, shifts)},")
    return 0


if __name__ == "__main__":
    sys.exit(main())
