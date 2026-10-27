#!/usr/bin/env python3
"""Round 27: eight everyday developer domains, swept capability-first.

Rounds 8-26 covered the popularity lists, human formatting, dates, typography,
text repair, MIME, feeds, geo hashing, healthcare, energy, field buses, threat
intelligence, supply chain, market data, identity, cheminformatics,
bioinformatics, geospatial, audio, addresses, phones, recurrence, meteorology,
bibliography, office documents, EPUB, typesetting, diff, segmentation, emoji,
banking, product codes, quantitative finance, payment cards, aviation, clinical
terminology, JSON query languages, CAD, astronomy, video containers, email,
password strength, one-time passwords, semver, JWT/JWK, audio tags,
natural-language dates, CLDR plurals, label printers, mainframe data, maritime
telemetry, PostScript, vCard, RADIUS, Kerberos, NTP, Indonesian identifiers,
marine encoding, chess notation, geotechnical units, museum interchange,
pharmacy dosing, flight planning and legal citation.

Round 26 closed on the rule that a *name-based* pass is not evidence: four of
eight domains it looked at read as FREE by name and were served once the
capability was searched. This round starts from capabilities a developer meets
in ordinary application code - a geocode to store, a number to spell, a name to
split, a page to clean - and searches each one before any verdict.

Eight domains:

  * Open Location Code / Plus Codes (short, recoverable geocodes)
  * multilingual number-to-words (cardinal, ordinal, currency)
  * human name parsing (given/family, prefix, suffix)
  * extractive summarization and keyword extraction
  * human-readable formatting (durations, byte sizes, ordinals)
  * English inflection and pluralization
  * perceptual image hashing (aHash/dHash/pHash)
  * boilerplate removal and readability extraction

Every figure is fetched live from `crates.io/api/v1`, `index.crates.io`,
`api.npmjs.org`, `pypistats.org` and `api.github.com`.

Usage:

    python3 round27.py sweep          # the eight domains
    python3 round27.py names <n>...   # is a proposed crate name free?
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
    "Open Location Code / Plus Codes (short, recoverable geocodes)": {
        "rust": ["open-location-code", "pluscodes", "olc", "openlocationcode",
                 "geohash", "maidenhead", "mgrs", "geoconvert"],
        "pypi": ["openlocationcode", "pluscodes"],
        "npm": ["open-location-code", "pluscodes", "openlocationcode"],
    },
    "Multilingual number-to-words (cardinal, ordinal, currency)": {
        "rust": ["num2words", "number-to-words", "text2num", "num2words2-core",
                 "italian_numbers", "num2words2"],
        "pypi": ["num2words", "inflect", "num2words2"],
        "npm": ["number-to-words", "to-words", "written-number", "num2words"],
    },
    "Human name parsing (given/family, prefix, suffix)": {
        "rust": ["human_name", "name-parser", "name_parser", "probablepeople",
                 "human-name", "person-name"],
        "pypi": ["nameparser", "probablepeople", "usaddress", "name-parser"],
        "npm": ["parse-full-name", "humanparser", "name-parser", "parse-name"],
    },
    "Extractive summarization and keyword extraction": {
        "rust": ["keyword_extraction", "rake", "yake-rust", "sumy",
                 "textrank", "keyphrases", "lexrank"],
        "pypi": ["sumy", "yake", "rake-nltk", "keybert", "textstat"],
        "npm": ["node-summarizer", "textrank", "yake", "keyword-extractor"],
    },
    "Human-readable formatting (durations, byte sizes, ordinals)": {
        "rust": ["human_format", "human_bytes", "duration-str", "num-format",
                 "thousands", "human-repr", "size_format"],
        "pypi": ["humanize", "humanfriendly", "quantiphy", "prettytable"],
        "npm": ["humanize-duration", "filesize", "pretty-bytes",
                "javascript-time-ago"],
    },
    "English inflection and pluralization": {
        "rust": ["pluralizer", "Inflector", "str_inflector", "inflector"],
        "pypi": ["inflection", "pluralizer"],
        "npm": ["pluralize", "inflection"],
    },
    "Perceptual image hashing (aHash/dHash/pHash)": {
        "rust": ["image_hasher", "img_hash", "fast-dhash", "imagehash",
                 "imgfprint"],
        "pypi": ["imagehash", "ImageHash"],
        "npm": ["image-hash", "blockhash-core", "imghash"],
    },
    "Boilerplate removal and readability extraction": {
        "rust": ["dom_smoothie", "readabilityrs", "readability-rust", "legible",
                 "readability", "article-extractor"],
        "pypi": ["trafilatura", "readability-lxml", "newspaper3k",
                 "boilerpy3", "python-readability"],
        "npm": ["@mozilla/readability", "@postlight/parser", "readability",
                "article-extractor"],
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
