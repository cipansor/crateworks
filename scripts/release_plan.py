#!/usr/bin/env python3
"""Report which crates have changed since their last release, and why.

A release here is one crate, so the first question is which crates deserve one.
Publishing every crate because the workspace shares a version number is what this
script exists to prevent: a crate with no change since its last tag is not a
release, and re-uploading it publishes a version nobody asked for.

Two things make a crate due for a release:

* **Direct changes** - commits that touched `crates/<name>/` since that crate's
  last tag. This is the crate's own work.
* **A dependency move** - a workspace dependency (only `xmlkit` and `pdfa` have
  any) released a version the crate does not yet build against. A patch bump of
  the dependency still satisfies the existing `^` requirement, so the crate keeps
  working and is not automatically due; a minor bump does not, so a dependent is
  due whether or not its own code changed.

The suggested bump level comes from the commit subjects, using the same
conventional-commit types the release notes group by: any `feat` (or a `!`
breaking marker) asks for a minor, otherwise a patch. It is a suggestion to
confirm, not a decision - the script never edits a manifest.

Read-only. It runs `git log` and `git tag` and prints a report.

Usage:
    python3 scripts/release_plan.py              # every crate
    python3 scripts/release_plan.py --crate xmlkit
    python3 scripts/release_plan.py --json       # machine-readable
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

# `publish` is a sibling script, not an installed package. Run as a file its
# directory is already on the path; imported from elsewhere it is not.
sys.path.insert(0, str(Path(__file__).resolve().parent))

import publish
import release_notes

ROOT = Path(__file__).resolve().parent.parent

# A commit subject is "new functionality" for bump purposes when it is a `feat`
# or carries the conventional-commit breaking marker (`!` before the colon). A
# breaking change below 1.0 is a minor bump, not a major one - see
# `publish.next_version`.
FEATURE = re.compile(r"^(feat|feature)(\([^)]*\))?!?: ")
BREAKING = re.compile(r"^[a-z]+(\([^)]*\))?!: ")

# Commit types that do not, on their own, justify a release. A crate whose only
# changes since its last tag are documentation, tests, CI or housekeeping is not
# due: re-releasing it publishes a version nobody asked for, which is the habit
# this repository is trying to break. An unconventional subject is NOT in this
# set - it is assumed to matter, because guessing "no release" from a subject
# nobody followed the convention for is the more expensive mistake.
NO_RELEASE_TYPES = frozenset({"docs", "test", "ci", "build", "chore", "style"})


def commit_type(subject: str) -> str | None:
    match = release_notes.CONVENTIONAL.match(subject)
    return match.group("type") if match else None


def changed_paths(crate: str, commit: str) -> list[str]:
    """The crate's paths a commit touched."""
    out = release_notes.git(
        "show", "--name-only", "--format=", commit, "--", f"crates/{crate}"
    )
    return [line.strip() for line in out.splitlines() if line.strip()]


def commit_is_release_worthy(subject: str, paths: list[str]) -> bool:
    """Whether one commit justifies a new published version.

    It does not when it is documentation, tests, CI or housekeeping, or when it
    changed only the crate's `Cargo.toml`. That second rule is what keeps a
    packaging refactor - moving where the version is declared, say - from
    re-releasing every crate: the manifest changed, the published code did not.
    An unconventional subject is assumed to matter, because guessing "no release"
    from a subject that did not follow the convention is the costlier mistake.
    """
    kind = commit_type(subject)
    if kind is not None and kind in NO_RELEASE_TYPES:
        return False
    if paths and all(path.endswith("Cargo.toml") for path in paths):
        return False
    return True


def release_worthy(crate: str, commits: list[tuple[str, str]]) -> bool:
    """Whether a new version should carry any of these commits."""
    return any(
        commit_is_release_worthy(subject, changed_paths(crate, hash_))
        for hash_, subject in commits
    )


def crate_changes(crate: str, since: str | None) -> list[tuple[str, str]]:
    """(short hash, subject) for commits touching the crate since its last tag."""
    return release_notes.crate_commits(crate, since)


def suggested_level(commits: list[tuple[str, str]]) -> str:
    """The bump level the commit subjects argue for: minor for a feature, else patch."""
    for _, subject in commits:
        if BREAKING.match(subject) or FEATURE.match(subject):
            return "minor"
    return "patch"


def workspace_dependencies(manifest: dict) -> list[str]:
    """The workspace crates this manifest depends on, by name.

    Only `xmlkit` and `pdfa` are workspace dependencies; a `{ workspace = true }`
    entry is what marks one, and the name is the dependency key.
    """
    deps = manifest.get("dependencies", {})
    return [
        name
        for name, spec in deps.items()
        if isinstance(spec, dict) and spec.get("workspace") is True
    ]


def dependency_moved(crate: str, dep: str, manifests: dict[str, dict]) -> bool:
    """Whether `dep` released a version the crate's requirement does not accept.

    A patch bump stays inside `^`, so the crate still builds against it and is not
    due on that account alone; a minor bump falls outside `^0.x`, so the crate is
    pinned to the old one until it is re-released against the new.
    """
    dep_version = publish.version_of(manifests[dep])
    dep_tag = release_notes.latest_tag(dep)
    if dep_tag is None:
        return False
    _, released = publish.crates_from_tag(dep_tag)
    return not requirement_accepts(released, dep_version)


