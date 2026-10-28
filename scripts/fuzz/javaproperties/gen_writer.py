"""Build the writer-side fixture for `crates/javaproperties`.

The existing `gen.py` checks the *reader*: it feeds documents through
`javaproperties.loads` and records the pairs. A corpus of inputs cannot check the
writer at all, so this records the reference's own output for the writer API that
this crate now exposes:

    escape, join_key_value, to_comment, java_timestamp, dumps(separator=,
    comments=, sort_keys=, ensure_ascii=), unescape

Every expectation here is produced by Python's `javaproperties` 0.8.2, which
replicates `java.util.Properties` and is itself tested against Java. Nothing is
hand-written.

One deliberate difference is recorded rather than hidden: `java_timestamp` in the
reference renders the *local* timezone, and the crate renders UTC because it has
no timezone database. The generator therefore runs with `TZ=UTC` and records the
`UTC` spellings, and the crate's function is named `java_timestamp_utc` so the
difference is visible at the call site.

Usage:
    TZ=UTC python3 gen_writer.py [target.tsv]
"""

import javaproperties
import random
import re
import sys

random.seed(20261026)

# Java writes upper-case hex in `\uXXXX`; the Python port writes lower case.
# Normalise every reference output to what Java would write, which is the
# behaviour the crate implements. The reader generator does the same, for the
# same reason. A lone backslash is always doubled by the reference, so a `\u` in
# an output is always an escape and never literal text, which makes this safe.
_U_ESCAPE = re.compile(r"\\u([0-9a-fA-F]{4})")


def java_case(text: str) -> str:
    return _U_ESCAPE.sub(lambda m: "\\u" + m.group(1).upper(), text)

# --- Curated cases -----------------------------------------------------------
# Each pins a rule the reference's docstrings state, so a failure names the rule.

ESCAPE_CASES = [
    "", "a", "a b", " lead", "trail ", "a=b", "a:b", "a#b", "a!b", "a\\b",
    "a\tb", "a\nb", "a\rb", "a\x0cb", "a\x00b", "a\x7fb", "a\x85b", "a\xa0b",
    "café", "中文", "😀", "a=b:c#d!e f", "\\", "#", "!", "=", ":", " ",
]

JOIN_CASES = [
    ("a", "b"), ("a b", "c d"), ("a=b", "c:d"), ("a", " lead"), ("a", "trail "),
    ("", ""), ("", "x"), ("k", ""), ("a#b", "c!d"), ("café", "中文"),
    ("a\\b", "c\\d"), ("a\tb", "c\nd"),
]

COMMENT_CASES = [
    "hello", "one\ntwo", "one\r\ntwo", "one\rtwo", "#already", "!already",
    "one\n#two", "one\n!two", "a\n\nb", "\n", "", "café", "中文", "one\n#",
    "#", "a\r\n#b", "line1\nline2\nline3",
]

# A fixed instant, so the fixture is reproducible. The reference renders local
# time, so these are only stable under TZ=UTC.
TIMESTAMP_SECONDS = [0, 1, 86_399, 86_400, 946_684_800, 951_782_400,
                     1_000_000_000, 1_700_000_000, -1]

SEPARATORS = ["=", ": ", " ", "\t", " = "]


def escape_tsv(value: str) -> str:
    """Encode a string for a tab-separated fixture, like the reader generator."""
    out = []
    for c in value:
        o = ord(c)
        if c == "\\":
            out.append("\\\\")
        elif c == "\n":
            out.append("\\n")
        elif c == "\r":
            out.append("\\r")
        elif c == "\t":
            out.append("\\t")
        elif c == "\f":
            out.append("\\f")
        elif 0x20 <= o < 0x7F:
            out.append(c)
        elif o <= 0xFF:
            out.append("\\x%02x" % o)
        elif o <= 0xFFFF:
            out.append("\\u%04x" % o)
        else:
            out.append("\\U%08x" % o)
    return "".join(out)


def write(name: str, rows: list[str]) -> None:
    target = sys.argv[1] if len(sys.argv) > 1 else (
        "crates/javaproperties/tests/fixtures/writer.tsv"
    )
    path = target.replace("writer.tsv", name)
    with open(path, "w") as fp:
        fp.write("\n".join(rows) + "\n")
    print("wrote", len(rows) - 1, "cases to", path)


