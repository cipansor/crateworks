# Review of `holidays-rs` against `holidays` (2026-11-16)

This round closes the backlog the gate had been carrying: `verify_holidays.py`
reported 63 calendars with an undocumented difference, and this round takes it to
38 by fixing 25 of them. The remaining 38 are all the same class of problem -
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

The French family is a second group, fixed after the review above: 1 May, 11
November and each entity's abolition day were ungated `Fixed` rules, and New
Caledonia's 24 September and French Polynesia's 29 June were `Until`-only, so
each ran backwards past its first year.

| Country | Cause |
| --- | --- |
| France | 1 May from 1919, 11 November from 1922 |
| FrenchGuiana | shared bounds, plus 10 June abolition from 1984 |
| Guadeloupe | shared bounds, plus 27 May abolition from 1984, Schoelcher from 1984 |
| Martinique | shared bounds, plus 22 May abolition from 1984, Schoelcher from 1984 |
| Reunion | shared bounds, plus 20 December abolition from 1983 |
| Mayotte | shared bounds, plus 27 April abolition from 1984 |
| NewCaledonia | shared bounds, plus 24 September is 1953-2003 |
| FrenchPolynesia | shared bounds, plus 5 March from 1978, 29 June is 1985-2024 |
| SaintPierreAndMiquelon | shared bounds only |

## Review follow-up

Review found four defects in the first cut of this round. All are fixed here.

**`Rule::Until` invented the holiday in earlier years.** Two entries used
`Until` with no lower bound, so the form was reported back to year zero:
Estonia's `taassünnipäev` (`Until { year: 1993 }`) made Monday 16 November 1987
a holiday although the declaration it marks was in 1988, and Lesotho's 2 May
King's Birthday reached 1935, before Moshoeshoe II was born. Both also made
`is_working_day` false for those weekdays. The reference reports these forms
only from its own `start_year` - 1991 for Estonia, 1996 for Lesotho - so both
are now `Between` with that first year. The gate missed them because its window
starts at `start_year`, which is exactly where the leak stops.

**France still listed 8 May every year, and the overseas entities with it.**
`Fête de la Victoire` was one ungated `Fixed`, so it was reported before 1953
and through the 1960-1981 gap. The reference has it 1953-1959 and from 1982.
France now carries that shape, and so do the eight entities that copy the
metropolitan rule (French Guiana, Guadeloupe, Martinique, Reunion, Mayotte, New
Caledonia, French Polynesia, Saint Pierre and Miquelon). The French Southern
Territories is bounded at 1956, its own reference start year, not at 1953;
Wallis and Futuna was already correct at `Since 1982` and is unchanged.

**The fixes had no CI coverage.** The holiday corpus covered six calendars, none
of them touched here, and the closest-holiday probes start after the dates
changed, so nothing would have caught either regression. `tests/historical_bounds.rs`
adds seven cases that pair each first valid year with the year before it,
including the `is_working_day` consequence. Reverting the Estonia fix makes two
of them fail, so they are a real guard rather than a restatement.

Those probes are still only hand-picked dates, so the corpus itself now covers
every calendar this round touched: `scripts/fuzz/holidays-rs/gen.py` goes from 6
calendars to 31, and the fixture from 10,935 rows to 62,481. That is 34,691
required dates, 9,190 weekend-allowed ones and 18,600 dates the crate must stay
silent on. The window is the gate's own - each country's reference `start_year`,
floored at `EARLIEST = 1900` - run to `LAST = 2075` instead of the gate's 2050.
That is 25 extra years of fixture coverage, not ten. A year before the fit
window is not evidence about the crate, so freezing it would record the crate's
own guesses as correct. The generator enforces `EXCLUDED` rather than only
documenting it, and Botswana is the single entry there - its extra Boxing Day
and Labour Day holidays are pinned as `Rule::Once` years instead of being
derived from the weekday, so it agrees through 2049 (measured: no difference at
or before 2049, six between 2054 and 2071) and would have had those six frozen
as correct. `EXCLUDED` is not a catalogue of the gate's 38 failing calendars;
those are listed below, and none of them is a corpus entry. The French family is *not*
excluded, because it now passes; the earlier plan to leave it out would have
frozen a known-wrong answer.

