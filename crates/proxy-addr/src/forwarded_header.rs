//! The RFC 7239 `Forwarded` header field.
//!
//! `X-Forwarded-For` is a list of addresses and nothing else, which is why
//! [`crate::addr`] has to guess which entry is a proxy. `Forwarded` says so:
//! each element is a set of parameters, and `for` names the node that made the
//! request while `by` names the proxy that received it. A chain of them is a
//! chain of hops, and the proxy is identified rather than inferred.
//!
//! This is a port of the npm `forwarded-parse` package
//! <<https://github.com/lpinca/forwarded-parse>>, which is the reference for the
//! accept/reject decision here. The grammar is HTTP's: `token` and
//! `quoted-string` from RFC 7230 section 3.2.6, and the parameter list from
//! RFC 7239 section 4.

use std::fmt;
use std::net::IpAddr;

/// One element of a `Forwarded` header: the parameters for a single hop.
///
/// Parameter names are lowercased, as the reference lowercases them, and a
/// name that appears twice in one element keeps its first position and its
/// last value, which is what assigning to a JavaScript object property does.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ForwardedElement {
    params: Vec<(String, String)>,
}

impl ForwardedElement {
    /// The `for` parameter: the node that made the request.
    ///
    /// This is the raw text, which may be a quoted address with a port, an
    /// IPv6 address in brackets, `unknown`, or an obfuscated identifier
    /// starting with an underscore. [`Node::parse`] reads it into a typed
    /// value.
    pub fn for_node(&self) -> Option<&str> {
        self.param("for")
    }

    /// The `by` parameter: the proxy that received the request.
    pub fn by(&self) -> Option<&str> {
        self.param("by")
    }

    /// The `proto` parameter: the protocol of the request as the client made
    /// it, such as `https` when the proxy terminated TLS.
    pub fn proto(&self) -> Option<&str> {
        self.param("proto")
    }

    /// The `host` parameter: the `Host` the client asked for, which is how a
    /// proxy forwards a virtual host that it is not itself serving.
    pub fn host(&self) -> Option<&str> {
        self.param("host")
    }

    /// Any parameter by name, lowercased, or `None` if the element lacks it.
    ///
    /// This is how a parameter RFC 7239 does not define is reached; the four
    /// accessors above are the ones it does.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, value)| value.as_str())
    }

    /// Every parameter in the element, in the order it first appeared.
    pub fn params(&self) -> impl Iterator<Item = (&str, &str)> + '_ {
        self.params
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str()))
    }

    /// Is the element empty? The reference can produce one for a header that
    /// is a bare `,`.
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// The `for` parameter read as a node.
    ///
    /// ```
    /// use proxy_addr::{Node, parse_forwarded};
    ///
    /// let hops = parse_forwarded("for=\"[2001:db8::1]:4711\"").unwrap();
    /// let node = hops[0].for_node().and_then(Node::parse).unwrap();
    /// assert_eq!(node.to_string(), "[2001:db8::1]:4711");
    /// ```
    pub fn node(&self) -> Option<Node> {
        self.for_node().and_then(Node::parse)
    }
}

/// Why a `Forwarded` header could not be read.
///
/// A malformed header is attacker-controlled input, so the parser reports
/// where it stopped rather than guessing: a header that cannot be read in full
/// is not partially trusted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForwardedError {
    position: Option<usize>,
}

impl ForwardedError {
    /// The character index at which parsing failed, or `None` when the header
    /// ended in the middle of an element.
    ///
    /// The index counts characters, not bytes, so it lines up with the header
    /// text a caller prints.
    pub fn position(&self) -> Option<usize> {
        self.position
    }
}

impl fmt::Display for ForwardedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.position {
            Some(index) => write!(f, "unexpected character at index {index}"),
            None => f.write_str("unexpected end of the Forwarded header"),
        }
    }
}

impl std::error::Error for ForwardedError {}

/// A node identifier from a `for` or `by` parameter.
///
/// RFC 7239 section 6.3 lets a node be an address, the word `unknown`, or an
/// obfuscated identifier, and an address may carry the client's port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// An address, and the port when the parameter gave one.
    Address {
        /// The address the node is reachable at.
        ip: IpAddr,
        /// The port, when the parameter carried one. A client's source port is
        /// not an identifier and is usually omitted.
        port: Option<u16>,
    },
    /// The node deliberately did not identify itself (`unknown`).
    Unknown,
    /// The node identified itself with a pseudonym (`_hidden`). The name is
    /// the identifier without the leading underscore.
    Obfuscated(String),
}