# --- escape ------------------------------------------------------------------
rows = ["input\tescaped"]
for s in ESCAPE_CASES:
    rows.append("\t".join([escape_tsv(s), escape_tsv(java_case(javaproperties.escape(s)))]))
write("escape.tsv", rows)

# --- join_key_value ----------------------------------------------------------
rows = ["key\tvalue\tseparator\tjoined"]
for key, value in JOIN_CASES:
    for sep in SEPARATORS:
        rows.append("\t".join([
            escape_tsv(key), escape_tsv(value), escape_tsv(sep),
            escape_tsv(java_case(javaproperties.join_key_value(key, value, separator=sep))),
        ]))
write("join.tsv", rows)

# --- to_comment --------------------------------------------------------------
rows = ["comment\tformatted"]
for s in COMMENT_CASES:
    rows.append("\t".join([escape_tsv(s), escape_tsv(java_case(javaproperties.to_comment(s)))]))
write("comment.tsv", rows)

# --- java_timestamp ----------------------------------------------------------
rows = ["seconds\ttimestamp"]
for seconds in TIMESTAMP_SECONDS:
    rows.append("\t".join([str(seconds), escape_tsv(java_case(javaproperties.java_timestamp(seconds)))]))
write("timestamp.tsv", rows)

# --- dumps with options ------------------------------------------------------
# The options are the reference's keyword arguments: separator, comments,
# sort_keys and ensure_ascii. `timestamp=False` keeps the fixture reproducible;
# the timestamp format has its own file above.
#
# The pairs are stored count-prefixed, key and value alternating, the way the
# reader corpus stores them. A `key=value|key=value` encoding looks simpler and is
# wrong: a key may itself hold `=` or `|`, and an empty pair list produces an
# empty field that decodes to no pairs at all.
PAIRS = [
    [("a", "1"), ("b", "2")],
    [("b", "2"), ("a", "1")],
    [("k", " lead"), ("k2", "trail ")],
    [("a b", "c d")],
    [("café", "中文")],
    [("", "x"), ("k", "")],
    [("a#b", "c=d:e!f")],
    [("dup", "1"), ("dup", "2")],
    [("a\tb", "c\nd")],
    [],
]
HEADER = "count\tseparator\thas_comments\tcomments\tsort_keys\tensure_ascii\tdocument\tpairs"


def dumps_row(pairs, sep, comments, sort_keys, ensure_ascii) -> str:
    document = java_case(javaproperties.dumps(
        pairs,
        separator=sep,
        comments=comments,
        timestamp=False,
        sort_keys=sort_keys,
        ensure_ascii=ensure_ascii,
    ))
    # `comments=""` and `comments=None` are different: the first writes a bare
    # `#` line, the second writes nothing. Both encode as an empty field, so the
    # flag column is what tells them apart.
    fields = [
        str(len(pairs)),
        escape_tsv(sep),
        "1" if comments is not None else "0",
        "" if comments is None else escape_tsv(comments),
        "1" if sort_keys else "0",
        "1" if ensure_ascii else "0",
        escape_tsv(document),
    ]
    for key, value in pairs:
        fields.append(escape_tsv(key))
        fields.append(escape_tsv(value))
    return "\t".join(fields)


rows = [HEADER]
for pairs in PAIRS:
    for sep in SEPARATORS:
        for comments in [None, "", "note", "one\ntwo"]:
            for sort_keys in [False, True]:
                for ensure_ascii in [True, False]:
                    rows.append(dumps_row(pairs, sep, comments, sort_keys, ensure_ascii))
write("dumps.tsv", rows)

# --- Random corpus -----------------------------------------------------------
# The curated cases pin the rules; this finds the interactions between them that
# nobody thinks to write down. It is the same idea as the reader corpus: build
# strings from the format's own punctuation and non-ASCII, and record what the
# reference produces. The crate must reproduce every one.
TOKENS = [
    "", " ", "a", "b", "=", ":", "#", "!", "\\", "\t", "\n", "\r", "\f",
    "\\n", "\\u0041", "é", "ü", "中", "😀", "\x00", "\x7f", "\x85", "\xa0",
    "\u2028", "€", "'", '"', "x" * 20, " lead", "trail ",
]


