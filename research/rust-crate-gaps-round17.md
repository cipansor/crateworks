# Gap scan round 17: the survivors re-measured, and seven scientific and everyday-data domains (2026-10-26)

Round 16 closed with STIX/TAXII as its one new candidate and the
"actively-claimed cluster" as its method finding. This round does the same two
jobs rounds 13 through 16 did: measure every survivor again, then sweep domains
no earlier round touched.

Rounds 8-15 searched the npm and PyPI popularity lists and then utility domains;
round 16 swept specialist protocols (healthcare, energy, field buses, threat
intel, supply chain, market data, identity). This round picks seven **scientific
and everyday-data** domains that those lists under-represent but that whole
industries depend on: cheminformatics, bioinformatics, geospatial, audio and
music, postal addresses and phone numbers, recurrence, and meteorology. They are
chosen for fit with this workspace, whose strength is transcribing a real
specification and checking the result against a reference package.

Every figure was fetched on 2026-10-26 from `crates.io/api/v1`,
`index.crates.io` (the sparse index, for a real release date), `api.npmjs.org`,
`pypistats.org` and `api.github.com`. A gap is real when the Rust incumbent has
under about 20,000 recent downloads and near-zero reverse dependencies, while
the source counterpart has real volume. Recent downloads are the rolling
ninety-day figure.

## The surviving candidates, re-measured

| crate | recent | last release | revdeps | verdict |
| --- | --- | --- | --- | --- |
| `pdf-extract` | 3,658,772 | 2026-09-16 | 282 | text, not tables; the table opening stands |
| `pdfplumber` (Rust) | 42,562 | 2026-08-30 | 1 | still v0.4.1, self-described alpha |
| `stix` / `rstix` | 890 / 297 | 2021 / 2026-10-19 | 1 / 2 | **still open**, unchanged |
| `taxii-rs` | — | — | — | **FREE**, still unclaimed |
| `ofx-rs` / `ofxy` / `qif_parser` | 2,968 / 1,324 / 377 | 2026 | 1 / 0 / 0 | **still open**, unchanged |
| `subtp` / `subparse` / `subtitler` | 21,549 / 4,928 / 1,987 | 2024 / 2021 / 2026 | 3 / 9 / 0 | **still open**, unchanged |
| `dom_smoothie` / `readability` | 536,848 / 399,739 | 2026-09 / 2023-12 | 43 / 34 | the readability incumbents, live |
| `fuzzy-matcher` | 9,642,212 | 2020-10-04 | 470, **archived** | `fuzzy-score` shipped here; migration in progress |
| `im` | 4,804,708 | 2022-04-29 | 238, **archived** | `rpds` 4.85M and `ordermap` 7.25M both live |

Nothing changed verdict since round 16. The four open candidates (PDF tables,
OFX/QIF, subtitles, STIX/TAXII) and the readability watch are unchanged, so they
are not re-argued here.

## The seven new domains

### Cheminformatics: a race, and the clearest one yet

| | |
| --- | --- |
| Demand | PyPI `rdkit` 4,679,992/month, `pubchempy` 1,525,820/month, `openbabel` 28,623/month. npm `smiles-drawer` 54,494/week, `openchemlib` 23,572/week. |
| Rust side | `chemcore` 483 recent (2021), `gamma` 715 (2021), `purr` 14,166 (2021), `rdkit` 625, `chemfiles` 9,647. All from the same abandoned 2021 family. |
| The race | A single author published `chematic` plus 14 sibling crates (`chematic-smiles`, `chematic-inchi`, `chematic-fp`, `chematic-3d`, …) on **2026-10-23**, and a second team published `cosmolkit` plus siblings on **2026-10-14**. Two complete toolkits, one week apart, each under 25K recent. |
| Names free | `chemkit`, `smiles` |

Cheminformatics is the textbook case of round 16's method finding, and it is
sharper here than in any domain that round measured: not a cluster of one
capability but **two full toolkits** claiming SMILES, SMARTS, InChI, fingerprints,
2D depiction and IUPAC naming at the same time. A new entry would be the third
team in a two-week-old race. **Do not build a cheminformatics crate.**

### Bioinformatics: served, many times over

| | |
| --- | --- |
| Demand | PyPI `biopython` 4,377,509/month, `pysam` 1,218,720/month, `pyfaidx` 368,613/month, `cyvcf2` 136,827/month. |
| Rust side | `bio` 118,102 recent (164 revdeps), `noodles` 116,623 (141 revdeps), `needletail` 85,304 (120 revdeps), `rust-htslib` 96,989 (110 revdeps), `bio-types` 185,008. |

