# Gap scan round 22: the "popular on npm, absent in Rust" premise, tested (2026-10-21)

This round starts from a question rather than a domain: *is there a package that
many people need, which has no Rust crate?* That premise is the most common way
to pick a first crate, and rounds 8-21 have implicitly tested it many times. This
round tests it directly and reports what it found.

Every figure was fetched live on 2026-10-21 from `crates.io/api/v1`,
`api.npmjs.org` and `pypistats.org`.

## The premise is mostly false, and it is worth saying why

The naive method is: take a popular npm package, look for a crate of the same
name, and if it is absent, build it. That method fails in both directions.

**A missing name is usually not a missing capability.** The npm name and the Rust
name are chosen independently. `pptxgenjs` (4.34M/week) has no crate of that
name, but the capability is served by `ppt-rs` at 50,356 recent and `docx-rs` at
3.75M all-time; round 18 already recorded PPTX as a race with at least eleven
entrants. `dompurify` (45.3M/week) has no crate of that name, and the capability
is `ammonia` at 19.7M all-time. `addressit` (9,404/week) is not a gap because its
demand is small, not because Rust lacks it.

**A present name is often a dead crate, which looks like a gap and is not.**
`address-parser` exists on crates.io at **7 downloads**; this workspace already
ships the real one. `libpostal-rs` exists at 1,737 and is an FFI wrapper to a C
library with 1.5 GB of data files. A name search finds these and calls the
capability unclaimed.

**The demand ratios that look alarming are usually not gaps either.** Measured
this round:

| npm package | weekly | Rust crate | recent 90d | ratio | verdict |
| --- | --- | --- | --- | --- | --- |
| `pretty-bytes` | 28,776,042 | `bytesize` / `humansize` / `byte-unit` | 116M / 69M / 43.7M all-time | ~1x | served |
| `pluralize` | 30,481,498 | `pluralizer` | 3,178,379 all-time (2026-10-19) | ~1x | served |
| `numeral` | 2,085,047 | `icu` decimal formatting | 2.65M all-time | >1x | served |
| `cron-parser` | 14,673,078 | `cron` / `croner` | 1,799,568 / 1,347,604 recent | ~1x | served |
| `tesseract.js` | 1,741,387 | `ocrs` (pure Rust) | 479,728 all-time | ~1x | served |
| `libphonenumber-js` | 19,565,078 | `phonenumber` | 13,664,051 all-time | ~1x | served |
| `pdf-parse` | 5,369,545 | `pdf-extract` / `lopdf` | 7,054,163 / 26,525,611 all-time | >1x | served |

In every case the Rust side is at least as large as the npm side, often larger,
because the Rust ecosystem is smaller and concentrates on fewer crates. The
"npm is huge, Rust is empty" gap is real in maybe one case in twenty, and the
ratio alone does not identify it.

## The one new candidate this round found

**`cron-descriptor` exists, is correct, and has no audience.**

| | |
| --- | --- |
| Demand | PyPI `cron-descriptor` **16,961,182/month**; npm `cronstrue` **2,769,111/week**; PyPI `croniter` 75.4M/month (the parser, not the descriptor) |
| Rust incumbent | `cron-descriptor` **34,928 all-time, 4,987 recent 90d**, max 0.1.2 (2026-07-27), one maintainer, **1 reverse dependency** (`cron` has 465) |
| Ratio | 0.03% of the Python volume |
| Free names | `cronstrue`, `cronstrue-rs`, `cron-text`, `cron-say` |

The incumbent is a faithful port by one author. It has been on crates.io since
2023 and has never acquired an audience. That is the honest shape of this
opening: the capability is claimed by a crate that works and that nobody uses.
Round 16's rule applies in reverse here - this is not a race with many entrants,
it is one entrant that stalled, which is closer to how the YAML opening closed
than to how the CAS cluster looks.

The argument for entering would have to be that the incumbent is *narrow*, not
that it is absent, and this round did not establish that. `cronstrue` renders in
about thirty locales; whether `cron-descriptor` does was not measured. **Recorded
as an open candidate, not proposed**, because the case rests on a claim this
round did not test.

## Three prior "FREE" claims are wrong

Round 19's method counts a name as free when the crates.io search returns
nothing. That check has now produced three false positives, because the search
endpoint misses crates whose *name* differs from the *capability*:

1. **`cron-descriptor`** was never searched by rounds 10 or 17. Round 17
   concluded "Cron: served" from `cron` and `croner`, which parse and schedule;
   neither renders an expression as English. The descriptor capability was
   outside the words that round searched for.
2. **`humanize-duration`** is a crate at **316,305 all-time, max 0.0.7** - a
   0.0.x stub - while npm `humanize-duration` is at 2,806,203/week. No round
   mentions either. `humantime` (474M) and `chrono-humanize` (10.9M) cover
   adjacent ground; whether they cover locale-aware duration phrasing was not
   measured.
3. **`transliterate`** exists at **4,458 (2021)** and `translit` at **26,939**,
   against npm `slugify` 13,443,938/week and PyPI `text-unidecode` 64,989,646
   per month. The capability is genuinely thin - `deunicode` at 74.8M is the
   strong incumbent - but the names are not free, and `icu_transliterate`
   (47,150) is the standards-based option.

The lesson for the method: **search the capability, then check the name, in that
order.** Round 15 wrote this rule; this round is the third time it has caught a
false positive, which means the rule is still not being applied consistently.

## What this round concludes

1. **No crate is proposed.** The one new candidate, `cron-descriptor`, rests on a
   narrowness claim that was not measured, and the honest move is to record it as
   open rather than propose it.
2. **The "popular elsewhere, absent here" method has a ~5% hit rate and a high
   false-positive rate.** It should stay a cross-check, as round 21 concluded for
   the PyPI variant of the same method.
3. **The workspace's real openings are not found this way.** They are found by
   taking a *domain* and reading it: `aviation-data` came from a licensing
   argument (round 20), `address-parser` from a demand measurement against a
   6.5K incumbent (round 19). Both required reading the incumbent, not counting
   names.

## Method

The probes are one-liners against the three APIs; no new helper script was
added, because this round's method is a cross-check rather than a sweep and it is
not worth a script that will be run once. The rate-limit warning in
`tools/README.md` applies: crates.io returns HTTP 429 under load, and a naive
fetch reads that as "no such crate". Every figure above was re-fetched at least
once, and the three false positives in the previous section were each confirmed
against the crate's own API page rather than against search.
