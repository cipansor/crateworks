# Gap scan round 15: the survivors re-measured, and seven utility domains (2026-10-24)

Round 14 closed with `fuzzy-matcher` as "the strongest opening measured", and
that crate was built inside this workspace as `fuzzy-score` before this round
ran. That is the fourth time a round's headline was acted on and the fourth time
the re-measurement is the work, so this round does the same two jobs as rounds
13 and 14: measure every survivor again, and sweep domains no earlier round
touched.

Every figure was fetched on 2026-10-24 from `crates.io/api/v1`, `index.crates.io`
(the sparse index, for a real release date), `api.npmjs.org`, `pypistats.org` and
`api.github.com`. A gap is real when the Rust incumbent has under about 20,000
recent downloads and near-zero reverse dependencies, while the source
counterpart has real volume. Recent downloads are the rolling ninety-day figure.

## The surviving candidates, re-measured

| crate | recent | last release | repo | verdict |
| --- | --- | --- | --- | --- |
| `fuzzy-matcher` | 9,616,350 | 2020-10-04 | **archived**, 469 revdeps | shipped here as `fuzzy-score` |
| `im` | 4,767,839 | 2022-04-29 | **archived**, 238 revdeps | contested by `rpds`, watch |
| `smartstring` | 13,231,596 | 2022-03-24 | **archived** | closed by `compact_str` (67M) |
| `multipart` | 5,782,645 | 2021-05-29 | **archived** | closed by `multer` (29M) |
| `rustls-pemfile` | 78,565,016 | 2024-09-30 | **archived** | closed by `pem`/`pem-rfc7468` |
| `ofx-rs` / `ofxy` / `qif_parser` | 2,914 / 1,311 / 367 | 2026 | tiny, 0-1 revdeps | **still open**, unchanged |
| `subtp` / `subparse` | 20,493 / 4,864 | 2024 / 2021 | 3 / 9 revdeps | **still open**, unchanged |

### `fuzzy-matcher` is now served, by this workspace

`fuzzy-score` v0.1.4 published 2026-10-23 from `crates/fuzzy-score`, and the
round-14 document is why it exists. The archived incumbent still carries 469
reverse dependencies and 9.6M recent downloads, so the migration has not
finished — but the opening round 14 named is no longer empty. Do not re-propose
it.

### The other three archives are all closed, and for the same reason

Round 14 closed `smartstring` by finding `compact_str`. This round finds the
identical shape twice more:

- **`multipart` is archived** with 58 reverse dependencies, but `multer` does
  29,103,215 recent downloads, released 2024-05-04, and has 184 reverse
  dependencies. That is five times the audience, maintained, covering the same
  multipart-stream parsing. **Do not build a multipart crate.**
- **`rustls-pemfile` is archived** with 1,378 reverse dependencies, the largest
  count seen in any round — but `pem` does 96,017,319 recent downloads (released
  2026-07-28) and `pem-rfc7468` does 100,477,283 (released 2025-11-08). PEM
  parsing is served twice over. **Do not build a PEM crate.**

The pattern is now unambiguous and worth stating as the round's method finding:
**an archived crate with 100+ reverse dependencies is a migration in progress,
not an empty space.** Three rounds have now closed four archives this way
(`serde_yaml`, `smartstring`, and these two). A high reverse-dependency count
measures the *past*, not the opening.

### `im` stays a watch, not a build

`rpds` did 4,843,141 recent downloads, released 2026-05-15, 1,783 stars, 79
reverse dependencies. `ordermap` (7,184,018 recent, released 2026-09-05) adds a
second maintained option in the same space. `im` still has 238 reverse
dependencies, so the migration is not finished, but nothing here is empty. Watch.

## The seven utility domains

Earlier rounds all searched the npm and PyPI *popularity* lists, which are
dominated by web frameworks and data-science packages. This round deliberately
picked seven utility domains those lists under-represent. **Six of the seven are
already served.** That is the result, and it is worth recording row by row so no
later round re-scans them.

### Served: human-readable formatting

`humanize` (148 recent) and `filesize` (754,401) are the weak spots, but the
capability is covered by maintained crates with large audiences: `bytesize`
23,103,746, `humansize` 13,389,598, `num-format` 14,527,069, `duration-str`
3,816,811, `parse-size` 3,898,110. npm `pretty-bytes` (19,981,092/week) and PyPI
`humanize` (46,399,361/month) have no Rust crate *by that name*, but the
capability is served several times over. **Do not enter.**

### Served: natural-language dates

PyPI `dateparser` 38,372,577/month, npm `chrono-node` 1,395,930/week. Rust:
`dateparser` 2,089,112 recent (released 2026-03-25, 64 revdeps) and
`chrono-english` 391,770 (released 2026-08-26). Two maintained crates for a
small niche. **Do not enter.**

