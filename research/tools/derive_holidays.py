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
# The most distinct `Rule`s one holiday name may need before the calendar is
# judged to need a table rather than a rule set. A name that is two observances
# needs two or three (one per run of years); a name whose dates are decided
# year by year needs one per year and is refused.
MAX_GROUPS = 8


class Unexpressible(Exception):
    """A holiday name that no small set of crate rules reproduces."""


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
        # A run of one year fires in that year only, and the gate has to say so
        # for every shape. `Rule::Once` is the crate's one-off for a fixed date;
        # a movable shape has no `Once`, so it is bounded with a one-year
        # `Between`. Letting a movable one-year run fall through to the
        # `Since` branch below rendered it open-ended, which invented a date in
        # every year after `to` - the defect the held-out check now catches.
        if kind == "fixed":
            return f"Rule::Once {{ year: {frm}, month: {shape[1]}, day: {shape[2]} }}"
        return f"Rule::Between {{ from: {frm}, to: {to}, base: &{core} }}"
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
    byname = collections.defaultdict(lambda: collections.defaultdict(list))
    for d, n in ref.items():
        # The reference collapses two holidays that fall on the same day into one
        # entry whose name is the two joined by "; " (ANZAC Day; Easter Monday,
        # 2011). Split it back into its parts so each holiday keeps a single name
        # and the year it was collapsed for is not left uncovered: the crate has
        # no such joined name, and treating the collapse as a gap would make the
        # individual holiday's rule skip that year.
        #
        # A name may also fall on more than one date in the same year, because
        # the reference reuses it for two observances (Hungary's `Nemzeti ünnep`
        # is 15 March and 23 October). The dates are kept as a list so the
        # grouping step can tell that apart from a holiday that merely moved.
        for part in n.split("; "):
            byname[part][d.year].append(d)
    base_names = {n for n in byname if not PAREN.search(n)}
    bases: dict = {}
    observed: dict = collections.defaultdict(set)
    for n, byyear in byname.items():
        m = PAREN.search(n)
        root = n[: m.start()] if m else None
        if root is not None and root in base_names:
            for dates in byyear.values():
                observed[root].update(dates)
        else:
            bases[n] = byyear
    return bases, observed


def safe_resolve(shape, y):
    """`resolve`, treating a shape that does not exist in `y` as no date.

    A candidate fitted to 29 February of a leap year is out of range in the
    year after, so a shape that fits the dates it was chosen for can still raise
    on a neighbouring year.
    """
    try:
        return resolve(shape, y)
    except ValueError:
        return None


def candidate_shapes(dates):
    """Every crate shape that fits a single date, cheapest first, deduplicated.

    `shapes_for` is called one date at a time so that a candidate which fits one
    observance of a name but not another is still offered; the caller then keeps
    the candidate that accounts for the most dates.
    """
    out = []
    seen = set()
    for d in dates:
        for shape in shapes_for([d]):
            if shape not in seen:
                seen.add(shape)
                out.append(shape)
    return out


def fit_shift(shape, actual, observed):
    """Fit `shape` plus a per-weekday shift table to the observed dates.

    The crate reports a holiday on the date it is *observed*, and
    `Rule::ObservedShift` carries one shift per weekday of the date it falls on.
    The reference reports the observed date either under a second key with a
    trailing parenthetical (Chile) or under the same key as the raw date
    (Aruba), so the actual date for a year is the parenthetical one when there
    is one and the reported date otherwise.

    Returns `(from_year, table)` when a single table reproduces every year, or
    `None`. An empty table means the shape alone already matches, and the caller
    then emits no shift.

    `from_year` is only advanced past the first year when every earlier year
    needs no shift at all. Without that check a wrong base shape fits by
    starting its table late, after the years that contradict it: Hungary's
    Easter Sunday fits `Rule::Fixed { month: 4, day: 5 }` from 2045 and nowhere
    before, which is not a rule.
    """
    peryear = {}
    for y, fall in actual.items():
        near = {o for o in observed if 0 < abs((o - fall).days) <= 7}
        if near:
            deltas = {(o - fall).days for o in near}
            if len(deltas) != 1:
                return None
            delta = deltas.pop()
        else:
            delta = 0
        base = safe_resolve(shape, y)
        if base is None:
            return None
        peryear[y] = (crate_weekday(base), (fall + datetime.timedelta(days=delta) - base).days)
    for frm in range(FIRST, LAST + 1):
        table: dict = {}
        for y in sorted(peryear):
            weekday, delta = peryear[y]
            if y < frm:
                if delta != 0:
                    break
                continue
            if weekday in table and table[weekday] != delta:
                break
            table[weekday] = delta
        else:
            return frm, {k: v for k, v in table.items() if v}
    return None


