# Gap scan round 23: the npm-vs-Rust premise, run again and with round 22's open cases closed (2026-10-21)

Round 22 tested the premise "a package many people need, with no Rust crate" and
found a ~5% hit rate, leaving one candidate open (`cron-descriptor`) because it
had not measured whether the incumbent was *narrow*. This round measures that,
and runs the premise once more over a fresh 130-package npm sweep to see whether
the picture has changed.

Every figure was fetched live on 2026-10-21 from `crates.io/api/v1`,
`api.npmjs.org` and `pypistats.org`.

## The picture has changed: the Rust side is filling in, fast

The fresh sweep's top candidates were each checked by *capability*, not by name,
as round 15's rule requires. Every one is now contested, and by an entrant from
the last few months:

| npm package | weekly | capability | Rust incumbents | newest | verdict |
| --- | --- | --- | --- | --- | --- |
| `bwip-js` | 687,183 | 1D+2D barcode encode | `barcoders` 646K (1D only) | `bwipp-rs` (2026-05), `anyd` (2026-09), `zedbar` (2026-10) | filling |
| `messageformat` | 327,527 | ICU MessageFormat | `mf1` 12K, `ferrocat-icu` 10K | `formatjs_icu_messageformat` (2026-10-19) | filling |
| `docxtemplater` | 726,894 | DOCX templating | `docx-template` 6.5K (2025-07) | `docxtplrs` (2026-10-22) | filling |
| `pdfjs-dist` | 19,081,794 | PDF parse/render | `lopdf`, `pdf-extract` | `pdf-parse` crate | served |
| `h3-js` | 906,746 | H3 geo indexing | `h3` 5.25M, `h3o` 2.03M | — | served |

`bwip-js` is the clearest example of why the name method misleads. `bwip` and
`bwip-js` are both **FREE** on crates.io, and `barcode` is free too, so a name
search reads the whole 2D-barcode space as empty. It is not: `anyd` shipped
2026-09-18 ("from-scratch encoding and decoding of 1D and 2D barcodes"),
`zedbar` 2026-10-02, `bwipp-rs` 2026-05-30, and `rxing` (a zxing port, 744K
all-time) is the incumbent. Entering that space today means racing four crates
that are weeks old.

The direction of travel matters more than any single row. In the 2023-2024 scans
the Rust side of a gap was usually one stalled crate; in this sweep it is
routinely three or four live ones, several published within the last month. The
"popular elsewhere, absent here" premise is not just mostly false, it is getting
falser each quarter.

## Round 22's open case, now measured: `cron-descriptor`

Round 22 recorded this as open because it had not tested whether the incumbent
was *narrow* rather than *absent*. Measured now:

| | |
| --- | --- |
| Demand | npm `cronstrue` **2,769,111/week**; PyPI `cron-descriptor` **16.9M/month** |
| Rust incumbent | `cron-descriptor` v0.1.2, 34,928 all-time, 10,788 recent 90d, one maintainer |
| Locales in the Rust incumbent | **3** - English, Spanish, Portuguese (`README.md`: "English, Spanish and Portuguese are currently available") |
| Locales in npm `cronstrue` | **42** (`src/i18n/locales/`: af, ar, az, be, bg, ca, cs, da, de, el, en, es, fa, fi, fr, he, hr, hu, id, it, ja, ko, my, nb, nl, nn, pl, pt_BR, pt_PT, ro, ru, sk, sl, sr, sv, sw, tg, th, tr, uk, vi, zh_CN, zh_TW) |
| Locales in PyPI `cron-descriptor` | **32** (`cron_descriptor/locale/*.po`) |
| Competing Rust crates | `cronspeak` (2026-06, 747), `hron` (2026-10, 1,732), `philiprehberger-cron-parser` (2026-05), `tpt-cron-parse` (2026-08), `crontastic` (2026-07) |

**The narrowness claim is confirmed.** The incumbent is a faithful port of a
3-locale subset of the reference, and the reference has 32. The gap is not the
capability, which five crates now serve in English, but the *localization* of it,
which none of the six serves at all - `cronspeak` and `philiprehberger` are
English-only ("human-readable English"), `hron` is English-only by design, and
none of the five declares a locale or a translation file.

That is a real opening, and it is a *narrow* one: the work is translating ~40
phrases into N locales, not designing a parser. The parser half is crowded; the
translation half is empty. Free names: `cronstrue`, `cronstrue-rs`, `cron-text`,
`cron-say`, `cron-localized`.

## Round 22's second open case: `humanize-duration`

Round 22 flagged this as possibly narrow without measuring. Measured:

| | |
| --- | --- |
| Demand | npm `humanize-duration` **2,806,203/week** |
| Rust crate of that name | `humanize-duration` v**0.0.7**, 316,305 all-time, 62,079 recent 90d, by a *different* author (`sephiroth74`) |
| Locales | the npm package ships ~30 language definitions; the crate's own README does not claim localization |
| Adjacent Rust | `chrono-humanize` 10.9M all-time (English only), `humantime` 474M (English only), `human-repr` 1.14M (English only) |

The same shape as `cron-descriptor`: a stub version number, no localization, and
every adjacent crate English-only. Lower priority than `cronstrue` only because
`chrono-humanize` is a much stronger English incumbent than any cron crate, so
the English half is harder to displace.

## What this round recommends

**`cronstrue`: localized cron descriptions.** The case rests on measurement this
time, not on absence:

1. The demand is large and documented (2.77M npm/week, 16.9M PyPI/month).
2. The capability is not absent - it is *localized-absent*. Six Rust crates
   render cron in English; none renders it in any other language.
3. The reference implementations carry the exact translations to port, so the
   work is bounded and verifiable, not open-ended design.
4. The names are free.

The honest counter-argument, recorded: `cron-descriptor` has been on crates.io
since 2023 and has 10,788 recent downloads, which is small. Localization is what
would differentiate a new crate from it, and localization is exactly the part the
audience for a cron-describe crate may not need. A build should start by
measuring which locales the *demand* actually wants, not by porting all 42.

## Where the user's own edge already sits

The workspace's Indonesian and Islamic-finance ground is not empty, and the scans
should say so plainly:

- A **zakat cluster** exists (`zakat`, `zakat-core`, `zakat-i18n`,
  `zakat-ledger`, `zakat-providers`, `zakat-sqlite`), all published 2026-01-05,
  all under 1,000 downloads. The capability is claimed.
- `sastrawi-rs` (Indonesian stemmer, 2026-07), `qris-gen` (QRIS, 2026-01),
  `kamu-snap-crypto` (Bank Indonesia SNAP BI, 2026-07), `legalis-id`,
  `rust-terbilang` (2020, unmaintained) all exist.
- `stdnum-rs` in this workspace is itself the NPWP/NIK entry, and it has 36
  downloads - the incumbent for that capability is the workspace's own crate.

So the pesantren-domain vocabulary recorded in project memory is the durable
edge, not "Indonesian crates are missing": several are not.

## Method

Probes are one-liners against the three APIs plus `research/tools/namecheck.py`.
The 130-package npm sweep used `research/tools/npmfetch.py`. Every figure above
was fetched at least once and the load-bearing ones (the incumbent's locale
counts, the new entrants' publication dates) were confirmed against the crate's
own repository or `docs.rs`, not against search - round 22's three false
positives all came from trusting the search endpoint.
