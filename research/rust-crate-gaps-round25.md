# Gap scan round 25: eight narrow regulated domains, all served (2026-10-21)

Rounds 22, 23 and 24 each ended with the same recommendation: the
general-purpose space is served, and the durable openings are *narrow*,
regulated or region-specific. This round acts on that conclusion directly. It
picks eight domains that are narrow, regulated or region-specific on purpose, to
test whether the hit rate actually rises when the premise is adjusted the way
the previous three rounds said it should be.

It does not. Eight for eight, the domains are served or being actively claimed.
The value of the round is that the *reason* is now visible, and it is a
measurement artefact rather than an absence: **a name-based sweep produces
"FREE" for a domain that is already served under a different name.** Four of the
eight domains produced a false FREE on the first pass.

## Method, and the false positives it produced

`research/tools/round25.py` measures each domain the way rounds 15-24 do - demand
from PyPI and npm, then the nearest crates by recent downloads, release date,
reverse dependencies and GitHub activity. The first pass, driven by *names*,
reported these as free:

| Name reported FREE | What is actually there |
| --- | --- |
| `ghostscript` | `stet` 207,657 recent, v0.8.7, published 2026-10-15 - a pure-Rust PostScript Level 3 interpreter |
| `kerberos` | `kerberos-parser` 130,903 recent (2025-02) for the wire format, plus `synta-krb5` 863 and `negotiate-auth` 21 both published 2026-10-17 |
| `cobol` / `cobol-copybook` | a full `copybook-*` suite (ten crates, `copybook-core` 749 recent, 12 revdeps) and an `ironwork-*` suite (`ironwork-zarch`, `ironwork-numeric`, `ironwork-syntax`) |
| `libais` | `aivdm` 205 recent, `ship162` 31, `marlin-ais` 151, plus `nmea-parser` 124,512 recent for the sentence layer |

Searching the *capability* instead of the name overturned all four immediately.
This is rounds 15, 22 and 23's rule again, and it is now the fourth consecutive
round where the rule caught a false positive that the name-based pass had
already written down. The rule is not being applied consistently; it should be
applied before any candidate is recorded, not as a check afterwards.

## The eight domains

| Domain | Demand | Rust today | Verdict |
| --- | --- | --- | --- |
| Label and receipt printers (ZPL/EPL/TSPL/ESC-POS) | PyPI `python-escpos` 71,444/mo, `zpl` 46,313/mo; npm `node-thermal-printer` 27,179/wk, `escpos` 11,521/wk | `escpos` 11,263 recent (2026-10-07, 119 stars), `ipp` 26,012, `tspl` 3,957, `labelize` 2,712 published **2026-10-21** covering ZPL, EPL and TSPL in one crate, `zpl-rs` 332, `zpl` 160 | **Served, and claimed today** |
| Mainframe / EBCDIC (COBOL copybook, COMP-3, JCL) | PyPI `cobol-parser` 339/mo, `pycobol` 139/mo; the `ebcdic` PyPI figure (6.9M/mo) is a codec table, not this capability | `copybook-*` suite, `ironwork-*` suite, `gnucobol-rs` 1,072, `ebcdic` 900, `ironwork-zarch` 298 | **Served twice over** |
| Maritime and radio telemetry (AIS, ADS-B, APRS, AX.25) | PyPI `pyais` 152,581/mo, `pynmea2` 112,298/mo | `nmea-parser` 124,512 recent, `nmea` 43,581 (2026-08, 111 stars), `ax25` 1,085 (2026-04, 63 stars), `ais` 3,080 (19 stars, pushed 2026-02), `aivdm`, `adsb` | **Served, decoding fragmented** |
| PostScript and PCL output | PyPI `ghostscript` 323,879/mo | `stet` 207,657 recent (pure-Rust PS Level 3), `postscript` 4.0M (fonts), `pdf-writer` 3.8M, `hayro-postscript` 3.4M | **Served** |
| vCard and vCalendar | PyPI `icalendar` 7.56M/mo, `vobject` 1.42M/mo, `caldav` 541,588/mo; npm `ics` 520,967/wk, `ical.js` 317,619/wk | `icalendar` 406,199 recent (43 revdeps), `ical` 338,725, `calcard` 36,092, `ics` 14,077, `vcard_parser` 9,289, `vcard` 1,478, `vcard-rs` 2,113 | **Served four times over** |
| RADIUS and TACACS+ | PyPI `pyrad` 255,575/mo, `tacacs_plus` 74,674/mo; npm `radius` 19,257/wk | `radius` 13,833 (2024-01), `radius-tokio` 1,523, `tacacs-plus-protocol` 2,760 (no-std, 2025-01), `tacacs-plus` 6, `diameter` 518 | **Served, RADIUS stale** |
| Kerberos and GSSAPI | PyPI `pyspnego` 14,265,353/mo, `gssapi` 1,567,954/mo, `requests-kerberos` 1,481,737/mo, `pykerberos` 1,414,012/mo; npm `kerberos` 274,102/wk | `libgssapi` 573,266, `sspi` 538,310, `picky` 625,514, `kerberos-parser` 130,903, `winauth` 140,424, `ntlmclient` 11,572, `synta-krb5`, `negotiate-auth` | **Served on both platforms** |
| NTP and PTP | PyPI `ntplib` 1,914,370/mo; npm `sntp` 1,401,163/wk | `ntp-proto` 18,360 (2026-06, the `ntpd-rs` core), `ntp` 6,853, `nts` 3, `ptp` 24 | **Served** |

