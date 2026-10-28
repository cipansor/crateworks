# Plan

The order in which the crates in this workspace were built, what each one
covers today, and what is deliberately left out. The dependency order matters:
`xmlkit` is the foundation that `iso20022-rs`, `soap-client`, `wssec`, `ebics`
and `factur-x` build on, and `pdfa` is the foundation for `pades`, so both had
to be correct before the crates above them could be.

## 1. `xmlkit` (foundation)

**Why first.** Both the ISO 20022 and SOAP crates need to read XML, and pulling
in a large parser would have made them heavy. This is a small, dependency-free
parser that keeps the tree, so a caller can inspect attributes and namespaces
directly.

**Covers today.** Parsing with attributes, namespaces, comments and CDATA;
text access (`text`, `deep_text`); escaping for text and attribute values;
round-trip serialisation; local-name matching for namespace-prefixed elements.

**Not covered.** Schema validation, XPath, streaming over documents larger than
memory. Those belong to a dedicated parser.

## 2. `holidays-rs`

**Why.** Working-day arithmetic is needed by HR systems, banks computing
settlement dates, and ticket systems computing SLAs. The existing `holidays`
crate has one release from 2023 and an abandoned repository.

**Covers today.** 45 countries, from Indonesia and the United States through
Europe (including the Orthodox-Easter calendars Cyprus, Greece and Romania, and
Malta) and Latin America (Mexico, Brazil, El Salvador, Nicaragua and Peru).
Fixed-date rules, Easter-relative rules via both the Gregorian and the
Orthodox computus, and *n*-th-weekday rules including "last Monday of the
month".
Weekend substitution per country, including the United States rule (Saturday
back to Friday) and the United Kingdom rule (both days forward to Monday), and
collision resolution so Christmas and Boxing Day never land on the same day.
Queries: `is_holiday`, `holiday_name`, `is_working_day`, `next_working_day`,
`nth_working_day_after`, `nth_working_day_before`, `working_days_between`.

**Not covered.** Islamic and Chinese-derived Indonesian observances (Idul Fitri,
Imlek, Nyepi) depend on a yearly government decree rather than a formula, so
they are added per year with `Calendar::with_extra`. Regional holidays (German
state holidays, United Kingdom local holidays) are out of scope. The reference
`holidays` package carries around 250 calendars; the remaining countries are the
obvious next step, and every one is checked against it by
`research/tools/verify_holidays.py`. `research/tools/derive_holidays.py` fits a
country to the `Rule` set and refuses to emit a table that disagrees with the
reference, and `research/tools/fit_holidays.py` ranks the remaining countries by
how well they fit, so the next additions are mechanical.

## 3. `stdnum-rs`

**Why.** Rust has per-country crates for single identifier types but no shared
entry point, and no crate that covers IBAN alongside barcodes and tax numbers.
The Python `python-stdnum` library covers 200+ types; this is the start of the
Rust equivalent.

**Covers today.** IBAN (78 countries, MOD-97, check-digit generation), ISBN-10
and ISBN-13 with conversion, EAN-8/EAN-13/UPC-A with UPC-to-EAN conversion,
Luhn, Damm and Verhoeff with check-digit generation, nineteen international
identifiers (ISIN, CUSIP, FIGI, LEI, BIC, GRID, ISSN, ISRC, IMEI, ISMN, ISNI,
CAS number, ISO 11649 creditor reference, IMO number, MEID, ISAN, a Bitcoin
address, a MAC address and an ISIL), and
**112 national identifiers across 90 countries**, reached through
`Country::ALL`. Every one of `python-stdnum`'s 89 national modules now has a
counterpart. The four newest need no registry: the IMO number is a seven-digit
hull identifier with a check digit, the MEID is a fourteen-digit hexadecimal
one, the ISAN carries its own check character over a hyphenated block, and the
Bitcoin address is verified by its Base58 double-SHA-256 or Bech32 checksum,
with the SHA-256 implemented inside the crate rather than pulled in. The MAC
address checks the shape, the zero padding and the address-class bits but
deliberately omits the IEEE manufacturer registry, and the ISIL checks the
registered agency prefix from the authority's published list.

**Not covered.** Anything that needs a registry lookup. A number that passes may
still be unissued; this is a form filter, not proof of existence.

## 4. `spreadsheet-formula`

**Why.** Spreadsheet formula engines in Rust are either abandoned or welded to
a file format. This crate takes a formula string and a cell resolver, so it
works with `xlsx`, `ods`, or a database of cells.

