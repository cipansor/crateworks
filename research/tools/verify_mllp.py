#!/usr/bin/env python3
"""Differential gate: the mllp HL7 v2 parser against hl7apy.

`crates/mllp` splits an HL7 v2 message into segments, fields, repetitions,
components and subcomponents. The crate's own tests assert the cases its author
thought of. This gate asserts the cases the *reference implementation* thinks
of, which is a different and much larger set: hl7apy 1.3.5 is the widely used
Python HL7 v2 parser, and where the two agree on a corpus, the crate is right by
the standard's own reading rather than by its author's.

The corpus is generated here, not copied: a table of realistic messages built
from the structures that appear in every HL7 v2 interface - a patient
administration feed, an order, an observation result, a repeating OBX, a
non-default delimiter set, and a value full of escapes. Each is parsed by both
sides and every field, component and subcomponent is compared.

    pip install hl7apy==1.3.5
    python3 research/tools/verify_mllp.py

Needs a Rust toolchain on PATH. The Rust driver is compiled in a temporary
directory, so nothing is written into the crate. Exits non-zero on any
disagreement, so it can be a CI gate.
"""

import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CRATE = ROOT / "crates" / "mllp"

# Messages that between them exercise every delimiter the parser handles.
CORPUS = [
    # A patient administration feed, the shape of the ADT^A01 every interface
    # starts with. PID-3 repeats, PID-5 has components, PID-11 has a full
    # address.
    "MSH|^~\\&|SENDING_APP|SENDING_FAC|RECV_APP|RECV_FAC|20260101120000||ADT^A01^ADT_A01|MSG0001|P|2.5.1\r"
    "EVN|A01|20260101120000\r"
    "PID|1||12345^^^MRN^MR~67890^^^SSN^SS||DOE^JOHN^A^JR^DR^^L||19800101|M|||123 MAIN ST^APT 4^SPRINGFIELD^IL^62704^USA^H\r"
    "NK1|1|DOE^JANE|SPO\r",

    # An observation result, the ORU^R01 that carries lab values. OBX repeats,
    # and OBX-5 carries the value.
    "MSH|^~\\&|LAB|FAC|EMR|FAC|20260101130000||ORU^R01^ORU_R01|MSG0002|P|2.5.1\r"
    "PID|1||98765^^^MRN||SMITH^ANNA\r"
    "OBR|1|ORD123|FIL456|CBC^COMPLETE BLOOD COUNT\r"
    "OBX|1|NM|WBC^WHITE BLOOD COUNT||7.2|10*3/uL|4.0-11.0|N|||F\r"
    "OBX|2|NM|HGB^HEMOGLOBIN||13.5|g/dL|12.0-16.0|N|||F\r"
    "OBX|3|ST|NOTE^NOTE||SEE COMMENT\r",

    # An order, the ORM^O01.
    "MSH|^~\\&|CPOE|FAC|LAB|FAC|20260101140000||ORM^O01^ORM_O01|MSG0003|P|2.5.1\r"
    "PID|1||55555^^^MRN||BROWN^CARL\r"
    "ORC|NW|ORD789|||||||20260101140000|||1234^ORDERING^DOC\r"
    "OBR|1|ORD789|FIL999|CMP^COMPREHENSIVE METABOLIC PANEL\r",

    # A value full of delimiter escapes: \F\ \S\ \R\ \T\ \E\.
    "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|MSG0004|P|2.5.1\r"
    "PID|1||A\\F\\B\\S\\C\\R\\D\\T\\E\\E\r",

    # Subcomponents inside a component: the HD and CE structures use them.
    "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|MSG0005|P|2.5.1\r"
    "PID|1||12345^^^MRN&AUTH&ISO^MR||DOE^JOHN\r"
    "PV1|1|I|WARD^ROOM^BED^FACILITY&1.2.3.4&ISO\r",

    # A non-default delimiter set, which is legal and appears in the wild.
    "MSH#*~\\&#S|F|R|F|20260101###ADT*A01#MSG0006#P#2.5.1\r"
    "PID#1##12345***MRN**DOE*JOHN\r",

    # A segment with no fields at all.
    "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|MSG0007|P|2.5.1\r"
    "ZZZ\r"
    "PID|1\r",

    # Unicode in a name, which must survive byte for byte.
    "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|MSG0008|P|2.5.1\r"
    "PID|1||1^^^MRN||أحمد^محمد\r",
]


