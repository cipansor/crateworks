//! Determine the address of the client behind a reverse proxy.
//!
//! A server usually sits behind one or more proxies — a load balancer, an
//! ingress, a CDN — so the TCP peer of a request is a proxy, not the client.
//! Each proxy appends the address it saw to the `X-Forwarded-For` header, and
//! the client's address is somewhere in that list. Picking the wrong entry
//! records a forged address, because the caller can seed the header: the value
//! they send is at the **left**, and each proxy appends to the **right**.
//!
//! This crate resolves the address the way npm `proxy-addr` does, which is what
//! Express uses for `trust proxy`: you name the proxies you operate, and the
//! client is the first address from the right that is *not* one of yours.
//!
//! ```
//! use std::net::IpAddr;
//! use proxy_addr::{addr, all, Trust};
//!
//! let peer: IpAddr = "10.0.0.5".parse().unwrap();
//! // The ingress appended the real client; the client pre-seeded a forgery.
//! let header = "1.2.3.4, 198.51.100.7";
//!
//! // Trusting the ingress's network resolves to the client, not the forgery.
//! let ingress = Trust::parse("10.0.0.0/8").unwrap();
//! assert_eq!(addr(peer, Some(header), &ingress), "198.51.100.7");
//!
//! // `all` keeps every hop up to and including the first untrusted one.
//! assert_eq!(all(peer, Some(header), Some(&ingress)), ["10.0.0.5", "198.51.100.7"]);
//! ```
//!
//! # The rule
//!
//! 1. If the peer is not a proxy you named, the header is ignored and the peer
//!    is the answer. A caller who talks to your server directly cannot choose
//!    the address you record.
//! 2. Otherwise walk the list from the right, skipping every entry that is one
//!    of your proxies. The first entry that is not is the client.
//! 3. An entry that does not parse as an address ends the walk, and the last
//!    address that was verified is used. The crate never trusts past a value it
//!    cannot read.
//!
//! # Trust specifications
//!
//! A trust entry is an address, a CIDR (`10.0.0.0/8`), an IPv4 netmask
//! (`10.0.0.0/255.0.0.0`), or one of three names that expand to the ranges
//! below. Several entries are given as a list, and an address is trusted when
//! it falls in any of them. An empty list trusts nobody.
//!
//! | Name | Ranges |
//! |---|---|
//! | `loopback` | `127.0.0.1/8`, `::1/128` |
//! | `linklocal` | `169.254.0.0/16`, `fe80::/10` |
//! | `uniquelocal` | `10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7` |
//!
//! ```
//! use proxy_addr::Trust;
//! let lan = Trust::parse("uniquelocal").unwrap();
//! assert!(lan.is_trusted_str("10.1.2.3"));
//! assert!(lan.is_trusted_str("192.168.0.1"));
//! assert!(!lan.is_trusted_str("8.8.8.8"));
//! ```
//!
//! # What this crate is
//!
//! An independent Rust implementation of the resolution `proxy-addr` performs,
//! matching its behaviour and that of the `forwarded` and `ipaddr.js` libraries
//! it builds on, including the treatment of IPv4-mapped IPv6 addresses. It has
//! no dependencies and takes no framework types: the peer is an [`IpAddr`] and
//! the header is a `&str`, so it fits any HTTP stack.

#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod ip;

use std::fmt;
use std::net::IpAddr;

use ip::Subnet;

/// The predefined ranges a trust entry may name instead of spelling out.
///
/// Each name expands to the networks in [`RANGES`].
pub const LOOPBACK: &[&str] = &["127.0.0.1/8", "::1/128"];
/// The link-local ranges (`169.254.0.0/16`, `fe80::/10`).
pub const LINKLOCAL: &[&str] = &["169.254.0.0/16", "fe80::/10"];
/// The private ranges (`10.0.0.0/8`, `172.16.0.0/12`, `192.168.0.0/16`, `fc00::/7`).
pub const UNIQUELOCAL: &[&str] = &["10.0.0.0/8", "172.16.0.0/12", "192.168.0.0/16", "fc00::/7"];

