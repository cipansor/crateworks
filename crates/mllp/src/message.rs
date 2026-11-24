//! HL7 v2 message parsing, for the payload inside a frame.
//!
//! The framing layer in [`crate::frame`] treats the payload as opaque bytes.
//! This module opens it up: a message is a list of segments, a segment is a
//! list of fields, and a field is a list of repetitions of components of
//! subcomponents, all separated by the delimiters the message declares in its
//! own header.
//!
//! The parser is deliberately a *reader*, not a validator. It does not know
//! which fields are required for which message type, and it does not check
//! against a schema: it splits on the delimiters and hands the pieces back.
//! Real HL7 is full of site-specific segments and optional fields, and a parser
//! that rejects them is worse than one that returns them intact.
//!
//! ```
//! use mllp::Message;
//!
//! let raw = "MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|1|P|2.5.1\r\
//!            PID|1||12345^^^MRN||DOE^JOHN\r";
//! let message = Message::parse(raw).unwrap();
//!
//! assert_eq!(message.segment_names().collect::<Vec<_>>(), ["MSH", "PID"]);
//! assert_eq!(message.segment("PID").unwrap().field(5).unwrap().text(), "DOE^JOHN");
//! assert_eq!(
//!     message.segment("PID").unwrap().field(5).unwrap().component(1),
//!     Some("DOE")
//! );
//! ```

use std::fmt;

/// The five delimiter characters a message declares in its header.
///
/// MSH-1 and MSH-2 carry them, so they are read from the message rather than
/// assumed. A message that uses `#` as its field separator is legal and parses
/// with the same code.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Separators {
    /// Separates fields within a segment. MSH-1.
    pub field: char,
    /// Separates components within a field. First character of MSH-2.
    pub component: char,
    /// Separates repetitions of a field. Second character of MSH-2.
    pub repetition: char,
    /// Introduces an escape sequence. Third character of MSH-2.
    pub escape: char,
    /// Separates subcomponents within a component. Fourth character of MSH-2.
    pub subcomponent: char,
}

impl Default for Separators {
    /// The delimiters every HL7 v2 example uses: `|`, `^`, `~`, `\`, `&`.
    fn default() -> Self {
        Self {
            field: '|',
            component: '^',
            repetition: '~',
            escape: '\\',
            subcomponent: '&',
        }
    }
}

/// Why a byte sequence is not a parseable HL7 v2 message.
///
/// These are separate from [`crate::Error`], which is about framing: a message
/// can be a perfect frame and still not be HL7.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    /// The input holds no segment at all.
    Empty,
    /// The first segment is not `MSH`. HL7 v2 puts the delimiters there, so
    /// nothing after it can be split correctly without it.
    MissingHeader,
    /// The header is too short to hold MSH-1 and MSH-2, so the delimiters are
    /// not there to be read.
    TruncatedHeader,
    /// A segment has no name before its first field separator.
    UnnamedSegment {
        /// Zero-based position of the offending segment.
        index: usize,
    },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => f.write_str("the message contains no segments"),
            Self::MissingHeader => f.write_str("the first segment is not MSH"),
            Self::TruncatedHeader => {
                f.write_str("the MSH segment is too short to carry its delimiters")
            }
            Self::UnnamedSegment { index } => {
                write!(f, "segment {index} has no name")
            }
        }
    }
}

impl std::error::Error for ParseError {}

/// A parsed HL7 v2 message.
///
/// Borrows from the input, so parsing copies nothing: every field, component
/// and subcomponent is a slice of the original text.
#[derive(Debug, Clone)]
pub struct Message<'a> {
    raw: &'a str,
    segments: Vec<Segment<'a>>,
    separators: Separators,
}

