"""Build the `closest_holiday` fixture for `crates/holidays-rs`.

`verify_holidays.py` already pins which dates each country observes. This pins
the *query* built on top of that set: the reference's `get_closest_holiday`,
which returns the first holiday strictly after a date (forward) or the last one
strictly before it (backward). The date is what the crate must reproduce; the
name is the crate's own and is checked separately, so the fixture records the
date only.

Every expectation is produced by the Python `holidays` package. Nothing is
hand-written.

Usage:
    python3 gen_holidays_closest.py [crate_dir]
"""

import datetime
import pathlib
import sys

import holidays as pyholidays

crate = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else "crates/holidays-rs")

# crate variant name -> reference country code. The full map from
# verify_holidays.py, minus countries whose reference carries non-formulaic
# ad-hoc days the crate documents as out of scope (see the filter below).
COUNTRIES = {
    "Indonesia": "ID", "UnitedStates": "US", "UnitedKingdom": "GB-ENG",
    "Germany": "DE", "France": "FR", "Netherlands": "NL", "Spain": "ES",
    "Italy": "IT", "Portugal": "PT", "Belgium": "BE", "Austria": "AT",
    "Poland": "PL", "Sweden": "SE", "Norway": "NO", "Denmark": "DK",
    "Finland": "FI", "Ireland": "IE", "Canada": "CA", "NewZealand": "NZ",
    "Czechia": "CZ", "Slovakia": "SK", "Croatia": "HR", "Slovenia": "SI",
    "Estonia": "EE", "Lithuania": "LT", "Luxembourg": "LU", "Iceland": "IS",
    "Cyprus": "CY", "Malta": "MT", "Greece": "GR", "Romania": "RO",
    "Mexico": "MX", "Latvia": "LV", "Brazil": "BR", "ElSalvador": "SV",
    "Nicaragua": "NI", "Peru": "PE", "Andorra": "AD", "CaboVerde": "CV",
    "FaroeIslands": "FO", "Greenland": "GL", "Switzerland": "CH",
    "PitcairnIslands": "PN", "Tokelau": "TK", "Samoa": "WS", "Australia": "AU",
    "Uruguay": "UY", "FrenchGuiana": "GF", "SaintBarthelemy": "BL",
    "SaintPierreAndMiquelon": "PM", "FrenchSouthernTerritories": "TF",
    "Reunion": "RE", "Mayotte": "YT", "Liberia": "LR", "Congo": "CG",
}

# Countries whose holiday *set* already differs from the reference in a way
# verify_holidays.py documents. A closest-holiday query over a different set
# gives a different answer, so these would test the set, not the search. Each
# reason names the same cause as the entry in verify_holidays.py's
# KNOWN_DIFFERENCES, or a weekend-redundant holiday the crate omits on purpose.
EXCLUDED = {
    "Indonesia": "non-formulaic ministerial-decree days (see verify_holidays.py)",
    "Spain": "crate keeps a national holiday that falls on a Sunday",
    "Ireland": "Saint Stephen's Day is observed on the substitute Monday",
    "Latvia": "cabinet-granted ad-hoc rest days",
    "Canada": "crate models the federal-wide set, the reference default is narrower",
    "UnitedStates": "reference carries '(observed)' duplicate dates",
    "UnitedKingdom": "reference carries '(observed)' duplicate dates",
    "NewZealand": "reference carries '(observed)' duplicate dates",
    "Belgium": "crate omits Easter Sunday and Whit Sunday, which are always Sundays",
    "Italy": "crate omits Easter Sunday, which is always a Sunday",
    "Mexico": "reference shifts a fixed holiday to a nearby Monday",
    "Greece": "crate omits the observed Labour Day duplicate",
}

YEARS = list(range(2018, 2034))

# Probes: the 1st, 15th and 28th of every month over the middle years, so a
# query lands before, on and after holidays, plus the very ends of a year where
# the answer crosses into the next one.
probes = []
for y in range(2020, 2031):
    for m in range(1, 13):
        for d in (1, 15, 28):
            probes.append(datetime.date(y, m, d))
probes += [datetime.date(2029, 12, 30), datetime.date(2029, 12, 31),
           datetime.date(2030, 1, 1), datetime.date(2030, 1, 2)]

rows = ["country\tprobe\tforward\tbackward"]
for name, code in COUNTRIES.items():
    if name in EXCLUDED:
        continue
    if code.endswith("-ENG"):
        cal = pyholidays.country_holidays(code[:-4], subdiv="ENG", years=YEARS)
    else:
        cal = pyholidays.country_holidays(code, years=YEARS)
    for p in probes:
        fwd = cal.get_closest_holiday(p, "forward")
        bwd = cal.get_closest_holiday(p, "backward")
        # Skip a row only when the reference itself found nothing; the crate's
        # `None` is then also correct.
        rows.append("\t".join([
            name,
            p.isoformat(),
            fwd[0].isoformat() if fwd else "-",
            bwd[0].isoformat() if bwd else "-",
        ]))

out = crate / "tests" / "fixtures" / "closest.tsv"
out.write_text("\n".join(rows) + "\n")
print(f"wrote {len(rows) - 1} rows to {out}")