impl Node {
    /// Read a `for` or `by` value.
    ///
    /// Returns `None` for text that is not a node under RFC 7239: an
    /// unbracketed IPv6 address, a port that is not a number, or anything the
    /// grammar does not allow. The value is taken as it appears in the header,
    /// so a quoted-string has already been unescaped by [`parse_forwarded`].
    ///
    /// ```
    /// use proxy_addr::Node;
    /// use std::net::IpAddr;
    ///
    /// let ip: IpAddr = "192.0.2.43".parse().unwrap();
    /// assert_eq!(Node::parse("192.0.2.43"), Some(Node::Address { ip, port: None }));
    /// assert_eq!(Node::parse("unknown"), Some(Node::Unknown));
    /// assert_eq!(Node::parse("_hidden"), Some(Node::Obfuscated("hidden".into())));
    ///
    /// // An IPv6 node has to be bracketed, and may carry a port.
    /// assert!(Node::parse("2001:db8::1").is_none());
    /// let ip: IpAddr = "2001:db8::1".parse().unwrap();
    /// assert_eq!(
    ///     Node::parse("[2001:db8::1]:4711"),
    ///     Some(Node::Address { ip, port: Some(4711) })
    /// );
    /// ```
    pub fn parse(value: &str) -> Option<Self> {
        if value == "unknown" {
            return Some(Self::Unknown);
        }
        if let Some(name) = value.strip_prefix('_') {
            return is_token(name).then(|| Self::Obfuscated(name.to_string()));
        }
        if let Some(rest) = value.strip_prefix('[') {
            let (inside, after) = rest.split_once(']')?;
            let ip: IpAddr = inside.parse().ok()?;
            return match after {
                "" => Some(Self::Address { ip, port: None }),
                _ => {
                    let port = after.strip_prefix(':')?.parse().ok()?;
                    Some(Self::Address {
                        ip,
                        port: Some(port),
                    })
                }
            };
        }
        let (host, port) = match value.split_once(':') {
            Some((host, port)) => (host, Some(port)),
            None => (value, None),
        };
        // Only an IPv4 literal reaches here: a bare IPv6 address has colons of
        // its own and the split above would have taken the wrong one.
        let ip: IpAddr = host.parse().ok()?;
        if ip.is_ipv6() {
            return None;
        }
        let port = match port {
            Some(port) => Some(port.parse().ok()?),
            None => None,
        };
        Some(Self::Address { ip, port })
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Address { ip, port } => match (ip, port) {
                // An IPv6 node keeps its brackets even without a port, so that
                // `Display` and `Node::parse` are inverses: unbracketed, the
                // address's own colons would read back as a port separator.
                (IpAddr::V6(_), Some(port)) => write!(f, "[{ip}]:{port}"),
                (IpAddr::V6(_), None) => write!(f, "[{ip}]"),
                (_, Some(port)) => write!(f, "{ip}:{port}"),
                (_, None) => write!(f, "{ip}"),
            },
            Self::Unknown => f.write_str("unknown"),
            Self::Obfuscated(name) => write!(f, "_{name}"),
        }
    }
}

