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

# The delimiter pair a converted reply uses. The gate builds a second ACK with
# these in place of the incoming field and component separators, so a conversion
# is checked by the reference parser and not only by the crate's own reader.
# The pair that a source already uses is skipped, so the conversion is real.
ALTERNATE_PAIRS = [("#", "*"), ("@", "!"), (";", "%")]


def converted_pair(raw: str) -> tuple[str, str]:
    """The alternate (field, component) pair to answer `raw` with.

    A pair is usable when neither character is already one of the source's five
    delimiters; answering with a separator the source also uses would leave that
    axis unconverted and test nothing.
    """
    field, encoding = raw[3], raw[4:8]
    used = {field, *encoding[:4]}
    for pair in ALTERNATE_PAIRS:
        if not used & set(pair):
            return pair
    raise SystemExit(f"no alternate delimiter pair left for {raw!r}")


def field_only_pair(raw: str) -> tuple[str, str]:
    """A pair that changes only the field separator.

    The component separator is the source's own, so a delimiter inside an opaque
    body keeps its component boundary and the reply must be accepted. Replacing
    both, as `converted_pair` does, never exercises that case - and a rule that
    refuses it would look correct against this gate.
    """
    field, encoding = raw[3], raw[4:8]
    used = {field, *encoding[:4]}
    in_bodies = {
        character
        for reference in ("MSH-3", "MSH-4", "MSH-5", "MSH-6")
        for body in _opaque_bodies_per_component(raw, reference)
        for character in body
    }
    for candidate, _ in ALTERNATE_PAIRS:
        # A candidate inside an opaque body would collide for real, so the reply
        # could not carry it; that is a conversion refusal, not the case this
        # pair is for.
        if candidate not in used and candidate not in in_bodies:
            return candidate, encoding[0]
    raise SystemExit(f"no field-only separator left for {raw!r}")

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

    # The documented fallback: an opaque escape body holding the *source's own*
    # component separator. Legal - the parser accepts it - so the reply built
    # with the source's separators must exist. A refusal here would leave a
    # receiver unable to acknowledge a message it could parse.
    "MSH|#~\\&|LAB\\ZID#1\\|F|R|F|20260101||ADT#A01|MSG0012|P|2.5.1\r"
    "PID|1||12345^^^MRN\r",

    # A site-defined escape whose body holds `#`. The reply built with the
    # original's separators carries it untouched, but the converted reply below
    # uses `#` as its field separator and cannot: the reader would split MSH-5
    # there. The crate must refuse that reply rather than shift the control id.
    "MSH|^~\\&|S\\ZLAB#1\\|F|R|F|20260101||ADT^A01|MSG0011|P|2.5.1\r"
    "PID|1||12345^^^MRN\r",

    # The escape character itself changes while the component separator stays.
    # `LAB%ZID^1%` is two components whose `%` markers are stranded, because a
    # reader splits before it resolves escapes. Rewriting those markers as
    # backslashes would change the identity a peer decodes.
    "MSH|^~%&|LAB%ZID^1%|F|R|F|20260101||ADT^A01|MSG0013|P|2.5.1\r"
    "PID|1||12345^^^MRN\r",

    # An opaque body holding the subcomponent separator, which a field-only reply
    # keeps. This is the case that separates a rule keyed on *presence* from one
    # keyed on *role*: `&` is in both delimiter sets, so a presence rule refuses,
    # while its role is unchanged and the boundaries do not move, so the reply
    # carries the body exactly. A body can never hold the component separator -
    # that splits the component before any reader sees a body.
    "MSH|^~\\&|LAB\\ZID&1\\|F|R|F|20260101||ADT^A01|MSG0014|P|2.5.1\r"
    "PID|1||12345^^^MRN\r",

    # A marker with no closer inside its component: `LAB\ZID&1` is component 1
    # and `2\` is component 2. The marker is data, but the `&` after it still
    # separates subcomponents, so a reply that escapes the whole tail as text
    # merges two subcomponents into one and still parses.
    "MSH|^~\\&|LAB\\ZID&1^2\\|F|R|F|20260101||ADT^A01|MSG0015|P|2.5.1\r"
    "PID|1||12345^^^MRN\r",

    # A literal `&` beside a structural one, in the same component. A reply that
    # moved the boundary - `LAB&X\\T\\Y` for `LAB\\T\\X&Y` - would keep the
    # subcomponent count and the decoded text, so only a per-subcomponent split
    # shows it.
    "MSH|^~\\&|LAB\\T\\X&Y|F|R|F|20260101||ADT^A01|MSG0016|P|2.5.1\r"
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


