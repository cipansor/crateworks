# Gap scan round 20: aviation data, quant finance, CAD and five more (2026-10-26)

Rounds 8-19 each re-measured every open candidate and then swept domains no
earlier round had touched. This round does the same. The eight new domains are
ones the npm and PyPI popularity lists under-represent, but which a real
application meets constantly: **aviation reference data**, **technical
indicators**, **payment-card data**, **clinical terminology**, **JSON query
languages**, **CAD interchange**, **astronomy data** and **video containers**.

Every figure was fetched live on 2026-10-26 from `crates.io/api/v1`,
`index.crates.io` (the sparse index, for a real release date), `api.npmjs.org`,
`pypistats.org` and `api.github.com` by `research/tools/round20.py`.

The rule rounds 15-19 established is applied throughout: **before proposing a
crate, sort the nearest crates by release date, not only by downloads, and ask
whether the incumbent has an audience.** A cluster of unadopted crates is a
race, not a gap. Round 15's second rule also applies: **a keyword search on the
upstream package's name is not a gap check** - search the *capability*. Every
`FREE` below was re-checked, and every "served" verdict was reached by searching
for the capability, not the name.

## Verdict summary

| Domain | Upstream volume | Best Rust incumbent | Verdict |
| --- | --- | --- | --- |
| Aviation reference data | PyPI `airportsdata` 1.59M/mo | `airports` 875,912 (2022, **timezone only**) | **strongest opening** |
| Technical indicators | PyPI `pandas-ta` 441K/mo, npm `technicalindicators` 41.9K/wk | `ta` 74,367 (2021, 872 stars) | **open**, active repo but 2021 crate |
| Payment cards | npm `credit-card-type` 1.50M/wk, `card-validator` 1.20M/wk | `card-validate` 456,948 (2024) | served |
| Clinical terminology | PyPI `fhir.resources` 2.52M/mo | `snomed` 1,037, `fhir` 896 (both 2026) | race, above nothing but tiny |
| JSON query languages | PyPI `jmespath` 501M/mo, npm `jsonpath-plus` 11.1M/wk | `jsonpath-rust` 22.9M, `jmespath` 766K | served |
| CAD interchange | PyPI `ezdxf` 4.89M/mo | `dxf` 127,242 (2026, 142 stars) | served for DXF, **open for STEP/IGES** |
| Astronomy data | PyPI `astropy` 2.67M/mo, `pyswisseph` 1.01M/mo | `fitsio` 18,067, `sofars` 59,342 | served |
| Video containers | PyPI `ffmpeg-python` 7.82M/mo, `mutagen` 6.95M/mo | `mp4` 3.40M, `matroska` 73,638 | served |

## The survivors, re-measured

Nothing changed verdict since round 19. The long-open candidates and the
readability watch are unchanged, so they are not re-argued here. Their live
numbers:

| crate | recent | last release | revdeps |
| --- | --- | --- | --- |
| `ofx-rs` / `ofxy` / `qif_parser` | 2,977 / 1,325 / 386 | 2026-03 / 2025-09 / 2026-09 | 1 / 0 / 0 |
| `subtp` / `subparse` / `subtitler` | 22,032 / 4,932 / 1,980 | 2024-02 / 2021-04 / 2026-10 | 3 / 9 / 0 |
| `pdf-extract` / `pdfplumber` | 3,675,601 / 43,103 | 2026-09 / 2026-08 | 284 / 1 |
| `stix` / `rstix` | 892 / 302 | 2021-04 / 2026-10 | 1 / 2 |
| `dom_smoothie` / `readability` | 541,242 / 395,667 | 2026-09 / 2023-12 | 43 / 34 |
| `metar` / `metar-taf-parser` | 602 / 5,896 | 2026-01 / 2026-03 | 0 / 0 |
| `postal` / `address` / `us_address_parser` | 6,456 / 622 / 545 | 2023-09 / 2026-09 / 2025-03 | 1 / 2 / 0 |
| `epcgen` / `qrplatba` / `sepa` | 14 / 40 / 1,548 | 2025-11 / 2026-10 / 2026-10 | 0 / 0 / 1 |

`taxii` and `taxii-rs` are still `FREE`. `pdf-extract` has crossed three and a
half million recent downloads, so PDF text extraction is closed for good; the
table-extraction question is separate and still the round 15 finding.

