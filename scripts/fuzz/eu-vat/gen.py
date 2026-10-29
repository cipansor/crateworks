#!/usr/bin/env python3
"""Builds the `eu-vat` corpus from `python-stdnum`, the reference implementation.

The crate's hand-picked fixture list proves the cases a person thought of are
right; this generator adds the cases a person would not write down. It starts
from the numbers the crate already records as valid, mutates them (a changed
check digit, a separator in the wrong place, the wrong country prefix, a
truncated or padded tail), and records what the reference says about each one.
The result is `crates/eu-vat/tests/fixtures/oracle.tsv`, one row per case:

    input <TAB> accepted <TAB> canonical

`accepted` is `1` or `0`; `canonical` is the reference's normalised form and is
empty when the reference rejects the input.

One deliberate divergence is folded in here rather than left to fail the test:
the reference canonicalises a Greek number as `GR...` and the crate as `EL...`,
and the crate documents `EL` as the canonical spelling, so the recorded
canonical rewrites the reference's `GR` prefix to `EL`.

Run:  pip install python-stdnum==2.2
      python3 scripts/fuzz/eu-vat/gen.py [out]
"""

import pathlib
import random
import re
import sys

import stdnum.eu.vat as reference

ROOT = pathlib.Path(__file__).resolve().parents[3]
DEFAULT_OUT = ROOT / "crates" / "eu-vat" / "tests" / "fixtures" / "oracle.tsv"
FIXTURE_RS = ROOT / "crates" / "eu-vat" / "tests" / "fixtures.rs"

SEPARATORS = [" ", ".", "-", "/", ":", ","]

# The characters `Vat::parse` drops before it validates, mirroring
# `normalize()` in the crate. The reference strips fewer of them, so the
# generator removes them before asking the reference, and the two sides are
# then compared on the same characters: what is under test is the validation
# rule, not the separator policy.
CRATE_SEPARATORS = " -./:()\u00a0"


def crate_normalize(text):
    return "".join(c for c in text if c not in CRATE_SEPARATORS).upper()


def seeds():
    """The valid numbers the crate's own fixture list records.

    Reading them from `fixtures.rs` keeps this generator in step with the
    curated cases: a number added there becomes a seed here too.
    """
    text = FIXTURE_RS.read_text()
    block = re.search(r"const VALID: &\[\(&str, &str, &str\)\] = &\[(.*?)\];", text, re.S)
    if block is None:
        raise SystemExit("could not find VALID in tests/fixtures.rs")
    found = re.findall(r'\("([A-Z]{2})",\s*"([^"]*)",\s*"([^"]*)"\)', block.group(1))
    if not found:
        raise SystemExit("VALID block parsed empty")
    return [canonical for _prefix, _input, canonical in found]


def oracle(text):
    """(accepted, canonical) for `text` once the crate's separators are removed.

    The reference's own normalisation is left in place, except that its Greek
    prefix `GR` is rewritten to the `EL` the crate documents as canonical.
    """
    try:
        canonical = reference.validate(crate_normalize(text))
    except Exception:
        return (0, "")
    if canonical.startswith("GR"):
        canonical = "EL" + canonical[2:]
    return (1, canonical)


def mutate(number, rng):
    """A handful of one-step edits of a valid number."""
    out = []
    digits = [i for i, c in enumerate(number) if c.isdigit()]
    if digits:
        # Change one digit, which normally breaks the check digit.
        i = rng.choice(digits)
        out.append(number[:i] + str((int(number[i]) + rng.randint(1, 9)) % 10) + number[i + 1 :])
    if len(number) > 3:
        out.append(number[:-1])  # drop the last character
        # Insert a separator the crate is documented to ignore.
        i = rng.randint(2, len(number) - 1)
        out.append(number[:i] + rng.choice(SEPARATORS) + number[i:])
    out.append(number + str(rng.randint(0, 9)))  # pad
    out.append(number.lower())  # the crate upper-cases its input
    return out


def main():
    out = pathlib.Path(sys.argv[1]) if len(sys.argv) > 1 else DEFAULT_OUT
    rng = random.Random(0xC1A55)

    seen = set()
    rows = []

    def add(text):
        if text in seen:
            return
        seen.add(text)
        accepted, canonical = oracle(text)
        rows.append((text, accepted, canonical))

    for canonical in seeds():
        add(canonical)
        for mutated in mutate(canonical, rng):
            add(mutated)
        # Wrong country prefix, which the crate's own checks must reject.
        for other in ("DE", "FR", "IT", "XI", "US", "ZZ"):
            add(other + canonical[2:])

    lines = [f"{text}\t{accepted}\t{canonical}" for text, accepted, canonical in rows]
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text("\n".join(lines) + "\n")
    accepted = sum(1 for _, a, _ in rows if a)
    print(f"{len(rows)} cases ({accepted} accepted) -> {out}", file=sys.stderr)


if __name__ == "__main__":
    main()
