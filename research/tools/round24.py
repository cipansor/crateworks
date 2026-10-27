#!/usr/bin/env python3
"""Round 24: eight everyday-application domains no earlier round swept.

Rounds 8-23 covered, in order: the PyPI/npm top lists, human formatting,
natural-language dates, typography, text repair, MIME sniffing, web feeds, geo
hashing, healthcare, energy, field buses, threat intelligence, supply chain,
market data, identity, cheminformatics, bioinformatics, geospatial, audio,
addresses, phones, recurrence, meteorology, bibliography, office documents,
EPUB, typesetting, diff, segmentation, emoji, banking, product codes,
quantitative finance, payment cards, aviation, clinical terminology, JSON query
languages, CAD, astronomy, video containers, and the npm-vs-Rust premise itself.

This round takes eight domains a normal business application meets that those
sweeps still left: email address validation, password strength estimation,
one-time passwords, semantic-version ranges, JWT/JWK handling, audio tag
reading, natural-language date parsing, and CLDR plural/ordinal rules.

The rule every round since 15 applies: sort the nearest crates by *release
date*, not only by downloads, and ask whether the incumbent has an audience. A
cluster of unadopted crates is a race, not a gap. Search the capability, not
the upstream package's name.

Every figure is fetched live from `crates.io/api/v1`, `index.crates.io`,
`api.npmjs.org`, `pypistats.org` and `api.github.com`.

Usage:

    python3 round24.py sweep          # the eight new domains
    python3 round24.py names <n>...   # is a proposed crate name free?
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


SWEEP = {
    "email address validation and normalisation": {
        "rust": ["email_address", "email-address-parser", "email_address_parser",
                 "mailparse", "email-validator", "validator", "fast_chemail"],
        "pypi": ["email-validator", "email_validator", "dnspython"],
        "npm": ["email-validator", "validator", "isemail"],
    },
    "password strength estimation (zxcvbn)": {
        "rust": ["zxcvbn", "passwords", "password-strength", "pwstrength",
                 "strength", "zxcvbn-rs"],
        "pypi": ["zxcvbn", "password-strength"],
        "npm": ["zxcvbn", "zxcvbn-ts", "@zxcvbn-ts/core", "owasp-password-strength-test"],
    },
    "one-time passwords (TOTP/HOTP) and their URIs": {
        "rust": ["totp-rs", "totp", "otpauth", "hotp", "oathtool", "totp-lite",
                 "google-authenticator"],
        "pypi": ["pyotp", "onetimepass"],
        "npm": ["otplib", "speakeasy", "otpauth"],
    },
    "semantic version ranges and resolution": {
        "rust": ["semver", "node-semver", "semver-spec", "versions",
                 "semver-range", "pubgrub"],
        "pypi": ["semantic-version", "packaging", "semver"],
        "npm": ["semver", "compare-versions"],
    },
    "JWT, JWK and JOSE token handling": {
        "rust": ["jsonwebtoken", "josekit", "jwt", "biscuit", "jose", "jwt-simple"],
        "pypi": ["PyJWT", "python-jose", "authlib", "joserfc"],
        "npm": ["jsonwebtoken", "jose", "jws", "jwks-rsa"],
    },
    "audio tag and metadata reading": {
        "rust": ["id3", "id3v2", "lofty", "metaflac", "audiotags", "mp3-metadata",
                 "taglib", "symphonia"],
        "pypi": ["mutagen", "eyed3", "tinytag"],
        "npm": ["music-metadata", "node-id3", "jsmediatags"],
    },
    "natural-language date parsing": {
        "rust": ["dateparser", "chrono-english", "parse_datetime", "humantime",
                 "dtparse", "friendly-date", "whenever"],
        "pypi": ["dateparser", "dateutil", "parsedatetime"],
        "npm": ["chrono-node", "date-fns", "dayjs", "moment"],
    },
    "CLDR plural and ordinal rules": {
        "rust": ["intl_pluralrules", "fluent", "pluralrules", "icu_plurals",
                 "unic-langid", "icu", "intl-pluralrules"],
        "pypi": ["babel", "pluralrules", "pyicu"],
        "npm": ["make-plural", "intl-pluralrules", "cldr-core", "@formatjs/intl-pluralrules"],
    },
}


def sweep():
    for domain, sources in SWEEP.items():
        print(f"\n## {domain}")
        print("  -- Rust crates --")
        for name in sources["rust"]:
            print(measure(name))
        print("  -- demand (pypi / npm) --")
        for name in sources["pypi"]:
            d = pypi(name)
            print(f"  pypi {name:30s} {fmt(d) if d else '?'} /month")
        for name in sources["npm"]:
            d = npm(name)
            print(f"  npm  {name:30s} {fmt(d) if d else '?'} /week")


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "names":
        from round15 import namecheck

        namecheck(sys.argv[2:])
    else:
        sweep()
