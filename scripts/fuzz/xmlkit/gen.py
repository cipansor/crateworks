#!/usr/bin/env python3
"""Regenerate `crates/xmlkit/tests/fixtures/wellformed.tsv`.

`xmlkit` is a hand-written XML parser with no third-party dependency to diff
against, so nothing has caught a divergence between it and a real parser. This
script is that oracle: it generates documents from a grammar-directed
vocabulary, asks Python's `xml.parsers.expat` whether each is well-formed, and
records the verdict.

Every line is `<verdict>\t<document>`, where `<verdict>` is `ok` when expat
accepts the document and `err` when it does not, and the document is
backslash-escaped so it stays on one line (`\\n` and `\\t` are a real newline and
tab, `\\\\` is a backslash).

One of the crate's behaviours is deliberate and differs from expat's, so the
generator does not let it into the corpus; it is asserted in
`crates/xmlkit/src/lib.rs` instead, with the reason:

* A bare `&` that begins no entity is a literal (the npm `@xmldom/xmldom`
  behaviour, which this crate's SOAP and ISO 20022 consumers depend on), where
  expat rejects it. `strip_bare_ampersands` normalises those to `&amp;`, which
  both accept and decode to the same `&`, so the rest is compared.

Everything else is expat's, including `<a/><b/>` (a second root is not
well-formed), an unbound prefix, and a comment or PI after the root element.

Run from the repository root:

    python3 scripts/fuzz/xmlkit/gen.py

It rewrites the fixture in place. Needs only the Python standard library.
"""

from __future__ import annotations

import random
import sys
from pathlib import Path
from xml.parsers import expat

ROOT = Path(__file__).resolve().parent.parent.parent.parent
FIXTURE = ROOT / "crates" / "xmlkit" / "tests" / "fixtures" / "wellformed.tsv"

NAMES = ["a", "b", "ns:Doc", "p:a", "Elem_1", "x-y.z"]
ATTR_NAMES = ["x", "y", "id", "lang", "xmlns:p"]
TEXT_WORDS = ["x", "text here", "a b", " ", "\n", "\t", "1", "café", "中文"]
ENTITIES = ["&amp;", "&lt;", "&gt;", "&quot;", "&apos;", "&#65;", "&#x42;"]

# Fragments that are well-formed on their own, and fragments that are a single
# deliberate error. Sequences of these exercise the parser's decision at each
# boundary rather than only in isolation.
GOOD_FRAGMENTS = [
    "<!-- c -->", "<?pi ?>", "<![CDATA[x]]>", "<![CDATA[<raw> & more]]>",
    "<!DOCTYPE a>", " ", "\n", "<?xml-stylesheet href=\"a.xsl\"?>",
]
BAD_FRAGMENTS = [
    "<a", ">", "<", '"', "'", "=", "<1a>", "<a b>", "<a x>", "<a x=1>",
    "</a>", "</b>", "<a></b>", "x</a>", "<!--", "-->", "<!--x", "<?pi",
    "<![CDATA[unterminated", "<!DOCTYPE a><!DOCTYPE a>", "<a/><b/>",
    "&unknown;", "&#;", "&;", "&#xZZ;", "&#x110000;", "<a b='1'c='2'>",
    "<a/", "<a></a></a>", "</>", "<>", "<?xml?>", "<?xml version=\"1.0\"?>",
    "<!-- a -- b -->", "<!--a--->", "<!DOCTYPE >", "<!DOCTYPEa>",
    "<a x=\"1<2\">", "<?pi<>?>", "<?xml-stylesheet?>",
]


def strip_bare_ampersands(text: str) -> str:
    """Rewrite every bare `&` so expat and the crate see the same document.

    The crate keeps an ampersand that begins no entity as a literal `&`, which is
    the lenient npm `@xmldom/xmldom` behaviour its consumers depend on; expat
    rejects it. To compare the two on everything else, each such ampersand is
    rewritten to `&amp;`, and a `;` that belonged to the rejected reference (as
    in `&#xZZ;`) is dropped, so the text both parsers see is identical.

    An `&` that begins one of the five predefined entities or a *well-formed*
    numeric character reference is left alone, so expat's verdict on a malformed
    or out-of-range reference is kept - the crate rejects those too.
    """
    out = []
    i = 0
    while i < len(text):
        if text[i] != "&":
            out.append(text[i])
            i += 1
            continue
        semi = text.find(";", i)
        body = text[i + 1:semi] if semi != -1 else ""
        valid = (
            body in {"amp", "lt", "gt", "quot", "apos"}
            or (body.startswith("#x") and len(body) > 2 and all(c in "0123456789abcdefABCDEF" for c in body[2:]))
            or (body.startswith("#") and len(body) > 1 and body[1:].isdigit())
        )
        if valid:
            out.append(text[i:semi + 1])
            i = semi + 1
        else:
            out.append("&amp;")
            i += 1
    return "".join(out)


