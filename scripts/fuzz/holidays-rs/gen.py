#!/usr/bin/env python3
"""Build the holidays-rs oracle corpus.

`verify_holidays.py` compares every country table against the Python `holidays`
package, but it does so *live*: it needs the reference installed and CI does not
run it, so a change that broke a calendar would be caught only by whoever
remembered to run the gate. This generator freezes the reference's answer as a
committed fixture, so the crate's own test suite replays it without the
reference installed.

The six calendars added in 0.8.0 - Bermuda, Dominica, Grenada, Gibraltar, the
Solomon Islands and the British Virgin Islands - each carry a per-holiday weekend
shift, and that is the part a hand-written case misses. A test that asserts
"Bermuda Day is 22 May 2026" says nothing about the Saturday New Year's shift
(`(0, 1), (6, 2)`) or the Solomon Islands' backward move (`(6, -1)`); a Saturday
occurrence in the corpus does.

The 0.8.1 fix bounded several tables at the year each holiday actually began,
which `historical_bounds.rs` then checks at the two years either side of a
bound. That file probes boundaries; this one covers the interior, so the two
answer different questions. The list grew from six to 31: the calendars this
round changed, plus the ones whose history it rewrote, plus Congo, which was
gate-clean and uncovered. It is not every gate-clean calendar - 44 of those are
still absent, plus five the gate records as known differences - so the corpus is
a growing subset, not a complete freeze.

`EXCLUDED` is not a catalogue of the gate's differences; the gate reports those
itself. It holds a calendar the corpus would otherwise freeze but cannot, with
the reason, and today that is only Botswana, which agrees through 2049 and
diverges in the 25 years the extended window adds. A calendar belongs there only
if freezing its answer would record something wrong; a calendar the gate merely
fails is not a corpus entry at all.

The corpus is the reference's answer over the whole window, not a sample of it.
A calendar's oracle is small enough to take whole, and a random sample can miss
the one year a shift rule diverges, so the fixture lists every reference date in
the three classes `verify_holidays.py` separates:

* `must`   - a reference holiday on a working day. The crate reports the
             observed date only, so it owes this date: a `must` date the crate
             does not report is a working day it calls working.
* `allow`  - a reference holiday that falls on a Saturday or Sunday. The crate
             may report it or not - the reference keeps the raw date, the crate
             usually moves it, and both are correct - so it is listed only to
             bound the crate's dates from above.
* `absent` - a seeded random date the reference has no holiday for. The crate
             must report nothing. A `must`-only corpus leaves this direction
             untested, so a seeded sample of the complement is included.

The test checks both directions: a crate date outside `must ∪ allow` is an
invented day off, and a `must` date the crate does not report is a missing one.
That is exactly what `verify_holidays.py` checks, frozen into the repository.

The probe dates are seeded, so the same script produces the same corpus every
time.

Usage:
    python3 gen.py [crate_dir]
Needs:
    pip install 'holidays==0.108'
"""

import datetime
import pathlib
import random
import sys

import holidays as pyholidays

PINNED = "0.108"
if pyholidays.__version__ != PINNED:
    raise SystemExit(
        f"holidays {pyholidays.__version__} is installed, but this corpus is "
        f"baselined against {PINNED} (pip install 'holidays=={PINNED}')."
    )

CRATE = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "crates/holidays-rs")