/// The names a trust entry may use, each mapped to the ranges it stands for.
///
/// The three names and their ranges are the ones the reference defines.
pub const RANGES: &[(&str, &[&str])] = &[
    ("loopback", LOOPBACK),
    ("linklocal", LINKLOCAL),
    ("uniquelocal", UNIQUELOCAL),
];

/// Why a trust specification was rejected.
///
/// A rejected specification is a configuration error, so the variants name the
/// offending text: a typo in a proxy list must fail loudly at startup, never
/// silently trust the wrong network.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The specification was empty.
    Missing,
    /// A trust entry that is neither an address, a range, nor a known name.
    InvalidAddress(String),
    /// A range whose prefix is out of bounds, zero, or a netmask with a hole.
    InvalidRange(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("a trust specification is required"),
            Self::InvalidAddress(addr) => write!(f, "invalid IP address: {addr}"),
            Self::InvalidRange(note) => write!(f, "invalid range on address: {note}"),
        }
    }
}

impl std::error::Error for Error {}

/// A compiled set of trusted networks.
///
/// Build one with [`Trust::parse`] for a single entry or [`Trust::parse_list`]
/// for several, then pass it to [`addr`] or [`all`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Trust {
    subnets: Vec<Subnet>,
}

impl Trust {
    /// Trust nobody. [`addr`] then returns the peer, because the header is
    /// never believed.
    pub fn none() -> Self {
        Self::default()
    }

    /// Compile one trust entry: an address, a CIDR, an IPv4 netmask, or a
    /// range name (`loopback`, `linklocal`, `uniquelocal`).
    ///
    /// ```
    /// use proxy_addr::Trust;
    /// assert!(Trust::parse("10.0.0.0/8").unwrap().is_trusted_str("10.1.2.3"));
    /// assert!(Trust::parse("loopback").unwrap().is_trusted_str("::1"));
    /// assert!(Trust::parse("").is_err());
    /// ```
    pub fn parse(spec: &str) -> Result<Self, Error> {
        if spec.is_empty() {
            return Err(Error::Missing);
        }
        let mut subnets = Vec::new();
        push_entry(&mut subnets, spec)?;
        Ok(Self { subnets })
    }

    /// Compile several trust entries. An empty list trusts nobody, exactly as
    /// the reference's empty array does.
    ///
    /// ```
    /// use proxy_addr::Trust;
    /// let t = Trust::parse_list(["10.0.0.0/8", "loopback"]).unwrap();
    /// assert!(t.is_trusted_str("10.0.0.5"));
    /// assert!(t.is_trusted_str("127.0.0.1"));
    /// assert!(!t.is_trusted_str("8.8.8.8"));
    /// ```
    pub fn parse_list<I, S>(specs: I) -> Result<Self, Error>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let mut subnets = Vec::new();
        for spec in specs {
            push_entry(&mut subnets, spec.as_ref())?;
        }
        Ok(Self { subnets })
    }

    /// Does this set trust nobody?
    pub fn is_empty(&self) -> bool {
        self.subnets.is_empty()
    }

    /// Is `addr` inside one of the trusted networks?
    pub fn is_trusted(&self, addr: IpAddr) -> bool {
        ip::is_trusted(&self.subnets, addr)
    }

    /// Is the address written in `text` inside one of the trusted networks?
    ///
    /// Text that does not parse as an address is never trusted, which is the
    /// reference's behaviour: a header entry it cannot read cannot be one of
    /// your proxies. The text is not trimmed first, so `" 10.0.0.5"` is not an
    /// address and is not trusted, but an IPv6 zone suffix (`fe80::1%eth0`) is
    /// accepted and ignored, as the reference does.
    pub fn is_trusted_str(&self, text: &str) -> bool {
        match ip::parse_addr(text) {
            Some(addr) => self.is_trusted(addr),
            None => false,
        }
    }
}

/// Expand a range name or parse a network, appending to `subnets`.
fn push_entry(subnets: &mut Vec<Subnet>, spec: &str) -> Result<(), Error> {
    if let Some((_, ranges)) = RANGES.iter().find(|(name, _)| *name == spec) {
        for range in *ranges {
            subnets.push(Subnet::parse(range).map_err(Error::InvalidRange)?);
        }
        return Ok(());
    }
    subnets.push(Subnet::parse(spec).map_err(classify)?);
    Ok(())
}

