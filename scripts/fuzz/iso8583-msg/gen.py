#!/usr/bin/env python3
"""Builds the `iso8583-msg` corpus from `pyiso8583`, the reference implementation.

The crate's hand-picked fixture list (`tests/fixtures.rs`) proves the cases a
person thought of are right; this generator adds the cases a person would not
write down. It builds random messages over the reference's own `default_ascii`
specification, asks the reference to encode each one, and records the exact
bytes it produced. The result is `crates/iso8583-msg/tests/fixtures/oracle.tsv`:

    mti <TAB> field=value;field=value <TAB> hex

A row means: a message with these fields, encoded with `default_ascii`, is
exactly these bytes. The crate has to reproduce the bytes and read the fields
back out of them, so a divergence in the bitmap, a length prefix or a field's
encoding shows up as a named case rather than a wrong answer.

The field values are generated inside the reference's own limits (a fixed field
is exactly its length, a variable one is at most its maximum), so every row is a
message the reference accepts; the corpus tests the encoding, not the rejection
rules, which the curated fixture list already covers.

Run:  pip install pyiso8583==4.0.1
      python3 scripts/fuzz/iso8583-msg/gen.py [out]
"""

import pathlib
import random
import string
import sys

import iso8583
from iso8583.specs import default_ascii as reference_spec

ROOT = pathlib.Path(__file__).resolve().parents[3]
DEFAULT_OUT = ROOT / "crates" / "iso8583-msg" / "tests" / "fixtures" / "oracle.tsv"

# The message type indicators the curated fixtures use. Keeping to these means
# every row is a message the crate's own list already treats as in scope.
MTIS = ["0100", "0110", "0200", "0210", "0400", "0410", "0800", "0810"]

# Field 0 is the MTI, field 1 the primary bitmap and field 65 the extended
# bitmap; all three are structural and are written by the encoder, not supplied
# as data.
STRUCTURAL_FIELDS = {0, 1, 65}
DATA_FIELDS = [n for n in range(2, 129) if n not in STRUCTURAL_FIELDS]

PRINTABLE = string.ascii_uppercase + string.digits + " .-/"


def value_for(field, rng):
    """A value inside the reference's own limits for `field`.

    Variable fields are generated with at least one character. The crate models
    an empty value as "field absent" (`Message::set` removes the field), while
    the reference encodes a present-but-empty variable field as a zero length
    prefix; the two cannot agree on a case the crate cannot represent, so the
    corpus leaves it out rather than inventing an expectation for it.
    """
    entry = reference_spec[str(field)]
    max_len = entry["max_len"]
    len_type = entry["len_type"]
    if len_type == 0:
        length = max_len
    else:
        length = rng.randint(1, max_len)
    return "".join(rng.choice(PRINTABLE) for _ in range(length))


def build_message(rng):
    mti = rng.choice(MTIS)
    count = rng.randint(1, 6)
    fields = rng.sample(DATA_FIELDS, count)
    data = {"t": mti}
    for field in fields:
        data[str(field)] = value_for(field, rng)
    return mti, data


def main():
    out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    rng = random.Random(0x158583)

    rows = []
    seen = set()
    while len(rows) < 500:
        mti, data = build_message(rng)
        # `iso8583.encode` writes the bitmap fields back into the mapping it is
        # given, so the intended fields are recorded before the call, not after.
        fields = ";".join(
            f"{k}={v}"
            for k, v in sorted(data.items(), key=lambda kv: 0 if kv[0] == "t" else int(kv[0]))
            if k != "t"
        )
        try:
            raw, _ = iso8583.encode(dict(data), spec=reference_spec)
        except Exception:
            continue
        key = (mti, fields)
        if key in seen:
            continue
        seen.add(key)
        rows.append((mti, fields, raw.hex().upper()))

    lines = [f"{mti}\t{fields}\t{hexed}" for mti, fields, hexed in rows]
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")
    print(f"{len(rows)} cases -> {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
