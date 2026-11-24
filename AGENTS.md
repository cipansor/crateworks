# AGENTS.md

Instructions for anyone, human or agent, working in this repository.

## What this is

`crateworks` is a Cargo workspace of small, single-purpose Rust crates. Each one
fills a gap where npm or PyPI has a mature library and Rust does not, and each is
published to crates.io independently. There is no umbrella crate.

Crates are grouped by domain in the root `README.md`. `CONTRIBUTING.md` states
the rules a contribution must keep — read it before changing behaviour.
`PLAN.md` records the build history and per-crate scope. It covers all 28
crates now, but a crate's own `README.md` remains the current statement of what
it covers, and `PLAN.md` summarises it.

## Commands

Everything is checked by CI (`.github/workflows/ci.yml`) and must pass locally:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Single crate, which is much faster while iterating:

```sh
cargo test -p adhan
cargo clippy -p adhan --all-targets -- -D warnings
```

Publishing order and readiness, without uploading anything:

```sh
python3 scripts/publish.py
```

Uploading needs a crates.io token; see the "Authenticating" section of
`scripts/README.md`. Run the dry run first, then upload with `--publish` (it
asks for confirmation; `--yes` skips the prompt for CI).

## Versioning and what gets released

**Each crate carries its own version, and a release publishes only the crates
that changed.** This is the rule most easily got wrong, and getting it wrong is
expensive: a version number on crates.io cannot be deleted, so releasing a crate
nobody changed publishes a version nobody asked for, permanently.

The crates used to share one version through `[workspace.package] version`, and
every crate said `version.workspace = true`. That is why every release published
all of them, and why the number only ever moved in the last place (`0.1.8` ->
`0.1.9`, a patch bump for everything, even a change that added a function). That
shared version is gone. A crate's version lives in its own `Cargo.toml`:

```toml
[package]
name = "stdnum-rs"
version = "0.1.9"
```

Do not put `version` back in `[workspace.package]` and do not use
`version.workspace = true` in a crate. One bump must touch one manifest.

**Before releasing, ask which crates are due:**

```sh
python3 scripts/release_plan.py
```

It reports a crate only when it has a release-worthy change since its last tag: a
commit that touched its code (documentation, tests, CI, housekeeping and a
`Cargo.toml`-only edit do not count), an uncommitted change, a manifest bumped
past its last tag, or a workspace dependency that released a version outside the
crate's `^` requirement. A crate with none of those is not a release.

**Bump one crate at a time, with an explicit level:**

```sh
python3 scripts/bump.py stdnum-rs patch
```

`bump.py` refuses a crate with no change since its last release unless `--force`
is given. It edits only that crate's manifest. When the crate is `xmlkit` or
`pdfa` - the two the workspace depends on - it also updates the
`[workspace.dependencies]` requirement and names the dependents that now need a
release.

**Choose the level by Cargo's compatibility rule, not by habit.** Below `1.0`
the leading non-zero field is the boundary, so `^0.1.9` accepts `0.1.10` but not
`0.2.0`:

| Level | `0.1.9` becomes | When |
| --- | --- | --- |
| `patch` | `0.1.10` | A compatible fix. The only bump a dependent is guaranteed to accept |
| `minor` | `0.2.0` | New functionality, **or any breaking change below 1.0** |
| `major` | `1.0.0` | The deliberate move to a stable API. Not something a routine change earns |

A change that adds a function is a **minor** bump (`0.1.9` -> `0.2.0`), not a
patch. Reaching for `patch` by reflex is how a crate stays on `0.1.x` forever.
`publish.py` holds this arithmetic in `next_version`; do not compute a version by
hand.

**Then release each crate on its own.** Commit, tag `<crate>-v<version>`, push
that one tag, and let the workflow publish that one crate. Repeat for each crate
the plan named. Never tag a crate the plan did not name.

A release is a tag naming one crate, `<crate>-v<version>`, for example
`xmlkit-v0.1.1`. `.github/workflows/release.yml` runs on `*-v*`, checks that the
tag names a real crate at the version in its manifest, publishes just that crate
in dependency order, and needs the `CARGO_REGISTRY_TOKEN` repository secret. It
can also be started by hand, with the tag as input, to finish a partial release;
a version already on crates.io is skipped, not failed.

There is deliberately no bare `v<version>` tag. The crates version
independently, so one version cannot describe all of them, and reading `v0.1.1`
as "release everything" is what would publish every crate nobody asked for -
each of which crates.io then rejects because the version already exists.
`publish.py` refuses a bare version tag rather than guessing. See the "Releasing"
section of `scripts/README.md`.

**Push tags one at a time.** A single `git push` carrying several tags starts the
workflow once, not once per tag: one push is one event, and the run is named after
one of the tags. `git push origin --tags` with the seventeen tags that existed
then therefore produced a single run and sixteen releases that silently never
appeared. Push each tag on its own, or push one and start the rest from the
Actions tab (`workflow_dispatch`, which takes the tag as an input). A re-run is
safe: a version already on crates.io is reported and skipped rather than failing.

**A crate published before the workflow existed still needs its tag.** The tag is
what the release page, the notes range and a reader's trust hang on, so a crate
that is on crates.io with no tag is not finished. Tag the commit the published
version came from, not `HEAD`: the tarball on crates.io records it in
`.cargo_vcs_info.json` as `sha1`, and the manifest there must say the published
version or `--check-tag` rejects it. Pushing that tag re-runs the release for a
version that already exists, which the workflow reports and skips, and it creates
the missing GitHub release.

Anything that turns a git remote into a URL must strip the userinfo. This
repository pushes with `https://x-access-token:$GITHUB_TOKEN@github.com/...`, so
`git remote get-url origin` returns a live token, and `release_notes.py` publishes
what it returns on the release page. `release_notes.py` keeps only the scheme,
host and path for that reason; a new script that prints a remote URL needs the
same treatment. The leak is invisible in CI, where `actions/checkout` keeps the
token in git config headers instead of the remote URL, so it only shows up when
the script is run locally against a push remote.

Cargo lives at `$HOME/.cargo/bin` in this environment. `rust-toolchain.toml`
pins `stable`; edition is 2024 and `rust-version` is 1.88.

`rust-version` is a **floor**, not the toolchain CI builds with. CI runs two
jobs: one on current stable, one that installs 1.88 and runs `cargo check
--workspace --all-targets`, clippy with `-D warnings`, and the doctests. The
second job exists because the floor was once wrong for months: it read 1.85 while
`pdfa`, `iso20022-rs` and `soap-client` used let-chains, which need 1.88. Nothing
caught it because no job ever compiled against the declared minimum. If you raise
the floor deliberately, raise it in `Cargo.toml` and in the `msrv` job's
`toolchain:` input together, or the job will pass while the claim is false.

## Non-negotiable standards

A change that breaks one of these is not mergeable, however good it is otherwise.

- License is **Apache-2.0 only**. Every crate ships `LICENSE` and `NOTICE`.
- **No dependency beyond the standard library** unless it is genuinely
  necessary. `xmlkit` is the one crate other crates may depend on; it is part of
  this workspace. No network access anywhere.
- **Failures carry a dedicated error type with a `Display` implementation**,
  never a bare `String`. Most crates use an enum of variants; `xmlkit` uses a
  struct carrying a byte offset and a message, because where the parse failed is
  the useful part. Crates whose functions cannot fail (`adhan`, `arabic`,
  `holidays-rs`, `object-hash`) have no error type at all, deliberately.
- **`unsafe` is forbidden** at the workspace lint level. Do not add an `allow`.
- **No `#[allow(dead_code)]`.** If something is unused, delete it. More
  generally, do not silence a lint to make it pass: fix the cause. The only
  allows in the repository are five narrowly scoped clippy exceptions
  (`too_many_arguments` on functions mirroring a reference's parameter list,
  `similar_names` in tests); treat that as the ceiling, not a precedent.
- Every crate has a **doctest on its crate root** (`# Example` in `src/lib.rs`)
  and a **`tests/readme.rs`** that executes the README example. Both must exist;
  the README test is what stops the documentation drifting from the API.
- `-D warnings` means a clippy warning fails the build. The workspace runs
  `clippy::all` plus `missing_debug_implementations`.

## How correctness is established here

This is the part that is easy to get wrong.

**Never invent an expectation. Take it from a reference implementation.** Where a
crate reproduces behaviour another ecosystem already defines, install that
implementation, ask it for the answer, and record the answer as a fixture. The
oracles used so far: `python-stdnum` (`eu-vat`), `pyiso8583` (`iso8583-msg`),
`arabic-reshaper` (`arabic`), the official `factur-x` package, the JavaScript
`adhan` package, `object-hash` npm v3, Python `javaproperties`, and the
`container-validator` npm package (`iso6346`). An oracle is
reference material for development; it is **never** a dependency of a crate and
is never installed by CI.

**When a port disagrees with a fixture, read the oracle source before "fixing"
the port.** Most failures turn out to be a wrong expectation, not a bug in the
crate. Diff them.