impl<'a> Message<'a> {
    /// Splits `raw` into segments and fields.
    ///
    /// Segments are separated by a carriage return, a line feed, or both, which
    /// covers every sender in practice. The delimiters are taken from the MSH
    /// segment, so the input must begin with one.
    ///
    /// # Errors
    ///
    /// [`ParseError::Empty`] for input with no segment,
    /// [`ParseError::MissingHeader`] when the first segment is not MSH,
    /// [`ParseError::TruncatedHeader`] when MSH cannot carry its delimiters,
    /// and [`ParseError::UnnamedSegment`] for a segment with no name.
    pub fn parse(raw: &'a str) -> Result<Self, ParseError> {
        let mut lines = raw.split(['\r', '\n']).filter(|line| !line.is_empty());

        let header = lines.next().ok_or(ParseError::Empty)?;
        if !header.starts_with("MSH") {
            return Err(ParseError::MissingHeader);
        }
        let separators = read_separators(header)?;

        let mut segments = Vec::new();
        segments.push(Segment::parse(header, separators, true, 0)?);
        for (index, line) in lines.enumerate() {
            segments.push(Segment::parse(line, separators, false, index + 1)?);
        }

        Ok(Self {
            raw,
            segments,
            separators,
        })
    }

    /// The message as it was given, byte for byte.
    #[must_use]
    pub fn text(&self) -> &'a str {
        self.raw
    }

    /// The delimiters this message declared in its header.
    #[must_use]
    pub const fn separators(&self) -> Separators {
        self.separators
    }

    /// Every segment, in order.
    #[must_use]
    pub fn segments(&self) -> &[Segment<'a>] {
        &self.segments
    }

    /// The names of every segment, in order.
    pub fn segment_names(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.segments.iter().map(Segment::name)
    }

    /// The first segment with the given name, which is what a caller wants when
    /// a message has at most one of them, such as MSH or PID.
    #[must_use]
    pub fn segment(&self, name: &str) -> Option<&Segment<'a>> {
        self.segments.iter().find(|s| s.name() == name)
    }

    /// Every segment with the given name, in order. Use this for the segments
    /// that repeat, such as OBX or NK1.
    pub fn segments_named<'n>(
        &'n self,
        name: &'n str,
    ) -> impl Iterator<Item = &'n Segment<'a>> + 'n {
        self.segments.iter().filter(move |s| s.name() == name)
    }

    /// The value of a field, addressed as `SEG-n`, for example `PID-5`.
    ///
    /// Returns `None` when the segment is absent or has too few fields.
    #[must_use]
    pub fn field(&self, reference: &str) -> Option<&Field<'a>> {
        let (name, index) = reference.rsplit_once('-')?;
        let index: usize = index.parse().ok()?;
        self.segment(name)?.field(index)
    }

    /// The value of `SEG-n` as plain text, with the first repetition taken and
    /// the delimiter escapes resolved. This is the form a caller wants for a
    /// patient name or a message control id.
    #[must_use]
    pub fn value(&self, reference: &str) -> Option<String> {
        Some(self.field(reference)?.value().unescape(self.separators))
    }

    /// Serialises the message back to ER7, the wire form.
    ///
    /// Round-trips the fields the parser split, so `parse` followed by
    /// `to_er7` returns the input modulo the line endings it normalises.
    #[must_use]
    pub fn to_er7(&self) -> String {
        let mut out = String::with_capacity(self.raw.len());
        for (index, segment) in self.segments.iter().enumerate() {
            if index > 0 {
                out.push('\r');
            }
            out.push_str(&segment.to_er7());
        }
        out
    }
}

/// Reads the delimiters out of an MSH segment.
///
/// MSH-1 is the field separator and sits at a fixed offset; MSH-2 follows it
/// and carries the other four. A message cannot be split before this is known,
/// which is why the header is special-cased everywhere in this module.
fn read_separators(header: &str) -> Result<Separators, ParseError> {
    let mut chars = header.chars();
    let (Some('M'), Some('S'), Some('H'), Some(field)) =
        (chars.next(), chars.next(), chars.next(), chars.next())
    else {
        return Err(ParseError::TruncatedHeader);
    };
    // MSH-2 follows the field separator directly: `MSH|^~\&|...` gives `^~\&`.
    let encoding: Vec<char> = chars.take(4).collect();
    if encoding.len() < 4 {
        return Err(ParseError::TruncatedHeader);
    }
    Ok(Separators {
        field,
        component: encoding[0],
        repetition: encoding[1],
        escape: encoding[2],
        subcomponent: encoding[3],
    })
}

