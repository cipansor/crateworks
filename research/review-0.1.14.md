# Review of every crate against its mature comparison package (2026-10-31)

This is the follow-up to `review-0.1.13.md`. That round re-asked "of what we
have, where does a mature package in another ecosystem do more, and is that
difference worth closing", closed the `factur-x` profile reader, and recorded
the remaining candidates with the reason each stays out.

This round re-runs every gate first, then diffs each crate's public surface
against its reference once more. Three candidates came out of that pass and were
closed: the `object-hash` SHA-3 family, which `review-0.1.13.md` itself recorded
as the npm reference's supported-but-unported set; the `pacs.002` payment status
report, the `iso20022-rs` gap its own README named; and, when the first draft's
claims about Node's hash list were checked and found false, the eight remaining
Node `crypto` digests the reference offers and the crate did not (`ripemd160`,
`sm3`, `sha512-224`, `sha512-256`, `blake2b512`, `blake2s256`, `shake128`,
`shake256`). With those, every name in `crypto.getHashes()` is either computed
here or an alias for one that is. The other candidates that the diff surfaced are
recorded below with the reason each stays out.

## Correctness first: the differential gates were re-run

Every gate was re-run before any change. All were green on entry.

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_holidays.py` | PyPI `holidays` | 0 undocumented defects, 86 calendars |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` | 85,713 vectors agree |
| `verify_spreadsheet.py` | npm `@formulajs/formulajs` | 1,068/1,068 cases agree |
| `verify_aviation.py` | OurAirports `airports.csv` | 11,887 rows and 46,456 lookups agree |
| `verify_arabic.py` | PyPI `arabic-reshaper` 3.0.0 | 6,558/6,558 agree (per language) |
| `verify_hijri_locales.py` | PyPI `hijridate` 2.6.0 | 132/132 names agree |
| `verify_fuzzy_score.py` | PyPI `fuzzywuzzy` 0.18.0 | 0 mismatches, 5 process functions |
| `verify_units.py` | PyPI `pint` | 973 compared, 0 mismatches |
| `verify_mllp.py` | PyPI `hl7apy` | 135/135 fields, 8/8 ACK, 8/8 enhanced ACK replies agree |
| `verify_openlocationcode.py` | PyPI `openlocationcode` 1.0.1 | 1,389 rows agree |
| `verify_iso20022.py` | the official ISO 20022 XSD | generated pacs.008, pain.001 **and pacs.002** validate |
| `verify_proxy_addr.py` | npm `proxy-addr` 2.0.8 | 12,216 rows agree |
| `tests/oracle.rs` (fast-redact) | npm `fast-redact` 3.5.0 | 6,000 redactions and 90 path verdicts agree |
| `tests/{fuzz,oracle}.rs` (object-hash) | npm `object-hash` 3.0.0 | 4,000 random values and 66 curated cases agree, all sixteen digests |

No gate regressed.

## What this round develops

### `object-hash` 0.4.1 -> 0.5.0, the SHA-3 family

The npm reference's `algorithm` option accepts anything in Node's
`crypto.getHashes()`. `review-0.1.13.md` recorded the SHA-3 and
`shake`/`ripemd`/`sm3` families as "not in this crate", with the reason that
adding a digest means adding a hash implementation by hand because the workspace
forbids dependencies. That reason explains the cost, not the omission; the cost
is the same for every digest the crate already carries by hand. SHA-3 is the one
a Node service that has moved off SHA-1 is most likely to name, so it is the one
worth paying for.

The port adds four `Algorithm` variants - `Sha3_256`, `Sha3_224`, `Sha3_512`,
`Sha3_384`, matching the reference's `sha3-256` and siblings - and one new
module, `src/sha3.rs`. The four share a single Keccak-f[1600] permutation,
differing only in the rate (136, 144, 72, 104 bytes) and the output length, so
they are one sponge with four entry points rather than four copies. Padding uses
the FIPS 202 `0x06` domain-separation byte, not the original Keccak `0x01`.

**How it is trusted.** Three ways:

* The published FIPS 202 vectors for `abc` and the empty string, in `src/sha3.rs`.
* The lengths on either side of the SHA3-256 and SHA3-224 rate (135-145 bytes),
  where a message fills a block exactly and the padding starts a fresh one - the
  one place an off-by-one in the pad byte would show and nowhere else.
* The `object-hash` npm reference. The oracle fixture gains four columns,
  `sha3_256 sha3_224 sha3_512 sha3_384`, taken from Node's own `crypto` (which is
  where the reference gets them), for all 66 curated cases; the generator
  `scripts/fuzz/object-hash/gen.js` gains the matching `extendedSha3` step so the
  columns can be rebuilt. All 66 rows agree.

### `object-hash` 0.5.0 -> 0.6.0, the rest of Node's `crypto` digests