**The README did not describe the behaviour change.** `CONTRIBUTING.md` requires
it. The Scope section now records the `Until` lower-bound trap with Estonia as
the worked example, and states that a year before a country's reference
`start_year` is best-effort rather than verified.

## Second review round

Review found two more defects in the corpus, both fixed here.

**The Republic of the Congo was gate-clean and left out with no reason given.**
It was not the only one - 44 gate-clean calendars are still absent from the
corpus, and five more are known differences the gate records rather than defects
- but it was the one this round had just fixed and re-verified, so
leaving it out meant the fix had no regression guard in the data. Its reference
start year is 1994 and it agrees on every date from 1994 to 2075 (0 differences),
including the Labour Day that begins in 1994, so it is now in `COUNTRIES` rather
than `EXCLUDED`. The corpus is 31 calendars and 62,481 rows: 34,691 required,
9,190 weekend-allowed and 18,600 absent.

The corpus does not guard that bound, though, and measuring it showed why. Its
window starts at the country's `start_year`, which for Congo is 1994 - exactly
the year the leak stops - so reverting the `Since` leaves the corpus test green.
The boundary is guarded by `historical_bounds.rs` instead, which now pairs 1993
with 1994 for Congo and fails when the `Since` is removed. `closest.tsv` could
not catch it either, since its probes start in 2020. The remaining 44 gate-clean
calendars are the backlog, and the generator's docstring now says so rather than
implying the corpus is every gate-clean calendar.

**The README described the corpus window wrongly.** It said the corpus starts at
1900 rather than at each country's `start_year`, and that this was the one place
it differs from the gate. The generator actually uses
`max(country_start, EARLIEST)` - the same floor as the gate - so the two agree
on where to begin and differ only in where they end: `LAST = 2075` against the
gate's 2050. The README now says that.

## Verification

* `verify_holidays.py` reports `ok` for all 25, and the full run drops from 63 to
  38 undocumented calendars with no new failures.
* `cargo test -p holidays-rs --all-targets` passes, including the
  `holidays_oracle.tsv` and `closest.tsv` fixtures that CI enforces and the new
  `historical_bounds.rs`.
* The claim in the first cut of this section - "every date in the changed tables
  was re-checked against `holidays==0.108` for 1945-2000: 0 mismatches across 13
  calendars" - was wrong, and this round measured why. Comparing the *whole*
  calendar rather than the dates this round edited, those 13 calendars differed
  on 1,870 dates over 1945-2000 at `712a72e`, 606 of them inside the reference's
  own window. The 0 was true only of the entries that had been edited; it said
  nothing about the calendars around them.
* Measured correctly, the French family was still wrong inside the window:
  1 May, 11 November and the abolition days were ungated `Fixed` rules, so each
  was reported from 1900 while the reference starts 1 May in 1919 and 11
  November in 1922. After bounding them the nine French entities show **0
  weekday mismatches over 1945-2000** and all nine pass the gate, which is what
  lets them enter the corpus at all. Repeating the whole-calendar comparison
  after the fix gives 1,139 differing dates for the same 13 calendars, all of
  them before the reference's `start_year` - that is, none inside the verified
  window.
* Years before a country's reference `start_year` remain unverified by design,
  and the crate still answers them from its rules. Estonia reports 322 dates and
  Lesotho 510 before their reference start years (1991 and 1996). That is the
  documented best-effort behaviour, not a defect this round fixed, and no claim
  here should be read as covering those years.
* `cargo fmt --all -- --check` and `cargo clippy -p holidays-rs --all-targets --
  -D warnings` are clean. No `#[allow]` was added.

## What is left

The 38 remaining calendars are not the same bug. Each one is a table that applies
today's rule to the whole of its history, so the divergence runs from the
reference's `start_year` to about 1991: Finland reports 1509 differences, Italy
and the United States 527 each, Poland 320. Fixing them means adding the
historical layers the reference models (Italy before and after 1977, Poland's
pre-1989 list, the United States before the 1971 Uniform Monday Holiday Act) -
a per-country rewrite, not a start-year correction. That is the next round's
work, and the gate output is the specification for it.

Note that this class is visible in the same place the `Until` bug was: the
reference answers nothing before `start_year`, so the crate answers from its
rules alone. Those years are best-effort and should stay unverified rather than
be "fixed" by pushing today's rule further back.

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
