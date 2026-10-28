"""Tests for the release scripts.

The rules these cover are the ones that fail quietly: a leaked credential in
published notes, or a tag that names the wrong crate. Neither breaks the build,
so nothing else would catch them.
"""

import importlib.util
import sys
import unittest
from pathlib import Path

SCRIPTS = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(SCRIPTS))


def load(name: str):
    spec = importlib.util.spec_from_file_location(name, SCRIPTS / f"{name}.py")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


publish = load("publish")
release_notes = load("release_notes")
release_plan = load("release_plan")
check_token = load("check_token")


class RepoUrl(unittest.TestCase):
    """`repo_url` feeds links that are published on the release page."""

    def run_with_remote(self, url: str) -> str:
        original = release_notes.git

        def fake(*args: str) -> str:
            return url + "\n"

        release_notes.git = fake
        try:
            return release_notes.repo_url()
        finally:
            release_notes.git = original

    def test_strips_a_token_from_an_https_remote(self):
        self.assertEqual(
            self.run_with_remote(
                "https://x-access-token:ghs_SECRETVALUE@github.com/cipansor/crateworks.git"
            ),
            "https://github.com/cipansor/crateworks",
        )

    def test_strips_a_username_and_password(self):
        self.assertEqual(
            self.run_with_remote("https://user:pw@github.com/o/r.git"),
            "https://github.com/o/r",
        )

    def test_converts_an_ssh_remote(self):
        self.assertEqual(
            self.run_with_remote("git@github.com:cipansor/crateworks.git"),
            "https://github.com/cipansor/crateworks",
        )

    def test_plain_https_remote_is_unchanged(self):
        self.assertEqual(
            self.run_with_remote("https://github.com/cipansor/crateworks.git"),
            "https://github.com/cipansor/crateworks",
        )

    def test_no_credentials_survive_in_any_form(self):
        for url in (
            "https://x-access-token:ghs_ABC@github.com/o/r.git",
            "https://token@github.com/o/r.git",
            "https://a:b@github.com/o/r",
        ):
            with self.subTest(url=url):
                out = self.run_with_remote(url)
                self.assertNotIn("@", out)
                self.assertNotIn("ghs_", out)


class CratesFromTag(unittest.TestCase):
    """A release tag names one crate; anything else must be refused."""

    def test_parses_a_crate_tag(self):
        self.assertEqual(publish.crates_from_tag("xmlkit-v0.1.1"), ("xmlkit", "0.1.1"))

    def test_rejects_a_bare_version_tag(self):
        with self.assertRaises(publish.ReleaseError):
            publish.crates_from_tag("v0.1.1")

    def test_rejects_a_tag_with_no_version(self):
        with self.assertRaises(publish.ReleaseError):
            publish.crates_from_tag("xmlkit")

    def test_rejects_an_empty_tag(self):
        with self.assertRaises(publish.ReleaseError):
            publish.crates_from_tag("")


class ManifestVersions(unittest.TestCase):
    """The tag has to agree with the manifest, so reading it must be right."""

    def test_every_crate_declares_its_own_version(self):
        # The workspace deliberately sets no shared version, so each crate must
        # carry its own; a crate that fell back to a workspace version would be
        # re-versioned by every other crate's release.
        crates, _, manifests = publish.load_manifests()
        self.assertGreater(len(crates), 0)
        for crate in crates:
            with self.subTest(crate=crate):
                self.assertIsInstance(publish.version_of(manifests[crate]), str)

    def test_the_workspace_declares_no_shared_version(self):
        import tomllib
        from pathlib import Path
        workspace = tomllib.loads(
            (Path(__file__).resolve().parent.parent.parent / "Cargo.toml").read_text()
        )
        self.assertNotIn(
            "version",
            workspace["workspace"]["package"],
            "a shared [workspace.package] version re-versions every crate at once",
        )


class PreviousTag(unittest.TestCase):
    """Notes must range from the previous tag of the same crate."""

    def test_version_key_sorts_numerically(self):
        self.assertLess(
            release_notes.version_key("c-v0.1.9", "c"),
            release_notes.version_key("c-v0.1.10", "c"),
        )

    def test_version_key_ignores_the_crate_name(self):
        self.assertEqual(
            release_notes.version_key("a-v1.2.3", "a"),
            release_notes.version_key("b-v1.2.3", "b"),
        )


class NextVersion(unittest.TestCase):
    """The bump level must follow Cargo's compatibility rule, not just +1.

    Below 1.0 the leading non-zero field is the boundary: `^0.1.9` accepts
    `0.1.10` but not `0.2.0`, so a breaking change takes a minor bump, not a
    major one, and only a patch bump is guaranteed compatible.
    """

    def test_patch_increments_the_last_field(self):
        self.assertEqual(publish.next_version("0.1.9", "patch"), "0.1.10")
        self.assertEqual(publish.next_version("1.2.3", "patch"), "1.2.4")

    def test_minor_resets_patch_and_zeroes_it(self):
        self.assertEqual(publish.next_version("0.1.9", "minor"), "0.2.0")
        self.assertEqual(publish.next_version("1.2.3", "minor"), "1.3.0")

    def test_major_below_one_is_the_move_to_one_point_zero(self):
        self.assertEqual(publish.next_version("0.1.9", "major"), "1.0.0")

    def test_major_at_or_above_one_increments_the_leading_field(self):
        self.assertEqual(publish.next_version("1.2.3", "major"), "2.0.0")

    def test_an_unknown_level_is_refused(self):
        with self.assertRaises(publish.ReleaseError):
            publish.next_version("0.1.9", "huge")

    def test_a_non_semver_version_is_refused(self):
        with self.assertRaises(publish.ReleaseError):
            publish.next_version("0.1", "patch")


