#!/usr/bin/env python3
"""Round 18: re-measure the open survivors, then sweep seven document and
text-processing domains.

Rounds 13-17 all reached the same conclusion from different directions: a gap
written down in a document is not a gap today. So this round repeats the two
jobs: measure every survivor again, then sweep domains no earlier round touched.

Rounds 8-15 searched the npm and PyPI popularity lists and then utility domains;
round 16 swept specialist protocols; round 17 swept scientific and everyday-data
domains. This round picks seven **document, bibliography and text-processing**
domains, which those lists under-represent but which every publishing, academic
and localisation pipeline depends on:

    bibliography     BibTeX, RIS, CSL citation styles
    office documents DOCX, ODT, RTF read and write
    EPUB             ebook packaging and reading
    typesetting      LaTeX math to MathML
    diff and patch   Myers diff, diff-match-patch, patch application
    segmentation     CJK word segmentation, sentence splitting
    emoji & IDNA     emoji data, grapheme clusters, punycode

Usage:

    python3 round18.py candidates       # re-measure the open survivors
    python3 round18.py sweep            # the seven new domains
    python3 round18.py names <name>...  # is a proposed crate name free?
"""

import sys
import time

# round17 imports round15 and round16 the same way; importing is safe because
# every one of them guards its main block.
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
        "stix", "rstix", "stix-rs", "taxii", "taxii-rs"],
    "readability / main content (round 15, a watch)": [
        "dom_smoothie", "readability", "article-rs"],
    "runtime unit conversion (round 17's find)": [
        "uom", "dimensioned", "measurements", "quantities", "quantity",
        "runtime_units", "ucum-units", "gnu-units", "octofhir-ucum",
        "runits", "unitkit"],
    "METAR/TAF meteorology (round 17's small opening)": [
        "metar", "metar-taf-parser", "aviation-wx", "rweather-decoder",
        "metar-rs", "grib"],
    "immutable collections (a watch)": ["im", "rpds", "ordermap"],
}

# Seven document and text-processing domains no earlier round swept. Each group
# is (Rust crates to measure, source packages that show the demand).
SWEEP = {
    "bibliography: BibTeX, RIS, CSL citation styles": {
        "rust": ["biblatex", "bibtex", "ris", "hayagriva", "citeproc",
                 "citationberg", "csl", "pandoc", "biblib"],
        "pypi": ["citeproc-py", "pybtex", "bibtexparser", "rispy", "pandocfilters"],
        "npm": ["citeproc", "bibtex-parse-js", "citation-js"],
    },
    "office documents: DOCX, ODT, RTF read and write": {
        "rust": ["docx-rs", "docx", "rdocx", "office", "odt", "rtf-parser",
                 "rtf", "lopdf", "zip", "docx-rs-template"],
        "pypi": ["python-docx", "odfpy", "striprtf", "docxtpl", "openpyxl"],
        "npm": ["docx", "mammoth", "officegen", "rtf-parser"],
    },
    "EPUB: ebook packaging and reading": {
        "rust": ["epub", "epub-builder", "epub-rs", "epub_parser", "rbook"],
        "pypi": ["ebooklib", "epubcheck", "kindleunpack", "pypandoc"],
        "npm": ["epub-gen", "epubjs", "epub2", "epub-gen-memory"],
    },
    "typesetting: LaTeX math to MathML": {
        "rust": ["latex", "texmath", "latex2mathml", "mathml", "tex2mathml",
                 "ratex-parser", "katex", "mathjax"],
        "pypi": ["latex2mathml", "pylatexenc", "tex2mathml", "matplotlib"],
        "npm": ["katex", "mathjax", "mathjax-node", "temml"],
    },
    "diff and patch: Myers diff, diff-match-patch, patch apply": {
        "rust": ["similar", "diff", "dissimilar", "diffy", "diff-match-patch",
                 "dmp", "patch", "diffs"],
        "pypi": ["diff-match-patch", "jsondiff", "deepdiff", "patch-ng",
                 "difflib"],
        "npm": ["diff", "diff-match-patch", "jsondiffpatch", "fast-diff"],
    },
    "segmentation: CJK word segmentation, sentence splitting": {
        "rust": ["jieba-rs", "jieba", "lindera", "tantivy-tokenizer",
                 "vibrato", "sentence", "unicode-segmentation", "tinysegmenter"],
        "pypi": ["jieba", "pkuseg", "sentencepiece", "nltk", "blingfire",
                 "syntok"],
        "npm": ["nodejieba", "segmentit", "sbd", "sentence-splitter"],
    },
    "emoji and IDNA: emoji data, grapheme clusters, punycode": {
        "rust": ["emoji", "emojis", "unicode-emoji", "unicode-segmentation",
                 "idna", "punycode", "grapheme", "unicode-width", "icu"],
        "pypi": ["emoji", "idna", "grapheme", "unicodedata2", "emoji-data"],
        "npm": ["emoji-regex", "node-emoji", "punycode", "grapheme-splitter"],
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
        for kind in ("pypi", "npm"):
            for n in groups[kind]:
                d = pypi(n) if kind == "pypi" else npm(n)
                unit = "per month" if kind == "pypi" else "per week"
                print(f"    {kind:4s} {n:26s} {fmt(d):>14s} {unit}")
                time.sleep(SLEEP)


def show_names():
    for n in sys.argv[2:]:
        c = crate(n)
        print(f"  {n:26s} {'FREE' if c is None else 'TAKEN recent=' + fmt(c['recent'])}")
        time.sleep(SLEEP)


if __name__ == "__main__":
    command = sys.argv[1] if len(sys.argv) > 1 else "candidates"
    if command == "candidates":
        show_candidates()
    elif command == "sweep":
        show_sweep()
    elif command == "names":
        show_names()
    else:
        print(__doc__)
