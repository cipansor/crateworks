# Gap scan round 27: Plus Codes are the opening, and the incumbent proves it (2026-10-21)

Round 26 asked for the capability search to come before the verdict. This round
sweeps eight capabilities a developer meets in ordinary application code - a
geocode to store, a number to spell, a name to split, a page to clean - and for
each one the name pass was followed by a `?q=` capability search, a read of the
incumbent's own source, and a run of its test suite before any verdict was
written down.

One domain survives all four passes: **Open Location Code / Plus Codes**. Two
that looked like openings on demand alone (`number-to-words`) are closed, and one
of them was closed three months ago by a port this workspace would have
duplicated.

## The eight domains

| Domain | Demand | Rust today | Verdict |
| --- | --- | --- | --- |
| Open Location Code / Plus Codes | PyPI `openlocationcode` 97,535/mo; npm `open-location-code` 39,242/wk | `open-location-code` 2,475 **v0.1.0 2018, its whole test suite fails**; `pluscodes` 291 (2022, no `shorten`/`recover`); `olc` 3 (2021); `openlocationcode` FREE | **Open - the finding** |
| Multilingual number-to-words | PyPI `num2words` 7,789,143/mo, `inflect` 22,422,041/mo; npm `number-to-words` 612,957/wk | `num2words2-core` 27,645 (2026-10, **120+ languages**, MIT/Apache-2.0); `num2words` 96,973 (2024, **5 languages only**) | **Closed three months ago** |
| Human name parsing | PyPI `nameparser` 3,132,865/mo, `usaddress` 5,029,779/mo; npm `parse-full-name` 225,568/wk | `human_name` 503,606 (2024-10, 61 stars, **1 revdep**), `name-parser` / `probablepeople` FREE | **Served by one crate, weakly** |
| Extractive summarization and keyword extraction | PyPI `keybert` 381,292/mo, `yake` 236,844/mo, `rake-nltk` 223,069/mo, `sumy` 113,419/mo; npm `keyword-extractor` 218,722/wk | `rake` 50,319, `yake-rust` 19,273, `keyword_extraction` 10,011, `keyphrases` 1,179; `sumy`/`textrank`/`lexrank` FREE | **Served, three algorithms** |
| Human-readable formatting | PyPI `humanize` 46,399,361/mo, `humanfriendly` 44,515,852/mo; npm `pretty-bytes` 28,776,042/wk, `filesize` 15,728,380/wk | `num-format` 14,540,775, `thousands` 7,533,065, `duration-str` 3,814,076, `human_format` 2,434,173, `human_bytes` 1,574,251, `human-repr` 186,515 | **Served six times over** |
| English inflection and pluralization | PyPI `inflection` 35,354,420/mo; npm `pluralize` 30,481,498/wk, `inflection` 5,149,078/wk | `Inflector` 19,533,319 (**ARCHIVED**, 2019), `pluralizer` 1,924,234 (2026-10), `str_inflector` 1,412,705 | **Served** |
| Perceptual image hashing (aHash/dHash/pHash) | PyPI `imagehash` 5,224,426/mo; npm `blockhash-core` 56,847/wk, `imghash` 46,400/wk | `image_hasher` 488,789 (2026-02, 21 revdeps), `fast-dhash` 219,629, `img_hash` 35,089 | **Served** |
| Boilerplate removal and readability | PyPI `trafilatura` 12,938,590/mo, `readability-lxml` 1,336,059/mo; npm `@mozilla/readability` 3,695,092/wk | `dom_smoothie` 543,084 (2026-09, 43 revdeps), `readability` 391,082, `readabilityrs` 112,145, `readability-rust` 58,066, `legible` 32,623 | **Served, five deep** |

## The one opening: Open Location Code / Plus Codes

A Plus Code is a short, human-readable way to name any point on earth -
`9FFW84J9+XG` is a building in Stockholm, and the last four digits can be dropped
and recovered from a nearby reference (`84J9+XG` plus "I am in Stockholm" gets
the full code back). Google runs the reference implementation and it is
**Apache-2.0, 4,368 stars, actively maintained**.

