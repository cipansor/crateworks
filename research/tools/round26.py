#!/usr/bin/env python3
"""Round 26: eight further domains, plus the demand-first check rounds 22-25 asked for.

Rounds 8-25 covered the popularity lists, human formatting, dates, typography,
text repair, MIME, feeds, geo hashing, healthcare, energy, field buses, threat
intelligence, supply chain, market data, identity, cheminformatics,
bioinformatics, geospatial, audio, addresses, phones, recurrence, meteorology,
bibliography, office documents, EPUB, typesetting, diff, segmentation, emoji,
banking, product codes, quantitative finance, payment cards, aviation, clinical
terminology, JSON query languages, CAD, astronomy, video containers, email
validation, password strength, one-time passwords, semver, JWT/JWK, audio tags,
natural-language dates, CLDR plurals, label printers, mainframe data, maritime
telemetry, PostScript, vCard, RADIUS, Kerberos and NTP.

Round 25 closed by asking for the rule to be applied *before* a candidate is
recorded rather than as a check afterwards: search the capability, not the
upstream package's name, and sort the nearest crates by release date. This round
applies that rule from the first line.

Eight further domains, each one a business or developer application meets:

  * Indonesian and Malaysian government / pesantren administration identifiers
  * AIS/VDES and marine safety message *encoding* (round 25 covered decoding)
  * chess and general game notation (PGN, FEN, UCI, SAN)
  * geotechnical and civil engineering units (soil, concrete, survey)
  * archaeological and museum collection interchange (LIDO, CDWA, EAD)
  * pharmacy and medication dosing (NDC, RxNorm, ISMP dose checks)
  * civil aviation flight planning (ICAO flight plan, NOTAM, METAR decode)
  * courtroom and legal citation (Bluebook, ECLI, neutral citation)

Every figure is fetched live from `crates.io/api/v1`, `index.crates.io`,
`api.npmjs.org`, `pypistats.org` and `api.github.com`.

Usage:

    python3 round26.py sweep          # the eight new domains
    python3 round26.py names <n>...   # is a proposed crate name free?
"""

import sys

from round15 import crate, fmt, github, npm, pypi, release_date, reverse_deps  # noqa: F401


def measure(name):
    c = crate(name)
    if c is None:
        return f"  {name:26s} FREE"
    ver, pub = release_date(name)
    rd = reverse_deps(name)
    gh = github(c["repo"])
    star = f"{gh['stars']}*" if gh and gh.get("stars") is not None else "-"
    arch = " ARCHIVED" if gh and gh.get("archived") else ""
    return (
        f"  {name:26s} recent={fmt(c['recent']):>12s} v{ver or '?':9s} "
        f"pub={pub or '?':10s} revdeps={rd if rd is not None else '?':>4} "
        f"stars={star}{arch}"
    )


SWEEP = {
    "Indonesian and Malaysian administrative identifiers": {
        "rust": ["nik", "npwp", "kemenag", "emis", "dapodik", "pesantren",
                 "madrasah", "raport", "kurikulum-merdeka", "mykad", "nric"],
        "pypi": ["python-stdnum", "nik-validator"],
        "npm": ["nik-validator", "ktp-validator", "npwp-validator",
                "mykad-validator"],
    },
    "Marine safety message encoding (AIS/VDES, NMEA 0183 sentences)": {
        "rust": ["ais", "ais-encode", "aivdm", "vdes", "nmea", "nmea-parser",
                 "gpsd", "signal-k"],
        "pypi": ["pyais", "libais", "pynmea2", "nmeatools"],
        "npm": ["ais-stream", "nmea", "nmea-0183", "signalk-server"],
    },
    "Chess and general game notation (PGN, FEN, UCI, SAN)": {
        "rust": ["shakmaty", "chess", "pgn", "fen", "cozy-chess", "vampirc",
                 "chess-notation", "uci"],
        "pypi": ["python-chess", "chess", "pgn-parser"],
        "npm": ["chess.js", "chess", "pgn-parser", "cm-chessboard"],
    },
    "Geotechnical and civil engineering units (soil, concrete, survey)": {
        "rust": ["geotech", "soil", "concrete", "survey", "geodesy", "proj",
                 "geo-types", "crs-definitions"],
        "pypi": ["pyproj", "geopy", "soil-mechanics"],
        "npm": ["proj4", "geodesy", "mgrs", "utm"],
    },
    "Archaeological and museum collection interchange (LIDO, EAD, CDWA)": {
        "rust": ["lido", "cdwa", "ead", "museum", "collection", "dublin-core",
                 "oai-pmh", "marc"],
        "pypi": ["lido", "pymarc", "oai"],
        "npm": ["marcjs", "oai-pmh", "dublin-core"],
    },
    "Pharmacy and medication dosing (NDC, RxNorm, dose checks)": {
        "rust": ["ndc", "rxnorm", "drug", "pharmacy", "dose", "fhir",
                 "icd10", "snomed"],
        "pypi": ["rxnorm", "ndc", "pyfhir", "fhir.resources"],
        "npm": ["rxnorm", "fhir", "@types/fhir", "ndc-parser"],
    },
    "Civil aviation flight planning (ICAO flight plan, NOTAM, METAR decode)": {
        "rust": ["metar", "taf", "notam", "flight-plan", "icao", "aviation",
                 "aviation-data", "metar-parser"],
        "pypi": ["metar", "python-metar", "metar-taf-parser", "notam"],
        "npm": ["metar", "metar-taf-parser", "notams", "aviation-weather"],
    },
    "Legal citation (Bluebook, ECLI, neutral citation)": {
        "rust": ["bluebook", "ecli", "citation", "legal", "law", "court",
                 "jurisdiction", "lex"],
        "pypi": ["eyecite", "ecli", "citation-parser"],
        "npm": ["ecli", "citation", "legal-cite"],
    },
}


def show_sweep():
    for domain, groups in SWEEP.items():
        print("=" * 78)
        print(domain)
        print("=" * 78)
        print("  demand:")
        for p in groups["pypi"]:
            print(f"    pypi {p:26s} {fmt(pypi(p))}/month")
        for n in groups["npm"]:
            print(f"    npm  {n:26s} {fmt(npm(n))}/week")
        print("  rust:")
        for r in groups["rust"]:
            print(measure(r))
        print()


def show_names(names):
    for n in names:
        print(measure(n))


def main():
    if len(sys.argv) < 2:
        print(__doc__)
        return 1
    cmd = sys.argv[1]
    if cmd == "sweep":
        show_sweep()
    elif cmd == "names":
        show_names(sys.argv[2:])
    else:
        print(__doc__)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
