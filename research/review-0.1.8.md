# Review of every crate against its mature comparison package (2026-10-27)

This is the follow-up to `review-0.1.7.md`. That round asked "of what we have,
where does a mature package in another ecosystem do more, and is that difference
worth closing". The answer then was that every crate with an independent
reference was sound and the one gap left was in `spreadsheet-formula`.

This round re-asks the question and finds the one gap that is large, oracle-
verifiable and still open: **`stdnum-rs` against PyPI `python-stdnum`**. The
reference library covers 200+ number types and installs 5.1M times a month; the
crate had a counterpart for 112 national identifiers and no more. The difference
is not a handful of exotic numbers - it is the everyday tax and register numbers
of Poland, Norway, Denmark, Canada, Australia, India, Mexico and half of Europe.

## Correctness first: the differential gates were re-run

| Gate | Reference | Result |
| --- | --- | --- |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` 2.2 | 45,134 kept + **4,166 new vectors agree** |
| `verify_spreadsheet.py` | npm `@formulajs/formulajs` | 826/826 cases agree |
| `verify_aviation.py` | OurAirports `airports.csv` | 11,888 rows and 22,477 codes agree |
| `verify_arabic.py` | PyPI `arabic-reshaper` | 6,558/6,558 agree |
| `verify_holidays.py` | PyPI `holidays` | 0 undocumented defects |
| `verify_hijri_locales.py` | PyPI `hijridate` 2.6.0 | 132/132 names agree |
| `verify_fuzzy_score.py` | PyPI `fuzzywuzzy` 0.18.0 | 0 mismatches |

## What this round develops

**`stdnum-rs` 0.3.0 -> 0.4.0.** Seventeen national identifiers were added, one
`Country` arm each, every one taken from the reference's own algorithm and
verified by the fixture gate:

| Identifier | Country | Shape |
| --- | --- | --- |
| NIP | Poland | 10 digits, weighted sum to zero |
| REGON | Poland | 9 or 14 digits, one or two check digits |
| NIF | France | 13 digits, first ten modulo 511 |
| bank account number | Norway | 11 digits mod-11, or 7-digit postgiro by Luhn |
| MVA | Norway | an organisation number with an `MVA` suffix |
| CVR | Denmark | 8 digits, weighted sum to a multiple of eleven |
| Business Number | Canada | 9 digits Luhn, or a 15-character BN15 |
| ACN | Australia | 9 digits, weights eight down to one |
| PAN | India | 10 characters, holder type and a non-zero serial |
| GSTIN | India | 15 characters, state code, PAN and Luhn mod 36 |
| CURP | Mexico | 18 characters, a weighted base-37 check digit |
| registrikood | Estonia | 8 digits, the isikukood's check over seven |
| KMKR | Estonia | 9 digits, weighted sum to a multiple of ten |
| PVM | Lithuania | 9 or 12 digits, the asmens kodas check |
| IČ DPH | Slovakia | 10 digits, a birth number or a multiple of eleven |
| Cartão de Cidadão | Portugal | digits, two version characters, a check digit |
| tax reference number | South Africa | 10 digits, leading digit in `01239`, Luhn |

Two of these caught a real divergence between the port and the reference while
the gate was being built, and in both cases the reference was right:

* The **Australian ACN** weights run eight, seven ... one but the first weight is
  applied to the first digit as `i - 8`, so the running sum is negative. Python's
  `% 10` returns a non-negative remainder for a negative operand; Rust's `% 10`
  does not. The port now uses `rem_euclid`.
* The **Portuguese Cartão de Cidadão** is not fixed at twelve characters. The
  reference's own regex is `^\d*[A-Z0-9]{2}\d$`, so it accepts any run of leading
  digits. The first port required nine and rejected inputs the reference accepts;
  it now matches the reference's shape exactly.

The fixture gate grew from 45,134 to 49,300 lines. `gen_stdnum_vectors.py` gained
the seventeen new rows in its `COUNTRIES` map, and `tests/readme.rs` dispatches
the new names to their `Country` arms. The gate is what makes the claim; the
crate's own `newly_covered_identifiers` test pins one accepted and one rejected
value per identifier so a regression is caught without the Python oracle present.

## Documentation

* `crates/stdnum-rs/README.md` moved from "one hundred and two national
  identifiers" (already stale at 112) to **129**, and the coverage prose now
  names the new countries. The README is the crate's `#![doc]`, so this is the
  docs.rs front page.
* `PLAN.md` moved from 112 to 129 identifiers, from 90 to 98 countries, and from
  nineteen to twenty-one international identifiers, and records the 0.4.0 round.

## Method

The gates are the review: each one is a reference implementation driven over a
corpus, so "does this crate do what the mature package does" is a pass/fail
rather than a reading exercise. The cost of this round was in extending the
stdnum corpus, which is the only gate whose reference had more to give.
