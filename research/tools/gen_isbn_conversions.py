#!/usr/bin/env python3
"""Regenerate `crates/stdnum-rs/tests/fixtures/isbn_conversions.tsv`.

The validation gate (`gen_stdnum_vectors.py`) checks whether a number is
accepted. It cannot check a conversion, because a conversion has an output and
not just a verdict. This script is the oracle for `Isbn::to_isbn10` and
`Isbn::to_isbn13`: it asks python-stdnum to convert each number and records what
it returns.

Every line is `<kind>\t<input>\t<expected>`, where `kind` is `to_isbn10` or
`to_isbn13`, and `expected` is the reference's output, or `-` when the reference
raises (a `979` ISBN-13 has no ten-digit form, for example).

Inputs come from two places:

* the reference's own doctests, and
* valid ISBNs built by giving a random nine- or twelve-digit payload the check
  character the reference's `ean.calc_check_digit` computes, so the corpus is
  large and every entry is a number the reference accepts.

Run from the repository root:

    python3 research/tools/gen_isbn_conversions.py

It rewrites the fixture in place. Needs `pip install python-stdnum`.
"""

from __future__ import annotations

import random
import sys
from pathlib import Path

import stdnum.ean
import stdnum.isbn

ROOT = Path(__file__).resolve().parent.parent.parent
FIXTURE = ROOT / "crates" / "stdnum-rs" / "tests" / "fixtures" / "isbn_conversions.tsv"


def isbn13_from_payload(payload12: str) -> str:
    """A valid ISBN-13 for a twelve-digit body, using the reference's digit."""
    return payload12 + stdnum.ean.calc_check_digit(payload12)


def isbn10_from_payload(payload9: str) -> str:
    """A valid ISBN-10 for a nine-digit body, using the reference's digit."""
    body = "978" + payload9
    # The ISBN-10 check character is computed by the reference's own helper.
    return payload9 + stdnum.isbn._calc_isbn10_check_digit(payload9)


def collect_inputs() -> list[str]:
    """ISBN-13 and ISBN-10 values to convert, every one valid to the reference."""
    rng = random.Random(20261027)
    values: set[str] = set()
    # The reference's published examples.
    values.update(
        [
            "9780306406157",
            "9780975229804",
            "9783161484100",
            "9791234567896",
            "0306406152",
            "097522980X",
        ]
    )
    # Valid ISBN-13s across both Bookland prefixes.
    for _ in range(3000):
        prefix = rng.choice(["978", "979"])
        payload = prefix + "".join(rng.choice("0123456789") for _ in range(9))
        values.add(isbn13_from_payload(payload))
    # Valid ISBN-10s.
    for _ in range(1500):
        body = "".join(rng.choice("0123456789") for _ in range(9))
        values.add(isbn10_from_payload(body))
    return sorted(v for v in values if stdnum.isbn.is_valid(v))


def convert(kind: str, value: str) -> str:
    """The reference's output for one conversion, or `-` if it raises."""
    try:
        if kind == "to_isbn10":
            return stdnum.isbn.to_isbn10(value)
        return stdnum.isbn.to_isbn13(value)
    except Exception:
        return "-"


def main() -> int:
    try:
        import stdnum  # noqa: F401
    except ImportError:
        print("needs python-stdnum: pip install python-stdnum", file=sys.stderr)
        return 1
    rows: list[str] = []
    inputs = collect_inputs()
    for value in inputs:
        for kind in ("to_isbn10", "to_isbn13"):
            rows.append(f"{kind}\t{value}\t{convert(kind, value)}")
    FIXTURE.write_text("\n".join(rows) + "\n")
    missing = sum(1 for r in rows if r.endswith("\t-"))
    print(f"{len(rows)} conversion rows from {len(inputs)} ISBNs")
    print(f"    {missing} of them the reference refuses (expected `-`)")
    print(f"fixture -> {FIXTURE.relative_to(ROOT)}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