### Served: typography and hyphenation

PyPI `pyphen` 32,451,084/month. Rust: `hyphenation` 309,301 (2021) is stale, but
`hypher` — now maintained by **typst** — does 1,825,547 recent downloads and
shipped 2026-09-17, and `textwrap` does 62,438,400. Hyphenation is served by the
same project that maintains a major Rust typesetting engine. **Do not enter.**

### Served, and this is the round's sharpest correction: text repair (ftfy)

`ftfy` looked like the round's best find — 13,881,709 PyPI downloads a month, the
name `ftfy` **free** on crates.io, and a search for `ftfy`/`mojibake` returning
only a 3-download joke crate. It is not a gap. **`plsfix` is a Rust
reimplementation of ftfy**, published both as a PyPI package and as a Rust crate
(57,028 recent downloads, `github.com/kevinhu/plsfix`), and its own README calls
itself "drop-in replacement for ftfy, written in Rust and ~10x faster". The
search missed it because its name and description ("Text cleaner upper") contain
neither the word `ftfy` nor `mojibake`. The transliteration and sanitising
neighbours are served too: `deunicode` 17,985,791, `slug` 8,394,224, `ammonia`
6,270,979, `encoding_rs` 133,224,213, `chardetng` 6,561,033. **Do not build an
ftfy port.** The lesson: search by *capability*, not by the upstream package's
name, and read the nearest competitor's README before declaring a gap.

### Served: MIME and file-type sniffing

PyPI `python-magic` 24,443,408/month, `filetype` 35,406,517/month, npm
`file-type` 36,586,834/week. Rust: `infer` 32,695,217, `mime` 167,878,428,
`mime_guess` 80,273,649, `tree_magic_mini` 10,351,276. **Do not enter.**

### Served: web feeds and crawler policy

PyPI `feedparser` 16,632,653/month, npm `sitemap` 3,823,820/week,
`robots-parser` 3,630,668/week. Rust: `feed-rs` 615,899, `rss` 608,681,
`atom_syndication` 493,908, `texting_robots` 455,234, plus a dozen robots.txt
parsers (`robotstxt` 69,245, `robotxt`, `fast-robots`, `susbot`, and more). The
feed side is covered and the robots.txt side is *crowded*. **Do not enter.**

### Served: geo hashing and spatial indexing

PyPI `h3` 14,389,948/month, `geohash2` 1,129,341/month. Rust: `geohash`
1,033,151 (2026-05-24), `h3o` 533,394 (2026-08-29), `s2` 129,919, and `geo`
4,952,453. `h3ron` is archived (10,292) but `h3o` replaces it. **Do not enter.**

## What is genuinely open

Two candidates survived, both in document and text extraction, both extending
lines this workspace already ships.

### Candidate: PDF table extraction

| | |
| --- | --- |
| Demand | PyPI `pdfplumber` **40,689,748/month**, `camelot-py` 721,592/month, `tabula-py` 1,157,398/month. |
| Best Rust crate | `pdf-extract` 3,606,239 recent, but it extracts *text*, not tables. `pdfplumber` (Rust) is 41,780 recent, v0.4.1, self-described **"Alpha evidence-driven"**. `tabula` (42 recent) is a Java binding. `pdf_tables` is 1 download. |
| Shape | The demand is large and the Rust side is one alpha crate and one abandoned binding. |
| Names free | `pdftable`, `pdf-table-rs`, `pdftab`, `tabula-rs` |

This is the strongest candidate of the round. Table extraction is a distinct
capability from text extraction — it needs column and row inference from
coordinates — and no Rust crate does it to production quality. It extends the
workspace's existing PDF line (`pdfa`, `pades`, `pdfmake`) and the same
"transcribe a corpus of real files and compare against a reference" method
applies. The `pdfplumber` demand is one of the largest numbers measured in any
round.

### Candidate: readability / main-content extraction

| | |
| --- | --- |
| Demand | PyPI `trafilatura` 12,938,590/month, `readability-lxml` 1,336,059/month. npm `@mozilla/readability` 1,792,340/week. |
| Best Rust crate | `dom_smoothie` 523,015 recent (2026, actively maintained) — **this is the incumbent and it is real**. `readability` 403,652. `trafilatura` (Rust) only 13,290. |
| Shape | `dom_smoothie` is maintained and covers the Mozilla Readability algorithm, which is most of the demand. |
| Names free | `readability-rs` (name is TAKEN, dead since 2024), so `article-rs`, `main-content` |

