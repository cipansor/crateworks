# Gap scan round 19: bank/payment tooling, product codes, addresses and phones (2026-10-21)

Rounds 8-18 each re-measured every open candidate and then swept domains no
earlier round had touched. This round does the same. The four new domains are
ones the npm and PyPI popularity lists under-represent, but which a business
application meets constantly: **bank and IBAN tooling**, **product and price
codes**, **free-text address parsing**, and **phone numbers**.

Every figure was fetched live on 2026-10-21 from `crates.io/api/v1`,
`index.crates.io` (the sparse index, for a real release date), `api.npmjs.org`,
`pypistats.org` and `api.github.com`. A gap is real when the Rust incumbent has
under about 20,000 recent downloads (the rolling ninety-day figure) and near-zero
reverse dependencies, while the source counterpart has real volume.

The rule rounds 15-18 established is applied throughout: **before proposing a
crate, sort the nearest crates by release date, not only by downloads, and ask
whether the incumbent has an audience.** A cluster of unadopted crates is a
race, not a gap. Round 15's second rule also applies: **a keyword search on the
upstream package's name is not a gap check** - search the *capability*. Every
`FREE` below was re-checked, and every "claimed" verdict below was reached by
searching for the capability, not the name.

## Verdict summary

| Domain | Upstream volume | Best Rust incumbent | Verdict |
| --- | --- | --- | --- |
| Free-text address parsing | `usaddress` 5.03M/mo, `parse-address` 68k/wk | `postal` 6,413 (2023, FFI) | **strongest opening** |
| EPC QR / SEPA payment QR | every EU invoice | `epcgen` 14 (2025-11) | **open**, tiny incumbent |
| GS1 application identifiers | `bwip-js` 800k/wk | `qrcode-parse` 32,713 partial | race, above threshold |
| GTIN / EAN / UPC product codes | `jsbarcode` 1.02M/wk, `python-barcode` 2.65M/mo | `barcode` **FREE**, others dead | **open** |
| IBAN / BBAN / bank identifiers | `ibantools` 389k/wk, `schwifty` 995k/mo | `iban_validate` 2.03M | served |
| Phone numbers | `libphonenumber-js` 18.0M/wk, `phonenumbers` 34.3M/mo | `phonenumber` 3.23M | served |

## The survivors, re-measured

Nothing changed verdict since round 18. The four long-open candidates
(OFX/QIF, subtitles, PDF tables, STIX/TAXII), the readability watch, the
METAR/TAF opening and the runtime unit-conversion find are all unchanged, so
they are not re-argued here. Their live numbers:

| crate | recent | last release | revdeps |
| --- | --- | --- | --- |
| `ofx-rs` / `ofxy` / `qif_parser` | 2,959 / 1,324 / 380 | 2026-03 / 2025-09 / 2026-09 | 1 / 0 / 0 |
| `subtp` / `subparse` / `subtitler` | 21,660 / 4,887 / 1,978 | 2024-02 / 2021-04 / 2026-10 | 3 / 9 / 0 |
| `pdf-extract` / `pdfplumber` | 3,634,439 / 42,602 | 2026-09 / 2026-08 | 282 / 1 |
| `stix` / `rstix` | 889 / 298 | 2021-04 / 2026-10 | 1 / 2 |
| `dom_smoothie` / `readability` | 532,798 / 394,156 | 2026-09 / 2023-12 | 43 / 34 |
| `uom` / `runtime_units` | 2,653,164 / 1,785 | 2026-02 / 2026-05 | 215 / 0 |
| `metar` / `metar-taf-parser` | 600 / 5,733 | 2026-01 / 2026-03 | 0 / 0 |
| `im` (archived) / `rpds` | 4,744,361 / 4,791,636 | 2022-04 / 2026-05 | 238 / 79 |

`taxii` and `taxii-rs` are still `FREE`.

## New domain 1: free-text address parsing

This is the round's strongest opening.

| | |
| --- | --- |
| Upstream | PyPI `usaddress` **5,029,779/month**; npm `parse-address` 67,958/week; npm `addresser` 40,253/week; PyPI `postal` 72,056/month |
| Rust incumbents | `postal` 6,413 recent (2023-09, FFI bindings to the C libpostal); `address` 608 (2026-09, 0 stars); `us_address_parser` 545 (2025-03); `usaddress` 3,073 (2021-06); `libpostal-rs` 45 (2025-08); `libpostal` 1 (2021-07) |
| Free names | `address-parser`, `address_parser`, `postal-address`, `rust-libpostal`, `postal-codes`, `zipcode` |

Every Rust crate in this space is either a thin FFI wrapper around the C
libpostal (which needs a 1 GB model download and a C toolchain) or an abandoned
US-only regex. `usaddress` alone is downloaded five million times a month, and
its capability - split a free-text address into house number, street, city,
state, ZIP - has no pure-Rust implementation with an audience. `postal` at 6,413
is the nearest, and it is 2023, FFI, and 33 stars.

The opening is a pure-Rust address parser: rule-based for one country, with the
same components the reference emits. It is a genuine parser, not a validator, so
it is not the same job as `stdnum-rs`.

## New domain 2: EPC QR / SEPA payment QR codes

