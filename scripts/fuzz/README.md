# Fuzz corpus generators

The fuzz corpora in `crates/*/tests/fixtures/fuzz.tsv` are not hand-written. Each
was produced by asking the reference implementation, in its own language, for the
answer to a few thousand inputs that a human would not think to try, and recording
it. This directory keeps the generator for each one, so a corpus can be rebuilt
and extended instead of guessed at.

They are development tools. Nothing here is compiled into a crate, run by CI, or
needed to build or test the workspace. A crate's dependency on the standard
library alone is unaffected.

## Why they are committed

A fixture without its generator is a dead end: when a case has to be added or a
seed changed, the only option left is to write a second generator that reproduces
the first, and it will not. Committing the generator makes the corpus
reproducible, and a corpus you can rebuild is one you can grow.

## What each one needs

Each generator drives a reference library that is not part of this repository, so
each needs that library installed before it will run. This is reference material,
never a dependency of any crate.

| Crate | Generator | Reference library | Run |
| --- | --- | --- | --- |
| `object-hash` | `object-hash/gen.js` | `object-hash` npm 3.0.0 | `NODE_PATH=<modules> node object-hash/gen.js <fuzz out> <objects out> [oracle to extend] [oracle to append objects to] [oracle to add sha512+base64 to] [oracle to add sha384 to]` |
| `adhan` | `adhan/gen.js` | `adhan` npm 4.4.6 | `NODE_PATH=<modules> node adhan/gen.js [out]` |
| `javaproperties` | `javaproperties/gen.py` | `javaproperties` PyPI 0.8.2 | `python3 javaproperties/gen.py [out]` |
| `javaproperties` (writer) | `javaproperties/gen_writer.py` | `javaproperties` PyPI 0.8.2 | `TZ=UTC python3 javaproperties/gen_writer.py [out]` |
| `javaproperties` (`unescape`) | `javaproperties/gen_writer.py` | `javaproperties` PyPI 0.8.2 | `TZ=UTC python3 javaproperties/gen_writer.py [out]` |
| `javaproperties` (XML) | `javaproperties/gen_xml.py` | `javaproperties` PyPI 0.8.2 | `python3 javaproperties/gen_xml.py [out]` |
| `iso6346` | `iso6346/gen.js` | `container-validator` npm 1.1.0 | `NODE_PATH=<modules> node iso6346/gen.js [out]` |
| `address-parser` | `address-parser/gen.js` | `parse-address` npm 1.1.2 | `NODE_PATH=<modules> node address-parser/gen.js [out]` |
| `eu-vat` | `eu-vat/gen.py` | `python-stdnum` PyPI 2.2 | `python3 eu-vat/gen.py [out]` |
| `iso8583-msg` | `iso8583-msg/gen.py` | `pyiso8583` PyPI 4.0.1 | `python3 iso8583-msg/gen.py [out]` |
| `factur-x` | `factur-x/gen.py` | `factur-x` PyPI 7.8 | `python3 factur-x/gen.py [out-dir]` |
| `pdfa` | `pdfa/gen.py` | `factur-x` PyPI 7.8 + `pypdf` | `python3 pdfa/gen.py [fixture-dir]` |
| `ebics` (RSA + response) | `ebics/gen.py` | `cryptography` PyPI | `python3 ebics/gen.py [fixture-dir]` |
| `ebics` (request) | `ebics/gen.js` | `ebics-client` npm 5.0.0 | `NODE_PATH=<modules> node ebics/gen.js [fixture-dir]` |
| `openlocationcode` | `openlocationcode/gen.py` | `openlocationcode` PyPI 1.0.1 | `python3 openlocationcode/gen.py [out]` |
| `proxy-addr` | `proxy-addr/gen.js` | `proxy-addr` npm 2.0.8 | `NODE_PATH=<modules> node proxy-addr/gen.js` |
| `xmlkit` | `xmlkit/gen.py` | Python `expat` (stdlib) | `python3 xmlkit/gen.py [out]` |
| `wssec` | `wssec/gen.py` | Python `hashlib` and `base64` (stdlib) | `python3 wssec/gen.py <fixture-dir>` |

With no `out` argument each writes to the crate's own fixture, which is the path
you want when rebuilding it in place. The recorded versions are the ones the
committed fixtures were built against; a newer reference may legitimately change
an answer, and then the crate has to change with it or say why not.