The section above first recorded `ripemd160` and `sm3` as staying out, and
`shake128`/`shake256` as unrepresentable, on two claims that were both wrong (see
"the candidates the diff surfaced" below). The reference's `algorithm` option
accepts any name in `crypto.getHashes()`, and eight of those names are digests
this crate did not compute: `ripemd160`, `sm3`, `sha512-224`, `sha512-256`,
`blake2b512`, `blake2s256`, `shake128` and `shake256`. A Node service naming any
of them could not match a Rust one - exactly the gap the crate exists to close.
They are now `Algorithm::Ripemd160`, `Sm3`, `Sha512_224`, `Sha512_256`,
`Blake2b512`, `Blake2s256`, `Shake128` and `Shake256`.

Two are truncations of a function already here: SHA-512/224 and SHA-512/256 are
SHA-512 with their own FIPS 180-4 initial states and a shortened output, so they
add two `digest512_224`/`digest512_256` entry points to `src/sha512.rs` and no
new round code. Two more share the sponge `src/sha3.rs` already had: SHAKE128 and
SHAKE256 are that sponge with the FIPS 202 `0x1f` domain-separation byte instead
of `0x06` and their own rate, so the pad byte becomes a parameter and the four
SHA-3 entry points are joined by two SHAKE ones. The remaining four are their own
constructions and got their own modules: `src/ripemd160.rs` (two parallel lines
over a block, five boolean functions, combined at the end), `src/sm3.rs`
(SHA-256's shape with its own message expansion and constants) and
`src/blake2.rs` (the ChaCha-derived ARX hash, in both its 64-bit/128-byte and
32-bit/64-byte widths).

SHAKE's output length is a caller parameter in FIPS 202, but the reference's
`algorithm` is a fixed string with no place to put one, so both sides use Node's
default for the bare name - 16 bytes for `shake128`, 32 for `shake256`. That
fixed-length reading is what makes them gateable like any other digest.

**How it is trusted.** Each is checked against its published vectors and against
the reference over the whole corpus:

* Published vectors in the module: RFC 7693 for BLAKE2b and BLAKE2s, the
  RIPEMD-160 paper for the empty string, `abc` and one million `a`s,
  GB/T 32905-2016 for SM3, and FIPS 202 for SHAKE. The exact-block cases (a full
  BLAKE2 block, a SHAKE message filling its rate) are pinned too, because that is
  where a wrong `chunks` split drops the final flag or the padding starts a fresh
  block.
* The `object-hash` reference. The curated oracle and the 4,000-row fuzz corpus
  each gain eight columns, taken from Node's `crypto`, and the generator gains an
  `extendedRare` step that rebuilds them.

This is the part worth recording. The published vectors passed for RIPEMD-160
while two whole rows of its step tables were still wrong, and BLAKE2's empty and
`abc` vectors passed while its partial-block handling was wrong - each a
transcription slip that a two-message vector set cannot see. The 4,000-length
corpus caught both. It is the same lesson as the round-25 capability search: the
answer comes from asking the reference, not from the two cases that were thought
of.

### `iso20022-rs` 0.1.9 -> 0.2.0, the `pacs.002` payment status report

The crate's own README said `pacs` clearing messages were not covered, and only
`pacs.008` was. A `pacs.008` is not known to have succeeded until its `pacs.002`
comes back, so the crate could send a payment and not read the answer - the other
half of the job.

`PaymentStatusReport` models `pacs.002.001.10`: the group header with optional
instructing and instructed agents, the group-level `OrgnlGrpInfAndSts` (the
message being reported on, the count and sum it claimed, the group status and its
reasons), and the per-transaction `TxInfAndSts` (the original identifiers
including the UETR, the transaction status, its reasons, and the acceptance,
servicer and clearing references). It parses with `parse` and writes with
`to_xml`, and round-trips.

Status codes (`TxSts`, `GrpSts`) and reason codes (`Rsn/Cd`) are kept as
`String`s rather than mapped to enums: the ISO 20022 external code lists are
large and grow without a schema change, so an enum would be stale on arrival and
would reject a code the schema still allows. This is stated in the README.

**How it is trusted.** The existing `verify_iso20022.py` gate gains a `pacs002`
case and a full fixture; the crate builds the document and the official
`pacs.002.001.10.xsd` validates it. `research/tools/verify_iso20022.py pacs002`
prints `VALID`. `crates/iso20022-rs/src/pacs.rs` adds nine round-trip and edge
tests (a proprietary reason, a group-level reason, the empty-report error, text
escaping, the namespace).

## Coverage: the candidates the diff surfaced and why they stay out

The public-surface diff against each reference left the following candidates.
None is a gap left by oversight; each is recorded so the next round does not
re-open it.

**`object-hash`: the remaining digests.** The first draft of this section was
wrong and is corrected here. It claimed `ripemd160` and `sm3` are "not in Node's
default TLS build on every platform, so the reference does not reliably offer
them either". That is false: Node's `crypto.getHashes()` lists `ripemd160` and
`sm3` (and `sha512-224`, `sha512-256`, `blake2b512`, `blake2s256`) on the
toolchain the reference is measured against, and the reference's `algorithm`
option accepts any name there. The six digests are therefore in the reference
and were absent here - a real gap, not a footnote - and this round closed them;
see the section below.

`shake128`/`shake256` were the first draft's second error, and are also now
closed. The draft claimed the reference "cannot ask for a specific `shake`
output" because its `algorithm` is a fixed string. That is true and beside the
point: the reference does not have to ask for a length, because Node supplies a
default for the bare name, and the reference returns a fixed 16-byte or 32-byte
digest like any other algorithm. Measured directly,
`object-hash(x, { algorithm: 'shake128' })` returns a 32-hex-character digest and
`'shake256'` a 64-hex-character one. So SHAKE was gateable all along and is now
covered too; see the section above. Nothing in Node's `crypto.getHashes()` is
left unrepresented: every other name there is an alias for a digest already
here (`ripemd`/`rmd160` for `ripemd160`, `ssl3-sha1` for `sha1`, and so on) or a
signature scheme, not a digest.

**`spreadsheet-formula`: the dynamic-array and volatile family.** The volatile
functions (`NOW`, `RAND`, `TODAY`) are non-deterministic and so cannot be gated
against a fixture; the dynamic-array and matrix family (`MMULT`, `TRANSPOSE`,
`HSTACK`, `VSTACK`, `DROP`, `TAKE`, `SORT`, `UNIQUE`, `LINEST`, `TREND`) returns
array objects this scalar evaluator does not model.

**`holidays-rs`: the remaining countries.** The reference carries about 250
calendars; the crate has 86. Every remaining country fails the fit for the same
reason: its reference table shifts each holiday independently, while the crate
holds one crate-wide `SubstituteStyle` per country. The countries that fit have
all been added.

**`stdnum-rs`: the six missing modules.** An earlier draft of this paragraph was
wrong about which modules are missing, and is corrected here. `python-stdnum` has
six modules the crate's public surface does not name: `Casrn`, `Cfi`, `Imsi`,
`Iso11649`, `Iso9362` and `Vatin`. Two of those are false gaps - the crate
already covers them under another name, and a name-based diff had missed it.
`Casrn` is the crate's `CasNumber` (CAS registry numbers, same check digit) and
`Iso11649` is its `CreditorReference` (the RF creditor reference, same
mod-97-10); `Iso9362` is its `Bic`. Only three are genuinely absent: `Cfi`,
`Imsi` and `Vatin`.

All three are blocked by data, not by algorithm. `Cfi` (ISO 10962) needs the
classification table; `Imsi` needs the MCC/MNC database (`split`/`validate`
reject an unknown MCC, so a table-free IMSI would accept numbers the reference
rejects and fail its own gate); `Vatin` is a dispatcher over per-country VAT
modules, and the crate's per-country VAT lives in the separate `eu-vat` crate.
Those tables ship in `python-stdnum` under LGPL-2.1+, which cannot be copied into
an Apache-2.0 crate, so each is shelved rather than shipped. The distinction
matters: it is a licence question to settle before the code, not a port to write
after it, and the previous draft's two non-gaps would have sent the next round
looking for work that was already done.

**`hijridate` and `iso8583-msg`.** The surfaces match their references; the only
differences are Rust-idiomatic shape (borrowed returns, iterators) rather than
missing capabilities.

## Documentation

* `crates/object-hash/README.md`, which is the crate's `#![doc]` and therefore
  the docs.rs front page, gains the four SHA-3 rows in its algorithm table, the
  sentence explaining the shared Keccak round function, and the extended
  "How it is checked" text.
* `crates/iso20022-rs/README.md` gains a `pacs.002` section with a runnable
  example and the reason the status codes are strings; its introduction and
  `Cargo.toml` description now name status reports.
* The `pacs` module doc comment in `crates/iso20022-rs/src/pacs.rs` names
  `PaymentStatusReport` as the answer to a `pacs.008`.
* `PLAN.md`'s `object-hash` and `iso20022-rs` sections move SHA-3 and `pacs.002`
  into "covers today" and narrow the gaps accordingly.
* `scripts/fuzz/README.md` gains the `extendedSha3` argument in the object-hash
  generator row, and `research/tools/README.md` names `pacs002` in
  `verify_iso20022.py`'s row.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. This round's developments came from the same
loop. For `object-hash`, the previous round's own record named the SHA-3 family
as present in the reference and absent here; the reference reaches it through
Node's `crypto`, which can be asked for the answer, so the gap was closable and
gateable in one step. For `iso20022-rs`, the crate's README named `pacs` clearing
messages as uncovered, and the official XSD validates a generated document, so
that gap was closable and gateable the same way.