def reference_parse(raw: str) -> dict:
    """Parse with hl7apy and flatten to {SEG-n: text}.

    hl7apy nests segments inside message-type groups, so the walk is recursive.
    It also splits a repeating field into one child per repetition, so those are
    joined back with `~` to match the single field the crate returns.
    """
    from hl7apy.parser import parse_message

    out: dict[str, str] = {}

    def walk(node) -> None:
        for child in node.children:
            if child.name in ("Segment",) or type(child).__name__ == "Segment":
                seen: dict[str, list[str]] = {}
                for field in child.children:
                    reference = field.name.replace("_", "-", 1)
                    seen.setdefault(reference, []).append(field.to_er7())
                for reference, values in seen.items():
                    out[reference] = "~".join(values)
            else:
                walk(child)

    walk(parse_message(raw))
    return out


def rust_parse(corpus: list[str]) -> list[dict]:
    """Compile a driver against the crate and parse the corpus with it."""
    driver = r'''
use mllp::Message;
use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    for (index, raw) in input.split('\n').enumerate() {
        if raw.is_empty() {
            continue;
        }
        let message = Message::parse(raw).expect("parse");
        for segment in message.segments() {
            let name = segment.name();
            for (position, field) in segment.fields().iter().enumerate() {
                // Field n is at index n - 1 in every segment, MSH included:
                // MSH-1 is the separator at index 0.
                let number = position + 1;
                println!("{index}\t{name}-{number}\t{}", field.text());
            }
        }
    }
}
'''
    tmp = Path(tempfile.mkdtemp(prefix="verify-mllp-"))
    try:
        (tmp / "main.rs").write_text(driver)
        (tmp / "Cargo.toml").write_text(
            "[package]\n"
            'name = "verify-mllp"\n'
            'version = "0.0.0"\n'
            'edition = "2021"\n\n'
            "[dependencies]\n"
            f'mllp = {{ path = "{CRATE}" }}\n\n'
            "[[bin]]\n"
            'name = "driver"\n'
            'path = "main.rs"\n'
        )
        env = dict(os.environ)
        env.setdefault("CARGO_TARGET_DIR", str(tmp / "target"))
        # cwd stays at the repository root so rustup picks up rust-toolchain.toml;
        # the throwaway manifest is addressed with --manifest-path instead.
        build = subprocess.run(
            ["cargo", "build", "--quiet", "--manifest-path", str(tmp / "Cargo.toml")],
            capture_output=True, text=True, env=env, cwd=ROOT,
        )
        if build.returncode != 0:
            sys.stderr.write(build.stderr)
            raise SystemExit("the driver failed to build")
        out = subprocess.run(
            [str(tmp / "target" / "debug" / "driver")],
            input="\n".join(corpus),
            capture_output=True, text=True, check=True, env=env,
        ).stdout
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    parsed: list[dict[str, str]] = [{} for _ in corpus]
    for line in out.splitlines():
        parts = line.split("\t")
        if len(parts) != 3:
            continue
        index, reference, text = parts
        parsed[int(index)][reference] = text
    return parsed


def main() -> int:
    try:
        from hl7apy.parser import parse_message  # noqa: F401
    except ImportError:
        print("needs: pip install hl7apy==1.3.5")
        return 2

    rust = rust_parse(CORPUS)
    mismatches = 0
    compared = 0

    for index, raw in enumerate(CORPUS):
        expected = reference_parse(raw)
        actual = rust[index]
        for reference, value in sorted(expected.items()):
            compared += 1
            got = actual.get(reference)
            if got is None:
                print(f"  msg {index}: {reference} missing from the crate")
                mismatches += 1
            elif got != value:
                print(f"  msg {index}: {reference}")
                print(f"    reference {value!r}")
                print(f"    crate     {got!r}")
                mismatches += 1

    print(f"\n{compared - mismatches}/{compared} fields agree with hl7apy")
    if mismatches:
        print(f"{mismatches} MISMATCH(ES)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