`object-hash/gen.js` and `javaproperties/gen.py` take the output path from an
argument or a constant; `adhan/gen.js` writes straight to the crate fixture. Note
the one thing the npm scripts need: `NODE_PATH` has to point at a `node_modules`
holding the reference package, because the script is run from a directory where
it is not installed.

`pdfa/gen.py` is the one generator that does not build random inputs. It rebuilds
a fixed pair of reference fixtures - a Factur-X container and its XMP packet -
from two committed inputs (`blank.pdf` and `reference_embedded.xml`) that do not
change, because the `pdfa` tests check the container as a whole rather than
answer a question per case. The reference stamps its output with the wall clock,
so the generator pins the two timestamp functions to a fixed instant to keep the
result reproducible.

## Determinism

Every generator uses a fixed seed, so the same script produces the same corpus
every time. That is the point: a fixture that changes on each run cannot be said
to be checked. `object-hash/gen.js` uses a small xorshift, `adhan/gen.js` a
linear congruential generator, `javaproperties/gen.py` Python's `random` with a
set seed.

The first version of the object-hash corpus used `Math.random`, which made it
unreproducible; it has since been replaced. If you change a seed you are changing
the corpus, so regenerate the fixture in the same commit and let the crate's test
confirm the crate still agrees.

## The lesson each corpus taught

These are here because the curated fixtures, chosen by hand, all passed while the
crate was still wrong. The value of a corpus is exactly the cases nobody thought
to write down.

- `object-hash` — 4,000 values, plus 2,000 plain objects: numbers past 1e21 and
  below 1e-6, strings above U+FFFF, the `Set`/`Map` deduplication rules, and the
  `object:` encoding a plain object gets that a `Map` does not. Found three
  divergences the curated cases missed, and the object/`Map` split is what the
  second corpus exists for. Plain objects are hashed with `respectType:false`,
  the only form reproducible outside JavaScript.
- `adhan` — 6,000 cases: latitudes near the poles, the manual adjustments, the
  shafaq and polar-circle options, and `Rounding::None`. Found three gaps the
  37,440-time grid missed, because the grid never set an adjustment and only used
  the default rounding. Widening the corpus to the shafaq and polar-circle
  options then showed both were missing from the crate altogether. A seventh
  column, the reference's `sunset`, showed a fourth: the reference reports
  `sunset` and `maghrib` as separate readings and a Maghrib angle or adjustment
  moves only the second, so a caller had no way to read the raw sunset.
- `javaproperties` — 6,000 documents built from the format's own punctuation.
  Found nothing, which is the useful outcome too: it is what turns "the rules we
  thought of are right" into "the rules we checked agree".
- `javaproperties` (writer) — 8,000 cases across `escape`, `join_key_value`,
  `to_comment` and `dumps(separator=, comments=, sort_keys=, ensure_ascii=)`. Found
  two rules the first version of the writer had wrong: the reference sorts by key
  *and then value*, and an empty comment string writes a bare `#` line rather than
  nothing. Neither is reachable from a corpus of inputs, which is why the writer
  needs its own generator.
- `javaproperties` (`unescape`) — 2,029 fields through the reference's standalone
  `unescape`, which this crate now exposes too. Found two corners the reader can
  never reach and a hand-written test had therefore got wrong: a lone trailing
  backslash stands for itself rather than being dropped, and a backslash
  immediately before a line feed stays literal because the reference's escape
  pattern cannot match a newline. Both were pinned by a unit test that asserted
  the wrong answer until the corpus disagreed with it.
- `iso6346` — 4,000 codes from every owner-prefix letter, all three categories and
  a serial across the whole range. Found nothing: the check digit is one page of
  arithmetic and the reference agrees with it on every case. The corpus is what
  makes that a measurement rather than a claim.
- `address-parser` — 3,959 addresses: the reference's own test table, generated
  street, PO-box, intersection and unit-word forms, and a dozen degenerate
  inputs. Found three divergences the curated cases missed: an intersection must
  always report `type1` and `type2` even when empty, a whitespace-only capture is
  a present-but-empty field rather than an absent one, and the direction rewrite
  in the city changes only the direction word, not the case of the rest.
