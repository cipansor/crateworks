# Gap scan round 21: a demand-first sweep of the PyPI top list (2026-10-21)

Rounds 8-20 each re-measured the open candidates and then swept domains no
earlier round had touched. This round tries a **different method**, because the
domain-sweep method has a known blind spot: it only finds a gap if the sweep
happens to name the domain.

The method here is **demand-first**. Take the 600 most-downloaded PyPI packages
(the `top-pypi-packages` list), ask crates.io for a crate of the same name, and
then, for every capability that looked unclaimed, search crates.io for the
*capability* rather than the name. The first step is a coarse filter and is
mostly noise - 393 of the 600 names have no crates.io crate of the same name,
because they are Python-only libraries (`boto3`, `pytest`, `pandas`). The second
step is the measurement that matters.

Every figure was fetched live on 2026-10-21 from `crates.io/api/v1`,
`index.crates.io` (the sparse index, for a real release date), `api.npmjs.org`,
`pypistats.org` and `api.github.com`. A gap is real when the Rust incumbent has
under about 20,000 recent downloads (the rolling ninety-day figure) and
near-zero reverse dependencies, while the source counterpart has real volume.

Round 15's two rules are applied throughout: **a keyword search on the upstream
package's name is not a gap check** - search the *capability*; and **before
proposing a crate, sort the nearest crates by release date, not only by
downloads, and ask whether any of them has an audience.** A cluster of unadopted
crates is a race, not a gap.

## Verdict summary

| Domain | Upstream volume | Best Rust incumbent | Verdict |
| --- | --- | --- | --- |
| Symbolic maths / CAS | PyPI `sympy` 60M/mo | ~15 crates, largest `cas-domain` 3,511 | **demand is real, difficulty blocks it** |
| PDF table extraction | PyPI `pypdf` 126M/mo, `pdfplumber` | `pdfplumber` 43,124 (alpha), `pdf-text-extract` 35,389 | **closing** - was open in round 15 |
| Structural JSON diff | PyPI `deepdiff` 53M/mo | `json-structural-diff` 1,023,053 | **served on volume, thin on revdeps** |
| Colour science | PyPI `colour-science` 289k/mo, npm `color` 46M/wk | `palette` **20,605,564** (425 revdeps) | served, decisively |
| Spell checking | PyPI `pyspellchecker` 2.26M/mo | `spellbook` 162,886 (20 revdeps) | served |
| Language detection | PyPI `langdetect` 9.97M/mo | `whatlang` 1,672,273 (71 revdeps) | served |
| PDF forms (AcroForm) | PyPI `pypdf` 126M/mo, npm `pdf-lib` 7.4M/wk | `acroform` 35,088, `pdfer_forms` 4,997 | served |
| Money / ISO 4217 | PyPI `py-moneyed` 1.23M/mo | `iso_currency` 637,488 (36 revdeps) | served |
| ISO 8601 durations | PyPI `isodate` 173M/mo | `iso8601` 6,518,823, `humantime` 71,732,184 | served |
| HTML to Markdown | PyPI `markdownify` 54.7M/mo | `htmd` 6,325,906 (127 revdeps) | served |
| vCard / iCalendar | PyPI `icalendar` 7.56M/mo | `icalendar` 404,885 (43 revdeps) | served |
| Plain-text accounting | PyPI `beancount` 132,884/mo | `rustledger-*` 7 crates, `doubleentry` 1,301 | served, by a race that shipped |
| Hyphenation | PyPI `pyphen` 15M/mo | `hyphenation` 307,394 | served |
| Time mocking | PyPI `freezegun` | `faux` 114,116, `qubit-clock` 45,754 | served, by a different design |

## The one domain worth arguing about: symbolic maths

PyPI `sympy` is downloaded about 60 million times a month, and it has no Rust
counterpart with an audience. Searching the *capability* rather than the name
finds a dense cluster of attempts:

```
symcas          3,445    cas-domain      3,511    cas-poly   3,501
cas-expr        3,487    alkahest-cas    2,409    mathcore   1,126
savage_core       100    mathr             193    usme           6
ocas              261    thales             74    rme            2
scirs2-symbolic   423    arael             529    tpt-math-symbolic 70
```

Fifteen crates, none above 3,511 recent downloads, none with a real audience.

