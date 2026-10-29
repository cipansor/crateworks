#!/usr/bin/env python3
"""Rebuilds the `pdfa` reference fixtures from the `factur-x` Python package.

The `pdfa` crate checks itself against a Factur-X container the reference
produced: the PDF, the invoice it carries, and the XMP metadata packet. Those
were committed without a generator, which means they could not be rebuilt or
extended, and the AGENTS.md rule is that a fixture without its generator is not
finished. This is the generator.

It takes two inputs that stay committed and unchanged - `blank.pdf`, a minimal
PDF the reference can extend, and `reference_embedded.xml`, the EN 16931 invoice
to carry - and writes two outputs:

    reference_facturx.pdf   the Factur-X container the reference built
    reference_xmp.xml       its XMP packet, extracted from the metadata stream

The invoice is embedded, so `reference_embedded.xml` must equal what the
reference reads back out; the generator checks that and refuses to write if it
does not, because a fixture that disagrees with itself proves nothing.

The reference stamps the PDF and the XMP with the wall clock, which would make
the fixture change on every run. The two timestamp functions are pinned to a
fixed instant so the output is reproducible, which is the point of a committed
fixture.

Usage:
    pip install factur-x pypdf
    python3 scripts/fuzz/pdfa/gen.py [fixture-dir]

The fixture directory defaults to `crates/pdfa/tests/fixtures`.
"""

import io
import pathlib
import sys

from pypdf import PdfReader

import facturx.facturx as reference
from facturx import generate_from_binary, get_facturx_xml_from_pdf

ROOT = pathlib.Path(__file__).resolve().parents[3]
DEFAULT_DIR = ROOT / "crates" / "pdfa" / "tests" / "fixtures"

# The instant the committed fixture was built at, so a rebuild reproduces it.
FIXED_PDF_DATE = "D:20261021144753+00'00'"
FIXED_META_DATE = "2026-10-21T14:47:53+00:00"

FLAVOR = "factur-x"
LEVEL = "en16931"


def main() -> int:
    fixture_dir = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_DIR
    blank = (fixture_dir / "blank.pdf").read_bytes()
    invoice = (fixture_dir / "reference_embedded.xml").read_bytes()

    reference._get_pdf_timestamp = lambda date=None: FIXED_PDF_DATE
    reference._get_metadata_timestamp = lambda: FIXED_META_DATE

    pdf = generate_from_binary(
        blank,
        invoice,
        flavor=FLAVOR,
        level=LEVEL,
        check_xsd=False,
        check_schematron=False,
    )

    name, embedded = get_facturx_xml_from_pdf(pdf, check_xsd=False)
    if name != "factur-x.xml":
        raise SystemExit(f"the attachment is named {name!r}, expected factur-x.xml")
    if embedded != invoice:
        raise SystemExit("the embedded invoice is not the one that was put in")

    reader = PdfReader(io.BytesIO(pdf))
    xmp = reader.xmp_metadata
    if xmp is None or xmp.stream is None:
        raise SystemExit("the reference wrote no XMP packet")
    packet = xmp.stream.get_data()

    (fixture_dir / "reference_facturx.pdf").write_bytes(pdf)
    (fixture_dir / "reference_xmp.xml").write_bytes(packet)
    print(f"reference_facturx.pdf -> {len(pdf)} bytes")
    print(f"reference_xmp.xml -> {len(packet)} bytes")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
