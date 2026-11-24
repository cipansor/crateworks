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
is what rapidfuzz, the `strsim` crate and the original Winkler paper all say.

The other divergence is the unit of comparison. `jellyfish` 1.3 segments into
extended grapheme clusters and `rapidfuzz` into Unicode scalar values, so every
pair containing a multi-scalar grapheme can disagree. That is not tolerated on
trust: for each such pair the generator re-asks `rapidfuzz` with both strings
reduced to one scalar per grapheme cluster, and requires the answer to equal what
`jellyfish` returned. A disagreement that segmentation does not explain fails the
run.

Floats are written with `repr()`, which round-trips exactly in Rust's `f64`
parser, so a comparison is exact rather than approximate.

Usage:
    pip install "rapidfuzz==3.14.6" "jellyfish==1.3.0"
    python3 scripts/fuzz/fuzzy-score/gen_distance.py

The output path defaults to the tracked fixture and is written **atomically**:
the file is replaced only after every check has passed. Redirecting stdout with
`>` truncates the fixture before this script runs, so a version mismatch would
leave an empty file behind and the oracle test would fail until it was restored
from git. Use `--out PATH` for an unseen-seed run, and never `>`.

Existing pairs are preserved. The generated draws below are seeded, so changing
any draw changes every later one - the first attempt at widening the corpus
silently dropped 1,283 pairs that had been in it. Any pair already in the output
file is re-asked and kept, so a corpus only ever grows.