**Pin the oracle's version, or its next release becomes your defect.** A
differential gate compares the crate against whatever the oracle is today, so a
release that moves the reference's own data reports as a crate defect. Running
`verify_holidays.py` against `holidays` 0.109 failed four countries
(Martinique, Jamaica, Namibia, Hungary) that the round had not touched, and the
same four failed on a clean checkout with the change stashed - the reference had
moved, not the crate. The version that reconciles the crate is 0.108; 0.107
fails two others (Botswana, Namibia). Read a new failure as "the reference
moved", diff the two oracle releases before editing a table, and record the
version the gate is pinned to beside the gate (`pip install 'holidays==0.108'`
in `verify_holidays.py` and `research/tools/README.md`).

**Hand-picked cases are not enough when the output must match a reference.**
Generate a few thousand random inputs, run them through the oracle, commit the
result as a corpus under `crates/<name>/tests/fixtures/`, and **commit the
generator too**, under `scripts/fuzz/<name>/`. A fixture without its generator
cannot be rebuilt or extended. Two of the three corpora in the repository found a
real divergence the curated cases had passed straight over; the third found
nothing, which is also worth having, because it is what turns "the cases we
thought of are right" into "the cases we checked agree".

**A generator is code, and its bugs look exactly like crate bugs.** Before
concluding the crate is wrong, print the failing case and check that the fixture
means what you think it means. This has cost two separate debugging rounds.

**A fixed fit window back-projects every introduction year to its start.** The
generator originally fitted 2000-2050 for every country. That is invisible over
the fitted window and wrong the moment a caller asks for an earlier year: a
holiday introduced in 1995 is fitted as if it had always existed, and the fit
absorbs the mismatch by shifting the base. The Isle of Man answered Tynwald Day
on 6 July because the fitter hid the real 5 July base behind an `ObservedShift`,
and South Africa answered Human Rights Day for 1990, five years before the
holiday existed. The window must start at the reference's own `start_year`
(`getattr(CountryHolidays, "start_year")`), so the introduction year and every
`ObservedShift.from` are fitted where they actually are. Then check the result
over that same range, not just from 2000: `verify_holidays.py` now defaults to
`[start_year(code), 2050]`, which is what surfaced the defect - the modern
window was clean throughout. The trap generalises: any gate whose window is
narrower than the data it checks can only ever confirm what it already fitted.

**A large shift table is a wrong base wearing a disguise.** `ObservedShift` is
the crate's rule for a holiday moved to a nearby weekday, so a shift is days,
not months. A candidate whose table needs a shift beyond a week is not an
observance rule but a different date hidden under the same name: South Africa's
Y2K Changeover is 31 December 1999 *and* 2 January 2000, and the fitter accepted
31 December with a 363-day shift for the 2000 weekday rather than split the two
dates. `fit_shift` now rejects any shift beyond seven days. The same code pooled
a name's observed dates across every year, so a substitute near a year boundary
was attributed to the wrong base - the 3 January 2000 substitute for the 2
January date was three days from 31 December 1999 too. Attribute each observed
date to its nearest base once per name (`attribute_observed`), not once per rule
being fitted.

**A run that starts at `FIRST` is not a run that starts at the beginning of
time.** When the reference genuinely starts later than the floor (South Africa
1911, Cayman Islands 2006), `FIRST` is a real year. Rendering its runs ungated,
or as an `Until` that leaves the lower end open, invents the holiday in every
earlier year - the defect that a historical gate sees and a 2000-onward gate
cannot. `rust_rule` and `rule_applies` gate a partial run at `FIRST` unless
`FIRST` is the floor itself, and the two must keep mirroring each other: the
Rust side decides what ships, the Python side decides what the held-out check
believes, and a divergence between them makes the check certify the wrong
thing.

**A fit that is right for 51 years can still be a table, and the date check
cannot see it.** `derive_holidays.py` reproduces the reference exactly over the
fitted window and then re-resolves the fit over 2051-2075, refusing a table that
disagrees there. But a holiday that only ever falls on a Sunday has no weekday
date to be missing, so a fit that silently *drops* it after 2050 - a stack of
per-year `Between` rules, which is a table, not a rule - passes the date check.
The tool therefore also rejects a fit that does not produce, in *each* held-out
year, a holiday the reference reports in every one of them (`vanished_names`).
The check is per year, not pooled: a name produced in one held-out year and not
the next is exactly the partial coverage it exists to catch, so a set union
across the years would defeat it. Madagascar's Mother's Day is the case that
found it: the reference keeps the last Sunday of May, *plus a week when that
Sunday is Pentecost*, which no `Rule` variant expresses, so a 51-year `Between`
stack was the only fit - and it was wrong after 2050. Madagascar is not shipped
for the same reason Antarctica is not: the rule is inexpressible, and a
knowingly-wrong date is worse than no calendar.

**A one-year run of a movable shape is not `Since`.** `rust_rule` renders a run
that ends at `LAST` as `Rule::Since`, which fires from its year onward. A run of
exactly one year is the exception: `Once` for a fixed date, a one-year `Between`
for anything else. Rendering a movable one-year run as `Since` invented a date
in every later year - Madagascar's `easter` offset 56 at 2050 did exactly that.
`rule_applies`, the tool's model of what the Rust rule fires, has to reproduce
that branch order or the held-out check tests the wrong thing.

**A weekend move is not always forward, and not always one day.** The
`ObservedShift` shift table is keyed on the base's weekday, so a country may move
a Saturday holiday *back* to the preceding Friday (`(6, -1)`, the Solomon
Islands), two days forward past a Sunday (`(0, 1), (6, 2)`, Gibraltar and
Bermuda) or leave the pair of Christmas and Boxing Day on a shared two-day move
so they never collide. A fitter that assumes "a weekend holiday goes to the next
Monday" fits those countries with a base a day or two off and then hides the
error behind a larger table; when a fit for a weekend-moving country disagrees,
read the reference's own shift for that date rather than the shape you expect.
The filter itself is cheap to re-run and its refusals are stable, so the cheapest
way to grow the crate is to point `derive_holidays.py` at every uncovered country
again after any generator change and keep exactly what it accepts.

**A weekend shift needs a corpus, not a spot test.** A hand-written case names
one year's date - "Bermuda Day is 22 May 2026" - and that year happens to move the
holiday the way the test expects, so the test passes whatever the shift does in
the years it does not name. Changing Bermuda's Saturday New Year's shift from
`(0, 1), (6, 2)` to `(0, 1), (6, 1)` leaves every hand-picked case green while the
calendar reports the wrong day in every year the base falls on a Saturday. The
corpus at `crates/holidays-rs/tests/fixtures/holidays_oracle.tsv`, built by
`scripts/fuzz/holidays-rs/gen.py`, is what catches it: it lists the reference's
answer for the whole window, so a shift that diverges anywhere fails. `verify_holidays.py`
is the same check but live - it needs the reference installed and CI does not run
it - so a calendar whose behaviour is not frozen as a committed fixture has no
regression coverage in CI at all.

## Known traps
- **A memory test with a wide alphabet cannot tell snapshots from a matrix.**
  With `n` distinct characters, `s == n`, so `s * (n + 1)` snapshot cells and an
  `n * n` matrix are the same size and no ceiling separates them. The test in
  `crates/fuzzy-score/tests/memory.rs` therefore uses a two-character alphabet
  with one adjacent swap: 64 KiB of snapshots against a 128 MiB matrix at 4,096
  characters. Verified by reintroducing the matrix, which peaks at 36 MiB and
  fails the 16 MiB ceiling. A wide-alphabet version of the same test passed with
  a matrix restored.
- **`VmHWM` is a per-process maximum, so a delta around one call measures
  nothing.** Any earlier larger allocation leaves the high-water mark above the
  current call, so `after - before` reads zero and the assertion is vacuous.
  `clear_refs` resets `VmHWM` but not `VmPeak`, and a counting allocator needs
  `unsafe`, which this workspace forbids. The measurement has to run in a fresh
  child process that reads `VmHWM` once. `VmHWM` also only counts pages that
  were written: a `vec![0; n]` that is never touched showed +4 KiB for 64 MiB, so
  a case whose rows are never written measures as free.
- **A test that needs `/proc` must skip, not fail, elsewhere.** Reading
  `VmHWM` panics on macOS and Windows. Guard the read and return early, so the
  crate's test suite still passes on a platform that cannot measure; otherwise a
  portable crate acquires a test that only passes on Linux.
- **Replacing a fixture directory deletes whatever else lives in it.** A
  maintainer's notes beside the documents are not generator output, and
  `rmtree` plus rename removes them on every successful run. Either publish only
  files the generator owns, or write a marker into each directory the generator
  makes and refuse to remove one without it. Content-named corpus directories
  (`profile/<digest>/`) plus an atomically replaced index give both: the index is
  the only fixed path, and a stale corpus is removed only after the index stops
  naming it and only if it carries the marker.
- **A staged write is not atomic if the index and the payload are published
  separately.** `verify_facturx_level.py` once wrote the new table, then replaced
  the document directory, so an interrupt between the two left a table naming
  documents that did not exist and the replay failed. Publish the payload first
  and switch the index last, so before the switch the old pair is intact and
  after it the new pair is complete.

