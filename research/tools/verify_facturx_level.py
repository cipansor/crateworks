#!/usr/bin/env python3
"""Gate the factur-x profile reader against the reference package.

The crate's `profile_of` reads the specification identifier (BT-24) and returns
the profile it names; the reference's `get_facturx_level` / `get_level` does the
same. This script builds a corpus of documents - the crate's own generated
documents, the reference's generated documents, and hand-written documents that
put each known URN and a few malformed ones into each syntax - asks the
reference for the level, and checks the crate agrees.

Usage:
    pip install factur-x
    python3 research/tools/verify_facturx_level.py

The corpus is staged and swapped in at the end, so a run that fails part-way -
no `factur-x`, a reference change, Ctrl-C - leaves the previous fixtures intact
rather than deleted.

It writes the corpus and the expected levels to
`crates/factur-x/tests/fixtures/profile.tsv` and
`crates/factur-x/tests/fixtures/profile/<digest>/`, so the Rust test can replay
them without the reference installed. The reference is never a dependency of the
crate and is never installed by CI.

Publication replaces only the table, by one atomic rename, and gives each corpus
a directory named after its own content. A directory with that name is reused only
after every file in it is compared byte for byte against what this run staged, so
a file a maintainer deleted or edited is republished rather than silently kept.
Files a maintainer keeps beside the documents are left alone: a corpus directory
is only reused, never replaced or removed, and only when it carries the
generator's marker file.

Earlier corpora are kept. The replay test embeds the table at compile time but
opens the documents at run time, so removing a directory an already-built test
binary still names would break that binary.
"""

import hashlib
import os
import pathlib
import shutil
import subprocess
import sys

from lxml import etree

import facturx.facturx as f

ROOT = pathlib.Path(__file__).resolve().parents[2]
CRATE = ROOT / "crates" / "factur-x"
FIXTURES = CRATE / "tests" / "fixtures"
CORPUS_ROOT = FIXTURES / "profile"
MARKER = ".generated"
MARKER_TEXT = (
    "Every file in this directory is written by "
    "research/tools/verify_facturx_level.py and is replaced when it runs.\n"
)

CII = "urn:un:unece:uncefact:data:standard:CrossIndustryInvoice:100"
RAM = "urn:un:unece:uncefact:data:standard:ReusableAggregateBusinessInformationEntity:100"
UBL = "urn:oasis:names:specification:ubl:schema:xsd:Invoice-2"
CBC = "urn:oasis:names:specification:ubl:schema:xsd:CommonBasicComponents-2"
UBL_CN = "urn:oasis:names:specification:ubl:schema:xsd:CreditNote-2"

# Every URN the reference's tables recognise, plus the qualified spellings and
# a few that name no level (the reference raises on those, and so does the
# crate; they are recorded so the error path is checked too).
URNS = [
    "urn:factur-x.eu:1p0:minimum",
    "urn:factur-x.eu:1p0:basicwl",
    "urn:factur-x.eu:1p0:basic",
    "urn:factur-x.eu:1p0:en16931",
    "urn:factur-x.eu:1p0:extended",
    "urn:factur-x.eu:1p0:extended-ctc-fr",
    "urn:cen.eu:en16931:2017",
    "urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:basic",
    "urn:cen.eu:en16931:2017#conformant#urn:factur-x.eu:1p0:extended",
    "urn:cen.eu:en16931:2017#conformant#urn.cpro.gouv.fr:1p0:extended-ctc-fr",
    "urn:order-x.eu:1p0:basic",
    "urn:order-x.eu:1p0:comfort",
    "urn:order-x.eu:1p0:extended",
    "urn:factur-x.eu:1p0:unknownthing",
    "garbage",
    "urn:cen.eu:en16931:2017#compliant#urn:factur-x.eu:1p0:not-a-level",
]


