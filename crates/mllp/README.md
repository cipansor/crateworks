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

## What it does not do

`Message` reads the structure of a message; it does not validate one. It does not
know which fields a message type requires, it does not check a field against a
table, and it does not build an acknowledgement (`ACK`) for you: that is a
message-construction task on top of the parser, not part of it.

The framing layer does not open sockets, retry, or decide how long to wait for a
reply, and it does not implement TLS or the newer, block-based MLLP (MLLP over
HTTPS).

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