**This one is weaker than it first looked, and honestly it is a watch.** The
round-14 rule ("an archived incumbent with a maintained successor is a migration
in progress") applies in reverse here: `dom_smoothie` is the successor and it is
already live. The only opening is `trafilatura`-specific extraction (comments,
metadata, multi-page), which is a feature gap, not a capability gap. Build only
if the workspace wants a second web-extraction crate next to its XML line; do
not treat it as a measured opening.

## Rejected this round

- **Number to words.** PyPI `num2words` 7,789,143/month. Served: `num2words`
  96,566 recent (released 2024-03-17), `numeral` 5,347, `text2num` 21,932,
  `italian_numbers` 13,837. Thin but present.
- **Short IDs.** `nanoid` 6,048,845, `ulid` 17,372,005, `base62` 3,472,799,
  `sqids` 79,694. Served many times over.
- **Color.** `palette` 19,950,035, `csscolorparser` 14,468,957, `colorsys`
  188,052. Served.
- **String distance / phonetics.** `strsim` 255,960,938, `rapidfuzz` 4,631,314,
  `jellyfish`, `rphonetic` 384,623. Served.
- **Pluralisation.** `pluralizer` 1,885,915, `inflector` 19,451,012. Served.
- **HTML to Markdown.** npm `turndown` is the reference; Rust has `htmd`
  6,002,281 (a Turndown port), `html2md` 280,082, `fast_html2md` 108,052.
  Served.
- **Text diff.** `similar` 63,205,146, `diff` 45,227,226, `dissimilar`
  13,337,646, `imara-diff` 8,754,292. Served.
- **Spell check.** `spellbook` 152,249, `zspell` 45,913. Served.
- **Units and quantities.** `uom` 2,674,729, `dimensioned` 118,819. Served.
- **Address parsing.** `usaddress` 3,189 recent, `libpostal` 1. PyPI `libpostal`
  is a C library with no registry demand figure, and address parsing is
  data-set-bound (the same shape round 13 rejected for postcodes). Not a
  measured gap.
- **Roman numerals.** `roman-numerals` 1,176,568. Served.
- **SQL formatting.** `sqlformat` 4,604,022, `sqlparser` 15,581,216. Served.
- **Unicode confusables / homoglyphs.** `unicode-security` 4,729,604. Served.
- **CJK segmentation.** `jieba-rs` 1,835,297, `lindera` 1,269,764. Served.
- **Spreadsheet read/write.** `calamine` 5,894,617, `rust_xlsxwriter` 2,523,885,
  `umya-spreadsheet` 509,536. Served, and the workspace already ships
  `spreadsheet-formula`.

## What this round concludes

1. **PDF table extraction is the strongest opening measured.** 40.7M/month for
   `pdfplumber` alone, and the Rust side is one self-described alpha crate plus
   an abandoned Java binding. Build this first; it extends the `pdfa`/`pades`/
   `pdfmake` line.
2. **The re-measurement rate is now the most reproducible finding in fifteen
   rounds.** Round 15 closed two more archives (`multipart`, `rustls-pemfile`)
   on the `compact_str` pattern, and found that round 14's own top pick had been
   shipped here. Four of the last five "strongest candidates" were stale or
   already acted on. The re-measurement *is* the work.
3. **`ftfy` is the sharpest correction.** It looked like the round's best find —
   13.9M/month and a free name — and `plsfix` already reimplements it in Rust.
   Search by capability, not by upstream name, and read the nearest competitor's
   README.
4. **Six of seven new utility domains are served.** Human-readable formatting,
   natural-language dates, typography, text repair, MIME sniffing, web feeds and
   geo hashing all have maintained Rust crates with large audiences. Do not
   re-scan them for "is there a crate".
5. **Readability extraction is a watch, not a build.** `dom_smoothie` (523,015
   recent) is the maintained incumbent; only `trafilatura`-specific features are
   open.
6. **OFX/QIF and subtitles are still open and unchanged**, in the order round 14
   ranked them (OFX better on fit, subtitles better on volume).

## Method note

The script is `tools/round15.py`:

```sh
python3 round15.py candidates   # re-measure the open survivors
python3 round15.py sweep        # the seven new utility domains
python3 round15.py names <name> # is a proposed name still free?
```

Two method corrections from round 14 carry forward and were re-confirmed: the
sparse index field is `pubtime`, not `pubdate` (a wrong field name fails
silently as an empty string), and the index path is `2/<name>` for two
characters, `1/<name>` and `3/<c>/<name>` otherwise. The rate-limit warning in
`tools/README.md` applies in full: crates.io returns HTTP 429 under load and a
naive fetch reads that as "no such crate". Every `FREE` here was re-checked, and
the script sleeps 0.7s between crates.io requests and retries a 429 with a
growing backoff.

One new method rule, from the `ftfy`/`plsfix` near-miss: **a keyword search on
the upstream package's name is not a gap check.** `plsfix`'s description is
"Text cleaner upper" and its name contains neither `ftfy` nor `mojibake`, so the
search returned it only under `mojibake` by accident. Before writing a gap down,
search the crates.io *description* for the capability and open the README of the
closest-looking crate.