**Covers today.** Tokeniser with absolute references, sheet qualifiers, ranges
and dotted modern names (`NORM.S.DIST`, `CHISQ.INV.RT`); recursive-descent
parser with spreadsheet operator precedence; evaluator with 341 functions plus
`IF` and `IFERROR`, which are handled specially so they do not evaluate both
branches. A range size limit bounds the work a hostile formula can request. The
functions cover arithmetic and rounding, trigonometry and its hyperbolic and
reciprocal forms, logarithms, the gamma and error functions, number theory and
combinatorics, descriptive statistics and the statistical distributions
(normal, lognormal, Student's *t*, chi-square, *F*, binomial, Poisson,
exponential, Weibull, gamma, beta, hypergeometric and negative binomial, with
their inverses), regression and correlation, the hypothesis tests (`ZTEST`,
`TTEST`, `FTEST`, `CHITEST`, `PROB`), base conversion and bitwise operations,
roman numerals, text case, search and formatting, date and time (including the
`NETWORKDAYS.INTL` and `WORKDAY.INTL` weekend forms), complex numbers, financial
and depreciation calculations (including `RATE` and `COUPDAYS`), the
`AGGREGATE` selector, lookup and reference, `SUBTOTAL`, position-by-position
products, unit conversion (`CONVERT`, from a generated 146-unit table), and the
conditional aggregates (`SUMIFS`, `COUNTIFS`, `AVERAGEIFS`,
`MAXIFS`, `MINIFS`). The distributions and the tests are ported from
`@formulajs/formulajs`, which delegates to `jStat`, and
`research/tools/verify_spreadsheet.py` compares every one of them against it.

**Not covered.** Array formulas, volatile functions (`NOW`, `RAND`), and
iterative recalculation. Cyclic references are detected and reported rather than
resolved.

## 5. `hijridate`

**Why.** The Hijri calendar cannot be computed from a formula: a month begins at
the crescent sighting, so software uses a published table. Umm al-Qura is the
Saudi sighting-based calendar that most software treats as the reference. On
crates.io the only Hijri crates are `hijri` (a command-line tool, 11 recent
downloads) and `hijridate`-adjacent stubs; the name `hijridate` itself was free.
For comparison the PyPI package `hijri-converter` has millions of monthly
downloads, and `hijridate` (its successor) has more. This crate also fills a hole
this workspace had already documented: `holidays-rs` deliberately omits
Indonesian Idul Fitri and Idul Adha because they depend on a decree rather than a
formula, and the Umm al-Qura table is what those dates are looked up in.

**Covers today.** Hijri to Gregorian and Gregorian to Hijri across Hijri years
1343 to 1500 (Gregorian 1924 to 2077), taken from the published Umm al-Qura month
starts. Real month lengths (29 or 30, varying by year), month names, weekday and
day names, validation that rejects a day past the end of its month, and `Display`
as `YYYY-MM-DD`. A Gregorian date also carries the proleptic ordinal the
reference inherits from `datetime.date` - `toordinal`, `fromordinal` and a
component-wise `replace` - pinned to the reference by a fixture. No dependencies.

**Not covered.** Outside the table the crate falls back to the classical
arithmetic calendar and flags the result with `is_tabular`; those dates can be a
day or two out. The month, day and era names are available in English, Arabic,
Bengali and Turkish through `Language`; there is no time-of-day or prayer-time
handling. Most importantly
the crate is one authority, not every authority: Indonesia (Kemenag), Malaysia
and others announce their own month starts, so a date here can differ by a day
from what a country observes. That is a property of the calendar, not a bug.

**How the data is trusted.** The table and the test fixtures are both generated
from the MIT-licensed `hijridate` Python package, so the tests compare against an
independent implementation rather than against this crate's own output. See
`research/tools/gen_hijri_table.py` and the crate `NOTICE` for provenance.

## 6. `iso20022-rs`

**Why.** ISO 20022 is replacing SWIFT MT, and every bank integration needs it.
The only crate on crates.io is from 2020 and unmaintained.

**Covers today.** `camt.053` bank statements: account, balances, entries with
credit/debit sign, counterparty, remittance and value date; closing balance
either as reported or computed from the entries. `pain.001` payment initiation:
read and generate, with the control sum and transaction count computed from the
transfers so the output cannot disagree with itself.

