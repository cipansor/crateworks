#!/usr/bin/env python3
"""Round 21: a demand-first sweep of the top PyPI packages.

The other rounds sweep domains. This one starts from the other end: it takes the
ranked PyPI download list, asks crates.io for a crate of the same name, and then
checks the capability behind every name that looked unclaimed. The first step is
a coarse filter (most top PyPI packages are Python-only and have no Rust
counterpart to be missing); the second step is the measurement.

The round-15 rules apply: a keyword search on the upstream package's name is not
a gap check, so search the capability; and sort the nearest crates by release
date, not only by downloads, and ask whether any of them has an audience.

Run it from `research/tools`:

    python3 round21.py                # the ten capability groups
    python3 round21.py toppypi        # the 600-name demand-first cross-check
"""

from __future__ import annotations

import json
import os
import sys
import time

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from round15 import crate, fmt, github, npm, pypi, release_date, reverse_deps  # noqa: E402


def measure(name):
    """One crate's live figures: downloads, release date, revdeps, stars."""
    c = crate(name)
    if c is None:
        return f"  {name:24s} FREE"
    ver, pub = release_date(name)
    rd = reverse_deps(name)
    gh = github(c["repo"])
    star = f"{gh['stars']}*" if gh and gh.get("stars") is not None else "-"
    arch = " ARCHIVED" if gh and gh.get("archived") else ""
    return (
        f"  {name:24s} recent={fmt(c['recent']):>12s} v{ver or '?':9s} "
        f"pub={pub or '?':10s} revdeps={rd if rd is not None else '?':>4} "
        f"stars={star}{arch}"
    )


SWEEP = {
 "colour science / colorimetry (CIELAB, delta-E, palettes)": {
   "rust": ["palette","colors-transform","colorous","color_art","colorgrad","chroma","colour"],
   "pypi": ["colour-science","colormath","colorspacious"],
   "npm": ["chroma-js","color","culori","colorjs.io"]},
 "double-entry accounting / plain-text ledger": {
   "rust": ["beancount","ledger","hledger","double-entry","accounting","bookkeeping"],
   "pypi": ["beancount","ledger","piecash"],
   "npm": ["ledger-cli","accounting","node-ledger"]},
 "spell checking / hunspell dictionaries": {
   "rust": ["hunspell","spellcheck","spellbook","nspell","enchant","symspell"],
   "pypi": ["pyspellchecker","hunspell","symspellpy","spylls"],
   "npm": ["nspell","hunspell-spellchecker","spellchecker","typo-js"]},
 "language detection": {
   "rust": ["whatlang","lingua","langdetect","franc","cld3","language-detection"],
   "pypi": ["langdetect","langid","lingua-language-detector","fasttext"],
   "npm": ["franc","cld","language-detector","langdetect"]},
 "PDF form fields (AcroForm fill/flatten)": {
   "rust": ["pdf-form","pdf_forms","acroform","lopdf","pdfium-render"],
   "pypi": ["pypdf","fillpdf","pdftk"],
   "npm": ["pdf-lib","pdffiller","hummus"]},
 "vCard / iCalendar address book": {
   "rust": ["vcard","vobject","icalendar","vcard4","calcard"],
   "pypi": ["vobject","vcard","icalendar"],
   "npm": ["vcard4","ical.js","vcf"]},
 "currency and money (ISO 4217, exchange, formatting)": {
   "rust": ["money","iso_currency","currency","monetize","rusty-money","dinero"],
   "pypi": ["py-moneyed","money","forex-python","CurrencyConverter"],
   "npm": ["dinero.js","currency.js","money","js-money","currency-symbol-map"]},
 "ISO 8601 durations and interval arithmetic": {
   "rust": ["iso8601-duration","iso8601","duration-str","fundu","humantime"],
   "pypi": ["isodate","durationpy","pytimeparse"],
   "npm": ["iso8601-duration","parse-duration","luxon"]},
 "HTML to text / markdown extraction": {
   "rust": ["html2text","htmd","html2md","fast_html2md","nanohtml2text"],
   "pypi": ["html2text","markdownify","inscriptis"],
   "npm": ["turndown","html-to-text","node-html-markdown"]},
 "college/education: student transcripts, ECTS": {
   "rust": ["transcript","ects","credits"],
   "pypi": ["edx-transcript","ects"],
   "npm": ["transcript"]},
}

def sweep():
    """The ten capability groups this round measured."""
    for domain, spec in SWEEP.items():
        print(f"\n=== {domain}")
        for name in spec["rust"]:
            print(measure(name))
            time.sleep(0.7)
        for eco, names in (("PyPI", spec["pypi"]), ("npm", spec["npm"])):
            for name in names:
                n = pypi(name) if eco == "PyPI" else npm(name)
                unit = "mo" if eco == "PyPI" else "wk"
                print(f"    {eco:4s} {name:24s} {fmt(n)}/{unit}")


TOP_PYPI_URL = (
    "https://raw.githubusercontent.com/hugovk/top-pypi-packages/main/"
    "top-pypi-packages-30-days.min.json"
)


def toppypi(count=600):
    """Demand-first cross-check: top PyPI names against crates.io exact names."""
    import urllib.request

    request = urllib.request.Request(
        TOP_PYPI_URL, headers={"User-Agent": "crateworks-gap-research/1.0"}
    )
    with urllib.request.urlopen(request, timeout=60) as response:
        rows = json.loads(response.read().decode())["rows"]
    names = [row["project"] for row in rows[:count]]
    print(f"checking {len(names)} top PyPI packages against crates.io")
    absent = []
    for name in names:
        if crate(name) is None:
            absent.append(name)
        time.sleep(0.35)
    print(f"\n{len(absent)} of {len(names)} have no crates.io crate of the same name:")
    for name in absent:
        print("   ", name)


def main() -> int:
    if len(sys.argv) < 2:
        print(__doc__)
        return 2
    mode = sys.argv[1]
    if mode == "sweep":
        sweep()
    elif mode == "toppypi":
        toppypi(int(sys.argv[2]) if len(sys.argv) > 2 else 600)
    else:
        print(__doc__)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
