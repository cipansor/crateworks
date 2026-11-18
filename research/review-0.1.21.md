# Review of `stdnum-rs` against `python-stdnum` (2026-11-16)

This round closes the flagship gap in `stdnum-rs`. The crate opens its README
with `Iban`, `Ean`, `Isbn` and `Luhn`, and the oracle gate in
`research/tools/gen_stdnum_vectors.py` covered none of them: the four names were
absent from the gate's type list, so the types a reader meets first were the only
ones with no differential coverage. A gap in the gate is not a gap in the crate
until it is measured, so the round measured it first and then closed it.

## What the measurement found

A differential run over 25,000 generated inputs, replaying each through the crate
and through the reference with the same verdict rule, reported:

* **`Iban`** - the crate's `parse` doc claimed it checked the country-specific
  *"length and structure"*, but it checked length only. Two malformed IBANs the
  reference rejects, `DE171OCTNFFVAV1D4OWFYE` and `GB538WZQVVOGJBMXEJ82SO`, both
  closed the MOD-97 check and were accepted.
* **`Isbn`** - a nine-digit ISBN-10 is a printed form whose leading zero was
  dropped; the reference pads it back before checking and the crate did not, so a
  published number was rejected. A 13-digit number whose GS1 prefix is neither
  `978` nor `979` is an EAN, not a book number, and the crate accepted it.
* **`Luhn`** - the reference accepts the lone digit `0`, because its one-digit
  checksum is zero. The crate requires at least two characters. This is a
  deliberate divergence, recorded below rather than fixed.

## `stdnum-rs` 0.7.0 -> 0.8.0

### IBAN: the structure table and the national check digit

`Iban::parse` now checks three independent things, in order, after the length:

1. **The BBAN character structure.** `research/tools/gen_iban_structures.py`
   reads the reference's own registry file and writes an 89-country table of
   `<count>!<class>` groups (class `n` a digit, `a` an upper-case letter, `c`
   either) between `BEGIN`/`END GENERATED` markers in `iban.rs`. A German BBAN is
   `18!n`, so a letter anywhere in it is rejected; a Greek BBAN is `3!n4!n16!c`,
   so a letter in the last group is fine. The error names the first offending
   character.
2. **The MOD-97-10 check**, unchanged.
3. **The national check digit** that four countries carry inside the BBAN. All
   four are arithmetic, so none needs a registry: Belgium's last two digits are
   the first ten modulo 97 (a remainder of zero written as 97); Spain's CCC is
   two check digits over two halves of the twenty-digit account; Norway's
   `kontonr` is a modulus-11 digit over the first ten, after a leading `0000`
   postgiro prefix is stripped and the account re-read as seven digits; and
   Montenegro's is one modulo 97 over the whole BBAN.

The ordering matters: the MOD-97 check is cheapest and rejects most mutations,
the structure check needs no arithmetic, and the national check is the most
specific. The crate's module doc now says "three independent ways" rather than
the earlier claim.

### ISBN: the nine-digit form and the Bookland prefix

Two fixes, each taken from the reference rather than from the specification
alone. A nine-digit input gains a leading `0` before the length dispatch, which
is what the reference's `compact` does. And a 13-digit number whose GS1 prefix is
neither `978` nor `979` now returns `Error::InvalidPrefix`, which is the
reference's `InvalidComponent` for the same input. `Error::InvalidPrefix` was
already in the enum and already used by `to_isbn10`, so the round added no
variant.

### Luhn: a deliberate divergence, recorded not fixed

The reference's `checksum` of a single digit is that digit, so `0`, `00`, `000`
and `0000` are all valid there: a string of zeros sums to zero and closes. The
crate requires a payload and a check digit, so it requires at least two
characters, and rejects `0`. A one-character number has no payload and no check
digit, and no issuer issues one, so the crate keeps its rule. The fixture records
the reference's verdict and the crate's own test records the reason, so the
divergence is visible in both directions rather than silently.

## Verification

The gate is the point of the round, so the four types were added to it and the
proof is that the gate now *fails* on the old behaviour.

`gen_stdnum_vectors.py` gains `Ean`, `Iban`, `Isbn` and `Luhn`, each with a
corpus that builds valid numbers from the reference's own check-digit routine
because their doctests offer one example each. The IBAN corpus is the careful
one: alongside a valid IBAN for every one of the 89 countries it adds a BBAN with
a letter where the registry requires a digit, and for Belgium, Spain, Norway and
Montenegro a BBAN whose national check digit is wrong, each closed with *correct*
MOD-97 check digits so that only the structure check or the national check
separates the two implementations. Belgium is left out of the national-check rows
on purpose, because the reference looks the bank code up in a register this crate
deliberately does not ship.

The regenerated `crates/stdnum-rs/tests/fixtures/stdnum_vectors.tsv` holds
132,573 rows, up from 85,713. Replaying the *old* crate logic against the new
fixture finds 126 IBAN rows and 11 ISBN rows it would have accepted but the
fixture labels rejected - those rows are exactly the divergences the fixes close,
and they are the evidence that the gate is a guard and not a formality. For Luhn
the count is zero, which is correct: the divergence there is the crate's own
stricter rule, and the corpus documents it rather than contradicting it.

The full suite passes: `cargo fmt --all -- --check`, `cargo clippy --workspace
--all-targets -- -D warnings` and `cargo test --workspace --all-targets`, with
`stdnum-rs` at 274 unit tests, 3 README/oracle tests and its doctests.

| Gate | Reference | Result |
| --- | --- | --- |
| `gen_stdnum_vectors.py` | PyPI `python-stdnum` | 132,573 rows; `Ean`/`Iban`/`Isbn`/`Luhn` added |
| `tests/readme.rs` | the frozen corpus | every labelled number agrees; 0 divergences |
| old logic vs new fixture | the frozen corpus | 126 IBAN and 11 ISBN rows caught that the old crate accepted |
| `cargo test --workspace` | the crate's own suite | all pass, 0 failed |

## Documentation

* `crates/stdnum-rs/README.md` - the IBAN paragraph names the three checks and
  the four national checks, the ISBN paragraph names the nine-digit form and the
  Bookland prefix, the fixture count moves to 132,573, and a new paragraph records
  the two deliberate divergences (Luhn's minimum length, the missing Belgian bank
  register).
* `PLAN.md` - the `stdnum-rs` "Covers today" line moves from "78 countries" to
  the 89 in the registry and names the structure and national checks.
* `crates/stdnum-rs/src/iban.rs` - the generated structure table, the structure
  and national-check functions, the module and `parse` docs, and new tests for
  the structure check, the national checks and the Belgian boundary.
* `crates/stdnum-rs/src/isbn.rs` - the nine-digit padding, the prefix rule, the
  `parse` doc, and new tests.
* `crates/stdnum-rs/src/luhn.rs` - a test recording why the lone `0` is
  rejected.
* `research/tools/gen_iban_structures.py` and the IBAN/ISBN/EAN/Luhn corpora in
  `research/tools/gen_stdnum_vectors.py` are the generators, committed so the
  fixture can be rebuilt.

## Follow-up

The gate now covers the four flagship types; the rest of the crate's types are
still covered by the same file, so the next round can widen the corpora the same
way where a doctest is thin. The reference's remaining national IBAN modules are
only the four this round added, so no country's national check is left out except
Belgium's register-backed one, which stays out by the crate's standing rule that
it ships no registry for any identifier.