**Not covered.** `pacs` clearing messages, other `camt` variants, schema
validation, and the full generated type set. Amounts use `f64` with the original
decimal text preserved in `Amount::text`.

## 7. `soap-client`

**Why.** SOAP remains the only interface for many banks, government systems and
enterprise products, and every SOAP crate on crates.io has zero reverse
dependencies. The widest gap, and the least proven Rust demand.

**Covers today.** SOAP 1.1 and 1.2 envelopes: build a request, parse a
response, detect and read a fault in either version's shape. WSDL reading:
endpoint address, port types, operations with their input and output messages,
and the transport binding.

**Not covered.** The HTTP call itself, WS-Security, and typed code generation
from a WSDL. Those are deliberate omissions: the HTTP layer belongs to the
caller's client, and WS-Security needs a certificate story that should not be
guessed at.

## 8. `iso6346`

**Why.** A shipping container code carries a check digit, and the industry
exchanges them in volume: the `container-validator` npm package is downloaded
around twenty thousand times a month. crates.io has no crate for the standard at
all - searching the name returns nothing - so the field is JavaScript and PHP.

**Covers today.** Parsing a code with case and separators ignored, validating its
check digit, generating the check digit for a new code, and reading back the
owner code, category and serial. The category is typed, so `U`, `J` and `Z` are
enum variants rather than a char the caller has to match.

**Not covered.** Whether an owner code is registered with the BIC. That register
is a dataset that changes, and a crate shipping a copy would be wrong within a
month; the code can be well formed and still name an operator that does not
exist.

## 9. `eu-vat`

**Why.** VAT number validation is a form field in every European B2B flow, and
the `vat` crate stopped in 2016 with nothing replacing it. The rules are per
country, so the work is in the national details rather than in the shape.

**Covers today.** All 27 member states plus Northern Ireland (`XI`), each with
its own shape and check digit: Belgium modulo 97, Germany ISO 7064 Mod 11,10,
Italy the province of residence, Latvia a birth date in the personal code.
Greece accepts both `EL` and `GR` and canonicalises to `EL`. Separators are
tolerated and stripped.

**Not covered.** Whether a number is live. That needs the Commission's VIES
service, and there is no network access in the crate. A number that passes may
still be deregistered.

**How the data is trusted.** The rules were differential-tested against
`python-stdnum` over generated inputs, which found six real check-digit bugs;
see `crates/eu-vat/NOTICE`.

## 10. `iso8583-msg`

**Why.** ISO 8583 is the format card switches use for ATM, EDC and POS traffic,
and `pyiso8583` is downloaded around 400,000 times a month. The Rust crates in
the space are all abandoned.

**Covers today.** The message type indicator, the bitmap chain, fixed and
variable-length fields, ASCII, BCD, binary and EBCDIC code page 500 data
encodings, and length prefixes in ASCII, BCD or binary. A `Spec` is data, so a
switch's dialect is configured rather than compiled.

**Not covered.** Transport, MAC verification and PIN block handling. Field
contents stay opaque strings: what a processing code means is per-switch, so no
business-rule validation is attempted.

## 11. `arabic`

**Why.** Terminals, older renderers and PDF generators expect Arabic to arrive
already shaped. The Rust reshaping crates have under ten thousand downloads
between them, and shaping is easy to get subtly wrong at the joins.

**Covers today.** Contextual presentation forms for each letter in logical order
or reversed for display, the lam-alef ligatures and the word Allah, harakat
carried with their letter or dropped, and a separate `normalize` for a matching
key. No dependencies.

**Not covered.** Full bidi layout and hundreds of rare ligatures that Unicode
defines but most fonts cannot render.

## 12. `factur-x`

**Why.** Factur-X and ZUGFeRD are the European hybrid e-invoice, and the PyPI
package is downloaded hundreds of thousands of times a month. The Rust crates in
the area have a few thousand downloads and model only one syntax.

**Covers today.** EN 16931 modelled once and written or read as either CII or
UBL, with `detect_syntax` to tell them apart, and `validate` for the standard's
arithmetic rules (`BR-01`..`BR-06`, `BR-09`, `BR-16`, `BR-CO-10`..`BR-CO-18`,
`BR-S-08`). Fields carry the standard's own numbers (`BT-1`, `BG-25`).

**Not covered.** Embedding the XML in a PDF - that is `pdfa` - and code list
validation and country profiles such as XRechnung, which extend EN 16931.