/// One segment: a name and its fields.
#[derive(Debug, Clone)]
pub struct Segment<'a> {
    name: &'a str,
    fields: Vec<Field<'a>>,
    separators: Separators,
    is_header: bool,
}

impl<'a> Segment<'a> {
    fn parse(
        line: &'a str,
        separators: Separators,
        is_header: bool,
        index: usize,
    ) -> Result<Self, ParseError> {
        // In the header the name is the fixed three characters, MSH-1 is the
        // separator that follows them, and MSH-2 begins right after that. The
        // offsets are byte offsets, so the split works for any separator, not
        // only the one-byte ASCII case.
        let (name, rest, header_separator) = if is_header {
            let name_end = line
                .char_indices()
                .nth(3)
                .map(|(i, _)| i)
                .ok_or(ParseError::TruncatedHeader)?;
            let separator = line[name_end..]
                .chars()
                .next()
                .ok_or(ParseError::TruncatedHeader)?;
            let rest_start = name_end + separator.len_utf8();
            (
                &line[..name_end],
                &line[rest_start..],
                Some(&line[name_end..rest_start]),
            )
        } else {
            match line.split_once(separators.field) {
                Some((name, rest)) => (name, rest, None),
                None => (line, "", None),
            }
        };
        if name.is_empty() {
            return Err(ParseError::UnnamedSegment { index });
        }

        let mut fields: Vec<Field<'a>> = Vec::new();
        if let Some(separator) = header_separator {
            fields.push(Field::new(separator, separators));
        }
        if !rest.is_empty() {
            fields.extend(
                rest.split(separators.field)
                    .map(|f| Field::new(f, separators)),
            );
        } else if !is_header {
            // A bare segment name still has one, empty, field, which is how
            // `EVN` with nothing after it is read.
            fields.push(Field::new("", separators));
        }

        Ok(Self {
            name,
            fields,
            separators,
            is_header,
        })
    }

    /// The segment name, such as `MSH` or `PID`.
    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }

    /// Every field, with MSH-1 first for the header and field 1 first for any
    /// other segment. Field `n` is at index `n - 1`, matching HL7 numbering.
    #[must_use]
    pub fn fields(&self) -> &[Field<'a>] {
        &self.fields
    }

    /// Field `index`, numbered from 1 as in the standard, so `PID-5` is
    /// `segment.field(5)`.
    #[must_use]
    pub fn field(&self, index: usize) -> Option<&Field<'a>> {
        index.checked_sub(1).and_then(|i| self.fields.get(i))
    }

    /// Serialises the segment back to its ER7 text.
    #[must_use]
    pub fn to_er7(&self) -> String {
        let mut out = String::with_capacity(self.name.len() + self.fields.len());
        out.push_str(self.name);
        for (index, field) in self.fields.iter().enumerate() {
            // MSH-1 is the separator that follows the name and MSH-2 begins
            // right after it, so neither is preceded by another separator.
            if self.is_header && index <= 1 {
                out.push_str(field.text());
            } else {
                out.push(self.separators.field);
                out.push_str(field.text());
            }
        }
        out
    }
}

/// One field: repetitions of components of subcomponents, as one slice.
///
/// The structure is not split up front. A caller that wants the whole field
/// takes [`Field::text`], one that wants a component calls
/// [`Field::component`], and the slices come out of the same borrowed string.
#[derive(Debug, Clone, Copy)]
pub struct Field<'a> {
    raw: &'a str,
    separators: Separators,
}

impl<'a> Field<'a> {
    fn new(raw: &'a str, separators: Separators) -> Self {
        Self { raw, separators }
    }

