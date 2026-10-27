# Gap scan round 26: one real opening, and it is legal citation (2026-10-21)

Round 25 closed by asking for its own rule to be applied *before* a candidate is
recorded rather than as a check afterwards: search the capability, not the
upstream package's name, and sort the nearest crates by release date. This round
applies that rule from the first line. Eight further domains were swept, and for
each one the name-based pass was followed by a `?q=` capability search before
any verdict was written down.

The result is the first domain since round 20 that survives both passes: **legal
citation extraction**. Seven of the eight are served or racing, one is served
under a different name, and one is genuinely empty but has no measurable demand.

## The eight domains

| Domain | Demand | Rust today | Verdict |
| --- | --- | --- | --- |
| Legal citation (Bluebook, ECLI, neutral citation, reporter lookup) | PyPI `eyecite` 762,876/mo, `reporters-db` 776,821/mo, `citeurl` 238,645/mo; npm `citation-js` 11,577/wk, `citeproc` 88,479/wk | **nothing extracts citations**; `reporters-db` 47 (an unofficial data port, 2025-09), `peira-citation` 132 (hash-verified packets, not extraction), `citation` 9 (2023), `ecli` 11 ARCHIVED (2019), `citationberg` 1.77M (**CSL bibliographies, a different job**) | **Open - the finding** |
| Pharmacy and medication dosing (NDC, RxNorm, dose checks) | PyPI `fhir.resources` 2,516,366/mo; npm `fhir` 46,228/wk, `@types/fhir` 252,439/wk | `fhir-types` 14,941 and a 15-crate HL7 vocabulary suite (`rxnorm-rrf` 379, `icd11` 390, `concept-graph` 598, all v0.1.120 pushed 2026-10-22), `fhir` 895, `snomed` 1,047, `formulary` 3,263, `autogen-dosespot` 11,162 | **Served, and claimed this week** |
| Museum and library collection interchange (LIDO, EAD, CDWA, OAI-PMH) | PyPI `pymarc` 49,990/mo, `sickle` 22,524/mo, `edtf` 33,241/mo, `pyoai` 1,169/mo; npm `marcjs` 2,120/wk, `edtf` 6,570/wk | `marc-rs` 3,712 and `marctk` 1,084 and `marc` 2,099 (three MARC 21 readers), `z3950-rs` 3,482, `oai-pmh-rs` 990, `library_stdnums` 1,023, `edtf-core` 3,148 + `citum-edtf` 958 + `edtf` 6,153 | **Served four times over** |
| Civil aviation flight planning (ICAO flight plan, NOTAM, METAR/TAF) | PyPI `metar` 66,935/mo; npm `metar-taf-parser` 5,846/wk | `metar-taf-parser` 6,004 (2026-03), `metar` 589, `aerocontext-*` suite (seven crates, `aerocontext-core` 2,820, all v0.4.5 pushed 2026-07-03), `aviation-wx-*` suite | **Served, twice** |
| Marine safety message encoding (AIS/VDES, NMEA 0183) | PyPI `pyais` 152,581/mo, `pynmea2` 112,298/mo; npm `signalk-server` 1,961/wk | `nmea-parser` 124,512, `nmea` 43,581, `nmea-kit` 1,050 (2026-09), `ais` 3,080, `aivdm` 205 + `aivdm-cli` + `aivdm-ffi`, `marlin-ais` 151, `nmea-0183-rs` 126, `canboat` 615 | **Served, decoding fragmented** |
| Chess and game notation (PGN, FEN, UCI, SAN) | PyPI `chess` 2,615,872/mo, `python-chess` 1,696,847/mo; npm `chess.js` 188,984/wk | `shakmaty` 85,241 (2026-10, 44 revdeps), `cozy-chess` 36,729, `chess` 21,017, `uci` 5,359, `pgn` 11, `fen` 317 | **Served** |
| Geotechnical and civil engineering units (soil, concrete, survey) | PyPI `pyproj` 21,187,829/mo, `geopy` 10,715,841/mo; npm `proj4` 940,278/wk, `mgrs` 905,886/wk | `geo-types`, `proj`, `crs-definitions`, `geodesy` and the wider geo stack serve the projection and coordinate half; `soil` 2 (2026-03) and `geotech` FREE cover the soil half, which has **no demand figure at all** (`soil-mechanics` is not on PyPI) | **Half served, half demanded by nobody** |
| Indonesian and Malaysian administrative identifiers (NIK, NPWP, EMIS, KDP) | PyPI `python-stdnum` 5,079,475/mo; npm `npwp-validator` 6/wk, `nik-validator` 0/wk | `stdnum-rs` 44 (this workspace) is the **only** crate in the registry that knows `npwp`; `nik`, `emis`, `dapodik`, `pesantren`, `madrasah`, `raport`, `kurikulum-merdeka`, `mykad`, `nric` are all FREE | **Empty, and that is the point** |

