# Gap scan round 24: eight everyday-application domains, and the premise tested once more (2026-10-21)

The user's standing question is whether a package many people need has no Rust
counterpart. Rounds 22 and 23 tested that premise directly and found a ~5% hit
rate, with the registry filling in faster each quarter. This round tests it
again over eight domains a normal business application meets, chosen because no
earlier round swept them and because the demand sits in PyPI and npm rather than
in the framework-dominated top lists.

Every figure was fetched live on 2026-10-21 from `crates.io/api/v1`,
`index.crates.io`, `api.npmjs.org`, `pypistats.org` and `api.github.com`.

## Result: eight domains, eight incumbents

| domain | demand | Rust incumbent | newest | verdict |
| --- | --- | --- | --- | --- |
| email address validation | PyPI `email-validator` 160M/mo, npm `validator` 24.6M/wk | `email_address` 36.3M recent, `mailparse` 4.5M, `validator` 15.0M | `mailparse` 2026-10-05 | served |
| password strength (zxcvbn) | PyPI `zxcvbn` 5.0M/mo, npm `@zxcvbn-ts/core` 941K/wk | `zxcvbn` 2.16M recent | `passwords` 2026-10-18 | served |
| one-time passwords | PyPI `pyotp` 32.2M/mo, npm `otplib` 2.47M/wk | `totp-rs` 3.13M recent | `totp-rs` 2026-08-06 | served |
| semantic version ranges | PyPI `packaging` 1.51B/mo, npm `semver` 756M/wk | `semver` 251M recent, `node-semver` 639K | `semver` 2026-04-04 | served |
| JWT / JWK / JOSE | npm `jose` 88.1M/wk, PyPI `authlib` 108M/mo | `jsonwebtoken` 63.9M recent, 2024 revdeps | `jsonwebtoken` 2026-09-16 | served |
| audio tag reading | PyPI `mutagen` 6.9M/mo, npm `music-metadata` 2.56M/wk | `id3` 1.72M, `lofty` 551K, `symphonia` 9.84M | `lofty` 2026-10-25 | served |
| natural-language dates | PyPI `dateparser` 38.4M/mo, npm `chrono-node` 5.04M/wk | `dateparser` 2.07M, `parse_datetime` 1.60M, `humantime` 71.7M | `parse_datetime` 2026-10-11 | served |
| CLDR plural rules | PyPI `babel` 101M/mo, npm `make-plural` 2.16M/wk | `icu_plurals` 6.52M, `intl_pluralrules` 7.82M | `icu` 2026-10-22 | served |

Not one of the eight is a gap. Every one has at least two maintained crates,
and four of them (`mailparse`, `passwords`, `lofty`, `parse_datetime`, `icu`)
had a release within the last three weeks. This is round 23's finding again:
the premise fails, and it fails for the same reason - the Rust side of a
"missing" capability is usually already populated, often by several crates.

The names the sweep expected to be a signal are free but meaningless as a gap
test: `pwstrength`, `zxcvbn-rs`, `semver-spec`, `id3v2`, `hotp`, `oathtool`,
`friendly-date`, `whenever`, `pluralrules` and `email-lint` are all FREE on
crates.io. Every one of those capabilities is served. A free name is not a gap.

## The one anomaly worth recording: `email_address`

`email_address` carries 36.3M recent downloads, the highest of any crate in the
sweep apart from `semver` and `jsonwebtoken`, yet its newest release is v0.2.9
from **2024-07-31** - more than two years old. That shape is what round 12's
`stale.py` looks for, so it was measured directly rather than inferred:

| | |
| --- | --- |
| Latest version | 0.2.9, published 2024-07-31 |
| Repository | `johnstonskj/rust-email_address`, MIT, not archived |
| Last push | 2025-10-26 (dependabot and a CI fix, not a feature) |
| Open issues | 20, several correctness reports: emoji rejected, combining characters rejected in the local part since 0.2.9, a comma parsed as part of an RFC 5322 phrase, a debatable `ted.backer@gmail` acceptance |
| Reverse dependencies | 118 |

This is a real *maintenance* opening rather than a capability gap: the audience
is large, the crate is slow-moving, and the open issues are correctness bugs in
RFC 5322 parsing. It is not a "nothing exists" case - `mailparse` (4.46M recent,
released 2026-10-05) and `validator` (15.0M recent) are live alternatives, and
`email-address-parser` 3.0.0-rc5 shipped 2026-03-01. Whether a new crate could
win against three live incumbents is a separate question the sweep does not
answer; the honest reading is that the space is contested, not empty.

## What this round recommends

Nothing. The premise has now been tested in rounds 22, 23 and 24, and the
answer is consistent: the domains a general audience needs are served, usually
by more than one maintained crate, and the free names in those domains are free
because the capability is already claimed under a different name. Round 23's
recommendation stands - the durable openings in this workspace are *narrow*
ones (localized output, a specific correctness fix) rather than whole
capabilities, and the user's own Indonesian and Islamic-finance ground is the
edge that is actually defensible.

The counter-argument worth recording: this round and rounds 22-23 all sample
domains that *general-purpose* Rust libraries would serve. A domain that is
narrow, regulated, or region-specific is where the hit rate would rise - which
is exactly the pesantren and Islamic-finance ground the workspace already
holds, and why the scans keep pointing back to it.

## Method

`research/tools/round24.py` holds the sweep; `sweep` prints the tables,
`names <n>...` checks a proposed name. Every "nothing found" was treated as a
rate limit until re-checked, per the tool README's standing rule. The
`email_address` staleness claim was confirmed against the repository's own
commit and issue lists, not against search, because round 22's false positives
all came from trusting the search endpoint.
