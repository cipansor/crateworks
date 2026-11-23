//! Behaviour of the typed layer over a parsed `Forwarded` element.
//!
//! `tests/forwarded_oracle.rs` checks the parser against npm `forwarded-parse`,
//! which works in strings and has no notion of an address. `Node::parse` is
//! this crate's addition, so its expectations are RFC 7239 section 6.3 read
//! directly rather than taken from that reference.

use std::net::IpAddr;

use proxy_addr::{Node, Port, parse_forwarded};

fn ip(text: &str) -> IpAddr {
    text.parse().expect("test address parses")
}

fn address(text: &str, port: Option<Port>) -> Node {
    Node::Address { ip: ip(text), port }
}

#[test]
fn reads_the_four_parameters_the_rfc_defines() {
    // A `host` carrying a port has to be quoted: a colon is not a token
    // character, so `host=example.com:8443` is a syntax error in the reference
    // too. Quoting is the RFC's rule, not a gap here.
    let hops =
        parse_forwarded(r#"for=192.0.2.60;proto=https;by=203.0.113.43;host="example.com:8443""#)
            .unwrap();
    let hop = &hops[0];

    assert_eq!(hop.for_node(), Some("192.0.2.60"));
    assert_eq!(hop.by(), Some("203.0.113.43"));
    assert_eq!(hop.proto(), Some("https"));
    assert_eq!(hop.host(), Some("example.com:8443"));
}

#[test]
fn parameter_names_are_lowercased() {
    let hops = parse_forwarded("For=192.0.2.43;PROTO=https;By=203.0.113.43").unwrap();
    assert_eq!(hops[0].for_node(), Some("192.0.2.43"));
    assert_eq!(hops[0].proto(), Some("https"));
    assert_eq!(hops[0].by(), Some("203.0.113.43"));
}

#[test]
fn a_repeated_parameter_keeps_its_first_position_and_last_value() {
    // The reference assigns to an object property, so a second `for` overwrites
    // the value without moving the key. The corpus checks the value; this
    // checks that the position did not move either, which `params` exposes.
    let hops = parse_forwarded("for=1.2.3.4;proto=http;for=5.6.7.8").unwrap();
    assert_eq!(hops[0].for_node(), Some("5.6.7.8"));
    let names: Vec<&str> = hops[0].params().map(|(name, _)| name).collect();
    assert_eq!(names, ["for", "proto"]);
}

#[test]
fn elements_stay_in_header_order() {
    // RFC 7239 appends each hop, so the client is first and the nearest proxy
    // last. This is the opposite of `X-Forwarded-For`, which the crate reverses.
    let hops = parse_forwarded("for=192.0.2.43, for=198.51.100.17, for=203.0.113.9").unwrap();
    let nodes: Vec<&str> = hops.iter().filter_map(|h| h.for_node()).collect();
    assert_eq!(nodes, ["192.0.2.43", "198.51.100.17", "203.0.113.9"]);
}

#[test]
fn reads_a_node_as_an_address_with_an_optional_port() {
    assert_eq!(Node::parse("192.0.2.43"), Some(address("192.0.2.43", None)));
    assert_eq!(
        Node::parse("192.0.2.43:4711"),
        Some(address("192.0.2.43", Some(Port::Number(4711))))
    );
    assert_eq!(
        Node::parse("[2001:db8::1]"),
        Some(address("2001:db8::1", None))
    );
    assert_eq!(
        Node::parse("[2001:db8::1]:4711"),
        Some(address("2001:db8::1", Some(Port::Number(4711))))
    );
}

#[test]
fn reads_the_two_non_address_forms() {
    assert_eq!(Node::parse("unknown"), Some(Node::Unknown { port: None }));
    assert_eq!(
        Node::parse("_hidden"),
        Some(Node::Obfuscated {
            name: "hidden".to_string(),
            port: None
        })
    );
    // An obfuscated name holds only ALPHA, DIGIT, `.`, `_` and `-`.
    assert_eq!(Node::parse("_a b"), None);
    assert_eq!(Node::parse("_"), None);
}

#[test]
fn the_port_is_independent_of_the_node_name() {
    // RFC 7239 gives `node = nodename [ ":" node-port ]`, so every name form
    // takes every port form. Each of these is a legal header, so dropping the
    // typed node would lose information the header carried.
    assert_eq!(
        Node::parse("192.0.2.43:_edge"),
        Some(address("192.0.2.43", Some(Port::Obfuscated("edge".into()))))
    );
    assert_eq!(
        Node::parse("_hidden:443"),
        Some(Node::Obfuscated {
            name: "hidden".into(),
            port: Some(Port::Number(443))
        })
    );
    assert_eq!(
        Node::parse("_hidden:_port"),
        Some(Node::Obfuscated {
            name: "hidden".into(),
            port: Some(Port::Obfuscated("port".into()))
        })
    );
    assert_eq!(
        Node::parse("unknown:443"),
        Some(Node::Unknown {
            port: Some(Port::Number(443))
        })
    );
    assert_eq!(
        Node::parse("unknown:_p"),
        Some(Node::Unknown {
            port: Some(Port::Obfuscated("p".into()))
        })
    );
    assert_eq!(
        Node::parse("[2001:db8::1]:_edge"),
        Some(address(
            "2001:db8::1",
            Some(Port::Obfuscated("edge".into()))
        ))
    );

    // The example from the review: a header that parses must not lose its node.
    let hops = parse_forwarded(r#"for="192.0.2.43:_edge", for="_hidden:443""#).unwrap();
    assert_eq!(
        hops[0].node(),
        Some(address("192.0.2.43", Some(Port::Obfuscated("edge".into()))))
    );
    assert_eq!(
        hops[1].node(),
        Some(Node::Obfuscated {
            name: "hidden".into(),
            port: Some(Port::Number(443))
        })
    );
}

#[test]
fn unknown_is_case_insensitive() {
    // The ABNF quotes `"unknown"`, and an ABNF literal matches
    // case-insensitively (RFC 5234 section 2.3), so `Unknown` is the same
    // identifier.
    for text in ["unknown", "Unknown", "UNKNOWN", "uNkNoWn"] {
        assert_eq!(
            Node::parse(text),
            Some(Node::Unknown { port: None }),
            "{text} is the unknown identifier"
        );
    }
    assert_eq!(
        Node::parse("Unknown:443"),
        Some(Node::Unknown {
            port: Some(Port::Number(443))
        })
    );
    // `Display` writes the canonical lowercase form.
    assert_eq!(Node::parse("UNKNOWN").unwrap().to_string(), "unknown");
}

#[test]
fn an_obfuscated_name_uses_its_own_narrower_grammar() {
    // The outer header grammar allows any token, but RFC 7239's `obfnode` is
    // `"_" 1*( ALPHA / DIGIT / "." / "_" / "-" )`. These are tokens the outer
    // grammar accepts and the typed layer must reject.
    for text in ["_edge+2", "_a!b", "_a~b", "_a*b", "_a'b", "_a|b", "_a^b"] {
        assert_eq!(Node::parse(text), None, "{text} is not an obfnode");
    }
    // The characters the obfnode grammar does allow.
    for text in ["_a.b", "_a_b", "_a-b", "_A9", "__", "_."] {
        assert!(
            matches!(Node::parse(text), Some(Node::Obfuscated { .. })),
            "{text} is a legal obfnode"
        );
    }
    // The same grammar governs `obfport`.
    assert_eq!(Node::parse("192.0.2.43:_e+1"), None);
}

#[test]
fn brackets_are_for_ipv6_only() {
    // RFC 7239 brackets an IPv6 address; an IPv4 address is written bare, so a
    // bracketed one is not a node the grammar produces.
    assert_eq!(Node::parse("[192.0.2.43]"), None);
    assert_eq!(Node::parse("[192.0.2.43]:80"), None);
    assert!(Node::parse("[2001:db8::1]").is_some());
}

#[test]
fn a_decimal_port_is_one_to_five_digits() {
    // `port = 1*5DIGIT`. Rust's `u16` parser is more permissive than that: it
    // takes a leading `+` and any number of leading zeros, so the shape is
    // checked separately.
    for text in [
        "192.0.2.43:+443",
        "192.0.2.43:000443",
        "192.0.2.43:-1",
        "192.0.2.43:443443",
        "[2001:db8::1]:+443",
        "[2001:db8::1]:000443",
        "[2001:db8::1]:443443",
        "unknown:000443",
        "_hidden:+443",
    ] {
        assert_eq!(Node::parse(text), None, "{text} has an illegal port");
    }
    // The boundaries that are legal.
    assert_eq!(
        Node::parse("192.0.2.43:0"),
        Some(address("192.0.2.43", Some(Port::Number(0))))
    );
    assert_eq!(
        Node::parse("192.0.2.43:65535"),
        Some(address("192.0.2.43", Some(Port::Number(65535))))
    );
    assert_eq!(
        Node::parse("[2001:db8::1]:00443"),
        Some(address("2001:db8::1", Some(Port::Number(443))))
    );
}

#[test]
fn rejects_text_the_rfc_does_not_allow_as_a_node() {
    // A bare IPv6 address is ambiguous with `address:port`, so RFC 7239
    // requires the brackets. Accepting it would read `2001:db8::1` as host
    // `2001:db8:` with port `1`.
    assert_eq!(Node::parse("2001:db8::1"), None);
    // A port that is neither digits nor an obfuscated name is not a port.
    assert_eq!(Node::parse("192.0.2.43:http"), None);
    assert_eq!(Node::parse("192.0.2.43:70000"), None);
    assert_eq!(Node::parse(""), None);
    assert_eq!(Node::parse("not-an-address"), None);
}

#[test]
fn a_node_round_trips_through_display() {
    for text in [
        "192.0.2.43",
        "192.0.2.43:4711",
        "192.0.2.43:_edge",
        "[2001:db8::1]",
        "[2001:db8::1]:4711",
        "[2001:db8::1]:_edge",
        "unknown",
        "unknown:4711",
        "unknown:_p",
        "_hidden",
        "_hidden:4711",
        "_hidden:_p",
    ] {
        let node = Node::parse(text).unwrap_or_else(|| panic!("{text} should parse"));
        assert_eq!(node.to_string(), text, "round trip of {text}");
        assert_eq!(
            Node::parse(&node.to_string()),
            Some(node),
            "{text} survives a Display round trip"
        );
    }
}

#[test]
fn a_quoted_node_reaches_the_typed_layer_unescaped() {
    // The header quotes an address because a colon is not a token character;
    // by the time `Node::parse` sees it the quotes are gone.
    let hops = parse_forwarded(r#"for="[2001:db8:cafe::17]:4711""#).unwrap();
    assert_eq!(
        hops[0].node(),
        Some(address("2001:db8:cafe::17", Some(Port::Number(4711))))
    );
}

#[test]
fn an_element_without_for_has_no_node() {
    let hops = parse_forwarded("proto=https").unwrap();
    assert_eq!(hops[0].for_node(), None);
    assert_eq!(hops[0].node(), None);
    assert!(hops[0].param("for").is_none());
}

#[test]
fn reports_where_an_unreadable_header_stopped() {
    let error = parse_forwarded("for=1.2.3.4;proto").unwrap_err();
    assert_eq!(error.position(), None, "the header ended mid-element");

    let error = parse_forwarded(" for=1.2.3.4").unwrap_err();
    assert_eq!(
        error.position(),
        Some(0),
        "the reference rejects a leading space at index 0"
    );
}

#[test]
fn a_value_needing_a_delimiter_must_be_quoted() {
    // A colon, a slash or a bracket is a `separator` in RFC 7230, not a token
    // character, so an unquoted value holding one is a syntax error. The
    // reference rejects it, and so must this crate: quietly accepting it would
    // read `host=example.com:8443` where the reference stops.
    for header in [
        "host=example.com:8443",
        "for=[2001:db8::1]",
        "proto=https/1",
        "by=a(b)",
    ] {
        assert!(
            parse_forwarded(header).is_err(),
            "{header} is not a token value and must not parse"
        );
        // The same value quoted is legal, and keeps its content.
        let (name, value) = header.split_once('=').unwrap();
        let quoted = format!("{name}=\"{value}\"");
        let hops = parse_forwarded(&quoted)
            .unwrap_or_else(|error| panic!("{quoted} should parse: {error}"));
        assert_eq!(hops[0].param(name), Some(value));
    }
}

#[test]
fn the_error_index_counts_characters_not_bytes() {
    // `é` inside a quoted-string is legal, so the error lands after it, where a
    // byte index and a char index diverge: `é` is two bytes and one character.
    // The reference reports 14 here, which is the character index. Reporting
    // bytes would point a caller at the wrong place in the header it printed.
    let header = r#"for="é";proto=("#;
    assert_eq!(header.len(), 16, "the header is 16 bytes");
    assert_eq!(header.chars().count(), 15, "and 15 characters");

    let error = parse_forwarded(header).unwrap_err();
    assert_eq!(error.position(), Some(14));
}

#[test]
fn an_empty_header_is_an_error_not_an_empty_list() {
    // An empty list would read as "no proxy spoke", which is a different claim
    // from "this header is broken".
    assert!(parse_forwarded("").is_err());
    assert!(parse_forwarded(" ").is_err());
}

#[test]
fn empty_list_elements_are_ignored() {
    // RFC 7230 section 7 requires a recipient to parse and ignore empty list
    // elements, so a sender that merges two header lines with a doubled comma
    // still produces a header this crate reads. The npm reference rejects
    // these, which is the one deliberate divergence.
    for (header, expected) in [
        (
            "for=192.0.2.43,,for=198.51.100.7",
            vec!["192.0.2.43", "198.51.100.7"],
        ),
        (
            "for=192.0.2.43, ,for=198.51.100.7",
            vec!["192.0.2.43", "198.51.100.7"],
        ),
        (
            "for=192.0.2.43,,,for=198.51.100.7,",
            vec!["192.0.2.43", "198.51.100.7"],
        ),
        (",for=192.0.2.43", vec!["192.0.2.43"]),
        ("for=192.0.2.43,", vec!["192.0.2.43"]),
    ] {
        let hops = parse_forwarded(header).unwrap_or_else(|e| panic!("{header:?} failed: {e:?}"));
        let nodes: Vec<&str> = hops.iter().filter_map(|hop| hop.for_node()).collect();
        assert_eq!(
            nodes, expected,
            "empty elements contribute no hop in {header:?}"
        );
    }

    // A dropped element leaves the ones around it untouched.
    let hops = parse_forwarded("for=192.0.2.43,,for=198.51.100.7").unwrap();
    assert_eq!(hops.len(), 2);
    assert_eq!(hops[1].for_node(), Some("198.51.100.7"));
}

#[test]
fn a_list_of_nothing_but_separators_is_still_an_error() {
    // `1#forwarded-element` needs at least one element. Accepting `,` would
    // turn a header with no hops into an empty list, which reads as "no proxy
    // spoke" rather than "this header is broken".
    for header in [",", ",,", ", ,", " ,", "\t,"] {
        assert!(
            parse_forwarded(header).is_err(),
            "{header:?} holds no element"
        );
    }
}

#[test]
fn an_empty_element_does_not_excuse_a_second_fault() {
    // The tolerance is for empty elements only. A header that also has a
    // syntax error is still rejected, so the leniency cannot be used to smuggle
    // a malformed element through.
    for header in [
        "for=192.0.2.43,,=x",       // a value with no name
        "for=192.0.2.43,,for",      // a name with no value
        "for=192.0.2.43,,host=a b", // an unquoted space
    ] {
        assert!(
            parse_forwarded(header).is_err(),
            "{header:?} has a fault beyond its empty element"
        );
    }
    // A node that is merely untyped is not a syntax error, so this one parses.
    let hops = parse_forwarded("for=192.0.2.43,,for=not-an-address").unwrap();
    assert_eq!(hops.len(), 2);
    assert_eq!(
        hops[1].node(),
        None,
        "the text parses, it is just not a node"
    );
}

#[test]
fn a_dangling_semicolon_before_a_comma_is_not_an_empty_element() {
    // `for=1.1.1.1;,` opens a parameter with `;` and never closes it. The comma
    // that follows is not an empty list element: it comes after an element that
    // has parameters, so it is a syntax error. Treating it as an empty element
    // would silently merge the next element into this one.
    let error = parse_forwarded("for=1.1.1.1;,for=2.2.2.2").unwrap_err();
    assert_eq!(error.position(), Some(12));
}

#[test]
fn a_trailing_semicolon_is_truncation_not_an_empty_element() {
    // The empty-element rule covers a trailing comma, where the list simply has
    // nothing after it. A trailing `;` is different: it opened a parameter and
    // the header ended, so the element is truncated. Accepting it dropped the
    // parameter and returned the element as if it were complete, which a
    // fresh-seed corpus caught.
    for header in [
        "for=1.1.1.1;",
        "for=1.1.1.1;proto",
        "zz=198.51.100.17;",
        "for=1.1.1.1, zz=198.51.100.17;",
        "for=1.1.1.1;;",
    ] {
        assert!(
            parse_forwarded(header).is_err(),
            "{header:?} ends inside a parameter"
        );
    }
}
