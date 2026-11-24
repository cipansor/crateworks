#!/usr/bin/env python3
"""Generate the edit-distance fixture for `fuzzy-score`.

Two independent references are consulted for every pair: `rapidfuzz` (the
maintained successor to fuzzywuzzy's C++ core) and `jellyfish`. They agree on
every metric below except one, and the exception is recorded rather than
papered over:

    Jaro/Jaro-Winkler of two empty strings
        rapidfuzz 1.0        jellyfish 0.0

Neither is wrong; the references disagree about whether "two empty strings are
identical" or "there is nothing to compare" wins. This crate returns 1.0, which
is what rapidfuzz, the `strsim` crate and the original Winkler paper all say, and
the generator asserts that the pair is the only disagreement so a future
reference update cannot quietly widen it.

Floats are written with `repr()`, which round-trips exactly in Rust's `f64`
parser, so a comparison is exact rather than approximate.

Usage:
    pip install rapidfuzz jellyfish
    python3 scripts/fuzz/fuzzy-score/gen_distance.py > crates/fuzzy-score/tests/fixtures/distance.tsv
"""

import itertools
import random
import sys

from rapidfuzz.distance import (
    DamerauLevenshtein,
    Hamming,
    Jaro,
    JaroWinkler,
    Levenshtein,
    OSA,
)

import jellyfish

# The alphabet is deliberately small: it produces far more ties and transpositions
# per pair than real text, which is where an edit-distance implementation breaks.
ALPHABETS = [
    "ab",
    "abc",
    "abcd",
    "abcdefgh",
    "aàé😀",
    "\u00e9\u00e8\u00ea",  # combining-free accents, one char each
]

# Words that exercise the boundaries: empty, single characters, repeated
# characters, and a long pair where the Jaro match window matters.
FIXED = [
    "",
    "a",
    "ab",
    "abc",
    "abcd",
    "CA",
    "ABC",
    "MARTHA",
    "MARHTA",
    "DIXON",
    "DICKSONX",
    "JELLYFISH",
    "SMELLYFISH",
    "kitten",
    "sitting",
    "a cat",
    "an act",
    "abcdefgh",
    "abcdefgx",
    "abcdef",
    "abcxyz",
    "café",
    "cafe",
    "😀a",
    "😀b",
    "aaaa",
    "aaab",
    "abab",
    "baba",
]


def metrics(a, b):
    """The seven values the crate must reproduce, in fixture column order.

    `JaroWinkler.normalized_similarity` is deliberately absent: for a similarity
    already in `0..=1` it is `1 - (1 - similarity)`, which re-rounds the last bit
    rather than measuring anything new. Asserting bit-equality against it would
    pin an implementation artifact, not the algorithm.
    """
    return (
        Levenshtein.distance(a, b),
        DamerauLevenshtein.distance(a, b),
        OSA.distance(a, b),
        Hamming.distance(a, b),
        Jaro.similarity(a, b),
        JaroWinkler.similarity(a, b),
        Levenshtein.normalized_similarity(a, b),
    )


def jelly(a, b):
    """The same values from the second reference, for the cross-check."""
    return (
        jellyfish.levenshtein_distance(a, b),
        jellyfish.damerau_levenshtein_distance(a, b),
        None,  # jellyfish has no OSA
        jellyfish.hamming_distance(a, b),
        jellyfish.jaro_similarity(a, b),
        jellyfish.jaro_winkler_similarity(a, b),
        None,  # computed by the caller from the distance
    )


def escape(s):
    """Escape a field so it never contains a tab or a newline."""
    out = []
    for c in s:
        if c == "\\":
            out.append("\\\\")
        elif c == "\t":
            out.append("\\t")
        elif c == "\n":
            out.append("\\n")
        elif c == "\r":
            out.append("\\r")
        elif " " <= c <= "~":
            out.append(c)
        else:
            out.append(f"\\u{ord(c):04x}" if ord(c) <= 0xFFFF else f"\\U{ord(c):08x}")
    return "".join(out)


def main():
    random.seed(20260116)

    pairs = set()
    for a, b in itertools.product(FIXED, repeat=2):
        pairs.add((a, b))

    for _ in range(4000):
        alphabet = random.choice(ALPHABETS)
        len_a = random.randint(0, 12)
        len_b = random.randint(0, 12)
        a = "".join(random.choice(alphabet) for _ in range(len_a))
        b = "".join(random.choice(alphabet) for _ in range(len_b))
        pairs.add((a, b))

    # Mutations of a base string, which is where a transposition bug shows: a
    # single swap is one Damerau edit and two Levenshtein ones.
    for _ in range(1500):
        alphabet = random.choice(ALPHABETS)
        base = [random.choice(alphabet) for _ in range(random.randint(1, 10))]
        mutated = list(base)
        for _ in range(random.randint(1, 3)):
            op = random.choice(["sub", "ins", "del", "swap"])
            if not mutated:
                mutated = [random.choice(alphabet)]
            i = random.randrange(len(mutated))
            if op == "sub":
                mutated[i] = random.choice(alphabet)
            elif op == "ins":
                mutated.insert(i, random.choice(alphabet))
            elif op == "del":
                mutated.pop(i)
            elif len(mutated) >= 2 and i + 1 < len(mutated):
                mutated[i], mutated[i + 1] = mutated[i + 1], mutated[i]
        pairs.add(("".join(base), "".join(mutated)))

    disagreements = []
    rows = []
    for a, b in sorted(pairs):
        ours = metrics(a, b)
        theirs = jelly(a, b)

        # jellyfish has no OSA and no normalized similarity, so compare the five
        # that both define.
        for name, mine, other in (
            ("levenshtein", ours[0], theirs[0]),
            ("damerau", ours[1], theirs[1]),
            ("hamming", ours[3], theirs[3]),
            ("jaro", ours[4], theirs[4]),
            ("jaro_winkler", ours[5], theirs[5]),
        ):
            if abs(mine - other) > 1e-12:
                disagreements.append((name, a, b, mine, other))

        rows.append((a, b, ours))

    # The empty-string Jaro pair is the one documented divergence. Anything else
    # is a reference bug or a bad assumption in this generator, and the fixture
    # must not be written until it is understood.
    unexpected = [
        d
        for d in disagreements
        if not (d[1] == "" and d[2] == "" and d[0] in ("jaro", "jaro_winkler"))
    ]
    if unexpected:
        print(f"{len(unexpected)} unexpected oracle disagreements:", file=sys.stderr)
        for d in unexpected[:10]:
            print(f"  {d[0]} {d[1]!r} {d[2]!r}: rapidfuzz={d[3]} jellyfish={d[4]}", file=sys.stderr)
        return 1

    empty_disagreements = len(disagreements) - len(unexpected)
    print(f"# {len(rows)} pairs", file=sys.stderr)
    print(
        f"# oracles agree on {len(rows) * 5 - len(disagreements)} of {len(rows) * 5} "
        f"comparisons; the {empty_disagreements} exceptions are the empty-string "
        f"Jaro pairs, where this crate follows rapidfuzz and returns 1.0",
        file=sys.stderr,
    )

    print("# a\tb\tlev\tdamerau\tosa\thamming\tjaro\tjaro_winkler\tlev_norm")
    for a, b, values in rows:
        fields = [escape(a), escape(b)] + [repr(v) for v in values]
        print("\t".join(fields))
    return 0


if __name__ == "__main__":
    sys.exit(main())
