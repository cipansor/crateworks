# Tools: Rust crate gap research

The scripts used for the research in `../rust-crate-5-final.md`. All use public
APIs without a key. They need Python 3 and no external dependencies.

| Script | Purpose |
|---|---|
| `crates.py` | crates.io API wrapper (cache + retry). Usable directly: `python3 crates.py semver glob` |
| `npmfetch.py` | Fetch weekly downloads and metadata from npm. Batching is safe for scoped names |
| `demand.py` | npm and PyPI demand figures for a curated list of candidates |
| `pipeline.py` | The `pypi`, `crates-npm` and `crates-pypi` stages |
| `probe.py` | Probe ~263 curated capabilities against crates.io |
| `revdeps.py` | **The most important one**: reverse dependencies and incumbent repo activity |
| `incumbents.py` | Competitor detail for each candidate |
| `verify2.py` | Check 30 further areas (transit, health, finance, and so on) |
| `round8.py` | Round 8: the PyPI top list against crates.io. Prints the tables in `../rust-crate-gaps-round8.md`. Treats a 404 as `FREE` and a rate limit as an error, so the two are never confused |
| `gen_hijri_table.py` | Regenerate the Umm al-Qura table and test fixtures for the `hijridate` crate. Needs `pip install hijridate`. Run from the repository root: `python3 research/tools/gen_hijri_table.py crates/hijridate` |
| `gen_hijridate_api.py` | Regenerate the formatting-and-accessor fixture for the `hijridate` crate (`tests/fixtures/hijridate_api.tsv`). Needs `pip install hijridate`. `python3 research/tools/gen_hijridate_api.py crates/hijridate` |
| `namecheck.py` | **Run this before writing code**: is the crate name still free? `python3 namecheck.py iso20022 xmlkit` |
| `stale.py` | Round 12: **the abandoned incumbent, measured**. Pages the crates.io ranked list by recent downloads and reads each crate's newest live version from the sparse index, so "popular but nobody is maintaining it" becomes a scan rather than an anecdote. A yank is not a release, so a pulled version does not count as activity. `--github` adds an archived-repository check (needs `GITHUB_TOKEN`). Caches index reads under `.cache/` |
| `verify_holidays.py` | **Regression gate for `crates/holidays-rs`**: compares every country table against the Python `holidays` package over 2000-2050 and exits non-zero on any difference that is not in its `KNOWN_DIFFERENCES` list. Needs `pip install holidays` |
| `round14.py` | Round 14: **re-measures every candidate earlier rounds left open, then sweeps the Islamic/Indonesian domain.** `candidates` prints each survivor with recent downloads, the release date read from the sparse index, reverse dependencies and the GitHub archived flag; `islamic` sweeps prayer times, Hijri, Quran, Arabic text, Islamic finance and Indonesian government identifiers; `names <name>...` checks whether a proposed crate name is free. Prints the tables in `../rust-crate-gaps-round14.md` |
| `round15.py` | Round 15: **re-measures the survivors, then sweeps seven utility domains no earlier round touched** (human-readable formatting, natural-language dates, typography/hyphenation, text repair/transliteration, MIME sniffing, web feeds and crawler policy, geo hashing). `candidates`, `sweep`, `names <name>...`. Six of the seven are served; the finding is the PDF table-extraction opening and the `ftfy`/`plsfix` correction. Prints the tables in `../rust-crate-gaps-round15.md` |
| `round13.py` | Round 13: **run this before acting on any round-12 candidate**. Re-measures the round-12 headlines and sweeps four new areas. Its result is a correction: the YAML opening round 12 called the largest in twelve rounds is closed, because `yaml-rust2` (19.6M recent), `serde_yaml_ng` (11.6M) and `serde-saphyr` (9.3M) already carry the audience. Sections: `yaml`, `candidates`, `demand` |
| `round16.py` | Round 16: **re-measures the survivors, then sweeps seven specialist protocol and data-format domains** the popularity lists under-represent (healthcare HL7 v2/FHIR/DICOM, energy and EV charging OCPP/Modbus/DLMS, field buses CAN/J1939/OPC UA, threat intelligence STIX/TAXII/YARA/Sigma/PCAP, supply chain SBOM/in-toto/sigstore, market data XBRL/FIX/NACHA, identity JOSE/WebAuthn/PKCS#12). `candidates`, `sweep`, `names <name>...`. Six of the seven are served or being actively claimed; the finding is the STIX/TAXII opening and the "actively-claimed cluster" method rule (sort the nearest crates by release date, not only downloads). Prints the tables in `../rust-crate-gaps-round16.md` |
| `round17.py` | Round 17: **re-measures the survivors, then sweeps seven scientific and everyday-data domains** (cheminformatics SMILES/InChI/fingerprints, bioinformatics FASTA/SAM/VCF, geospatial projections/shapefiles/GeoTIFF, audio and music ID3/MIDI/MusicXML, postal addresses and phone numbers, recurrence cron/RRULE/iCalendar, meteorology METAR/GRIB/BUFR). Imports the measured helpers from `round15` and `github` from `round16`. `candidates`, `sweep`, `names <name>...`. Six of the seven are served or racing (two complete cheminformatics toolkits were published within two weeks); the find is the runtime unit-conversion opening (`pint` 5.24M/month, `uom` is compile-time only), which was surfaced outside the seven. Prints the tables in `../rust-crate-gaps-round17.md` |
| `round18.py` | Round 18: **re-measures the survivors, then sweeps seven document and text-processing domains** the popularity lists under-represent (bibliography BibTeX/RIS/CSL, office documents DOCX/ODT/RTF, EPUB, typesetting LaTeX-to-MathML, diff and patch, segmentation CJK/sentences, emoji and IDNA). Imports the measured helpers from `round15` and `github` from `round16`. `candidates`, `sweep`, `names <name>...`. Six of the seven are served or racing — the ODT/ODF cluster is the sharpest measured in any round, seven crates published on one day — and the find is the **structural diff of nested data** opening (`deepdiff` 53.0M/month against a leader that is a 2024 research artifact). Prints the tables in `../rust-crate-gaps-round18.md` |
| `verify_fuzzy_score.py` | **Regression gate for `crates/fuzzy-score`**: runs the crate and Python `fuzzywuzzy` 0.18 over a seeded corpus of ASCII, Unicode and long strings, and exits non-zero on any difference. Refuses to run if `python-Levenshtein` is installed, because that silently makes `fuzzywuzzy` use a different matcher and every comparison would be meaningless. `--write-fixture` regenerates `tests/fixtures/oracle.tsv`. Needs `pip install fuzzywuzzy==0.18.0` |
| `verify_iso20022.py` | **Regression gate for the ISO 20022 writers in `crates/iso20022-rs`**: builds each message with the crate and validates it against the official ISO 20022 XSD (downloaded once into `.cache/`). A message a bank rejects is worse than no message, so the schema is the oracle. Takes `pacs008 pain001` to pick messages. Needs `pip install xmlschema` |
| `gen_stdnum_vectors.py` | **Fixture generator for `crates/stdnum-rs`**: builds `tests/fixtures/stdnum_vectors.tsv`, the numbers `tests/readme.rs` replays against the crate. Every vector is labelled by `python-stdnum`, so the file is an oracle rather than a sample. Merge mode: lines for a country already in the file are kept and only new countries are appended, so an existing country never loses coverage. Needs `pip install python-stdnum` |

## Operational warnings

- **Do not run two scripts at once.** crates.io rate-limits requests; running them
  in parallel triggers HTTP 429 and produces false empty results — a gap that
  looks real but is not.
- **`revdeps.py` runs its whole scan on import**, which is exactly the trap: a
  script that does `import revdeps` to reuse `revdeps()` also fires 33 crates.io
  requests, gets rate-limited, and then every later search in that script returns
  empty. A capability that is actually served then reads as a gap. Round 5 lost a
  run to this and nearly recorded "Hashids: nothing found". It now has a
  `if __name__ == "__main__"` guard, so importing it is safe; `probe.py`,
  `incumbents.py`, `verify2.py` and `gen_hijri_table.py` do not, so never import
  those — run them, or copy the helper out.
- **Treat any "nothing found" as a rate limit until it is re-checked.** A second
  run a minute later is cheap; a wrong gap is not.
- **Re-measure before building, not after.** A gap that is "the registry is
  empty" or "the incumbent is dead" can close within weeks. MRZ went from empty to
  served between rounds 10 and 11; the YAML opening round 12 called the largest in
  twelve rounds had three maintained successors by round 13. The document records
  the finding; it does not keep it true.
- The npm bulk endpoint rejects a mix of scoped and unscoped names (HTTP 400).
  Keep the two apart, at most 32 per call.
- `revdeps.py` is the true measure of Rust demand. PyPI and npm download figures
  measure demand in Python and JavaScript, not in Rust.
- **Run `namecheck.py` before writing code, not after.** A crate name cannot be
  changed after publication. `iso20022` looked like an obvious gap until it turned
  out the name was taken by a crate dead since 2020.