FASTA/FASTQ, SAM/BAM, VCF and sequence alignment all have maintained Rust crates
with three-digit reverse-dependency counts. **Do not build a bioinformatics
crate.**

### Geospatial: served, and the projection question is answered

| | |
| --- | --- |
| Demand | PyPI `shapely` 59,541,121/month, `pyproj` 21,187,829/month, `geopandas` 13,727,629/month, `rasterio` 5,070,539/month. npm `proj4` 1,015,518/week, `geotiff` 800,868/week. |
| Rust side | `geo` 4,950,203 recent (417 revdeps), `geo-types` 6,689,459 (365), `geojson` 2,124,552 (220), `proj` 204,052, `proj4rs` 393,782, `shapefile` 166,577, `gdal` 219,471, `h3o` 534,450. |

Every sub-domain — geometry, projections, shapefiles, GeoTIFF, GDAL, H3 — has a
maintained crate with a real audience. `geotiff` is young (7,593 recent, 2025)
but it sits behind the served `tiff` (43M recent) and the served `rasterio`
story, so it is not an opening. **Do not build a geospatial crate.**

### Audio and music: tags served, notation a race

| | |
| --- | --- |
| Demand | PyPI `mutagen` 6,946,517/month, `mido` 1,642,568/month, `music21` 522,439/month. npm `music-metadata` 1,205,196/week, `midi-file` 58,706/week. |
| Tags | **Served.** `id3` 1,731,809 recent, `lofty` 546,975 (139 revdeps), `symphonia` 9,795,859. Do not build an audio-tag crate. |
| MIDI | **Served.** `midly` 188,163 recent (98 revdeps), `wmidi` 31,002. |
| MusicXML / notation | **A race.** `musicxml` 1,923 recent (2024), but `acorde` published an umbrella plus `acorde-io`, `acorde-core`, `acorde-render-svg`, `acorde-layout` on **2026-10-25**, and `croma-core`, `cantara-songlib`, `music21-rs`, `abc-parser` and `chordsketch` all appeared or updated within October. |

Audio tags and MIDI are served. Music notation is being claimed by a
multi-crate toolkit published the day before this scan, so it is a race rather
than an opening. **Do not build a music-notation crate.**

### Postal addresses and phone numbers: phone served, addresses thin

| | |
| --- | --- |
| Demand | PyPI `phonenumbers` 34,298,405/month, `usaddress` 5,029,779/month, `postal` 72,056/month. npm `libphonenumber-js` 18,004,556/week, `parse-address` 67,958/week, `addresser` 40,253/week. |
| Phone | **Served.** `phonenumber` 3,273,270 recent (67 revdeps). |
| Address parsing | Thin. `postal` 6,519 recent (libpostal FFI, 2023), `addr` 1,645,293 (2022, but it is a DNS/network address type, not a postal one), `address` 612, `address-formatter` 295 (2022). |

Phone-number parsing is served by a `libphonenumber` port. Postal-address
parsing has no pure-Rust incumbent with an audience; every crate is either an
FFI binding to `libpostal` (a C library and its 1.5 GB data files) or abandoned.
The demand is real but the domain needs the `libpostal` statistical model, not a
transcription, so it does not fit this workspace's method. **Not recommended**;
it is recorded as a genuine opening for someone willing to ship the data.

### Recurrence: cron and RRULE both served

| | |
| --- | --- |
| Demand | PyPI `croniter` 75,361,738/month, `icalendar` 7,563,624/month, `recurring-ical-events` 2,254,275/month. npm `cron-parser` 14,673,078/week, `node-cron` 3,266,647/week, `rrule` 1,942,405/week, `ical.js` 561,836/week. |
| Cron | **Served.** `cron` 9,552,579 recent (465 revdeps), `croner` 4,887,100 (119). |
| RRULE | **Served.** `rrule` 660,438 recent (23 revdeps), `rrule-rs` free but the capability is covered. |
| iCalendar | **Served.** `icalendar` 407,612 recent (43 revdeps), `ical` 337,794. |

The demand here is among the highest measured in any round, and so is the
coverage: cron and RRULE both have maintained crates with hundreds of reverse
dependencies. **Do not build a recurrence crate.**

### Meteorology: a small, real opening

| | |
| --- | --- |
| Demand | PyPI `eccodes` 1,070,420/month, `cfgrib` 679,243/month, `pygrib` 212,668/month, `metar` 66,935/month. |
| GRIB | **Served-ish.** `grib` 48,556 recent, v0.20.2, published 2026-10-22, 8 revdeps, 86 stars. Live and improving. |
| METAR/TAF | **Thin.** `metar` 620 recent (2026-01), `metar-taf-parser` 5,734 recent but 0 stars and last pushed 2026-03, `aviation-wx` cluster ~150 recent, `rweather-decoder` 31 (2024). |
| BUFR | Near-empty (`bufr` 46 recent, 2023) but the source demand is thin too. |
| Names free | `metar-rs`, `wgrib`, `noaa`, `grib2` |

