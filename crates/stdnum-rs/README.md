# stdnum-rs

[![crates.io](https://img.shields.io/crates/v/stdnum-rs.svg)](https://crates.io/crates/stdnum-rs)
[![docs.rs](https://docs.rs/stdnum-rs/badge.svg)](https://docs.rs/stdnum-rs)

Validation for standard numbers: IBAN, ISBN, EAN/UPC, Luhn, the ISO 7064
check-digit algorithms, the financial instrument and institution identifiers,
the publication and device identifiers, and national identifiers.

```rust
use stdnum_rs::{Iban, Isin, Luhn};

assert!(Iban::parse("GB82 WEST 1234 5698 7654 32").is_ok());
assert!(Luhn::parse("4539148803436467").is_ok());
assert!(Isin::parse("US0378331005").is_ok());
```

Checksum and structure only; nothing here contacts a registry.

## What is covered

One hundred and two national identifiers are covered. Europe accounts for the
largest share: the Irish VAT and PPS numbers, the Croatian OIB, the Czech and
Slovak birth numbers, the Swiss AHV/AVS and UID, the Russian ИНН and ОГРН, the
Ukrainian ЄДРПОУ, the Austrian UID and TIN, the Latvian PVN, the Monégasque TVA,
and the microstate numbers of Andorra, Liechtenstein, San Marino, Albania,
Azerbaijan, Moldova and the Faroe Islands. The Americas bring the United States
Social Security and taxpayer numbers, the Chilean RUT, the Colombian NIT, the
Peruvian RUC, the Argentine CUIT, the Paraguayan RUC and the Salvadoran NIT.
Asia and Oceania contribute the Japanese corporate and My Number identifiers,
the Pakistani CNIC, the Taiwanese UBN, the Israeli identity number, the Thai
PIN, the Singapore UEN, the Malaysian NRIC, the Vietnamese MST, the Indonesian
NPWP and NIK, and the Chinese, Korean and Australian numbers. The United Kingdom
is represented by the NHS number, the UTR, the VAT number and SEDOL, and Africa
and the Middle East by the South African identity number, the Greek ΑΜΚΑ, and
the tax and company numbers of Algeria, Egypt, Ghana, Kenya, Morocco,
Mozambique, Senegal and Tunisia. [`Country::ALL`](https://docs.rs/stdnum-rs)
lists them all.

The numbers whose separator sets differ per country are shape checks that clean
the raw input themselves rather than sharing the crate's global normalisation.
Egypt's tax number is also accepted in Arabic-Indic digits, which is how it is
printed in an Arabic document.

Thirteen international identifiers are covered in addition to those, each a type
of its own:

| Type | Standard | What it identifies |
|---|---|---|
| `Isin` | ISO 6166 | a listed security |
| `Cusip` | CUSIP | a North American security |
| `Figi` | FIGI | a financial instrument across venues |
| `Lei` | ISO 17442 | a legal entity |
| `Bic` | ISO 9362 | a bank or one of its branches |
| `Grid` | GRID | a research organisation |
| `Issn` | ISO 3297 | a periodical |
| `Isrc` | ISO 3901 | a sound or music video recording |
| `Imei` | IMEI | a mobile device |
| `Ismn` | ISO 10957 | printed music |
| `Isni` | ISO 27729 | a contributor to media content |
| `CasNumber` | CAS Registry | a chemical substance |
| `CreditorReference` | ISO 11649 | a payment reference |

`Cusip::to_isin` and `Isin::from_national_id` move a national identifier to the
international form. `Ismn` accepts both the current thirteen-digit form and the
legacy `M` form. `Imei` accepts all three lengths and reports which one it is.

The check-digit algorithms themselves are public, in
[`iso7064`](https://docs.rs/stdnum-rs/latest/stdnum_rs/iso7064/index.html): MOD
11-2, MOD 37-36, MOD 97-10, Damm and Verhoeff. Each has a `checksum` and a
`check_digit` function, so a caller can generate a number as well as check one.

## Data provenance

The rules were cross-checked against
[`python-stdnum`](https://arthurdejong.org/python-stdnum/) by Arthur de Jong and
contributors, which in turn follows each authority's published specification.
This is an independent reimplementation and shares no code with it; see
[`NOTICE`](https://github.com/cipansor/crateworks/blob/main/crates/stdnum-rs/NOTICE).

`tests/fixtures/stdnum_vectors.tsv` holds 24,533 numbers labelled by that
reference: every number in its own documentation plus each single-digit
substitution of a valid number, the last character dropped and a character
appended. `tests/readme.rs` replays the file, so a change that makes this crate
disagree with the reference fails the build. The check digit, the accepted
shapes and the separator characters all came out of that comparison, and
`research/tools/gen_stdnum_vectors.py` regenerates the file when a type is
added.

Two behaviours were settled by that comparison rather than by the
specification alone. `mod_37_36` replaces a zero intermediate value with the
modulus, which changes the answer for inputs such as `0000`. `CasNumber`
rejects a hyphen that is not where the specification puts it, rather than
repairing it, because a misplaced hyphen is a transcription error.

Each number type lives in its own module so that a program only pays for
what it uses, and `validate` offers a uniform entry point when the type
is only known at runtime.

## Example

```
use stdnum_rs::{Iban, validate, NumberType};

// IBAN: country code, two check digits, then the national part.
assert!(Iban::parse("GB82 WEST 1234 5698 7654 32").is_ok());
assert!(Iban::parse("GB82 WEST 1234 5698 7654 33").is_err());

assert!(validate(NumberType::Iban, "DE89370400440532013000").is_ok());

// An ISIN can be built from a national identifier, and a CUSIP promoted.
assert_eq!(
    stdnum_rs::Isin::from_national_id("gb", "BYXJL75").unwrap().as_str(),
    "GB00BYXJL758",
);
```

## Scope

Only checksum and structural validation is performed. Nothing here contacts
a registry, so a number that passes may still be unissued. Treat this as a
first filter in a form, never as proof that a number exists.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
