//! Behaviour of the typed layer over a parsed `Forwarded` element.
//!
//! `tests/forwarded_oracle.rs` checks the parser against npm `forwarded-parse`,
//! which works in strings and has no notion of an address. `Node::parse` is
//! this crate's addition, so its expectations are RFC 7239 section 6.3 read
//! directly rather than taken from that reference.

use std::net::IpAddr;

use proxy_addr::{Node, parse_forwarded};

fn ip(text: &str) -> IpAddr {
    text.parse().expect("test address parses")
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
    assert_eq!(
        Node::parse("192.0.2.43"),
        Some(Node::Address {
            ip: ip("192.0.2.43"),
            port: None
        })
    );
    assert_eq!(
        Node::parse("192.0.2.43:4711"),
        Some(Node::Address {
            ip: ip("192.0.2.43"),
            port: Some(4711)
        })
    );
    assert_eq!(
        Node::parse("[2001:db8::1]"),
        Some(Node::Address {
            ip: ip("2001:db8::1"),
            port: None
        })
    );
    assert_eq!(
        Node::parse("[2001:db8::1]:4711"),
        Some(Node::Address {
            ip: ip("2001:db8::1"),
            port: Some(4711)
        })
    );
}

#[test]
fn reads_the_two_non_address_forms() {
    assert_eq!(Node::parse("unknown"), Some(Node::Unknown));
    assert_eq!(
        Node::parse("_hidden"),
        Some(Node::Obfuscated("hidden".to_string()))
    );
    // An obfuscated name is a token, so a space or a colon is not one.
    assert_eq!(Node::parse("_a b"), None);
    assert_eq!(Node::parse("_"), None);
}

#[test]
fn rejects_text_the_rfc_does_not_allow_as_a_node() {
    // A bare IPv6 address is ambiguous with `address:port`, so RFC 7239
    // requires the brackets. Accepting it would read `2001:db8::1` as host
    // `2001:db8:` with port `1`.
    assert_eq!(Node::parse("2001:db8::1"), None);
    // A port that is not a number, or does not fit, is not a node.
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
        "[2001:db8::1]",
        "[2001:db8::1]:4711",
        "unknown",
        "_hidden",
    ] {
        let node = Node::parse(text).unwrap_or_else(|| panic!("{text} should parse"));
        assert_eq!(node.to_string(), text, "round trip of {text}");
    }
}

#[test]
fn a_quoted_node_reaches_the_typed_layer_unescaped() {
    // The header quotes an address because a colon is not a token character;
    // by the time `Node::parse` sees it the quotes are gone.
    let hops = parse_forwarded(r#"for="[2001:db8:cafe::17]:4711""#).unwrap();
    assert_eq!(
        hops[0].node(),
        Some(Node::Address {
            ip: ip("2001:db8:cafe::17"),
            port: Some(4711)
        })
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