- `iso8583-msg` — 500 random messages over the reference's own `default_ascii`
  specification. Found a real bug in the crate's field table the 400 curated
  cases had passed over: fields 28 to 31, the transaction and settlement fee
  amounts, were capped at eight digits while both ISO 8583:1987 and `pyiso8583`
  define them as nine. The generator also had to learn that field 65 is the
  extended bitmap rather than data, and that the crate models an empty value as
  "field absent" while the reference encodes a zero-length variable field — the
  corpus uses non-empty variable fields and the crate's README says why.
- `factur-x` — 500 documents (250 random invoices, each rendered as CII and as
  UBL) from the official `factur-x` package. Found a real feature gap the four
  hand-written fixtures had passed over: the reference renders a credit note as
  UBL's `CreditNote` root, with `CreditNoteTypeCode`, `CreditNoteLine` and
  `CreditedQuantity` where an invoice has `Invoice`, `InvoiceTypeCode`,
  `InvoiceLine` and `InvoicedQuantity`. The crate only ever wrote `<Invoice>`,
  so it could not read a credit note back and `detect_syntax` returned `None`
  for one. The gate parses each recorded document, regenerates it and compares
  the set of element paths, so it also holds the crate to the reference on which
  optional fields are written; the reference's own CII output is not valid
  against its XSD, so the generator renders with `check_xsd=False` and the crate
  models what the reference writes.
- `ebics` — not a corpus of random inputs but the four committed oracle fixtures,
  which were made with a reference and committed without a generator, so they
  could not be rebuilt. Two references are involved: Python's `cryptography` and
  `zlib` produced the RSA key fixture and the encrypted bank response, and the
  npm `ebics-client` produced the HPB request whose digest the crate reproduces.
  The generator had to learn that the request fixture's signature was over a key
  drawn fresh each run, which is why the fixture originally could not be checked
  end to end; pinning the key and committing the public half turns the
  `AuthSignature` into a second, independent check that the crate now passes.
- `proxy-addr` — 12,216 cases: the raw forwarded list over every peer and header
  shape, the trust resolution over random CIDR lists, and named corners for
  IPv4-mapped matching, netmasks and single-bit prefixes. Found one real gap the
  curated cases passed over: `ipaddr.js` accepts an IPv6 zone identifier
  (`fe80::1%eth0`) and ignores it when matching, which Rust's own parser rejects,
  so the crate was treating a zoned address as unparseable and ending the walk
  early. The crate now splits the zone off before parsing, and the corpus carries
  the shapes that would catch it again.
- `xmlkit` — 7,916 documents, each valid by construction or a single mutation of
  one, with the verdict taken from Python's `expat`. The crate first accepted
  nearly five hundred documents expat rejected; those were not one bug but a
  family, and closing them meant enforcing the grammar instead of reading what it
  could: names checked against `NameStartChar`/`NameChar`, whitespace required
  between attributes, `<` rejected in an attribute value, `--` and a trailing `-`
  rejected in a comment, an unescaped `]]>` rejected in content, and the XML
  declaration and DOCTYPE parsed rather than skipped. Two false-rejects the corpus
  found were real: a leading BOM was not stripped, and `Misc` after the root
  element was refused. The one deliberate leniency that remains - an ampersand
  that begins no entity is a literal `&`, the npm `@xmldom/xmldom` behaviour the
  SOAP and ISO 20022 consumers need - is normalised by the generator before expat
  is asked, so it is excluded from the count rather than hidden in it.

## Companion extractors

Some corpora need more than the inputs: the crate has to carry the reference's
own rules, and transcribing them by hand is how a port drifts.

| Crate | Script | What it extracts | Run |
| --- | --- | --- | --- |
| `address-parser` | `address-parser/extract_patterns.js` | the reference's pattern text and lookup tables, as Rust source | `NODE_PATH=<modules> node address-parser/extract_patterns.js <out.rs> <fixture.tsv>` |

`address-parser` is a parser, not a checksum, so its rules are a grammar rather
than a formula. The reference is a single backtracking regular expression with
named capture groups and lookahead, which Rust's `regex` crate cannot express.
Rather than translate the grammar into hand-written code, where a difference
from the reference would be invisible, `extract_patterns.js` rebuilds the
reference's *own* pattern strings and writes them into `src/patterns.rs`, and
the crate runs them on a small backtracking engine of its own. The script strips
JavaScript's extended (`x`) mode as it goes, and refuses to write the file unless
the stripped pattern accepts and rejects exactly what the reference does on every
case in the fixture.

