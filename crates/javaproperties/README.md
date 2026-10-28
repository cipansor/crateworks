# javaproperties

[![crates.io](https://img.shields.io/crates/v/javaproperties.svg)](https://crates.io/crates/javaproperties)
[![docs.rs](https://docs.rs/javaproperties/badge.svg)](https://docs.rs/javaproperties)

Reading Java `.properties` files.

This is the format behind `application.properties` in every Spring service and
`messages_id.properties` in every Java i18n bundle. It looks like a list of
`key=value` lines and is not: the rules are older than the format's common use,
and getting them wrong is quiet.

```rust
use javaproperties::parse;

let pairs = parse(b"name = cipansor\ncity\\:bandung = west java\n").unwrap();
assert_eq!(pairs[0], ("name".to_string(), "cipansor".to_string()));
// The escaped colon is part of the key, not a separator.
assert_eq!(pairs[1], ("city:bandung".to_string(), "west java".to_string()));
```

## Why this is not a five-line parser

Every rule below is one somebody gets wrong:

- A line whose first non-blank character is `#` or `!` is a comment.
- A key ends at the first unescaped `=`, `:`, or run of whitespace. `a\=b=c`
  splits into the key `a=b` and the value `c`; `a\\=b` splits after the doubled
  backslash into the key `a\` and the value `b`.
- Whitespace after the separator is not part of the value; whitespace at the end
  of the line is.
- A line ending in an odd number of backslashes continues onto the next, whose
  leading whitespace is dropped and nothing put in its place, so `one\` then
  `two` is `onetwo`.
- `\t`, `\n`, `\f`, `\r`, `\\` and `\uXXXX` are escapes. A backslash before any
  other character is dropped, so `\q` is `q`.
- A `\u` not followed by four hex digits is an error, not a literal `u`.
- Characters above the basic plane are written as a surrogate pair, and are
  rejoined into one character.
- Bytes are ISO-8859-1 unless you say otherwise. `parse` takes bytes and reads
  them that way; `parse_str` takes text you have already decoded. Reading a Java
  file as UTF-8 is the classic source of mojibake.

A key given twice keeps both entries in order, and `Properties::get` returns the
last, which is what Java does.

## Writing

`Properties::store` writes the document back, and it is the other half of the
parser rather than a formatter: the escapes are the ones `java.util.Properties.store`
uses, so what is written reads back as it was. That means a space in a key is
escaped, the first character of a value is escaped when it is a space, `#`, `!`,
`=` and `:` are escaped wherever they appear in a key or a value, and anything
outside printable ASCII becomes `\uXXXX` in upper case, the way Java writes it.
Nothing is wrapped: a long value stays on one line, as Java leaves it.

```rust
use javaproperties::Properties;

let properties = Properties::from_pairs(vec![
    ("a b".to_string(), "one".to_string()),
    ("café".to_string(), "two".to_string()),
]);
assert_eq!(properties.store(None), "a\\ b=one\ncaf\\u00E9=two\n");
```

`store_bytes` gives the ISO-8859-1 bytes Java writes to a stream, which are the
same because every escape is ASCII, and `store_to` writes them to anything
writable.

`store_with` is the rest of the reference's writer: the separator, a comment, a
timestamp comment, sorting, and whether non-ASCII is escaped. The pieces are
exported on their own too - `escape`, `join_key_value` and `to_comment` - because
they are useful without a whole document.

```rust
use javaproperties::{Properties, WriteOptions, join_key_value, to_comment};

// The three pieces, as the reference's `escape`, `join_key_value` and
// `to_comment`.
assert_eq!(join_key_value("a b", "c d", "="), "a\\ b=c d");
assert_eq!(to_comment("one\ntwo"), "#one\n#two");

let properties = Properties::from_pairs(vec![
    ("b".to_string(), "2".to_string()),
    ("a".to_string(), "1".to_string()),
]);
let text = properties.store_with(&WriteOptions {
    separator: ": ",
    sort_keys: true,
    timestamp: Some(0),
    ..WriteOptions::default()
});
assert_eq!(text, "#Thu Jan 01 00:00:00 UTC 1970\na: 1\nb: 2\n");
```

The one deliberate difference from the reference is the timestamp. Java renders
it in the local timezone, reading the host's; this crate has no timezone database
and takes no dependency, so it renders UTC and says so, which is why the function
is `java_timestamp_utc` rather than a silent `java_timestamp`.

## Decoding a single field

`unescape` is the decode half of the reader, and the reference exposes it as a
function of its own, so this crate does too. The reader applies it to every key
and value, but a caller who already has a field in hand does not need to parse a
document to decode it.

```rust
use javaproperties::unescape;

assert_eq!(unescape(r"caf\u00e9").unwrap(), "café");
assert_eq!(unescape(r"a\tb").unwrap(), "a\tb");
// A backslash before an ordinary character is dropped.
assert_eq!(unescape(r"a\qb").unwrap(), "aqb");
// A `\u` without four hex digits is an error, not a literal `u`.
assert!(unescape(r"\u12").is_err());
```

Two corners of the reference's standalone function are worth naming, because
they differ from what the reader ever produces and a hand-written test would
have got them wrong. A backslash with nothing after it stands for itself, so
`unescape(r"abc\")` is `abc\` - the reader never sees this, because a lone
trailing backslash is a line continuation and is joined away before any field is
decoded. And a backslash immediately before a line feed stays literal, because
the reference's escape pattern cannot match a newline; the reader does not see
that either, for the same reason. Matching the reference here is what the
fixture checks.

## The XML format

`java.util.Properties` has a second representation, reached through
`loadFromXML` and `storeToXML`, which carries the same key/value data as an XML
document. It is used where the data has to travel as XML: a configuration
fragment inside a larger document, or an attachment.

```rust
use javaproperties::{dumps_xml, loads_xml};

let xml = dumps_xml(
    &[("name".to_string(), "cipansor".to_string())],
    Some("a note"),
    false,
);
assert!(xml.starts_with("<!DOCTYPE properties"));
assert_eq!(
    loads_xml(&xml).unwrap(),
    vec![("name".to_string(), "cipansor".to_string())],
);
```

The shape is fixed: a `properties` root, an optional `comment` element, and one
`entry` element per pair, each with a `key` attribute. The escaping is XML's, not
the backslash escaping of the line-based format, so this is a separate module
with its own rules. A value escapes `&`, `<` and `>`; a key is quoted with
whichever of `'` and `"` the key does not contain, and a tab, newline or carriage
return in a key is written as a numeric reference so the reader cannot normalise
it away. The `Properties::from_xml` and `Properties::to_xml` methods are the same
two operations on the type.

## How it is checked

Against Python's `javaproperties` package, which replicates
`java.util.Properties` and is itself tested against Java. The corpus is a
hand-written set that pins each rule above, plus **six thousand random cases**
built from the format's own punctuation: separators, escapes, `\uXXXX` sequences,
line continuations, comments, all four line endings and the ISO-8859-1 default
all mixed into the same document. All 6,084 agree.

Writing is checked the same way, from the other side: **four thousand random
key/value pairs** are written by `javaproperties.dumps` and the documents it
produces are replayed against `Properties::store`. That is the side which found
the two disagreements above - the missing escapes for `#`, `!`, `=` and `:`, and
the line wrap that could cut a `\uXXXX` escape in half - because a corpus of
inputs can only ever check the reader.

The writer API added later is checked against the reference directly. A corpus of
inputs cannot check an escape function, so `scripts/fuzz/javaproperties/gen_writer.py`
records what the reference's own `escape`, `join_key_value`, `to_comment`,
`java_timestamp` and `dumps(separator=, comments=, sort_keys=, ensure_ascii=)`
return for a curated set and **two thousand random strings**, and the crate has to
produce the same bytes. That corpus immediately found two more rules the first
version of the writer had wrong: the reference sorts by key *and then value*, not
by key alone, so a key given twice is ordered by its value; and an empty comment
string writes a bare `#` line, which is not the same as no comment at all.

The standalone `unescape` is checked the same way, by
`gen_writer.py`'s `unescape.tsv` and `unescape-random.tsv`: the reference's own
answers for a curated set and **two thousand random fields**, with an invalid
`\u` recorded as the reference's `InvalidUEscapeError`. Making `unescape` public
is what made that gate possible, and it immediately found two corners the reader
could never reach and a hand-written test had therefore got wrong. A backslash
with nothing after it stands for itself rather than being dropped, so
`unescape(r"abc\")` is `abc\`; and a backslash immediately before a line feed
stays literal, because the reference's escape pattern cannot match a newline. The
one place the crate cannot agree is a *lone* surrogate: Rust text cannot hold
one, so it becomes U+FFFD, and the generator normalises that single case rather
than hiding it.

The random corpus is generated with a fixed seed by `scripts/fuzz/javaproperties/`,
so it can be rebuilt exactly,
and the cases where two rules meet are the ones it exists for: a key that ends in
a backslash just before a comment, a `\uXXXX` split across a continuation, a
separator that is also an escaped character. Those are where the obvious
implementation and the reference part ways.

The XML format is checked by `scripts/fuzz/javaproperties/gen_xml.py`, which asks
the reference's `loads_xml` and `dumps_xml` for their answers and records them:
**513 documents to read and 216 pairs to write**, every expectation the
reference's own. That corpus caught a rule the XML parser underneath this crate
had wrong and no other crate had exercised: XML 1.0 end-of-line handling, where a
literal carriage return - and a carriage-return/line-feed pair - is normalised to
a single line feed before anything else sees it, and the same whitespace in an
attribute value becomes a space. The fix is in `xmlkit`, and `xml_oracle.rs`
keeps it from coming back.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
