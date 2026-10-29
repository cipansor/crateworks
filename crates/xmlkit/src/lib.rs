#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod namespace;

pub use namespace::{
    Namespaces, XML_NAMESPACE, XMLNS_NAMESPACE, all_namespaced, attribute_namespaced,
    element_namespace, find_namespaced, namespaces_at, split_name,
};

use std::collections::BTreeMap;
use std::fmt::{self, Display, Write as _};

/// An error produced while parsing XML.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    /// Byte offset in the input where the problem was detected.
    pub offset: usize,
    /// Human readable description.
    pub message: String,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "XML error at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for Error {}

type Result<T> = std::result::Result<T, Error>;

/// A parsed XML element.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// Element name, including any namespace prefix as written.
    pub name: String,
    /// Attributes in document order-independent sorted order.
    pub attributes: BTreeMap<String, String>,
    /// Child nodes.
    pub children: Vec<Node>,
}

/// A child of an [`Element`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// A nested element.
    Element(Element),
    /// Character data. Consecutive runs of text are merged.
    Text(String),
    /// A comment, retained so that round-tripping is faithful.
    Comment(String),
}

/// Frees a tree without recursing on the call stack.
///
/// The derived drop descends one stack frame per level, so a deep tree aborts the
/// process. A tree can be built by hand past [`MAX_DEPTH`], and even a parsed one
/// is dropped on whatever stack the caller happens to be on. This walks the tree
/// instead and frees each element as it goes, so the cost is heap, not stack.
impl Drop for Element {
    fn drop(&mut self) {
        // A leaf is the common case and stays as cheap as the derived drop.
        if self.children.is_empty() {
            return;
        }
        let mut pending = std::mem::take(&mut self.children);
        while let Some(node) = pending.pop() {
            if let Node::Element(mut element) = node {
                // Taking the children leaves this element empty, so dropping it at
                // the end of this arm cannot recurse.
                pending.append(&mut element.children);
            }
        }
    }
}

impl Element {
    /// Creates an element with no attributes or children.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            attributes: BTreeMap::new(),
            children: Vec::new(),
        }
    }

    /// The element's tag name, with any prefix it carries.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Looks up an attribute value.
    #[must_use]
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes.get(name).map(String::as_str)
    }

    /// Returns the first direct child element with the given name.
    #[must_use]
    pub fn child(&self, name: &str) -> Option<&Self> {
        self.children.iter().find_map(|n| match n {
            Node::Element(e) if e.name == name => Some(e),
            _ => None,
        })
    }

    /// Iterates over direct child elements, skipping text and comments.
    pub fn children_named(&self, name: &str) -> impl Iterator<Item = &Self> {
        self.children.iter().filter_map(move |n| match n {
            Node::Element(e) if e.name == name => Some(e),
            _ => None,
        })
    }

    /// Concatenates all direct text children.
    #[must_use]
    pub fn text(&self) -> String {
        let mut out = String::new();
        for n in &self.children {
            if let Node::Text(t) = n {
                out.push_str(t);
            }
        }
        out
    }

    /// Collects text from this element and every descendant, in document order.
    #[must_use]
    pub fn deep_text(&self) -> String {
        let mut out = String::new();
        self.collect_text(&mut out);
        out
    }

    fn collect_text(&self, out: &mut String) {
        for n in &self.children {
            match n {
                Node::Text(t) => out.push_str(t),
                Node::Element(e) => e.collect_text(out),
                Node::Comment(_) => {}
            }
        }
    }

    /// Appends a child element.
    pub fn push(&mut self, child: Self) -> &mut Self {
        self.children.push(Node::Element(child));
        self
    }

    /// Appends text.
    pub fn push_text(&mut self, text: impl Into<String>) -> &mut Self {
        self.children.push(Node::Text(text.into()));
        self
    }

    /// Sets an attribute.
    pub fn set_attr(&mut self, name: impl Into<String>, value: impl Into<String>) -> &mut Self {
        self.attributes.insert(name.into(), value.into());
        self
    }
}

impl Display for Element {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{}", self.name)?;
        for (k, v) in &self.attributes {
            write!(f, " {k}=\"{}\"", escape_attr(v))?;
        }
        if self.children.is_empty() {
            return f.write_str("/>");
        }
        f.write_char('>')?;
        for n in &self.children {
            match n {
                Node::Element(e) => write!(f, "{e}")?,
                Node::Text(t) => f.write_str(&escape_text(t))?,
                Node::Comment(c) => write!(f, "<!--{c}-->")?,
            }
        }
        write!(f, "</{}>", self.name)
    }
}