- **A holiday table fitted from 2000 back-projects the year-2000 rules.** Two
  separate defects hid behind that: `ObservedShift { from: 2000 }` on a country
  whose reference starts substituting earlier (Seychelles 1994, Namibia 1991,
  Micronesia 1987, Saint Lucia 1979), and a rule that only ever began later left
  ungated (Estonia's Easter feasts, 1994). Both are invisible in a 2000-onwards
  window and both are found by `verify_holidays.py` once it checks from the
  reference `start_year`. Read the reference's `_populate_public_holidays` for
  the `if self._year >= ...` guards rather than trusting the crate's `from`.
- **A holiday can be absent for a middle stretch, so `Since` and `Until` alone
  do not describe it.** France's `Fête de la Victoire` ran 1953-1959, was dropped
  by a 1959 decree, and returned in 1982; the crate carried one ungated `Fixed`.
  Wallis and Futuna and the French Southern Territories inherit the same gap, so
  a single country fix has to be applied to every child entity that copies it.
  `Between` or two rules describe it; a single `Since` gets the early years
  wrong.
- **`Rule::Until` has no lower bound, so it invents the holiday in every earlier
  year.** It sets an inclusive last year and nothing else, and `holidays_in`
  applies no country-level start year, so an `Until`-only form is reported back
  to year zero. Two of these shipped in one round: Estonia's
  `taassünnipäev` (`Until { year: 1993 }`) made Monday 16 November 1987 a
  holiday although the declaration it marks was in 1988, and Lesotho's 2 May
  King's Birthday reached back to 1935, before Moshoeshoe II was born. Both also
  made `is_working_day` return false for those weekdays. `verify_holidays.py`
  missed them because its default window starts at each reference `start_year`,
  so it never looks at the years where the leak happens. When a form has a known
  first year use `Between`, not `Until`, and add a negative case for the year
  before the first valid one - `tests/historical_bounds.rs` is the model.
- **The reference returns nothing before its own `start_year`, and the crate
  does not.** That is the root of the trap above and of the wider historical
  gap: 47 calendars apply today's rule across their whole history because the
  reference stops at 1991 or 1996 and models no earlier regime. Treat any year
  before a country's reference start as best-effort, never as verified, and do
  not "fix" it by copying today's rule further back.

- **A recursive descendant read reports a nested element's data at the wrong
  level.** `iso20022-rs` reads status reasons with `find_all`, which walks the
  whole subtree. That is right for a `pacs.002`, where a group header and a
  transaction never nest a `StsRsnInf` of the same type, but a `pain.002`
  payment instruction *contains* its transactions, each with its own
  `StsRsnInf`, so a recursive read would report a transaction's reason as the
  instruction's. The instruction level uses `read_status_reasons_direct`, which
  stops at the direct children. When a message nests an element that also
  appears at the level above, read the direct children, not the subtree.
- **A newer clippy can fail an untouched crate.** `-D warnings` turns every new
  lint into a build failure, so a green build on one toolchain says nothing about
  the next. A clippy bump to 1.99 produced 17 failures across nine crates, none of
  which had changed: `manual_is_multiple_of` (`x % 2 == 0` becomes
  `x.is_multiple_of(2)`), `chunks_exact_to_as_chunks` (a constant chunk size
  becomes `as_chunks::<64>().0`), and `collapsible_if` (a nested `if let` becomes
  a let-chain). The fix is to satisfy the lint, not to add an `allow`; the repo's
  five existing allows are the ceiling, not a precedent. When a failure lands in
  a crate your branch never touched, check whether it reproduces on `main` before
  reading your own diff: it usually does, and the answer is to fix it in a commit
  of its own so the history says what it was for. `needless_range_loop` on
  clippy 1.100 was exactly that, in `ebics/src/bignum.rs` and `pades/src/rsa.rs`.
- **The lints differ in BOTH directions, so the floor must be linted too.**
  `uninlined_format_args` is warn-by-default in 1.88 but moved to `pedantic` by
  1.99, so `format!("{:?}", x)` is an error on the floor and fine on stable.
  Fixing those five sites in favour of 1.88 keeps both green; an `allow` would
  have papered over it. Lint the floor, not just stable, or the floor's clippy
  stays red forever.
- **A credit in the code must also be in `NOTICE`.** Several crates are
  independent reimplementations of an existing library, and the upstream licence
  requires that library to be named in `NOTICE`. `stdnum-rs` cited
  `python-stdnum` in `src/id/mod.rs` while its `NOTICE` listed nothing, even
  though `eu-vat`, which shares that lineage, did name it. No compiler catches
  this, so `scripts/check_attribution.py` does, in CI. When you add a crate that
  reimplements something, add an entry to `scripts/attribution.toml` (the data
  file the script reads) and the attribution to the crate's `NOTICE` in the same
  change. The marker must be distinctive to the upstream project, usually a
  repository path; a generic marker such as `xmlkit` matches every dependent.
- **Every action must be pinned to a commit SHA.** The cipansor org rejects
  unpinned actions: the job dies at "Set up job" with "all actions must be pinned
  to a full-length commit SHA", before any step runs. Keep the tag in a trailing
  comment, since a bare SHA says nothing: `uses:
  actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4`. An annotated
  tag (such as `Swatinem/rust-cache@v2`) points at a tag object, so dereference
  `git/tags/<sha>` to reach the commit. `scripts/check_actions_pinned.py` enforces
  this, in a `pinned` job of its own as well as in `test`: a job that contains an
  unpinned action cannot run its own check, so the check must live somewhere else
  too.
- **The README guard is a real test, not ceremony.** Adding the `ReadmeDoctests`
  marker to `iso20022-rs` immediately failed: its README said
  `use iso20022-rs::camt::BankStatement;`, and a hyphen is not legal in a Rust
  path, so the documented import could never have compiled. The marker belongs in
  every crate's `src/lib.rs`; a crate without one has an unverified README.
- **`rust-toolchain.toml` outranks the toolchain you select.** `rustup` reads the
  file in the working directory, so `rustup run 1.88.0` or a CI job whose action
  installs 1.88 still compiles with `stable` unless `RUSTUP_TOOLCHAIN` is set
  (that variable beats the file) or the action uses `override: true`. This is why
  the MSRV job sets both: without them the job would pass while testing the wrong
  compiler, which is exactly how a `rust-version` of 1.85 survived in a tree that
  needed 1.88.
- **The toolchain is wiped when the sandbox restarts.** `$HOME/.cargo` and
  `$HOME/.rustup` disappear; `target/` survives. Reinstall rustup, or install to a
  directory that is not under `$HOME` (for example
  `CARGO_HOME=/workspace/.toolchain/cargo RUSTUP_HOME=/workspace/.toolchain/rustup`)
  to keep it across restarts. `pip` packages are wiped too.
- **A toolchain that survives the restart is still off `PATH`.** A
  `CARGO_HOME=/workspace/.toolchain/cargo` install persists, but nothing adds its
  `bin/` to the shell, so `cargo: command not found` means "find it", not
  "reinstall it": `export PATH=/workspace/.toolchain/cargo/bin:$PATH
  CARGO_HOME=/workspace/.toolchain/cargo` before the first `cargo`. The same
  restart drops the credential in `$HOME/.openhands/git-credentials`, so a push
  that worked earlier fails with `403` until a token is supplied again.
- **`cargo clippy` prints nothing on a cached build.** If a lint seems to have
  vanished, `touch crates/*/src/lib.rs` (or `find crates -name '*.rs' -exec touch {} +`)
  to force a rebuild before believing the result.
- **A generated float that equals a named maths constant must be written by
  name.** `runits` emits the logarithm base of `Np` as `std::f64::consts::E`
  rather than `2.718281828459045`, because clippy's `approx_constant` fires on
  the literal and the crate forbids an `allow`. `gen_units.py`'s `main` detects a
  base equal to `e` and writes the constant by name. Any new generated float
  that equals a named maths constant needs the same treatment; the data table
  itself is allowed to hold the literal, but a hand-written line is not.
- **A sum of two dimensioned logarithms has no logarithm to return.** `pint`
  computes `0 dBm + 1 dBm` by multiplying the two reference quantities and
  returning a *squared* watt, a quantity neither operand named, so `runits`
  refuses it (`ArithmeticError::LogarithmicUnit`) rather than invent a number. A
  sum of two dimensionless logarithms is well defined and allowed: `3 dB + 3 dB`
  is `6 dB` and `3 dB + 1 Np` is `11.686 dB`. When extending log arithmetic,
  check which side of that line the reference falls on before copying it.
- **A logarithmic conversion must stay in log space.** `runits` converts between
  two logarithmic scales with `divisor * (log(value) * ln(from.base) +
  ln(from.reference) - ln(to.reference)) / ln(to.base)` rather than by
  exponentiating the source and taking the logarithm of the target. A
  logarithmic scale names a value far outside `f64`'s range - `400 Np` is about
  `3474 dB`, whose linear equivalent is `e^800` - so the round trip through the
  base quantity overflows to infinity (or underflows to zero, which then reads as
  `NotPositive`). The oracle cannot cover that range either: `pint` overflows on
  its own linear intermediate above about `150 Np`, which is why the corpus stops
  there and the out-of-range cases live in `tests/pint.rs` instead.
- **A generator's random stream is shared, so new draws go at the end.** The
  fuzz generators seed `random` once and every corpus draws from that stream in
  file order. Adding a `piece()` call before an existing corpus regenerates it
  and every later one, which looks like a crate bug in the diff. Append the new
  block last, then confirm the untouched fixtures are byte-identical.
- **A fixture normaliser is only as safe as the invariant it assumes.** The
  `javaproperties` generators upper-case `\uXXXX` hex to match Java. That rewrite
  cannot tell a `\u` the reference emitted from one the caller typed literally, so
  it is only sound when every input is guaranteed to contain no literal `\u` with
  non-digit hex - where upper-casing is a no-op. Guard the *input*, not the
  output: `_check_literal_escapes` now runs inside `piece()` and `dumps_row` and
  over every curated list, because a check on the curated lists alone was bypassed
  by the random corpora.
  Trying to infer which `\u` is real from the output does not work. Skipping a
  `\u` that follows an escaped backslash looks correct for keys and values, where
  the reference doubles a literal backslash, but a comment never doubles one: a
  caller's `\` before an escaped emoji renders as `\\uD83D\uDE00`, so parity reads
  the second backslash as the escape's and leaves `\uD83D` lower-case. Three
  fixtures changed and the diff showed it. When a normaliser rewrites text, write
  down the invariant it relies on and assert it over the inputs.
- **A three-valued reference argument is `Option<bool>`, not `bool`.**
  `javaproperties`' `to_comment(comment, ensure_ascii=None)` has three positions:
  `None` escapes only what ISO-8859-1 cannot hold, `Some(true)` everything above
  ASCII, `Some(false)` nothing. Collapsing it to a `bool` makes the default
  unrepresentable and silently changes what existing callers write. The fixture
  column spells the mode out (`none` / `1` / `0`) so a two-valued test cannot
  pass with the wrong default.
- **A stream writer must encode what the reader decodes, and the escaping
  default is what hides a violation.** Java's `store(OutputStream, ...)` encodes
  ISO-8859-1, and `parse` decodes it. `store_bytes` was `store().into_bytes()`,
  which is UTF-8. Nothing caught it for a release, because the argument for
  safety only looked at entries: `ensure_ascii: true` escapes those to ASCII, so
  the two encodings agree. The comment is a *separate* switch whose default
  (`ensure_ascii_comments: None`) passes Latin-1 through, so the default call
  `store_bytes(Some("café"))` emitted `#caf\xC3\xA9` where Java emits `#caf\xE9`,
  and a comment-preserving ISO-8859-1 reader saw `cafÃ©`. Two lessons: a document
  has more than one escaping switch, so "the default escapes everything" is only
  true of the field you were thinking about; and when a function's name or doc
  promises an encoding, test the *bytes*, not the round trip through your own
  reader, which agrees with itself whatever it encodes. `store_bytes` and
  `store_to` stay infallible because the default options are exactly the ones
  that keep every character inside ISO-8859-1; `store_to_with` takes arbitrary
  options, so a character above `U+00FF` is `Error::NotLatin1` rather than a
  silent transcode.
- **`cargo doc` is a separate gate, and the three commands in this file do not
  cover it.** CI runs `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`,
  which fails on a broken intra-doc link (`rustdoc::broken_intra_doc_links`).
  Neither `cargo clippy --all-targets` nor `cargo test --all-targets` resolves
  links in doc comments, so a doc-only change can be fmt-clean, clippy-clean and
  test-green locally and still fail CI. A link to an item in another module needs
  its path (`[`crate::parse`]`, not `[`parse`]`), because the item is not in scope
  in the module holding the doc comment. Run the doc command before pushing
  anything that touches documentation.
- **A written HL7 value must be escaped, and a reader cannot tell when it was
  not.** `mllp`'s ACK builders wrote MSA-3 raw, so a failure text holding `|`
  ended the field: hl7apy read `MSA-3` as `"bad "` and the rest of the text
  became phantom fields. The reply parsed cleanly, which is why nothing caught
  it - the gate's MSA-3 was `"ok"`, and no case carried a delimiter. Any value
  an `ACK` echoes from the peer (MSH-3..MSH-6, MSH-10, the failure text, the
  ERR location) is attacker-influenced, so writing it unescaped is a
  correctness and injection bug. `escape` inverts the resolution
  `Field::unescape` does; use it for every primitive value a builder writes.
  A **composite** field is the trap: MSH-3..MSH-6 are HD and MSH-12 is a VID, so
  escaping the whole field turns its component separators into data and
  `LAB^1.2.3^ISO` becomes one component. Keep parsed composites as ER7 and
  re-emit them; only decode-then-escape if you have a structure to rejoin. Do
  not put `escape` on `Field`: a parsed field is already ER7, so escaping it
  doubles every escape it holds. The mirror trap is a **locally generated**
  value treated as ER7: the `2.5.1` fallback version is plain text, so
  transcoding it writes `2^5^1` when the source's component separator is `.`,
  and a period field separator splits it into three fields. Decide per value
  whether it came from parsing or from `format!`. Keep an identity fast path in
  any transcoder: answering in the source's own delimiters must never refuse,
  because the parser already accepted that text and copy cannot introduce a
  delimiter. A transcoder without that path refuses legal messages, and the
  refusal message then tells the caller to do what it already did. A raw
  delimiter inside an **opaque** escape body blocks any *conversion* in either
  direction - a reader splits before it resolves escapes, so such a body has
  source structure no other delimiter set can express - but only when a
  delimiter *changes role*. Judge that by the role each character plays, not by
  whether it appears in both sets: changing only the field separator leaves a
  body holding the component separator byte-identical, and refusing it rejects a
  header the reply can carry exactly. A validation that must hold regardless of
  the reply - a raw field separator from a hand-built context - has to run
  before any branch on the reply's delimiters, or the caller can make an invalid
  value acceptable by choosing them. A body can never hold the component
  separator, because splitting happens first: `LAB%ZID^1%` is two components with
  stranded markers, not one escape, so rewriting those markers changes the
  identity a peer decodes. Read the structure before rewriting it, and make the
  gate's fixtures agree - a corpus with no opaque body holding a delimiter that
  keeps its role cannot tell a role rule from a presence rule, and both mutations
  pass. A boundary is not visible in decoded text either: escaping `&` as `\T\`
  decodes back to `&`, so a lost subcomponent boundary looks correct unless the
  check counts the split a reader performs - and a count is not enough either: one
  literal separator swapping places with one structural separator keeps the count
  and the decoded text, so compare the per-subcomponent split, not the number of
  parts. Fix the check once and apply it to every path: the converted path kept the
  weaker comparison for a round after the field-only path was fixed, and a mutation
  that only damages a two-separator reply went unseen. Count *replies*, not findings:
  a checker that returns one per printed finding makes `len(corpus) - problems` go
  negative, and the summary then understates what passed. Keep one unit per total:
  field differences are counted per message and reply checks per reply, so summing
  `mismatches` with them mixes two units and reports findings as failures, and summing
  the per-check counters reports one message failing five checks as five failures. The
  headline is distinct messages that failed, with the per-check counts printed beside
  it, never added into it. When checking
  this against `hl7apy`, compute the expectation from the reply's own MSH-1 and
  MSH-2: a message using `#` and `*` leaves `|` and `^` alone, so a hardcoded
  default-delimiter expectation fails on a case that is correct.
- **A gate that shells out must force its build directory, not default it.**
  `verify_mllp.py` compiled a driver into a tempdir and then ran it from that
  path, but set `CARGO_TARGET_DIR` with `setdefault`. An ambient value - left
  over from an earlier release build - sent the binary elsewhere and the run
  died with `FileNotFoundError` on the binary, which reads like a compile
  failure and is not one. Assign the variable. And when an oracle rejects its
  input outright, report it as a finding: hl7apy raises `UnsupportedVersion` on
  a reply whose MSH-12 was corrupted, and an uncaught exception printed a
  traceback instead of the one line naming the bad field.
- **A corpus case can be malformed and look like a crate bug.** A raw `~` in
  MSH-5 made it a repeating field, which MSH-5 is not; the reference then
  reported the value differently and the crate looked wrong. Escape delimiters
  in the fixture when the point is that a value *holds* one, or the case tests
  the parser's repetition split instead.
- **A value the builder generates is not a value the source sent.** MSH-9 (`ACK`)
  and the `2.5.1` fallback version are written locally, so they are plain text
  under the reply's delimiters and must be escaped, never transcoded. Sending them
  through the source-ER7 path judged `ACK` against the *incoming* field separator:
  a source using `K` made it look like a value holding a raw field separator, and
  the builder refused a reply the outgoing delimiters carry unchanged.
- **`xmlkit` name matching.** `Element::child` and `children_named` match the
  full name including the prefix, so `child(el, "IssueDateTime")` does **not**
  find `ram:IssueDateTime`. Match on the local name. `Element::text()`
  concatenates only direct text children, so a value nested in a sub-element
  comes back empty.
- **A `Forwarded` value is a token unless it is quoted, and the two header
  families are in opposite order.** In RFC 7239 a bare value may only contain
  `tchar`, so `host=example.com:8443` and `for=[2001:db8::1]` are syntax errors
  that need quotes (`host="example.com:8443"`); the reference `forwarded-parse`
  rejects them and `proxy-addr`'s parser must agree rather than helpfully
  splitting on the colon. The order trap is worse because it fails silently:
  RFC 7239 appends each hop, so the client is **first**, while
  `X-Forwarded-For` is prepended and the crate's `forwarded` returns peer
  first. `parse_forwarded` therefore keeps header order and does not reverse.
  Also: the reference's `mustUnescape` flag is set once for the whole header and
  never cleared, so a backslash in one element decodes later elements' values
  too; the port keeps that quirk, since matching the reference is the point.
- **A `Forwarded` corpus must be weighted towards valid headers.** Random text
  over the reference's character classes is almost always a syntax error, so an
  even split produced 848 accepted cases out of 6,013 and left the accept path
  thin. The generator now builds whole `name=value` parameters from a token
  pool and a quoted pool, and a test asserts each outcome has a floor.
- **A `Forwarded` node port is independent of the node name.** RFC 7239 gives
  `node = nodename [ ":" node-port ]` with
  `nodename = IPv4address / "[" IPv6address "]" / "unknown" / obfnode` and
  `node-port = port / obfport`, so `unknown:443`, `_hidden:443`,
  `192.0.2.43:_edge` and `_hidden:_p` are all legal. Typing the port as
  `Option<u16>` on the address variant made every one of those unparseable,
  which silently erased a node from a header the outer grammar had already
  accepted - the worst kind of gap, because `parse_forwarded` succeeded and only
  the typed layer lost the value. The port is now a `Port` enum carried by every
  variant.
- **`obfnode` is narrower than `token`.** The outer grammar accepts any token as
  a value, but RFC 7239's `obfnode` and `obfport` are
  `"_" 1*( ALPHA / DIGIT / "." / "_" / "-" )`, so `_edge+2`, `_a!b` and `_a~b`
  are tokens that are not obfuscated names. Validating them with the token
  predicate accepted all three. Likewise `unknown` is an ABNF literal and so
  case-insensitive (`Unknown` is the same identifier), and brackets are reserved
  for IPv6, so `[192.0.2.43]` is not a node.
- **Rust's `u16` parser is looser than `1*5DIGIT`.** `"+443"` parses, and so does
  `"000443"`, so a port validated only by `str::parse::<u16>()` accepts shapes
  RFC 7239 does not allow. Check the digit count and the character set before
  parsing, as `Port::parse` does.
- **A differential fixture must record an oracle disagreement, not hide it.**
  RFC 7230 section 7 makes accepting empty list elements a **MUST** for a
  recipient, and the reference `forwarded-parse` rejects them, so `,,` and a
  trailing `,` are headers `proxy-addr` must read and the reference will not.
  Deleting those rows would drop coverage; asserting agreement would pin the
  crate to the reference's bug. Each such row carries an `empty` marker and the
  reference's verdict for the header with the empty elements removed, and a test
  re-derives that stripping them is what changes the reference's mind, so a
  header broken for a second reason cannot be relabelled into passing. An error
  **index** can only be compared when it has not moved: skipping an element
  shifts every later position, so an index at or after the first removed
  character is meaningless and those rows compare rejection only. An index
  before it is still the index in the original header - ` for=a,,for=b` fails
  at 0 for its leading space either way - so keep it and compare it exactly
  rather than discarding a comparison that still holds.
- **A comma after a dangling `;` is not an empty list element.** `for=1.1.1.1;,`
  opened a parameter and never closed it; treating the comma as an empty element
  merged the following element into the current one, turning two hops into one
  with no error. The tolerance applies only when the element has collected no
  parameters yet.
- **A trailing `,` is an empty element; a trailing `;` is truncation.** Both
  leave the parser with no parameter open, so a single "nothing open, done"
  branch accepted `zz=1.1.1.1;` and silently dropped the element. Track the
  separator that closed the last thing and allow the end only after a comma.
  This one shipped through a green build and was caught by generating a corpus
  from a **different seed** and replaying it; the committed corpus had no such
  shape. Run the oracle against a fresh seed before believing a parser change.
- **A generator's helper is code, and its bugs look like crate bugs.** The
  "strip empty elements" helper first trimmed every surviving segment, which
  repaired a leading space - a separate fault - and relabelled an
  already-agreeing row as a divergence. It also joined with `", "`, adding a
  space that changed the header being measured. Print the rewritten header and
  diff it against the original before trusting a derived expectation.
- **Whitespace around a dropped separator is part of the separator.** The list
  rule is `#element => [ element ] *( OWS "," OWS [ element ] )`, so the space
  in `, for=a` belongs to the comma, not to the element. Stripping an empty
  element but leaving its neighbouring whitespace produces ` for=a`, which is
  broken for a *different* reason (leading whitespace), and a fixture built on
  that rewrite expects a rejection the crate correctly does not give. Drop the
  whitespace with the separator, and only when the adjacent segment is dropped -
  whitespace next to a surviving comma is that comma's OWS and must stay.
- **A "position-free" verdict is not automatically comparable.** `!eof` carries
  no index, so tightening a rejection-only row to compare `!eof` looks like it
  needs no translation - and it is wrong in general. The crate skips a comma
  only at an element boundary, with nothing collected yet; `for=,` faults at the
  comma itself (index 4), while its stripped form `for=` is merely truncated.
  A corpus that compares `!eof` for every stripped row failed on three of five
  unseen seeds for that one shape. The sound condition is that *every* removed
  element was leading, where the crate provably skips exactly those commas.
  Verify a corpus-tooling change with unseen seeds (`SEED=<n> OUT=<path>`), not
  only against the committed fixture - the default fixture passed the wrong rule.
- **A secret can go empty mid-session, and the failure looks like a permissions
  bug.** `GH_WRITE_TOKEN` was populated for the first pushes and later expanded to
  the empty string, so a credential helper wrote `password=` and GitHub answered
  "Invalid username or token"; `GITHUB_TOKEN` still authenticated but is a
  read-only token here and answered 403 "Permission to ... denied". Check
  `${#GH_WRITE_TOKEN}` and probe `https://api.github.com/user` before concluding
  the remote or the helper is wrong.
- **Test a credential helper's output before blaming the token.** A helper that
  takes its secret from `"$1"` sends `password=get`, because git invokes a helper
  as `helper get` - `$1` is the operation name, never the token. The helper was
  never handed a token at all. Quoting is a red herring here: single quotes only
  stop expansion when `$1` sits *inside* the format text (`printf 'password=$1'`
  does print a literal `$1`), and this helper used `%s`, which expands its
  argument correctly - it printed `get` because that is what it was given.
  GitHub answers
  "Invalid username or token", which reads exactly like a dead token, and a valid
  PAT was reported dead twice because of it. The `$1` substitution is in the
  shell, which has no idea git is the caller, so nothing about the code looks
  wrong. Validate the token against the API (`GET /user`, then
  `GET /repos/<owner>/<repo>` for `permissions.push`) and, when the API accepts
  it, suspect the helper. Pass the secret through the environment and echo it
  from a heredoc-written script (`echo "password=${GIT_PUSH_TOKEN}"`, variable set
  for that one command); a `password=` with nothing after it then points at the
  empty-variable trap above rather than at git. Feeding the helper a sample
  `protocol=https`/`host=github.com` on stdin and reading its output takes a
  second and is what separates the two faults: `password=get` is this trap,
  `password=` is an empty secret.
- **A mirrored helper must be diffed against its original, not just run.** The
  Rust test carries its own copy of the generator's split-and-strip transform,
  and both replay the same corpus, so a divergence between the two is invisible
  to the tests: each agrees with the fixture it computes from. The copy trimmed
  trailing whitespace when a header ended with no comma (`for=1.2.3.4 `), because
  a missing neighbour at the end of the list was read as a dropped segment, while
  the generator kept it. The corpus replay passed either way. Dump both
  implementations' output over the whole corpus and `diff` them - that is the
  only check that sees this class of bug.
- **A new npm oracle dependency must regenerate the lockfile.** `package.json`
  gained `forwarded-parse` while `package-lock.json` was untouched, so `npm ci`
  failed with `Missing: forwarded-parse@2.2.0 from lock file` while `npm install`
  worked, because `npm install` silently updates the lock. Run `npm install` and
  commit the lock in the same change; the diff should add only the new package.
- **An empty inline element is still a box.** WeasyPrint gives `<span></span>`
  an empty inline box with its own strut, even with no text in it: `<div>a<span
  style="font-size:24px"></span></div>` raises the line to 24px, and an empty
  `<span>` on its own leaves a line box of height 0 that holds the 16px inline
  box. `cssprint` currently emits no box for an element with no in-flow
  children, so its line stays 16px and the empty inline box is missing. A
  whitespace-only node is different and already handled (see `push_text`): it
  survives as an empty text box, not an empty inline box.
- **A reference-derived fixture must use the same window as the gate.** The
  holiday corpus originally started at each country's own `start_year`, while
  `holidays_oracle.tsv` was generated from year 1. Generating from year 1 for the
  31 calendars in the corpus gives 439,456 differing dates before 1900 - years
  the fit never saw, where the crate answers from its rules and the reference is
  silent or models a different regime. A fixture is only evidence inside the
  window the tables were fitted against; freezing years outside it records the
  crate's guesses as if they were correct. Both sides now floor at
  `EARLIEST = 1900`.
- **That same window means a corpus cannot guard a `start_year` bound.** Adding
  Congo to the corpus looked like it would protect its 1994 Labour Day, and
  reverting the `Since` to a bare `Fixed` left the corpus test green: the corpus
  begins at Congo's `start_year`, which is the exact year the leak stops, so the
  one year that would differ is outside it. Check a new fixture by breaking the
  behaviour it is supposed to catch. A bound at `start_year` needs
  `historical_bounds.rs`, which pairs the year before with the year itself;
  nothing derived from the reference's own window can see it.
- **A review comment cites a revision, not a file.** The same two comments on
  `holidays-rs/README.md` (lines 167 and 169, "ten years of coverage" and
  "`EXCLUDED` ... a record of known divergences") arrived twice after both were
  fixed. They quote commit `8e24081` verbatim, which is an ancestor of the PR
  head three commits back; at head, 167 says 25 years and 169 is about the
  corpus subset. Checking the cited lines in the working tree showed accurate
  text and no defect, and rewriting it would have broken a correct file. Confirm
  which revision a comment describes before acting: read the cited lines, then
  `git log --oneline <cited>..HEAD -- <path>` to see whether they moved. A
  comment that no longer applies is answered, not edited around.
- **A quoted measurement decays; re-derive it before you repeat it.** The review
  doc for `holidays-rs` carried five figures that no longer matched the tool that
  produced them: Finland "1509" differences (the gate says 323), a divergence
  ending "about 1991" (the latest of the 38 runs to 1999, and only 15 stop by
  1990), Estonia "322" and Lesotho "510" pre-start dates (the gate counts 637 and
  951 over 1900 to the year before), and 1,870 pre-fix differences for the French
  family (2,063, measured at `712a72e` in a worktree). The one figure that
  matched exactly, 606 in-window, was the clue: it showed the metric was right
  and only the total had drifted. When a count supports a claim, record the
  window and the command beside it, and re-run it in the round that repeats the
  claim. A number with no window is not checkable, and a stale one reads exactly
  like a fresh one.
- **"Gate-clean" is two numbers, not one.** `verify_holidays.py` prints `ok` for
  a calendar that agrees and `known-difference` for one whose divergence is
  deliberate and recorded. Counting both as clean overstated the corpus backlog
  by five: 44 calendars are `ok` and absent from the corpus, not 49, and the
  other five are the known differences. Read the verdict, not just the absence
  of `BUG`, before quoting a count.
- **A comparison that only looks at what you changed will report zero.** A
  review claim of "0 mismatches 1945-2000 across 13 calendars" was true of the
  entries that round had edited and false of the calendars containing them.
  Measured over the whole calendar, those 13 calendars differed on 1,870 dates
  in 1945-2000 before the French family was bounded, 606 of them inside the
  reference's own window; after the fix the same comparison gives 1,139, all of
  them before the reference's `start_year` and so outside the verified window.
  The method matters: compare the full output of the thing that changed over a
  stated year range and record that range, not the subset the diff touched. A
  number without its window and its calendar set cannot be checked by the next
  reader, which is how the 0 survived a review.
- **`Rule::Until` alone has no lower bound, and the gate cannot see the leak.**
  It sets a last year only, so a form whose modern shape ends before the
  reference's `start_year` is reported back to year zero. The gate starts at
  `start_year`, which is exactly where the leak stops, so it never fires. Use
  `Between` when the first year is known; `verify_holidays.py` will not catch a
  bare `Until`, and Estonia's `uusaasta` still reports 1950 for a reference that
  begins in 1991.
- **A trailing space only makes a phantom line when it shares its text node with
  the content.** When a trailing collapsible space overflows a line that already
  has an internal break opportunity, WeasyPrint starts a line of its own for the
  space, drops it, and counts it - so the line it left is not last and takes
  `text-align` rather than `text-align-last`. A whitespace-only node never does
  this: the reference drops it from the box the line keeps and leaves that box
  empty, so the line stays last. `cssprint` had the phantom test fire for both;
  the fix is the `!space.lone` guard in `shape_lines`. The two differ only in how
  the space was written, so a curated case for each sits side by side
  (`align_last_trailing_space_overflow`, `align_last_trailing_lone_space_keeps_line`).
- **WeasyPrint gives the `style` attribute id specificity, so an id rule beats
  it.** The spec puts the `style` attribute above every author rule, but
  WeasyPrint's cascade assigns it specificity `(1,0,0)` - equal to a single id -
  and then applies the sheet's rules *after* it, so `#x{width:30px}` overrides
  `style="width:90px"` on the same element. `cssprint` follows the spec and lets
  the attribute win, so a corpus case pairing an inline style with an id selector
  would disagree for a reason that is not a crate bug. Pair the inline style with
  a class selector instead (the attribute still wins there, in both engines) or
  leave the pair out. See the cascade loop in the reference's
  `css/__init__.py`, which sets `layer_order = inf` for the attribute and gives
  it `specificity` `(1,0,0)`.
- **A descendant selector over an inline ancestor needs the chain, not just the
  block parents.** `collect` walks the inline subtree separately from
  `layout_element`, so a selector like `div span` only matches if the inline
  elements opened so far are pushed onto the same ancestor chain the block walk
  builds. Both walks now thread that chain (`Walk::ancestors`, `Collector::chain`).
- **An inline match is only visible through an inherited property.** A box-model
  declaration (`width`, `height`, `margin`, `padding`) is ignored on a
  non-replaced inline by both engines, so a corpus that gives an inline subject
  only those would pass even if the inline ancestry were dropped entirely. A
  rule whose subject may be inline must carry `font-size` (or `display: none`),
  which moves the text, for the match to show up in the box tree. The random
  selector corpus nests spans and gives inline subjects `font-size` for this
  reason; breaking the inline chain now fails 167 random cases, where before it
  failed none.
- **A tag name in a selector must accept what the HTML parser keeps.** The HTML
  parser lowercases and keeps a tag as written, so `café` is a real element and
  `café { … }` must parse. Rejecting a non-ASCII tag because the check used
  `is_ascii_alphanumeric` is a regression the old permissive parser did not have.
  Accept any alphanumeric character (plus `-` and `_`) via `is_alphanumeric`.
- **A compound selector's name ends at every sigil that can follow it.** The
  boundary predicate `ends_name` decides where a tag, `.class` or `#id` stops,
  so it has to stop at `[`, `:` and the sibling combinators `+`/`~` as well as
  the descendant space and `>`. A predicate that stops only at whitespace reads
  `div[a]` as the tag `div[a]` and then fails to parse. The universal `*` is the
  one sigil that is a whole compound of its own: `ends_name` treats it as a
  boundary so `*:first-child` splits, which means `parse_compound` must accept a
  leading `*` (and skip it) before it reads a name, or the earlier universal
  selector tests fail with "cannot read selector `*`".
- **A parenthesized selector argument nests.** `:not(:nth-child(2n+1))` puts a
  `)` inside the `:not()` argument, so `read_parenthesized` has to count depth
  and take the `)` that closes the opening `(`. Finding the first `)` reads the
  argument as `:nth-child(2n+1` and then fails on the leftover `)`. The random
  selector corpus drew this on its first case; the curated cases did not.
- **An attribute selector's name ends at the operator, not only at `=`.** The
  operators `~=`, `|=`, `^=` and `$=` begin with a sigil, so a name scan that
  stops only at `=` or whitespace swallows the sigil into the name (`data-x~`
  then `=v`) and the rule silently never matches. Stop the name at `~ | ^ $ *`
  as well, then read the operator from what follows.
- **A quoted string in a selector is not syntax.** An attribute value may be a
  CSS string holding any character, so `[title="Doe, Jane"]` must not split on
  the comma, `[title="a]b"]` must not close at the quoted `]`, and
  `:not([title=")"])` must not close at the quoted `)`. Every scanner that looks
  for a delimiter - the comment stripper, the rule-brace finder, the
  selector-list splitter, the attribute-close finder and `read_parenthesized` -
  has to step over a quoted string, honouring `\` escapes, rather than read its
  contents as syntax. They now share `string_length`, `find_unquoted` and
  `split_unquoted` for that reason. A quoted value's escapes are decoded
  (`\62` is `b`) before it is compared, as the reference's tokenizer does.
- **A selector engine grows in lockstep with its corpus, or the corpus is a
  rubber stamp.** The `cssprint` selector grammar (attribute selectors, the
  structural pseudo-classes, `:not()`, the sibling combinators) was added
  together with the generator that draws them and the fixtures regenerated, so
  the new grammar is checked against WeasyPrint rather than against a
  hand-written expectation. Extend `scripts/fuzz/cssprint/gen.py` in the same
  change as the grammar, and regenerate the selector fixtures, or the random
  corpus keeps testing only the old shapes.
- **A structural pseudo-class must not re-count siblings per rule.** The
  sibling position and count a `:nth-child()` or a sibling combinator reads are
  computed once by the DOM traversal that already walks the parent
  (`element_children(element).count()`), and threaded down as `Ancestor { element,
  index, total }`. Computing them inside matching - walking the parent's children
  to count and again to locate the subject, for every rule against every element
  - is quadratic on a flat document: a 1000-child list with one `div` rule cost
  ~1.5M sibling visits. The fix took the 4x-scaling ratio from 15.2 to 3.5 on an
  8000-child document. A selector with no structural pseudo-class and no sibling
  combinator reads none of it, so `index`/`total` are free for the caller to
  pass as `0`.
- **An `an+b` coefficient is an `i64` and its arithmetic can overflow.** The
  `:nth-child()` parser accepts a signed 64-bit coefficient, so
  `-n-9223372036854775807` makes `index - b` overflow `i64` (panicking in debug,
  wrapping then panicking on `% -1` in release). Widen the subtraction, remainder
  and division to `i128`, where the difference of two `i64`s is exact in every
  profile.
- **An inline run must carry its nodes' original element-child index.** The run
  of inline content inside a block holds the nodes between the block's
  block-level children, and omits the hidden ones, so a node's position in the
  run is not its position among the element's children. Renumbering from zero
  inside the run makes an inline element read the wrong `index`, and a structural
  pseudo-class then hides or shows the wrong box: with
  `<div><div></div><span>visible</span></div>` and
  `span:first-child { display: none }`, the span is the parent's second element
  child and must stay, but a run that restarts at zero reads it as the first and
  drops its text. A preceding `display: none` sibling triggers the same with no
  block boundary. Carry the index from where the block's `child_info` is built
  (`Vec<Option<(usize, Display)>>`) through the run into the inline walk, rather
  than deriving it from the filtered nodes.
- **CSS normalizes newlines before it tokenizes, so a string scanner must see
  only LF.** A CRLF pair, a lone CR and a form feed each become one LF, and that
  happens before strings are read. A scanner that knows only LF reads a
  backslash-CRLF continuation inside a quoted value as an unescaped line end and
  rejects the whole stylesheet, and a decoder that recognizes only LF as a
  continuation leaves the CR or form feed in the value. Normalize once, at the top
  of `Stylesheet::parse`, before comment stripping and delimiter scanning, so every
  scanner below - `string_length`, `find_unquoted`, `split_unquoted`,
  `decode_string_escapes` - only has to handle LF. Do not normalize the source
  again anywhere below, or a literal `\r`/`\f` inside a string is silently
  rewritten twice.
- **`cargo publish` resolves path dependencies against the registry.** A crate
  that depends on `xmlkit` cannot be packaged until `xmlkit` is *published*, and
  the error ("no matching package named `xmlkit` found") reads like a broken
  manifest but is only the order. **`--no-verify` does not help**: the failure is
  in dependency resolution, not in building.
- **Cargo's uncommitted-files check is per package.** A dirty file at the
  repository root does not stop `cargo publish -p xmlkit`; one under
  `crates/xmlkit/` does. `scripts/publish.py` reports this per crate.
- **A published version number cannot be reused or deleted.** If `0.1.0` is
  wrong, the fix ships as `0.1.1`. Dry-run before uploading. This is why a crate
  nobody changed must not be released: the version is permanent.
- **A shared `[workspace.package] version` re-versions every crate at once.** That
  is what made every release publish all 26 crates, and what kept the number in
  the patch position. Each crate now declares its own `version`, and the workspace
  sets none. Do not restore it; `release_plan.py` reports which crates actually
  changed, and `bump.py` bumps one at a time.
- **A change that adds a function is a minor bump below 1.0.** `0.1.9` -> `0.2.0`,
  not `0.1.10`: `^0.1.9` does not reach `0.2.0`, so a minor bump is also the
  breaking-change bump pre-1.0. `publish.next_version` implements this; a reflexive
  `patch` is how a crate stays on `0.1.x` forever.
- **A patch bump of `xmlkit`/`pdfa` keeps dependents building; a minor bump does
  not.** `^0.1.9` accepts `0.1.10`, so dependents need no release for it, but
  `0.2.0` falls outside the requirement and every dependent has to be re-released
  against it. `bump.py` updates `[workspace.dependencies]` and names them.
- **A quadratic distance matrix aborts the caller before it returns a wrong
  answer.** Unrestricted Damerau-Levenshtein and OSA both allocate a cell per
  pair of positions, so two 8 KiB strings need 512 MiB. That is not a slow
  function, it is an allocation failure in a library whose signature has no way
  to report one. OSA needs only the previous two rows. Damerau-Levenshtein needs
  an arbitrary earlier row for its transposition term, so rolling rows alone
  cannot do it: save one copy of the row per distinct character, taken when that
  character's last occurrence is processed. That is `O(s * min(m, n))` rather
  than `O(m * n)`. The saving is not unconditional: a character that occurs only
  once in the longer string still needs its row (`ab` against `ba` reads row 0
  through the single `a`), so a string of thousands of *distinct* characters
  still allocates tens of megabytes. Measured at 8 KiB: 26 distinct characters
  cost 1 MiB, 2,048 cost 94 MiB, 8,192 cost 416 MiB. Say `O(s * min(m, n))` and
  show the measurements; do not write "linear" and stop. The corpus must contain
  long inputs or the bug is invisible - the original fixture capped at 12
  characters, where the matrix is 144 cells.
- **`gen.py > fixture.tsv` destroys the fixture on failure.** The shell truncates
  before the script starts, so a version mismatch or a new oracle disagreement
  leaves an empty file and the replay test fails until git restores it. Fixture
  generators must write to a temporary sibling and rename, and take the path as
  an argument.
- **A seeded corpus regenerated with more draws silently loses pairs.** Adding
  one draw shifts every later one, so widening a corpus that shares a
  `random.seed` drops cases it used to have - 1,283 of them in this repository.
  Read the existing fixture, re-ask those pairs and merge, so a corpus only
  grows; report the carried count so a shrink is visible in the log.
- **A test whose input does not need the code under test passes without it.**
  The wide-alphabet Damerau test used a substitution, which the ordinary
  recurrence already scores as one edit, so it passed with the snapshots deleted
  and with the snapshot taken after the row swap. A transposition in an
  all-distinct alphabet is the shape that *requires* the term. Check a new test
  by deleting the code it covers: if it still passes, it tests nothing.
- **`VmHWM` cannot measure a single call.** It is a maximum for the whole
  process and never falls, so a delta around one call is zero after any larger
  earlier allocation, and `clear_refs` does not reset `VmPeak`. Untouched
  `vec![0; n]` pages are also not resident, so a 64 MiB buffer reads as +4 KiB.
  Measuring peak needs a fresh child process; `crates/fuzzy-score/tests/memory.rs`
  re-executes its own binary to get one. A global counting allocator would be
  better but needs `unsafe`, which this workspace forbids.
- **A `Vec<char>` copy of both inputs is `O(m + n)` that the DP bounds miss.**
  Every function except `hamming` copies its arguments, because a recurrence
  indexes the column string by position. Document the copies or the table
  overstates the saving. The shorter string belongs in the column role, or a
  long *second* argument allocates a row as long as itself - `levenshtein` was
  missing that swap while `osa` had it.
- **"Attributable to X" is not a check; "equal to what X predicts" is.** A
  classifier that says "this pair contains a grapheme, so the references may
  disagree" accepts every future mismatch on any such pair, including one caused
  by a bug. The check that means something re-derives the expected value - here,
  re-asking `rapidfuzz` with both strings reduced to one scalar per cluster - and
  fails when the two differ.
- **A reference that segments grapheme clusters is not comparable on the same
  unit.** `jellyfish` 1.3 compares extended grapheme clusters; `rapidfuzz` and
  `strsim` compare Unicode scalar values. They disagree on any text with a
  combining mark or a joined emoji - `"e"` against `"e"` + U+0301 is Jaro `0.833`
  against `0.0`, and `"👩‍💻"` against `"👨‍🔬"` is Levenshtein 2 against 1 - and
  neither is wrong. Pick the unit, say so in the crate docs, and put the
  divergent pairs in the generator with both values pinned, or the cross-check
  either fails on every such pair or hides them by skipping.
- **A distance function that returns `Result` and one that returns a number have
  different contracts, not different quality.** `strsim::hamming` rejects unequal
  lengths; `rapidfuzz` and `jellyfish` count the extra positions. A caller
  switching crates changes behaviour on the first truncated input, so the
  divergence belongs in the function docs rather than in a changelog.
- **An edit-distance matrix with a sentinel row shifts every index by one.** The
  published Damerau-Levenshtein transcription (Lowrance and Wagner) reserves
  `d[0][*]` and `d[*][0]` for a sentinel, so the cell for characters `i` and `j`
  is `d[i + 1][j + 1]`, and the transposition term reads `d[k][l]` with `k` from
  the last row a character was seen in and `l` from the current match column.
  Writing the loop 1-based but reading `d[i - 1][j - 1]` reads the sentinel and
  under-reports the distance. It only shows on pairs with no characters in
  common, which is why a curated test passes and a corpus fails: the fixture in
  `crates/fuzzy-score/tests/fixtures/distance.tsv` caught it on the first run.
- **A "half a transposition" is integer division in every reference.** Jaro
  counts transpositions in pairs and halves the count with `//` (jellyfish
  literally writes `trans_count //= 2`), so an odd count loses its remainder.
  Dividing by `2.0` gives a score that is too high on roughly half of all pairs
  and looks plausible on the examples.
