#!/usr/bin/env python3
"""Generate the three letter tables in `crates/arabic/src/forms.rs`.

The `arabic-reshaper` library ships three letter tables - `Arabic` (the
default), `ArabicV2` and `Kurdish` - selected by its `language` setting. This
crate ported only the default one, so a caller who needs the other two had
nowhere to go. This script reads all three from the reference and emits the Rust
arrays, so the tables are taken from the reference rather than retyped.

`ArabicV2` and `Kurdish` leave most letters in their *unshaped* form when they
stand alone, which is what makes them render in fonts that lack the presentation
forms; only a handful of letters keep a presentation form. Deriving that by hand
is exactly the kind of transcription that goes wrong silently, so it is not
derived here either: the three tables are emitted verbatim.

Run:  python3 research/tools/gen_arabic_tables.py [--write]
Needs: pip install arabic-reshaper

The emitted arrays are wrapped differently from `rustfmt`'s style, so run
`cargo fmt --all` after `--write`; the file then regenerates byte-for-byte.
"""

import argparse
import pathlib
import sys

import arabic_reshaper
from arabic_reshaper import letters

ROOT = pathlib.Path(__file__).resolve().parents[2]
FORMS = ROOT / "crates" / "arabic" / "src" / "forms.rs"

# The order the arrays are declared in, and the names the Rust code uses. The
# first is the crate's existing default table, so it must stay first.
TABLES = [
    ("LETTERS_ARABIC", letters.LETTERS_ARABIC),
    ("LETTERS_ARABIC_V2", letters.LETTERS_ARABIC_V2),
    ("LETTERS_KURDISH", letters.LETTERS_KURDISH),
]

BEGIN = "// BEGIN GENERATED TABLES"
END = "// END GENERATED TABLES"


def escape(c: str) -> str:
    """A Rust `char` literal for one code point."""
    return f"'\\u{{{ord(c):x}}}'"


def shapes(value: tuple) -> str:
    """The `[Option<char>; 4]` array for one letter, in isolated/initial/medial/final order."""
    parts = [f"Some({escape(f)})" if f else "None" for f in value]
    return "[" + ", ".join(parts) + "]"


def emit(name: str, table: dict) -> str:
    # Sorted by code point, which is what the crate's binary search needs.
    entries = sorted(table.items(), key=lambda kv: ord(kv[0]))
    lines = [f"const {name}: [(char, Shapes); {len(entries)}] = ["]
    for letter, value in entries:
        lines.append(f"    ({escape(letter)}, {shapes(value)}),")
    lines.append("];")
    return "\n".join(lines)


def block() -> str:
    parts = [BEGIN]
    for name, table in TABLES:
        parts.append(emit(name, table))
    parts.append(END)
    return "\n".join(parts)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--write",
        action="store_true",
        help="rewrite the tables in forms.rs; without it, print them",
    )
    args = parser.parse_args()

    generated = block()
    if not args.write:
        print(generated)
        return 0

    text = FORMS.read_text()
    if BEGIN not in text or END not in text:
        print(
            f"forms.rs has no {BEGIN!r} / {END!r} markers; add them first",
            file=sys.stderr,
        )
        return 1
    head = text[: text.index(BEGIN)]
    tail = text[text.index(END) + len(END) :]
    FORMS.write_text(head + generated + tail)
    print(
        f"wrote {sum(len(t) for _, t in TABLES)} rows across "
        f"{len(TABLES)} tables to {FORMS.relative_to(ROOT)}"
    )
    print(f"reference: arabic-reshaper {arabic_reshaper.__version__}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
