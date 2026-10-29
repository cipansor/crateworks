//! Address parsing and CIDR matching.
//!
//! This is the part of the crate that answers "is this address inside that
//! network". It is written to agree with the JavaScript `ipaddr.js` library,
//! which is what npm `proxy-addr` uses, including its treatment of IPv4-mapped
//! IPv6 addresses: a subnet and a candidate in different families only match
//! when the IPv6 side is an IPv4-mapped network whose prefix reaches the
//! `::ffff:` marker.

use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

/// One compiled trust entry: a network address and its prefix length.
///
/// The prefix is stored as a bit count, `0` excluded, because the reference
/// rejects a zero-length range rather than treating it as "everything".
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Subnet {
    pub(crate) addr: IpAddr,
    pub(crate) prefix: u8,
}

impl Subnet {
    /// Parse one trust entry, in the notation the reference accepts:
    /// `10.0.0.0/8`, `10.0.0.0`, `::1`, `fe80::/10`, or an IPv4 netmask such
    /// as `10.0.0.0/255.0.0.0`.
    ///
    /// A bare IPv4-mapped IPv6 address (`::ffff:10.0.0.0`) is folded to its
    /// IPv4 form, which is what `ipaddr.js` does when no `/` is present.
    pub(crate) fn parse(note: &str) -> Result<Self, String> {
        let (head, range) = match note.rfind('/') {
            Some(pos) => (&note[..pos], Some(&note[pos + 1..])),
            None => (note, None),
        };

        let parsed = parse_addr(head).ok_or_else(|| format!("invalid IP address: {head}"))?;
        let addr = match parsed {
            IpAddr::V6(v6) if range.is_none() => match v6.to_ipv4_mapped() {
                Some(v4) => IpAddr::V4(v4),
                None => IpAddr::V6(v6),
            },
            other => other,
        };

        let max = match addr {
            IpAddr::V4(_) => 32u8,
            IpAddr::V6(_) => 128u8,
        };

        let prefix = match range {
            None => Some(max),
            Some(r) if !r.is_empty() && r.bytes().all(|b| b.is_ascii_digit()) => r
                .parse::<u16>()
                .ok()
                .filter(|v| *v <= u16::from(max))
                .map(|v| v as u8),
            Some(r) => match (addr, parse_addr(r)) {
                (IpAddr::V4(_), Some(IpAddr::V4(mask))) => prefix_from_subnet_mask(mask),
                _ => None,
            },
        };

        match prefix {
            Some(p) if p > 0 && p <= max => Ok(Self { addr, prefix: p }),
            _ => Err(format!("invalid range on address: {note}")),
        }
    }

    /// Does `candidate` fall inside this network?
    ///
    /// The candidate must already be canonicalised (an IPv4-mapped IPv6 address
    /// folded to IPv4), which is what [`is_trusted`] does before calling.
    pub(crate) fn contains(&self, candidate: IpAddr) -> bool {
        let subnet_v6 = matches!(self.addr, IpAddr::V6(_));
        let cand_v6 = matches!(candidate, IpAddr::V6(_));

        let matched = if cand_v6 != subnet_v6 {
            if !subnet_v6 {
                // An IPv6 candidate never falls in an IPv4 network.
                return false;
            }
            // A mapped IPv6 network only spans IPv4 when its prefix covers the
            // `::ffff:` marker; a shorter prefix must not match.
            if !(self.prefix >= 96 && is_v4_mapped(&self.addr)) {
                return false;
            }
            to_v4_mapped(candidate)
        } else {
            if cand_v6 && is_v4_mapped(&self.addr) {
                // A native IPv6 candidate cannot match an IPv4-mapped network.
                return false;
            }
            candidate
        };

        match_cidr(matched, self.addr, self.prefix)
    }
}

