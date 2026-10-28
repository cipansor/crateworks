#!/usr/bin/env python3
"""Rank reference countries by how well they fit the holidays-rs `Rule` set.

A country is a candidate for the crate when every holiday it observes can be
expressed as one of the crate's rules (fixed date, nth weekday, Easter offset,
with a year bound). This prints the candidates with the fewest unexpressible
holidays first, so the cheap ones are tried before the expensive ones.

This ranks the *base* shapes only. A country can also fail here and still be
expressible, because `Rule::ObservedShift` gives each holiday its own weekend
shift table on top of the base shape; `derive_holidays.py` fits that too and is
the authority on whether a country actually fits.

Usage:  python3 fit_holidays.py [limit]
"""

import collections
import datetime
import os
import sys

import holidays as pyholidays

D = os.path.join(os.path.dirname(pyholidays.__file__), "countries")


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


def shape(dates):
    """The crate rule that would produce `dates`, or None."""
    if len({(d.month, d.day) for d in dates}) == 1:
        return "fixed"
    if (
        len({d.month for d in dates}) == 1
        and len({d.weekday() for d in dates}) == 1
        and len({(d.day - 1) // 7 for d in dates}) == 1
    ):
        return "nth"
    if len({(d - easter(d.year)).days for d in dates}) == 1:
        return "easter"
    return None


def main():
    limit = int(sys.argv[1]) if len(sys.argv) > 1 else 30
    rows = []
    for f in sorted(os.listdir(D)):
        if not f.endswith(".py") or f == "__init__.py":
            continue
        mod = f[:-3]
        try:
            cls = pyholidays.registry.COUNTRIES.get(mod.lower())
            code = cls[1] if isinstance(cls, tuple) else mod.upper()
            if len(code) != 2:
                continue
            h = pyholidays.country_holidays(code, years=range(2000, 2051))
        except Exception:
            continue
        byname = collections.defaultdict(list)
        for d, n in h.items():
            byname[n].append(d)
        if not byname:
            continue
        bad = [n for n, ds in byname.items() if shape(ds) is None]
        rows.append((len(bad), len(byname), code, mod, bad))
    rows.sort()
    for n, total, code, mod, bad in rows[:limit]:
        print(f"bad={n:2d} total={total:3d} {code} {mod}  {bad[:4]}")


if __name__ == "__main__":
    main()