GRIB is served by a live crate. METAR/TAF is the one thin spot with real source
demand (`metar` 66,935/month, plus the aviation data feeds), and no incumbent has
an audience: the nearest maintained-looking crate has zero stars. This is a
genuine but **small** opening — the whole domain is worth less than one percent
of the cheminformatics or geospatial demand. It is recorded, not recommended as a
priority.

## The domain the sweep found outside the seven: runtime unit conversion

The seven chosen domains mostly came back served or racing, but the sweep
surfaced a strong opening in an adjacent domain that no earlier round measured:

| | |
| --- | --- |
| Demand | PyPI `pint` **5,241,817/month**. npm `convert-units` 128,323/week, `js-quantities` 47,827/week. |
| Rust side | `uom` 2,675,890 recent (215 revdeps) is **compile-time only**: the unit is part of the type, so it cannot parse `"3.5 kg"` or `"12 mph"` at runtime. `dimensioned` 118,020 (2022, compile-time). `measurements` 253,043 (34 revdeps, embedded, fixed set). `quantities` 11,175, `simple-si-units` 35,070, `unit` 159. |
| UCUM | `octofhir-ucum` 13,639 recent (v0.8.0, 2026-10-19, 8 revdeps) parses the UCUM code vocabulary for healthcare, but it is a code-string validator, not a general quantity-and-conversion engine with the full unit registry `pint` carries. |
| Names free | `runits`, `unitkit`, `unitparse`, `measure-rs` |

This is the same shape as `stdnum-rs` and `fuzzy-score`: a mature, heavily used
reference package whose Rust side is either compile-time-only or a fixed embedded
set, with no runtime string-to-quantity parser and converter. `pint`'s defining
feature is exactly the missing one — `ureg("3.5 kg").to("lb")` — and the
unit-definition file (`default_en.txt`, ~1,000 units and prefixes) is a data
table that can be transcribed and checked against the reference, which is this
workspace's method. **This is the round's recommended candidate.**

## What this round concludes

1. **Six of seven new domains are served or racing.** Cheminformatics (two
   toolkits in two weeks), bioinformatics (served four times over), geospatial
   (served six times over), audio/music (tags and MIDI served, notation racing),
   recurrence (cron and RRULE served), postal addresses (thin but needs
   `libpostal`'s statistical model). Do not re-scan them.
2. **Meteorology is the one small opening inside the seven**, METAR/TAF only,
   and it is small. Recorded, not prioritised.
3. **Runtime unit conversion is the round's real find**, and it was found by
   probing outside the seven planned domains. `pint` at 5.24M/month has no Rust
   counterpart: `uom` is compile-time by design, and the only runtime parser is a
   healthcare-scoped UCUM validator.
4. **The survivors are unchanged.** PDF tables remains the strongest opening
   measured in any round; OFX/QIF, subtitles and STIX/TAXII are still open;
   readability is still a watch.
5. **The method finding is confirmed and extended.** Round 16 saw the
   actively-claimed cluster; this round saw it at its most extreme — two complete
   cheminformatics toolkits and one music-notation toolkit published within two
   weeks. The rule stands: **before proposing a crate, sort the nearest crates by
   release date, not only by downloads.** A new entry into a two-week-old
   multi-crate race needs a reason to win that is not "nothing is here".

## Method note

The script is `tools/round17.py`:

```sh
python3 round17.py candidates       # re-measure the open survivors
python3 round17.py sweep            # the seven new domains
python3 round17.py names <name>...  # is a proposed crate name free?
```

Round 16's corrections are carried forward and re-confirmed: the sparse index
field is `pubtime`, not `pubdate`, and the index path is `2/<name>` for two
characters, `1/<name>` and `3/<c>/<name>` otherwise. The rate-limit warning in
`tools/README.md` applies in full: crates.io returns HTTP 429 under load and a
naive fetch reads that as "no such crate". Every `FREE` here was re-checked.

Round 15's rule also applies: **a keyword search on the upstream package's name
is not a gap check.** The two complete cheminformatics toolkits were found by
searching crates.io for the *capability* (`SMILES`, `molecule cheminformatics`,
`substructure`), not for `rdkit` or `openbabel`, because neither toolkit's name
contains the upstream package's name. The unit-conversion opening was found by
searching `dimensional analysis` and `physical quantity` rather than `pint`. A
name search would have reported both domains empty and both would have been
wrong.