- **A reference that returns a similarity in `0..=1` has no normalized form.**
  `rapidfuzz`'s `JaroWinkler.normalized_similarity` is `1 - (1 - s)`, which
  re-rounds the last bit and can differ from `s` by one ULP. Pinning a fixture to
  it asserts a floating-point artifact rather than the algorithm, so the fixture
  records the similarity and not its normalization.
- **Two references can disagree, and the disagreement is the finding.** Jaro of
  two empty strings is `1.0` in `rapidfuzz` and `0.0` in `jellyfish`. A generator
  that quietly accepted either would hide a future change in both, so
  `scripts/fuzz/fuzzy-score/gen_distance.py` asserts that this pair is the *only*
  disagreement and fails on any other. Pick a side, document it, and make the
  choice a checked assertion rather than a comment.
- **A crate name cannot be changed after publication.** Check with
  `research/tools/namecheck.py` before writing code, not after.
- **A gate that fetches live data fails on the data, not the code.**
  `verify_aviation.py` compares the crate against OurAirports' current
  `airports.csv`, so an upstream row added, renamed or re-coordinated since the
  fixture was cut shows up as a disagreement in a crate nobody touched. The
  crate's own numbers are the check that matters: `cargo test -p aviation-data`
  against the committed fixture passes, and `verify_aviation.py`'s row and
  lookup counts (`11,887` rows, `46,456` lookups) still match. Read the failure
  before "fixing" it, and only regenerate the fixture deliberately, as its own
  change.