## 13. `wssec`

**Why.** The missing half of `soap-client`: almost every bank and government
SOAP endpoint rejects a request with no `Security` header, and the only attempt
on crates.io has eleven downloads.

**Covers today.** Username tokens with the password digest, timestamps, and
reading a header back so a password can be checked without holding it. The
nonce is treated as the raw bytes the specification digests, not the base64
text, which is where `zeep` gets it wrong; `digest_with_text_nonce` reproduces
that behaviour for the rare server written to match it.

**Not covered.** `Signature` and `EncryptedData`. Those need a certificate story
this crate deliberately leaves alone. It also does not generate a nonce or read
a clock, so the caller records the timestamp and a test can reproduce it.

## 14. `pdfa`

**Why.** The EN 16931 XML is not what a tax authority accepts: it has to travel
inside a PDF/A-3 document as an attachment, with metadata that says so. The Rust
PDF writers have no notion of conformance, so this is the other missing half.

**Covers today.** Reading all three object layouts (plain `xref` table,
cross-reference stream, object stream, predictors included), full `FlateDecode`,
and building the Factur-X container with the checksum and `/AFRelationship` that
PDF/A-3 requires. `check_factur_x` verifies the container contract.

**Not covered.** The full ISO 19005 rules. Device-independent colour, every font
embedded, no encryption, transparency: that is veraPDF's job, and a small crate
claiming it would be lying. The report says exactly what was checked.

## 15. `adhan`

**Why.** Prayer times are arithmetic every prayer app and mosque schedule needs,
and it sits beside `hijridate` and `arabic`. The Rust space is split across an
848-download crate and a 2023 stub.

**Covers today.** Fajr, sunrise, Dhuhr, Asr, Maghrib and Isha from a date and
coordinates, thirteen calculation methods, both Asr schools, three high latitude
rules, the Moonsighting Committee seasonal curve, qibla and the sunnah times.
`Rounding::None` keeps the seconds the reference produces. A time that does not
exist inside the polar circles is `None`, as the reference reports it.

**Not covered.** Local time. The result is UTC and the caller adds the offset,
because a place can be an hour or more from its solar time.

**How the data is trusted.** Checked against the JavaScript `adhan` library, not
its Python port, which is wrong for eastern longitudes.

## 16. `object-hash`

**Why.** npm `object-hash` is downloaded over 40 million times a week, the name
was free, and the only Rust equivalent had 58 recent downloads. `std::hash::Hash`
cannot do the job: `RandomState` is seeded per process and a `HashMap` iterates
in an unspecified order.

**Covers today.** A canonical form identical to the npm package's byte for byte,
so a Rust service and a Node service agree on a cache key. SHA-1 by default, with
SHA-256 and MD5 selectable; all three are written out here, so the crate still
has no dependencies. Number formatting follows the ECMAScript rules rather than
Rust's `Display`, and entries sort by UTF-16 code unit.

**Not covered.** JavaScript prototypes and constructors, which the reference
mixes into its hash. A Rust caller has no prototype to respect, so `Value::Map`
is the entries and nothing else. The digests are not security primitives.

## 17. `javaproperties`

**Why.** The format behind `application.properties` in every Spring service, and
npm `properties` is downloaded 300,000 times a week. The Rust `java-properties`
has not moved since 2023. It looks like `key=value` lines and is not.

**Covers today.** Reading and writing, with the escapes, continuations, comment
rules and ISO-8859-1 default that `java.util.Properties` uses. `Properties::store`
writes what Java writes, including `\uXXXX` for non-ASCII and the escapes for
`#`, `!`, `=` and `:`.

**Not covered.** Nothing of the format is omitted; the crate is the reader and
the writer rather than a formatter.

## 18. `mllp`

**Why.** MLLP is how almost every HL7 v2 message travels over TCP, and PyPI
`hl7apy` is downloaded 758,000 times a month. The Rust HL7 crates are leaderless
and none frames a stream.

**Covers today.** Framing and unframing, plus a streaming `Decoder` that
reassembles frames from reads that do not line up with them. The terminator is
the pair `0x1C 0x0D`, never a lone `0x0D`, because HL7 v2 ends every segment with
a carriage return. A message can be read into its segments and fields with
`Message`, and answered with `ack::build`, which writes the `MSA` acknowledgement
an interface must send back - swapping the sender and receiver and echoing the
control id, in the delimiters the received message used. `verify_mllp.py` checks
both the parse and the reply against `hl7apy`.

