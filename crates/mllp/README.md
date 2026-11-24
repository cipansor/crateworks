# mllp

[![crates.io](https://img.shields.io/crates/v/mllp.svg)](https://crates.io/crates/mllp)
[![docs.rs](https://docs.rs/mllp/badge.svg)](https://docs.rs/mllp)

MLLP framing and HL7 v2 parsing. No dependencies.

MLLP, the Minimal Lower Layer Protocol, is how almost every HL7 v2 message
travels over TCP. It wraps each message in a start block, the payload, and an
end block, so a receiver reading a byte stream knows where one message stops and
the next begins.

```rust
use mllp::{frame, unframe, Decoder};

let message = b"MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|1|P|2.5.1\r";

// Put a message on the wire.
let wire = frame(message);
assert_eq!(wire[0], 0x0B);
assert_eq!(&wire[wire.len() - 2..], &[0x1C, 0x0D]);

// Take one back off, when you already hold a whole frame.
assert_eq!(unframe(&wire).unwrap(), message);

// Or reassemble from a stream, where reads do not line up with frames.
let mut decoder = Decoder::new();
decoder.push(b"\x0bMSH|^~\\&|SEND|FAC|RECV");
assert!(decoder.next_frame().is_none()); // half a frame, wait for more
decoder.push(b"|FAC|20260101||ADT^A01|1|P|2.5.1\r\x1c\x0d");
assert_eq!(decoder.next_frame().unwrap().unwrap(), message);
```

## Parsing the message

Framing gets the bytes to you; `Message` opens them up. It splits a message into
segments, a segment into fields, and a field into repetitions of components of
subcomponents, using the delimiters the message declares in its own header.

```rust
use mllp::Message;

let raw = "MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|1|P|2.5.1\r\
           PID|1||12345^^^MRN~67890^^^MRN||DOE^JOHN\r";
let message = Message::parse(raw).unwrap();

// The segments, in order.
assert_eq!(message.segment_names().collect::<Vec<_>>(), ["MSH", "PID"]);

// A field, by its HL7 reference.
let name = message.field("PID-5").unwrap();
assert_eq!(name.text(), "DOE^JOHN");
assert_eq!(name.component(1), Some("DOE"));

// A repeating field, one repetition at a time.
let ids = message.field("PID-3").unwrap();
assert_eq!(ids.repetitions().count(), 2);

// A value with the delimiter escapes resolved.
assert_eq!(message.value("MSH-3").unwrap(), "SEND");
```

Everything borrows from the input, so parsing copies nothing. The parser is a
*reader*, not a validator: it does not know which fields a given message type
requires, and it does not reject a site-specific segment. It splits on the
delimiters and returns the pieces, which is what a receiver needs before it can
decide what the message means.

`Message::parse` takes the delimiters from MSH, so a message that uses `#` as
its field separator parses with the same code. `Message::to_er7` writes the
message back out.

## The framing bytes

| Role | Name | Byte |
|---|---|---|
| Start block | `SB`, vertical tab | `0x0B` |
| End block | `EB`, file separator | `0x1C` |
| Trailing | `CR`, carriage return | `0x0D` |

A frame is `0x0B` + payload + `0x1C 0x0D`.

The terminator is two bytes, and that matters. An HL7 v2 message ends each
segment with a carriage return, so the payload normally contains `0x0D`. If the
frame ended at the first `0x0D`, every message would be cut at its first segment.
This crate searches for the pair `0x1C 0x0D`, so a carriage return inside the
payload is data. A `0x1C` that is not followed by `0x0D` is data too, because a
binary payload may contain any byte.

## Reading a stream

`Decoder` holds the bytes that do not yet form a whole frame. Push each socket
read in, drain `next_frame` until it returns `None`. A frame split at any byte
comes out whole, and one read carrying several frames yields several messages;
both cases are covered by tests.

`next_frame` returns three things: `Some(Ok(payload))` for a whole frame,
`None` when more bytes are needed, and `Some(Err(..))` for input that cannot be
part of a frame. Bytes before the next start block are discarded and reported
once, so a decoder resynchronises after a desync instead of stalling.

A frame that never terminates would otherwise grow the buffer without bound.
`Decoder` stops at 16 MiB by default and returns `Error::FrameTooLarge`;
`Decoder::with_max_frame_size` changes the ceiling. That is a guard against a
missing terminator, not a limit on real messages.

## Answering a message

An interface that receives a message must send an acknowledgement back. HL7 v2
defines that as a message of its own: `MSH`, then `MSA`, whose MSH-10 echoes the
control id of the message being acknowledged and whose MSA-1 is the code that
says what happened to it.

```rust
use mllp::{AckCode, AckContext, Message, build_ack};

let received = Message::parse(
    "MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|MSG0001|P|2.5.1\rPID|1||12345\r",
).unwrap();

// Read the header values the reply has to carry.
let context = AckContext::from_message(&received);

// Build the reply. The control id is the reply's own MSH-10, which is new;
// the original one is echoed in MSA-2.
let ack = build_ack(&context, AckCode::Accept, "ACK0001", "", received.separators()).unwrap();

let reply = Message::parse(&ack).unwrap();
assert_eq!(reply.value("MSA-1").unwrap(), "AA");
assert_eq!(reply.value("MSA-2").unwrap(), "MSG0001");
assert_eq!(reply.value("MSH-3").unwrap(), "RECV"); // the facilities swap
```

`AckCode` has the three application codes: `Accept` (`AA`), `Error` (`AE`) for
an application failure and `Reject` (`AR`) for a version or format problem. The
reply is written with the delimiters of the message it answers, so a peer that
uses `#` as its field separator gets a reply it can read. MSH-7, the build time,
is left empty; a caller that wants it sets it afterwards.

Every value the builders write is escaped first. A control id or a failure text
is caller-supplied and can hold any character, including a delimiter; written
raw, the first `|` would end its field and every later field would shift. The
reply would still parse, which is what makes it dangerous - it would mean
something the sender never intended. `mllp::escape` is the inverse of the
resolution `Field::unescape` does, and takes plain text:

```rust
use mllp::{AckCode, AckContext, Message, build_ack};

let received = Message::parse(
    "MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|MSG0001|P|2.5.1\rPID|1||12345\r",
).unwrap();
let context = AckContext::from_message(&received);

// The text quotes the value that failed, so it holds a field separator.
let ack = build_ack(
    &context,
    AckCode::Error,
    "ACK0003",
    "rejected | bad value",
    received.separators(),
)
.unwrap();

// On the wire the separator is escaped, so MSA-3 is still one field.
let reply = Message::parse(&ack).unwrap();
assert_eq!(reply.field("MSA-3").unwrap().text(), "rejected \\F\\ bad value");
// And reading it back returns the text the caller supplied.
assert_eq!(reply.value("MSA-3").unwrap(), "rejected | bad value");
```

The header is different. MSH-3, MSH-4, MSH-5 and MSH-6 are HD values and MSH-12
is a VID, so their components are structure: `LAB^1.2.3^ISO` is a namespace, a
universal id and an id type. `AckContext` therefore keeps those fields as ER7,
and the builder re-emits them rather than escaping them, so the reply carries the
same three components. A delimiter that was escaped in the source stays data
while the separators around it stay structure.

```rust
use mllp::{AckCode, AckContext, Message, build_ack};

let received = Message::parse(
    "MSH|^~\\&|LAB^1.2.3^ISO|FAC|RECV|FAC|20260101||ADT^A01|MSG0002|P|2.5.1\rPID|1||12345\r",
).unwrap();
let context = AckContext::from_message(&received);
let ack = build_ack(&context, AckCode::Accept, "ACK0004", "", received.separators()).unwrap();

let reply = Message::parse(&ack).unwrap();
let application = reply.field("MSH-5").unwrap();
assert_eq!(application.component(1), Some("LAB"));
assert_eq!(application.component(2), Some("1.2.3"));
assert_eq!(application.component(3), Some("ISO"));
```

Because the reply is built with `received.separators()`, it never has to convert
anything, so it cannot fail - a receiver can always acknowledge a message it was
able to parse, which is the whole point of the fallback.

Answering with a *different* delimiter set is where the errors live. The rule is
about *roles*, not about which characters appear. A reader splits on a delimiter
before it resolves escapes, so a delimiter inside a site-defined escape body such
as `\ZLAB#1\` is structure: the body survives a conversion only when every
character keeps the role it had. A separator that becomes data stops splitting
the field, and data that becomes a separator starts splitting it, so both are
refused - rewriting the body would silently change how many components the field
has. A body can never hold the component separator: that separator splits the
component before any reader looks for an escape.

Changing only the field separator keeps every component and subcomponent
boundary, so such a reply is normally accepted. It is still refused in one case:
the new field separator itself appears inside an opaque escape body. That is the
role rule again - the character was data in the message and would become a field
boundary in the reply - and no rewrite of the body can carry it. The example below
is exactly that case, and it is why the way out it shows is to answer with the
original's separators.

The wire bytes of an accepted field-only reply can still differ from the source's,
because a marker that was stranded - one ending a component, or opening one, with
no pair - has to be re-encoded as `\E\` once the reply treats the escape
character differently. What is preserved is what a peer decodes, and the component
and subcomponent boundaries it splits on, not the bytes.

A raw field separator is refused as well, and that verdict does not depend on the
reply: the value is invalid before a reply exists, so no choice of delimiters can
make it acceptable. It cannot occur in a parsed field - the parser splits on it -
so it only comes from a hand-built [`AckContext`], where it was meant as data but
would be read as structure.

Two values are written by the builders rather than copied, and neither is judged
against the incoming message: MSH-9 (`ACK`) and the `2.5.1` fallback version. They
are plain text under the reply's own delimiters, so they are escaped, never
transcoded.

```rust
use mllp::{AckCode, AckContext, Message, Separators, build_ack};

let received = Message::parse(
    "MSH|^~\\&|S\\ZLAB#1\\|F|R|F|20260101||ADT^A01|MSG0005|P|2.5.1\rPID|1||12345\r",
).unwrap();
let context = AckContext::from_message(&received);

// A reply whose field separator is `#` cannot carry that escape body.
let mut reply_separators = Separators::default();
reply_separators.field = '#';
let error = build_ack(&context, AckCode::Accept, "ACK0006", "", reply_separators).unwrap_err();
assert!(error.to_string().contains("MSH-5"));

// The way out is to answer with the original's separators, which is what the
// usual call does.
let ack = build_ack(&context, AckCode::Accept, "ACK0007", "", received.separators()).unwrap();
assert_eq!(Message::parse(&ack).unwrap().value("MSH-10").unwrap(), "ACK0007");
```



## Explaining an error

An `AE` or an `AR` can say what went wrong. The *enhanced* acknowledgement fills
MSA-4 with an error condition code and appends an `ERR` segment carrying the
same code with its text and severity, which is what [`build_ack_enhanced`] writes.

```rust
use mllp::{AckCode, AckContext, ErrorDetail, ErrorLocation, Message, Severity, build_ack_enhanced};

let received = Message::parse(
    "MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|MSG0001|P|2.5.1\rPID|1||12345\r",
).unwrap();
let context = AckContext::from_message(&received);

// The error condition is HL7 table 0357: 207 is "application internal error".
// The location says where it is: PID, the second segment, its third field.
let error = ErrorDetail::new("207", "Application internal error", Severity::Error)
    .at(ErrorLocation::field("PID", 2, 3));
let ack = build_ack_enhanced(
    &context,
    AckCode::Error,
    "ACK0002",
    "bad patient",
    &error,
    received.separators(),
)
.unwrap();

let reply = Message::parse(&ack).unwrap();
assert_eq!(reply.value("MSA-4").unwrap(), "207"); // the code, repeated
assert_eq!(reply.value("ERR-2").unwrap(), "PID^2^3"); // the location
assert_eq!(reply.value("ERR-3").unwrap(), "207^Application internal error^HL70357");
assert_eq!(reply.value("ERR-4").unwrap(), "E"); // the severity
```

`Severity` is the ERR-4 code from HL7 table 0516: `Warning` (`W`), `Error` (`E`)
and `Fatal` (`F`). `ErrorDetail` writes `HL70357` as the coding system unless the
caller sets another.

`ErrorLocation` is the ERR-2 location, an ERL value: the segment id, its one-based
position in the message, the field's one-based position, and then the repetition,
component and subcomponent when the error is narrower than a field. Only the
first three are required, so a caller that knows the field and stops writes
`PID^2^3`; `with_component`, `with_repetition` and `with_subcomponent` narrow it
further and keep the empty components they skip, so a component that comes after
an unset repetition lands in the position a reader expects.

## What it does not do

`Message` reads the structure of a message; it does not validate one. It does not
know which fields a message type requires, and it does not check a field against
a table. `build_ack` and `build_ack_enhanced` write the reply an interface needs
to send; the enhanced form carries the error condition and, when the caller
supplies it, the location in ERR-2. The caller still decides *what* the error is
and *where* it is - the builder does not read the message to find the offending
field, because what counts as an error is the application's judgement, not the
parser's.

The framing layer does not open sockets, retry, or decide how long to wait for a
reply, and it does not implement TLS or the newer, block-based MLLP (MLLP over
HTTPS).

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
