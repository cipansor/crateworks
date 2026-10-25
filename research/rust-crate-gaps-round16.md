# Gap scan round 16: the survivors re-measured, and seven specialist protocol domains (2026-10-25)

Round 15 closed with PDF table extraction as "the strongest opening measured"
and readability extraction as "a watch, not a build". This round does the same
two jobs rounds 13, 14 and 15 did: measure every survivor again, then sweep
domains no earlier round touched.

Rounds 8-15 searched the npm and PyPI popularity lists and then seven *utility*
domains (formatting, dates, typography, text repair, MIME, feeds, geo). Those
lists are dominated by web frameworks and data-science packages. This round
picks seven **specialist protocol and data format** domains that the popularity
lists under-represent but that whole industries depend on: healthcare, energy
and EV charging, field buses, threat intelligence, software supply chain,
market data, and identity. They are chosen for fit with this workspace, whose
strength is transcribing a real specification and checking the result against a
reference.

Every figure was fetched on 2026-10-25 from `crates.io/api/v1`,
`index.crates.io` (the sparse index, for a real release date), `api.npmjs.org`,
`pypistats.org` and `api.github.com`. A gap is real when the Rust incumbent has
under about 20,000 recent downloads and near-zero reverse dependencies, while
the source counterpart has real volume. Recent downloads are the rolling
ninety-day figure.

## The surviving candidates, re-measured

| crate | recent | last release | repo | verdict |
| --- | --- | --- | --- | --- |
| `pdf-extract` | 3,645,973 | 2026-09-16 | 282 revdeps | text, not tables; the table opening stands |
| `pdfplumber` (Rust) | 42,412 | 2026-08-30 | 1 revdep, 6 stars | still v0.4.1, self-described alpha |
| `dom_smoothie` | 533,347 | 2026-09-21 | 43 revdeps | the readability incumbent, live |
| `ofx-rs` / `ofxy` / `qif_parser` | 2,957 / 1,324 / 374 | 2026 | 1 / 0 / 0 revdeps | **still open**, unchanged |
| `subtp` / `subparse` / `subtitler` | 21,377 / 4,907 / 1,987 | 2024 / 2021 / 2026 | 3 / 9 / 0 revdeps | **still open**, unchanged |
| `fuzzy-score` | 11 | 2026-10-23 | 0 revdeps | shipped here; migration in progress |
| `im` | 4,784,458 | 2022-04-29 | 238 revdeps, **archived** | `rpds` 4.84M and `ordermap` 7.24M both live |

Nothing changed verdict since round 15 except that `fuzzy-score` now exists on
crates.io. The four open candidates (PDF tables, OFX/QIF, subtitles, and the
readability watch) are unchanged, so they are not re-argued here.

## The seven specialist domains

### Healthcare: HL7 v2 and FHIR are being claimed as this round runs

| | |
| --- | --- |
| Demand | PyPI `pydicom` 5,802,226/month, `fhir.resources` 2,516,366/month, `hl7apy` 757,923/month, `hl7` 563,118/month. npm `dicom-parser` 113,118/week, `dcmjs` 91,871/week. |
| DICOM | **Served.** `dicom` 302,533 recent, `dicom-core` 420,543, `dicom_object` 418,388, all from one maintained repository (569 stars). Do not build a DICOM crate. |
| HL7 v2 | **Being claimed right now.** `hl7-parser` 6,615 recent but its own description says it "does not validate the correct structure"; `hl7-2` 503, `hl7` 251 and `vyna-hl7` 71 all published **2026-10-16**, within ten days of this scan. |
| FHIR | **Being claimed.** `helios-fhir` 53,494 recent, published 2026-10-18, 13 revdeps; `fhir` 793. |
| Names free | `hl7-rs`, `rust-fhir` |

The shape here is new. In earlier rounds a domain was either served (a
maintained crate with an audience) or open (nothing). HL7 v2 is neither: it is a
**cluster of brand-new, tiny, near-zero-download crates all published in the
same week**, which is a snapshot of a race rather than an opening. This
workspace already ships `mllp`, the transport half of HL7 v2, so the message
parser is the adjacent crate — but publishing into a five-way race in one week
is a different bet from round 14's `fuzzy-score`, and it should be treated as
one.

### Energy and EV charging: OCPP and Modbus are served, DLMS is a race