def cii_document(guideline: str) -> str:
    return (
        f'<rsm:CrossIndustryInvoice xmlns:rsm="{CII}" xmlns:ram="{RAM}">'
        "<rsm:ExchangedDocumentContext>"
        "<ram:GuidelineSpecifiedDocumentContextParameter>"
        f"<ram:ID>{guideline}</ram:ID>"
        "</ram:GuidelineSpecifiedDocumentContextParameter>"
        "</rsm:ExchangedDocumentContext>"
        "</rsm:CrossIndustryInvoice>"
    )


def ubl_document(guideline: str, root: str = "Invoice", ns: str = UBL) -> str:
    return (
        f'<{root} xmlns="{ns}" xmlns:cbc="{CBC}">'
        f"<cbc:CustomizationID>{guideline}</cbc:CustomizationID>"
        f"</{root}>"
    )


def cases() -> list[tuple[str, str, str]]:
    """(name, syntax, xml). The syntax is `cii` or `ubl`."""
    out = []
    for i, urn in enumerate(URNS):
        out.append((f"urn{i:02d}_cii", "cii", cii_document(urn)))
        out.append((f"urn{i:02d}_ubl", "ubl", ubl_document(urn)))
    # A credit note carries `CustomizationID` too and must read the same way.
    out.append(("creditnote_ubl", "ubl", ubl_document("urn:cen.eu:en16931:2017", "CreditNote", UBL_CN)))
    return out


def reference_level(xml: str) -> str | None:
    """The reference's answer, or None when it raises (the error path)."""
    root = etree.fromstring(xml.encode())
    try:
        return f.get_level(root)
    except Exception:
        return None


def corpus_digest(documents: list[tuple[str, str, str]]) -> str:
    """A short name for this corpus, so a new one never reuses an old path."""
    digest = hashlib.sha256()
    for name, _syntax, xml in documents:
        digest.update(name.encode())
        digest.update(b"\0")
        digest.update(xml.encode())
        digest.update(b"\0")
    return digest.hexdigest()[:16]


def owned_by_generator(directory: pathlib.Path) -> bool:
    """Whether this script made `directory`, and may therefore reuse it.

    The marker is written into every corpus this script produces. A directory
    without one was not made here - a maintainer's notes directory, say - so it
    is left alone rather than reused.
    """
    return (directory / MARKER).is_file()


def holds_exactly(directory: pathlib.Path, documents: list[tuple[str, str, str]]) -> bool:
    """Whether `directory` already holds these documents, byte for byte.

    The directory name is a digest of what the generator produced, not of what is
    on disk, so a directory can carry the right name and still be wrong: a
    maintainer may have deleted a file to see it regenerated, or edited one while
    investigating. Comparing only names would keep the damaged copy and then read
    it against freshly computed expectations.
    """
    if not owned_by_generator(directory):
        return False
    expected = {f"{name}.xml": xml for name, _syntax, xml in documents}
    present = {entry.name for entry in directory.iterdir() if entry.is_file()}
    if present != set(expected) | {MARKER}:
        return False
    return all((directory / name).read_text() == xml for name, xml in expected.items())


def choose_corpus(
    root: pathlib.Path, digest: str, documents: list[tuple[str, str, str]]
) -> tuple[pathlib.Path, bool]:
    """Where to publish, and whether that directory is already correct.

    The content-named directory is reused only when it is ours and already holds
    these exact bytes. Otherwise the first free name is taken, because replacing a
    damaged directory in place would delete documents while the live table still
    names them, and a directory this script did not make must not be replaced at
    all. A suffixed name still records the content digest.
    """
    primary = root / digest
    if holds_exactly(primary, documents):
        return primary, True
    candidate, suffix = primary, 1
    while candidate.exists():
        suffix += 1
        candidate = root / f"{digest}-{suffix}"
    return candidate, False