/// Escapes text content for use inside an element.
#[must_use]
pub fn escape_text(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

/// Escapes a value for use inside a double-quoted attribute.
#[must_use]
pub fn escape_attr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// Applies XML 1.0 end-of-line handling to element content.
///
/// Section 2.11: a literal carriage return and a carriage-return/line-feed pair
/// both become a single line feed before the document is parsed, so a value can
/// only ever contain a line feed from the source. A `&#13;` reference is a real
/// carriage return and is left alone, because it is produced after this step.
fn normalize_newlines(s: &str) -> String {
    if !s.contains('\r') {
        return s.to_string();
    }
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\r' {
            if chars.peek() == Some(&'\n') {
                chars.next();
            }
            out.push('\n');
        } else {
            out.push(c);
        }
    }
    out
}

/// Applies XML 1.0 attribute-value normalization.
///
/// Section 3.3.3: after end-of-line handling, every literal whitespace character
/// in an attribute value becomes a space. A character reference is again exempt,
/// so `&#9;` stays a tab and `&#13;` stays a carriage return.
fn normalize_attribute(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '&' => {
                // Copy a reference whole, so its body is not touched.
                out.push('&');
                for c in chars.by_ref() {
                    out.push(c);
                    if c == ';' {
                        break;
                    }
                }
            }
            '\r' => {
                // A CRLF pair is one line break, and so one space.
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                out.push(' ');
            }
            '\n' | '\t' => out.push(' '),
            _ => out.push(c),
        }
    }
    out
}

/// Expands the entities in `s`.
///
/// A character reference must name a legal XML character: one of the five
/// predefined entities, or `&#n;`/`&#xn;` in the XML 1.0 character range (no
/// surrogate, no `0`, no `FFFE`/`FFFF`). A reference outside that range is an
/// error, because the alternative - keeping it as written - is how a document
/// could smuggle a code point past a consumer that expects text.
///
/// An ampersand that begins no entity at all, such as the one in `Tom & Jerry`,
/// is a literal, which is the lenient behaviour the npm `@xmldom/xmldom` parser
/// has and this crate's SOAP and ISO 20022 consumers depend on. A named entity
/// that is not one of the five predefined ones is not an error here either,
/// because this crate does not read the internal subset that would define it.
fn unescape(s: &str, base: usize) -> Result<String> {
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'&' {
            let start = i;
            while i < bytes.len() && bytes[i] != b'&' {
                i += 1;
            }
            out.push_str(&s[start..i]);
            continue;
        }
        let Some(semi) = s[i..].find(';').map(|p| i + p) else {
            out.push('&');
            i += 1;
            continue;
        };
        let entity = &s[i + 1..semi];
        let decoded = match entity {
            "amp" => Some('&'),
            "lt" => Some('<'),
            "gt" => Some('>'),
            "quot" => Some('"'),
            "apos" => Some('\''),
            _ if entity.starts_with("#x") || entity.starts_with("#X") => {
                match u32::from_str_radix(&entity[2..], 16) {
                    Ok(code) => Some(legal_char(code).ok_or_else(|| Error {
                        offset: base + i,
                        message: format!("&#{entity}; is not a legal XML character"),
                    })?),
                    Err(_) => {
                        return Err(Error {
                            offset: base + i,
                            message: format!("&#{entity}; is not a valid character reference"),
                        });
                    }
                }
            }
            _ if entity.starts_with('#') => match entity[1..].parse::<u32>() {
                Ok(code) => Some(legal_char(code).ok_or_else(|| Error {
                    offset: base + i,
                    message: format!("&{entity}; is not a legal XML character"),
                })?),
                Err(_) => {
                    return Err(Error {
                        offset: base + i,
                        message: format!("&{entity}; is not a valid character reference"),
                    });
                }
            },
            _ => None,
        };
        if let Some(c) = decoded {
            out.push(c);
            i = semi + 1;
        } else {
            out.push('&');
            i += 1;
        }
    }
    Ok(out)
}

/// The character a code point names, or `None` if XML forbids it.
///
/// The XML 1.0 `Char` production allows tab, line feed, carriage return, the
/// range `20`-`D7FF`, `E000`-`FFFD` and `10000`-`10FFFF`. That excludes the
/// other C0 controls, the surrogates (which are not characters), `FFFE`/`FFFF`
/// and anything above `10FFFF`.
fn legal_char(code: u32) -> Option<char> {
    let allowed = matches!(code, 0x9 | 0xA | 0xD)
        || (0x20..=0xD7FF).contains(&code)
        || (0xE000..=0xFFFD).contains(&code)
        || (0x1_0000..=0x10_FFFF).contains(&code);
    if allowed { char::from_u32(code) } else { None }
}