The versions are pinned and checked, because both libraries have changed results
between releases - `jellyfish` moved to grapheme clusters in 1.2 and reports a
different answer for multi-scalar text as a result. A fixture regenerated against
a different version is a different fixture, so the script refuses to run rather
than silently rebase the corpus on whatever happens to be installed.
"""

import argparse
import itertools
import os
import random
import sys
import tempfile
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


def grapheme_clusters(s):
    """Split `s` into extended grapheme clusters, approximately.

    The rules that matter for this comparison are the ones that make
    `jellyfish`'s unit differ from a scalar: a combining mark, a variation
    selector, a skin-tone modifier, a zero-width-joiner sequence, and a regional
    indicator pair. Full UAX #29 also handles Hangul syllable composition and
    prepend characters, which no case in this corpus uses.

    This is an approximation of the *segmentation*, but it is used as an oracle
    test rather than as a definition: the check below only passes if reducing
    both strings to one scalar per cluster reproduces `jellyfish`'s answer
    exactly. A wrong segmentation fails that, it does not silently pass.
    """
    clusters = []
    current = ""
    join_next = False
    regional_pending = False
    for ch in s:
        cp = ord(ch)
        combining = bool(unicodedata.combining(ch)) or cp in (0xFE0E, 0xFE0F)
        modifier = 0x1F3FB <= cp <= 0x1F3FF
        zwj = cp == 0x200D
        regional = 0x1F1E6 <= cp <= 0x1F1FF
        if not current:
            current = ch
            join_next = zwj
            regional_pending = regional
            continue
        if join_next or combining or modifier or zwj:
            current += ch
            join_next = zwj
            continue
        if regional_pending and regional:
            current += ch
            regional_pending = False
            continue
        clusters.append(current)
        current = ch
        join_next = zwj
        regional_pending = regional
    if current:
        clusters.append(current)
    return clusters


def scalar_per_cluster(a, b):
    """Reduce both strings to one scalar per grapheme cluster.

    The mapping is shared between the two strings, so a cluster that appears in
    both gets the same scalar. That is what makes this an isomorphism onto a
    scalar string: it preserves equality and adjacency, so any metric computed on
    the reduced strings is the metric computed on clusters.
    """
    mapping = {}

    def convert(s):
        out = []
        for cluster in grapheme_clusters(s):
            if cluster not in mapping:
                mapping[cluster] = chr(0xE000 + len(mapping))
            out.append(mapping[cluster])
        return "".join(out)

    return convert(a), convert(b)


def grapheme_sensitive(a, b):
    return len(grapheme_clusters(a)) != len(a) or len(grapheme_clusters(b)) != len(b)


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


def read_existing_pairs(path):
    """The `a`/`b` columns of an existing fixture, unescaped.

    Pairs are read back rather than regenerated, so widening the corpus cannot
    drop a pair that a previous corpus contained. That is what the first attempt
    at this did: it changed the seeded draws and lost 1,283 pairs.
    """
    if not os.path.exists(path):
        return set()
    pairs = set()
    with open(path, encoding="utf-8") as f:
        for line in f:
            if line.startswith("#"):
                continue
            fields = line.rstrip("\n").split("\t")
            if len(fields) >= 2:
                pairs.add((unescape(fields[0]), unescape(fields[1])))
    return pairs


def unescape(s):
    """Inverse of `escape`."""
    out = []
    i = 0
    while i < len(s):
        c = s[i]
        if c != "\\":
            out.append(c)
            i += 1
            continue
        nxt = s[i + 1]
        if nxt == "\\":
            out.append("\\")
            i += 2
        elif nxt == "t":
            out.append("\t")
            i += 2
        elif nxt == "n":
            out.append("\n")
            i += 2
        elif nxt == "r":
            out.append("\r")
            i += 2
        elif nxt in ("u", "U"):
            width = 4 if nxt == "u" else 8
            out.append(chr(int(s[i + 2 : i + 2 + width], 16)))
            i += 2 + width
        else:
            out.append(c)
            i += 1
    return "".join(out)


def write_atomically(path, text):
    """Replace `path` only once the whole file has been produced.

    A `>` redirect truncates before the script runs, so a failure - a version
    mismatch, a new oracle disagreement - leaves an empty fixture behind and the
    oracle test fails until the file is restored from git. Writing to a sibling
    temporary file and renaming means a failed run leaves the old corpus intact.
    """
    directory = os.path.dirname(os.path.abspath(path))
    os.makedirs(directory, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=directory, prefix=".distance.", suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            f.write(text)
        os.replace(tmp, path)
    except BaseException:
        if os.path.exists(tmp):
            os.unlink(tmp)
        raise


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


DEFAULT_OUT = "crates/fuzzy-score/tests/fixtures/distance.tsv"


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--out",
        default=DEFAULT_OUT,
        help=f"fixture path to replace atomically (default: {DEFAULT_OUT})",
    )
    args = parser.parse_args(argv)

    if not check_versions():
        return 1

    # Pairs already in the output are re-asked and kept, so widening the corpus
    # can only add. Without this, a new seeded draw shifts every later draw and
    # silently drops pairs the previous corpus had.
    pairs = read_existing_pairs(args.out)
    carried = len(pairs)

    random.seed(20260116)
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

    # Every grapheme-sensitive disagreement must be *explained*, not merely
    # counted. Reducing both strings to one scalar per grapheme cluster and
    # asking `rapidfuzz` again must reproduce what `jellyfish` returned; if it
    # does not, the disagreement is not about segmentation and this generator
    # has no business recording it. This is what makes the divergence a measured
    # property rather than an assumption that happens to hold.
    #
    # The comparison is per metric, so the mapping from a fixture column to the
    # two reference functions lives here.
    cluster_pair = {}
    unexplained = []
    for name, a, b, mine, other in grapheme:
        key = (a, b)
        if key not in cluster_pair:
            cluster_pair[key] = scalar_per_cluster(a, b)
        ca, cb = cluster_pair[key]
        if name == "levenshtein":
            predicted = Levenshtein.distance(ca, cb)
        elif name == "damerau":
            predicted = DamerauLevenshtein.distance(ca, cb)
        elif name == "hamming":
            predicted = Hamming.distance(ca, cb)
        elif name == "jaro":
            predicted = Jaro.similarity(ca, cb)
        elif name == "jaro_winkler":
            predicted = JaroWinkler.similarity(ca, cb)
        else:
            raise AssertionError(f"no cluster prediction for metric {name}")
        if abs(predicted - other) > 1e-12:
            unexplained.append((name, a, b, other, predicted))

    if unexplained:
        print(
            f"{len(unexplained)} grapheme disagreement(s) that cluster segmentation "
            f"does not explain:",
            file=sys.stderr,
        )
        for name, a, b, other, predicted in unexplained[:10]:
            print(
                f"  {name} {a!r} {b!r}: jellyfish={other}, "
                f"rapidfuzz-on-clusters={predicted}",
                file=sys.stderr,
            )
        return 1

    # The pinned table is a spot-check on top of that: a handful of specific
    # pairs whose two reference values are recorded by hand, so a release that
    # changes its mind about the *values* fails here even if the cluster
    # explanation still holds.
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

    empty_disagreements = len(empty_string)

    print(
        f"# {len(rows)} pairs ({carried} carried over from the existing fixture)",
        file=sys.stderr,
    )
    print(
        f"# oracles agree on {len(rows) * 5 - len(disagreements)} of {len(rows) * 5} "
        f"comparisons. Of the {len(disagreements)} exceptions, {empty_disagreements} are "
        f"the empty-string Jaro pairs (rapidfuzz 1.0, jellyfish 0.0) and {len(grapheme)} "
        f"are pairs containing a multi-scalar grapheme. Every one of the latter is "
        f"reproduced by rapidfuzz when both strings are reduced to one scalar per "
        f"grapheme cluster, which is what establishes that segmentation - and not a "
        f"reference bug - is the cause. This crate follows rapidfuzz in both cases, so "
        f"the fixture records scalar semantics and the crate is NOT jellyfish-compatible "
        f"for multi-scalar text.",
        file=sys.stderr,
    )

    lines = ["# a\tb\tlev\tdamerau\tosa\thamming\tjaro\tjaro_winkler\tlev_norm"]
    for a, b, values in rows:
        fields = [escape(a), escape(b)] + [repr(v) for v in values]
        lines.append("\t".join(fields))

    write_atomically(args.out, "\n".join(lines) + "\n")
    print(f"# wrote {len(rows)} pairs to {args.out}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
