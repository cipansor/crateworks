"""Build the XML-format fixtures for `crates/javaproperties`.

`gen.py` and `gen_writer.py` cover the line-based `.properties` format. This
covers the second format the crate now implements, `java.util.Properties`'
`loadFromXML` and `storeToXML`, which Python's `javaproperties` exposes as
`loads_xml`/`dumps_xml`.

Two fixtures are written:

* ``xml_dumps.tsv`` records the reference's own output for `dumps_xml` over a
  fixed set of pairs, comments and `sort_keys` values, so the writer is pinned
  from the reference rather than from our reading of the format.
* ``xml_loads.tsv`` records the pairs the reference reads from a set of
  documents, so the reader is pinned the same way.

Every expectation comes from `javaproperties` 0.8.2, which is tested against
Java. Nothing is hand-written.

Usage:
    python3 gen_xml.py [target.tsv]
"""

import javaproperties
import random
import sys

random.seed(20261027)

# --- Curated cases -----------------------------------------------------------
# Each pins a rule the reference's docstrings and `xmlprops` module state.

PAIRS = [
    [("a", "1")],
    [("a", "1"), ("b", "2")],
    [("b", "2"), ("a", "1")],
    [("name", "cipansor")],
    [("k", "a&b<c>d")],
    [("k", 'a"b\'c')],
    [("a b", "c d")],
    [("k\"q", "v")],
    [("k'q", "v")],
    [("both\"'x", "v")],
    [("tab\tkey", "v")],
    [("nl\nkey", "v")],
    [("cr\rkey", "v")],
    [("café", "中文")],
    [("", "x")],
    [("k", "")],
    [("dup", "1"), ("dup", "2")],
    [],
]

COMMENTS = [None, "", "hi", "one\ntwo", "a&b<c>d", "café"]

# Documents for the reader. The first few are the reference's own output, so a
# writer bug that also reads back is still caught by `dumps.tsv`.
DOCUMENTS = [
    "<properties><entry key=\"a\">1</entry></properties>",
    "<properties/>",
    "<properties></properties>",
    "<properties><entry key=\"a\">a&amp;b&lt;c&gt;d</entry></properties>",
    "<properties><entry key=\"k\">a&#38;b&#x3C;c</entry></properties>",
    "<properties><comment>ignored</comment><entry key=\"a\">1</entry></properties>",
    "<properties><entry key=\"a\">1</entry><entry key=\"a\">2</entry></properties>",
    "<properties><entry key=\"k\"/></properties>",
    "<properties><entry key=\"café\">中文</entry></properties>",
    "<properties><entry key=\"a b\">c d</entry></properties>",
    "<properties><entry key=\"k\" extra=\"x\">v</entry></properties>",
    "<!DOCTYPE properties SYSTEM \"http://java.sun.com/dtd/properties.dtd\">"
    "<properties><entry key=\"a\">1</entry></properties>",
    "<?xml version=\"1.0\" encoding=\"UTF-8\"?>"
    "<properties><entry key=\"a\">1</entry></properties>",
]


def escape_tsv(value: str) -> str:
    """Encode a string for a tab-separated fixture, like the other generators."""
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


TOKENS = [
    "", " ", "a", "b", "&", "<", ">", "\"", "'", "\t", "\n", "\r", "é", "中",
    "x" * 20, "k=v", "a&b", "café",
]


def piece(maxlen=5) -> str:
    return "".join(random.choice(TOKENS) for _ in range(random.randint(0, maxlen)))


def write(name: str, rows: list[str]) -> None:
    target = sys.argv[1] if len(sys.argv) > 1 else (
        "crates/javaproperties/tests/fixtures/xml.tsv"
    )
    path = target.replace("xml.tsv", name)
    with open(path, "w") as fp:
        fp.write("\n".join(rows) + "\n")
    print("wrote", len(rows) - 1, "cases to", path)


# --- dumps_xml ---------------------------------------------------------------
HEADER = "count\thas_comment\tcomment\tsort_keys\tdocument\tpairs"


def dumps_row(pairs, comment, sort_keys) -> str:
    document = javaproperties.dumps_xml(pairs, comment=comment, sort_keys=sort_keys)
    fields = [
        str(len(pairs)),
        "1" if comment is not None else "0",
        "" if comment is None else escape_tsv(comment),
        "1" if sort_keys else "0",
        escape_tsv(document),
    ]
    for key, value in pairs:
        fields.append(escape_tsv(key))
        fields.append(escape_tsv(value))
    return "\t".join(fields)


rows = [HEADER]
for pairs in PAIRS:
    for comment in COMMENTS:
        for sort_keys in [False, True]:
            rows.append(dumps_row(pairs, comment, sort_keys))
write("xml_dumps.tsv", rows)

# --- loads_xml ---------------------------------------------------------------
rows = ["document\tcount\tpairs"]
for document in DOCUMENTS:
    pairs = javaproperties.loads_xml(document, object_pairs_hook=list)
    fields = [escape_tsv(document), str(len(pairs))]
    for key, value in pairs:
        fields.append(escape_tsv(key))
        fields.append(escape_tsv(value))
    rows.append("\t".join(fields))

# Round-trip the reference's own writer output, which mixes every escaping rule.
for _ in range(500):
    pairs = [(piece(), piece()) for _ in range(random.randint(0, 4))]
    document = javaproperties.dumps_xml(pairs, sort_keys=random.choice([False, True]))
    read = javaproperties.loads_xml(document, object_pairs_hook=list)
    fields = [escape_tsv(document), str(len(read))]
    for key, value in read:
        fields.append(escape_tsv(key))
        fields.append(escape_tsv(value))
    rows.append("\t".join(fields))
write("xml_loads.tsv", rows)