The demand is real and both ecosystems have it: PyPI `openlocationcode`
**97,535 downloads a month**, npm `open-location-code` **39,242 a week**. It sits
beside `geohash` (1,036,977 recent in Rust), which is the same kind of thing -
a short geocode - and which Rust serves properly.

Rust does not serve it. Three crates exist and none of them does the job:

* **`open-location-code` (2,475 recent, v0.1.0, 2018-10-14).** This is the
  leading crate and it has had **one release in eight years**. Its metadata
  points at `github.com/google/open-location-code`, which is the *reference
  implementation's* repository, not this crate's - so the star count and the
  activity signal both belong to Google's code, not to this port. docs.rs
  reports **46.67% documented, 7 of 15 items, 0 examples**.
  Its test suite is the strongest evidence: it ships `tests/all_test.rs` that
  reads four CSV fixture files, and **the fixtures are not in the published
  crate**. `cargo test` fails on all three tests:

  ```
  thread 'is_valid_test' panicked at tests/csv_reader.rs:16:55:
  called `Result::unwrap()` on an `Err` value:
  Os { code: 2, kind: NotFound, message: "No such file or directory" }
  test result: FAILED. 0 passed; 3 failed
  ```

  A crate whose own test suite cannot run has never been verified against the
  reference's vectors. Its `exclude = ["rust.iml"]` in `Cargo.toml` is an IDE
  file, which says the packaging was set up by hand and never revisited.
* **`pluscodes` (291 recent, v0.5.0, 2022-11-21, 1 star).** It encodes and
  decodes a *full* code and nothing else. `grep` for `shorten`, `recover` or
  `short_code` across its source returns nothing, so the two functions that make
  a Plus Code worth having - the shortening and the recovery from a reference
  point - are absent. Five versions were published on three days in November
  2022 and then it stopped.
* **`olc` (3 recent, v0.0.0, 2021-06-22).** A v0.0.0 from 2021. Not a candidate.

The name `openlocationcode` is free on crates.io, and so is the one the reference
project itself would use.

### Why this one is worth building rather than noting

It is the same shape as `holidays-rs` and `aviation-data`, which this workspace
has already shipped: **an open dataset, a permissive licence, a small API, and a
regenerable verifier.**

* **The oracle is shipped by the reference.** `google/open-location-code` carries
  `test_data/` with four CSVs - **302 encoding, 420 decoding, 19 short-code and
  25 validity vectors, 766 in total** - and they are Apache-2.0 like the code.
  A differential gate over them is a direct port of `verify_aviation.py`: read
  the CSVs, drive the crate, diff field by field. The reference's own Rust port
  was *supposed* to do this and cannot, because it does not ship the files.
* **The licence is compatible.** Apache-2.0, which is the only licence this
  workspace may ship. No table has to be regenerated to avoid a copyleft
  inheritance, unlike the `num2words` case below.
* **The surface is small and closed.** Encode, decode, `shorten`, `recover`,
  `is_valid`/`is_short`/`is_full`. There is no algorithmic research left in it -
  the grid arithmetic is a fixed table of 20 characters and a pair of
  interval-narrowing loops. It is a weekend of transcription plus the gate.
* **The failure is a *correctness* gap, not a feature gap.** The incumbent's
  encode/decode is plausible; what is missing is that anyone ever proved it.
  That is exactly the opening `stdnum-rs` took.

## The opening that closed while this round looked at it

**Multilingual number-to-words looked like the strongest candidate in the sweep
and is not.** PyPI `num2words` is **7,789,143 downloads a month** and the Rust
crate named `num2words` (96,973 recent) supports **five languages** - English,
French, French-BE, French-CH and Ukrainian - which its README states plainly and
invites contributions to. That is a 56-to-5 language gap against a reference
whose own module directory holds 56 language files. It reads as the perfect
"port the reference" task.

