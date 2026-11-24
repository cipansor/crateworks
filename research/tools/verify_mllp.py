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

# The MSA-3 text the driver builds every ACK with. It holds every delimiter on
# purpose: an interface quotes the offending value, so the reply has to escape
# what it writes. Kept in one place so the driver and the check cannot drift.
#
# Both delimiter sets appear, because a reply escapes only its own. A case using
# `#` and `*` must leave `|` and `^` alone and must escape those two instead;
# with only the default set in the text, that half of the rule was never tested.
ACK_TEXT = "rejected | value ^ ~ & \\ here # and * too"

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
    # Every separator here is `#`, including between the header values; a `|`
    # would land the control id in MSH-7 and leave MSH-10 empty.
    "MSH#*~\\&#S#F#R#F#20260101##ADT*A01#MSG0006#P#2.5.1\r"
    "PID#1##12345***MRN**DOE*JOHN\r",

    # A segment with no fields at all.
    "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|MSG0007|P|2.5.1\r"
    "ZZZ\r"
    "PID|1\r",

    # Unicode in a name, which must survive byte for byte.
    "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|MSG0008|P|2.5.1\r"
    "PID|1||1^^^MRN||أحمد^محمد\r",

    # A control id and facility names holding escaped delimiters. An `ACK`
    # echoes MSH-3..MSH-6 and MSH-10 back into its own fields, so a value
    # written unescaped splits the reply: `MSA-2` would look like two fields and
    # MSA-3 would start in the middle of the control id. The values here are the
    # inputs that would have caught that.
    #
    # The delimiters are written in escaped form on purpose. A raw `~` in MSH-5
    # would make it a repeating field, which MSH-5 is not, so the message would
    # be malformed rather than testing anything.
    "MSH|^~\\&|SEND\\F\\APP|FAC\\S\\1|RECV\\R\\APP|FAC\\T\\TWO|20260101||ADT^A01|MSG\\F\\0009|P|2.5.1\r"
    "PID|1||12345^^^MRN\r",

    # Composite header identities. MSH-3..MSH-6 are HD values and MSH-12 is a
    # VID, so their components are structure: the reply must keep
    # `LAB^1.2.3^ISO` as three components rather than escaping the separators
    # into one. MSH-4's universal id also holds a literal `^`, written `\S\`,
    # which is data and must stay data while the separators around it do not.
    "MSH|^~\\&|LAB^1.2.3^ISO|FAC^9.9\\S\\1^DNS|RECV^4.5^ISO|FAC|20260101||ADT^A01|MSG0010|P|2.5.1^ABC\r"
    "PID|1||12345^^^MRN\r",
]