    /// The field exactly as it appeared, delimiters and escapes untouched.
    #[must_use]
    pub const fn text(&self) -> &'a str {
        self.raw
    }

    /// Whether the field is empty, which in HL7 means absent.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.raw.is_empty()
    }

    /// The repetitions of the field, split on the repetition separator. A field
    /// with no repetition separator yields one item.
    pub fn repetitions(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.raw.split(self.separators.repetition)
    }

    /// The first repetition, which is the value of the field when it does not
    /// repeat.
    #[must_use]
    pub fn value(&self) -> &'a str {
        self.repetitions().next().unwrap_or("")
    }

    /// Component `index` of the first repetition, numbered from 1, so `PID-5.1`
    /// is `field.component(1)`.
    #[must_use]
    pub fn component(&self, index: usize) -> Option<&'a str> {
        index
            .checked_sub(1)
            .and_then(|i| self.value().split(self.separators.component).nth(i))
    }

    /// The components of the first repetition, in order.
    pub fn components(&self) -> impl Iterator<Item = &'a str> + '_ {
        self.value().split(self.separators.component)
    }

    /// Subcomponent `index` of component `component`, both numbered from 1.
    #[must_use]
    pub fn subcomponent(&self, component: usize, index: usize) -> Option<&'a str> {
        index.checked_sub(1).and_then(|i| {
            self.component(component)?
                .split(self.separators.subcomponent)
                .nth(i)
        })
    }

    /// The field with its delimiter escapes resolved.
    ///
    /// The five standard escapes are replaced by the characters they stand for:
    /// `\F\` `\S\` `\R\` `\E\` `\T\`. Anything else between backslashes is left
    /// as written, because the remaining escape sequences carry formatting
    /// instructions a plain string cannot represent, and dropping them silently
    /// would lose data.
    #[must_use]
    pub fn unescape(&self) -> String {
        self.value().unescape(self.separators)
    }
}

/// Escapes the delimiters in `text` so it can be written as one primitive value.
///
/// This is the inverse of the delimiter resolution [`Field::unescape`] does, and
/// the form a caller-supplied value must be written in: a value that holds the
/// field separator would otherwise end the field and shift every field after it,
/// which a reader cannot detect. The five standard escapes are produced: `\F\`
/// `\S\` `\R\` `\T\` `\E\`.
///
/// `text` is plain text, not ER7. Escaping a string that still holds its own
/// escapes doubles them, so a caller re-emitting a parsed field unchanged wants
/// [`Field::text`], not this function.
#[must_use]
pub fn escape(text: &str, separators: Separators) -> String {
    // Every value goes through this, and most hold no delimiter, so the scan is
    // skipped rather than allocating a copy that would be identical.
    if !text.chars().any(|c| is_delimiter(c, separators)) {
        return text.to_owned();
    }
    let mut out = String::with_capacity(text.len() + 8);
    for c in text.chars() {
        push_escaped(&mut out, c, separators);
    }
    out
}

/// Whether `c` is one of the five delimiters of `separators`.
fn is_delimiter(c: char, separators: Separators) -> bool {
    c == separators.field
        || c == separators.component
        || c == separators.repetition
        || c == separators.escape
        || c == separators.subcomponent
}

/// Appends one character of plain text, escaped if `separators` treats it as a
/// delimiter.
pub(crate) fn push_escaped(out: &mut String, c: char, separators: Separators) {
    let code = if c == separators.field {
        'F'
    } else if c == separators.component {
        'S'
    } else if c == separators.repetition {
        'R'
    } else if c == separators.escape {
        'E'
    } else if c == separators.subcomponent {
        'T'
    } else {
        out.push(c);
        return;
    };
    out.push(separators.escape);
    out.push(code);
    out.push(separators.escape);
}

