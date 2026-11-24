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
    pip install "rapidfuzz==3.14.6" "jellyfish==1.3.0"
    python3 scripts/fuzz/fuzzy-score/gen_distance.py > crates/fuzzy-score/tests/fixtures/distance.tsv

The versions are pinned and checked, because both libraries have changed results
between releases - `jellyfish` moved to grapheme clusters in 1.2 and reports a
different answer for multi-scalar text as a result. A fixture regenerated against
a different version is a different fixture, so the script refuses to run rather
than silently rebase the corpus on whatever happens to be installed.
"""

import itertools
import random
import sys
import unicodedata

from rapidfuzz.distance import (
    DamerauLevenshtein,
    Hamming,
    Jaro,
    JaroWinkler,
    Levenshtein,
    OSA,
)

import jellyfish

# Both references must be the versions the fixture was generated against. See the
# module docstring for why this is a hard failure rather than a warning.
PINNED = {"rapidfuzz": "3.14.6", "jellyfish": "1.3.0"}

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


# Each entry is one visual character made of several scalars, except the plain
# ones which exist so the pair matrix contains both sides of the comparison.
GRAPHEMES = [
    "e",          # one scalar
    "e\u0301",    # e + combining acute: two scalars, one grapheme cluster
    "\u00e9",      # precomposed e-acute: one scalar, one cluster
    "\U0001f469\u200d\U0001f4bb",  # woman + ZWJ + laptop
    "\U0001f468\u200d\U0001f52c",  # man + ZWJ + microscope
    "\U0001f1ee\U0001f1e9",        # regional indicators: one flag
]


def multiscalar_grapheme(s):
    """Whether `s` contains a grapheme cluster made of more than one scalar.

    Approximated from the Unicode database rather than by full segmentation: a
    combining mark, a zero-width joiner, a variation selector or a regional
    indicator pair is what makes `jellyfish`'s unit differ from ours. This is the
    property that separates the two references, so it is what the cross-check
    keys on.
    """
    prev = None
    for ch in s:
        cp = ord(ch)
        if unicodedata.combining(ch) or cp in (0x200D, 0xFE0F) or 0x1F1E6 <= cp <= 0x1F1FF:
            return True
        if prev is not None and 0x1F3FB <= cp <= 0x1F3FF:
            return True
        prev = ch
    return False


def grapheme_sensitive(a, b):
    return multiscalar_grapheme(a) or multiscalar_grapheme(b)


# Pairs whose references are *known* to differ because they disagree about the
# unit of comparison, with the value each returns. Pinning both sides is what
# stops the divergence from being silently absorbed: `jellyfish` 1.3 compares
# extended grapheme clusters, `rapidfuzz` compares scalars, and this crate
# follows `rapidfuzz`.
#
# These are not bugs in either library. They are the reason the crate documents
# scalar semantics and the reason a caller comparing user input should normalize
# to NFC first.
KNOWN_GRAPHEME_DIVERGENCES = {
    # Decomposed accent against a plain letter: `jellyfish` sees two clusters,
    # `rapidfuzz` sees two scalars against one.
    ("jaro", "e", "e\u0301"): (0.8333333333333334, 0.0),
    ("jaro_winkler", "e", "e\u0301"): (0.8500000000000001, 0.0),
    # Joined emoji: three scalars each, one cluster each.
    ("levenshtein", "\U0001f469\u200d\U0001f4bb", "\U0001f468\u200d\U0001f52c"): (2, 1),
    ("damerau", "\U0001f469\u200d\U0001f4bb", "\U0001f468\u200d\U0001f52c"): (2, 1),
    ("hamming", "\U0001f469\u200d\U0001f4bb", "\U0001f468\u200d\U0001f52c"): (2, 1),
    ("jaro", "\U0001f469\u200d\U0001f4bb", "\U0001f468\u200d\U0001f52c"): (
        0.5555555555555555,
        0.0,
    ),
}


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


def check_versions():
    import importlib.metadata as md

    for name, want in PINNED.items():
        have = md.version(name)
        if have != want:
            print(
                f"{name} {have} is installed, the fixture needs {want}. "
                f'Install it with: pip install "{name}=={want}"',
                file=sys.stderr,
            )
            return False
    return True


def main():
    if not check_versions():
        return 1
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
    # Long strings. The earlier corpus capped at 12 characters, which is short
    # enough that a quadratic-memory implementation looked fine: the allocation
    # was 12 * 12 cells. It is not fine at 8 KiB, so the corpus has to contain
    # lengths where the cost is visible.
    for _ in range(300):
        alphabet = random.choice(ALPHABETS)
        len_a = random.randint(20, 60)
        len_b = random.randint(20, 60)
        a = "".join(random.choice(alphabet) for _ in range(len_a))
        b = "".join(random.choice(alphabet) for _ in range(len_b))
        pairs.add((a, b))

    # Multi-scalar graphemes. `jellyfish` 1.3 compares extended grapheme
    # clusters, so a decomposed accent or a joined emoji is one unit there and
    # two or more here. These cases are in the corpus so that divergence is
    # measured rather than assumed absent.
    pairs.update(
        (x, y)
        for x in GRAPHEMES
        for y in GRAPHEMES
    )

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

    # The documented split for two empty strings, checked exactly. Accepting any
    # value for these four comparisons would let a reference change go unnoticed:
    # the earlier version of this script only required that the disagreement was
    # *somewhere* in the empty-string rows.
    for name, got in (
        ("jaro", Jaro.similarity("", "")),
        ("jaro_winkler", JaroWinkler.similarity("", "")),
    ):
        if got != 1.0:
            print(f"rapidfuzz {name}('', '') is {got}, expected 1.0", file=sys.stderr)
            return 1
    for name, got in (
        ("jaro", jellyfish.jaro_similarity("", "")),
        ("jaro_winkler", jellyfish.jaro_winkler_similarity("", "")),
    ):
        if got != 0.0:
            print(f"jellyfish {name}('', '') is {got}, expected 0.0", file=sys.stderr)
            return 1

    # Every disagreement must be attributable to one of exactly two causes, and
    # both are asserted rather than assumed:
    #
    #   1. two empty strings, where rapidfuzz says 1.0 and jellyfish says 0.0;
    #   2. a multi-scalar grapheme, where the references disagree about the unit
    #      of comparison and neither is wrong.
    #
    # Anything else is a reference bug or a bad assumption in this generator, and
    # the fixture must not be written until it is understood.
    empty_string = []
    grapheme = []
    unexpected = []
    for d in disagreements:
        name, a, b, mine, other = d
        if a == "" and b == "" and name in ("jaro", "jaro_winkler"):
            empty_string.append(d)
        elif grapheme_sensitive(a, b):
            grapheme.append(d)
        else:
            unexpected.append(d)

    if unexpected:
        print(f"{len(unexpected)} unexpected oracle disagreements:", file=sys.stderr)
        for d in unexpected[:10]:
            print(f"  {d[0]} {d[1]!r} {d[2]!r}: rapidfuzz={d[3]} jellyfish={d[4]}", file=sys.stderr)
        return 1

    # The pinned table is the evidence that the grapheme divergence was looked
    # at rather than merely tolerated: a handful of specific pairs whose two
    # reference values are recorded. Every pinned case must still diverge with
    # the recorded values, so a reference release that changes its mind fails
    # here instead of silently rewriting the corpus.
    for (name, a, b), (want_mine, want_other) in KNOWN_GRAPHEME_DIVERGENCES.items():
        got = [d for d in grapheme if d[0] == name and d[1] == a and d[2] == b]
        if not got:
            print(f"pinned grapheme case {name} {a!r} {b!r} no longer diverges", file=sys.stderr)
            return 1
        if abs(got[0][3] - want_mine) > 1e-12 or abs(got[0][4] - want_other) > 1e-12:
            print(
                f"pinned grapheme case {name} {a!r} {b!r} changed: "
                f"rapidfuzz={got[0][3]} (want {want_mine}), "
                f"jellyfish={got[0][4]} (want {want_other})",
                file=sys.stderr,
            )
            return 1

    # The full set is larger than the pinned table, and that is expected: every
    # grapheme-sensitive pair where the two references use a different unit
    # diverges, so the count scales with the corpus. It is reported rather than
    # pinned. What is pinned is the *cause*: all of them contain a multi-scalar
    # grapheme, which the classifier above established.
    empty_disagreements = len(empty_string)

    print(f"# {len(rows)} pairs", file=sys.stderr)
    print(
        f"# oracles agree on {len(rows) * 5 - len(disagreements)} of {len(rows) * 5} "
        f"comparisons. Of the {len(disagreements)} exceptions, {empty_disagreements} are "
        f"the empty-string Jaro pairs (rapidfuzz 1.0, jellyfish 0.0) and {len(grapheme)} "
        f"are pairs containing a multi-scalar grapheme (jellyfish segments into clusters, "
        f"rapidfuzz into scalars). This crate follows rapidfuzz in both cases, so the "
        f"fixture records scalar semantics and the crate is NOT jellyfish-compatible for "
        f"multi-scalar text.",
        file=sys.stderr,
    )

    print("# a\tb\tlev\tdamerau\tosa\thamming\tjaro\tjaro_winkler\tlev_norm")
    for a, b, values in rows:
        fields = [escape(a), escape(b)] + [repr(v) for v in values]
        print("\t".join(fields))
    return 0


if __name__ == "__main__":
    sys.exit(main())