## The two shapes worth recording

Neither is a build proposal. Both are measurements that contradict a common
assumption, which is the only thing this round has to offer.

**1. The "served" verdict now depends on a suite, not a crate.** Mainframe data
is the clearest case in twenty-five rounds of a capability served by a
*cluster*: `copybook-core`, `copybook-lexer`, `copybook-codec`,
`copybook-cli`, `copybook-arrow` and five more, plus a second independent suite
under the `ironwork-*` prefix. A name-based check sees two free names
(`cobol`, `cobol-copybook`) and concludes the space is empty, while twelve
crates with a maintainer behind them are shipping. The check that would have
caught it is a `?q=` capability search, which costs one request.

**2. A stale incumbent with a live successor is the common shape, not the
exception.** RADIUS is the example this round: `radius` is at 13,833 recent
downloads with no release since 2024-01-15 and zero reverse dependencies, which
is round 12's staleness filter exactly - but `radius-tokio` (1,523) and
`radius-rust` (222) are both younger, so the space is a race rather than an
opening. `ntp` (2018, 6,853) has `ntp-proto` (2026-06, 18,360) as its live
successor. `ais` (2024-10) has `nmea-parser` and `aivdm`. In every case the
abandoned crate is not the capability; it is one crate in a field that has
already moved past it.

## What this round concludes

1. **Nothing is proposed.** The premise was adjusted exactly as rounds 22-24
   recommended - narrow, regulated, region-specific - and the hit rate did not
   improve: eight of eight are served or racing.
2. **The false-positive rate of a name-based pass is 50% on this sample** (four
   of eight domains). That is higher than round 22's ~5% estimate, because that
   estimate was for a different population; for narrow domains, where a
   capability is often served under a generic crate name, the name is an even
   weaker signal. Run the capability search first.
3. **Round 24's recommendation stands unchanged, now with a fourth
   confirmation.** The workspace's defensible ground remains the Indonesian and
   Islamic-finance domain the scans keep pointing back to, plus the
   correctness-fix openings (`fuzzy-score`, `email_address`) rather than whole
   capabilities.

## Method

`research/tools/round25.py` holds the sweep (`sweep` prints the tables,
`names <n>...` checks a proposed name); it imports the measured helpers from
`round15`. Every figure was fetched live from `crates.io/api/v1`,
`index.crates.io`, `api.npmjs.org`, `pypistats.org` and `api.github.com`. The
four false positives above were each confirmed against a `?q=` capability search
and then against the crate's own API entry, never against a name lookup alone.