/// What a character means in an ER7 delimiter set.
///
/// A reply can only carry a header value whose characters keep the role they had
/// in the message being answered. `Data` is not a delimiter but is just as
/// important: a character that is ordinary data in one set can be a separator in
/// another, and that change alters how a reader splits the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DelimiterRole {
    /// MSH-1, which separates fields.
    Field,
    /// MSH-2's first character, which separates the components of a field.
    Component,
    /// MSH-2's second character, which separates repetitions.
    Repetition,
    /// MSH-2's third character, which opens and closes an escape.
    Escape,
    /// MSH-2's fourth character, which separates subcomponents.
    Subcomponent,
    /// Not one of the five delimiters, so the character is plain content.
    Data,
}

impl std::fmt::Display for DelimiterRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Field => "field",
            Self::Component => "component",
            Self::Repetition => "repetition",
            Self::Escape => "escape",
            Self::Subcomponent => "subcomponent",
            Self::Data => "data",
        })
    }
}

/// Why [`transcode`] cannot re-emit a header value.
///
/// Both variants describe a value the reply cannot carry as written, and they are
/// separate because the caller acts on them differently: a raw field separator is
/// a bug in a hand-built context, while an opaque escape body is a legal value the
/// chosen reply delimiters cannot express.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TranscodeError {
    /// A site-defined escape whose body holds a delimiter whose *role* changes
    /// in the reply.
    OpaqueEscapeCollision {
        /// The escape body, without its delimiters.
        body: String,
        /// The delimiter in the body that changes role.
        delimiter: char,
        /// What the character means in the source.
        in_source: DelimiterRole,
        /// What the same character would mean in the reply.
        in_reply: DelimiterRole,
    },
    /// A raw field separator in the value, which only a hand-built context can
    /// produce.
    RawFieldSeparator {
        /// The offending separator.
        separator: char,
    },
}

impl std::fmt::Display for TranscodeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::OpaqueEscapeCollision {
                body,
                delimiter,
                in_source,
                in_reply,
            } => write!(
                f,
                "the site-defined escape \\{body}\\ contains {delimiter:?}, which is {} \
                 in the message but {} in the reply",
                article(*in_source),
                article(*in_reply)
            ),
            Self::RawFieldSeparator { separator } => {
                write!(
                    f,
                    "the value contains the raw field separator {separator:?}"
                )
            }
        }
    }
}

/// `a field separator`, `an escape character`, `plain data`, for reading in a
/// sentence.
pub(crate) fn article(role: DelimiterRole) -> String {
    match role {
        DelimiterRole::Escape => "an escape character".to_owned(),
        DelimiterRole::Data => "plain data".to_owned(),
        _ => format!("a {role} separator"),
    }
}

/// The role a character plays in a delimiter set.
fn role_of(character: char, separators: Separators) -> DelimiterRole {
    if character == separators.field {
        DelimiterRole::Field
    } else if character == separators.component {
        DelimiterRole::Component
    } else if character == separators.repetition {
        DelimiterRole::Repetition
    } else if character == separators.escape {
        DelimiterRole::Escape
    } else if character == separators.subcomponent {
        DelimiterRole::Subcomponent
    } else {
        DelimiterRole::Data
    }
}