**Not covered.** The enhanced acknowledgement mode (the `ERR` segment and
`MSA-4`), validation against a message profile, opening sockets, TLS, and the
newer block-based MLLP over HTTPS. A frame that never terminates stops at 16 MiB
rather than growing the buffer without bound.

## 19. `ebics`

**Why.** EBICS is how a European company sends payment files to its bank, and
there is no Rust implementation at all: the field is Java, Python and TypeScript.

**Covers today.** The H004 messages and the cryptography a client needs, all
written here rather than pulled in: RSA PKCS#1 v1.5 and PSS, AES-128-CBC, SHA-256,
DEFLATE, base64, a bignum, and the canonical XML the header digest is taken over.
Each layer is checked against an independent implementation, and the header
digest reproduces byte for byte what `node-ebics-client` put in its own message.

**Not covered.** H005. The return-code table is transcribed from both
specifications, but the message layer implements H004 only.

## 20. `pdfmake`

**Why.** The JavaScript `pdfmake` model has no Rust equivalent: the Rust PDF
crates are low-level writers or full layout engines, with nothing in between. A
generated invoice is data, and building it should not mean concatenating PDF
operators by hand.

**Covers today.** Text, columns, lists and tables as a Rust value, with text
wrapping, a table header redrawn on every page, and page numbers resolved after
the last page is laid out. The 14 standard fonts are used unembedded, which is
what makes a dependency-free PDF possible.

**Not covered.** Font embedding, so text is WinAnsi and a character it cannot
represent becomes `?`, with the count recorded in `/Info`. No encryption, no
signing, no images, no justified text or hyphenation. Reading a PDF is `pdfa`.

## 21. `pades`

**Why.** Three Rust crates produce PAdES signatures and none verifies one. A
verifier needs the opposite of what a signer does, and it is the smaller and less
crowded problem.

**Covers today.** Reading a signed PDF and reporting, per signature, the field
name, `/SubFilter`, the digest algorithm read from the CMS, the `/ByteRange` and
what it covers, whether the covered bytes still hash correctly, the signer
subject and validity window, whether an RFC 3161 timestamp is embedded, the
revocation material in the DSS, and the highest baseline level reached.

**Not covered.** Building a certification path. Full RFC 5280 validation,
revocation fetching and policy processing is a large subject, and the crate is
network-free; it reports the chain and the material present and the caller
decides trust by passing anchors. Producing a signature is out of scope.

## 22. `aviation-data`

**Why.** Airport reference data is asked for constantly and served badly in Rust.
The crate with the volume is `airports` (about 875,000 recent downloads), which
maps an IATA code to a timezone and stops there; the only crate carrying a
complete table is unadopted and licensed CC-BY-4.0, so it cannot be used from a
permissive project. The data itself, OurAirports, is public domain, so a
permissively licensed crate can ship it.

**Covers today.** The whole OurAirports table compiled in: 11,889 airports, every
non-closed row with an IATA or ICAO code. Lookup by IATA, ICAO, GPS, local code
and `ident`, case-insensitively, with a documented precedence between the code
spaces. Filtering by country, continent and kind; search by name or municipality;
`distance_km` between two airports and `near` for the airports within a radius,
by the haversine formula.

**Not covered.** Live data. The table is a snapshot regenerated by
`research/tools/gen_aviation.py`, pinned at 11,889 rows so a silent upstream
change fails the generator rather than shrinking the crate. The 60,000 rows with
no IATA or ICAO code (small private fields, heliports, closed strips) are dropped
because no lookup can address them. `search` is a plain substring match with no
ranking or fuzzy matching. Runways, navaids, frequencies and country/region
metadata are in other OurAirports files and are not shipped.

## 23. `cssprint`

**Why.** Turning HTML and CSS into a paginated document is asked for constantly
and served badly in Rust. WeasyPrint on PyPI and `pdfmake` on npm (1,864,382
downloads a week) carry the demand; on crates.io the six crates that attempt it
are all under 5,100 recent downloads, and none has a layout engine complete
enough to have won an audience. The usual answer is to drive a headless browser,
which means shipping a Chromium binary and accepting that a document may lay out
differently on a different version.

