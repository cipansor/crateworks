# Gap scan round 18: the survivors re-measured, and seven document and text-processing domains (2026-10-26)

Round 17 closed with runtime unit conversion as its find and confirmed the
"actively-claimed cluster" rule: before proposing a crate, sort the nearest
crates by release date, not only by downloads. This round does the same two jobs
rounds 13 through 17 did: measure every survivor again, then sweep domains no
earlier round touched.

Rounds 8-15 searched the npm and PyPI popularity lists and then utility domains;
round 16 swept specialist protocols; round 17 swept scientific and everyday-data
domains. This round picks seven **document, bibliography and text-processing**
domains, which those lists under-represent but which every publishing, academic
and localisation pipeline depends on: bibliography (BibTeX/RIS/CSL), office
documents (DOCX/ODT/RTF), EPUB, typesetting (LaTeX to MathML), diff and patch,
segmentation (CJK and sentences), and emoji and IDNA.

Every figure was fetched on 2026-10-26 from `crates.io/api/v1`,
`index.crates.io` (the sparse index, for a real release date), `api.npmjs.org`,
`pypistats.org` and `api.github.com`. A gap is real when the Rust incumbent has
under about 20,000 recent downloads and near-zero reverse dependencies, while
the source counterpart has real volume. Recent downloads are the rolling
ninety-day figure.

## The surviving candidates, re-measured

| crate | recent | last release | revdeps | verdict |
| --- | --- | --- | --- | --- |
| `pdf-extract` | 3,634,439 | 2026-09-16 | 282 | text, not tables; the table opening stands |
| `pdfplumber` (Rust) | 42,602 | 2026-08-30 | 1 | still v0.4.1, self-described alpha |
| `stix` / `rstix` | 889 / 298 | 2021 / 2026-10-19 | 1 / 2 | **still open**, unchanged |
| `taxii` / `taxii-rs` | — | — | — | **FREE**, still unclaimed |
| `ofx-rs` / `ofxy` / `qif_parser` | 2,959 / 1,324 / 380 | 2026 | 1 / 0 / 0 | **still open**, unchanged |
| `subtp` / `subparse` / `subtitler` | 21,660 / 4,887 / 1,978 | 2024 / 2021 / 2026 | 3 / 9 / 0 | `subtp` is now at the threshold; `subparse` still 9 dependents on a 2021 release |
| `dom_smoothie` / `readability` | 532,798 / 394,156 | 2026-09 / 2023-12 | 43 / 34 | the readability incumbents, live |
| `metar` / `metar-taf-parser` | 600 / 5,733 | 2026-01 / 2026-03 | 0 / 0 | still thin; round 17's small opening |
| `uom` | 2,653,164 | 2026-02-14 | 215 | compile-time only; round 17's find stands |
| `runtime_units` | 1,785 | 2026-05-25 | 0 | the nearest runtime parser, still tiny |
| `runits` / `unitkit` | — | — | — | **FREE**, still unclaimed |
| `im` | 4,744,361 | 2022-04-29 | 238, **archived** | `rpds` 4.79M and `ordermap` 7.22M both live |

Nothing changed verdict since round 17. The four open candidates (PDF tables,
OFX/QIF, subtitles, STIX/TAXII), the readability watch, the METAR/TAF opening and
the unit-conversion find are all unchanged, so they are not re-argued here.

## The seven new domains

### Bibliography: BibTeX, RIS and CSL are served

| | |
| --- | --- |
| Demand | PyPI `pybtex` 1,178,196/month, `bibtexparser` 1,484,337/month, `pandocfilters` 48,249,973/month, `citeproc-py` 162,370/month. npm `citeproc` 82,976/week. |
| Rust side | `biblatex` 1,989,331 recent (26 revdeps), `hayagriva` 1,769,639 (21), `citationberg` 1,742,369 (2), `biblib` 178,311 (3), `pandoc` 133,301 (8). |

BibTeX parsing, CSL styles and citation rendering all have maintained Rust
crates, and the three leaders carry an audience in the millions. `biblatex` and
`citationberg` are from the same author and both released in June 2026, so this
is not a stale corner either. **Do not build a bibliography crate.**

### Office documents: DOCX served, ODT and PPTX both a race, and one thin writer