/// Whether `c` may begin an element or attribute name (XML 1.0 `NameStartChar`).
///
/// The colon is allowed, so a qualified name such as `ns:Doc` is one name; this
/// crate matches on the local name rather than validating the prefix.
fn is_name_start_char(c: char) -> bool {
    matches!(c, ':' | '_' | 'A'..='Z' | 'a'..='z')
        || matches!(c,
            '\u{C0}'..='\u{D6}'
            | '\u{D8}'..='\u{F6}'
            | '\u{F8}'..='\u{2FF}'
            | '\u{370}'..='\u{37D}'
            | '\u{37F}'..='\u{1FFF}'
            | '\u{200C}'..='\u{200D}'
            | '\u{2070}'..='\u{218F}'
            | '\u{2C00}'..='\u{2FEF}'
            | '\u{3001}'..='\u{D7FF}'
            | '\u{F900}'..='\u{FDCF}'
            | '\u{FDF0}'..='\u{FFFD}'
            | '\u{1_0000}'..='\u{E_FFFF}')
}

/// Whether `c` may continue an element or attribute name (XML 1.0 `NameChar`).
fn is_name_char(c: char) -> bool {
    is_name_start_char(c)
        || matches!(c, '-' | '.' | '0'..='9' | '\u{B7}')
        || matches!(c, '\u{300}'..='\u{36F}' | '\u{203F}'..='\u{2040}')
}

/// One open element: its name, its attributes, and the children read so far.
///
/// The parser's explicit stack is a list of these, which is what keeps it off the
/// call stack.
type Frame = (String, BTreeMap<String, String>, Vec<Node>);

struct Parser<'a> {
    src: &'a str,
    pos: usize,
}

/// The deepest element nesting [`parse`] will accept.
///
/// This is a policy limit, not a safety one: the parser and the drop both keep
/// their own stack, so a document past this is refused because nothing legitimate
/// nests that deep, not because accepting it would overflow. Real SOAP, WSDL and
/// ISO 20022 documents nest a few dozen levels at most. Raising it is safe; the
/// cost is heap, and `Element`'s own traversal is iterative too.
pub const MAX_DEPTH: usize = 128;

impl<'a> Parser<'a> {
    fn err<T>(&self, message: impl Into<String>) -> Result<T> {
        Err(Error {
            offset: self.pos,
            message: message.into(),
        })
    }

    fn peek(&self) -> Option<char> {
        self.src[self.pos..].chars().next()
    }

    fn starts_with(&self, s: &str) -> bool {
        self.src[self.pos..].starts_with(s)
    }

    fn skip_ws(&mut self) {
        while matches!(self.peek(), Some(c) if c.is_whitespace()) {
            self.pos += 1;
        }
    }

    fn expect(&mut self, s: &str) -> Result<()> {
        if self.starts_with(s) {
            self.pos += s.len();
            Ok(())
        } else {
            self.err(format!("expected {s:?}"))
        }
    }

    fn parse_name(&mut self) -> Result<String> {
        let start = self.pos;
        match self.peek() {
            Some(c) if is_name_start_char(c) => self.pos += c.len_utf8(),
            _ => return self.err("expected element or attribute name"),
        }
        while let Some(c) = self.peek() {
            if is_name_char(c) {
                self.pos += c.len_utf8();
            } else {
                break;
            }
        }
        Ok(self.src[start..self.pos].to_string())
    }

    fn skip_comment(&mut self) -> Result<()> {
        self.expect("<!--")?;
        let body_start = self.pos;
        let Some(end) = self.src[self.pos..].find("-->") else {
            return self.err("unterminated comment");
        };
        // XML 1.0 section 2.5: the content of a comment cannot contain `--` and
        // cannot end with `-`, because either would make the terminator
        // ambiguous. `<!-- a -- b -->` and `<!--a--->` are both ill-formed.
        let body = &self.src[body_start..body_start + end];
        if body.contains("--") || body.ends_with('-') {
            return self.err("comment content must not contain '--' or end with '-'");
        }
        self.pos += end + 3;
        Ok(())
    }

    fn skip_pi(&mut self) -> Result<()> {
        // The target `xml` is reserved for the XML declaration, so a processing
        // instruction cannot use it; expat rejects `<?xml ... ?>` anywhere but
        // the start of the document, which is handled before this is reached.
        if self.is_xml_declaration() {
            return self.err("the target 'xml' is reserved for the XML declaration");
        }
        self.skip_pi_body()
    }

    /// Advances past `<? ... ?>` without the reserved-target check.
    ///
    /// Used for the XML declaration itself, which is the one place `<?xml` is
    /// allowed. XML 1.0 `PI ::= '<?' PITarget (S (Char* - (Char* '?>' Char*)))? '?>'`,
    /// so the target must be a name and anything after it must be separated by
    /// whitespace; `<?pi<>?>` is ill-formed.
    fn skip_pi_body(&mut self) -> Result<()> {
        self.expect("<?")?;
        if self.parse_name().is_err() {
            return self.err("expected a processing instruction target");
        }
        match self.peek() {
            Some('?') if self.starts_with("?>") => {}
            Some(c) if c.is_whitespace() => {}
            _ => return self.err("expected whitespace after the processing instruction target"),
        }
        let Some(end) = self.src[self.pos..].find("?>") else {
            return self.err("unterminated processing instruction");
        };
        self.pos += end + 2;
        Ok(())
    }