/// Re-emits ER7 text read with `from` using `to`.
///
/// Structure is kept. A component separator stays a component separator, so an
/// HD value such as `LAB^1.2.3^ISO` keeps its three components; a delimiter that
/// was *escaped* in the source was data and stays data. Decoding first and
/// escaping the result would lose that distinction - every structural separator
/// would become a literal one, which is exactly the corruption this avoids.
///
/// # Errors
///
/// A raw `field` separator is refused outright, whatever `to` is. It cannot occur
/// in a parsed field - the parser splits on it - so one can only come from a
/// hand-built context, where it was meant as data but would be read as structure.
/// Checking it before anything else keeps the verdict the same for every reply
/// delimiter set, so a caller cannot make an invalid header acceptable by
/// changing the reply.
///
/// A *local* escape - one whose body is not `F`, `S`, `R`, `E` or `T`, such as
/// the site-defined `\ZLAB#1\` - has an opaque body this function cannot
/// rewrite. A reader splits on a delimiter before it resolves escapes, so the
/// body's structure is whatever `from` says it is. That structure survives a
/// conversion only when every delimiter in the body keeps its role; if one would
/// become a different delimiter, or data, no rewrite of the body can carry it and
/// the offending character is returned.
///
/// Answering in `from`'s own delimiter set is never refused: it is the documented
/// fallback, the value is already valid ER7 under those delimiters, and copying
/// it introduces nothing new.
pub(crate) fn transcode(
    er7: &str,
    from: Separators,
    to: Separators,
) -> Result<String, TranscodeError> {
    if er7.contains(from.field) {
        return Err(TranscodeError::RawFieldSeparator {
            separator: from.field,
        });
    }
    // Nothing needs rewriting, and an opaque body holding the source's own
    // delimiters is legal - the parser accepted it - because copying preserves
    // every role.
    if from == to {
        return Ok(er7.to_owned());
    }
    // Read the structure first. A reader splits on the component separator before
    // it resolves escapes, so a pair of escape characters that straddles one is
    // not an escape at all: it is a marker ending one component and another
    // starting the next. Rewriting such a pair as an escape changes literal
    // characters in the identity, which is why this walks the components rather
    // than the raw text.
    let mut out = String::with_capacity(er7.len());
    for (index, component) in er7.split(from.component).enumerate() {
        if index > 0 {
            out.push(to.component);
        }
        transcode_component(component, from, to, &mut out)?;
    }
    Ok(out)
}

/// Re-emits `text` keeping its structure: separators stay separators.
fn push_structure(out: &mut String, text: &str, from: Separators, to: Separators) {
    for c in text.chars() {
        push_structure_char(out, c, from, to);
    }
}

/// Re-emits one character, keeping a repetition or subcomponent boundary.
fn push_structure_char(out: &mut String, c: char, from: Separators, to: Separators) {
    if c == from.repetition {
        out.push(to.repetition);
    } else if c == from.subcomponent {
        out.push(to.subcomponent);
    } else {
        push_escaped(out, c, to);
    }
}

/// Re-emits one component, where escape markers are balanced by construction.
fn transcode_component(
    component: &str,
    from: Separators,
    to: Separators,
    out: &mut String,
) -> Result<(), TranscodeError> {
    let mut chars = component.chars().peekable();
    while let Some(c) = chars.next() {
        if c != from.escape {
            push_structure_char(out, c, from, to);
            continue;
        }
        let mut body = String::new();
        let mut closed = false;
        for inner in chars.by_ref() {
            if inner == from.escape {
                closed = true;
                break;
            }
            body.push(inner);
        }
        if !closed {
            // The component ends with an unmatched escape. The marker is data,
            // because that is what a reader sees, but the delimiters after it are
            // still structure: escaping the whole tail as text would merge
            // subcomponents that the source kept apart. Only the marker itself is
            // escaped, so the boundaries after it survive.
            push_escaped(out, from.escape, to);
            push_structure(out, &body, from, to);
            continue;
        }
        let delimiter = match body.as_str() {
            "F" => Some(from.field),
            "S" => Some(from.component),
            "R" => Some(from.repetition),
            "E" => Some(from.escape),
            "T" => Some(from.subcomponent),
            _ => None,
        };
        match delimiter {
            // The escape stood for a delimiter, so it was data under `from` and
            // is data under `to`.
            Some(delimiter) => push_escaped(out, delimiter, to),
            None => {
                // The body is opaque, so a delimiter inside it is structure
                // under `from`. Every character must keep its role for the
                // structure to survive: a separator that becomes data stops
                // splitting the field, and data that becomes a separator starts
                // splitting it. Both change how many components the field has,
                // and neither can be repaired inside a body whose meaning is the
                // site's.
                for character in body.chars() {
                    let in_source = role_of(character, from);
                    let in_reply = role_of(character, to);
                    if in_source != in_reply {
                        return Err(TranscodeError::OpaqueEscapeCollision {
                            body,
                            delimiter: character,
                            in_source,
                            in_reply,
                        });
                    }
                }
                out.push(to.escape);
                out.push_str(&body);
                out.push(to.escape);
            }
        }
    }
    Ok(())
}

