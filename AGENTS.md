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
  five existing allows are the ceiling, not a precedent.
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
- **`xmlkit` name matching.** `Element::child` and `children_named` match the
  full name including the prefix, so `child(el, "IssueDateTime")` does **not**
  find `ram:IssueDateTime`. Match on the local name. `Element::text()`
  concatenates only direct text children, so a value nested in a sub-element
  comes back empty.
- **An empty inline element is still a box.** WeasyPrint gives `<span></span>`
  an empty inline box with its own strut, even with no text in it: `<div>a<span
  style="font-size:24px"></span></div>` raises the line to 24px, and an empty
  `<span>` on its own leaves a line box of height 0 that holds the 16px inline
  box. `cssprint` currently emits no box for an element with no in-flow
  children, so its line stays 16px and the empty inline box is missing. A
  whitespace-only node is different and already handled (see `push_text`): it
  survives as an empty text box, not an empty inline box.
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
