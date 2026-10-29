#!/usr/bin/env python3
"""Builds the `openlocationcode` corpus from the Python reference.

The crate already replays the reference's own four CSV files in
`tests/reference.rs`; those are the cases its authors wrote down. This generator
adds the cases nobody wrote down: random strings over the alphabet, the
separator, padding and junk; valid codes built from random coordinates and then
mutated; and random coordinates for the encode path. Each answer is recorded as
the reference gives it, so the corpus is an oracle rather than a sample.

The result is `crates/openlocationcode/tests/fixtures/fuzz.tsv`, one row per
case, tab-separated, keyed by a leading kind:

    V <code> <isValid> <isShort> <isFull>
    D <code> <latLow> <lngLow> <latHigh> <lngHigh> <codeLength>
    S <code> <lat> <lng> <shortened>
    R <short> <lat> <lng> <recovered>
    E <lat> <lng> <length> <code>

The validity, decode, shorten and recover rows are exact and the Rust test
compares them exactly (decode to 1e-9, as the reference's own test does to
1e-10). The `E` rows are the exception the reference itself documents: `encode`
converts degrees to integers, and that conversion can land one step out, so the
reference accepts a small mismatch rate for it. `tests/fuzz.rs` allows the same
5% for `E` and no error for the rest; the exact integer path is covered by the
`encoding.csv` replay instead.

Run:  pip install openlocationcode
      python3 scripts/fuzz/openlocationcode/gen.py [out]
"""

import pathlib
import random
import sys

from openlocationcode import openlocationcode as reference

ROOT = pathlib.Path(__file__).resolve().parents[3]
DEFAULT_OUT = ROOT / "crates" / "openlocationcode" / "tests" / "fixtures" / "fuzz.tsv"

ALPHABET = "23456789CFGHJMPQRVWX"
JUNK = "0+abcdeZ!-* 189.,/"

# A fixed seed, so the corpus is the same every run; a corpus that changes
# between runs cannot be reviewed as a diff.
RNG = random.Random(0x0C7_2026)

LENGTHS = [2, 4, 6, 8, 10, 11, 12, 13, 14, 15]


def random_string() -> str:
    n = RNG.randint(0, 16)
    return "".join(RNG.choice(ALPHABET + "0+" * 3 + JUNK) for _ in range(n))


def random_point() -> tuple[float, float]:
    return RNG.uniform(-90.0, 90.0), RNG.uniform(-180.0, 180.0)


def build_rows() -> list[list[str]]:
    rows: list[list[str]] = []

    # Validity: random strings, then valid codes and their mutations, so both
    # the accept and the reject side carry real cases.
    for _ in range(4000):
        code = random_string()
        rows.append(["V", code, _b(reference.isValid(code)), _b(reference.isShort(code)),
                     _b(reference.isFull(code))])

    for _ in range(3000):
        lat, lng = random_point()
        length = RNG.choice(LENGTHS)
        code = reference.encode(lat, lng, length)
        for candidate in (code, code.lower(), _mutate(code), _mutate(code)):
            rows.append(["V", candidate, _b(reference.isValid(candidate)),
                         _b(reference.isShort(candidate)), _b(reference.isFull(candidate))])

    # Decode: every valid full code the reference produced.
    decoded = set()
    for _ in range(2000):
        lat, lng = random_point()
        code = reference.encode(lat, lng, RNG.choice(LENGTHS))
        if code in decoded or not reference.isFull(code):
            continue
        decoded.add(code)
        area = reference.decode(code)
        rows.append(["D", code, repr(area.latitudeLo), repr(area.longitudeLo),
                     repr(area.latitudeHi), repr(area.longitudeHi), str(area.codeLength)])

    # Shorten and recover: a code, a reference point near it, and both answers.
    for _ in range(2000):
        lat, lng = random_point()
        code = reference.encode(lat, lng, 10)
        if len(code) < 6:
            continue
        try:
            short = reference.shorten(code, lat, lng)
        except Exception:
            continue
        rows.append(["S", code, repr(lat), repr(lng), short])
        rows.append(["R", short, repr(lat), repr(lng), reference.recoverNearest(short, lat, lng)])

    # Encode: coordinates and length to a code, with the reference's tolerance.
    for _ in range(2000):
        lat, lng = random_point()
        length = RNG.choice(LENGTHS + [16])
        rows.append(["E", repr(lat), repr(lng), str(length), reference.encode(lat, lng, length)])

    return rows


def _b(value: bool) -> str:
    return "true" if value else "false"


def _mutate(code: str) -> str:
    if not code:
        return code
    i = RNG.randrange(len(code))
    return code[:i] + RNG.choice(ALPHABET + "0+") + code[i + 1 :]


def main() -> int:
    out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    rows = build_rows()
    out.write_text("".join("\t".join(row) + "\n" for row in rows))
    print(f"wrote {len(rows)} rows to {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