- **crates.io category names must be in the live list**, which is not the same as
  an intuitive guess. Fetch
  `https://crates.io/api/v1/categories?per_page=100` and compare against the
  `slug` field, not the display name (`parser-implementations` is the slug for
  "Parser implementations"). Without `per_page` it returns 10 per page and the
  list runs to 58, so a single default page looks complete when it is not.
- **A crate can sit at a version its last tag is behind but that was never
  published.** `cssprint` was bumped to `0.2.0` in the manifest while the last
  tag was `cssprint-v0.1.11`, and `0.2.0` was never uploaded. `release_plan.py`
  reads that as "bumped past its last tag" and suggests `0.2.1`, which would
  spend a number nobody asked for: the pending release is the manifest version
  itself. The script only compares the manifest to the last tag; it never asks
  crates.io. Check the manifest version against the registry before tagging a
  crate the plan flags as `bumped` -
  `curl -so /dev/null -w '%{http_code}' https://crates.io/api/v1/crates/<crate>/<version>`
  answers `404` when the version is still unpublished.
- **A trailing space decides `text-align` versus `text-align-last`.** When a
  collapsible trailing space does not fit on a line that has a break in it (two
  or more words), WeasyPrint lets it start a line of its own, then drops that
  line. It still counts it, so the line the space left is not last and is placed
  by `text-align`, not `text-align-last`. A single word has no break to drop the
  space at, so the space stays and the line is last. Getting this wrong moves a
  line by the whole alignment offset, which is what `cssprint`'s
  `align_last_trailing_space_overflow` fixture pins.