def reference_parse(raw: str, required: bool = True) -> dict:
    """Parse with hl7apy and flatten to {SEG-n: text}.

    `required` says whether a parse failure is the caller's problem. A reply the
    reference rejects is a finding, not a crash, so the ACK checks pass False and
    get `None` back to report against their own field list.

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

    try:
        walk(parse_message(raw))
    except Exception:
        if required:
            raise
        return None
    return out


def rust_parse(corpus: list[str]) -> tuple[list[dict], list[str]]:
    """Compile a driver against the crate and parse the corpus with it.

    Also builds an `ACK` for every message, so the gate checks the reply the
    crate produces and not only the fields it reads.
    """
    driver = r'''
use mllp::{AckCode, AckContext, ErrorDetail, ErrorLocation, Message, Severity, build_ack, build_ack_enhanced};
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
        let context = AckContext::from_message(&message);
        let ack = build_ack(
            &context,
            AckCode::Accept,
            &format!("ACK{index:04}"),
            // The text carries every delimiter on purpose; see ACK_TEXT.
            __ACK_TEXT__,
            message.separators(),
        );
        println!("ACK\t{index}\t{}", ack.replace('\r', "\\r"));

        // The enhanced reply: an application error explained with MSA-4, ERR-2
        // (the location) and an ERR segment. The error condition is fixed so the
        // gate can name it, and the location points at PID-3, component 1.
        let error = ErrorDetail::new("207", "Application internal error", Severity::Error)
            .at(ErrorLocation::field("PID", 2, 3).with_component(1));
        let enhanced = build_ack_enhanced(
            &context,
            AckCode::Error,
            &format!("ERR{index:04}"),
            "bad patient",
            &error,
            message.separators(),
        );
        println!("ENH\t{index}\t{}", enhanced.replace('\r', "\\r"));
    }
}
'''
    tmp = Path(tempfile.mkdtemp(prefix="verify-mllp-"))
    try:
        # ACK_TEXT lives in Python so the driver and the check share it; it is
        # injected as a Rust literal so the two cannot drift apart.
        (tmp / "main.rs").write_text(
            driver.replace("__ACK_TEXT__", '"' + ACK_TEXT.replace("\\", "\\\\") + '"')
        )
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
        # Forced, not `setdefault`: an ambient CARGO_TARGET_DIR - left over from
        # a release build, say - would send the driver somewhere other than the
        # directory this script looks in, and the run would fail with a bare
        # "No such file or directory" for the binary.
        env["CARGO_TARGET_DIR"] = str(tmp / "target")
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
    acks: list[str] = ["" for _ in corpus]
    enhanced: list[str] = ["" for _ in corpus]
    for line in out.splitlines():
        parts = line.split("\t")
        if parts[0] == "ACK" and len(parts) == 3:
            acks[int(parts[1])] = parts[2].replace("\\r", "\r")
            continue
        if parts[0] == "ENH" and len(parts) == 3:
            enhanced[int(parts[1])] = parts[2].replace("\\r", "\r")
            continue
        if len(parts) != 3:
            continue
        index, reference, text = parts
        parsed[int(index)][reference] = text
    return parsed, acks, enhanced


def escape_text(text: str, field: str, encoding: str) -> str:
    """Escape `text` for one message's delimiters.

    `field` is MSH-1 and `encoding` is MSH-2, the four characters after it:
    component, repetition, escape, subcomponent. The crate applies exactly this
    rule when it writes a value, so the expectation is computed from the reply's
    own delimiters rather than assuming the default set.
    """
    component, repetition, escape, subcomponent = encoding[:4]
    replacements = {
        field: f"{escape}F{escape}",
        component: f"{escape}S{escape}",
        repetition: f"{escape}R{escape}",
        escape: f"{escape}E{escape}",
        subcomponent: f"{escape}T{escape}",
    }
    return "".join(replacements.get(c, c) for c in text)


def check_ack(raw: str, ack: str, index: int) -> int:
    """Read the crate's ACK back with hl7apy and assert the reply contract.

    hl7apy is used as the reader, so a reply the crate considers well formed is
    only accepted here if the reference parser agrees on every field an ACK is
    required to carry.
    """
    from hl7apy.parser import parse_message

    original = reference_parse(raw)
    reply = reference_parse(ack, required=False)
    if reply is None:
        print(f"  ack {index}: the reference parser rejects the reply: {ack!r}")
        return 1
    problems = 0

    def expect(reference: str, value: str, why: str) -> None:
        nonlocal problems
        # hl7apy omits an absent field rather than returning an empty string,
        # while the crate writes the empty field. Both mean "no value", so an
        # absent field is compared as empty.
        got = reply.get(reference) or ""
        if got != value:
            print(f"  ack {index}: {reference} is {got!r}, expected {value!r} ({why})")
            problems += 1

    # MSH-10 is the reply's own control id, not the one being acknowledged.
    expect("MSH-10", f"ACK{index:04}", "fresh control id")
    # MSA-1 is the acceptance code, MSA-2 echoes the original MSH-10.
    expect("MSA-1", "AA", "accept code")
    expect("MSA-2", original.get("MSH-10") or "", "echoed control id")
    # The reply travels the other way, so sender and receiver swap.
    expect("MSH-3", original.get("MSH-5") or "", "swapped sending application")
    expect("MSH-5", original.get("MSH-3") or "", "swapped receiving application")
    expect("MSH-6", original.get("MSH-4") or "", "swapped receiving facility")
    # The version is kept so both ends agree on the grammar. A message with no
    # MSH-12 is malformed; the reply still names the version it speaks.
    expect("MSH-12", original.get("MSH-12") or "2.5.1", "kept version")
    # MSA-3 is the text the crate was given, and that text holds every
    # delimiter. hl7apy returns it in escaped ER7, so this compares the wire
    # form: a value written raw would leave MSA-3 holding only the part before
    # the first `|`, and every later field would be missing or shifted. The
    # expected escaping follows the reply's own delimiter set, which is the
    # original message's: a message using `#` and `*` leaves `|` and `^` alone.
    expect(
        "MSA-3",
        escape_text(ACK_TEXT, reply.get("MSH-1") or "|", reply.get("MSH-2") or "^~\\&"),
        "escaped failure text",
    )
    return problems


def check_enhanced(raw: str, enhanced: str, index: int) -> int:
    """Read the crate's enhanced ACK back with hl7apy and assert the contract.

    The enhanced mode is the same reply plus MSA-4 and an ERR segment. hl7apy
    reads the ERR segment and its CWE component, so the check is that the
    reference parser finds the error condition where the standard puts it, not
    only that the crate wrote the text it meant to.
    """
    from hl7apy.parser import parse_message

    original = reference_parse(raw)
    reply = reference_parse(enhanced, required=False)
    if reply is None:
        print(f"  enh {index}: the reference parser rejects the reply: {enhanced!r}")
        return 1
    problems = 0

    def expect(reference: str, value: str, why: str) -> None:
        nonlocal problems
        got = reply.get(reference) or ""
        if got != value:
            print(f"  enh {index}: {reference} is {got!r}, expected {value!r} ({why})")
            problems += 1

    # The reply uses the original's delimiters, so the component separator is
    # read from MSH-2 rather than assumed to be `^`. MSH-1 is at index 3 and
    # MSH-2 begins at index 4, whatever the field separator is. Message 5 of the
    # corpus uses `*`, and the reference returns ERR-3 in that encoding.
    component = enhanced[4]

    expect("MSA-1", "AE", "error code")
    expect("MSA-2", original.get("MSH-10") or "", "echoed control id")
    # MSA-4 repeats the ERR-3.1 code, which is what ties the summary to the
    # segment that explains it.
    expect("MSA-4", "207", "error condition code")
    # ERR-2 is the location: the segment, its sequence and the field, then the
    # component. The component is ERL-5, so the repetition it skips is an empty
    # ERL-4, which is why there are two separators before the component.
    expect("ERR-2", f"PID{component}2{component}3{component}{component}1", "error location")
    expect(
        "ERR-3",
        f"207{component}Application internal error{component}HL70357",
        "error condition (CWE)",
    )
    expect("ERR-4", "E", "severity")
    # hl7apy exposes ERR-3 as a CWE: component 1 the code, 2 the text, 3 the
    # coding system. Read them individually so a shifted component is caught.
    try:
        message = parse_message(enhanced)
        err = message.err
        cwe = err.err_3
        for component, value in (
            ("CWE_1", "207"),
            ("CWE_2", "Application internal error"),
            ("CWE_3", "HL70357"),
        ):
            got = getattr(cwe, component, None)
            got = got.value if got is not None else ""
            if got != value:
                print(f"  enh {index}: ERR-3.{component} is {got!r}, expected {value!r}")
                problems += 1
        # ERR-2 is an ERL. Read its components so a location put in the wrong
        # place is caught even if the raw text happens to match.
        erl = err.err_2
        for component_name, value in (
            ("ERL_1", "PID"),
            ("ERL_2", "2"),
            ("ERL_3", "3"),
            ("ERL_5", "1"),
        ):
            got = getattr(erl, component_name, None)
            got = got.value if got is not None else ""
            if got != value:
                print(f"  enh {index}: ERR-2.{component_name} is {got!r}, expected {value!r}")
                problems += 1
    except Exception as error:  # noqa: BLE001
        print(f"  enh {index}: hl7apy could not read the ERR segment: {error}")
        problems += 1
    return problems


def main() -> int:
    try:
        from hl7apy.parser import parse_message  # noqa: F401
    except ImportError:
        print("needs: pip install hl7apy==1.3.5")
        return 2

    rust, acks, enhanced = rust_parse(CORPUS)
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

    ack_problems = 0
    for index, raw in enumerate(CORPUS):
        ack_problems += check_ack(raw, acks[index], index)
    print(f"{len(CORPUS) - ack_problems}/{len(CORPUS)} ACK replies agree with hl7apy")

    enhanced_problems = 0
    for index, raw in enumerate(CORPUS):
        enhanced_problems += check_enhanced(raw, enhanced[index], index)
    print(
        f"{len(CORPUS) - enhanced_problems}/{len(CORPUS)} enhanced ACK replies agree with hl7apy"
    )

    if mismatches or ack_problems or enhanced_problems:
        print(f"{mismatches + ack_problems + enhanced_problems} MISMATCH(ES)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