/// Resolves the delimiter escapes in a value.
trait Unescape {
    fn unescape(&self, separators: Separators) -> String;
}

impl Unescape for str {
    fn unescape(&self, separators: Separators) -> String {
        if !self.contains(separators.escape) {
            return self.to_owned();
        }
        let mut out = String::with_capacity(self.len());
        let mut chars = self.chars().peekable();
        while let Some(c) = chars.next() {
            if c != separators.escape {
                out.push(c);
                continue;
            }
            // HL7 escapes are delimited: `\F\` is the field separator, not a
            // backslash followed by `F`. So the character that ends the escape
            // is another escape character, and an escape never nests.
            let mut body = String::new();
            let mut closed = false;
            for inner in chars.by_ref() {
                if inner == separators.escape {
                    closed = true;
                    break;
                }
                body.push(inner);
            }
            if !closed {
                // No closing escape: the text is malformed, so keep it as it
                // was rather than inventing a delimiter.
                out.push(separators.escape);
                out.push_str(&body);
                continue;
            }
            match body.as_str() {
                "F" => out.push(separators.field),
                "S" => out.push(separators.component),
                "R" => out.push(separators.repetition),
                "E" => out.push(separators.escape),
                "T" => out.push(separators.subcomponent),
                // A formatting instruction or an escape this crate does not
                // model: keep it verbatim so nothing is lost.
                _ => {
                    out.push(separators.escape);
                    out.push_str(&body);
                    out.push(separators.escape);
                }
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "MSH|^~\\&|SEND|FAC|RECV|FAC|20260101||ADT^A01|1|P|2.5.1\r\
                          PID|1||12345^^^MRN~67890^^^MRN||DOE^JOHN^A^Jr\r";

    #[test]
    fn segments_and_names_come_out_in_order() {
        let message = Message::parse(SAMPLE).unwrap();
        assert_eq!(message.segment_names().collect::<Vec<_>>(), ["MSH", "PID"]);
    }

    #[test]
    fn the_header_separators_are_read_from_the_message() {
        let message = Message::parse(SAMPLE).unwrap();
        assert_eq!(message.separators(), Separators::default());
        assert_eq!(message.field("MSH-1").unwrap().text(), "|");
        assert_eq!(message.field("MSH-2").unwrap().text(), "^~\\&");
    }

    #[test]
    fn a_non_default_separator_is_honoured() {
        let message =
            Message::parse("MSH#*~\\&#SEND#FAC#R#F#20260101###ADT*A01#1#P#2.5.1\rPID#1\r").unwrap();
        assert_eq!(message.separators().field, '#');
        assert_eq!(message.separators().component, '*');
        assert_eq!(message.field("MSH-3").unwrap().text(), "SEND");
        assert_eq!(message.field("PID-1").unwrap().text(), "1");
    }

    #[test]
    fn a_field_can_be_addressed_by_segment_and_number() {
        let message = Message::parse(SAMPLE).unwrap();
        assert_eq!(message.field("PID-5").unwrap().text(), "DOE^JOHN^A^Jr");
        assert_eq!(message.field("PID-5").unwrap().component(1), Some("DOE"));
        assert_eq!(message.field("PID-5").unwrap().component(3), Some("A"));
        assert_eq!(message.field("PID-5").unwrap().component(4), Some("Jr"));
    }

    #[test]
    fn repetitions_split_on_the_repetition_separator() {
        let message = Message::parse(SAMPLE).unwrap();
        let field = message.field("PID-3").unwrap();
        assert_eq!(
            field.repetitions().collect::<Vec<_>>(),
            ["12345^^^MRN", "67890^^^MRN"]
        );
        assert_eq!(field.value(), "12345^^^MRN");
        assert_eq!(field.component(1), Some("12345"));
        assert_eq!(field.component(4), Some("MRN"));
    }

    #[test]
    fn subcomponents_split_on_the_subcomponent_separator() {
        let message = Message::parse(
            "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|1|P|2.5.1\r\
                                      PID|1||12345^^^MRN&MR&A^A\r",
        )
        .unwrap();
        let field = message.field("PID-3").unwrap();
        assert_eq!(field.component(4), Some("MRN&MR&A"));
        assert_eq!(field.subcomponent(4, 1), Some("MRN"));
        assert_eq!(field.subcomponent(4, 2), Some("MR"));
    }

    #[test]
    fn the_value_of_a_field_resolves_delimiter_escapes() {
        let message = Message::parse(
            "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|1|P|2.5.1\r\
                                      PID|1||A\\F\\B\\S\\C\\T\\D\\E\\E\r",
        )
        .unwrap();
        assert_eq!(
            message.value("PID-3").unwrap(),
            "A|B^C&D\\E",
            "each escape becomes the delimiter it names"
        );
    }

    #[test]
    fn an_unknown_escape_is_left_alone() {
        let message = Message::parse(
            "MSH|^~\\&|S|F|R|F|20260101||ADT^A01|1|P|2.5.1\r\
                                      PID|1||A\\.br\\B\r",
        )
        .unwrap();
        assert_eq!(message.value("PID-3").unwrap(), "A\\.br\\B");
    }

    #[test]
    fn a_repeating_segment_is_found_by_name() {
        let message = Message::parse(
            "MSH|^~\\&|S|F|R|F|20260101||ORU^R01|1|P|2.5.1\r\
                                      OBX|1|ST|A||one\rOBX|2|ST|B||two\r",
        )
        .unwrap();
        let values: Vec<_> = message
            .segments_named("OBX")
            .map(|s| s.field(5).unwrap().text())
            .collect();
        assert_eq!(values, ["one", "two"]);
        assert_eq!(
            message.segment("OBX").unwrap().field(5).unwrap().text(),
            "one"
        );
    }

    #[test]
    fn line_feeds_and_carriage_returns_both_end_a_segment() {
        let crlf =
            Message::parse("MSH|^~\\&|S|F|R|F|20260101||ADT^A01|1|P|2.5.1\r\nPID|1\r").unwrap();
        let lf = Message::parse("MSH|^~\\&|S|F|R|F|20260101||ADT^A01|1|P|2.5.1\nPID|1\n").unwrap();
        assert_eq!(crlf.segment_names().collect::<Vec<_>>(), ["MSH", "PID"]);
        assert_eq!(lf.segment_names().collect::<Vec<_>>(), ["MSH", "PID"]);
    }

    #[test]
    fn an_absent_field_and_an_empty_field_are_both_empty() {
        let message = Message::parse(SAMPLE).unwrap();
        // PID-2 is the empty string between two separators; PID-9 is past the
        // end of the segment.
        assert!(message.field("PID-2").unwrap().is_empty());
        assert!(message.field("PID-9").is_none());
        assert!(message.field("NOPE-1").is_none());
    }

    #[test]
    fn the_message_round_trips_through_er7() {
        let message = Message::parse(SAMPLE).unwrap();
        // Segments are joined with a carriage return and no trailing one is
        // added, so the output equals the input modulo the final terminator.
        assert_eq!(message.to_er7(), SAMPLE.trim_end_matches('\r'));
        assert_eq!(
            message.text(),
            SAMPLE,
            "the original bytes are kept as given"
        );
    }

    #[test]
    fn a_message_without_a_header_is_rejected() {
        assert_eq!(
            Message::parse("PID|1\r").unwrap_err(),
            ParseError::MissingHeader
        );
        assert_eq!(Message::parse("").unwrap_err(), ParseError::Empty);
    }

    #[test]
    fn a_header_without_its_delimiters_is_rejected() {
        assert_eq!(
            Message::parse("MSH").unwrap_err(),
            ParseError::TruncatedHeader
        );
        assert_eq!(
            Message::parse("MSH|^~").unwrap_err(),
            ParseError::TruncatedHeader
        );
    }
}