- **The oracle gate can omit the types the README opens with.** `stdnum-rs`
  documents `Iban`, `Ean`, `Isbn` and `Luhn` first, and none of the four was in
  `gen_stdnum_vectors.py`'s type list, so the crate's own flagship types were
  the only ones with no differential coverage - and all three defects the round
  found (`Iban` structure, `Isbn` nine-digit form, `Isbn` Bookland prefix) lived
  there. A type is covered when its name is in the gate, not when the crate
  implements it; check the gate's list against the README's opening before
  trusting a green build.
- **A structure check is not a length check.** `Iban::parse` claimed to check
  "length and structure" while checking length only, so a BBAN with a letter
  where the registry requires a digit (`DE171OCTNFFVAV1D4OWFYE`) closed MOD-97
  and passed. The registry lives in the reference's own `iban.dat`; the fix is a
  generated `<count>!<class>` table, and the class vocabulary is exactly `n`
  (digit), `a` (upper-case letter) and `c` (either).
- **The reference's national IBAN checks are all arithmetic, except Belgium's.**
  `be, es, me, no` are the only national IBAN modules; `es`, `me` and `no` are
  arithmetic (Spain's CCC, Montenegro's mod-97, Norway's mod-11 after a leading
  `0000` postgiro strip), but Belgium also looks the bank code up in a 793-entry
  register. The crate ships no register for any identifier, so it checks the
  Belgian check digit and skips the lookup - the same boundary the MAC address
  and the BIC registry already observe. `u32` overflows on Belgium's ten-digit
  `first * 10` fold; use `u64`.