| | |
| --- | --- |
| Demand | PyPI `python-docx` 95,391,897/month, `openpyxl` 282,967,374/month, `python-pptx` 60,210,371/month, `odfpy` 5,197,259/month, `striprtf` 6,509,768/month, `docxtpl` 3,462,825/month. npm `docx` 4,798,794/week, `mammoth` 6,747,009/week, `pptxgenjs` 2,567,615/week, `officegen` 9,633/week. |
| DOCX | **Served.** `docx-rs` 781,776 recent (68 revdeps), v0.4.22, 2026-07-21. |
| DOCX templating | **A race.** `docx-handlebars` 2,388, `docxtplrs` 591, `easydoc-template` 431, `docxtpl-template` 194, `docx-template` 150, `docxtpl-rs` 142 — five of the six published or updated in the last six weeks. |
| ODT / ODF | **A race, and the sharpest one yet.** Three multi-crate toolkits and ten stand-alone crates: `lo_core`/`lo_odf`/`lo_zip`/`lo_math` (3,359-3,433 each, August 2026), `waddle`/`waddle-core`, `odox`/`odox-core`, `quarb-text-odt`, `xodt`, `xods` — **seven separate crates published on 2026-10-23 alone** — plus `spreadsheet-ods` 16,477, `libreoffice-pure` 2,591, `odf-crypto` 2,395, `ferrodoc-odt` 398, `unfair-codec-odf` 98. |
| PPTX | **A race.** `ppt-rs` 50,356 (2026-10-24), `pptx` 17,763 (2026-02-25), `zavora-slide` 275 + `-pdf` 299 + `-render` 320 + `slides-mcp-server` 171 (all 2026-08-25), `deckmint` 203 (2026-10-24), `pptx-rs2` 115, `md2star-rs` 72 (2026-10-19), `pptxcraft` 23 (2026-10-21), `ooxml-pml` 22, `pptx-rs` 16. |

DOCX reading and writing is served. Everything else in the domain is a race.
The PPTX case is round 16's method finding at its clearest since round 17:
twelve crates, four published in the last week and four more in August, none
with an audience (the leader has 2 reverse dependencies) and none archived. A
new entry would be the thirteenth team. **Do not build an ODT, ODF or PPTX crate.**

RTF is the one thin spot, and it is thin in one direction only:

| | |
| --- | --- |
| Readers | `rtf-parser` 73,958 recent (5 revdeps), `rtf-parser-tt` 2,819, `rtf-to-html` 614, `quarb-text-rtf` 443, `scrivener-rtf` 213, `rtf-grimoire` 3,070, `rclip-rtf` 8,934. |
| Writers | **None.** `rtf-writer`, `rtfgen` and `rtfkit` are all FREE. |
| Demand | The reader demand is real: PyPI `striprtf` 6,509,768/month. The **writer** demand is modest: npm `rtf.js` 60,655/week, `rtf-parser` 66,380/week, `html-to-rtf` 1,408/week. |

An RTF writer is genuinely absent, but the demand that exists is for reading RTF
(stripping it to text), not for writing it. **Recorded, not recommended**: the
gap is real and the audience is small, which is the opposite of the shape this
workspace looks for.

### EPUB: served

| | |
| --- | --- |
| Demand | PyPI `ebooklib` 903,541/month, `pypandoc` 4,140,318/month. npm `epub2` 96,091/week, `epubjs` 78,443/week, `epub-gen-memory` 76,765/week. |
| Rust side | `epub_parser` 223,405 recent, `epub` 100,816 (34 revdeps), `epub-builder` 32,377 (19), `rbook` 24,442 (7). |

Reading, parsing and building EPUB all have maintained crates. **Do not build an
EPUB crate.**

### Typesetting: LaTeX to MathML is served

| | |
| --- | --- |
| Demand | npm `katex` 18,896,310/week, `temml` 609,906/week, `mathjax` 117,977/week. PyPI `latex2mathml` 5,010,435/month, `pylatexenc` 6,742,002/month. |
| Rust side | `katex` 251,780 recent (24 revdeps), `ratex-parser` 120,873 (32 revdeps, 1,548 stars), `pulldown-latex` 58,476, `latex2mathml` 35,535 (21), `math-core` 23,676, `tex2math` 67, `latexml` 177. |

This is one of the highest-demand domains measured in any round, and the Rust
side matches it: a KaTeX port with 1,548 stars and a `pulldown-cmark` integration
that converts LaTeX in Markdown. **Do not build a typesetting crate.**

### Diff and patch: text served, structural diff of data the one thin spot

