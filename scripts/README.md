# scripts

Development tools for the workspace. None of them is needed to build or test a
crate. `check_attribution.py` and `check_actions_pinned.py` run in CI, on the
`test` job; the rest are run by hand.

| Script | What it does |
| --- | --- |
| `check_attribution.py` | Fails when a crate credits an upstream project in its code but not in `NOTICE` (run in CI) |
| `attribution.toml` | Data for the above: the upstream projects and the marker that identifies each |
| `check_actions_pinned.py` | Fails when a workflow uses an action by a mutable tag or branch instead of a commit SHA (run in CI) |
| `release_plan.py` | Reports which crates changed since their last release, and the bump each one suggests |
| `bump.py` | Bumps one crate's version by an explicit semver level, refusing a crate with no change |
| `publish.py` | Works out the order the crates can be published in, and checks or runs it |
| `check_token.py` | Checks that the crates.io token in `CARGO_REGISTRY_TOKEN` is one the registry accepts (used by the release workflow) |
| `release_notes.py` | Builds release notes for one crate from the commits that touched it (used by the release workflow) |
| `fuzz/` | Generators for the fuzz corpora in `crates/*/tests/fixtures/` |
| `tests/` | Tests for the scripts above (run in CI) |

## tests/

Tests for the scripts, run in CI with `python3 -m unittest discover -s scripts/tests`.
They cover the failure modes that are quiet rather than loud: a credential left in
the release notes, and a release tag that names the wrong crate or version. Neither
breaks a build, so a test is the only thing that catches a regression.

```sh
python3 -m unittest discover -s scripts/tests -v
```

## check_token.py

Checks the crates.io token in `CARGO_REGISTRY_TOKEN` before a release uploads
anything. `cargo publish` reports a bad token only at the upload, after the dry
run, so the release workflow calls this first and fails in seconds instead.

```sh
CARGO_REGISTRY_TOKEN=<token> python3 scripts/check_token.py
```

It asks `/api/v1/me` and reads the **body**, not the status code. That endpoint
answers `403` for an authenticated token too, because it is a web-session
endpoint:

| Body | Meaning |
| --- | --- |
| `this action can only be performed on the crates.io website` | the token is usable |
| `authentication failed` | the token is missing, invalid or expired |

A status-code check would call a rejected token good, since both are `403`. The
token is never printed.

## check_actions_pinned.py

The cipansor organisation requires every action to be pinned to a full-length
commit SHA. A tag or branch such as `@v4` is mutable, so whoever controls the
upstream repository can move it and the next run executes code nobody reviewed.
Review does not catch this, because `@v4` reads as a version.

```sh
python3 scripts/check_actions_pinned.py                 # scan .github/
python3 scripts/check_actions_pinned.py path/to/wf.yml  # scan one file
```

It accepts a 40-hex SHA, a local action (`./...`) and a docker image pinned by
digest (`docker://image@sha256:<64 hex>`), and rejects a tag, a branch, a bare
`owner/repo` with no ref, and a docker image with only a tag.

Keep the tag in a trailing comment, because a bare SHA says nothing to a reader:

```yaml
- uses: actions/checkout@11d5960a326750d5838078e36cf38b85af677262 # v4
```

To get the SHA for a tag, dereference it: an annotated tag points at a tag
object rather than a commit, so `git/tags/<sha>` is needed, not `git/ref/tags/`.

## check_attribution.py

Crates here are independent reimplementations of existing libraries, and the
upstream licence requires that library to be named in `NOTICE`. The credit is
easy to put in a source comment and forget in `NOTICE`: `stdnum-rs` cited
`python-stdnum` in `src/id/mod.rs` while its `NOTICE` said nothing, even though
`eu-vat`, from the same lineage, did name it. No compiler catches that, so the
script does.

The list of upstream projects lives in `attribution.toml`, so adding a crate
means editing data, not code. For each entry the script checks: if the entry's
`marker` appears anywhere in a crate's `src`, `tests` or `README.md`, then that
same marker must also appear in the crate's `NOTICE`.

```sh
python3 scripts/check_attribution.py   # exit 1 and name the crate if a credit is missing
```

Pick a marker that is distinctive to the upstream project, normally a repository
path, so that a crate cannot match its own name. `hijridate` matches itself,
which is why its marker is the author's handle `dralshehri` rather than the
project name. A marker that is too generic turns into a false positive: a marker
of `xmlkit` matches every crate that depends on `xmlkit`.

## release_plan.py

Answers the first question of a release: which crates have changed since their
last release? A release here publishes one crate, and a crate nobody changed is
not a release - a version on crates.io can never be deleted, so re-uploading one
publishes a version nobody asked for, permanently.

```sh
python3 scripts/release_plan.py            # every crate
python3 scripts/release_plan.py --crate xmlkit
python3 scripts/release_plan.py --json     # machine-readable
```

It is read-only. It reports a crate when any of these holds:

| Reason | What it means |
| --- | --- |
| A release-worthy commit | A commit touched the crate's code since its last tag |
| An uncommitted change | A changed or untracked file under `crates/<name>/` that is not just `Cargo.toml` |
| A pending bump | The manifest version is ahead of the crate's last tag |
| A dependency moved | `xmlkit` or `pdfa` released a version outside the crate's `^` requirement |