    /// Whether the cursor is on `<?xml` followed by whitespace or `?`.
    ///
    /// `<?xml-stylesheet` is an ordinary processing instruction; only the exact
    /// target `xml` is reserved.
    fn is_xml_declaration(&self) -> bool {
        self.starts_with("<?xml")
            && matches!(self.src[self.pos + 5..].chars().next(), Some(c) if c.is_whitespace() || c == '?')
    }

    /// Parses the XML declaration, which is only valid as the first thing in a
    /// document. The grammar is XML 1.0 `XMLDecl`, and every part of it is
    /// checked, because a declaration that does not match is not well-formed.
    fn skip_xml_declaration(&mut self) -> Result<()> {
        self.expect("<?xml")?;
        if !self.skip_ws_if_any() {
            return self.err("expected whitespace in the XML declaration");
        }
        self.expect_word("version")?;
        self.expect_eq()?;
        let version = self.parse_quoted()?;
        // XML 1.0 says `VersionNum ::= '1.' [0-9]+`, but expat accepts a wider
        // token and matching the reference parser is what this crate's
        // consumers need. What expat does reject is whitespace and the markup
        // characters that cannot appear in a version token.
        if version
            .chars()
            .any(|c| c.is_whitespace() || matches!(c, '<' | '>' | '&' | '"' | '\'' | ';' | '='))
        {
            return self.err("the XML version contains an illegal character");
        }
        let save = self.pos;
        self.skip_ws();
        if self.peek_word("encoding") {
            self.expect_word("encoding")?;
            self.expect_eq()?;
            let encoding = self.parse_quoted()?;
            let valid = encoding
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_alphabetic())
                && encoding
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
            if !valid {
                return self.err("invalid encoding name in the XML declaration");
            }
        } else {
            self.pos = save;
        }
        let save = self.pos;
        self.skip_ws();
        if self.peek_word("standalone") {
            self.expect_word("standalone")?;
            self.expect_eq()?;
            let standalone = self.parse_quoted()?;
            if standalone != "yes" && standalone != "no" {
                return self.err("standalone must be 'yes' or 'no'");
            }
        } else {
            self.pos = save;
        }
        self.skip_ws();
        if !self.starts_with("?>") {
            return self.err("expected '?>' to end the XML declaration");
        }
        self.pos += 2;
        Ok(())
    }

    /// Skips whitespace and reports whether any was skipped.
    fn skip_ws_if_any(&mut self) -> bool {
        let before = self.pos;
        self.skip_ws();
        self.pos != before
    }

    /// Consumes `word` exactly, or errors.
    fn expect_word(&mut self, word: &str) -> Result<()> {
        if self.starts_with(word) {
            self.pos += word.len();
            Ok(())
        } else {
            self.err(format!("expected '{word}' in the XML declaration"))
        }
    }

    /// Whether the cursor is on `word` followed by a non-name character.
    fn peek_word(&self, word: &str) -> bool {
        self.starts_with(word)
            && !matches!(self.src[self.pos + word.len()..].chars().next(), Some(c) if is_name_char(c))
    }

    /// Consumes optional whitespace, then `=`, then optional whitespace.
    fn expect_eq(&mut self) -> Result<()> {
        self.skip_ws();
        self.expect("=")?;
        self.skip_ws();
        Ok(())
    }

    /// Reads a quoted value and returns its content.
    fn parse_quoted(&mut self) -> Result<String> {
        let quote = self
            .peek()
            .filter(|c| *c == '"' || *c == '\'')
            .ok_or_else(|| Error {
                offset: self.pos,
                message: "expected a quoted value".into(),
            })?;
        self.pos += quote.len_utf8();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if c == quote {
                let value = self.src[start..self.pos].to_string();
                self.pos += quote.len_utf8();
                return Ok(value);
            }
            self.pos += c.len_utf8();
        }
        self.err("unterminated quoted value")
    }

    fn skip_doctype(&mut self) -> Result<()> {
        // Skips a DOCTYPE declaration. XML 1.0 section 2.8:
        //   doctypedecl ::= '<!DOCTYPE' S Name (S ExternalID)? S? ('[' intSubset ']' S?)? '>'
        // The internal subset is skipped, not read, so an entity it defines is
        // never resolved; that is what keeps this crate free of external entity
        // (XXE) handling.
        self.expect("<!DOCTYPE")?;
        if !matches!(self.peek(), Some(c) if c.is_whitespace()) {
            return self.err("expected whitespace after DOCTYPE");
        }
        self.skip_ws();
        self.parse_name()?;
        let had_ws = self.skip_ws_if_any();
        if self.peek_word("SYSTEM") || self.peek_word("PUBLIC") {
            if !had_ws {
                return self.err("expected whitespace before the external identifier");
            }
            self.skip_external_id()?;
            self.skip_ws();
        }
        if self.starts_with("[") {
            self.pos += 1;
            let Some(end) = self.src[self.pos..].find(']') else {
                return self.err("unterminated DOCTYPE internal subset");
            };
            self.pos += end + 1;
            self.skip_ws();
        }
        if !self.starts_with(">") {
            return self.err("expected '>' to end the DOCTYPE declaration");
        }
        self.pos += 1;
        Ok(())
    }

    /// Skips `SYSTEM SystemLiteral` or `PUBLIC PubidLiteral SystemLiteral`.
    fn skip_external_id(&mut self) -> Result<()> {
        if self.peek_word("SYSTEM") {
            self.expect_word("SYSTEM")?;
            self.skip_ws();
            self.parse_quoted()?;
            return Ok(());
        }
        self.expect_word("PUBLIC")?;
        self.skip_ws();
        self.parse_quoted()?;
        self.skip_ws();
        self.parse_quoted()?;
        Ok(())
    }

    fn parse_attributes(&mut self) -> Result<BTreeMap<String, String>> {
        let mut attrs = BTreeMap::new();
        loop {
            self.skip_ws();
            match self.peek() {
                Some('>') | Some('/') | None => return Ok(attrs),
                _ => {}
            }
            let name = self.parse_name()?;
            self.skip_ws();
            self.expect("=")?;
            self.skip_ws();
            let quote = self
                .peek()
                .filter(|c| *c == '"' || *c == '\'')
                .ok_or_else(|| Error {
                    offset: self.pos,
                    message: "expected quoted attribute value".into(),
                })?;
            self.pos += quote.len_utf8();
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c == quote {
                    break;
                }
                // XML 1.0 section 3.1: `<` is not allowed in an attribute value,
                // because a parser could not tell it from the start of a tag.
                if c == '<' {
                    return self.err("'<' is not allowed in an attribute value");
                }
                self.pos += c.len_utf8();
            }
            if self.peek().is_none() {
                return self.err("unterminated attribute value");
            }
            let raw = &self.src[start..self.pos];
            self.pos += quote.len_utf8();
            attrs.insert(name, unescape(&normalize_attribute(raw), start)?);
            // XML requires whitespace between the element name, each attribute
            // and the end of the tag, so `<a x="1"y="2">` is ill-formed.
            if !matches!(self.peek(), Some(c) if c.is_whitespace()) {
                match self.peek() {
                    Some('>') | Some('/') | None => return Ok(attrs),
                    _ => return self.err("expected whitespace before attribute"),
                }
            }
        }
    }

    /// Parses an element and its whole subtree.
    ///
    /// Iterative, not recursive. A recursive descent uses one stack frame per
    /// level, and measured on this crate `MAX_DEPTH` levels of it overflow a
    /// 256 KiB stack, which is a spawned thread's default on some platforms. The
    /// cap cannot be lowered enough to fix that without refusing documents that
    /// are fine, so the parser carries its own stack of open elements and the cap
    /// is a policy limit rather than the thing standing between a document and an
    /// aborted process.
    fn parse_element(&mut self) -> Result<Element> {
        let mut stack: Vec<Frame> = Vec::new();

        loop {
            // At the start, and only then, there is no open element and the next
            // thing must be the root's opening tag.
            if stack.is_empty() {
                let (name, attributes) = self.parse_open_tag()?;
                if self.starts_with("/>") {
                    self.pos += 2;
                    return Ok(Element {
                        name,
                        attributes,
                        children: Vec::new(),
                    });
                }
                self.expect(">")?;
                stack.push((name, attributes, Vec::new()));
                continue;
            }

            if self.starts_with("</") {
                self.pos += 2;
                let close = self.parse_name()?;
                self.skip_ws();
                self.expect(">")?;
                let (name, attributes, children) = stack.pop().expect("an open frame");
                if close != name {
                    return Err(Error {
                        offset: self.pos,
                        message: format!("closing tag </{close}> does not match <{name}>"),
                    });
                }
                let element = Element {
                    name,
                    attributes,
                    children,
                };
                // With no parent frame, this element is the document's root.
                if let Some(root) = attach(element, &mut stack) {
                    return Ok(root);
                }
                continue;
            }
            if self.starts_with("<!--") {
                let start = self.pos;
                self.skip_comment()?;
                let body = self.src[start + 4..self.pos - 3].to_string();
                self.children_mut(&mut stack).push(Node::Comment(body));
                continue;
            }
            if self.starts_with("<![CDATA[") {
                self.pos += 9;
                let Some(end) = self.src[self.pos..].find("]]>") else {
                    return self.err("unterminated CDATA section");
                };
                let body = self.src[self.pos..self.pos + end].to_string();
                self.pos += end + 3;
                let children = self.children_mut(&mut stack);
                push_text(children, body);
                continue;
            }
            if self.starts_with("<?") {
                self.skip_pi()?;
                continue;
            }
            if self.starts_with("<!") {
                return self.err("unexpected declaration inside element");
            }
            if self.starts_with("<") {
                let (name, attributes) = self.parse_open_tag()?;
                if self.starts_with("/>") {
                    self.pos += 2;
                    let child = Element {
                        name,
                        attributes,
                        children: Vec::new(),
                    };
                    self.children_mut(&mut stack).push(Node::Element(child));
                    continue;
                }
                if stack.len() >= MAX_DEPTH {
                    return self.err(format!("element nesting exceeds {MAX_DEPTH} levels"));
                }
                self.expect(">")?;
                stack.push((name, attributes, Vec::new()));
                continue;
            }
            if self.pos >= self.src.len() {
                let (name, _, _) = stack.last().expect("an open frame");
                return self.err(format!("unclosed element <{name}>"));
            }
            let start = self.pos;
            while let Some(c) = self.peek() {
                if c == '<' {
                    break;
                }
                self.pos += c.len_utf8();
            }
            let raw = &self.src[start..self.pos];
            // XML 1.0 section 2.4: `]]>` in content must be escaped, because a
            // reader could otherwise take it for the end of a CDATA section.
            if raw.contains("]]>") {
                return self.err("']]>' must be escaped in content");
            }
            let text = unescape(&normalize_newlines(raw), start)?;
            let children = self.children_mut(&mut stack);
            push_text(children, text);
        }
    }

    /// The children of the innermost open element, which is where content goes.
    fn children_mut<'s>(&self, stack: &'s mut [Frame]) -> &'s mut Vec<Node> {
        &mut stack
            .last_mut()
            .expect("the innermost frame is always open")
            .2
    }

    /// Reads `<name attr="value"...` up to, but not including, `>` or `/>`.
    fn parse_open_tag(&mut self) -> Result<(String, BTreeMap<String, String>)> {
        self.expect("<")?;
        let name = self.parse_name()?;
        let attributes = self.parse_attributes()?;
        self.skip_ws();
        Ok((name, attributes))
    }
}

