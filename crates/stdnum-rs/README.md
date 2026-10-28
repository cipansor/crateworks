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
Peruvian RUC, the Argentine CUIT, the Paraguayan RUC, the Salvadoran NIT, the
Ecuadorian CI and RUC, the Guatemalan NIT, the Costa Rican CPJ, the Dominican
RNC and the Cuban NI. Iceland contributes the kennitala and the VSK number, and
Mauritius the NID.
Asia and Oceania contribute the Japanese corporate and My Number identifiers,
the Pakistani CNIC, the Taiwanese UBN, the Israeli identity number, the Thai
PIN, the Singapore UEN, the Malaysian NRIC, the Vietnamese MST, the Indonesian
NPWP and NIK, and the Chinese, Korean and Australian numbers. The United Kingdom
is represented by the NHS number, the UTR, the VAT number and SEDOL, and Africa
and the Middle East by the South African identity number, the Greek ΑΜΚΑ, and
the tax and company numbers of Algeria, Egypt, Ghana, Kenya, Morocco,
Mozambique, Senegal, Tunisia and Guinea. [`Country::ALL`](https://docs.rs/stdnum-rs)
lists them all.

The numbers whose separator sets differ per country are shape checks that clean
the raw input themselves rather than sharing the crate's global normalisation.
Egypt's tax number is also accepted in Arabic-Indic digits, which is how it is
printed in an Arabic document.

Twenty-one international identifiers are covered in addition to those, each a
type of its own:

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
| `Imo` | IMO | a ship's hull |
| `Meid` | MEID | a piece of CDMA mobile equipment |
| `Isan` | ISO 15706 | an audiovisual work |
| `BitcoinAddress` | BIP-13 / BIP-173 | a Bitcoin payment destination |
| `Mac` | IEEE 802 | the hardware address of a network interface |
| `Isil` | ISO 15511 | a library or a related organisation |
| `EcNumber` | EC Number | a chemical substance in the EU |
| `EuropeanBanknote` | — | the serial number on a euro banknote |
| `SepaCreditorIdentifier` | AT-02 | a SEPA direct-debit creditor |
| `Eic` | EIC | an entity or object in the European energy market |

`Cusip::to_isin` and `Isin::from_national_id` move a national identifier to the
international form. `Ismn` accepts both the current thirteen-digit form and the
legacy `M` form. `Imei` accepts all three lengths and reports which one it is.
`Imo` also exposes `check_digit_for`, `Meid` `check_digit` and `to_decimal`, and
`Isan` a hyphenated `formatted` form that fills in a missing check character.
`BitcoinAddress` accepts the Base58 (`1…`, `3…`) and Bech32 (`bc1…`) forms,
checking the Base58 double-SHA-256 and the Bech32 BCH checksum; the SHA-256 it
needs is a private implementation in the crate, not a dependency.

The check-digit algorithms themselves are public, in
[`iso7064`](https://docs.rs/stdnum-rs/latest/stdnum_rs/iso7064/index.html) (MOD
11-2, MOD 37-36 and MOD 97-10), [`damm`](https://docs.rs/stdnum-rs/latest/stdnum_rs/damm/index.html),
[`verhoeff`](https://docs.rs/stdnum-rs/latest/stdnum_rs/verhoeff/index.html) and
[`luhn`](https://docs.rs/stdnum-rs/latest/stdnum_rs/luhn/index.html). The ISO
7064 functions are free functions; the other three are number types (`Damm`,
`Verhoeff`, `Luhn`) whose `check_digit_for` computes the digit for a payload, so
a caller can generate a number as well as check one. Damm and Verhoeff catch
every single-digit error and every adjacent transposition, including the
`09`/`90` swap that Luhn lets through.

## Data provenance

The rules were cross-checked against
[`python-stdnum`](https://arthurdejong.org/python-stdnum/) by Arthur de Jong and
contributors, which in turn follows each authority's published specification.
This is an independent reimplementation and shares no code with it; see
[`NOTICE`](https://github.com/cipansor/crateworks/blob/main/crates/stdnum-rs/NOTICE).

`tests/fixtures/stdnum_vectors.tsv` holds 45,134 numbers labelled by that
reference: every number in its own documentation plus each single-digit
substitution of a valid number, the last character dropped and a character
appended. Damm and Verhoeff also carry a 1,500-number random corpus each, built
by asking the reference for the check digit of a random payload, because their
own documentation offers a single example; `EcNumber`, `EuropeanBanknote`,
`SepaCreditorIdentifier` and `Eic` get a smaller corpus the same way, because
their documentation offers one example too. `tests/readme.rs` replays the whole
file, so a change that makes this crate disagree with the reference fails the
build. The check digit, the accepted shapes and the separator characters all
came out of that comparison, and `research/tools/gen_stdnum_vectors.py`
regenerates the file when a type is added.

Two behaviours were settled by that comparison rather than by the
specification alone. `mod_37_36` replaces a zero intermediate value with the
modulus, which changes the answer for inputs such as `0000`. `CasNumber`
rejects a hyphen that is not where the specification puts it, rather than
repairing it, because a misplaced hyphen is a transcription error. The
identifier ports added later were built the same way: the ISAN check is taken
over the whole concatenation rather than over the one character the reference
computes, and a Base58 Bitcoin address is compared case-sensitively, because
the fixture contains an address whose alphabet letter would otherwise be
rejected.

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

// The registry-free identifiers: a ship, a CDMA handset, an audiovisual work
// and a Bitcoin address.
assert!(stdnum_rs::Imo::parse("IMO 9319466").is_ok());
assert_eq!(
    stdnum_rs::Meid::parse("AF 01 23 45 0A BC DE C").unwrap().as_str(),
    "AF0123450ABCDE",
);
assert!(stdnum_rs::Isan::parse("0000-0001-8CFA-0000-I-0000-0000-K").is_ok());
assert!(stdnum_rs::BitcoinAddress::parse("1NEDqZPvTWRaoho48qXuLLsrYomMXPABfD").is_ok());

// The stronger check digits: Damm and Verhoeff catch the `09`/`90`
// transposition that Luhn misses. Each can generate the digit for a payload.
assert_eq!(stdnum_rs::Damm::check_digit_for("572").unwrap(), 4);
assert!(stdnum_rs::Damm::parse("5724").is_ok());
assert_eq!(stdnum_rs::Verhoeff::check_digit_for("1234").unwrap(), 0);
assert!(stdnum_rs::Verhoeff::parse("12340").is_ok());

// The EU-domain identifiers: a substance's EC Number and a banknote serial.
assert_eq!(
    stdnum_rs::EcNumber::parse("200-001-8").unwrap().digits(),
    "2000018",
);
assert_eq!(stdnum_rs::EcNumber::parse("2000018").unwrap().as_str(), "200-001-8");
assert!(stdnum_rs::EuropeanBanknote::parse("P36007033744").is_ok());

// A SEPA creditor identifier and an energy identification code.
assert_eq!(
    stdnum_rs::SepaCreditorIdentifier::parse("ES 23 ZZZ 47690558N")
        .unwrap()
        .business_code(),
    "ZZZ",
);
assert_eq!(
    stdnum_rs::Eic::parse("22XWATTPLUS----G").unwrap().object_type(),
    'X',
);
```

## Scope

Only checksum and structural validation is performed. Nothing here contacts
a registry, so a number that passes may still be unissued. Treat this as a
first filter in a form, never as proof that a number exists.

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
