#!/usr/bin/env python3
"""Review every crate in the workspace against the mature packages it competes
with, so the 0.1.6 development round is driven by measurements rather than by
what happens to be easy.

For each workspace crate this prints the crates.io incumbents that serve the
same capability, their recent downloads (the rolling ninety-day figure), their
release date, reverse dependencies and stars - the same columns round 19 used to
call something a gap. A crate is "served" when an incumbent is above about
twenty thousand recent downloads AND has an audience (reverse dependencies or a
repository that moves); a cluster of unadopted crates is a race, not a gap.

Usage:

    python3 review016.py all          # every workspace crate
    python3 review016.py <name>...    # a subset
    python3 review016.py names <n>... # is a crate name free?
"""

import sys
import time

from round15 import SLEEP, crate, fmt, npm, pypi, release_date, reverse_deps
from round16 import github

# The mature package each crate is measured against. The key is the workspace
# crate; the value is (upstream ecosystem, upstream package, crates.io
# incumbents to measure). The upstream figure is the demand; the incumbents are
# the supply. A crate with big upstream demand and only unadopted incumbents is
# worth extending; one whose incumbents are established is not a priority.
AREAS = {
    "xmlkit": ("npm", "fast-xml-parser", ["quick-xml", "roxmltree", "xml-rs", "serde-xml-rs"]),
    "holidays-rs": ("PyPI", "holidays", ["holidays", "public-holidays", "workalendar"]),
    "stdnum-rs": ("PyPI", "python-stdnum", ["stdnum", "id-validator", "validator"]),
    "spreadsheet-formula": ("npm", "formulajs", ["formula", "calamine", "formula-engine"]),
    "hijridate": ("PyPI", "hijridate", ["hijri", "hijri-converter", "hijridate"]),
    "iso20022-rs": ("PyPI", "iso20022", ["iso20022", "sepa", "mt940"]),
    "soap-client": ("PyPI", "zeep", ["wsdl", "soap", "rsoap"]),
    "iso6346": ("npm", "container-validator", ["iso6346", "container-validator"]),
    "eu-vat": ("PyPI", "python-stdnum", ["vat", "vat_id_validator", "eu-vat"]),
    "iso8583-msg": ("PyPI", "pyiso8583", ["iso8583", "iso8583-msg", "jpos"]),
    "arabic": ("PyPI", "arabic-reshaper", ["arabic-reshaper", "arabic", "unicode-bidi"]),
    "factur-x": ("PyPI", "factur-x", ["en16931", "zugferd", "factur-x"]),
    "wssec": ("PyPI", "zeep", ["wssecurity", "wssec", "xml-crypto"]),
    "pdfa": ("PyPI", "pikepdf", ["lopdf", "pdf-writer", "pdfa"]),
    "adhan": ("npm", "adhan", ["adhan", "praytimes", "salah"]),
    "object-hash": ("npm", "object-hash", ["object-hash", "sha1", "md5"]),
    "javaproperties": ("npm", "properties", ["java-properties", "javaproperties", "dotenv"]),
    "mllp": ("PyPI", "hl7apy", ["hl7", "hl7-mllp", "mllp"]),
    "ebics": ("PyPI", "python-ebics", ["ebics", "ebics-client"]),
    "pdfmake": ("npm", "pdfmake", ["printpdf", "pdf-canvas", "genpdf"]),
    "pades": ("PyPI", "endesive", ["pades", "signature", "cms"]),
    "fuzzy-score": ("PyPI", "thefuzz", ["strsim", "fuzzy-matcher", "rapidfuzz"]),
    "address-parser": ("npm", "parse-address", ["postal", "address", "us_address_parser"]),
}


def line(crate_name):
    c = crate(crate_name)
    if c is None:
        return f"    {crate_name:24s} FREE"
    ver, pub = release_date(crate_name)
    rd = reverse_deps(crate_name)
    gh = github(c["repo"])
    star = f"{gh['stars']}*" if gh and gh.get("stars") is not None else "-"
    arch = " ARCHIVED" if gh and gh.get("archived") else ""
    return (
        f"    {crate_name:24s} recent={fmt(c['recent']):>12s} v{ver or '?':9s} "
        f"pub={pub or '?':10s} revdeps={rd if rd is not None else '?':>4} "
        f"stars={star}{arch}"
    )


def upstream(kind, name):
    try:
        if kind == "npm":
            weekly = npm(name)
            if weekly is None:
                return f"    npm {name:24s} (not found)"
            return f"    npm {name:24s} {fmt(weekly):>12s}/week"
        monthly = pypi(name)
        if monthly is None:
            return f"    PyPI {name:23s} (not found)"
        return f"    PyPI {name:23s} {fmt(monthly):>12s}/month"
    except Exception as error:  # a network hiccup must not kill the review
        return f"    {kind} {name:24s} !! {error}"


def main(argv):
    if not argv or argv[0] == "all":
        names = list(AREAS)
    elif argv[0] == "names":
        for name in argv[1:]:
            print(line(name).replace("    ", "  "))
        return 0
    else:
        names = argv

    for name in names:
        if name not in AREAS:
            print(f"### {name} (no upstream mapping)")
            print(line(name))
            continue
        kind, pkg, incumbents = AREAS[name]
        print(f"### {name}")
        print(upstream(kind, pkg))
        for incumbent in incumbents:
            print(line(incumbent))
            time.sleep(SLEEP)
        time.sleep(SLEEP)
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