def rust_parse(corpus: list[str]) -> tuple[list[dict], list[str], list[str], list[str], dict[int, str]]:
    """Compile a driver against the crate and parse the corpus with it.

    Also builds an `ACK` for every message, so the gate checks the reply the
    crate produces and not only the fields it reads.
    """
    driver = r'''
use mllp::{AckCode, AckContext, ErrorDetail, ErrorLocation, Message, Severity, Separators, build_ack, build_ack_enhanced};
use std::io::Read;

/// The (field, component) pair each message is answered with, injected by the
/// gate so the check and the driver cannot pick different pairs.
const CONVERSIONS: [(char, char); __CONV_COUNT__] = [__CONV_PAIRS__];
/// Field-only pairs: the source's own component separator is kept, so an opaque
/// body keeps its boundaries and the reply must never be refused.
const FIELD_ONLY: [(char, char); __CONV_COUNT__] = [__FIELD_ONLY_PAIRS__];

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
        )
        // Answering in the source's own delimiter set never has to convert, so
        // it must never refuse: a refusal here is a finding of its own.
        .expect("a reply in the source's separators cannot refuse");
        println!("ACK\t{index}\t{}", ack.replace('\r', "\\r"));

        // The same reply in a different delimiter set. This is the case the
        // oracle had no coverage for: the composite identities above must still
        // read as the same three components after the conversion. A body the
        // alternate set cannot carry makes the builder refuse, which is a
        // result the gate checks for rather than a crash.
        let mut converted = Separators::default();
        converted.field = CONVERSIONS[index].0;
        converted.component = CONVERSIONS[index].1;
        match build_ack(
            &context,
            AckCode::Accept,
            &format!("CNV{index:04}"),
            __ACK_TEXT__,
            converted,
        ) {
            Ok(reply) => println!("CNV\t{index}\t{}", reply.replace('\r', "\\r")),
            Err(error) => println!("CNVERR\t{index}\t{error}"),
        }

        // Only the field separator changes. Every opaque body keeps its component
        // boundary, so this reply must exist for every fixture.
        let mut field_only = message.separators();
        field_only.field = FIELD_ONLY[index].0;
        match build_ack(
            &context,
            AckCode::Accept,
            &format!("FLD{index:04}"),
            __ACK_TEXT__,
            field_only,
        ) {
            Ok(reply) => println!("FLD\t{index}\t{}", reply.replace('\r', "\\r")),
            Err(error) => println!("FLDNONE\t{index}\t{error}"),
        }

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
        )
        .expect("a reply in the source's separators cannot refuse");
        println!("ENH\t{index}\t{}", enhanced.replace('\r', "\\r"));
    }
}
'''
    tmp = Path(tempfile.mkdtemp(prefix="verify-mllp-"))
    try:
        # ACK_TEXT lives in Python so the driver and the check share it; it is
        # injected as a Rust literal so the two cannot drift apart.
        pairs = ", ".join(
            "('" + converted_pair(raw)[0] + "', '" + converted_pair(raw)[1] + "')"
            for raw in corpus
        )
        field_only_pairs = ", ".join(
            "('" + field_only_pair(raw)[0] + "', '" + field_only_pair(raw)[1] + "')"
            for raw in corpus
        )
        (tmp / "main.rs").write_text(
            driver.replace("__ACK_TEXT__", '"' + ACK_TEXT.replace("\\", "\\\\") + '"')
            .replace("__CONV_COUNT__", str(len(corpus)))
            .replace("__CONV_PAIRS__", pairs)
            .replace("__FIELD_ONLY_PAIRS__", field_only_pairs)
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
        run = subprocess.run(
            [str(tmp / "target" / "debug" / "driver")],
            input="\n".join(corpus),
            capture_output=True, text=True, env=env,
        )
        if run.returncode != 0:
            # A driver that panics is a failed gate, not a crashed script: the
            # panic message names the case, and a traceback here would hide it.
            sys.stderr.write(run.stderr)
            raise SystemExit(
                f"the driver exited {run.returncode}; the first failing case is "
                "named in the panic above"
            )
        out = run.stdout
    finally:
        shutil.rmtree(tmp, ignore_errors=True)

    parsed: list[dict[str, str]] = [{} for _ in corpus]
    acks: list[str] = ["" for _ in corpus]
    enhanced: list[str] = ["" for _ in corpus]
    converted: list[str] = ["" for _ in corpus]
    field_only: list[str] = ["" for _ in corpus]
    field_only_refusals: dict[int, str] = {}
    refusals: dict[int, str] = {}
    for line in out.splitlines():
        parts = line.split("\t")
        if parts[0] == "CNV" and len(parts) == 3:
            converted[int(parts[1])] = parts[2].replace("\\r", "\r")
            continue
        if parts[0] == "CNVERR" and len(parts) == 3:
            refusals[int(parts[1])] = parts[2]
            continue
        if parts[0] == "ACK" and len(parts) == 3:
            acks[int(parts[1])] = parts[2].replace("\\r", "\r")
            continue
        if parts[0] == "ENH" and len(parts) == 3:
            enhanced[int(parts[1])] = parts[2].replace("\\r", "\r")
            continue
        if parts[0] == "FLD" and len(parts) == 3:
            field_only[int(parts[1])] = parts[2].replace("\\r", "\r")
            continue
        if parts[0] == "FLDNONE" and len(parts) == 3:
            field_only_refusals[int(parts[1])] = parts[2]
            continue
        if len(parts) != 3:
            continue
        index, reference, text = parts
        parsed[int(index)][reference] = text
    return parsed, acks, enhanced, converted, refusals, field_only, field_only_refusals


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
    # Every finding is printed, but the reply counts as one failure.
    failed = False

    def expect(reference: str, value: str, why: str) -> None:
        nonlocal failed
        # hl7apy omits an absent field rather than returning an empty string,
        # while the crate writes the empty field. Both mean "no value", so an
        # absent field is compared as empty.
        got = reply.get(reference) or ""
        if got != value:
            print(f"  ack {index}: {reference} is {got!r}, expected {value!r} ({why})")
            failed = True

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
    return int(failed)


def _conversion_must_refuse(raw: str) -> bool:
    """Whether no reply can carry this header under the chosen reply pair.

    True when a site-defined escape body in an HD field the reply echoes holds a
    character whose *role* differs between the two messages. A body that straddles
    a component boundary is not a body at all - a reader splits first - so the
    bodies are found per component, exactly as the crate reads them.
    """
    source = _separators_of(raw)
    field, component = converted_pair(raw)
    reply = {
        "field": field,
        "component": component,
        "repetition": source["repetition"],
        "escape": source["escape"],
        "subcomponent": source["subcomponent"],
    }
    return any(
        _role(character, source) != _role(character, reply)
        for reference in ("MSH-3", "MSH-4", "MSH-5", "MSH-6")
        for body in _opaque_bodies_per_component(raw, reference)
        for character in body
    )




def _shape(value: str, separators: dict[str, str]) -> list[list[str]]:
    """`value` as its components, each split into decoded subcomponents.

    A count is not enough. `LAB\\T\\X&Y` and `LAB&X\\T\\Y` each hold one raw
    `&`, so both have two subcomponents, and both decode to `LAB&X&Y` - a literal
    `&` trading places with a structural one is invisible to a count and to a
    decoded component string. Splitting on the raw separator and decoding each
    part keeps every boundary where the reader put it, so a moved separator shows
    up as a different subcomponent list.
    """
    component = separators["component"]
    subcomponent = separators["subcomponent"]
    return [
        [
            _decode(part, separators)
            for part in raw_component.split(subcomponent)
        ]
        for raw_component in value.split(component)
    ]


def _decode(text: str, separators: dict[str, str]) -> str:
    """Resolve the five standard escapes in one subcomponent."""
    escape = separators["escape"]
    named = {
        "F": separators["field"],
        "S": separators["component"],
        "R": separators["repetition"],
        "E": escape,
        "T": separators["subcomponent"],
    }
    out = []
    i = 0
    while i < len(text):
        if text[i] != escape:
            out.append(text[i])
            i += 1
            continue
        close = text.find(escape, i + 1)
        if close < 0:
            out.append(text[i:])
            break
        body = text[i + 1 : close]
        # An unknown escape is site-defined and kept verbatim.
        out.append(named.get(body, f"{escape}{body}{escape}"))
        i = close + 1
    return "".join(out)



def _opaque_bodies_per_component(raw: str, reference: str) -> list[str]:
    """The opaque escape bodies in one echoed MSH field, per component.

    A reader splits on the component separator before it resolves escapes, so a
    pair of escape characters straddling one is a stranded marker, not a body.
    Searching the whole field instead invents bodies that no reader sees.
    """
    component = _separators_of(raw)["component"]
    escape = _separators_of(raw)["escape"]
    bodies = []
    for part in _raw_msh_field(raw, reference).split(component):
        bodies.extend(_opaque_bodies(part, escape))
    return bodies


def _role(character: str, separators: dict[str, str]) -> str:
    """The role a character plays in one delimiter set, mirroring the crate."""
    for role, value in separators.items():
        if character == value:
            return role
    return "data"


def _raw_msh_field(raw: str, reference: str) -> str:
    """One MSH field of a message as it appears on the wire."""
    number = int(reference.split("-")[1])
    segments = raw.split("\r")
    header = segments[0]
    separator = header[3]
    # MSH-1 is the separator itself, so the fields start at index 1 after the name.
    fields = header[4:].split(separator)
    index = number - 2
    return fields[index] if 0 <= index < len(fields) else ""


def _opaque_bodies(value: str, escape: str) -> list[str]:
    """The bodies of the escapes in `value` that this crate cannot rewrite."""
    standard = {"F", "S", "R", "E", "T"}
    bodies = []
    parts = value.split(escape)
    # Odd indices are between a pair of escape characters.
    for part in parts[1::2]:
        if part and part not in standard:
            bodies.append(part)
    return bodies


def check_field_only(raw: str, reply_ack: str, index: int) -> int:
    """Check a reply that changed only the field separator.

    The component separator is the source's own, so an opaque body keeps its
    component boundaries and every identity must survive untouched. This is the
    case a rule keyed on delimiter *presence* refuses and a rule keyed on *role*
    accepts, so it is the case the gate must exercise: without it, a rule that
    rejected every opaque body would look correct.
    """
    from hl7apy.parser import parse_message

    if not reply_ack:
        print(f"  fld {index}: no field-only reply was produced for this fixture")
        return 1
    field, component = field_only_pair(raw)
    if reply_ack[3] != field:
        print(f"  fld {index}: reply uses field {reply_ack[3]!r}, expected {field!r}")
        return 1
    if reply_ack[4] != component:
        print(
            f"  fld {index}: reply changed the component to {reply_ack[4]!r}, "
            f"expected the source's {component!r} - that is a conversion, not "
            "a field-only change"
        )
        return 1

    try:
        source = parse_message(raw)
        reply = parse_message(reply_ack)
    except Exception as error:
        print(f"  fld {index}: a parser rejects the message: {error}")
        return 1

    source_sep = _separators_of(raw)
    reply_sep = _separators_of(reply_ack)
    # Every finding is printed, but the reply counts as one failure: the summary
    # subtracts this from the number of replies.
    failed = False
    for original, mirrored in (
        ("MSH-5", "MSH-3"),
        ("MSH-6", "MSH-4"),
        ("MSH-3", "MSH-5"),
        ("MSH-4", "MSH-6"),
    ):
        expected = _identity(_msh_field(source, original), source_sep)
        actual = _identity(_msh_field(reply, mirrored), reply_sep)
        if expected != actual:
            print(
                f"  fld {index}: {mirrored} is {actual!r}, expected {expected!r}"
            )
            failed = True
        # Decoded text alone cannot see a lost boundary: escaping a separator as
        # `\T\` still decodes to `&`, so the subcomponent count has to be
        # compared from the split a reader performs.
        expected_subs = _shape(_msh_field(source, original), source_sep)
        actual_subs = _shape(_msh_field(reply, mirrored), reply_sep)
        if expected_subs != actual_subs:
            print(
                f"  fld {index}: {mirrored} splits into subcomponents "
                f"{actual_subs!r}, expected {expected_subs!r}"
            )
            failed = True
    control = _identity(_msh_field(reply, "MSH-10"), reply_sep)
    if control != [f"FLD{index:04}"]:
        print(f"  fld {index}: MSH-10 is {control!r}, expected 'FLD{index:04}'")
        failed = True

    # MSA-2 must echo the *original* control id, which is what the sender matches
    # the reply against. Checking only the reply's own MSH-10 would let a reply
    # carrying the wrong original id pass.
    # `MSA-2` is absent from the mapping when the segment omits it, and indexing
    # it directly raised KeyError, which aborted the whole run instead of naming
    # this fixture and checking the rest.
    parsed_reply = reference_parse(reply_ack, required=False) or {}
    if "MSA-2" not in parsed_reply:
        print(f"  fld {index}: the reply has no MSA-2, so it echoes no original id")
        return 1
    original = _identity(parsed_reply["MSA-2"], reply_sep)
    expected_original = _identity(_msh_field(source, "MSH-10"), source_sep)
    if original != expected_original:
        print(
            f"  fld {index}: MSA-2 is {original!r}, expected the original's "
            f"MSH-10 {expected_original!r}"
        )
        failed = True
    return int(failed)


def check_converted(raw: str, reply_ack: str, refusal: str, index: int) -> int:
    """Check the reply built with a different delimiter pair than the source.

    The crate's own reader agrees with itself too easily here, so the comparison
    goes through hl7apy on both sides: the components the reference reads in the
    source must be the components it reads in the converted reply. That is what a
    peer at the other end does, and what this gate never asked before - it
    answered every message in its own delimiter set, so a conversion that
    flattened an HD identity passed.
    """
    from hl7apy.parser import parse_message

    field, component = converted_pair(raw)
    expects_refusal = _conversion_must_refuse(raw)

    if refusal:
        # A refusal is only correct where a delimiter of the *chosen reply pair*
        # sits inside an opaque escape body. Accepting any refusal whose text
        # looks right would let the gate report full agreement while parsing no
        # converted reply at all, so the expectation is derived from the fixture
        # rather than from the message.
        if not expects_refusal:
            print(
                f"  cnv {index}: refused, but this fixture can be converted "
                f"to {field!r}/{component!r}: {refusal!r}"
            )
            return 1
        if "site-defined escape" in refusal and "original's separators" in refusal:
            return 0
        print(f"  cnv {index}: refused for an unexpected reason: {refusal!r}")
        return 1

    if expects_refusal:
        print(
            f"  cnv {index}: expected a refusal - an opaque escape body holds a "
            f"{field!r} or {component!r} delimiter - but a reply was produced"
        )
        return 1

    # Guard before indexing: an omitted reply would otherwise raise IndexError
    # from inside this check, which aborts the gate with a traceback that never
    # names the fixture. That is the failure a reader most needs named.
    if not reply_ack:
        print(f"  cnv {index}: no converted reply was produced for this fixture")
        return 1

    if reply_ack[3] != field or reply_ack[4] != component:
        # A reply that kept the source's delimiters would agree trivially: the
        # identity comparison below would pass without a byte being converted.
        print(
            f"  cnv {index}: reply uses {reply_ack[3]!r}/{reply_ack[4]!r}, "
            f"expected {field!r}/{component!r} - nothing was converted"
        )
        return 1

    try:
        source = parse_message(raw)
        reply = parse_message(reply_ack)
    except Exception as error:
        print(f"  cnv {index}: a parser rejects the message: {error}")
        return 1

    # Every disagreement is printed, but a reply counts as one failure however
    # many of its fields are wrong: the summary subtracts this from the number of
    # replies, so counting each finding would understate how many passed.
    failed = False
    source_sep = _separators_of(raw)
    reply_sep = _separators_of(reply_ack)

    # The reply swaps sender and receiver, so the source's MSH-5 lands in MSH-3.
    # The fields are compared as *identities*, not as ER7 text: a field is split
    # into components and each component has its escapes resolved, using the
    # separators of the message it came from. Comparing the wire strings instead
    # would fail on a correct reply, because `\F\` in the source denotes a
    # literal `|` and that same `|` needs no escape once the reply's field
    # separator is `#`. Two renderings of one identity are what the conversion is
    # for; it is only a bug when the identities differ.
    for original, mirrored, why in (
        ("MSH-5", "MSH-3", "swapped sending application"),
        ("MSH-6", "MSH-4", "swapped sending facility"),
        ("MSH-3", "MSH-5", "swapped receiving application"),
        ("MSH-4", "MSH-6", "swapped receiving facility"),
    ):
        expected = _identity(_msh_field(source, original), source_sep)
        actual = _identity(_msh_field(reply, mirrored), reply_sep)
        if expected != actual:
            print(
                f"  cnv {index}: {mirrored} is {actual!r}, expected {expected!r} ({why})"
            )
            failed = True
        # A decoded component string hides where the subcomponent boundaries fell,
        # and a conversion that changes *both* separators has the most room to move
        # one. Compare the split, not only the text.
        expected_shape = _shape(_msh_field(source, original), source_sep)
        actual_shape = _shape(_msh_field(reply, mirrored), reply_sep)
        if expected_shape != actual_shape:
            print(
                f"  cnv {index}: {mirrored} splits into subcomponents "
                f"{actual_shape!r}, expected {expected_shape!r} ({why})"
            )
            failed = True

    # A delimiter copied out of an escape body would push the control id along.
    control = _identity(_msh_field(reply, "MSH-10"), reply_sep)
    if control != [f"CNV{index:04}"]:
        print(f"  cnv {index}: MSH-10 is {control!r}, expected 'CNV{index:04}'")
        failed = True
    return int(failed)


def _separators_of(er7: str) -> dict[str, str]:
    """The five delimiters an ER7 message declares in MSH-1 and MSH-2."""
    return {
        "field": er7[3],
        "component": er7[4],
        "repetition": er7[5],
        "escape": er7[6],
        "subcomponent": er7[7],
    }


def _identity(er7: str, separators: dict[str, str]) -> list[str]:
    """One field as its list of decoded components.

    Split on the raw component separator and resolve the five standard escapes in
    each part. A delimiter that arrives escaped is data and comes back as itself,
    while a raw separator is structure and ends the component - which is exactly
    the distinction the conversion has to preserve. This is written from the
    standard rather than shared with the crate, so it can disagree with it.
    """
    escape = separators["escape"]
    named = {
        "F": separators["field"],
        "S": separators["component"],
        "R": separators["repetition"],
        "E": escape,
        "T": separators["subcomponent"],
    }
    decoded = []
    for component in er7.split(separators["component"]):
        out = []
        i = 0
        while i < len(component):
            if component[i] != escape:
                out.append(component[i])
                i += 1
                continue
            close = component.find(escape, i + 1)
            if close < 0:
                out.append(component[i:])
                break
            body = component[i + 1 : close]
            # An unknown escape is site-defined and kept verbatim.
            out.append(named.get(body, f"{escape}{body}{escape}"))
            i = close + 1
        decoded.append("".join(out))
    return decoded


def _msh_field(message, reference: str) -> str:
    """One MSH field of a parsed message in its own ER7 encoding.

    Matched by name, not by index: hl7apy treats MSH-1, the field separator, as
    the encoding character rather than a child, so `msh[10]` is MSH-11 and the
    off-by-one silently reads the wrong field.
    """
    for segment in message.children:
        if getattr(segment, "name", None) != "MSH":
            continue
        for child in segment.children:
            if child.name.replace("_", "-", 1) == reference:
                return child.to_er7()
    return ""


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
    # Every finding is printed, but the reply counts as one failure.
    failed = False

    def expect(reference: str, value: str, why: str) -> None:
        nonlocal failed
        got = reply.get(reference) or ""
        if got != value:
            print(f"  enh {index}: {reference} is {got!r}, expected {value!r} ({why})")
            failed = True

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
                failed = True
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
                failed = True
    except Exception as error:  # noqa: BLE001
        print(f"  enh {index}: hl7apy could not read the ERR segment: {error}")
        failed = True
    return int(failed)


def main() -> int:
    try:
        from hl7apy.parser import parse_message  # noqa: F401
    except ImportError:
        print("needs: pip install hl7apy==1.3.5")
        return 2

    (
        rust,
        acks,
        enhanced,
        converted,
        refusals,
        field_only,
        field_only_refusals,
    ) = rust_parse(CORPUS)
    mismatches = 0
    compared = 0

    for index, raw in enumerate(CORPUS):
        expected = reference_parse(raw)
        actual = rust[index]
        for reference, value in sorted(expected.items()):
            compared += 1
            got = actual.get(reference)
            if got is not None:
                # Compared as an identity, not as ER7: hl7apy re-renders the
                # escape character itself (`\E\`), so the two sides are the same
                # value in different renderings. `_identity` resolves escapes on
                # both sides and compares what they mean.
                separators = _separators_of(raw)
                got = _identity(got, separators)
                value = _identity(value, separators)
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

    converted_problems = 0
    for index, raw in enumerate(CORPUS):
        converted_problems += check_converted(
            raw, converted[index], refusals.get(index, ""), index
        )
    print(f"{len(CORPUS) - converted_problems}/{len(CORPUS)} converted replies agree with hl7apy")

    # Changing only the field separator keeps every component boundary, so an
    # opaque body needs no rewrite and no reply may be refused.
    field_only_problems = 0
    for index, raw in enumerate(CORPUS):
        refusal = field_only_refusals.get(index)
        if refusal:
            print(f"  fld {index}: refused a field-only conversion: {refusal!r}")
            field_only_problems += 1
            continue
        field_only_problems += check_field_only(raw, field_only[index], index)
    print(
        f"{len(CORPUS) - field_only_problems}/{len(CORPUS)} field-only replies agree with hl7apy"
    )

    # Each category contributes the number of *replies* that failed, not the
    # number of findings printed for them, so the total is a count of bad replies.
    total = (
        mismatches
        + ack_problems
        + enhanced_problems
        + converted_problems
        + field_only_problems
    )
    if total:
        print(f"{total} MISMATCH(ES)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