# crate variant name -> (reference code, the reference's own first year).
COUNTRIES = {
    "Bermuda": ("BM", 1948),
    "Dominica": ("DM", 1990),
    "Grenada": ("GD", 2000),
    "Gibraltar": ("GI", 2000),
    "SolomonIslands": ("SB", 1979),
    "BritishVirginIslands": ("VG", 1967),
    # Added for the 0.8.1 historical-bounds fix. Each one was gate-clean over
    # the whole window before being added, so the corpus is a freeze of an
    # answer that already agreed rather than a new claim.
    "Armenia": ("AM", 1991),
    "CaboVerde": ("CV", 1976),
    "Croatia": ("HR", 1992),
    "Cyprus": ("CY", 1961),
    "Estonia": ("EE", 1991),
    "FrenchSouthernTerritories": ("TF", 1956),
    "Germany": ("DE", 1991),
    "Lesotho": ("LS", 1996),
    "Liberia": ("LR", 1957),
    "Lithuania": ("LT", 1990),
    "Micronesia": ("FM", 1987),
    "Namibia": ("NA", 1991),
    "SaintLucia": ("LC", 1979),
    "Seychelles": ("SC", 1994),
    "WallisAndFutuna": ("WF", 1962),
    # Gate-clean from its 1994 start year, so it can be frozen. The window
    # starts at that year, which is also where its Labour Day begins, so the
    # bound itself is guarded by historical_bounds.rs rather than here.
    "Congo": ("CG", 1994),
    # The French family, added once the 0.8.1 fix bounded their shared days
    # (1 May 1919, 11 November 1922, the abolition days in 1983-1984) and
    # turned the two `Until`-only rules into `Between`. Before that each one
    # failed the gate, and freezing a failing answer would have recorded the
    # leak as correct.
    "France": ("FR", 1803),
    "FrenchGuiana": ("GF", 1815),
    "Guadeloupe": ("GP", 1815),
    "Martinique": ("MQ", 1815),
    "Reunion": ("RE", 1815),
    "Mayotte": ("YT", 1842),
    "NewCaledonia": ("NC", 1854),
    "FrenchPolynesia": ("PF", 1881),
    "SaintPierreAndMiquelon": ("PM", 1815),
}

# Calendars the corpus deliberately leaves out, with the reason. A calendar
# belongs here only while `verify_holidays.py` reports a difference for it; a
# table that agrees everywhere is added above instead.
EXCLUDED = {
    "Botswana": (
        "the extra Boxing Day and Labour Day holidays are pinned as `Rule::Once` "
        "years (2020, 2026, 2037, ...) rather than derived from the weekday, so "
        "the table stops agreeing past 2049. Freezing the reference's answer "
        "would bake that lookup table in as if it were correct."
    ),
}

# The same floor `verify_holidays.py` uses. Before 1900 the tables were never
# fitted against the reference, so a difference there is not evidence about
# the crate; freezing those years would encode the crate's own guesses.
EARLIEST = 1900
LAST = 2075
# A fixed seed keeps the corpus reproducible; the country code is mixed in so
# adding a country does not reshuffle the others.
SEED = 0x686F6C69  # "holi"
ABSENT_PER_COUNTRY = 600


def main() -> int:
    out = CRATE / "tests" / "fixtures" / "holidays_oracle.tsv"
    overlap = sorted(set(COUNTRIES) & set(EXCLUDED))
    if overlap:
        raise SystemExit(
            f"{', '.join(overlap)} is in both COUNTRIES and EXCLUDED; "
            "a calendar is either agreed with the reference or left out with a reason"
        )
    rows = []
    for name, (code, first) in COUNTRIES.items():
        first = max(first, EARLIEST)
        cal = pyholidays.country_holidays(code, years=list(range(first, LAST + 1)))
        full = set(cal.keys())
        for d in sorted(full):
            rows.append((name, d, "must" if d.weekday() < 5 else "allow"))
        # Seeded dates the reference has nothing for: the crate must be silent.
        rng = random.Random(f"{SEED}:{code}")
        start = datetime.date(first, 1, 1).toordinal()
        end = datetime.date(LAST, 12, 31).toordinal()
        added = 0
        while added < ABSENT_PER_COUNTRY:
            d = datetime.date.fromordinal(rng.randint(start, end))
            if d in full:
                continue
            rows.append((name, d, "absent"))
            added += 1

    lines = [
        "# Generated by scripts/fuzz/holidays-rs/gen.py. Do not edit.",
        "# country\tdate\tkind",
        "# kind=must   the crate must report this date",
        "# kind=allow  a weekend reference date; the crate may report it or not",
        "# kind=absent the crate must report nothing for this date",
        "#",
        "# Calendars deliberately left out, and why:",
    ]
    lines += [f"#   {name}: {why}" for name, why in EXCLUDED.items()]
    lines += [f"{name}\t{d.isoformat()}\t{kind}" for name, d, kind in rows]
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")
    counts = {}
    for _, _, kind in rows:
        counts[kind] = counts.get(kind, 0) + 1
    print(f"wrote {out}: {counts} over {len(COUNTRIES)} countries")
    return 0


if __name__ == "__main__":
    sys.exit(main())