**Covers today.** The CSS 2.1 block formatting context and inline layout. A
block box fills its containing block unless `width` says otherwise and each later
sibling goes below the previous one. `width`, `height`, `min-width`, `max-width`,
`margin`, `padding` and `border` are resolved; `auto` margins are distributed,
percentages resolve against the containing block, adjacent vertical margins
collapse including through a parent, and `display: none` removes a subtree.
Text and inline elements are gathered into line boxes, wrapped to the block's
width, and placed on a shared baseline, with whitespace collapsed, a space at a
line break removed, and `font-size` and `line-height` inherited. Styles come from
a stylesheet and an inline `style` attribute, with the attribute winning.

Text is measured for one font: the monospace font the checked corpus uses, where
every glyph advances exactly one em and the ascent is 819/1024 em. A word's width
is its character count times the font size, so the fixture is arithmetic. A
proportional font would need a metrics source, which is future work.

**Not covered.** Floats, `position`, tables, flex, grid and pagination.
`white-space: pre`, `br`, and `text-align` are absent, as are descendant
selectors, pseudo-classes and at-rules.

**How it is checked.** Against WeasyPrint's own box tree, not against rendered
PDFs. `tests/fixtures/layout.tsv` holds the tree WeasyPrint produces for each
document in `tests/fixtures/cases.tsv`, generated by
`research/tools/gen_weasyprint_layout.py`; `tests/layout.rs` lays out the same
document and compares box by box on position, size, margins, padding and border
widths. The oracle is the reference's output rather than a transcription of it.
The generator loads the reference's own test font, which is derived from Ahem and
is monospace, so the text cases are arithmetic and the fixture is identical on
every machine. This is what
the fixture caught while the crate was being written: the page box being
confused with `html`, the missing synthesised `body`, the collapse rule
attributing a margin to the wrong box, WeasyPrint's departure from CSS 2.1 where
an over-constrained block keeps both margins instead of adjusting the right one,
`line-height: 2` being read as two pixels rather than twice the font size, an
inline box's child offsets being taken as absolute instead of relative to the
box they go into, and the half-leading a box reports as its top and bottom
margin.

## Publishing order

Each crate carries its own version, and a release publishes only the crates that
changed since their last release. `scripts/release_plan.py` reports which those
are and `scripts/bump.py <crate> <level>` bumps one of them; a crate nobody
changed is not a release, because a version on crates.io can never be deleted.
This is a change from the shared `[workspace.package] version` the workspace used
to carry, under which every release republished every crate.

`cargo package` resolves a `path` dependency against the registry, so a crate
that depends on another crate in this workspace cannot be packaged until that
dependency is published. Publish bottom-up. The check is `cargo publish
--dry-run -p <crate>`: it fails with "no matching package named `xmlkit` found"
for every crate in the second group until `xmlkit` is on the registry. There is
no flag that gets around this, `--no-verify` included, because the failure is in
resolving the dependency rather than in building it.

1. `xmlkit`
2. `iso20022-rs`, `soap-client`, `wssec`, `ebics`, `factur-x`, `pdfa`
   (each depends on `xmlkit`)
3. `pades` (depends on `pdfa`)
4. Everything else is independent and can go in any order: `holidays-rs`,
   `hijridate`, `stdnum-rs`, `spreadsheet-formula`, `eu-vat`, `iso8583-msg`,
   `arabic`, `adhan`, `object-hash`, `javaproperties`, `mllp`, `pdfmake`,
   `iso6346`, `fuzzy-score`, `address-parser`, `aviation-data`, `cssprint`.

The other rule is that a published version cannot be overwritten. `0.1.0` is
spent the moment it is uploaded, so a mistake in the first upload of a crate
costs the `0.1.0` number and the fix goes out as `0.1.1`. Check with
`cargo publish --dry-run` first. This is also why a crate nobody changed must not
be released: the version it would spend is permanent.

`scripts/publish.py` works out the order from the manifests and runs it:

```sh
python3 scripts/publish.py            # dry-run every crate, upload nothing
python3 scripts/publish.py --strict   # same, but exit 1 if anything is blocked
python3 scripts/publish.py --publish  # upload, in order, stopping at the first failure
```

## Verification

The counts below are for the whole workspace, all 26 crates, and they drift as
crates are added; they are here as an order of magnitude, not as a contract.

```sh
cargo test --workspace --all-targets      # 1367 unit and README tests
cargo test --workspace --doc              # 194 doctests, not covered by --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all -- --check
```

`rust-version` is 1.88 and CI has a second job that compiles, lints and runs the
doctests on exactly that toolchain, so the floor is checked rather than asserted.
