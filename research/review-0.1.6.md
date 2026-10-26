# Review of every crate against its mature comparison package (2026-10-26)

The 0.1.5 round shipped 22 crates and `address-parser` followed. This round
asks a different question from the gap scans: not "what should exist" but "of
what we have, where does a mature package in another ecosystem do more, and is
that difference worth closing". Every figure below was fetched live from
`crates.io/api/v1`, `index.crates.io`, `api.npmjs.org` and `pypistats.org` on
2026-10-26 by `research/tools/review016.py`.

Two measurements per crate:

* **demand** - the mature upstream package's download volume. This is the size
  of the problem, and it is why the crate exists at all.
* **supply** - the crates.io incumbents that serve the same capability, with
  their recent (rolling ninety-day) downloads, release date, reverse
  dependencies and stars. This is the competition.

A crate is worth extending when the upstream still does materially more than we
do AND our incumbents are weak. Where the incumbents are established
(`unicode-bidi` at 97M, `strsim` at 258M, `quick-xml` at 150M) the honest
answer is that the crate is a focused alternative, not a replacement, and the
work belongs in correctness and coverage of its own scope rather than in
chasing the incumbent's whole surface.

## Correctness first: the differential gates were re-run

Before any feature work, every crate that has an independent reference was
re-checked against it. All of them pass:

| Gate | Reference | Result |
| --- | --- | --- |
| `verify_holidays.py` | PyPI `holidays` 0.107 | **0 undocumented defects** across 31 calendars |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` | 19,104 vectors agree, 0 added |
| `verify_fuzzy_score.py` | PyPI `fuzzywuzzy` 0.18.0 | 2,100 pairs x 10 functions, **0 mismatches** |
| `address-parser` oracle | npm `parse-address` 1.1.2 | 3,954 labelled cases agree |
| `object-hash` canonical form | npm `object-hash` | byte-identical canonical form |
| `verify_spreadsheet.py` (new) | npm `@formulajs/formulajs` 4.6.1 | **171/171 cases agree** |
| `verify_hijri_locales.py` (new) | PyPI `hijridate` 2.6.0 | **132/132 names agree** |

`verify_fuzzy_score.py` refused to run with `python-Levenshtein` installed,
because that swaps the reference's `difflib.SequenceMatcher` for a different
implementation and would have made the comparison meaningless. That refusal is
the gate working.

## The 23 crates

Verdicts: **extend** (upstream does materially more and we should close it),
**hold** (scope is right; the work is correctness, not surface), **focused**
(the incumbent is enormous and a different product; we serve a narrower job).

| Crate | Upstream demand | Nearest incumbents (recent) | Verdict |
| --- | --- | --- | --- |
| `xmlkit` | npm `fast-xml-parser` 78M/wk | `quick-xml` 150M, `roxmltree` 32M | focused |
| `holidays-rs` | PyPI `holidays` 26.9M/mo | `holidays` 7.8K (2023) | **extend** |
| `stdnum-rs` | PyPI `python-stdnum` 5.1M/mo | `stdnum` 15, `id-validator` 20 | **extend** |
| `spreadsheet-formula` | npm `formulajs` 7.7K/wk | `formula` 19 (2022) | **extend** |
| `hijridate` | PyPI `hijridate` 3.3M/mo | `hijri` 11, `hijridate` 40 | **extend** |
| `iso20022-rs` | (SEPA/ISO 20022) | `iso20022` 21 (2020), `sepa` 1.5K | extend |
| `soap-client` | PyPI `zeep` 29.4M/mo | `wsdl` 8, `soap` 3, `rsoap` 49 | extend |
| `iso6346` | npm `container-validator` 1.8K/wk | `iso6346` 18 | hold |
| `eu-vat` | PyPI `python-stdnum` 5.1M/mo | `vat` 4 (2016), `vat_id_validator` 6.4K | hold |
| `iso8583-msg` | PyPI `pyiso8583` 404K/mo | `iso8583` 61 (2017) | hold |
| `arabic` | PyPI `arabic-reshaper` 4.8M/mo | `arabic-reshaper` 2.1K | **extend** |
| `factur-x` | PyPI `factur-x` 287K/mo | `en16931` 6.1K, `zugferd` 160 | extend |
| `wssec` | PyPI `zeep` 29.4M/mo | `wssecurity` FREE, `xml-crypto` FREE | hold |
| `pdfa` | PyPI `pikepdf` 9.5M/mo | `lopdf` 13M, `pdf-writer` 3.7M | focused |
| `adhan` | npm `adhan` 20.7K/wk | `adhan` 45, `salah` 822 | hold |
| `object-hash` | npm `object-hash` 75.7M/wk | `sha1` 191M, `md5` 43.6M | hold |
| `javaproperties` | npm `properties` 652K/wk | `java-properties` 890K | hold |
| `mllp` | PyPI `hl7apy` 758K/mo | `hl7` 309, `hl7-mllp` 80 | **extend** |
| `ebics` | PyPI `python-ebics` | `ebics` 25 | hold |
| `pdfmake` | npm `pdfmake` 1.86M/wk | `printpdf` 1.68M | focused |
| `pades` | PyPI `endesive` 115K/mo | `signature` 134M, `cms` 5.7M | focused |
| `fuzzy-score` | PyPI `thefuzz` 8.0M/mo | `strsim` 258M, `rapidfuzz` 4.9M | focused |
| `address-parser` | npm `parse-address` 105K/wk | `postal` 6.5K (2023, FFI) | **extend** |

## What this round develops

Four extensions, each with a differential gate that already exists, so the new
work is verified by the same oracle as the old:

1. **`holidays-rs`** - was 27 countries against the reference's ~200. **Shipped**:
   Orthodox Easter (`Rule::OrthodoxEasterOffset`, `Date::orthodox_easter()`, the
   Julian computus to the Gregorian date) and four new calendars - Cyprus,
   Malta, Greece and Romania - taking it to **31**, each checked by
   `verify_holidays.py`, which now reports **0 undocumented defects across 31
   calendars**.
2. **`stdnum-rs`** - 57 identifier types against `python-stdnum`'s 90 modules.
   Add national identifiers from regions the crate already reaches but does not
   finish (Africa, the Middle East, South Asia), each with its vectors generated
   by the existing script.
3. **`spreadsheet-formula`** - was 38 functions plus `IF`/`IFERROR`.
   **Shipped**: 33 new functions - the rounding family (`CEILING`, `FLOOR`,
   `MROUND`, `TRUNC`, `EVEN`, `ODD`, `QUOTIENT`), logarithms and number theory
   (`LOG`, `LOG10`, `LN`, `EXP`, `FACT`, `GCD`, `LCM`, `SUMSQ`), position-by-
   position products (`SUMPRODUCT`), the conditional aggregates (`COUNTIFS`,
   `SUMIFS`, `AVERAGEIFS`, `MAXIFS`, `MINIFS`), the text functions (`FIND`,
   `SEARCH`, `PROPER`, `CHAR`, `CODE`, `UNICHAR`, `UNICODE`, `CLEAN`, `T`, `N`)
   and number formatting (`TEXT`, `VALUE`) - taking it to **71**. A new gate,
   `verify_spreadsheet.py`, drives the crate through its public `Evaluator` and
   compares **171/171 cases** against npm `@formulajs/formulajs`. It found a real
   lexer bug: `LOG10(100)` was tokenised as the *cell* `LOG10` followed by a
   stray `(`, because a name ending in digits was always read as a reference.
4. **`hijridate`** - **Shipped**: the localized month, day and era names in the
   four languages the reference publishes (English, Arabic, Bengali, Turkish),
   behind a `Language` enum with `month_name_in`, `day_name_in`,
   `notation_in` and `from_tag`. A new gate, `verify_hijri_locales.py`, checks
   **132/132 names** against PyPI `hijridate` 2.6.0; it caught four Bengali
   transcription slips on its first run.

`arabic`, `soap-client`, `iso20022-rs` and `mllp` are recorded as extendable but
deferred: each needs a design decision (a layout engine, a code generator, a
`pacs` model, an HL7 v2 parser) that is a crate of its own rather than a
feature, and shipping a half of one would be worse than the honest gap.

## A new opening found while reviewing

Reviewing the crates against their mature counterparts turned up a domain none
of the gap rounds had measured: **aviation reference data**. It is written up in
`research/rust-crate-gaps-round20.md` and is the same shape as `holidays-rs` -
an open dataset, a permissive licence, a small API, and a regenerable verifier.

The one crate with volume, `airports` at **875,912 recent downloads**, does not
do the job its name suggests: its own README says it maps an IATA code to a
*timezone* and nothing else, and its last release is v0.0.2 from 2022. The
nearest complete crate has **642 downloads, one star and zero reverse
dependencies**, and is **CC-BY-4.0**, which a permissive crate cannot absorb. The
public-domain dataset (OurAirports, **86,286 airports**) is not shipped by any
Rust crate under Apache-2.0/MIT. Round 20 records this, seven further domains it
swept and rejected with the reason, and the re-measured survivors from rounds
13-19.

## Documentation drift found

`PLAN.md` is the per-crate coverage document and it had drifted from the code:

* It documents 21 crates; the workspace has 23 (`address-parser` and
  `fuzzy-score` are absent).
* It says `holidays-rs` "covers five countries (Indonesia, United States,
  United Kingdom, Germany, France)". The crate covers **27**.
* It says `stdnum-rs` covers a list that stops at Indian Aadhaar. The crate
  covers **57** types including African, Middle Eastern and South Asian
  identifiers.
* It lists `spreadsheet-formula` at 38 functions. The crate now implements
  **71**, and `hijridate` is documented as English-only while it now carries
  four languages.

A coverage document that undercounts its own coverage is worse than none,
because it invites someone to re-implement what is already there. This round
rewrites those sections from the code and adds the two missing crates.

## Method

```sh
python3 research/tools/review016.py all        # every crate
python3 research/tools/review016.py stdnum-rs  # one crate
python3 research/tools/review016.py names foo  # is a name free?
```
