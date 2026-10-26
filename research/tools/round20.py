#!/usr/bin/env python3
"""Round 20: eight domains rounds 8-19 did not sweep.

The standing lesson of rounds 15-19 is that a gap recorded in a document is not
a gap today, so every round starts by re-measuring the open survivors and then
sweeps domains no earlier round touched. Rounds 15-19 already covered human
formatting, natural-language dates, typography, text repair, MIME sniffing, web
feeds, geo hashing, healthcare, energy, field buses, threat intelligence,
supply chain, market data, identity, cheminformatics, bioinformatics,
geospatial, audio, addresses, phones, recurrence, meteorology, bibliography,
office documents, EPUB, typesetting, diff, segmentation, emoji, banking and
product codes.

This round takes eight domains those sweeps left: quantitative finance,
payment-card data, aviation reference data, clinical terminology, JSON query
languages, CAD interchange, astronomy data, and video container parsing.

The rule rounds 15-19 established is applied throughout: before proposing a
crate, sort the nearest crates by *release date*, not only by downloads, and ask
whether the incumbent has an audience. A cluster of unadopted crates is a race,
not a gap. A keyword search on the upstream package's name is not a gap check;
search the capability.

Every figure is fetched live from `crates.io/api/v1`, `index.crates.io` (the
sparse index, for a real release date), `api.npmjs.org`, `pypistats.org` and
`api.github.com`.

Usage:

    python3 round20.py sweep          # the eight new domains
    python3 round20.py names <n>...   # is a proposed crate name free?
"""

import sys

from round15 import SLEEP, crate, fmt, github, npm, pypi, release_date, reverse_deps  # noqa: F401


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


# Eight domains no earlier round swept. Each entry is the Rust crates that
# serve the capability, plus the source packages that show the demand.
SWEEP = {
    "quantitative finance: technical indicators": {
        "rust": ["ta", "ta-rs", "talipp", "technical_indicators", "indicators",
                 "fin-ta", "rust_ti"],
        "pypi": ["TA-Lib", "pandas-ta", "finta", "talipp"],
        "npm": ["technicalindicators", "tulind", "indicatorts"],
    },
    "payment cards: PAN validation, BIN/IIN, Luhn": {
        "rust": ["card-validate", "creditcard", "credit-card", "cardinity",
                 "luhn", "card_validate"],
        "pypi": ["creditcard", "card-identifier", "stripe"],
        "npm": ["card-validator", "credit-card-type", "luhn", "payment"],
    },
    "aviation reference data: IATA/ICAO codes, routes": {
        "rust": ["airport", "airports", "iata", "icao", "airline-codes",
                 "flights", "aviation"],
        "pypi": ["airportsdata", "airport-codes", "openap"],
        "npm": ["airport-codes", "airport-data", "iata-codes"],
    },
    "clinical terminology: ICD-10, LOINC, SNOMED": {
        "rust": ["icd", "icd10", "loinc", "snomed", "snomedct", "fhir", "hl7-fhir"],
        "pypi": ["fhir.resources", "pymedtermino", "icd10-cm", "loinc2hpo"],
        "npm": ["fhir", "fhirclient", "snomed"],
    },
    "JSON query languages: JMESPath, JSONPath, JSONata": {
        "rust": ["jmespath", "jsonpath", "jsonpath-rust", "jsonata", "jsonpath_lib",
                 "jq-rs", "jaq"],
        "pypi": ["jmespath", "jsonpath-ng", "jsonata"],
        "npm": ["jmespath", "jsonpath-plus", "jsonata", "jq-web"],
    },
    "CAD interchange: DXF, DWG, STEP, IGES": {
        "rust": ["dxf", "dxf-rs", "step", "iges", "stepfile", "truck", "opencascade"],
        "pypi": ["ezdxf", "steputils", "pythonocc-core"],
        "npm": ["dxf-parser", "dxf", "dxf-writer", "occt-import-js"],
    },
    "astronomy: FITS, ephemerides, coordinates": {
        "rust": ["fitsio", "fits", "astro", "astronomy", "ephemeris", "sofars",
                 "sunrise", "astro-rust", "rust-ephem"],
        "pypi": ["astropy", "skyfield", "pyephem", "pyswisseph"],
        "npm": ["astronomy-engine", "suncalc", "satellite.js"],
    },
    "video containers: MP4/MOV boxes, Matroska": {
        "rust": ["mp4", "mp4parse", "matroska", "mkv", "isobmff", "mp4box",
                 "ebml-iterable"],
        "pypi": ["pymediainfo", "mutagen", "ffmpeg-python"],
        "npm": ["mp4box", "matroska", "ebml", "mediainfo.js"],
    },
}


def sweep():
    for domain, spec in SWEEP.items():
        print(f"\n=== {domain}")
        for name in spec["rust"]:
            print(measure(name))
        for eco, names in (("PyPI", spec["pypi"]), ("npm", spec["npm"])):
            for name in names:
                n = pypi(name) if eco == "PyPI" else npm(name)
                unit = "mo" if eco == "PyPI" else "wk"
                print(f"    {eco:4s} {name:24s} {fmt(n)}/{unit}")


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    mode = sys.argv[1]
    if mode == "sweep":
        sweep()
    elif mode == "names":
        for name in sys.argv[2:]:
            print(measure(name))
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