It is already done. **`num2words2-core`** (27,645 recent) describes itself as
"Number-to-words conversion for **120+ languages** (Rust port of num2words)",
ships under **MIT OR Apache-2.0**, and its repository holds **307 `lang_*.rs`
files**. It was published 2026-07-17 and last updated 2026-10-05 - three months
ago. Building this now would be re-implementing a port that already exists and
already covers more languages than the Python reference.

Two lessons, both already in the rounds' method and both worth restating:

1. **A stale incumbent is not an absent one.** `num2words` at 96,973 recent and
   5 languages looks like a gap until you search the capability
   (`number to words`, `cardinal ordinal words`, `spell number words`) rather
   than the name. The name search returns the weak crate; the capability search
   returns the strong one.
2. **Check the reference's licence before planning the port.** PyPI `num2words`
   is **LGPL**, so its tables cannot be copied into an Apache-2.0 crate. A port
   has to be written from a permissive source or from scratch - which is what
   `num2words2` presumably had to do, and is a real cost that the download
   figure alone hides.

## The domain that is served but only just

**Human name parsing** is the one to keep an eye on rather than act on.
`human_name` (503,606 recent) is the only crate for it, and by its own README it
"will work best on Latin names", does not understand surname-first formats, and
does not handle single-word names. PyPI `nameparser` is **3,132,865 a month** and
`usaddress` **5,029,779**, and npm `parse-full-name` 225,568 a week, so the
demand is there.

What stops it being an opening is that the incumbent is not obviously failing:
v2.0.4 from 2024-10, a green build, and a README honest about its limits. The
signals that it is thin - **one reverse dependency and 61 stars** for the only
crate in a 3M-a-month category - are real but they are the same signals that
would justify a *second* implementation, not the first. Recorded, not proposed.

## What this round concludes

1. **Plus Codes are the opening, and it is a correctness gap with a shipped
   oracle.** The reference publishes 766 test vectors under Apache-2.0; the
   leading Rust crate cannot run its own tests because it does not ship the
   fixtures. That is the `aviation-data` shape and it is buildable this round.
2. **`num2words` is closed.** A 120-language MIT/Apache port landed in July.
   The five-language crate that the name search finds is a decoy.
3. **Six of eight domains are served, and two of them six and five times over.**
   Human-readable formatting (`num-format` 14.5M, `thousands` 7.5M,
   `duration-str` 3.8M, `human_format` 2.4M, `human_bytes` 1.6M) and readability
   (`dom_smoothie` 543K with 43 revdeps, plus four more) are not openings by any
   measure.
4. **The name `Inflector` at 19.5M recent is ARCHIVED and four years stale** -
   the round 12 "abandoned incumbent" pattern - but `pluralizer` (1.9M, 2026-10)
   has already taken the audience, so it is a migration, not a gap.
5. **Licence first, still.** The `num2words` LGPL finding is the second round in
   a row where the licence, not the demand, decides whether a port is possible.

## Method

```sh
python3 research/tools/round27.py sweep          # the eight domains
python3 research/tools/round27.py names <n>...   # is a proposed crate name free?
```

`round27.py` imports the measured helpers from `round15`. Every figure was
fetched live from `crates.io/api/v1`, `index.crates.io`, `api.npmjs.org`,
`pypistats.org` and `api.github.com` on 2026-10-21. The capability searches that
overturned the name-based passes used
`crates.io/api/v1/crates?q=...&sort=recent-downloads`.

Two checks went past the registry and are reproducible from this document:

```sh
# The leading crate's test suite, from its published tarball.
curl -sL https://static.crates.io/crates/open-location-code/open-location-code-0.1.0.crate \
  | tar xz && cd open-location-code-0.1.0 && cargo test    # 0 passed; 3 failed

# The reference's test vectors, which are Apache-2.0.
for f in encoding decoding shortCodeTests validityTests; do
  curl -sL "https://raw.githubusercontent.com/google/open-location-code/main/test_data/$f.csv" \
    | grep -vc '^#'
done                                                      # 302 420 19 25
```
