# proxy-addr

[![crates.io](https://img.shields.io/crates/v/proxy-addr.svg)](https://crates.io/crates/proxy-addr)
[![docs.rs](https://docs.rs/proxy-addr/badge.svg)](https://docs.rs/proxy-addr)

Determine the address of the client behind a reverse proxy.

A server behind a load balancer, an ingress, or a CDN sees the proxy's address
as its TCP peer, not the client's. The proxies append what they saw to
`X-Forwarded-For`, and the client is somewhere in that list. Reading the wrong
entry records a forged address, because the caller controls the left end of the
header: whatever they send sits at the **left**, and each proxy appends to the
**right**.

This crate resolves the address the way npm `proxy-addr` does, which is what
Express uses for `trust proxy`. You name the proxies you operate, and the client
is the first address from the right that is not one of yours.

## Example

```rust
use std::net::IpAddr;
use proxy_addr::{addr, all, Trust};

let peer: IpAddr = "10.0.0.5".parse().unwrap();

// The ingress appended the real client (198.51.100.7) to the right; the
// caller pre-seeded 1.2.3.4 at the left. Reading the left would record the
// forgery.
let header = "1.2.3.4, 198.51.100.7";
let ingress = Trust::parse("10.0.0.0/8").unwrap();
assert_eq!(addr(peer, Some(header), &ingress), "198.51.100.7");

// `all` keeps every hop up to and including the first untrusted one.
assert_eq!(all(peer, Some(header), Some(&ingress)), ["10.0.0.5", "198.51.100.7"]);

// A caller who reaches the server directly cannot choose the address recorded.
let direct: IpAddr = "203.0.113.9".parse().unwrap();
assert_eq!(addr(direct, Some(header), &ingress), "203.0.113.9");
```

## What it does

- **Resolves the client address.** [`addr`] returns the address to attribute the
  request to; [`all`] returns the chain of addresses up to the first untrusted
  hop; [`forwarded`] returns the raw list, peer first.
- **Trusts only what you name.** A [`Trust`] is built from addresses, CIDR
  blocks (`10.0.0.0/8`), IPv4 netmasks (`10.0.0.0/255.0.0.0`), or the three
  range names `loopback`, `linklocal` and `uniquelocal`. An address is trusted
  when it falls in any of them; an empty set trusts nobody.
- **Fails loudly on a bad specification.** A typo in a proxy list is a
  configuration error and is returned as an [`Error`], never silently widened.
- **Takes no framework types.** The peer is a [`std::net::IpAddr`] and the
  header is a `&str`, so the crate fits any HTTP stack.
- **Reads the standard header too.** [`parse_forwarded`] parses RFC 7239
  `Forwarded`, where each hop names itself instead of being inferred.

## RFC 7239 `Forwarded`

`X-Forwarded-For` is a list of addresses and nothing else, so which entry is a
proxy has to be guessed from your trust list. `Forwarded` says so directly: each
element carries parameters, and `for` names the node that made the request while
`by` names the proxy that received it.

```rust
use proxy_addr::{parse_forwarded, Node};

// Two hops, in header order: the client first, the nearest proxy last.
let hops = parse_forwarded("for=192.0.2.43, for=\"[2001:db8::1]:4711\"").unwrap();
assert_eq!(hops.len(), 2);
assert_eq!(hops[0].for_node(), Some("192.0.2.43"));
assert_eq!(hops[0].node(), Some(Node::Address {
    ip: "192.0.2.43".parse().unwrap(),
    port: None,
}));

// The other parameters a proxy adds.
let hops = parse_forwarded(r#"for=192.0.2.60;proto=https;by=203.0.113.43"#).unwrap();
assert_eq!(hops[0].proto(), Some("https"));
assert_eq!(hops[0].by(), Some("203.0.113.43"));
```

A `for` or `by` value is one of three things, and [`Node`] tells them apart: an
address with an optional port, the word `unknown` for a node that declines to
identify itself, or an obfuscated name such as `_hidden`.

Order is the one trap. RFC 7239 appends each hop, so the **client is first** and
the nearest proxy last - the reverse of `X-Forwarded-For`, which this crate
returns peer first. `parse_forwarded` keeps the header's own order rather than
reversing it, because the RFC's order is the meaningful one and reversing it
twice is how a client ends up attributed to a proxy.

Quoting is the second. A value may only be a bare token when it is made of token
characters, so a colon, slash, bracket or non-ASCII byte has to be inside a
quoted-string: `for=[2001:db8::1]` is a syntax error, and
`for="[2001:db8::1]"` is not. The parser rejects the first, as the reference
does, rather than guessing. Inside quotes, a backslash escapes the next
character, so `for="\"quoted\""` reads as `"quoted"`.

A header that does not parse returns a [`ForwardedError`] carrying the character
index where it stopped, or no index when it ended in the middle of an element. A
malformed header is not partially trusted.

## IPv4-mapped IPv6

An IPv6 address that carries an IPv4 one (`::ffff:10.0.0.5`) is folded to its
IPv4 form before matching, so a proxy configured as `10.0.0.0/8` still trusts a
peer that connects over a dual-stack socket as `::ffff:10.0.0.5`. This is the
behaviour of the reference's `ipaddr.js`, and it is what stops a mapped address
from bypassing the trust list.

## IPv6 zone identifiers

`fe80::1%eth0` carries a zone identifier naming the interface. The reference
accepts one and ignores it when matching, so `fe80::1%eth0` and `fe80::1` trust
the same network; the crate does the same. Rust's own parser rejects the `%`
suffix, so the crate splits it off before parsing, and it applies only to an
IPv6 address, as in the reference.

## Edge cases

- **Out-of-range prefixes.** A prefix of `0` or one longer than the family's
  width is rejected with an [`Error`], matching the reference, rather than
  treated as "match everything".
- **Whitespace.** Only ASCII spaces are trimmed from a header entry, as in the
  reference, so a tab inside an entry is not stripped.

## Licence

Apache-2.0. See `LICENSE` and `NOTICE`. The resolution behaviour follows the npm
`proxy-addr` package and the `forwarded` and `ipaddr.js` libraries it builds on,
all MIT-licensed, and `Forwarded` parsing follows `forwarded-parse`
<<https://github.com/lpinca/forwarded-parse>>, also MIT-licensed. This crate is
an independent reimplementation and shares no code with them.