/// Read a `Forwarded` header field value into one element per hop.
///
/// The elements are in header order, which is the order the hops were added:
/// the first element is the client's, the last is the one the nearest proxy
/// appended. Nothing here is reversed, unlike `X-Forwarded-For` in
/// [`crate::forwarded`], because RFC 7239 puts the original request first.
///
/// A header that does not parse is an error rather than a partial result,
/// because a truncated element would attribute the request to the wrong node.
///
/// ```
/// use proxy_addr::parse_forwarded;
///
/// let hops = parse_forwarded("for=192.0.2.43, for=\"[2001:db8::1]:4711\"").unwrap();
/// assert_eq!(hops.len(), 2);
/// assert_eq!(hops[0].for_node(), Some("192.0.2.43"));
/// assert_eq!(hops[1].for_node(), Some("[2001:db8::1]:4711"));
///
/// let hops = parse_forwarded("for=192.0.2.60;proto=https;by=203.0.113.43").unwrap();
/// assert_eq!(hops[0].proto(), Some("https"));
/// assert_eq!(hops[0].by(), Some("203.0.113.43"));
///
/// // A header with no parameters is not an element.
/// assert!(parse_forwarded("").is_err());
/// ```
pub fn parse_forwarded(header: &str) -> Result<Vec<ForwardedElement>, ForwardedError> {
    let chars: Vec<char> = header.chars().collect();
    let mut output = Vec::new();
    let mut params: Vec<(String, String)> = Vec::new();
    // `None` until the `=` that ends a parameter name.
    let mut parameter: Option<String> = None;
    let mut start: Option<usize> = None;
    let mut end: Option<usize> = None;
    let mut in_quotes = false;
    let mut is_escaping = false;
    // Set once any backslash is seen, and never cleared: the reference keeps
    // this as one flag for the whole header, so a later parameter is decoded
    // even when its own text has no backslash. Matching that is what keeps the
    // two parsers in agreement.
    let mut must_unescape = false;

    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];

        let Some(name) = parameter.as_ref() else {
            // Leading whitespace before the first name is skipped, but not at
            // index 0: the reference rejects a header that starts with a space.
            if i != 0 && start.is_none() && (c == ' ' || c == '\t') {
                i += 1;
                continue;
            }
            if is_token_char(c) {
                if start.is_none() {
                    start = Some(i);
                }
            } else if c == '=' && start.is_some() {
                parameter = Some(
                    chars[start.unwrap()..i]
                        .iter()
                        .collect::<String>()
                        .to_lowercase(),
                );
                start = None;
            } else {
                return Err(ForwardedError { position: Some(i) });
            }
            i += 1;
            continue;
        };

        let name = name.clone();

        if is_escaping && (c == '\t' || is_print(c) || is_extended(c)) {
            is_escaping = false;
        } else if is_token_char(c) {
            if end.is_some() {
                return Err(ForwardedError { position: Some(i) });
            }
            if start.is_none() {
                start = Some(i);
            }
        } else if is_delimiter(c) || is_extended(c) {
            if in_quotes {
                if c == '"' {
                    in_quotes = false;
                    end = Some(i);
                } else if c == '\\' {
                    if start.is_none() {
                        start = Some(i);
                    }
                    is_escaping = true;
                    must_unescape = true;
                } else if start.is_none() {
                    start = Some(i);
                }
            } else if c == '"' && i > 0 && chars[i - 1] == '=' {
                in_quotes = true;
            } else if (c == ',' || c == ';') && (start.is_some() || end.is_some()) {
                let value = match start {
                    Some(from) => {
                        let to = end.unwrap_or(i);
                        let slice: String = chars[from..to].iter().collect();
                        if must_unescape {
                            unescape(&slice)
                        } else {
                            slice
                        }
                    }
                    None => String::new(),
                };
                upsert(&mut params, &name, value);

                if c == ',' {
                    output.push(ForwardedElement { params });
                    params = Vec::new();
                }

                parameter = None;
                start = None;
                end = None;
            } else {
                return Err(ForwardedError { position: Some(i) });
            }
        } else if c == ' ' || c == '\t' {
            if end.is_some() {
                i += 1;
                continue;
            }
            if in_quotes {
                if start.is_none() {
                    start = Some(i);
                }
            } else if start.is_some() {
                end = Some(i);
            } else {
                return Err(ForwardedError { position: Some(i) });
            }
        } else {
            return Err(ForwardedError { position: Some(i) });
        }
        i += 1;
    }

    let last = chars.last().copied();
    if parameter.is_none()
        || in_quotes
        || (start.is_none() && end.is_none())
        || last == Some(' ')
        || last == Some('\t')
    {
        return Err(ForwardedError { position: None });
    }

    let name = parameter.unwrap_or_default();
    let value = match start {
        Some(from) => {
            let to = end.unwrap_or(chars.len());
            let slice: String = chars[from..to].iter().collect();
            if must_unescape {
                unescape(&slice)
            } else {
                slice
            }
        }
        None => String::new(),
    };
    upsert(&mut params, &name, value);
    output.push(ForwardedElement { params });

    Ok(output)
}

/// Assign a parameter, keeping the first position and the last value.
fn upsert(params: &mut Vec<(String, String)>, name: &str, value: String) {
    match params.iter_mut().find(|(key, _)| key == name) {
        Some(entry) => entry.1 = value,
        None => params.push((name.to_string(), value)),
    }
}

/// Drop a backslash when it escapes a following character.
///
/// This is the reference's `str.replace(/\\(.)/g, '$1')`: a backslash at the
/// end with nothing after it is left alone.
fn unescape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some(next) => out.push(next),
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// A whole value is a token when every character is a token character.
fn is_token(value: &str) -> bool {
    !value.is_empty() && value.chars().all(is_token_char)
}

/// `tchar` from RFC 7230 section 3.2.6.
fn is_token_char(c: char) -> bool {
    matches!(c,
        '!' | '#'..='\'' | '*' | '+' | '-' | '.' | '0'..='9' | 'A'..='Z' | '^'..='z' | '|' | '~')
}

/// `separators` from RFC 7230 section 3.2.6.
fn is_delimiter(c: char) -> bool {
    matches!(c, '"' | '(' | ')' | ',' | '/' | ':'..='@' | '['..=']' | '{' | '}')
}

/// `%x20-7E`, the printable ASCII range.
fn is_print(c: char) -> bool {
    (' '..='~').contains(&c)
}

/// `%x80-FF`, the range HTTP allows in a quoted-string but a token does not.
fn is_extended(c: char) -> bool {
    ('\u{80}'..='\u{FF}').contains(&c)
}
