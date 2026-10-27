#!/usr/bin/env python3
"""Round 25: eight domains rounds 8-24 did not sweep.

Rounds 8-24 covered the popularity lists, human formatting, dates, typography,
text repair, MIME, feeds, geo hashing, healthcare, energy, field buses, threat
intelligence, supply chain, market data, identity, cheminformatics,
bioinformatics, geospatial, audio, addresses, phones, recurrence, meteorology,
bibliography, office documents, EPUB, typesetting, diff, segmentation, emoji,
banking, product codes, quantitative finance, payment cards, aviation, clinical
terminology, JSON query languages, CAD, astronomy, video containers, email
validation, password strength, one-time passwords, semver, JWT/JWK, audio tags,
natural-language dates and CLDR plurals.

This round takes eight further domains a business application meets, chosen for
being *narrow, regulated or region-specific* - the shape rounds 22-24 concluded
is where the hit rate is higher than in the general-purpose space:

  * label and receipt printer command languages (ZPL, EPL, TSPL, ESC/POS)
  * mainframe and EBCDIC data (COBOL copybooks, packed decimal, JCL)
  * maritime and radio telemetry (AIS, ADS-B, APRS, AX.25)
  * PostScript and PCL page description output
  * vCard and vCalendar interchange
  * RADIUS and TACACS+ authentication accounting
  * Kerberos and GSSAPI
  * NTP and PTP time synchronisation

The rule every round since 15 applies: sort the nearest crates by *release
date*, not only downloads, and ask whether the incumbent has an audience. Search
the capability, not the upstream package's name.

Every figure is fetched live from `crates.io/api/v1`, `index.crates.io`,
`api.npmjs.org`, `pypistats.org` and `api.github.com`.

Usage:

    python3 round25.py sweep          # the eight new domains
    python3 round25.py names <n>...   # is a proposed crate name free?
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
    "label and receipt printer command languages (ZPL/EPL/TSPL/ESC-POS)": {
        "rust": ["zpl", "zpl-rs", "epl", "tspl", "escpos", "esc-pos", "thermal",
                 "label-printer", "lpr", "ipp", "zebra"],
        "pypi": ["zpl", "zebra", "escpos", "python-escpos", "zpl2"],
        "npm": ["zpl", "zpl-image", "node-thermal-printer", "escpos",
                "receipt-printer", "@node-escpos/core"],
    },
    "mainframe and EBCDIC data (COBOL copybook, packed decimal, JCL)": {
        "rust": ["ebcdic", "cobol", "copybook", "cobol-copybook", "packed-decimal",
                 "mainframe", "jcl", "zos"],
        "pypi": ["cobol-parser", "ebcdic", "pycobol", "zopfli"],
        "npm": ["cobol", "copybook", "ebcdic", "node-cobol"],
    },
    "maritime and radio telemetry (AIS, ADS-B, APRS, AX.25)": {
        "rust": ["ais", "adsb", "ads-b", "aprs", "ax25", "libais", "nmea",
                 "gpsd", "dump1090"],
        "pypi": ["pyais", "libais", "pyModeS", "aprs", "ax25"],
        "npm": ["ais-stream", "adsb", "mode-s", "aprs-parser"],
    },
    "PostScript and PCL page description output": {
        "rust": ["postscript", "pcl", "ps", "ghostscript", "pdf-writer",
                 "page-description"],
        "pypi": ["pypostscript", "ghostscript", "pcl", "reportlab"],
        "npm": ["postscript", "ghostscript", "node-pcl"],
    },
    "vCard and vCalendar interchange": {
        "rust": ["vcard", "vcard-rs", "icalendar", "ical", "vobject", "carddav",
                 "caldav"],
        "pypi": ["vobject", "vcard", "ics", "caldav", "icalendar"],
        "npm": ["vcard", "vcf", "ical", "ics", "node-ical", "ical.js"],
    },
    "RADIUS and TACACS+ authentication accounting": {
        "rust": ["radius", "radius-rs", "radclient", "tacacs", "tacacs-plus",
                 "freeradius", "diameter"],
        "pypi": ["pyrad", "radius", "tacacs_plus"],
        "npm": ["radius", "node-radius", "tacacs"],
    },
    "Kerberos and GSSAPI": {
        "rust": ["kerberos", "krb5", "gssapi", "krb5-src", "sasl", "gsasl",
                 "spnego", "ntlm"],
        "pypi": ["pykerberos", "gssapi", "requests-kerberos", "pyspnego",
                 "winkerberos"],
        "npm": ["kerberos", "node-gssapi", "gssapi.js"],
    },
    "NTP and PTP time synchronisation": {
        "rust": ["ntp", "ntp-proto", "sntp", "ptp", "nts", "chrony",
                 "ntpd-rs", "ntpsec"],
        "pypi": ["ntplib", "ntpsec", "python-ntp", "ntp"],
        "npm": ["ntp-client", "ntp", "sntp", "node-ntp"],
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