/// Attaches a finished element to the innermost open frame.
///
/// Returns `Some(root)` when there was no frame to attach it to, which means the
/// element just finished is the document's root and the caller should return it.
fn attach(element: Element, stack: &mut [Frame]) -> Option<Element> {
    match stack.last_mut() {
        Some((_, _, children)) => {
            children.push(Node::Element(element));
            None
        }
        None => Some(element),
    }
}

fn push_text(children: &mut Vec<Node>, text: String) {
    if let Some(Node::Text(existing)) = children.last_mut() {
        existing.push_str(&text);
    } else {
        children.push(Node::Text(text));
    }
}

/// Parses a complete XML document and returns its root element.
///
/// Prolog declarations, DOCTYPE declarations and top-level comments are
/// skipped. `text` must contain exactly one root element.
///
/// # Errors
///
/// Returns [`Error`] when the input is not well-formed XML.
///
/// # Example
///
/// ```
/// use xmlkit::parse;
///
/// let doc = parse("<a x=\"1\"><b>hi</b></a>").unwrap();
/// assert_eq!(doc.attr("x"), Some("1"));
/// assert_eq!(doc.child("b").unwrap().text(), "hi");
///
/// // A second root element is not well-formed.
/// assert!(parse("<a/><b/>").is_err());
/// ```
pub fn parse(text: &str) -> Result<Element> {
    // XML 1.0 section 4.3.3: a byte-order mark may open a document, and it is not
    // part of the content. A file read from disk or the network often carries
    // one, and every other parser skips it, so refusing it would reject ordinary
    // input.
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let mut p = Parser { src: text, pos: 0 };
    let mut seen_doctype = false;
    loop {
        p.skip_ws();
        if p.starts_with("<?") {
            // `<?xml ... ?>` is the XML declaration, which the grammar allows
            // only at the very start of the document; the target `xml` is
            // reserved, so anywhere else it is not a processing instruction.
            if p.is_xml_declaration() {
                if p.pos != 0 {
                    return p.err("XML declaration is only allowed at the start");
                }
                p.skip_xml_declaration()?;
            } else {
                p.skip_pi()?;
            }
        } else if p.starts_with("<!--") {
            p.skip_comment()?;
        } else if p.starts_with("<!DOCTYPE") {
            if seen_doctype {
                return p.err("only one DOCTYPE declaration is allowed");
            }
            seen_doctype = true;
            p.skip_doctype()?;
        } else {
            break;
        }
    }
    if p.peek() != Some('<') {
        return p.err("expected root element");
    }
    let root = p.parse_element()?;
    // XML 1.0 `document ::= prolog element Misc*`, so whitespace, comments and
    // processing instructions may follow the root element and the document is
    // still well-formed. expat and `@xmldom/xmldom` both accept one; rejecting
    // it here would refuse a document a caller's other tools take.
    loop {
        p.skip_ws();
        if p.starts_with("<!--") {
            p.skip_comment()?;
        } else if p.starts_with("<?") {
            if p.is_xml_declaration() {
                return p.err("XML declaration is only allowed at the start");
            }
            p.skip_pi()?;
        } else {
            break;
        }
    }
    if p.pos != p.src.len() {
        return p.err("trailing content after root element");
    }
    Ok(root)
}