- **A nine-digit ISBN-10 is a dropped leading zero, not a short number.** The
  reference's `compact` prepends `0` before checking, so `010010351` validates
  as `0010010351`. Without the pad the check digit is computed over the wrong
  positions and a published number is rejected. A 13-digit ISBN is a Bookland
  EAN, so its prefix must be `978` or `979`; any other GS1 prefix is an EAN, not
  a book number, and the reference raises `InvalidComponent` for it.
- **Luhn accepts the lone digit `0`, and that is deliberate.** The reference's
  one-digit checksum is the digit itself, so `0`, `00`, `000` and `0000` all
  close; the crate requires a payload and a check digit (two characters). This
  is the crate's rule, not a defect: the fixture records the reference's verdict
  and the crate's test records the reason. Do not "fix" it to match.
- **A corpus of only-rejected rows agrees with a crate that rejects
  everything.** The first IBAN corpus drew a random BBAN and closed the MOD-97
  check, so every Spanish, Norwegian and Montenegrin row it produced was a
  *rejection*: the reference rejects a random national check digit, and so did
  the crate, and a crate that rejected every `ES`/`NO`/`ME` IBAN outright would
  have passed the whole file. The fix is to generate the *accepted* side too -
  solve the national check digits by asking the reference's own `is_valid` for
  the one combination it accepts, rather than reimplementing the arithmetic -
  and prove the gate bites by mutating the crate to reject them all. A fixture
  whose every row for a check is negative tests only half of it.