| | |
| --- | --- |
| Upstream | The EPC QR code is on every EU invoice; `epcgen` is the nearest Rust crate |
| Rust incumbents | `epcgen` **14** recent (2025-11); `qrplatba` 40 (2026-10, Czech SPAYD variant); `sepa` 1,544 (2026-10, identifiers and ISO 20022 builders) |
| Free names | `epc-qr`, `sepa-qr`, `epc-qrcode`, `sepa-credit-transfer` |

`epcgen` exists, is 2025, and has **fourteen** recent downloads. That is an
unadopted crate, not a served one. The EPC QR code (the EPC069-12 standard
behind the "scan to pay" square on a European invoice) is a fixed field layout
that a payment integration needs and has to hand-roll today. `sepa` at 1,544
covers the message side, not the QR layout, and is itself thin.

## New domain 3: GS1 application identifiers

| | |
| --- | --- |
| Upstream | npm `bwip-js` 800,205/week; npm `jsbarcode` 1,019,248/week |
| Rust incumbents | `qrcode-parse` 32,713 (2026-10, includes a GS1 payload parser); `dpp-digital-link` 2,558 (2026-10); `gs1` 284 **ARCHIVED**; `barkod` 48 (2026-08) |
| Free names | `gs1-ai`, `gs1-parser`, `gs1-digital-link` |

This one is a **race, not a gap**: `qrcode-parse` at 32,713 is above the
twenty-thousand threshold and its description names GS1 payloads. The GS1
Application Identifier parser (the `(01)09501101530003(17)251231` string on
every shipping label) is covered in part by that crate, so a new entrant would
have to be clearly better. `gs1` being archived frees the name but not the
space. Recommend: do not build unless a real need appears.

## New domain 4: GTIN / EAN / UPC product codes

| | |
| --- | --- |
| Upstream | npm `jsbarcode` 1,019,248/week; PyPI `python-barcode` 2,653,583/month; PyPI `treepoem` 381,527/month |
| Rust incumbents | `gtin` **4** (2019); `gtin-validate` 663 (2020); `gtinny` 15 (2024); `upc` 2,143 (2026-05); `barkod` 48 (2026-08) |
| Free names | `barcode`, `ean13`, `barcode-generator` |

Every GTIN crate is dead: 4, 663 and 15 recent downloads, the newest from 2024.
`barcode` itself is a **free name**. The check-digit and structure side is
small enough to live in `stdnum-rs` (which already does EAN and ISBN), so the
standalone opening is narrower than it looks - the value would be in the
symbology encoding, which `barcoders` (159,084, 2024) and `rxing` (217,263,
2026) already serve. Recommend: the check-digit part belongs in `stdnum-rs`,
not a new crate.

## New domain 5: bank and IBAN tooling

| | |
| --- | --- |
| Upstream | npm `ibantools` 388,864/week; npm `iban` 108,738/week; PyPI `schwifty` 994,607/month |
| Rust incumbents | `iban_validate` **2,026,787** recent (2024-10, 10 revdeps); `iban` 142,648 (2025-06); `iban_validation_rs` 160 (2026-10); `bic` 9 (2026-10) |
| Free names | `rust-iban`, `iban_rs`, `bban`, `bank-identifiers`, `bic-iban` |

Served. `iban_validate` at 2.03 million recent downloads is the incumbent, and
this workspace's `stdnum-rs` already covers IBAN, and `eu-vat` the national VAT
side. The BBAN and bank-identifier names are free but the capability is thin
because the volume is concentrated in `iban_validate`. Recommend: no new crate.

## New domain 6: phone numbers

| | |
| --- | --- |
| Upstream | npm `libphonenumber-js` **18,004,556/week**; PyPI `phonenumbers` **34,298,405/month** |
| Rust incumbents | `phonenumber` **3,232,917** recent (2026-07, 67 revdeps) |
| Free names | `rust-phonenumber`, `libphonenumber`, `phonenumbers`, `e164` |

Served, decisively. `phonenumber` at 3.23 million recent downloads is the
`libphonenumber` port and it is maintained. The free names are traps: the
capability is claimed and the incumbent has an audience.

## Conclusion

Two openings survive this round, both with unadopted incumbents rather than
empty registries:

1. **A pure-Rust free-text address parser.** `usaddress` is at 5.03M/month and
   every Rust equivalent is an FFI wrapper or an abandoned US regex. Free
   names, a real capability, and no C dependency.
2. **The EPC QR code.** `epcgen` has fourteen recent downloads; the standard is
   fixed and every EU payment integration needs it.

The GTIN check digits belong in `stdnum-rs`; GS1 application identifiers are a
race; IBAN and phone numbers are served. `barcode` remains a free name, but the
symbology work it would imply is already done by `barcoders` and `rxing`.

## Method note

The script is `tools/round19.py`:

```sh
python3 round19.py candidates   # re-measure the open survivors
python3 round19.py sweep        # the four new domains
python3 round19.py names <name>...  # is a proposed crate name free?
```

It imports the helpers from `round15.py` and `round16.py`, which guard their
main blocks, so the sparse-index rules round 17 established carry over
unchanged: the field is `pubtime`, not `pubdate`, and the index path is
`2/<name>` for two characters, `1/<name>` and `3/<c>/<name>` otherwise.
crates.io returns HTTP 429 under load and a naive fetch reads that as "no such
crate", so every `FREE` here was re-checked.