def requirement_accepts(released: str, candidate: str) -> bool:
    """Whether `candidate` satisfies a caret requirement written at `released`.

    This is Cargo's `^` rule, which is not plain "same major": below 1.0 the
    leading non-zero field is the compatibility boundary, so `^0.1.9` accepts
    `0.1.10` but rejects `0.2.0`.
    """
    rel = tuple(int(p) for p in released.split("."))
    cand = tuple(int(p) for p in candidate.split("."))
    if rel[0] > 0:
        return cand[0] == rel[0] and cand >= rel
    if rel[1] > 0:
        return cand[0] == 0 and cand[1] == rel[1] and cand >= rel
    return cand[0] == 0 and cand[1] == 0 and cand == rel


def substantive_dirty(paths: list[str]) -> list[str]:
    """The dirty files that mean the published code changed.

    A changed `Cargo.toml` on its own is a version or packaging edit, not a
    change to what the crate does; a version bump is already detected by
    comparing the manifest against the last tag. Counting it here would make this
    refactor look like 26 releases.
    """
    return [path for path in paths if not path.endswith("Cargo.toml")]


def plan(crates: list[str], manifests: dict[str, dict]) -> list[dict]:
    """One entry per crate that is due for a release.

    A crate is due when any of these holds: it has a release-worthy commit since
    its last tag, it has uncommitted changes, its manifest version is already
    ahead of its last tag (bumped but not released), or a workspace dependency
    released a version its requirement no longer accepts. A crate whose only
    commits are documentation, tests, CI or housekeeping is not due - see
    `release_worthy`.
    """
    dirty = publish.dirty_by_crate(publish.dirty_files(), crates)
    entries = []
    for crate in crates:
        tag = release_notes.latest_tag(crate)
        commits = crate_changes(crate, tag)
        moved = [
            dep
            for dep in workspace_dependencies(manifests[crate])
            if dependency_moved(crate, dep, manifests)
        ]
        current = publish.version_of(manifests[crate])
        # A tag whose version is behind the manifest means a bump is pending a
        # tag; that is an unreleased change even with a clean tree.
        tagged_version = None
        if tag is not None:
            _, tagged_version = publish.crates_from_tag(tag)
        bumped = tagged_version is not None and tagged_version != current
        uncommitted = substantive_dirty(dirty.get(crate, []))
        changed = bool(release_worthy(crate, commits) or uncommitted or bumped or moved)
        if not changed:
            continue
        level = suggested_level(commits)
        # A crate with no tag has never been published, so its first release is
        # the version already in its manifest - there is no prior version to bump
        # past. `openlocationcode` went out as `0.1.0`, not `0.1.1`.
        first_release = tag is None
        entries.append(
            {
                "crate": crate,
                "version": current,
                "next": current if first_release else publish.next_version(current, level),
                "level": None if first_release else level,
                "first_release": first_release,
                "last_tag": tag,
                "released_version": tagged_version,
                "uncommitted": uncommitted,
                "bumped": bumped,
                "commits": [{"hash": h, "subject": s} for h, s in commits],
                "dependencies_moved": moved,
            }
        )
    return entries


def render_text(entries: list[dict], total: int) -> str:
    if not entries:
        return (
            f"No crate has changed since its last release "
            f"({total} crates checked). Nothing to release.\n"
        )
    lines = [f"{len(entries)} of {total} crates are due for a release:", ""]
    for entry in entries:
        if entry["first_release"]:
            lines.append(f"{entry['crate']} {entry['version']} (first release)")
        else:
            lines.append(
                f"{entry['crate']} {entry['version']} -> {entry['next']} "
                f"({entry['level']})"
            )
        if entry["last_tag"]:
            lines.append(f"    since {entry['last_tag']}")
        else:
            lines.append("    never released")
        if entry["bumped"]:
            lines.append(
                f"    manifest already bumped past {entry['released_version']}"
            )
        if entry["uncommitted"]:
            lines.append(
                f"    {len(entry['uncommitted'])} uncommitted file(s) in the crate"
            )
        if entry["dependencies_moved"]:
            lines.append(
                f"    dependency moved: {', '.join(entry['dependencies_moved'])}"
            )
        for commit in entry["commits"]:
            lines.append(f"    {commit['hash']} {commit['subject']}")
        lines.append("")
    if any(entry["first_release"] for entry in entries):
        lines.append(
            "A first release needs no bump: its manifest version is the version to "
            "publish."
        )
    lines.append("Release each one on its own: scripts/bump.py <crate> <level>")
    lines.append("then tag and push <crate>-v<version>. See scripts/README.md.")
    return "\n".join(lines) + "\n"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--crate", action="append", default=[], metavar="NAME")
    parser.add_argument("--json", action="store_true", help="print JSON instead of text")
    args = parser.parse_args()

    release_notes.require_full_history()

    crates, _, manifests = publish.load_manifests()
    if args.crate:
        unknown = [name for name in args.crate if name not in manifests]
        if unknown:
            print(f"error: not a crate here: {', '.join(unknown)}", file=sys.stderr)
            return 1
        selected = [name for name in crates if name in set(args.crate)]
    else:
        selected = crates

    entries = plan(selected, manifests)
    if args.json:
        print(json.dumps(entries, indent=2))
    else:
        print(render_text(entries, len(selected)), end="")
    return 0


if __name__ == "__main__":
    sys.exit(main())