def expat_ok(text: str) -> bool:
    parser = expat.ParserCreate()
    try:
        parser.Parse(text, True)
        return True
    except expat.ExpatError:
        return False


def escape(text: str) -> str:
    return text.replace("\\", "\\\\").replace("\n", "\\n").replace("\t", "\\t")


def valid_element(rng: random.Random, depth: int) -> str:
    """A well-formed element, built by construction, nested up to `depth`."""
    name = rng.choice(NAMES)
    attrs = ""
    for attr in rng.sample(ATTR_NAMES, rng.randint(0, 2)):
        value = rng.choice(TEXT_WORDS + ENTITIES)
        attrs += f' {attr}="{value}"'
    if depth <= 0 or rng.random() < 0.4:
        if rng.random() < 0.5:
            return f"<{name}{attrs}/>"
        text = "".join(rng.choice(TEXT_WORDS + ENTITIES) for _ in range(rng.randint(0, 3)))
        return f"<{name}{attrs}>{text}</{name}>"
    body = "".join(
        rng.choice([valid_element(rng, depth - 1), rng.choice(GOOD_FRAGMENTS), rng.choice(TEXT_WORDS + ENTITIES)])
        for _ in range(rng.randint(1, 4))
    )
    return f"<{name}{attrs}>{body}</{name}>"


def valid_document(rng: random.Random) -> str:
    """A whole document: an optional BOM, an optional prolog, one root, Misc."""
    bom = "\ufeff" if rng.random() < 0.15 else ""
    prolog = "".join(rng.choice(["<?xml version=\"1.0\"?>", "<!DOCTYPE a>", "<!-- c -->", " "])
                     for _ in range(rng.randint(0, 2)))
    root = valid_element(rng, rng.randint(0, 3))
    trailing = "".join(rng.choice(GOOD_FRAGMENTS) for _ in range(rng.randint(0, 2)))
    return bom + prolog + root + trailing


def mutate(rng: random.Random, document: str) -> str:
    """Break a valid document in one of a few ways."""
    if not document:
        return rng.choice(BAD_FRAGMENTS)
    kind = rng.randrange(4)
    at = rng.randrange(len(document))
    if kind == 0:  # delete one character
        return document[:at] + document[at + 1:]
    if kind == 1:  # insert a bad fragment
        return document[:at] + rng.choice(BAD_FRAGMENTS) + document[at:]
    if kind == 2:  # append a bad fragment
        return document + rng.choice(BAD_FRAGMENTS)
    # replace a run with a bad fragment
    return document[:at] + rng.choice(BAD_FRAGMENTS) + document[at + 1:]


def main() -> int:
    rng = random.Random(20261027)
    seen: set[str] = set()
    rows: list[tuple[str, str]] = []

    # Half the corpus is a valid document, so acceptance is tested as hard as
    # rejection; the other half is a single mutation of one, which lands on both
    # sides of the line.
    for i in range(8000):
        document = valid_document(rng) if i % 2 == 0 else mutate(rng, valid_document(rng))
        document = strip_bare_ampersands(document)
        if document in seen:
            continue
        seen.add(document)
        rows.append(("ok" if expat_ok(document) else "err", document))

    FIXTURE.parent.mkdir(parents=True, exist_ok=True)
    with FIXTURE.open("w", encoding="utf-8") as handle:
        handle.write("# Generated by scripts/fuzz/xmlkit/gen.py from Python expat - do not edit.\n")
        handle.write("# verdict\t(expat well-formedness; bare & normalised away) document\n")
        for verdict, document in rows:
            handle.write(f"{verdict}\t{escape(document)}\n")

    oks = sum(1 for verdict, _ in rows if verdict == "ok")
    print(f"wrote {len(rows)} rows to {FIXTURE.relative_to(ROOT)} ({oks} well-formed, {len(rows) - oks} rejected)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