def shift_entries(name, byyear, observed):
    """Express a name as one base shape with a per-weekday shift, or `None`.

    A name that falls on more than one date in a year cannot be one rule with a
    shift, so it is left to `partition`.

    A shape that matches the dates with no shift at all is preferred over one
    that needs a shift table, and `candidate_shapes` orders the candidates
    cheapest first. Without that preference Madagascar's Father's Day, which is
    the third Sunday of June, came out as a fixed 21 June plus a six-entry shift
    table: correct, but a puzzle. `NthWeekday` says it in one line.
    """
    if any(len(dates) > 1 for dates in byyear.values()):
        return None
    actual = {y: dates[0] for y, dates in byyear.items()}
    candidates = candidate_shapes(list(actual.values()))
    fits = []
    for shape in candidates:
        fit = fit_shift(shape, actual, observed)
        if fit is not None:
            fits.append((shape, fit))
    for shape, (frm_shift, table) in fits:
        if table:
            continue
        return [
            (frm, name, shape, frm, to, None) for frm, to in contiguous(sorted(actual))
        ]
    if not fits:
        return None
    shape, (frm_shift, table) = fits[0]
    return [
        (frm, name, shape, frm, to, (frm_shift, table))
        for frm, to in contiguous(sorted(actual))
    ]


def partition(dates):
    """Split a name's `(year, date)` pairs into the fewest crate shapes.

    A name may cover two observances that no single rule spans (Hungary's
    `Nemzeti ünnep` is 15 March and 23 October). Each pass takes the candidate
    that accounts for the most remaining dates and removes them, so the count
    stays minimal and the loop ends because every pass removes at least one.
    Ties keep the earlier, cheaper candidate because `candidate_shapes` is
    ordered.

    Returns `None` when more than `MAX_GROUPS` shapes are needed, which is the
    signature of a calendar whose dates are decided rather than computed (Japan,
    China, the Islamic observances): a handful of `Once` entries is a table, a
    hundred is a dataset this crate does not carry.
    """
    remaining = list(dates)
    groups = []
    while remaining:
        best_shape = best_taken = None
        for shape in candidate_shapes([d for _, d in remaining]):
            taken = [(y, d) for y, d in remaining if safe_resolve(shape, y) == d]
            if best_taken is None or len(taken) > len(best_taken):
                best_shape, best_taken = shape, taken
        groups.append((best_shape, best_taken))
        if len(groups) > MAX_GROUPS:
            return None
        taken = set(best_taken)
        remaining = [pair for pair in remaining if pair not in taken]
    return groups


def derive(code):
    ref = pyholidays.country_holidays(code, years=range(FIRST, LAST + 1))
    bases, observed = split_observed(ref)
    entries = []
    for name, byyear in bases.items():
        try:
            entries.extend(name_entries(name, byyear, observed))
        except Unexpressible:
            return None, f"unexpressible: {name}"
    return entries, None


def name_entries(name, byyear, observed):
    """The crate entries for one holiday name.

    A name is tried as a single base rule with a per-weekday shift first, which
    is what most countries need and keeps the table short; only when no single
    shape fits does the name get split into several rules.
    """
    single = shift_entries(name, byyear, observed.get(name, set()))
    if single is not None:
        return single
    groups = partition([(y, d) for y in sorted(byyear) for d in byyear[y]])
    if groups is None:
        raise Unexpressible(name)
    entries = []
    for shape, matched in groups:
        for frm, to in contiguous(sorted({y for y, _ in matched})):
            entries.append((frm, name, shape, frm, to, None))
    return entries


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