/// Parse an address, dropping an IPv6 zone identifier if one is present.
///
/// `ipaddr.js` accepts a `%` suffix on an IPv6 address and then ignores it when
/// matching, so `fe80::1%eth0` and `fe80::1` trust the same. Rust's parser
/// rejects the suffix, so it is split off here. The zone must be non-empty and
/// ASCII alphanumeric, which is the reference's `%[0-9a-z]+` rule; it never
/// applies to an IPv4 address.
pub(crate) fn parse_addr(text: &str) -> Option<IpAddr> {
    if let Ok(addr) = text.parse::<IpAddr>() {
        return Some(addr);
    }
    let (head, zone) = text.split_once('%')?;
    if zone.is_empty() || !zone.bytes().all(|b| b.is_ascii_alphanumeric()) {
        return None;
    }
    head.parse::<Ipv6Addr>().ok().map(IpAddr::V6)
}

/// Fold an IPv4-mapped IPv6 address (`::ffff:a.b.c.d`) to IPv4, leaving every
/// other address alone.
pub(crate) fn canonicalise(addr: IpAddr) -> IpAddr {
    match addr {
        IpAddr::V6(v6) => match v6.to_ipv4_mapped() {
            Some(v4) => IpAddr::V4(v4),
            None => addr,
        },
        other => other,
    }
}

/// Is this address the IPv4-mapped form `::ffff:a.b.c.d`?
fn is_v4_mapped(addr: &IpAddr) -> bool {
    matches!(addr, IpAddr::V6(v6) if v6.to_ipv4_mapped().is_some())
}

/// Render an IPv4 address as its mapped IPv6 form, `::ffff:a.b.c.d`.
fn to_v4_mapped(addr: IpAddr) -> IpAddr {
    match addr {
        IpAddr::V4(v4) => IpAddr::V6(v4.to_ipv6_mapped()),
        other => other,
    }
}

/// Compare two addresses bit by bit over `prefix` bits.
///
/// The two must be in the same family; a caller that reaches here with a
/// mismatched pair has a bug, and the slices would differ in length.
fn match_cidr(first: IpAddr, second: IpAddr, prefix: u8) -> bool {
    let (first, second, part_size): (Vec<u32>, Vec<u32>, i32) = match (first, second) {
        (IpAddr::V4(a), IpAddr::V4(b)) => (
            a.octets().iter().map(|o| u32::from(*o)).collect(),
            b.octets().iter().map(|o| u32::from(*o)).collect(),
            8,
        ),
        (IpAddr::V6(a), IpAddr::V6(b)) => (
            a.segments().iter().map(|s| u32::from(*s)).collect(),
            b.segments().iter().map(|s| u32::from(*s)).collect(),
            16,
        ),
        _ => return false,
    };

    let mut remaining = i32::from(prefix);
    let mut part = 0usize;
    while remaining > 0 {
        let shift = (part_size - remaining).max(0) as u32;
        if first[part] >> shift != second[part] >> shift {
            return false;
        }
        remaining -= part_size;
        part += 1;
    }
    true
}

/// The number of leading one-bits in a contiguous IPv4 netmask, or `None` when
/// the mask has a hole (`255.0.255.0`) and therefore names no network.
fn prefix_from_subnet_mask(mask: Ipv4Addr) -> Option<u8> {
    let zeros = |octet: u8| -> Option<u8> {
        Some(match octet {
            0 => 8,
            128 => 7,
            192 => 6,
            224 => 5,
            240 => 4,
            248 => 3,
            252 => 2,
            254 => 1,
            255 => 0,
            _ => return None,
        })
    };

    let octets = mask.octets();
    let mut zero_bits = 0u16;
    let mut seen_one = false;
    for &octet in octets.iter().rev() {
        let z = zeros(octet)?;
        if seen_one && z != 0 {
            return None;
        }
        if z != 8 {
            seen_one = true;
        }
        zero_bits += u16::from(z);
    }
    Some((32 - zero_bits) as u8)
}

/// Is `addr` inside any of `subnets`? An empty set trusts nobody.
pub(crate) fn is_trusted(subnets: &[Subnet], addr: IpAddr) -> bool {
    if subnets.is_empty() {
        return false;
    }
    let candidate = canonicalise(addr);
    subnets.iter().any(|subnet| subnet.contains(candidate))
}