| | |
| --- | --- |
| Demand | PyPI `pymodbus` 1,328,148/month, `ocpp` 142,781/month, `asyncua` 706,916/month, `dlms-cosem` 46,619/month. npm `modbus-serial` 48,176/week, `node-opcua` 48,953/week. |
| OCPP | **Served.** `rust-ocpp` 33,171 recent, v3.0.4, 2025-11-30, 105 stars. |
| Modbus | **Served.** `tokio-modbus` 413,169 recent, 58 revdeps. |
| IEC 61850 | Near-empty (`iec61850` 32 recent) but there is no source demand to match, so it is not a measured gap. |
| DLMS/COSEM | `dlms_cosem` 294 recent but last released **2022**; `dlms-cosem-rs` 36, published 2026-09-07; `spodes-rs` 25. A young cluster against 46,619/month. |
| Names free | `dlms-rs`, `modbus-serde` |

Do not build OCPP or Modbus. DLMS/COSEM is the only opening and it is small.

### Field buses: served

`socketcan` 2,173,323 recent (70 revdeps), `embedded-can` 2,826,856 (76
revdeps), `can-dbc` 176,772 (14 revdeps). CAN, SocketCAN and DBC parsing are
served. `j1939` is thin (913 recent) against a thin source (`j1939` on PyPI
3,430/month), so it is not a measured gap. `can-isotp` is a free name with no
source demand figure to justify it. **Do not build a CAN crate.**

### The one new opening: STIX/TAXII threat intelligence

| | |
| --- | --- |
| Demand | PyPI `yara-python` 1,420,099/month, `stix2` 703,515/month, `pysigma` 385,306/month, `taxii2-client` 139,012/month, `scapy` 3,249,071/month, `dpkt` 1,477,073/month. |
| YARA | **Served.** `yara-x` 322,997 recent, v1.22.0, published 2026-10-20, 1,331 stars, 30 revdeps. |
| Sigma | `rsigma` 228 recent against 385,306/month. Small but the incumbent is young. |
| STIX 2.1 | **Fragmented and thin.** `stix` 889 recent (v0.3.0, **2021**), `rstix` 297 (2026-10-19), `stix-rs` 97, `stix2` 40, `stixflayer` 9. Four separate young crates, none with an audience. |
| TAXII | `cc-taxii2-client-rs` 9 recent, v0.1.5, **2024-01-28**, 0 revdeps. Effectively one abandoned client. |
| Names free | `taxii-rs` |

STIX/TAXII is the round's best-shaped new opening: real demand (703,515/month
for `stix2`, 139,012/month for `taxii2-client`), and the Rust side is a
fragmented set of crates that either predate STIX 2.1 adoption (`stix`, 2021) or
have no users (the 2026 cluster). It extends this workspace's XML/JSON message
line (`iso20022-rs`, `iso8583-msg`) and the same "transcribe the spec, compare
against the reference" method applies. It is not as strong as PDF tables on
volume, but it is a genuine capability gap rather than a race.

### Software supply chain: served four times over

`spdx` 6,892,106 recent (79 revdeps), `cyclonedx-bom` 1,356,341 (12 revdeps),
`sigstore` 384,103, `tough` 758,170, `cargo-sbom` 69,060. SPDX, CycloneDX,
sigstore and TUF all have maintained Rust crates with real audiences.
`in-toto` is thin (6,258 recent) against a thin source (14,167/month). **Do not
build an SBOM crate.**

### Market data: served, or too small to matter

`fefix` 13,725 recent covers FIX. XBRL has a young cluster (`xbrl-rs` 388,
`crabrl` 78, `xbrlkit` 29) but the source demand is tiny (`arelle` 3,063/month,
`python-xbrl` 5,639/month), so the ratio is not there. NACHA is thin on both
sides (`bryl-nacha` 26 recent, `nacha` 444/month on PyPI). SWIFT MT has no
source demand figure. **Nothing here clears the bar.** Note this workspace
already ships `iso8583-msg`, so the card-message space is covered from inside.

### Identity: served many times over

`jsonwebtoken` 63,824,829 recent (2,014 revdeps), `openidconnect` 5,088,022,
`jwt` 4,634,578, `webauthn-rs` 2,999,707, `pkcs12` 2,325,647, `sequoia-openpgp`
331,858, `josekit` 695,043. Every sub-domain — JOSE/JWT, OpenID, WebAuthn/FIDO2,
PKCS#12, OpenPGP — has a maintained crate with a large audience. **Do not build
an identity crate.**

## What is genuinely open

### PDF table extraction (carried from round 15, still the strongest)

