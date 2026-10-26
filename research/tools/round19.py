#!/usr/bin/env python3
"""Round 19: sweep four domains earlier rounds did not reach, and re-measure
the open survivors.

Rounds 8-18 each re-measured every open candidate and then swept domains no
earlier round had touched. This round does the same. The four new domains are
ones the npm and PyPI popularity lists under-represent, but which a business
or a localisation pipeline meets constantly:

    bank and IBAN tooling    account-number validation, SEPA, BBAN/IBAN bridge
    product and price codes  GTIN/EAN, UPC, price and quantity barcodes
    address parsing          free-text postal addresses to components
    phone numbers            E.164 parsing and formatting

The rule rounds 15-18 established is applied throughout: before proposing a
crate, sort the nearest crates by *release date*, not only by downloads, and
ask whether the incumbent has an audience. A cluster of unadopted crates is a
race, not a gap. A `FREE` name is re-checked.

Every figure is fetched live from `crates.io/api/v1`, `index.crates.io` (the
sparse index, for a real release date), `api.npmjs.org`, `pypistats.org` and
`api.github.com`.

Usage:

    python3 round19.py candidates   # re-measure the open survivors
    python3 round19.py sweep        # the four new domains
    python3 round19.py names <n>... # is a proposed crate name free?
"""

import sys
import time

from round15 import SLEEP, crate, fmt, npm, pypi, release_date, reverse_deps
from round16 import github


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


CANDIDATES = {
    "OFX/QIF bank statements (open since round 13)": [
        "ofx-rs", "ofxy", "qif_parser", "ofx-parse", "ofxkit"],
    "subtitles SRT/VTT/ASS (open since round 13)": [
        "subtp", "subparse", "subtitler", "subkit"],
    "PDF table extraction (round 15's strongest)": [
        "pdf-extract", "pdfplumber", "tabula", "pdftable", "pdf-table-rs"],
    "STIX/TAXII threat intelligence (round 16)": [
        "stix", "rstix", "stix-rs", "taxii", "taxii-rs"],
    "readability / main content (round 15 watch)": [
        "dom_smoothie", "readability", "article-rs"],
    "runtime unit conversion (round 17's find)": [
        "uom", "dimensioned", "measurements", "quantities", "quantity",
        "runtime_units", "ucum-units", "gnu-units", "octofhir-ucum",
        "runits", "unitkit"],
    "METAR/TAF meteorology (round 17)": [
        "metar", "metar-taf-parser", "aviation-wx", "rweather-decoder",
        "metar-rs", "grib"],
    "immutable collections (a watch)": ["im", "rpds", "ordermap"],
    "structural data diff (round 18's thin spot)": [
        "json-structural-diff", "diff_json", "serde_json_diff", "json_diff_ng",
        "deepdiff", "json-diff"],
}


# The four new domains. Each entry is (upstream npm/PyPI name, what it is, and
# the crates.io names worth measuring).
NEW_DOMAINS = {
    "Bank and IBAN tooling": {
        "upstream": [("npm", "ibantools"), ("npm", "iban"),
                     ("pypi", "schwifty"), ("pypi", "python-stdnum")],
        "rust": ["iban", "iban_validate", "iban-validator", "rust-iban",
                 "iban_rs", "bic", "bic-iban", "bank-identifiers",
                 "bban", "sepa", "epc-qr", "sepa-qr"],
        "why": "IBAN validation and the BBAN bridge; SEPA credit-transfer "
               "messages and the EPC QR code every EU invoice carries",
    },
    "Product and price codes": {
        "upstream": [("npm", "jsbarcode"), ("npm", "bwip-js"),
                     ("pypi", "python-barcode"), ("pypi", "treepoem")],
        "rust": ["barcode", "barcoders", "rxing", "zxing-cpp", "gtin",
                 "gs1", "ean13", "upc", "barcode-generator", "quircs",
                 "qrcode", "qrcodegen", "rqrr"],
        "why": "GTIN/EAN/UPC check digits and the GS1 application identifiers "
               "behind every product label and price tag",
    },
    "Address parsing": {
        "upstream": [("npm", "parse-address"), ("npm", "addresser"),
                     ("pypi", "usaddress"), ("pypi", "postal")],
        "rust": ["address", "addr", "postal", "address-parser",
                 "address_parser", "postal-address", "usaddress",
                 "libpostal", "rust-libpostal", "postal-codes", "zipcode"],
        "why": "Free-text postal addresses to structured components; the first "
               "step of every checkout, KYC and shipping integration",
    },
    "Phone numbers": {
        "upstream": [("npm", "libphonenumber-js"), ("npm", "google-libphonenumber"),
                     ("pypi", "phonenumbers")],
        "rust": ["phonenumber", "rust-phonenumber", "libphonenumber",
                 "phone", "phonenumbers", "phone-number",
                 "phonenumber-fuzzy", "e164"],
        "why": "E.164 parsing, formatting and type detection; needed by every "
               "signup form and SMS gateway",
    },
}


def candidates():
    for area, names in CANDIDATES.items():
        print(f"### {area}")
        for name in names:
            print(measure(name))
        print()


def sweep():
    for area, spec in NEW_DOMAINS.items():
        print(f"### {area}")
        print(f"    {spec['why']}")
        print("  upstream:")
        for source, name in spec["upstream"]:
            if source == "npm":
                n = npm(name)
                print(f"    npm  {name:26s} last-week={fmt(n) if n else '?'}")
            else:
                p = pypi(name)
                print(f"    pypi {name:26s} last-month={fmt(p) if p else '?'}")
        print("  rust:")
        for name in spec["rust"]:
            print(measure(name))
        print()


def names(argv):
    for name in argv:
        c = crate(name)
        print(f"  {name:26s} {'FREE' if c is None else c['name'] + ' v' + (c['version'] or '?')}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        print(__doc__)
        raise SystemExit(1)
    if sys.argv[1] == "candidates":
        candidates()
    elif sys.argv[1] == "sweep":
        sweep()
    elif sys.argv[1] == "names":
        names(sys.argv[2:])
    else:
        print(__doc__)
        raise SystemExit(1)