def rule_applies(shape, frm, to, year):
    """Whether the rendered rule fires in `year`, mirroring `rust_rule`.

    A run that spans the whole fitted window is rendered without a gate and so
    fires in every year; a run that starts at `FIRST` becomes `Until`, which
    stops after `to`; one that ends at `LAST` becomes `Since`, which fires from
    `frm` onward; and anything narrower becomes `Between` or `Once`. Reproducing
    that mapping is what makes the held-out window mean the same thing the Rust
    side would compute.

    The order and the one-year case must match `rust_rule` exactly. A one-year
    run fires in that year only: `rust_rule` renders a fixed date as `Once` and
    any other shape as a one-year `Between`. Treating a movable one-year run as
    open-ended - which an earlier version of both functions did - is what let
    Madagascar's Mother's Day, a one-year `easter` run at `LAST`, be rendered as
    `Rule::Since` and invent a date in every later year.
    """
    if frm == FIRST and to == LAST:
        return True
    if frm == to:
        return year == frm
    if frm == FIRST:
        return year <= to
    if to == LAST:
        return year >= frm
    return frm <= year <= to


def resolve_range(entries, style, first, last):
    """`resolve_table`, but over an arbitrary year range.

    The fit is derived from 2000-2050, so re-resolving over a *later* window is
    what tells an over-fit from a rule: a table that only reproduces the fitted
    years is a lookup, not a calendar. A holiday gated to a run inside the
    fitted window is dropped here because the Rust side keeps that gate, which
    is exactly the behaviour a caller past 2050 would get.
    """
    out = set()
    for y in range(first, last + 1):
        falls = []
        for _, name, shape, frm, to, shifts in entries:
            if not rule_applies(shape, frm, to, y):
                continue
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
            out.add(moved)
    return {d.isoformat() for d in out}


def disagreements(code, entries, style, first, last):
    """The weekday gaps and invented dates over one window."""
    ours = resolve_range(entries, style, first, last)
    ref = {
        d.isoformat()
        for d in pyholidays.country_holidays(code, years=range(first, last + 1))
    }
    missing = [d for d in sorted(ref - ours) if datetime.date.fromisoformat(d).weekday() < 5]
    invented = sorted(ours - ref)
    return missing, invented


def vanished_names(code, entries, first, last):
    """Base names the reference reports every year in `[first, last]` but the
    fitted entries do not fire for in *every* one of those years.

    The date comparison in `disagreements` is blind to a holiday that a fit
    drops entirely, because a name that only ever fell on a Sunday has no
    weekday date to be missing. That is how a per-year `Between` stack, which
    is a table rather than a rule, passes the held-out check: the entries stop
    firing after `LAST` and nothing notices. Requiring a name the reference
    reports in each held-out year to be produced in each of those years is the
    "rule, not a table" property the crate's README states, tested rather than
    assumed.

    Coverage is per year, not pooled: producing the name in one held-out year
    and not the next is a gap, so the check compares year by year. Only a name
    the reference reports in *every* held-out year is considered, so a
    genuinely one-off or discontinued observance is not a failure.
    """
    ref = pyholidays.country_holidays(code, years=range(first, last + 1))
    bases, _ = split_observed(ref)
    ref_years = {name: {y for y in byyear} for name, byyear in bases.items()}
    span = list(range(first, last + 1))
    ours_by_year = {
        year: {
            name
            for _, name, shape, frm, to, _ in entries
            if rule_applies(shape, frm, to, year) and safe_resolve(shape, year) is not None
        }
        for year in span
    }
    every_year = set(span)
    return sorted(
        name
        for name, years in ref_years.items()
        if years == every_year
        and any(name not in ours_by_year[year] for year in span)
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
    # The fitted window cannot tell a rule from a table, so a calendar that only
    # reproduces 2000-2050 is refused rather than shipped: the crate's promise
    # is that a calendar stays correct for any year.
    hold_missing, hold_invented = disagreements(code, entries, style, LAST + 1, LAST + 25)
    vanished = vanished_names(code, entries, LAST + 1, LAST + 25)
    if hold_missing or hold_invented or vanished:
        print(
            f"FAIL {code}: over-fit, {LAST + 1}-{LAST + 25} has "
            f"missing={len(hold_missing)} invented={len(hold_invented)} "
            f"vanished={len(vanished)}"
        )
        for d in hold_missing[:6]:
            print("      missing ", d)
        for d in hold_invented[:6]:
            print("      invented", d)
        for n in vanished[:6]:
            print("      vanished", n)
        return 1
    print(f"ok  {code}  ({len(ours)} dates, style={style})")
    if show:
        print()
        for _, name, shape, frm, to, shifts in entries:
            print(f"    {rust_entry(name, shape, frm, to, shifts)},")
    return 0


if __name__ == "__main__":
    sys.exit(main())
