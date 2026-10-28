#!/usr/bin/env python3
"""Bump one crate's version, refusing to bump a crate that has not changed.

The version of a release is a decision, and this script makes it one: it takes
the crate and the semver level explicitly, edits only that crate's manifest, and
never touches another crate. That is the opposite of the shared workspace version
this repository used to carry, where one edit re-versioned all of them and every
release then published every crate.

Levels follow `publish.next_version` and Cargo's compatibility rule:

    patch   0.1.9 -> 0.1.10   a compatible fix
    minor   0.1.9 -> 0.2.0    new functionality, or a breaking change below 1.0
    major   0.1.9 -> 1.0.0    the deliberate move to a stable API

A crate with no change since its last tag is refused unless `--force` is given,
because re-releasing it publishes a version nobody asked for. When the crate is
`xmlkit` or `pdfa`, the `[workspace.dependencies]` requirement is updated too,
and the dependents that now need a release are named.

This edits files; it does not commit, tag or publish. Those are separate, visible
steps.

Usage:
    python3 scripts/bump.py stdnum-rs patch
    python3 scripts/bump.py xmlkit minor
    python3 scripts/bump.py stdnum-rs patch --dry-run
"""

import argparse
import re
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))

import publish
import release_notes
import release_plan

ROOT = Path(__file__).resolve().parent.parent


def manifest_path(crate: str) -> Path:
    return ROOT / "crates" / crate / "Cargo.toml"


def set_crate_version(crate: str, version: str) -> None:
    """Rewrite the `version = "..."` line in the crate's `[package]` table.

    Only the first `version =` at the start of a line inside `[package]` is
    touched: `rust-version` and any dependency table are left alone.
    """
    path = manifest_path(crate)
    text = path.read_text()
    # Anchor on the `[package]` header and replace the first bare `version =`
    # after it, so a dependency's `version` is never the one rewritten.
    match = re.search(
        r'(?ms)^\[package\]\n(.*?)(?=^\[|\Z)',
        text,
    )
    if not match:
        raise publish.ReleaseError(f"{path} has no [package] table")
    body = match.group(1)
    if not re.search(r'(?m)^version = "', body):
        raise publish.ReleaseError(
            f"{crate} does not declare its own version; add `version = \"{version}\"` "
            "to its [package] table"
        )
    new_body = re.sub(
        r'(?m)^version = "[^"]*"',
        f'version = "{version}"',
        body,
        count=1,
    )
    path.write_text(text[: match.start(1)] + new_body + text[match.end(1) :])


def update_workspace_requirement(crate: str, version: str) -> bool:
    """Update `[workspace.dependencies]` for a crate other crates depend on.

    Returns whether a requirement was rewritten. Only `xmlkit` and `pdfa` appear
    there; a crate that is not a workspace dependency needs no change.
    """
    path = ROOT / "Cargo.toml"
    text = path.read_text()
    pattern = re.compile(
        rf'(?m)^({re.escape(crate)} = \{{ version = )"[^"]*"(, path = )"[^"]*"( \}})$'
    )
    new_text, count = pattern.subn(rf'\g<1>"{version}"\g<2>"crates/{crate}"\g<3>', text)
    if count:
        path.write_text(new_text)
    return count > 0


def dependents_of(crate: str, crates: list[str], manifests: dict[str, dict]) -> list[str]:
    """Crates whose manifest depends on `crate`."""
    found = []
    for name in crates:
        deps = manifests[name].get("dependencies", {})
        if crate in deps:
            found.append(name)
    return found


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("crate", help="the crate to bump")
    parser.add_argument("level", choices=publish.LEVELS, help="the semver level")
    parser.add_argument(
        "--dry-run", action="store_true", help="print what would change, edit nothing"
    )
    parser.add_argument(
        "--force",
        action="store_true",
        help="bump even when the crate has no change since its last tag",
    )
    args = parser.parse_args()

    crates, _, manifests = publish.load_manifests()
    if args.crate not in manifests:
        print(f"error: not a crate here: {args.crate}", file=sys.stderr)
        return 1

    try:
        release_notes.require_full_history()
        entries = {e["crate"]: e for e in release_plan.plan(crates, manifests)}
    except SystemExit as error:
        # A shallow clone: the change check cannot be trusted, so only allow a
        # forced bump and say why.
        if not args.force:
            print(f"error: {error}", file=sys.stderr)
            print("re-run with --force to bump without the change check", file=sys.stderr)
            return 1
        entries = {}

    current = publish.version_of(manifests[args.crate])
    next_version = publish.next_version(current, args.level)

    if args.crate not in entries and not args.force:
        print(
            f"{args.crate} has no change since its last release ({current}); "
            "nothing to release. Use --force to bump it anyway.",
            file=sys.stderr,
        )
        return 1

    if next_version == current:
        print(f"{args.crate} is already at {current}; nothing to do")
        return 0

    print(f"{args.crate}: {current} -> {next_version} ({args.level})")
    is_workspace_dep = bool(
        re.search(
            rf"(?m)^{re.escape(args.crate)} = \{{",
            (ROOT / "Cargo.toml").read_text(),
        )
    )
    if is_workspace_dep:
        print(f"  also update [workspace.dependencies] {args.crate} = \"{next_version}\"")
        dependents = dependents_of(args.crate, crates, manifests)
        if dependents:
            print(
                "  dependents that must be re-released against the new version "
                f"when it is a minor or major bump: {', '.join(dependents)}"
            )

    if args.dry_run:
        print("dry run: no files changed")
        return 0

    set_crate_version(args.crate, next_version)
    if is_workspace_dep:
        update_workspace_requirement(args.crate, next_version)
    print(f"updated crates/{args.crate}/Cargo.toml")
    print("Next: commit, then tag and push "
          f"`{args.crate}-v{next_version}` (one tag, one push).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