/// Extracts the local name of a possibly prefixed name.
#[must_use]
pub fn local_name(name: &str) -> &str {
    name.rsplit_once(':').map_or(name, |(_, local)| local)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_carriage_return_in_text_becomes_a_line_feed() {
        // XML 1.0 section 2.11: the parser normalises a literal CR and CRLF
        // before anything else sees the text.
        assert_eq!(parse("<a>x\ry</a>").unwrap().text(), "x\ny");
        assert_eq!(parse("<a>x\r\ny</a>").unwrap().text(), "x\ny");
        // A character reference is a real carriage return and survives.
        assert_eq!(parse("<a>x&#13;y</a>").unwrap().text(), "x\ry");
    }

    #[test]
    fn whitespace_in_an_attribute_becomes_a_space() {
        // XML 1.0 section 3.3.3: tab, newline and carriage return in an
        // attribute value normalise to a space; a character reference does not.
        assert_eq!(parse("<a x=\"p\tq\"/>").unwrap().attr("x"), Some("p q"));
        assert_eq!(parse("<a x=\"p\nq\"/>").unwrap().attr("x"), Some("p q"));
        assert_eq!(parse("<a x=\"p\r\nq\"/>").unwrap().attr("x"), Some("p q"));
        assert_eq!(parse("<a x=\"p&#9;q\"/>").unwrap().attr("x"), Some("p\tq"));
    }

    #[test]
    fn parses_attributes_and_children() {
        let root = parse(r#"<a x="1" y='2'><b>hi</b></a>"#).unwrap();
        assert_eq!(root.name, "a");
        assert_eq!(root.attr("x"), Some("1"));
        assert_eq!(root.attr("y"), Some("2"));
        assert_eq!(root.child("b").unwrap().text(), "hi");
    }

    #[test]
    fn self_closing_and_empty_text() {
        let root = parse("<a/>").unwrap();
        assert!(root.children.is_empty());
        assert_eq!(root.to_string(), "<a/>");
    }

    #[test]
    fn decodes_predefined_entities() {
        let root = parse("<a>&lt;&amp;&gt;&quot;&apos;</a>").unwrap();
        assert_eq!(root.text(), "<&>\"'");
    }

    #[test]
    fn decodes_numeric_character_references() {
        let root = parse("<a>&#65;&#x42;</a>").unwrap();
        assert_eq!(root.text(), "AB");
    }

    #[test]
    fn cdata_is_literal() {
        let root = parse("<a><![CDATA[<not-a-tag> & raw]]></a>").unwrap();
        assert_eq!(root.text(), "<not-a-tag> & raw");
    }

    #[test]
    fn nested_and_repeated_children() {
        let root = parse("<r><i>1</i><i>2</i><i>3</i></r>").unwrap();
        let all: Vec<String> = root.children_named("i").map(Element::text).collect();
        assert_eq!(all, ["1", "2", "3"]);
    }

    #[test]
    fn deep_text_walks_descendants() {
        let root = parse("<a>x<b>y<c>z</c></b></a>").unwrap();
        assert_eq!(root.deep_text(), "xyz");
    }

    #[test]
    fn a_comment_or_pi_may_follow_the_root_element() {
        // XML 1.0 `document ::= prolog element Misc*`, so a trailing comment or
        // processing instruction is well-formed; expat and @xmldom/xmldom both
        // accept one. A second *element* still is not.
        assert!(parse("<a/><!-- done -->").is_ok());
        assert!(parse("<a/><?done?>").is_ok());
        assert!(parse("<a/>\n<!-- one -->\n<?two?>\n").is_ok());
        assert_eq!(parse("<a>hi</a><!-- x -->").unwrap().text(), "hi");
        assert!(parse("<a/><b/>").is_err());
        assert!(parse("<a/><!-- unterminated").is_err());
    }

    #[test]
    fn a_bare_ampersand_is_kept_as_text() {
        // The lenient behaviour the npm @xmldom/xmldom parser has and this
        // crate's consumers rely on: an ampersand that begins no entity is a
        // literal. A strict parser rejects the same input.
        assert_eq!(parse("<a>Tom & Jerry</a>").unwrap().text(), "Tom & Jerry");
        assert_eq!(parse("<a>&#38;</a>").unwrap().text(), "&");
        assert_eq!(parse("<a>&amp;</a>").unwrap().text(), "&");
        assert_eq!(parse("<a>x &</a>").unwrap().text(), "x &");
    }

    #[test]
    fn nesting_beyond_the_cap_is_rejected_not_crashed() {
        // Without the cap this input overflows the stack and aborts the process,
        // which is a denial of service for anything parsing a document off the
        // network. It must come back as an ordinary error instead.
        let deep = "<a>".repeat(MAX_DEPTH + 50) + &"</a>".repeat(MAX_DEPTH + 50);
        let err = parse(&deep).unwrap_err();
        assert!(err.message.contains("nesting exceeds"), "{}", err.message);
    }

    #[test]
    fn a_document_at_the_cap_parses_and_drops_on_a_small_stack() {
        // Guards the parser's own stack use. Measured before it was made
        // iterative: parsing MAX_DEPTH levels overflowed a 256 KiB stack, which is
        // a spawned thread's default on some platforms. A stack overflow aborts
        // the process rather than failing, so a regression here shows up as an
        // abort and not as a failed assertion; that is the only signal available.
        // The drop is covered by the deep hand-built tree test below.
        std::thread::Builder::new()
            .stack_size(256 * 1024)
            .spawn(|| {
                let src = "<a>".repeat(MAX_DEPTH) + &"</a>".repeat(MAX_DEPTH);
                let doc = parse(&src).unwrap();
                assert_eq!(doc.deep_text(), "");
                drop(doc);
            })
            .unwrap()
            .join()
            .expect("parsing and dropping a deep document must not overflow the stack");
    }

    #[test]
    fn dropping_a_deep_hand_built_tree_does_not_recurse() {
        // Past the parse cap, so the parser's guard does not help here: only the
        // iterative drop keeps this off the call stack.
        let mut root = Element::new("root");
        let mut cursor = &mut root;
        for _ in 0..50_000 {
            cursor.push(Element::new("x"));
            cursor = match cursor.children.last_mut() {
                Some(Node::Element(child)) => child,
                _ => unreachable!(),
            };
        }
        drop(root);
    }

    #[test]
    fn nesting_at_the_cap_still_parses() {
        let ok = "<a>".repeat(MAX_DEPTH) + &"</a>".repeat(MAX_DEPTH);
        assert!(parse(&ok).is_ok());
    }

    #[test]
    fn skips_prolog_comments_and_doctype() {
        let src = "<?xml version=\"1.0\"?><!-- hi --><!DOCTYPE a [<!ENTITY e \"x\">]><a>ok</a>";
        assert_eq!(parse(src).unwrap().text(), "ok");
    }

    #[test]
    fn text_runs_are_merged() {
        let root = parse("<a>one<!--c-->two</a>").unwrap();
        let texts: Vec<&String> = root
            .children
            .iter()
            .filter_map(|n| if let Node::Text(t) = n { Some(t) } else { None })
            .collect();
        assert_eq!(texts, ["one", "two"]);
        assert_eq!(root.text(), "onetwo");
    }

    #[test]
    fn namespace_prefix_is_preserved_and_local_name_available() {
        let root = parse(r#"<ns:Doc xmlns:ns="urn:x"><ns:Body/></ns:Doc>"#).unwrap();
        assert_eq!(root.name, "ns:Doc");
        assert_eq!(local_name(&root.name), "Doc");
        assert_eq!(root.attr("xmlns:ns"), Some("urn:x"));
    }

    #[test]
    fn round_trip_preserves_structure() {
        let src = r#"<a x="1"><b>hi &amp; bye</b></a>"#;
        let once = parse(src).unwrap().to_string();
        let twice = parse(&once).unwrap().to_string();
        assert_eq!(once, twice);
    }

    #[test]
    fn escapes_on_output() {
        let mut e = Element::new("a");
        e.set_attr("k", "a\"b");
        e.push_text("x<y&z");
        assert_eq!(e.to_string(), r#"<a k="a&quot;b">x&lt;y&amp;z</a>"#);
    }

    #[test]
    fn mismatched_close_tag_is_an_error() {
        let err = parse("<a></b>").unwrap_err();
        assert!(err.message.contains("does not match"), "{}", err.message);
    }

    #[test]
    fn unterminated_element_is_an_error() {
        assert!(parse("<a>").is_err());
        assert!(parse("<a").is_err());
    }

    #[test]
    fn trailing_content_is_an_error() {
        assert!(parse("<a/><b/>").is_err());
    }

    #[test]
    fn unknown_entity_is_left_alone() {
        let root = parse("<a>&weird;</a>").unwrap();
        assert_eq!(root.text(), "&weird;");
    }
}