def main() -> int:
    if not (CRATE / "Cargo.toml").is_file():
        print(f"cannot find the crate at {CRATE}", file=sys.stderr)
        return 2

    work = FIXTURES / ".profile-staging"
    if work.exists():
        shutil.rmtree(work)
    work.mkdir(parents=True)

    documents = list(cases())
    digest = corpus_digest(documents)
    CORPUS_ROOT.mkdir(parents=True, exist_ok=True)
    target, reusable = choose_corpus(CORPUS_ROOT, digest, documents)
    corpus = target.name
    rows: list[tuple[str, str, str, str]] = []
    try:
        for name, syntax, xml in documents:
            (work / f"{name}.xml").write_text(xml)
            rows.append((name, syntax, f"{corpus}/{name}.xml", reference_level(xml) or ""))

        lines = ["# Generated by research/tools/verify_facturx_level.py. Do not edit.",
                 "# name\tsyntax\tfile\tlevel"]
        for name, syntax, file, level in rows:
            lines.append(f"{name}\t{syntax}\t{file}\t{level}")
        (work / MARKER).write_text(MARKER_TEXT)
        (work / "profile.tsv").write_text("\n".join(lines) + "\n")
    except BaseException:
        # Nothing has been published, so the live fixtures are untouched.
        shutil.rmtree(work, ignore_errors=True)
        raise

    staged_table = FIXTURES / ".profile-staging.tsv"
    try:
        # The documents and the table are staged together but published
        # separately, because the table sits outside the corpus directory. Take
        # the table out first, so moving the rest leaves exactly the corpus.
        os.replace(work / "profile.tsv", staged_table)

        # Documents first, table second. Before the second rename the old corpus is
        # intact and the live table still names it; after it the new corpus is
        # complete and the table names that. No ordering leaves the table naming
        # documents that are not there. A directory that already holds these bytes
        # is left as it is, so an unchanged corpus does not churn the tree.
        if not reusable:
            os.replace(work, target)
        os.replace(staged_table, FIXTURES / "profile.tsv")
    finally:
        # An interrupt here is survivable - the live fixture is consistent - but
        # it must not leave a staging tree for the next run to trip over.
        shutil.rmtree(work, ignore_errors=True)
        staged_table.unlink(missing_ok=True)

    print(f"{len(rows)} cases -> {FIXTURES / 'profile.tsv'} (corpus {corpus})")
    ok = sum(1 for _, _, _, level in rows if level)
    print(f"  {ok} name a level, {len(rows) - ok} are the error path")

    # Drive the crate over the same corpus and compare, so a difference is
    # reported here as well as by the Rust test.
    example = CRATE / "examples" / "_oracle_profile.rs"
    example.parent.mkdir(exist_ok=True)
    example.write_text(EXAMPLE)
    env = dict(os.environ)
    env["PATH"] = os.environ.get("PATH", "")
    # Documents live under `<digest>/`, so the crate reads the same relative path
    # the table names rather than assuming a flat directory.
    files = [file for _, _, file, _ in rows]
    proc = subprocess.run(
        ["cargo", "run", "--quiet", "--example", "_oracle_profile", "--", *files],
        cwd=CRATE, capture_output=True, text=True, env=env,
    )
    if proc.returncode != 0:
        print(proc.stderr[-3000:], file=sys.stderr)
        return 2
    ours = {}
    for line in proc.stdout.splitlines():
        if "\t" in line:
            n, level = line.split("\t", 1)
            ours[n] = level

    mismatches = 0
    for name, _, _, level in rows:
        if ours.get(name, "<missing>") != level:
            mismatches += 1
            print(f"MISMATCH {name}: reference={level!r} crate={ours.get(name)!r}")
    print(f"\n{len(rows) - mismatches}/{len(rows)} agree with the reference")
    example.unlink()
    return 1 if mismatches else 0


EXAMPLE = """//! Generated by research/tools/verify_facturx_level.py. Do not edit.
//! Reads one document per argument from tests/fixtures/profile/ and prints
//! `name<TAB>level`, or `name<TAB>` when the crate reports an error.
use std::path::PathBuf;

fn main() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/profile");
    for file in std::env::args().skip(1) {
        let name = PathBuf::from(&file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap()
            .to_string();
        let xml = std::fs::read_to_string(dir.join(&file)).unwrap();
        match factur_x::profile_of(&xml) {
            Ok(p) => println!("{name}\\t{p}"),
            Err(_) => println!("{name}\\t"),
        }
    }
}
"""


if __name__ == "__main__":
    raise SystemExit(main())