## New domain 1: aviation reference data

This is the round's strongest opening.

| | |
| --- | --- |
| Upstream | PyPI `airportsdata` **1,593,710/month**; PyPI `openap` 8,411/month; PyPI `airport-codes` 1,029/month; npm `airport-codes` 1,937/week |
| Rust incumbents | `airports` 875,912 recent (2022-06, **maps IATA code to timezone only**); `airport-data` 642 (2026-10, 1 star, **CC-BY-4.0**, 0 revdeps); `iata` 381 (2025-09, IATA *resolutions*, a different thing); `icao-isa` 741 and `icao-units` 1,084 (physics, not the code registry); `metar-taf-parser` 5,896 (weather, not the registry) |
| Free names | `aviation-data`, `airport-codes`, `airports-rs`, `iata-codes`, `icao-codes`, `openap`, `aviation-codes` |

The one crate with volume, `airports` at **875,912 recent downloads**, does not
do the job its name suggests. Its own README says it "maps airport IATA code to
timezone" and nothing else - `get_tz_name("LHR") -> "Europe/London"`. It carries
no airport name, no coordinates, no country, no ICAO code and no city, and it
was last published in 2022 at v0.0.2. The whole field of callers that want to
turn `"LHR"` into a record, or `"London Heathrow"` into a code, is writing the
lookup itself.