This is the shape round 16 named - a cluster of unadopted crates - and the rule
says a race is not a gap. But the usual reading of that rule needs care here,
because the race is not evidence that the capability is easy and crowded. It is
evidence that the capability is **hard enough that nobody has finished it**: a
computer algebra system needs canonical forms, polynomial arithmetic, exact
rationals, simplification heuristics and a rewrite engine before it does
anything a `sympy` user would recognise. The crates in the cluster are each one
of those pieces.

**Verdict: do not enter.** Not because the registry is full, but because the
difficulty is research-grade and the winners would be a multi-year project. This
is the honest reason, and it is different from the reason the ODT race was
rejected (that race had no demand behind it; this one has a great deal).

## The finding that matters: the PDF table opening is closing

Round 15 recorded PDF table extraction as **the strongest opening measured in
any round**: PyPI `pdfplumber` and `camelot` have real volume and the Rust space
was empty. That is no longer true. Crates published since then:

| Crate | Recent | Note |
| --- | --- | --- |
| `pdf-inspector` | 1,150,929 | PDF classification and text, not table structure |
| `pdfplumber` | 43,124 | A Rust port, self-described **alpha** |
| `pdfplumber-core` | 43,187 | The same project, split across five crates |
| `pdf-text-extract` | 35,389 | "text, tables, and structured content" |
| `pdfsink-rs` | 6,572 | Fast extraction, tables claimed |
| `pdf-xlsx` | 93 | PDF table to XLSX |
| `pdfraw` | 64 | "pdfplumber-inspired", layout-preserving |
| `tabula` | 39 | Bindings to the Java `tabula` |

None of these has an audience yet, so by the letter of the rule this is a race
and not a gap. But the direction of travel is the point: a five-crate port of
`pdfplumber` at 43,000 downloads in alpha is exactly how the YAML opening closed
between rounds 12 and 13, and how the WOFF2 encoder closed between rounds 17 and
18. **An opening that has acquired eight entrants in a few weeks is not an
opening to start now.** If this crate is to be built, the reason to win has to
be something the eight do not have, not "nothing is here".

## Corrections this round records

1. **Round 18's structural-diff claim is weaker than stated.** It called the
   opening thin because the leader was a 2024 research artifact. The leader,
   `json-structural-diff`, has **1,023,053 recent downloads** - the reverse
   dependency count is 3, which is what made it look unadopted. Downloads and
   reverse dependencies disagree here, and for a library that people call
   directly from an application rather than through another crate, downloads are
   the better signal. The opening is thinner than round 18 implied.
2. **`pdf-inspector` at 1.15M recent was invisible to the round-15 sweep**,
   because it describes itself as "inspection and classification" rather than as
   a table extractor. This is the round-15 rule biting: the capability was there
   under a different word.
3. **The demand-first method has a low signal-to-noise ratio but it is not
   useless.** Of 600 top PyPI names, 393 were absent from crates.io and only one
   - the CAS cluster - was a genuinely unclaimed capability. The method costs
   about 600 rate-limited requests to find one candidate. The domain-sweep method
   costs less and found more. Use demand-first as a cross-check, not as the
   primary sweep.

## What this round concludes

1. **No new crate is proposed.** Ten domains were swept and every one is served,
   racing, or blocked by difficulty. The workspace's open list is unchanged.
2. **The PDF table opening, the archive's strongest, is closing.** It should be
   dropped from the "strongest opening" position it has held since round 15, or
   re-argued from scratch against the eight crates now in the space.
3. **The CAS cluster is a new pattern: a race caused by difficulty, not by
   ease.** Round 16's rule ("a cluster of unadopted crates is a race, not a
   gap") is correct but incomplete. A cluster can also mean *nobody has managed
   it yet*, and that is a reason to stay out that is worth writing down, because
   it looks like a gap from every download figure.
4. **Downloads and reverse dependencies can disagree.** When they do, check
   which one describes how the crate is actually consumed before calling
   something unadopted.

## Method

The scripts used are the existing helpers, not new ones:

```sh
python3 research/tools/round19.py candidates   # re-measure the open survivors
python3 research/tools/round20.py sweep        # the eight round-20 domains
```

The demand-first cross-check fetches
`https://raw.githubusercontent.com/hugovk/top-pypi-packages/main/top-pypi-packages-30-days.min.json`
for the ranked PyPI list, then calls `crate()` and the crates.io search endpoint
for each capability. The rate-limit warning in `tools/README.md` applies in
full: crates.io returns HTTP 429 under load and a naive fetch reads that as "no
such crate". Every `FREE` here was re-checked, and every "served" verdict was
reached by searching for the capability, not the name.