def piece(maxlen=6):
    return "".join(random.choice(TOKENS) for _ in range(random.randint(0, maxlen)))


rows = ["input\tescaped"]
for _ in range(2000):
    s = piece(8)
    rows.append("\t".join([escape_tsv(s), escape_tsv(java_case(javaproperties.escape(s)))]))
write("escape-random.tsv", rows)

rows = ["key\tvalue\tseparator\tjoined"]
for _ in range(2000):
    key, value = piece(), piece()
    sep = random.choice(SEPARATORS)
    rows.append("\t".join([
        escape_tsv(key), escape_tsv(value), escape_tsv(sep),
        escape_tsv(java_case(javaproperties.join_key_value(key, value, separator=sep))),
    ]))
write("join-random.tsv", rows)

rows = ["comment\tformatted"]
for _ in range(2000):
    s = piece(10)
    rows.append("\t".join([escape_tsv(s), escape_tsv(java_case(javaproperties.to_comment(s)))]))
write("comment-random.tsv", rows)

rows = [HEADER]
for _ in range(2000):
    pairs = [(piece(4), piece(4)) for _ in range(random.randint(0, 4))]
    sep = random.choice(SEPARATORS)
    comments = random.choice([None, "", piece(6)])
    sort_keys = random.random() < 0.5
    ensure_ascii = random.random() < 0.7
    rows.append(dumps_row(pairs, sep, comments, sort_keys, ensure_ascii))
write("dumps-random.tsv", rows)

# --- unescape ----------------------------------------------------------------
# The reader calls `unescape` on every key and value, but the reference also
# exposes it as a documented function, and so does this crate. The difference
# worth recording: a *lone* surrogate cannot be a Rust `char`, so the crate
# replaces it with U+FFFD, while Python keeps the raw code unit. A valid pair is
# decoded by both, so only the lone case is normalised here.
def crate_unescape(text: str) -> str:
    out = []
    i = 0
    while i < len(text):
        c = text[i]
        if 0xD800 <= ord(c) <= 0xDBFF and i + 1 < len(text):
            low = text[i + 1]
            if 0xDC00 <= ord(low) <= 0xDFFF:
                out.append(chr(0x10000 + ((ord(c) - 0xD800) << 10) + (ord(low) - 0xDC00)))
                i += 2
                continue
        if 0xD800 <= ord(c) <= 0xDFFF:
            out.append("\ufffd")
        else:
            out.append(c)
        i += 1
    return "".join(out)


UNESCAPE_CASES = [
    "", "a", r"a\tb", r"a\nb", r"a\fb", r"a\rb", r"a\\b", r"\u0041",
    r"caf\u00e9", r"a\ b", r"no\escape", r"a\qb", r"\u00e9", r"\ud83d\ude00",
    r"a\ud83d", r"\u0041\u0042", r"\t\u0041\n", "already plain", r"\\u0041",
    r"\u0000", r"\uFFFF", r"a\\", "\\",
]

# Inputs the reference rejects, as `\u` not followed by four hex digits.
UNESCAPE_ERROR_CASES = [r"\u12", r"\u", r"\uZZZZ", r"\u12g4", r"a\u12", r"\u123"]

rows = ["input\tunescaped"]
for s in UNESCAPE_CASES:
    rows.append("\t".join([escape_tsv(s), escape_tsv(crate_unescape(javaproperties.unescape(s)))]))
for s in UNESCAPE_ERROR_CASES:
    rows.append("\t".join([escape_tsv(s), "!error"]))
write("unescape.tsv", rows)

rows = ["input\tunescaped"]
for _ in range(2000):
    s = piece(8)
    try:
        expected = escape_tsv(crate_unescape(javaproperties.unescape(s)))
    except javaproperties.InvalidUEscapeError:
        expected = "!error"
    rows.append("\t".join([escape_tsv(s), expected]))
write("unescape-random.tsv", rows)