/// The reference reports a bad address and a bad range with different messages,
/// so the two are told apart here rather than collapsed.
fn classify(message: String) -> Error {
    if message.starts_with("invalid IP address") {
        Error::InvalidAddress(
            message
                .trim_start_matches("invalid IP address: ")
                .to_string(),
        )
    } else {
        Error::InvalidRange(
            message
                .trim_start_matches("invalid range on address: ")
                .to_string(),
        )
    }
}

/// Every address the request names, the TCP peer first and the leftmost
/// `X-Forwarded-For` entry last.
///
/// The list is returned as written: an entry that is not an address is kept, so
/// a caller can see it, and [`Trust`] treats it as untrusted.
///
/// ```
/// use std::net::IpAddr;
/// use proxy_addr::forwarded;
/// let peer: IpAddr = "10.0.0.5".parse().unwrap();
/// assert_eq!(forwarded(peer, Some("1.2.3.4, 5.6.7.8")), ["10.0.0.5", "5.6.7.8", "1.2.3.4"]);
/// assert_eq!(forwarded(peer, None), ["10.0.0.5"]);
/// ```
pub fn forwarded(peer: IpAddr, x_forwarded_for: Option<&str>) -> Vec<String> {
    let mut addrs = vec![peer.to_string()];
    if let Some(header) = x_forwarded_for {
        addrs.extend(parse_header(header));
    }
    addrs
}

/// The addresses up to and including the first one that is not trusted.
///
/// With `trust` as `None` the header is not consulted at all and every address
/// is returned; with an empty [`Trust`] only the peer survives, because the
/// first address is already untrusted.
///
/// ```
/// use std::net::IpAddr;
/// use proxy_addr::{all, Trust};
/// let peer: IpAddr = "10.0.0.5".parse().unwrap();
/// let ingress = Trust::parse("10.0.0.0/8").unwrap();
/// assert_eq!(
///     all(peer, Some("1.2.3.4, 198.51.100.7"), Some(&ingress)),
///     ["10.0.0.5", "198.51.100.7"]
/// );
/// assert_eq!(all(peer, Some("1.2.3.4"), None), ["10.0.0.5", "1.2.3.4"]);
/// ```
pub fn all(peer: IpAddr, x_forwarded_for: Option<&str>, trust: Option<&Trust>) -> Vec<String> {
    let mut addrs = forwarded(peer, x_forwarded_for);
    let Some(trust) = trust else {
        return addrs;
    };
    for (i, addr) in addrs.iter().enumerate().take(addrs.len().saturating_sub(1)) {
        if trust.is_trusted_str(addr) {
            continue;
        }
        addrs.truncate(i + 1);
        break;
    }
    addrs
}

/// The address to attribute the request to: the last address [`all`] keeps.
///
/// ```
/// use std::net::IpAddr;
/// use proxy_addr::{addr, Trust};
/// let peer: IpAddr = "10.0.0.5".parse().unwrap();
/// let ingress = Trust::parse("10.0.0.0/8").unwrap();
/// assert_eq!(addr(peer, Some("1.2.3.4, 198.51.100.7"), &ingress), "198.51.100.7");
/// // A peer that is not one of ours cannot choose its own address.
/// let client: IpAddr = "203.0.113.9".parse().unwrap();
/// assert_eq!(addr(client, Some("1.2.3.4"), &ingress), "203.0.113.9");
/// ```
pub fn addr(peer: IpAddr, x_forwarded_for: Option<&str>, trust: &Trust) -> String {
    let addrs = all(peer, x_forwarded_for, Some(trust));
    addrs
        .into_iter()
        .next_back()
        .unwrap_or_else(|| peer.to_string())
}

/// Split an `X-Forwarded-For` header into entries, rightmost first.
///
/// The reference walks the header backwards and trims only ASCII spaces
/// (`0x20`), not tabs, and drops empty entries, so `"a, , b"` is `["b", "a"]`.
fn parse_header(header: &str) -> Vec<String> {
    let mut list: Vec<String> = header
        .split(',')
        .map(|part| part.trim_matches(' '))
        .filter(|part| !part.is_empty())
        .map(str::to_string)
        .collect();
    list.reverse();
    list
}