| | |
| --- | --- |
| Text diff | **Served, emphatically.** `similar` 63,324,567 recent (1,257 revdeps), `diff` 44,968,147 (174), `dissimilar` 13,284,268 (54), `diffy` 5,371,453 (139). PyPI `diff-match-patch` 4,833,829/month. npm `diff` 90,771,594/week, `fast-diff` 25,307,287/week. |
| RFC 6902 / 7396 | **Served.** `json-patch` 34,590,056 recent (164 revdeps), `jsonptr` 33,117,814 (28), `serde-patch` 11,071. PyPI `jsonpatch` 103,604,610/month. |
| Test assertions | **Served.** `assert-json-diff` 43,630,089 recent (178 revdeps). |
| **Structural diff of data** | **Thin, and this is the finding.** See below. |

Text diffing and JSON Patch are served many times over. The one shape that is
not is a **structural diff of nested data in the DeepDiff sense** — a
human-readable change report with order-independent array comparison, numeric and
string tolerance, type-change detection, and a reversible delta.

| | |
| --- | --- |
| Demand | PyPI `deepdiff` **53,044,205/month**, `jsondiff` 6,839,500/month, `dictdiffer` 5,672,801/month. npm `deep-object-diff` 2,292,847/week, `microdiff` 2,014,811/week, `jsondiffpatch` 1,608,613/week, `json-diff` 573,338/week. |
| Rust leaders | `json-structural-diff` 1,022,489 recent but **last released 2024-12-09**, 3 revdeps, 13 stars. `diff_json` 120,323 (2026-02-09, 1 star, 1 revdep). `json_diff` 63,230 (2020). `serde_json_diff` 46,435 (2023). `json_diff_ng` 25,917 (2024). `sjdiff` 210. `deep-diff` 7. |
| New entrants | `rigg-diff` 372 (2026-10-08), `datadiff` 159 (2026-10-19), `diffx` 90 (2026-03-18), `palim` 145 (2026-10-07). All tiny, all recent. |
| Names free | `deepdiff`, `dictdiffer`, `structural-diff`, `deepeq`, `json-delta` |

The leader is the tell. `json-structural-diff` is the most-downloaded crate in
the space at 1.02M recent, and its own README says it "has been developed with
the aim of testing parallelism". It is a research artifact, not a maintained
library: no release in twenty months, 13 stars, 3 reverse dependencies, and it
is MIT. Every other entry is either stale (2020-2024) or a few weeks old with no
audience.

This is the same shape as `pint` in round 17 and `stdnum-rs` before it: a
mature, heavily used reference package whose Rust side is either stale or tiny,
with the reference's defining feature — here the DeepDiff report and delta —
absent. The difference from the ODT and PPTX races is that the cluster here is
**not** active: four new entrants in October against six ODF crates in a single
day. The rule still applies, so the check is: does any incumbent have an
audience? None does, and the biggest is twenty months stale.

**This is the round's recommended candidate.**

### Segmentation: CJK and sentence splitting both served

| | |
| --- | --- |
| Demand | PyPI `nltk` 42,025,258/month, `sentencepiece` 26,882,258/month, `jieba` 4,787,084/month, `blingfire` 789,652/month. npm `sentence-splitter` 145,728/week, `sbd` 24,013/week. |
| Rust side | `unicode-segmentation` 158,146,814 recent (2,009 revdeps), `jieba-rs` 1,851,229 (60), `lindera` 1,279,552 (42), `vibrato` 25,226, `tinysegmenter` 5,594, `sentence` 29. |

CJK word segmentation is served by `jieba-rs` and `lindera`, both with real
audiences. Sentence splitting looks thin (`sentence` at 29 recent), but it is a
few hundred lines of abbreviation rules with no large data table, so it is not a
crate-sized gap. **Do not build a segmentation crate.**

### Emoji and IDNA: served

| | |
| --- | --- |
| Demand | npm `emoji-regex` 281,028,234/week, `node-emoji` 12,754,111/week, `punycode` 141,051,352/week, `grapheme-splitter` 4,120,015/week. PyPI `emoji` 11,851,022/month, `idna` 1,343,340,011/month, `grapheme` 1,274,954/month. |
| Rust side | `idna` 234,109,080 recent (249 revdeps), `unicode-width` 199,493,320 (3,252), `unicode-segmentation` 158,146,814 (2,009), `emojis` 1,664,030 (101), `icu` 721,024 (58). |