| | |
| --- | --- |
| Demand | PyPI `pdfplumber` **40,689,748/month** (round 15), `camelot-py` 721,592/month, `tabula-py` 1,157,398/month. |
| Rust side | `pdf-extract` 3,645,973 recent extracts text, not tables. `pdfplumber` (Rust) 42,412 recent, v0.4.1, "Alpha evidence-driven". `tabula` 42 recent (a Java binding). `pdf_tables` 1 download. |
| Names free | `pdftable`, `pdf-table-rs` |

Unchanged and still the largest demand measured in any round. Extends the
workspace's PDF line (`pdfa`, `pades`, `pdfmake`).

### OFX/QIF and subtitles (carried since round 13)

Both unchanged. OFX/QIF: `ofx-rs` 2,957 recent, `ofxy` 1,324, `qif_parser` 374,
names `ofx-parse` and `ofxkit` free. Subtitles: `subtp` 21,377, `subparse`
4,907, `subtitler` 1,987, name `subkit` free. Small demand but no incumbent with
an audience.

### STIX/TAXII (new this round)

The one new candidate the sweep produced. See above.

## The method finding: the actively-claimed cluster

Rounds 11-15 established that a gap written down is not a gap today — an
archived incumbent is a migration in progress, and the re-measurement *is* the
work. This round adds a second, sharper shape:

**A domain can be neither served nor open. It can be a race.** HL7 v2, FHIR,
XBRL and DLMS all show the same signature: three to five tiny crates, each under
a thousand recent downloads, published within days of each other in the last two
weeks. `hl7`, `hl7-2` and `vyna-hl7` were all published on 2026-10-16;
`helios-fhir` on 2026-10-18; `rstix` on 2026-10-19. A name being free and the
space looking empty is a snapshot, and in these four domains it is a snapshot of
other people claiming the same ground at the same moment.

The practical rule: **before proposing a crate, sort the nearest crates by
release date, not only by downloads.** A cluster of near-zero-download crates
from the last month means the space is being claimed, and a new entry needs a
reason to win that is not "nothing is here".

This does not apply to PDF tables, OFX/QIF or subtitles, where the nearest
crates are months or years old, or to STIX/TAXII, where the incumbent (`stix`)
is from 2021 and the TAXII client is abandoned since 2024.

## What this round concludes

1. **Six of seven new specialist domains are served or being actively claimed.**
   Healthcare (DICOM served, HL7 v2 and FHIR a race), energy (OCPP and Modbus
   served), field buses (served), supply chain (served four times over), market
   data (served or too small), identity (served many times over). Do not re-scan
   them for "is there a crate"; the answer is yes.
2. **PDF table extraction remains the strongest opening measured**, unchanged
   from round 15. 40.7M/month for `pdfplumber` and still no production Rust
   crate.
3. **STIX/TAXII is the one new candidate.** Real demand (`stix2` 703,515/month,
   `taxii2-client` 139,012/month) against a fragmented, young Rust side, and it
   fits this workspace's XML/JSON message line.
4. **OFX/QIF and subtitles are still open and unchanged**, ranked as round 14
   left them (OFX better on fit, subtitles better on volume).
5. **The actively-claimed cluster is the round's method finding.** Sort by
   release date before proposing; a week of near-zero-download releases is a
   race, not an opening.

## Method note

The script is `tools/round16.py`:

```sh
python3 round16.py candidates       # re-measure the open survivors
python3 round16.py sweep            # the seven new specialist domains
python3 round16.py names <name>...  # is a proposed crate name free?
```

Two method corrections carried forward and re-confirmed: the sparse index field
is `pubtime`, not `pubdate` (a wrong field name fails silently as an empty
string), and the index path is `2/<name>` for two characters, `1/<name>` and
`3/<c>/<name>` otherwise. The rate-limit warning in `tools/README.md` applies in
full: crates.io returns HTTP 429 under load and a naive fetch reads that as "no
such crate". Every `FREE` here was re-checked, and the script sleeps 0.7s
between crates.io requests and retries a 429 with a growing backoff.

Round 15's rule applies to every finding above: **a keyword search on the
upstream package's name is not a gap check.** `rstix` and `dlms-cosem-rs` were
found by searching the crates.io *description* for the capability, not by
searching for `stix` or `dlms`, because neither name contains the upstream
package's name in a form a naive search would catch. The capability searches in
this round (`HL7`, `FHIR`, `STIX`, `TAXII`, `DLMS`, `XBRL`, `SBOM cyclonedx`)
are what turned "empty" into "a race".
