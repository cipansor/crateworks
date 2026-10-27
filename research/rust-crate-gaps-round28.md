# Gap scan round 28: npm demand against Rust supply, and the one wide opening (2026-10-21)

Rounds 5 to 27 swept capability by capability, mostly through the PyPI top list.
This round starts from the other end: **310 npm packages**, each paired with the
search terms that would find its Rust equivalent, and each measured for weekly
downloads on npm and all-time downloads on crates.io. The question is not "is
the name free" but "is the job done, and is the crate doing it adopted".

The answer is that the Rust ecosystem is far more complete than the popularity
lists suggest. **Of 310 candidates, 255 have a Rust crate with real adoption.**
Three looked like gaps on the first pass; two of those dissolve when the
incumbent is read properly. One does not.

## Method

`npm_rust_gap.py` (this round) walks a curated candidate list. For each npm
package it fetches the weekly download count, then searches crates.io for every
term paired with it and keeps the most-downloaded crate found. A candidate is
flagged only when npm demand is high **and** the best Rust crate is under
200,000 all-time downloads.

That flag is a starting point, not a verdict, because a low download count is
not proof of absence. `verify_shortlist.py` therefore re-checks every flagged
candidate by **name**, not by capability words, since round 22 established that
the search endpoint misses a crate whose name differs from the capability. A
gap is only called real when the name pass is also empty or trivially small.

## The three flags, and what each turned out to be

| Candidate | npm demand | Best Rust crate | Verdict |
| --- | --- | --- | --- |
| HTML/CSS to paginated PDF | PyPI `weasyprint` **31,898,679/mo**, `reportlab` 66,000,604/mo, `xhtml2pdf` 3,625,421/mo | `fullbleed` 5,028 recent, `fulgur` 3,239, `ironpress` 2,538, `renderium` 2, `jhtmltopdf` 37 | **Open, and wide** |
| Web Push (RFC 8030/8291/8292) | npm `web-push` **4,414,790/wk** | `web-push` 498,280 recent (2025-02), `web-push-native` 196,868, `ece` 673,122, `vapid` 476 (2022) | **Open but narrow** - the pieces exist, the assembly does not |
| Dedicated Anthropic SDK | npm `@anthropic-ai/sdk` **22,538,770/wk** | `anthropic-sdk` 3,696 recent (2024-07), `async-anthropic` 4,995 | **Closed in substance** by multi-provider clients |

### Anthropic: closed by the multi-provider libraries

The dedicated crates are all stale - `anthropic-sdk` last shipped 2024-07-23,
`anthropic` 2024-09-03, `async-anthropic` 2025-05-03 - which is what put the
candidate on the flag list. But the capability is not missing: `rig-core`
(**1,958,364 recent**) and `genai` (**249,634 recent**) both front Anthropic
alongside OpenAI and Gemini, and both shipped within the last two weeks. A
developer who wants Claude from Rust reaches for one of those, not for a
single-provider SDK. The gap is in the *shape* of the ecosystem, not in the
capability, and that is not an opening.

### Web Push: real, but the hard part is already written

Every cryptographic piece is present and maintained: `ece` (673,122 recent,
2026-09-15) does RFC 8188 encrypted content encoding, `p256` (66,990,097
recent) supplies the VAPID key type, `aes-gcm` and `jwt-simple` are the rest of
RFC 8291 and 8292. What is missing is the client that assembles them.

`web-push` (498,280 recent) is that client, and it has not shipped since
**2025-02-22** - twenty months. `web-push-native` (196,868 recent) is newer
(2026-07-26) and is a serious attempt, so this is a race with two entrants
rather than empty ground. The opening is real but the work is mostly
orchestration over crates that already exist, which caps how much of a
differentiator it can be.

## The finding: HTML/CSS to paginated PDF

**No round has ever scanned this.** `weasyprint`, `paged media`, and
`html-to-pdf` appear in zero of the 27 prior reports. Round 18 covered EPUB and
typesetting LaTeX-to-MathML; round 7 recorded PDF *generation* as served by
`typst-as-lib` and `oxidize-pdf`. Neither is this job.

The job is: take HTML and a stylesheet that a designer already wrote, apply
CSS paged-media rules (`@page`, page breaks, running headers, `counter(page)`),
and emit a PDF. That is what WeasyPrint does, and the demand is enormous and
sustained:

* PyPI `weasyprint` **31,898,679/month**
* PyPI `reportlab` 66,000,604/month (the incumbent it usually replaces)
* PyPI `xhtml2pdf` 3,625,421/month
* PyPI `pdfkit` 2,960,557/month

The Rust side is six one-author attempts, none adopted:

| Crate | Recent 90d | Version | Last release |
| --- | --- | --- | --- |
| `fullbleed` | 5,028 | 2.5.39 | 2026-10-27 |
| `fulgur` | 3,239 | 0.40.0 | 2026-07-31 |
| `ironpress` | 2,538 | 1.7.0 | 2026-09-16 |
| `spindrift` | 103 | 0.1.0 | 2026-09-04 |
| `jhtmltopdf` | 37 | 0.2.1 | 2026-09-02 |
| `renderium` | 2 | 0.0.1 | 2026-07-23 |

The leader has **5,028 recent downloads against 31.9 million a month on PyPI** -
0.05% of the reference volume, which is the same shape round 22 measured for
cron descriptors.

The alternative today is to drive a browser: `chromiumoxide` (2,165,990
recent), `headless_chrome` (741,505 recent), or `pdfium-render` (1,723,461
recent). That works and it is what most Rust services do, but it drags a
Chromium binary into the deployment, it is not deterministic across versions,
and it is far too heavy for generating an invoice. `wkhtmltopdf` bindings exist
and last shipped **2021**.

`typst` (1,694,156 recent) is the healthy Rust typesetting story, but it is a
markup language of its own - it does not take HTML and CSS, so it does not
serve anyone who already has a template.

### Why this is the opening worth taking

The three conditions that make a gap actionable all hold, and they hold
together rather than separately:

1. **Demand is proven in two ecosystems** - WeasyPrint on PyPI, `pdfmake`
   (1,864,382/week) and `pdfkit` on npm.
2. **The reference implementation is readable and permissively licensed.**
   WeasyPrint is BSD-3-Clause and its test suite is the specification: it
   carries a corpus of HTML/CSS inputs with expected PDF layout. That is a
   differential gate of the kind `verify_spreadsheet.py` provides for
   `spreadsheet-formula`.
3. **The incumbents are thin, not entrenched.** Six crates, all under 5,100
   recent, all one author, none with the layout engine complete enough to have
   won an audience. This is not a race with a funded entrant.

The honest cost is that a CSS layout engine is large - box tree, cascade,
line breaking, pagination - and it is the biggest thing proposed in any of the
28 rounds. It is also the only candidate this round found where the demand is
in the tens of millions and the supply is under six thousand.

## What the other 252 candidates showed

They are served, and mostly well. The ones that looked weakest and turned out
to be fine:

| Capability | npm/PyPI demand | Rust crate that serves it | Recent 90d |
| --- | --- | --- | --- |
| Excel formula evaluation | npm `formulajs` 7,674/wk, PyPI `formulas` 3,303,192/mo | `formualizer-eval` (2026-10-25), `xlformula_engine` | 8,999 / 2,452 |
| DOCX templating | npm `docxtemplater` 726,894/wk, PyPI `docxtpl` 3,462,825/mo | `docxtplrs` (2026-10-22), `docx-handlebars` | 576 / 2,458 |
| PPTX generation | npm `pptxgenjs` 4,344,862/wk, PyPI `python-pptx` 60,210,371/mo | `ppt-rs` (2026-10-24) | 49,694 |
| Human-readable cron | npm `cronstrue` 2,769,111/wk | `cron-descriptor` | 10,835 |
| DOCX to HTML | npm `mammoth` 5,103,692/wk | `office_oxide` | 727,039 |
| Barcode generation | npm `jsbarcode` 1,225,641/wk | `rxing` | 218,828 |
| Fuzzy search | npm `fuse.js` 10,530,628/wk | `nucleo`, `fuzzy-matcher` | adopted |
| Money and currency | npm `dinero.js` 423,825/wk | `rust_decimal`, `money` | adopted |
| File watching | npm `chokidar` 190,998,579/wk | `notify` | adopted |

DOCX templating and human-readable cron are already recorded as open
candidates in rounds 9, 18, 22 and 23. The Excel formula engine is the
capability this workspace is itself building in `spreadsheet-formula`; the
reference here is small (`formulajs` 7,674/week), which is why the crate is
extended rather than proposed as a gap.

## Recommendation

Take **HTML/CSS to paginated PDF**. It is the only candidate across 28 rounds
with demand in the tens of millions of monthly downloads and a Rust leader
under six thousand. Everything else in this scan is either served or already
recorded as a narrower opening.

The first slice should be the one that makes the differential gate possible:
port WeasyPrint's own test corpus, run it against the layout engine, and treat
any layout difference as a failure. That is the same discipline that turned
`spreadsheet-formula` from a partial implementation into 525/525 agreement.

## Reproduce

```
python3 research/tools/npm_rust_gap.py     # 310 candidates, npm vs crates.io
python3 research/tools/verify_shortlist.py # name-level check of the flagged few
```

Both use only public APIs and need no key. `npm_rust_gap.py` writes
`gap_rows.json`; `verify_shortlist.py` writes `shortlist.json`.
