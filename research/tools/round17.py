#!/usr/bin/env python3
"""Round 17: re-measure the open survivors, then sweep seven unswept domains.

Rounds 11-16 all reached the same conclusion from different directions: a gap
written down in a document is not a gap today. MRZ closed between rounds 10 and
11, round 12's YAML headline had three successors by round 13, round 14's top
pick shipped inside this workspace before round 15 ran, and round 16 found that
four of its seven domains were a *race* rather than an opening. So this round
does the same two jobs: measure every survivor again, then sweep domains no
earlier round touched.

Rounds 8-15 searched the npm and PyPI popularity lists and then utility
domains; round 16 swept specialist protocols (healthcare, energy, field buses,
threat intel, supply chain, market data, identity). This round picks seven
**scientific and everyday-data** domains that those lists under-represent but
that whole industries depend on. They are chosen for fit with this workspace,
whose strength is transcribing a real specification and checking the result
against a reference package:

    cheminformatics  SMILES, InChI, molecular fingerprints, substructure search
    bioinformatics   FASTA/FASTQ, SAM/BAM, VCF, sequence alignment
    geospatial       map projections, shapefiles, GeoTIFF, coordinate transforms
    audio/music      ID3 and other tags, MIDI, MusicXML notation
    postal & phone   address parsing, phone-number formatting and validation
    recurrence       cron expressions, RFC 5545 RRULE, iCalendar
    meteorology      METAR/TAF, GRIB, BUFR, station data

Usage:

    python3 round17.py candidates       # re-measure the open survivors
    python3 round17.py sweep            # the seven new domains
    python3 round17.py names <name>...  # is a proposed crate name free?
"""

import sys
import time

# round15.py guards its main block, so importing it is safe and gives the
# measured helpers (crate, release_date, reverse_deps, pypi, npm, fmt).
from round15 import SLEEP, crate, fmt, npm, pypi, release_date, reverse_deps

# Import the shared GitHub helper rather than re-deriving it; round16 imports
# round15 the same way and adds only what it needs.
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


# Every candidate earlier rounds left open, plus the crates this workspace
# shipped from them, so their own numbers stay visible.
CANDIDATES = {
    "OFX/QIF bank statements (open since round 13)": [
        "ofx-rs", "ofxy", "qif_parser", "ofx-parse", "ofxkit"],
    "subtitles SRT/VTT/ASS (open since round 13)": [
        "subtp", "subparse", "subtitler", "subkit"],
    "PDF table extraction (round 15's strongest)": [
        "pdf-extract", "pdfplumber", "tabula", "pdftable", "pdf-table-rs"],
    "STIX/TAXII threat intelligence (round 16's new opening)": [
        "stix", "rstix", "stix-rs", "taxii", "taxii-rs", "cc-taxii2-client-rs"],
    "readability / main content (round 15, a watch)": [
        "dom_smoothie", "readability", "article-rs"],
    "fuzzy matching (shipped here as fuzzy-score)": [
        "fuzzy-matcher", "fuzzy-score"],
    "immutable collections (a watch)": ["im", "rpds", "ordermap"],
}

# Seven scientific and everyday-data domains no earlier round swept. Each group
# is (Rust crates to measure, source packages that show the demand).
SWEEP = {
    "cheminformatics: SMILES, InChI, fingerprints, substructure": {
        "rust": ["chemcore", "purr", "gamma", "chemfiles", "openbabel", "rdkit",
                 "smiles", "inchi", "molecule", "chem-rust"],
        "pypi": ["rdkit", "openbabel", "indigo", "pubchempy", "molvs"],
        "npm": ["smiles-drawer", "openchemlib", "chemical-tools"],
    },
    "bioinformatics: FASTA/FASTQ, SAM/BAM, VCF, alignment": {
        "rust": ["bio", "rust-bio", "noodles", "needletail", "bio-types",
                 "rust-htslib", "bio-seq", "seq_io", "vcf", "bam"],
        "pypi": ["biopython", "pysam", "pyfaidx", "cyvcf2", "scikit-bio"],
        "npm": ["biojs-io-fasta", "bionode", "fasta-parser"],
    },
    "geospatial: projections, shapefiles, GeoTIFF, transforms": {
        "rust": ["proj", "proj4rs", "geo", "geojson", "shapefile", "gdal",
                 "geotiff", "tiff", "raster", "geo-types", "h3o"],
        "pypi": ["pyproj", "shapely", "gdal", "rasterio", "fiona", "geopandas"],
        "npm": ["proj4", "shapefile", "geotiff", "turf"],
    },
    "audio and music: ID3 tags, MIDI, MusicXML notation": {
        "rust": ["id3", "lofty", "audiotags", "symphonia", "midly", "midi",
                 "wmidi", "musicxml", "abc-notation", "music21"],
        "pypi": ["mutagen", "mido", "music21", "eyed3", "audioread"],
        "npm": ["music-metadata", "node-id3", "jsmidgen", "midi-file"],
    },
    "postal addresses and phone numbers": {
        "rust": ["phonenumber", "rust-phonenumber", "postal", "postal-address",
                 "address", "deunicode", "country", "iso3166", "addr"],
        "pypi": ["phonenumbers", "postal", "pypostal", "usaddress", "address"],
        "npm": ["libphonenumber-js", "parse-address", "addresser"],
    },
    "recurrence: cron, RFC 5545 RRULE, iCalendar": {
        "rust": ["cron", "croner", "cron-parser", "saffron", "rrule", "rrule-rs",
                 "icalendar", "ical", "chrono-tz", "cron_clock"],
        "pypi": ["croniter", "python-dateutil", "recurring-ical-events",
                 "icalendar", "dateutil"],
        "npm": ["cron-parser", "rrule", "ical.js", "node-cron", "later"],
    },
    "meteorology: METAR/TAF, GRIB, BUFR, station data": {
        "rust": ["metar", "metar-rs", "grib", "wgrib", "bufr", "weather",
                 "noaa", "grib2", "weather-rs"],
        "pypi": ["metar", "pygrib", "cfgrib", "eccodes", "python-metar"],
        "npm": ["metar", "grib2json", "weacast-grib"],
    },
}


def show_candidates():
    for area, names in CANDIDATES.items():
        print(f"\n### {area}")
        for n in names:
            print(measure(n))
            time.sleep(SLEEP)


def show_sweep():
    for area, groups in SWEEP.items():
        print(f"\n### {area}")
        for n in groups["rust"]:
            print(measure(n))
            time.sleep(SLEEP)
        for n in groups["pypi"]:
            print(f"  [pypi] {n:20s} {fmt(pypi(n))}/month")
            time.sleep(0.3)
        for n in groups["npm"]:
            print(f"  [npm ] {n:20s} {fmt(npm(n))}/week")
            time.sleep(0.3)


def show_names():
    for n in sys.argv[2:]:
        c = crate(n)
        print(f"  {n:26s} {'FREE' if c is None else 'TAKEN recent=' + fmt(c['recent'])}")
        time.sleep(SLEEP)


if __name__ == "__main__":
    cmd = sys.argv[1] if len(sys.argv) > 1 else "candidates"
    if cmd == "candidates":
        show_candidates()
    elif cmd == "sweep":
        show_sweep()
    elif cmd == "names":
        show_names()
    else:
        raise SystemExit(f"unknown command {cmd!r}")
