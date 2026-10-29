#!/usr/bin/env python3
"""Round 29: simple, everyday identifiers and text utilities, swept capability-first.

Rounds 8-28 covered the popularity lists, then narrow and regulated domains, then
started from npm. The last three rounds all concluded the same way: the registry
is far more complete than a name-based pass suggests, and the surviving openings
are narrow. This round goes back to the other kind of candidate - the small,
obvious, widely used helper - because those are what a *name* pass is most likely
to report as FREE when a differently named crate already serves the job.

Eight domains, each a thing an ordinary application reaches for:

  * sortable unique identifiers (ULID, KSUID, Snowflake, short UUID)
  * character-level diff and patch (diff-match-patch)
  * natural sort order (alphanumeric and version ordering)
  * colour parsing and conversion (hex/rgb/hsl/named)
  * ISO 8601 duration parsing and formatting
  * URL canonicalisation (not parsing: normalising)
  * user-agent string parsing
  * roman numerals and number spelling in a fixed set of formats

Every figure is fetched live from `crates.io/api/v1`, `index.crates.io`,
`api.npmjs.org` and `pypistats.org`.

Usage:

    python3 round29.py sweep          # the eight domains
    python3 round29.py names <n>...   # is a proposed crate name free?
"""

import sys

from round13 import search
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


def capability(terms, per_page=8):
    """The capability search: what a maintainer might have named the job.

    A name pass reports FREE for a domain already served under a generic name,
    which round 25 measured as four false FREEs in eight. This is run before any
    verdict, and it prints the adoption of the best hit, not just its existence.
    """
    hits = search(terms, per_page=per_page)
    out = []
    for c in hits[:5]:
        out.append(
            f"    {c['name']:26s} recent={fmt(c.get('recent_downloads')):>12s} "
            f"v{c.get('max_stable_version') or c.get('newest_version') or '?'}"
        )
    return "\n".join(out) if out else "    (no hits)"


SWEEP = {
    "Sortable unique identifiers (ULID, KSUID, Snowflake, short UUID)": {
        "rust": ["ulid", "ulid-rs", "ksuid", "svix-ksuid", "snowflake", "sonyflake",
                 "nanoid", "cuid", "cuid2", "short-uuid", "uuid7", "typeid"],
        "search": ["ulid", "ksuid", "sortable id", "snowflake id", "nanoid",
                   "short uuid", "lexicographic uuid"],
        "pypi": ["ulid-py", "python-ulid", "ksuid", "nanoid", "shortuuid",
                 "cuid2", "snowflake-id"],
        "npm": ["ulid", "ksuid", "nanoid", "shortid", "cuid", "cuid2", "uuid",
                "ulidx"],
    },
    "Character-level diff and patch (diff-match-patch)": {
        "rust": ["diff-match-patch", "dmp", "dissimilar", "similar", "diffy",
                 "imara-diff", "similar-asserts", "diff"],
        "search": ["diff match patch", "text diff", "diff patch", "fuzzy patch"],
        "pypi": ["diff-match-patch", "difflib"],
        "npm": ["diff-match-patch", "fast-diff", "diff", "jsdiff"],
    },
    "Natural sort order (alphanumeric and version ordering)": {
        "rust": ["natord", "human-sort", "alphanumeric-sort", "lexical-sort",
                 "natural-sort", "version-compare", "version-sort"],
        "search": ["natural sort", "alphanumeric sort", "version compare",
                   "human sort"],
        "pypi": ["natsort", "naturalsort", "versions"],
        "npm": ["natural-sort", "string-natural-compare", "alphanum-sort",
                "natural-compare", "compare-versions"],
    },
    "Colour parsing and conversion (hex/rgb/hsl/named)": {
        "rust": ["csscolorparser", "palette", "colorsys", "colorous", "colorgrad",
                 "color-name", "colorconv"],
        "search": ["css color", "color parse", "color convert", "hsl", "named color"],
        "pypi": ["colour", "webcolors", "colormath", "colorir"],
        "npm": ["color", "chroma-js", "tinycolor2", "culori", "color-convert"],
    },
    "ISO 8601 duration parsing and formatting": {
        "rust": ["iso8601-duration", "iso8601", "duration-str", "humantime",
                 "speedate", "duration-parse"],
        "search": ["iso8601 duration", "duration parse", "parse duration",
                   "timespan"],
        "pypi": ["isodate", "durations", "pytimeparse", "humanfriendly"],
        "npm": ["iso8601-duration", "parse-duration", "duration-fns", "tinyduration"],
    },
    "URL canonicalisation (normalising, not parsing)": {
        "rust": ["url-normalizer", "url-normalize", "normalize-url", "clean-url",
                 "urlnorm", "url"],
        "search": ["url normalize", "url canonical", "url clean", "url sanitize"],
        "pypi": ["urlnorm", "w3lib", "url-normalize", "url-normalizer"],
        "npm": ["normalize-url", "url-normalize", "canonical-url"],
    },
    "User-agent string parsing": {
        "rust": ["woothee", "uaparser", "user-agent", "user_agent_parser",
                 "uaparser-rs", "ua-parser", "woothee-rs"],
        "search": ["user agent parse", "ua parser", "browser detect", "device detect"],
        "pypi": ["ua-parser", "user-agents", "httpagentparser", "device-detector"],
        "npm": ["ua-parser-js", "useragent", "bowser", "platform"],
    },
    "Roman numerals and fixed-format number spelling": {
        "rust": ["roman", "roman-numerals", "num2words", "numbers-to-words",
                 "to-words", "spell-number"],
        "search": ["roman numeral", "number to words", "spell number", "ordinal"],
        "pypi": ["roman", "num2words", "inflect", "numerizer"],
        "npm": ["roman-numerals", "number-to-words", "to-words", "written-number"],
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
        print("  capability search (top 5 by recent downloads):")
        print(capability(groups["search"]))
        print("  by name:")
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
    elif cmd == "search":
        print(capability(sys.argv[2:]))
    else:
        print(__doc__)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
