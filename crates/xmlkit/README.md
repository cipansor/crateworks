# xmlkit

[![crates.io](https://img.shields.io/crates/v/xmlkit.svg)](https://crates.io/crates/xmlkit)
[![docs.rs](https://docs.rs/xmlkit/badge.svg)](https://docs.rs/xmlkit)

A small XML parser, tree and serialiser with no dependencies.

```rust
use xmlkit::{parse, local_name};

let doc = parse(r#"<ns:Doc xmlns:ns="urn:x"><ns:Id>7</ns:Id></ns:Doc>"#).unwrap();
assert_eq!(local_name(&doc.name), "Doc");
assert_eq!(doc.deep_text(), "7");
```

Keeps comments and CDATA so that a parse and a re-serialise round-trips
faithfully. Namespace prefixes are preserved and `local_name` strips them for
matching, which is what a SOAP or ISO 20022 reader needs.

This crate exists because `iso20022-rs` and `soap-client` both need the same
small amount of XML handling, and neither needs a full DOM. It deliberately
covers a subset of XML: elements, attributes, text, CDATA, comments,
processing instructions and the five predefined entities plus numeric
character references. It does not resolve external entities, which also
means it cannot be used for XXE attacks. Namespace URIs are resolved; nothing
else about a schema is.

## Namespaces

The prefix is arbitrary and the URI is the identity, so matching on the name as
written finds `<soap:Body>` and misses `<env:Body>` even when both are the same
element. `find_namespaced` and `all_namespaced` match on the URI and the local
name, and resolve the prefix through the declarations on the element and its
ancestors, which is what a rebinding needs.

```rust
use xmlkit::{XML_NAMESPACE, attribute_namespaced, find_namespaced, parse};

let doc = parse(
    r#"<env:Envelope xmlns:env="urn:soap"><env:Body xml:lang="en"/></env:Envelope>"#,
).unwrap();

// Any prefix bound to the URI matches, and the prefix in the document is irrelevant.
assert!(find_namespaced(&doc, Some("urn:soap"), "Body").is_some());
assert!(find_namespaced(&doc, Some("urn:other"), "Body").is_none());

// `xml:` is bound without a declaration, and an unprefixed attribute is in no
// namespace even inside an element that declares a default one.
let body = find_namespaced(&doc, Some("urn:soap"), "Body").unwrap();
assert_eq!(attribute_namespaced(&doc, body, XML_NAMESPACE, "lang"), Some("en"));
```

An unbound prefix names nothing rather than "no namespace", so a document cannot
pass off `<evil:Body>` as an un-namespaced `Body` by leaving `evil` undeclared.

## Well-formedness

The parser enforces the XML 1.0 grammar rather than accepting whatever it can
read. It checks names against `NameStartChar`/`NameChar`, requires whitespace
between attributes, rejects `<` in an attribute value, rejects `--` or a
trailing `-` in a comment, rejects an unescaped `]]>` in content, validates the
XML declaration and the DOCTYPE structure, and rejects a character reference
outside the XML character range.
`crates/xmlkit/tests/fixtures/wellformed.tsv` is a corpus of 7,916 documents -
half valid, half a single mutation of one - whose well-formedness was decided by
Python's `expat`, and the crate agrees with expat on every one.

A document is a prolog, one root element, and then `Misc` (whitespace, comments,
processing instructions), so `<a/><!-- done -->` parses and `<a/><b/>` does not.
A byte-order mark at the start is skipped, as XML 1.0 section 4.3.3 allows.

One case is deliberately looser than expat. An ampersand that begins no entity
is kept as a literal `&` rather than rejected, so `<a>Tom & Jerry</a>` reads as
the text `Tom & Jerry`. This matches the npm `@xmldom/xmldom` parser that most
SOAP and ISO 20022 tooling runs on, and it is the behaviour this crate's
consumers depend on. The five predefined entities, `&#n;` and `&#xn;` are all
decoded; an out-of-range reference is an error, not silently dropped.

## Whitespace and line endings

Two normalisations happen before any text or attribute value is handed out, and
both are easy to miss because they only show up on input a hand-written test does
not use. XML 1.0 section 2.11 turns a literal carriage return, and a
carriage-return/line-feed pair, into a single line feed in element content, so
`<a>x\ry</a>` reads as `"x\ny"`. Section 3.3.3 then turns a tab, a newline or a
carriage return inside an attribute value into a space, so `x="p\tq"` reads as
`"p q"`. A numeric character reference is exempt from both, because it is
resolved after them: `<a>x&#13;y</a>` really is `"x\ry"`, and `x="p&#9;q"` really
is `"p\tq"`. `javaproperties`'s XML reader depends on this, which is where the
rule was found.

Between markup, only the four characters XML 1.0 section 2.3 calls `S` separate
tokens: space, tab, carriage return and line feed. A Unicode space - a no-break
space `\u{a0}`, an ideographic space `\u{3000}`, or any other - is an ordinary
character there, so it cannot separate a name from an attribute and such a
document is not well-formed. In text and in an attribute value a Unicode space is
just a character. The parser steps over whitespace one byte at a time, so
accepting a multi-byte space as a separator would leave it inside the character;
expat, the reference for this crate, rejects these documents too.

```rust
use xmlkit::parse;

// A no-break space cannot separate a name from an attribute.
assert!(parse("<a\u{a0}b=\"1\"/>").is_err());
// In element content it is an ordinary character.
assert_eq!(parse("<a>\u{a0}</a>").unwrap().text(), "\u{a0}");
```

## Scope

* `parse` builds an `Element` tree from a string.
* `Element`'s `Display` impl writes it back.
* `find_namespaced`, `all_namespaced` and `Namespaces` resolve namespace URIs.
* No XPath, no schema validation, no DTD processing beyond skipping it.

The parser and the tree's `Drop` both keep their own stack rather than recursing,
so a deeply nested document cannot overflow the call stack; `MAX_DEPTH` is a
policy limit on nesting, not a safety one.

## Example

```
use xmlkit::{Element, parse};

let root = parse("<a x=\"1\"><b>hi</b></a>").unwrap();
assert_eq!(root.name, "a");
assert_eq!(root.attr("x"), Some("1"));
assert_eq!(root.child("b").unwrap().text(), "hi");
```

Part of the [crateworks](https://github.com/cipansor/crateworks)
workspace. Licensed under the Apache License, Version 2.0, copyright Yayasan
Pesantren Cipansor and contributors.
