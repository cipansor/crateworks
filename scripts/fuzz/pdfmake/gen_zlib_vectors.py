#!/usr/bin/env python3
"""Rebuilds the `pdfmake` zlib vectors from Python's `zlib`.

The `pdfmake` encoder writes fixed-Huffman DEFLATE blocks, and the test that
decodes its output carries its own inflater so the crate keeps no dependency
beyond the standard library. An inflater checked only against the encoder it
is meant to verify proves nothing, so it is checked against Python's `zlib`,
which is the reference and never saw this code.

The streams are produced with `strategy=Z_FIXED`, which is the same shape the
crate writes (stored and fixed-Huffman blocks, never dynamic), and one
incompressible input is included so a stored block is covered too.

Usage:
    python3 scripts/fuzz/pdfmake/gen_zlib_vectors.py [out]
"""

import pathlib
import sys
import zlib

ROOT = pathlib.Path(__file__).resolve().parents[3]
DEFAULT_OUT = ROOT / "crates" / "pdfmake" / "tests" / "fixtures" / "zlib_vectors.txt"

# Inputs chosen to exercise the block shapes: empty, a short literal, a long
# run (matches), a repeating line (the real content shape) and incompressible
# bytes (a stored block).
INPUTS = [
    b"",
    b"hello",
    b"a" * 1000,
    b"line number 0 with some words\n" * 200,
    bytes(range(256)) * 3,
    bytes((i * 37 + 11) % 256 for i in range(5000)),
]


def main() -> int:
    out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    lines = []
    for data in INPUTS:
        compressor = zlib.compressobj(level=9, strategy=zlib.Z_FIXED)
        packed = compressor.compress(data) + compressor.flush()
        assert zlib.decompress(packed) == data, "the reference round-trip failed"
        lines.append(f"{packed.hex()}\t{data.hex()}")
    # Level 0 makes zlib emit stored blocks, the third block shape a reader must
    # handle and the one `Z_FIXED` never produces.
    stored = bytes(range(256)) * 2
    stored_packed = zlib.compress(stored, level=0)
    assert zlib.decompress(stored_packed) == stored, "the stored round-trip failed"
    lines.append(f"{stored_packed.hex()}\t{stored.hex()}")
    out.write_text("\n".join(lines) + "\n")
    print(f"{out} -> {len(lines)} vectors")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