The nearest complete crate, `airport-data` (2026-10), has **642 recent
downloads, one star and zero reverse dependencies** - an unadopted crate, and it
is **CC-BY-4.0**, which is a data licence a permissive-licensed library cannot
absorb. That is the second opening: the permissively-licensed dataset exists
([OurAirports](https://ourairports.com/data/), public domain, **86,286 airports**
with `ident`, `type`, `name`, latitude, longitude, elevation, continent,
`iso_country`, `iso_region`, `municipality`, `scheduled_service`, `icao_code`,
`iata_code`, `gps_code`, `local_code`) and no crate ships it under
Apache-2.0/MIT.

The opening is a pure-Rust aviation reference crate: the OurAirports table
compiled in, lookups by IATA, ICAO, GPS and local code, search by name and
municipality, and distance between airports, under Apache-2.0. It is data
mapping, like `holidays-rs`, not algorithm research, so the risk is low and the
work is measurable.

**Why it fits this workspace.** It is the same shape as `holidays-rs`: an open
dataset, a permissive licence, a small API, and a verifier that regenerates the
table from the upstream CSV and diffs it. It also pairs with `adhan`, which
already needs coordinates for a city.

## New domain 2: technical indicators

| | |
| --- | --- |
| Upstream | PyPI `pandas-ta` 440,963/month; PyPI `talipp` 18,455/month; PyPI `finta` 9,130/month; npm `technicalindicators` 41,895/week; npm `indicatorts` 3,150/week |
| Rust incumbents | `ta` 74,367 recent (2021-06, 872 stars); `rust_ti` 1,512 (2025-10, 50 stars); `talib` 87 (2024-12, 257 stars); `talipp` 0 (2024-03); `technical_indicators` 51 (2017); `indicators` 12 (2021) |
| Free names | `ta-rs`, `fin-ta`, `talib-rs`, `technical-indicators` |

`ta` is the incumbent: 74,367 recent downloads and a healthy repo (872 stars,
pushed 2024-07), but its last **release** is v0.5.0 from **2021-06**. It is a
mature crate that has stopped shipping. A GitHub capability search turns up
`yata` (399 stars), `sliding_features-rs` (80), `wickra` (69) and
`CentaurTechnicalIndicators-Rust` (50) - a cluster of small, unadopted crates,
which by the round 15 rule is a race rather than a gap. `ta` still owns the
audience, so a new entrant has to be materially better, not merely newer.

Verdict: **open but contested**. Worth a crate only with a clear differentiator
(streaming/incremental indicators, or the exact `pandas-ta` set with a
differential oracle). Not this workspace's lowest-risk pick.

## New domain 3: payment-card data

| | |
| --- | --- |
| Upstream | npm `credit-card-type` 1,497,240/week; npm `card-validator` 1,198,195/week; npm `payment` 116,421/week; npm `luhn` 76,682/week; PyPI `stripe` 37.2M/month |
| Rust incumbents | `card-validate` 456,948 recent (2024-03, 12 revdeps); `luhn` 105,403 (2026-10); `creditcard` 1,366 (2025-01); `credit-card` 123 |
| Free names | `cardinity`, `pan-validator`, `bin-lookup`, `card-bin` |

`card-validate` has **456,948 recent downloads and 12 reverse dependencies**, so
this is **served**. The demand is real and large, but the incumbent has an
audience and the job (Luhn, length and IIN prefix per brand) is small enough
that a second crate would be a duplicate. A GitHub search for a BIN/IIN
database crate returns nothing, so a *BIN-range* crate is free, but a BIN
database has the same licensing problem as `airport-data`: the good ranges come
from paid feeds.

Verdict: **served**. Do not enter.

## New domain 4: clinical terminology

| | |
| --- | --- |
| Upstream | PyPI `fhir.resources` 2,516,366/month; PyPI `icd10-cm` 12,170/month; PyPI `pymedtermino` 643/month; npm `fhir` 46,228/week; npm `fhirclient` 40,806/week |
| Rust incumbents | `snomed` 1,037 (2026-10, RF2 parsing, 3 stars); `fhir` 896 (2026-10, complete R2-R6 model, 6 stars); `loinc` 244 (2026-10, **BUSL-1.1**); `icd`, `icd10`, `snomedct`, `hl7-fhir` all `FREE` |
| Free names | `clinical-codes`, `icd10-codes`, `loinc-rs`, `snomed-rs` |

The FHIR and SNOMED tooling arrived **this month**: `fhir` v4.2.4 and `snomed`
v0.56.1 both published 2026-10-16, and `loinc` on 2026-10-22. They are brand new
and unadopted (896, 1,037 and 244 recent downloads), so by the round 15 rule
they are a **race**. The `icd` and `icd10` names are still `FREE`, but the ICD
data itself is the problem: WHO ICD-10 and ICD-11 are not permissively licensed
for redistribution, and the US ICD-10-CM files are public-domain US government
work only. `loinc`'s author resolved that by shipping under **BUSL-1.1**, which
is not an open licence.

Verdict: **race, and licence-encumbered**. Do not enter without a clear data
rights position.

## New domain 5: JSON query languages

| | |
| --- | --- |
| Upstream | PyPI `jmespath` 501,265,337/month; PyPI `jsonpath-ng` 55,085,161/month; npm `jsonpath-plus` 11,101,557/week; npm `jmespath` 9,339,611/week; npm `jsonata` 1,473,789/week |
| Rust incumbents | `jsonpath-rust` 22,962,321 recent (2026-10, 79 revdeps); `jmespath` 765,844 (2026-01, 36 revdeps); `jsonpath_lib` 783,736 (2021); `jsonpath` 165,633 (2018); `jq-rs` 5,721 (2019); `jaq` 5,678 (2026-10, 3,811 stars) |
| Free names | `jsonata-rs` (the `jsonata` name is taken by a v0.0.0 stub) |

Every language here has a mature Rust implementation with an audience:
`jsonpath-rust` at 22.9M recent downloads and 79 reverse dependencies, `jmespath`
at 766K with 36, and `jaq` at 3,811 stars. The one thin spot is **JSONata** -
the Rust `jsonata` crate is v0.0.0 from 2022 with 7 recent downloads, while npm
`jsonata` does 1.47M/week - but JSONata is a large language (its own
transformation semantics) and the PyPI `jsonata` is only 5,215/month, so the
demand is npm-only.

Verdict: **served**, with one thin corner (JSONata) that is a large build.

## New domain 6: CAD interchange

| | |
| --- | --- |
| Upstream | PyPI `ezdxf` 4,893,719/month; PyPI `steputils` 50,335/month; npm `dxf-parser` 111,533/week; npm `occt-import-js` 83,905/week |
| Rust incumbents | `dxf` 127,242 recent (2026-10, 142 stars, 11 revdeps); `truck` 43 (2020, **dead**); `opencascade` 1,428 (2026-08, FFI, 271 stars); `step` 17; `dxf-rs`, `iges`, `stepfile` all `FREE` |
| Free names | `dxf-rs`, `iges`, `stepfile`, `step-file`, `dxf-writer` |

DXF is **served** by `dxf` (127,242 recent, 2026, 142 stars). STEP and IGES are
the opening: `step` is a v-less stub with 17 downloads, `truck` has been dead
since 2020, and the only real STEP/IGES work in Rust is `opencascade`, an FFI
wrapper around the 500 MB OpenCASCADE C++ library that needs a system install.
A pure-Rust STEP (ISO 10303-21, a text format) reader is a real gap.

The catch is size: a STEP reader that is useful needs the schema and the
geometry kernel, which is why `truck` stalled. A reader for the **file structure
and the product/assembly data** (without evaluating geometry) is feasible; a
full geometry kernel is not a single-crate job.

Verdict: **open for STEP/IGES, but scope it to the file structure**, not
geometry.

## New domain 7: astronomy data

| | |
| --- | --- |
| Upstream | PyPI `astropy` 2,665,681/month; PyPI `pyswisseph` 1,006,519/month; PyPI `skyfield` 506,984/month; npm `suncalc` 218,611/week; npm `satellite.js` 58,374/week; npm `astronomy-engine` 50,021/week |
| Rust incumbents | `sunrise` 200,109 recent (2026-01, 15 revdeps); `sofars` 59,342 (2026-10, ERFA/SOFA port); `astro` 49,014 (2016, **dead**, 314 stars); `fitsio` 18,067 (2026-04, FFI to cfitsio); `astronomy` 104; `astro-rust` 17; `ephemeris` 9; `fits`, `rust-ephem` `FREE` |
| Free names | `astronomy-engine`, `ephemeris-rs`, `rust-ephem`, `astro-rs` |

The astronomy stack is **served**: `sunrise` (200K) for solar position, `sofars`
(59K) for the ERFA/SOFA reference routines, `fitsio` (18K) for FITS. The pure-Rust
FITS gap was filled by `fitsrs` (35 stars, 2026-09). `astro` is dead (2016) but
its audience moved to `sunrise` and `sofars`.

Verdict: **served**.

## New domain 8: video containers

| | |
| --- | --- |
| Upstream | PyPI `ffmpeg-python` 7,822,402/month; PyPI `mutagen` 6,946,517/month; PyPI `pymediainfo` 2,128,669/month; npm `mp4box` 296,525/week; npm `mediainfo.js` 55,630/week |
| Rust incumbents | `mp4` 3,399,064 recent (2023-08, 45 revdeps); `mp4parse` 1,450,159 (2023-05, **Mozilla**, 448 stars); `ebml-iterable` 233,760 (2026-08); `matroska` 73,638 (2026-04); `mp4box` 4,604 (2026-09); `mkv` 64 (2021); `isobmff` `FREE` |
| Free names | `isobmff`, `mp4box-rs`, `mov-rs` |

**Served, and by Mozilla.** `mp4parse` (1.45M recent, 448 stars) and `mp4` (3.4M
recent, 45 revdeps) cover the ISOBMFF/MP4 family, and `matroska` plus
`ebml-iterable` cover Matroska/WebM. `isobmff` as a name is free but the
capability is not.

Verdict: **served**.

## What this round proposes

One crate, and it is the same shape as the crate this workspace already ships:

**`aviation-data`** - the OurAirports table (86,286 airports, public domain)
compiled into a Rust crate under Apache-2.0, with lookups by IATA, ICAO, GPS and
local code, search by name and municipality, and great-circle distance. The
`airports` crate holds 875,912 recent downloads doing a strict subset of this
(timezone only), the complete incumbent is unadopted at 642 and cannot be used
by a permissive crate because it is CC-BY-4.0, and no Rust crate ships the
permissively-licensed dataset at all. Free names: `aviation-data`,
`airport-codes`, `airports-rs`, `iata-codes`, `icao-codes`.

The verification story is the one that already works here: a generator reads the
upstream CSV, emits the table and the fixtures, and a gate re-runs the generator
and diffs the output, exactly as `gen_hijri_table.py` and
`gen_stdnum_vectors.py` do.

The other seven domains are recorded and rejected, with the reason, so a later
round does not re-open them by mistake.

## Method

```sh
python3 research/tools/round20.py sweep          # the eight new domains
python3 research/tools/round20.py names foo bar  # is a proposed name free?
```
