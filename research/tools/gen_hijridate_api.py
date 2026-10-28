"""Build the formatting-and-accessor fixture for `crates/hijridate`.

`gen_hijri_table.py` already pins the conversions, the month lengths and the
weekday, which are the parts with arithmetic in them. This pins the part that is
pure formatting and naming - `datetuple`, `isoformat`, `dmyformat`, `month_name`,
`day_name`, `year_length`, `month_length`, `isoweekday` and `notation` - because
those are exactly where a plausible-looking implementation drifts: a month name
with a different apostrophe, a `dmyformat` that pads the year, an `isoweekday`
off by one.

Every expectation is produced by the `hijridate` Python package (MIT,
(c) Mohammed Alshehri), the same reference the Umm al-Qura table came from.
Nothing is hand-written.

Usage:
    python3 gen_hijridate_api.py [crate_dir]
"""

import calendar
import pathlib
import sys

from hijridate import Gregorian, Hijri
from hijridate import ummalqura as u

Y0 = u.HIJRI_RANGE[0][0]
Y1 = u.HIJRI_RANGE[1][0]

crate = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "crates/hijridate")

HEADER = (
    "kind\tyear\tmonth\tday\tisoformat\tdmyformat\tmonth_name\tday_name\t"
    "year_length\tmonth_length\tisoweekday\tnotation"
)
rows = [HEADER]


def hijri_row(kind, h):
    return "\t".join([
        kind, str(h.year), str(h.month), str(h.day),
        h.isoformat(), h.dmyformat(), h.month_name(), h.day_name(),
        str(h.year_length()), str(h.month_length()), str(h.isoweekday()),
        h.notation(),
    ])


def gregorian_row(kind, g):
    return "\t".join([
        kind, str(g.year), str(g.month), str(g.day),
        g.isoformat(), g.dmyformat(), g.month_name(), g.day_name(),
        str(366 if calendar.isleap(g.year) else 365),
        str(calendar.monthrange(g.year, g.month)[1]),
        str(g.isoweekday()), g.notation(),
    ])


# Every month of a spread of years, so every month name and both month lengths
# appear. Day 1 only: the conversions and day counting are the other fixture's job.
for y in sorted({Y0, Y0 + 1, 1400, 1447, 1450, Y1 - 1, Y1}):
    for m in range(1, 13):
        rows.append(hijri_row("H", Hijri(y, m, 1)))

# All seven weekdays, so every day name appears and `isoweekday` is checked on each.
for offset in range(7):
    rows.append(hijri_row("H", Hijri(1447, 9, 1 + offset)))

# Every Gregorian month, across common years and a leap year.
for y in (1999, 2000, 2024, 2026):
    for m in range(1, 13):
        rows.append(gregorian_row("G", Gregorian(y, m, 1)))

# `dmyformat` with another separator, and with padding off, on both types.
for kind, date in [("H", Hijri(1447, 1, 5)), ("G", Gregorian(2026, 2, 5))]:
    for suffix, text in [
        ("n", date.dmyformat("-")),
        ("p", date.dmyformat("/", padding=False)),
    ]:
        fields = (hijri_row if kind == "H" else gregorian_row)(kind + suffix, date).split("\t")
        fields[5] = text
        rows.append("\t".join(fields))

# `fromisoformat` round trip, so the parser is pinned to the formatter.
for kind, date in [("H", Hijri(1447, 9, 1)), ("G", Gregorian(2026, 2, 18))]:
    parsed = (Hijri if kind == "H" else Gregorian).fromisoformat(date.isoformat())
    assert parsed.isoformat() == date.isoformat()
    rows.append((hijri_row if kind == "H" else gregorian_row)(kind + "i", parsed))

out = crate / "tests" / "fixtures" / "hijridate_api.tsv"
out.write_text("\n".join(rows) + "\n")
print(f"wrote {len(rows) - 1} rows to {out}")


# The Gregorian ordinal, its inverse, and `replace`. Only the Gregorian type
# carries these in the reference, so the fixture is Gregorian-only. The ordinal
# is the proleptic day count from 0001-01-01, which is what `datetime.date`
# uses and what a caller passes to `date.fromordinal`.
ordinal_rows = ["year\tmonth\tday\tordinal\treplaced"]
for y in (1, 2, 4, 1582, 1600, 1900, 1999, 2000, 2024, 2026, 9999):
    for m, d in ((1, 1), (2, 28), (3, 1), (12, 31)):
        try:
            g = Gregorian(y, m, d)
        except ValueError:
            continue
        replaced = g.replace(day=1)
        ordinal_rows.append(
            f"{y}\t{m}\t{d}\t{g.toordinal()}\t{replaced.isoformat()}"
        )

ordinal_out = crate / "tests" / "fixtures" / "hijridate_ordinal.tsv"
ordinal_out.write_text("\n".join(ordinal_rows) + "\n")
print(f"wrote {len(ordinal_rows) - 1} rows to {ordinal_out}")


# The ISO week date and its inverse. The reference exposes both on the Gregorian
# type only. A row is a date and its `isocalendar`; the inverse is checked by
# feeding the same three numbers back to `fromisocalendar`, which the Rust test
# does. The week-numbering year is not the calendar year at the year's ends, so
# late December and early January are included deliberately.
iso_rows = ["year\tmonth\tday\tiso_year\tiso_week\tiso_weekday"]
iso_dates = set()
for y in (1, 2, 4, 1582, 1600, 1900, 1999, 2000, 2004, 2015, 2019, 2020, 2021, 2022, 2024, 2025, 2026, 2027, 9999):
    # The whole of the first and last ten days of the year, where the ISO year
    # and the calendar year disagree.
    for m, days in ((1, range(1, 11)), (12, range(22, 32))):
        for d in days:
            try:
                iso_dates.add(Gregorian(y, m, d))
            except ValueError:
                pass
    for m in range(1, 13):
        iso_dates.add(Gregorian(y, m, 1))
        iso_dates.add(Gregorian(y, m, 15))
for g in sorted(iso_dates):
    iy, iw, iwd = g.isocalendar()
    iso_rows.append(f"{g.year}\t{g.month}\t{g.day}\t{iy}\t{iw}\t{iwd}")

iso_out = crate / "tests" / "fixtures" / "hijridate_isocalendar.tsv"
iso_out.write_text("\n".join(iso_rows) + "\n")
print(f"wrote {len(iso_rows) - 1} rows to {iso_out}")