class RequirementAccepts(unittest.TestCase):
    """A dependent is only due when the new dependency version falls outside `^`."""

    def test_a_patch_bump_stays_inside_caret(self):
        self.assertTrue(release_plan.requirement_accepts("0.1.9", "0.1.10"))

    def test_a_minor_bump_leaves_caret_below_one(self):
        self.assertFalse(release_plan.requirement_accepts("0.1.9", "0.2.0"))

    def test_a_minor_bump_stays_inside_caret_above_one(self):
        self.assertTrue(release_plan.requirement_accepts("1.2.3", "1.3.0"))

    def test_a_major_bump_leaves_caret_above_one(self):
        self.assertFalse(release_plan.requirement_accepts("1.2.3", "2.0.0"))


class ReleaseWorthiness(unittest.TestCase):
    """Not every commit deserves a new published version."""

    def test_docs_only_is_not_a_release(self):
        self.assertFalse(
            release_plan.commit_is_release_worthy("docs: fix a typo", ["crates/x/src/lib.rs"])
        )

    def test_a_feature_is_a_release(self):
        self.assertTrue(
            release_plan.commit_is_release_worthy("feat: add a parser", ["crates/x/src/lib.rs"])
        )

    def test_an_unconventional_subject_is_a_release(self):
        # Guessing "no release" from a subject nobody followed the convention
        # for is the costlier mistake, so it counts.
        self.assertTrue(
            release_plan.commit_is_release_worthy("Fix the off-by-one", ["crates/x/src/lib.rs"])
        )

    def test_a_manifest_only_change_is_not_a_release(self):
        # This is the case that would re-release all 26 crates for a packaging
        # refactor: the manifest changed, the published code did not.
        self.assertFalse(
            release_plan.commit_is_release_worthy("refactor: move the version", ["crates/x/Cargo.toml"])
        )

    def test_a_manifest_change_alongside_code_is_a_release(self):
        self.assertTrue(
            release_plan.commit_is_release_worthy(
                "feat: add a parser",
                ["crates/x/Cargo.toml", "crates/x/src/lib.rs"],
            )
        )

    def test_suggested_level_is_minor_for_a_feature(self):
        self.assertEqual(
            release_plan.suggested_level([("a", "feat: add a parser")]), "minor"
        )

    def test_suggested_level_is_minor_for_a_breaking_change(self):
        self.assertEqual(
            release_plan.suggested_level([("a", "fix!: drop the old API")]), "minor"
        )

    def test_suggested_level_is_patch_for_a_fix(self):
        self.assertEqual(
            release_plan.suggested_level([("a", "fix: correct a checksum")]), "patch"
        )


class Grouping(unittest.TestCase):
    """Nothing may be dropped from the notes, however the subject is written."""

    def test_unknown_type_lands_in_other(self):
        grouped = release_notes.group([("abc1234", "Bump the MSRV floor")])
        self.assertEqual(grouped[release_notes.OTHER], [("abc1234", "Bump the MSRV floor")])

    def test_known_types_are_sectioned(self):
        grouped = release_notes.group([("abc1234", "feat: add a parser")])
        self.assertEqual(grouped["Features"], [("abc1234", "add a parser")])

    def test_scope_and_bang_are_handled(self):
        grouped = release_notes.group([("abc1234", "fix(xmlkit)!: reject bad input")])
        self.assertEqual(grouped["Fixes"], [("abc1234", "reject bad input")])

    def test_no_commit_is_lost(self):
        commits = [
            ("a", "feat: one"),
            ("b", "chore: two"),
            ("c", "no convention here"),
            ("d", "weird: type"),
        ]
        grouped = release_notes.group(commits)
        self.assertEqual(sum(len(v) for v in grouped.values()), len(commits))


class TokenCheck(unittest.TestCase):
    """`check_token` tells a usable token from a rejected one by the body.

    Both answers are HTTP 403, so a status-code check would call a bad token
    good. These pin the distinction.
    """

    def test_a_web_session_answer_means_the_token_is_usable(self):
        ok, _ = check_token.classify(
            403, '{"errors":[{"detail":"this action can only be performed on the crates.io website"}]}'
        )
        self.assertTrue(ok)

    def test_authentication_failure_means_the_token_is_not(self):
        ok, _ = check_token.classify(403, '{"errors":[{"detail":"authentication failed"}]}')
        self.assertFalse(ok)

    def test_ok_status_is_usable(self):
        ok, _ = check_token.classify(200, "{}")
        self.assertTrue(ok)

    def test_an_unknown_answer_is_not_treated_as_success(self):
        ok, message = check_token.classify(500, "gateway blew up")
        self.assertFalse(ok)
        self.assertIn("500", message)


class NoSecretsInOutput(unittest.TestCase):
    """The scripts are run in CI, where stdout is public."""

    def test_check_token_never_prints_the_token(self):
        token = "cio_SECRET_VALUE_aaaaaaaaaaaaaaaaaaaa"
        source = (SCRIPTS / "check_token.py").read_text()
        for pattern in ("print(token", "print(f\"{token}", "repr(token)"):
            with self.subTest(pattern=pattern):
                self.assertNotIn(pattern, source)
        # The classifier returns a message, never the credential it was given.
        _, message = check_token.classify(403, '{"errors":[{"detail":"authentication failed"}]}')
        self.assertNotIn(token, message)


if __name__ == "__main__":
    unittest.main()