Every part of this domain has a maintained crate with a large audience, and the
`icu` crate brings the ICU4X data. **Do not build an emoji or IDNA crate.**

## Two domains the sweep found outside the seven

The seven chosen domains mostly came back served or racing, but probing around
them surfaced two more, and both are recorded as closed:

| | |
| --- | --- |
| **PPTX** | Twelve crates, four published in the last week, leader 2 revdeps. A race, as above. |
| **WOFF2 encoding** | **Closed since it was last considered.** The old `woff2` crate (887 recent, 2022) was decompression-only, which made a pure-Rust encoder look like a gap. It is not: `woofwoof` 87,919 recent (brotli-backed, 2026-01) and `ttf2woff2` 21,829 recent (2026-10-18) both compress, and `fontcull` 88,403 and `font-subset` 11,387 cover subsetting. PyPI `fonttools` 189,260,520/month, npm `subset-font` 325,306/week. **Do not build.** |
| NetFlow / IPFIX | `netflow_parser` 271,785 recent (1 revdep) but the source demand is tiny: PyPI `netflow` 9,025/month, `ipfix` 5,714/month. Not worth it. |
| IFC / BIM | `ifc-schema` 2,154 (28 revdeps), `ruststep` 77,131. PyPI `ifcopenshell` 777,906/month, npm `web-ifc` 174,704/week. Thin on the Rust side, but the domain needs the full IFC schema, which is a data-transcription project of a different size. Recorded, not recommended. |

## What this round concludes

1. **Six of the seven new domains are served or racing.** Bibliography (served
   four times over), office documents (DOCX served, ODT/ODF and PPTX both
   racing), EPUB (served), typesetting (served, with a 1,548-star KaTeX port),
   segmentation (served), emoji and IDNA (served). Do not re-scan them.
2. **The ODT/ODF race is the sharpest cluster measured in any round**: seven
   separate crates published on one day, plus three multi-crate toolkits. PPTX is
   the second, with twelve crates and four of them from the last week. Neither
   has an audience, and neither should be entered.
3. **Structural diff of nested data is the round's real find.** PyPI `deepdiff`
   at 53.0M/month has no Rust counterpart with an audience: the leader is a
   2024 research artifact whose README says it was built to test parallelism,
   and the rest are stale or weeks old.
4. **RTF writing is a genuine but small opening.** The readers exist, no writer
   does, and the demand that exists is for reading. Recorded, not prioritised.
5. **The WOFF2 encoder question is closed.** A pure-Rust WOFF2 compressor looked
   missing because the older `woff2` crate only decompressed; two crates now
   compress, one released a week ago.
6. **The survivors are unchanged.** PDF tables remains the strongest opening
   measured in any round; OFX/QIF, subtitles and STIX/TAXII are still open;
   readability is still a watch; unit conversion and METAR/TAF are round 17's.
7. **The method finding is confirmed for a fourth round.** Round 16 saw the
   actively-claimed cluster, round 17 saw it at its most extreme, and this round
   saw it twice in one domain. The rule stands: **before proposing a crate, sort
   the nearest crates by release date, not only by downloads** — and then ask
   whether any of them has an audience, because a cluster of unadopted crates is
   a race, not a gap.

## Method note

The script is `tools/round18.py`:

```sh
python3 round18.py candidates       # re-measure the open survivors
python3 round18.py sweep            # the seven new domains
python3 round18.py names <name>...  # is a proposed crate name free?
```

Round 17's corrections are carried forward and re-confirmed: the sparse index
field is `pubtime`, not `pubdate`, and the index path is `2/<name>` for two
characters, `1/<name>` and `3/<c>/<name>` otherwise. The rate-limit warning in
`tools/README.md` applies in full: crates.io returns HTTP 429 under load and a
naive fetch reads that as "no such crate". Every `FREE` here was re-checked.

Round 15's rule also applies: **a keyword search on the upstream package's name
is not a gap check.** The structural-diff opening was found by searching
crates.io for the *capability* (`deepdiff`, `deep compare`, `semantic json
diff`, `ignore order`) rather than for `deepdiff` alone, because the Rust
crates in the space use four different names for the same idea
(`json-structural-diff`, `diff_json`, `serde_json_diff`, `json_diff_ng`). The
ODT race was found the same way: searching `odf`, `opendocument` and
`openoffice document` rather than `odfpy` surfaced eleven crates that a name
search for `odfpy` would have reported as an empty registry.