- **A generator that reads the oracle must assert its version.** `gen_stdnum_vectors.py`
  and `gen_iban_structures.py` both read the installed `python-stdnum` (the
  first asks it for verdicts, the second reads its `iban.dat`), so a fixture
  regenerated against another release re-baselines the oracle silently and two
  maintainers on two releases get two fixtures from one checkout. Both now
  assert `PINNED = "2.2"` and stop the run otherwise, the same guard
  `verify_holidays.py` uses for `holidays==0.108`. Pin beside the generator and
  in `research/tools/README.md`, and keep a deliberate oracle upgrade its own
  change. `javaproperties/gen_writer.py` asserted nothing until a review caught
  it, which is the failure mode this rule describes: the corpus would have
  re-baselined silently on the next `pip install -U javaproperties`.

## Boundaries

- Branch `main` only. Do not create long-lived branches without being asked.
- **Never commit `.openhands/`** — it holds agent memory and is not intended to
  be public. Stage with `git add -A -- ':!.openhands'`.
- Do not push and do not open a pull request unless asked to.
- `git push` prompts and then hangs, because `origin` is plain HTTPS. Use
  `GIT_TERMINAL_PROMPT=0 git push "https://x-access-token:${GITHUB_TOKEN}@github.com/cipansor/crateworks.git" main`.
- If a change deleted something that was never committed, `git checkout -- <path>`
  recovers it. Do not reach for destructive git commands to undo work; ask first.

## Where things live

```
crates/<name>/          one crate: src/, tests/, README.md, LICENSE, NOTICE
scripts/release_plan.py which crates changed since their last release (read-only)
scripts/bump.py         bump one crate's version by an explicit semver level
scripts/publish.py      derive and run the publish order (dry-run by default)
scripts/fuzz/<name>/    corpus generators (dev tools, never built by CI)
research/               the measurements that identified the gaps
research/tools/         the scripts that produced those numbers
PLAN.md                 build history and per-crate scope
```

## Documentation language

Everything in this repository is written in **English**: crate and root READMEs,
doc comments, `AGENTS.md`, `CONTRIBUTING.md`, `PLAN.md`, `research/`, commit
messages, and the messages and status strings printed by the scripts. The owner's
first language is Indonesian, but the audience for a published crate is
international, so the documents are English and the chat can be Indonesian.

Comments explain why, not what: state a non-obvious invariant, a workaround, or a
deliberate trade-off. Do not restate the code or narrate the change history.