Documentation, tests, CI, housekeeping and a `Cargo.toml`-only edit do **not**
count, so a packaging change does not look like 26 releases. A commit subject
that does not follow the convention is assumed to matter, because guessing "no
release" from it is the costlier mistake.

The suggested level comes from the commit subjects: any `feat` or `!` breaking
marker asks for a minor, otherwise a patch. It is a suggestion to confirm.

A dependency only makes a dependent due when the new version falls outside `^`.
`^0.1.9` accepts `0.1.10`, so a patch bump of `xmlkit` needs no dependent
release; `0.2.0` does not, so every dependent is due whether or not its own code
changed.

Full history is required, for the same reason `release_notes.py` needs it: the
range and the tags come from git, and a shallow clone would silently report the
wrong thing.

## bump.py

Bumps one crate's version by an explicit semver level. It edits one manifest, and
never another crate's - which is the opposite of the shared workspace version
this repository used to carry, where a single edit re-versioned all of them.

```sh
python3 scripts/bump.py stdnum-rs patch
python3 scripts/bump.py xmlkit minor
python3 scripts/bump.py stdnum-rs patch --dry-run
```

The level follows Cargo's compatibility rule, which differs below `1.0` because
the leading non-zero field is the boundary:

| Level | `0.1.9` becomes | `1.2.3` becomes | When |
| --- | --- | --- | --- |
| `patch` | `0.1.10` | `1.2.4` | A compatible fix |
| `minor` | `0.2.0` | `1.3.0` | New functionality, or a breaking change below 1.0 |
| `major` | `1.0.0` | `2.0.0` | A deliberate move to a stable API |

Adding a function is a **minor** bump below 1.0, not a patch: `^0.1.9` does not
reach `0.2.0`, so a minor bump is also what a breaking change takes. Reaching for
`patch` by reflex is how a crate stays on `0.1.x` forever. The arithmetic lives
in `publish.next_version`, so it is not re-derived by hand.

A crate with no change since its last release is refused, because re-releasing it
publishes a version nobody asked for. `--force` overrides that when a release is
wanted anyway.

When the crate is `xmlkit` or `pdfa` - the two the workspace depends on - the
`[workspace.dependencies]` requirement is updated too, and the dependents that
need a re-release are named. A patch bump keeps them building; a minor bump does
not, so they must be released against the new version.

It edits files and stops there. Committing, tagging and publishing are separate,
visible steps.

## publish.py

`cargo publish` resolves a `path` dependency against the registry, so a crate
that depends on another crate in this workspace cannot be packaged until that
dependency is published. Get the order wrong and the error depends on whether the
dependency has ever been published. On a first release:

```
error: failed to prepare local package for uploading
Caused by:
  no matching package named `xmlkit` found
```

and, once an older version is on the registry, which is the shape every later
release takes:

```
error: failed to prepare local package for uploading
Caused by:
  failed to select a version for the requirement `xmlkit = "^0.1.1"`
  candidate versions found which didn't match: 0.1.0
```

Both read like a broken manifest but are only the order, and both are reported as
`blocked`. A version-selection failure is only treated as the order when the
named crate really is a dependency inside this workspace, so a conflict on a
third-party crate is still a failure. `--no-verify` does not help, because the
failure is in resolving the dependency rather than in building it.

The script derives the order from the manifests, so it cannot go stale the way a
hand-written list does: if a crate gains a workspace dependency, it moves.

### Authenticating

The dry run needs no credentials. Uploading does, and there is no token in this
repository or in CI — publishing is done by hand from a machine that has one.

```sh
cargo login          # paste the token once; cargo stores it in $CARGO_HOME/credentials.toml
```

Or pass it per command without storing it:

```sh
CARGO_REGISTRY_TOKEN=<token> python3 scripts/publish.py --publish
```

crates.io tokens are scoped: a token needs `publish-new` for a crate that does
not exist yet and `publish-update` for a later version. For this workspace's
first release the token needs both.

```sh
python3 scripts/publish.py            # dry-run every crate, upload nothing
python3 scripts/publish.py --strict   # same, but exit 1 if anything is blocked
python3 scripts/publish.py --publish  # upload, in order, stopping at the first failure
python3 scripts/publish.py --publish --delay 65   # space uploads out, one per minute
```

The default mode uploads nothing and always exits 0 unless a crate genuinely
fails to build or package. A crate reported as `blocked` is the expected state
before its dependency is published, not an error; `--strict` is the flag to use
after publishing a dependency, to confirm the next group is now clear.

`--publish` prints the order and waits for the word `publish` before doing
anything. It stops at the first failure rather than carrying on, because a
published version number cannot be reused: if `0.1.0` of a crate is wrong, the
fix has to ship as `0.1.1`. Run the dry run first.

Each line reports one of four states:

| State | Meaning |
| --- | --- |
| `ready` | `cargo publish --dry-run` succeeded; this crate can be uploaded now |
| `blocked` | A workspace dependency is not on crates.io yet. Publish that first |
| `uncommitted` | Files in the crate are changed or untracked, so cargo refuses to package it |
| `FAILED` | Something else is wrong. Fix it before publishing anything |

The `uncommitted` check is scoped to each crate because cargo's is: a changed
file at the repository root does not stop `cargo publish -p xmlkit`, but one
under `crates/xmlkit/` does.

## release_notes.py

Builds the release notes for one crate. The release workflow runs it and passes
the result to `gh release create`, but it is also useful on its own to see what a
tag would announce before pushing it.

```sh
python3 scripts/release_notes.py --tag xmlkit-v0.1.1
python3 scripts/release_notes.py --tag xmlkit-v0.1.1 --out notes.md
```

The notes cover one crate, because a release does. Only commits that touched
`crates/<crate>/` are listed, so a release for `xmlkit` does not describe the
other crates. The range starts at the previous tag of the *same* crate,
not at the previous tag overall, so a crate that skipped a release does not
repeat notes it already published; with no earlier tag for the crate this is its
first release and the crate's whole history is used.

Commit subjects are grouped by conventional-commit type (`feat`, `fix`, `docs`,
and so on). A subject that does not follow the convention is not dropped - it
lands under "Other changes" - so nothing silently disappears from the notes.

Full history is required: the range comes from `git log` and the previous tag
from `git tag`, and a shallow clone would silently produce an empty range rather
than fail. The script checks for a shallow clone and stops, and the workflow
checks out with `fetch-depth: 0` for the same reason.

## Rate limiting

crates.io limits how many *new* crates an account may publish in a short window,
about one every ten minutes, and the limit is on the account rather than on the
crate. A refused upload comes back as HTTP 429, which cargo reports as a plain
failure. The response names the moment to retry:

```
You have published too many new crates in a short period of time.
Please try again after Thu, 22 Oct 2026 03:32:25 GMT
```

`publish.py` reads that timestamp and sleeps until then, up to 12 attempts, so a
long release finishes on its own. A fixed backoff cannot work here: 70 seconds
does not clear a ten-minute window, it only spends an attempt. Pass `--delay 65`
to stay under the limit in the first place rather than relying on the retry.

A version that is already on crates.io (`crate xmlkit@0.1.0 already exists on
crates.io index`) is reported as `already published` and skipped. That makes a
re-run after a partial release finish the remaining crates instead of failing on
the ones already uploaded.

`cargo publish --workspace` also orders by dependency and is the command to use
when cargo's own output is wanted. It cannot check without attempting an upload,
which is why this script exists for the dry run.

## Releasing

Releases are per crate, because the crates version independently. Each crate
carries its own `version` in its own `Cargo.toml`; there is no shared workspace
version, because one would re-version every crate at once and that is how every
release came to publish all of them.

**Release only the crates that changed.** Start from the plan:

```sh
python3 scripts/release_plan.py          # which crates are due, and why
python3 scripts/bump.py xmlkit minor     # bump one of them, by an explicit level
```

Then commit, tag and push that one crate:

```sh
git tag xmlkit-v0.2.0 && git push origin xmlkit-v0.2.0
```

Repeat for each crate the plan named. Do not tag a crate it did not name: a crate
nobody changed publishes a version nobody asked for, and a version on crates.io
cannot be deleted.

`.github/workflows/release.yml` publishes one crate per tag. The tag is
`<crate>-v<version>`, and it runs on any `*-v*` tag or by hand from the Actions
tab (`workflow_dispatch`, with the tag as input) to finish a partial release.

Push the tags one at a time. A push carrying several tags starts the workflow
once, for one of them, so `git push --tags` with the seventeen tags that existed
then yielded a single run and sixteen releases that never appeared. For a batch,
push one tag and start the rest from the Actions tab (`workflow_dispatch`), which
takes the tag as an input; a version already on crates.io is reported and skipped,
so re-running is safe.

There is no bare `v<version>` tag. One version cannot describe every crate here,
which may sit at many different versions, and reading `v0.1.1` as "release
everything" publishes every crate nobody asked for, each of which crates.io
rejects because the version already exists. `publish.py` refuses such a tag rather
than guessing, so a mistaken `git tag v0.1.1` fails the check step instead of
uploading anything.

The workflow checks that the tag names a real crate at the version in its
manifest, runs the attribution check, dry-runs that crate, publishes it with
`--publish --yes`, and then confirms the version answers HTTP 200 from the
crates.io API. It needs one repository secret, `CARGO_REGISTRY_TOKEN`, a
crates.io API token with publish scope. Without it the publish step fails at the
first upload.

Bump the crate's manifest and tag the same version. A version number cannot be
reused or deleted, so a wrong upload has to be followed by a new version, not a
fix.

`--tag` also works locally, which is the way to see what a tag would release
without pushing it:

```sh
python3 scripts/publish.py --check-tag xmlkit-v0.1.1   # validate the tag alone
python3 scripts/publish.py --tag xmlkit-v0.1.1         # dry-run that crate
python3 scripts/publish.py --crate xmlkit              # dry-run by name
```

