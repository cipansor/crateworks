#!/usr/bin/env python3
"""Regenerate the IBAN BBAN structure table in `crates/stdnum-rs/src/iban.rs`.

The crate used to carry only the expected length of each country's IBAN, so a
number of the right length with letters where the registry requires digits was
accepted. The registry also fixes the *structure* of the BBAN, and this script
transcribes it.

The source is `iban.dat` in python-stdnum, which is generated from the SWIFT
IBAN registry (see the file's own header), so the table's authority is the
registry and python-stdnum is only the transcription this crate is checked
against. The output replaces the block between the BEGIN/END markers.

Run from the repository root:

    python3 research/tools/gen_iban_structures.py
"""

from __future__ import annotations

import os
import re
import sys

BEGIN = "// BEGIN GENERATED IBAN STRUCTURES"
END = "// END GENERATED IBAN STRUCTURES"


def structures() -> list[tuple[str, str]]:
    import stdnum

    root = os.path.dirname(stdnum.__file__)
    rows: list[tuple[str, str]] = []
    for line in open(os.path.join(root, "iban.dat"), encoding="utf-8"):
        if not line.strip() or line.startswith("#"):
            continue
        if len(line) - len(line.lstrip(" ")) != 0:
            continue
        match = re.match(r"([A-Z]{2})\s.*bban=\"([^\"]*)\"", line)
        if match:
            rows.append((match.group(1), match.group(2)))
    return rows


def main() -> int:
    path = "crates/stdnum-rs/src/iban.rs"
    if not os.path.exists(path):
        print(f"run me from the repository root ({path} not found)", file=sys.stderr)
        return 1
    rows = structures()
    body = "\n".join(f'    ("{cc}", "{struct}"),' for cc, struct in rows)
    block = f"{BEGIN}\n{body}\n    {END}"
    source = open(path, encoding="utf-8").read()
    pattern = re.compile(re.escape(BEGIN) + r".*?" + re.escape(END), re.DOTALL)
    if not pattern.search(source):
        print("markers not found in iban.rs", file=sys.stderr)
        return 1
    open(path, "w", encoding="utf-8").write(pattern.sub(lambda _: block, source))
    print(f"wrote {len(rows)} country structures")
    return 0


if __name__ == "__main__":
    sys.exit(main())