## The one opening: legal citation extraction

The demand is real and Python-side. `eyecite` (Free Law Project) is the
reference implementation of "find the legal citations in this text", and
`reporters-db` is the reporter table it reads; together they are **1.54M
downloads a month**, and `citeurl` adds 238,645. The npm side is `citeproc` and
`citation-js` at ~100K a week, which is *bibliography formatting* rather than
citation extraction - a different job that is easy to mistake for this one.

The Rust side does not have it. The capability search (`citation extraction`,
`legal citation parser`, `court opinion`, `law citation`, `bluebook`) returns
nothing that extracts a citation from free text, and the two obvious names are
both free (`round26.py names legal-cite citation-parser`). What exists is
adjacent:

* `reporters-db` (47 recent, 2025-09) is an unofficial port of the **data
  table**, not the extractor.
* `peira-citation` (132, 2026-09) is described by its own author as "frozen,
  hash-verified citation packets" - an integrity format, not a parser.
* `ecli` (11, ARCHIVED, 2019) was a single-identifier parser and is dead.
* `citationberg` (1.77M) parses **CSL** files, which is how a bibliography is
  *styled*; it does not read a court opinion.

So the shape is the one this workspace has built before (`stdnum-rs`,
`holidays-rs`, `aviation-data`): a public reference implementation, a public
dataset, a permissive licence, and an empty Rust registry. It is a **large**
job, not a `0.1.x` extension - the reference is a tokenizer, a reporters table,
a court-abbreviation table, a set of reporters-to-courts mappings and a
normalisation layer - which is exactly why it is still open.

**Licence caution before building.** `eyecite` is permissive, but the Free Law
Project ships some data under mixed terms and `reporters-db` carries its own
licence. This workspace is Apache-2.0-only, so the reporters table would have to
be either regenerated from the public domain sources the reference itself reads
or re-licensed with explicit permission. That check comes before any code.

## The one domain that is empty without being an opening

Indonesian and Malaysian administrative identifiers is the fourth consecutive
round to land on this region. The registry is empty: nothing knows `npwp`,
`emis`, `dapodik`, `pesantren`, `madrasah`, `raport`, `kurikulum-merdeka`,
`mykad` or `nric`, and this workspace's own `stdnum-rs` is the only crate that
answers to `npwp`. But the npm and PyPI demand figures are **zero or
single-digit per week**. There is no measurable audience, so it is not an
opening in the sense this scan measures - it is an opening in the sense that the
workspace already holds the domain vocabulary (`cipansor/packages/shared`,
12,913 LOC) and can serve it without a competitor.

That distinction matters and this round states it plainly rather than counting
the empty registry as a win.

## The two corrections this round records

**1. `citationberg` is not a legal-citation crate.** It is the second-largest
hit for the query and it is CSL bibliographies. A download-sorted capability
search puts it first and a careless read calls the domain served. It is not.

**2. "Three MARC readers" is the same artefact round 25 found with COBOL.**
`marc`, `marc-rs` and `marctk` are three independent crates for one format, and
`z3950-rs` and `marcr-rules` are from the same authors as `marc-rs`. A single
name lookup sees `museum`, `lido`, `dublin-core` and `collection` and reports
four free names while the capability is served by a cluster.

## What this round concludes

1. **Legal citation extraction is the opening.** It is the first new one since
   round 20 (`aviation-data`), and it passes the capability search that round 25
   made mandatory.
2. **It is a crate, not a feature.** The reference is a tokenizer plus two data
   tables plus a normaliser. Proposing it as a `0.1.x` extension would be
   dishonest about the size.
3. **The licence check precedes the code.** Apache-2.0-only means the reporter
   table must be regenerated or explicitly permitted.
4. **Seven of eight domains are served, and four of them were served by a
   suite rather than a crate** - the round 25 artefact, now confirmed a second
   time.
5. **The region-specific ground is still real but still unmeasured.** The
   Indonesian identifiers are free and unclaimed; they have no download demand.
   Build them because the workspace already owns the domain, not because the
   scan found an audience.

## Method

```sh
python3 research/tools/round26.py sweep          # the eight new domains
python3 research/tools/round26.py names <n>...   # is a proposed crate name free?
```

`round26.py` imports the measured helpers from `round15`. Every figure was
fetched live from `crates.io/api/v1`, `index.crates.io`, `api.npmjs.org`,
`pypistats.org` and `api.github.com` on 2026-10-21. The capability searches that
overturned the name-based passes used `crates.io/api/v1/crates?q=...&sort=recent-downloads`,
and every "Served" verdict was confirmed against the crate's own API entry
before it was written down.
