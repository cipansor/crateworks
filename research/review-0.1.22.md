# Review of `holidays-rs` against `holidays` (2026-11-16)

This round closes the backlog the gate had been carrying: `verify_holidays.py`
reported 63 calendars with an undocumented difference, and this round takes it to
47 by fixing 16 of them. The remaining 47 are all the same class of problem -
tables that back-project a modern rule into pre-1991 history - and are recorded
below as the next piece of work rather than half-fixed.

## What the measurement found

`research/tools/verify_holidays.py` checks each table from the reference's own
`start_year` through 2050. Every one of the 16 defects below was invisible in a
2000-onwards window, which is where the tables had been fitted. Two distinct
shapes recur:

1. **An `ObservedShift.from` anchored at 2000** on a country whose reference
   substitutes weekend holidays earlier. The rule was right and the start year
   was wrong, so every year between the real start and 2000 answered the raw
   date and not the observed one.
2. **A rule with no year gate at all** where the reference has one. Estonia's
   movable feasts, Armenia's post-Soviet national days and Cabo Verde's
   Democracy Day all exist only from a stated year, and the crate answered them
   for every year it was asked about.

A third shape appeared once, in France and its child entities: a holiday that is
absent for a *middle* stretch, which neither `Since` nor `Until` can express.

## Fixed calendars

| Country | Defects | Cause |
| --- | --- | --- |
| Germany | 4 | Buß- und Bettag is the Wednesday strictly before 23 November, national 1991-1994 |
| Lithuania | 1 | Statehood Day (6 July) from 1991 |
| Croatia | 4 | Dan državnosti 30 May is 1996-2001, not "until 2001" |
| Botswana | 3 | substitution starts 1995, not 2000 |
| Lesotho | 4 | King's Birthday moved from 2 May to 17 July in 1998 |
| Seychelles | 9 | substitution starts 1994 (the Act's own start year) |
| Estonia | 11 | the 1994 Act added Good Friday, Easter Sunday and Pentecost; `taassünnipäev` runs to 1993 |
| Armenia | 14 | 9 May from 1995, 5 July from 1996, 21 September from 1992, Motherhood Day 1994-2001, plus the 1998-12-07 one-off |
| Namibia | 16 | substitution starts 1991, plus seven ad-hoc holidays (Walvis Bay, election days, the Y2K changeover) |
| Cyprus | 18 | Independence Day from 1979 |
| WallisAndFutuna | 19 | 8 May follows France's 1982 restoration, not 1953 |
| CaboVerde | 24 | Democracy and Freedom Day from 2000 |
| FrenchSouthernTerritories | 21 | 8 May is 1953-1959 and 1982 onwards, with the 1960-1981 gap |
| Micronesia | 23 | substitution starts 1987; United Nations Day from 1991 |
| SaintLucia | 24 | substitution starts 1979 |
| Liberia | 9 | Unification Day from 1960; five pre-2006 inauguration years |

## Verification

* `verify_holidays.py` reports `ok` for all 16, and the full run drops from 63 to
  47 undocumented calendars with no new failures.
* `cargo test -p holidays-rs --all-targets` passes, including the
  `holidays_oracle.tsv` and `closest.tsv` fixtures that CI enforces.
* `cargo fmt --all -- --check` and `cargo clippy -p holidays-rs --all-targets --
  -D warnings` are clean. No `#[allow]` was added.

## What is left

The 47 remaining calendars are not the same bug. Each one is a table that applies
today's rule to the whole of its history, so the divergence runs from the
reference's `start_year` to about 1991: Finland reports 1509 differences, Italy
and the United States 527 each, Poland 320. Fixing them means adding the
historical layers the reference models (Italy before and after 1977, Poland's
pre-1989 list, the United States before the 1971 Uniform Monday Holiday Act) -
a per-country rewrite, not a start-year correction. That is the next round's
work, and the gate output is the specification for it.

## Release

`holidays-rs` 0.8.0 is already on crates.io (`holidays-rs-v0.8.0`, tag at
`9d362fd`), so this is a patch bump to 0.8.1: it corrects dates the crate
already answers rather than adding a calendar or a rule. Tag and push
`holidays-rs-v0.8.1` on its own after the PR merges.

## Files

* `crates/holidays-rs/src/countries.rs` - 16 country tables.
* `crates/holidays-rs/Cargo.toml` - 0.8.0 -> 0.8.1.
* `AGENTS.md` - two traps: the back-projected start year, and the holiday that is
  absent for a middle stretch.
