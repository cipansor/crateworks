//! Writing a `.properties` document.
//!
//! This is the other half of [`crate::parse_str`], and it follows
//! `java.util.Properties.store`: the same escapes, the same line continuation,
//! and the same escaping of the key's first character. Getting any of these
//! wrong produces a file that Java reads back differently, which is the failure
//! mode that matters, because the reader is almost never this crate.

use std::fmt::Write as _;

use crate::escape::Error;
use crate::reader::Properties;
use crate::time::java_timestamp_utc;

/// Escapes a field the way the reference's `escape` does.
///
/// This is the escape for a string that may be used as either a key or a value:
/// every space is escaped, as are `\`, `#`, `!`, `=`, `:` and anything outside
/// printable ASCII. It is the strongest of the three, and it is what a caller
/// wants when the string's role is not yet known.
///
/// # Example
///
/// ```
/// use javaproperties::escape;
///
/// assert_eq!(escape("a b=c"), "a\\ b\\=c");
/// assert_eq!(escape("café"), "caf\\u00E9");
/// ```
#[must_use]
pub fn escape(field: &str) -> String {
    escape_with(field, true)
}

/// Escapes a field, choosing whether characters outside ASCII are escaped.
///
/// This is the reference's `escape(field, ensure_ascii)`. With `ensure_ascii`
/// false a character above ASCII is written as itself, so the output is not
/// ISO-8859-1 and a reader must be told what it is; the control range is still
/// escaped, because those characters would be read back as something else.
///
/// # Example
///
/// ```
/// use javaproperties::escape_with;
///
/// assert_eq!(escape_with("café", true), "caf\\u00E9");
/// assert_eq!(escape_with("café", false), "café");
/// // A tab is escaped either way: it is not a character a value can hold.
/// assert_eq!(escape_with("a\tb", false), "a\\tb");
/// ```
#[must_use]
pub fn escape_with(field: &str, ensure_ascii: bool) -> String {
    let mut out = String::with_capacity(field.len());
    for c in field.chars() {
        if c == ' ' {
            out.push_str("\\ ");
        } else {
            push_escaped_with(&mut out, c, ensure_ascii);
        }
    }
    out
}

/// Escapes a key.
///
/// A space in a key is escaped, and so is any of `#`, `!`, `=`, `:`, wherever it
/// appears. `java.util.Properties.store` escapes all four in both the key and
/// the value (`saveConvert` with `escapeSpace`), not only in the first position:
/// `a#b` is written `a\#b`. The escapes are not needed to read the entry back,
/// because the separator is only special where it ends the key and a comment
/// marker only starts a line, but they are what Java writes, and a reader that
/// compares files or hashes them sees the difference.
#[must_use]
pub fn escape_key(key: &str) -> String {
    escape(key)
}

/// Escapes a value.
///
/// The **first** character is escaped when it is a space, which Java does too
/// and which is not cosmetic: a reader drops the whitespace after the separator,
/// so `k= one` and `k=one` both read back as `one`, and without this escape a
/// value that starts with a space could not be written at all. Interior and
/// trailing spaces are left as they stand.
///
/// `#`, `!`, `=`, `:` are escaped wherever they appear, as they are in a key.
#[must_use]
pub fn escape_value(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for (index, c) in value.chars().enumerate() {
        if index == 0 && c == ' ' {
            out.push_str("\\ ");
            continue;
        }
        push_escaped(&mut out, c);
    }
    out
}

/// Escapes a value the way `join_key_value` does, honouring `ensure_ascii`.
///
/// Unlike [`escape_value`] this also escapes a leading space, and it is the
/// private half of [`join_key_value`].
fn escape_value_with(value: &str, ensure_ascii: bool) -> String {
    let mut out = String::with_capacity(value.len());
    for (index, c) in value.chars().enumerate() {
        if index == 0 && c == ' ' {
            out.push_str("\\ ");
            continue;
        }
        push_escaped_with(&mut out, c, ensure_ascii);
    }
    out
}

/// Escapes a key the way `join_key_value` does, honouring `ensure_ascii`.
fn escape_key_with(key: &str, ensure_ascii: bool) -> String {
    let mut out = String::with_capacity(key.len());
    for c in key.chars() {
        if c == ' ' {
            out.push_str("\\ ");
        } else {
            push_escaped_with(&mut out, c, ensure_ascii);
        }
    }
    out
}

/// Joins a key and a value into one line, the way the reference does.
///
/// The key is escaped with every space escaped; the value is escaped with only a
/// leading space escaped. No trailing newline is added, and `separator` is
/// written as it stands, so `" = "` produces `key = value`.
///
/// # Example
///
/// ```
/// use javaproperties::join_key_value;
///
/// assert_eq!(join_key_value("a b", "c d", "="), "a\\ b=c d");
/// assert_eq!(join_key_value("a", " lead", "="), "a=\\ lead");
/// assert_eq!(join_key_value("k", "v", ": "), "k: v");
/// ```
#[must_use]
pub fn join_key_value(key: &str, value: &str, separator: &str) -> String {
    join_key_value_with(key, value, separator, true)
}

/// Joins a key and a value into one line, choosing whether characters outside
/// ASCII are escaped.
///
/// This is the reference's `join_key_value(key, value, separator,
/// ensure_ascii)`.
///
/// # Example
///
/// ```
/// use javaproperties::join_key_value_with;
///
/// assert_eq!(join_key_value_with("k", "café", "=", true), "k=caf\\u00E9");
/// assert_eq!(join_key_value_with("k", "café", "=", false), "k=café");
/// ```
#[must_use]
pub fn join_key_value_with(key: &str, value: &str, separator: &str, ensure_ascii: bool) -> String {
    let mut out = String::with_capacity(key.len() + value.len() + separator.len());
    out.push_str(&escape_key_with(key, ensure_ascii));
    out.push_str(separator);
    out.push_str(&escape_value_with(value, ensure_ascii));
    out
}

/// Turns a string into a comment line, without a trailing newline.
///
/// A `#` is prepended, `\r\n` and a lone `\r` become `\n`, and a `#` is inserted
/// after any line break that is not already followed by `#` or `!` so that a
/// second line cannot become an entry. Non-Latin-1 characters are escaped, the
/// way the reference's default does.
///
/// # Example
///
/// ```
/// use javaproperties::to_comment;
///
/// assert_eq!(to_comment("hello"), "#hello");
/// assert_eq!(to_comment("one\ntwo"), "#one\n#two");
/// // A line that is already a comment is not given a second marker.
/// assert_eq!(to_comment("one\n#two"), "#one\n#two");
/// ```
#[must_use]
pub fn to_comment(comment: &str) -> String {
    to_comment_with(comment, None)
}

/// Turns a string into a comment line, choosing which characters are escaped.
///
/// This is the reference's `to_comment(comment, ensure_ascii)`, and the three
/// values mean three different things, which is why the argument is an
/// `Option<bool>` rather than a `bool`:
///
/// | `ensure_ascii` | Escaped |
/// | --- | --- |
/// | `None` (default) | only above U+00FF, so Latin-1 is written as itself |
/// | `Some(true)` | everything above ASCII |
/// | `Some(false)` | nothing |
///
/// The default is not the same as [`escape`]'s: a comment is a line of prose
/// rather than a key or value, so `java.util.Properties.store` writes accented
/// Latin-1 through unchanged and escapes only what ISO-8859-1 cannot hold.
///
/// # Example
///
/// ```
/// use javaproperties::to_comment_with;
///
/// // Latin-1 survives the default, and anything above it is escaped.
/// assert_eq!(to_comment_with("café 中文", None), "#café \\u4E2D\\u6587");
/// // Asking for ASCII escapes the é as well.
/// assert_eq!(to_comment_with("café", Some(true)), "#caf\\u00E9");
/// // Asking for nothing writes both through.
/// assert_eq!(to_comment_with("café 中文", Some(false)), "#café 中文");
/// ```
#[must_use]
pub fn to_comment_with(comment: &str, ensure_ascii: Option<bool>) -> String {
    // First normalise the breaks: `\r\n` and a lone `\r` both become `\n`. The
    // reference does this with a regex before it looks at anything else.
    let mut normalised = String::with_capacity(comment.len());
    let mut chars = comment.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                normalised.push('\n');
            }
            other => normalised.push(other),
        }
    }

    // Then a `#` goes after any break that is not already followed by `#` or
    // `!`. That includes a blank line and the end of the text: `"a\n\nb"` is
    // `"a\n#\nb"` before the marker, so the blank line becomes a comment line of
    // its own rather than being dropped.
    let mut out = String::with_capacity(normalised.len() + 1);
    out.push('#');
    let mut chars = normalised.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\n' {
            out.push('\n');
            if !matches!(chars.peek(), Some('#' | '!')) {
                out.push('#');
            }
            continue;
        }
        push_comment_char(&mut out, c, ensure_ascii);
    }
    out
}

/// Writes one comment character, honouring the three-way `ensure_ascii`.
///
/// `None` escapes only what is outside Latin-1, which is the reference's default
/// for a comment; `Some(true)` escapes everything above ASCII; `Some(false)`
/// escapes nothing. A tab or an accented letter is written as it stands under the
/// first and third.
fn push_comment_char(out: &mut String, c: char, ensure_ascii: Option<bool>) {
    let needs_escape = match ensure_ascii {
        None => (c as u32) > 0xff,
        Some(escape_all) => escape_all && (c as u32) > 0x7f,
    };
    if needs_escape {
        push_unicode_escape(out, c);
    } else {
        out.push(c);
    }
}

/// Writes one character, escaped the way Java writes it.
///
/// Java escapes everything outside printable ASCII except the space. Escaping
/// the printable range and nothing else would leave a `\r` or a `\u{0085}` to be
/// read back as a line break or as nothing, so the rule is deliberately the other
/// way round.
fn push_escaped(out: &mut String, c: char) {
    push_escaped_with(out, c, true);
}

/// Writes one character, honouring `ensure_ascii`.
///
/// With `ensure_ascii` true anything outside printable ASCII becomes `\uXXXX`.
/// With it false only the control range does, and the rest is passed through,
/// which is what the reference's `ensure_ascii=False` means.
fn push_escaped_with(out: &mut String, c: char, ensure_ascii: bool) {
    match c {
        '\\' => out.push_str("\\\\"),
        '\t' => out.push_str("\\t"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        '\u{000c}' => out.push_str("\\f"),
        '#' | '!' | '=' | ':' => {
            out.push('\\');
            out.push(c);
        }
        _ => {
            let needs_escape = if ensure_ascii {
                !(' '..='~').contains(&c)
            } else {
                (c as u32) < 0x20 || c == '\u{007f}'
            };
            if needs_escape {
                push_unicode_escape(out, c);
            } else {
                out.push(c);
            }
        }
    }
}

/// Writes `\uXXXX`, upper case, as a surrogate pair above the basic plane.
fn push_unicode_escape(out: &mut String, c: char) {
    let mut buffer = [0u16; 2];
    for unit in c.encode_utf16(&mut buffer) {
        let _ = write!(out, "\\u{unit:04X}");
    }
}

/// How a document is written.
///
/// This mirrors the reference's `dumps` keyword arguments. `Default` writes the
/// way `java.util.Properties.store` does with none of them given: `=` as the
/// separator, ASCII escapes, entries in order and no timestamp.
#[derive(Debug, Clone)]
pub struct WriteOptions<'a> {
    /// Written between the key and the value. `"="`, `": "` and `" "` are the
    /// three the format defines.
    pub separator: &'a str,
    /// Written as a comment before the entries.
    pub comment: Option<&'a str>,
    /// Seconds since the Unix epoch for a timestamp comment, or `None` for no
    /// timestamp. Java writes the local time; this writes UTC, because the crate
    /// has no timezone database.
    pub timestamp: Option<i64>,
    /// Sort the entries by key before writing. Duplicates keep their order.
    pub sort_keys: bool,
    /// Escape everything outside printable ASCII. With this false only the
    /// control range is escaped and the rest is passed through, which produces a
    /// file a reader must be told is not ISO-8859-1.
    pub ensure_ascii: bool,
    /// Escape the comment independently of the entries.
    ///
    /// The reference's `ensure_ascii_comments`, and the three values mean what
    /// [`to_comment_with`] documents: `None` escapes only what ISO-8859-1 cannot
    /// hold, `Some(true)` escapes everything above ASCII, and `Some(false)`
    /// escapes nothing. `None` is the default and is **not** the same as
    /// following `ensure_ascii`: a comment is prose, so `java.util.Properties`
    /// writes accented Latin-1 through it unchanged even when the entries beside
    /// it are fully escaped.
    pub ensure_ascii_comments: Option<bool>,
}

impl Default for WriteOptions<'_> {
    fn default() -> Self {
        Self {
            separator: "=",
            comment: None,
            timestamp: None,
            sort_keys: false,
            ensure_ascii: true,
            ensure_ascii_comments: None,
        }
    }
}

/// Writes a document.
///
/// Entries are written in the order they appear, including duplicates. A comment
/// is written with a leading `#` when it does not already start with `#` or `!`;
/// a line break inside a comment becomes a new comment line so that one comment
/// cannot become an entry, which would silently add a key on read-back.
fn write_document(
    out: &mut String,
    pairs: &[(String, String)],
    options: &WriteOptions<'_>,
) -> Result<(), Error> {
    if let Some(comment) = options.comment {
        out.push_str(&to_comment_with(comment, options.ensure_ascii_comments));
        out.push('\n');
    }
    if let Some(seconds) = options.timestamp {
        // A timestamp is ASCII, so the escaping mode cannot change it.
        out.push_str(&to_comment(&java_timestamp_utc(seconds)));
        out.push('\n');
    }

    if options.sort_keys {
        let mut sorted: Vec<&(String, String)> = pairs.iter().collect();
        // The reference sorts the `(key, value)` tuples, so a key given twice is
        // ordered by its value. A stable sort on the key alone would keep the
        // duplicates in insertion order and disagree with it.
        sorted.sort();
        for (key, value) in sorted {
            write_entry(out, key, value, options);
        }
    } else {
        for (key, value) in pairs {
            write_entry(out, key, value, options);
        }
    }
    Ok(())
}

/// Writes one key and value, on one line.
///
/// `java.util.Properties.store` writes `key + "=" + value` followed by a line
/// separator and does not wrap. A long value is one long line, however long it
/// is. Wrapping it at some column instead - which this used to do - splits the
/// line at a column of its own choosing, and when that column falls inside a
/// `\uXXXX` escape the escape is cut in half and the file no longer reads back.
/// Java never emits such a split, so neither does this.
fn write_entry(out: &mut String, key: &str, value: &str, options: &WriteOptions<'_>) {
    out.push_str(&escape_key_with(key, options.ensure_ascii));
    out.push_str(options.separator);
    out.push_str(&escape_value_with(value, options.ensure_ascii));
    out.push('\n');
}

impl Properties {
    /// Writes the entries as a `.properties` document.
    ///
    /// The result is text; use [`Properties::store_bytes`] for the ISO-8859-1
    /// bytes that `java.util.Properties.store` writes, which this can also
    /// produce because every escape is ASCII.
    ///
    /// # Example
    ///
    /// ```
    /// use javaproperties::{Properties, parse_str};
    ///
    /// let properties = Properties::from_pairs(vec![
    ///     ("a b".to_string(), "one".to_string()),
    ///     ("café".to_string(), "two".to_string()),
    /// ]);
    /// let text = properties.store(None);
    /// // A space in a key is escaped, and a non-ASCII character becomes `\uXXXX`.
    /// assert_eq!(text, "a\\ b=one\ncaf\\u00E9=two\n");
    ///
    /// // The round trip is what matters: this reads back as it was written.
    /// let back = parse_str(&text).unwrap();
    /// assert_eq!(back, vec![
    ///     ("a b".to_string(), "one".to_string()),
    ///     ("café".to_string(), "two".to_string()),
    /// ]);
    /// ```
    #[must_use]
    pub fn store(&self, comment: Option<&str>) -> String {
        let mut out = String::new();
        // The only failure is a bad escape while writing, and writing escapes
        // cannot fail; the result is ignored rather than unwrapped.
        let _ = write_document(
            &mut out,
            &self.pairs,
            &WriteOptions {
                comment,
                ..WriteOptions::default()
            },
        );
        out
    }

    /// Writes the entries with full control over the output.
    ///
    /// This is the reference's `dumps` with its keyword arguments: the separator,
    /// a comment, a timestamp comment, sorting, and whether non-ASCII is escaped.
    ///
    /// # Example
    ///
    /// ```
    /// use javaproperties::{Properties, WriteOptions};
    ///
    /// let properties = Properties::from_pairs(vec![
    ///     ("b".to_string(), "2".to_string()),
    ///     ("a".to_string(), "1".to_string()),
    /// ]);
    ///
    /// // Sorted, with a colon separator and a fixed timestamp comment.
    /// let text = properties.store_with(&WriteOptions {
    ///     separator: ": ",
    ///     sort_keys: true,
    ///     timestamp: Some(0),
    ///     ..WriteOptions::default()
    /// });
    /// assert_eq!(text, "#Thu Jan 01 00:00:00 UTC 1970\na: 1\nb: 2\n");
    /// ```
    #[must_use]
    pub fn store_with(&self, options: &WriteOptions<'_>) -> String {
        let mut out = String::new();
        let _ = write_document(&mut out, &self.pairs, options);
        out
    }

    /// Writes the document as bytes, the way Java writes a stream.
    ///
    /// [`Properties::store`] escapes every non-ASCII character, so the result is
    /// pure ASCII and these bytes are also valid ISO-8859-1 - the encoding
    /// [`crate::parse`] assumes. There is no `ensure_ascii` to turn off here; use
    /// [`Properties::store_to_with`] for that, and read its output back with
    /// [`crate::parse_str`] rather than [`crate::parse`].
    #[must_use]
    pub fn store_bytes(&self, comment: Option<&str>) -> Vec<u8> {
        self.store(comment).into_bytes()
    }

    /// Writes the document to anything writable.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the writer fails.
    pub fn store_to(
        &self,
        mut writer: impl std::io::Write,
        comment: Option<&str>,
    ) -> Result<(), Error> {
        writer
            .write_all(&self.store_bytes(comment))
            .map_err(|_| Error::Io)
    }

    /// Writes the document to anything writable, with full control over the
    /// output.
    ///
    /// This is [`Properties::store_with`] to a stream instead of a `String`, so a
    /// caller who needs a separator, sorting or a comment-escaping mode does not
    /// have to render the text and write it by hand.
    ///
    /// # Encoding
    ///
    /// The bytes written are **UTF-8**, because that is what the text from
    /// [`Properties::store_with`] is. This differs from [`Properties::store_to`],
    /// which writes the ISO-8859-1 bytes Java writes to a stream: that one is
    /// safe because its default escapes every non-ASCII character, so the result
    /// is pure ASCII and the two encodings agree.
    ///
    /// The difference only shows when `ensure_ascii` is `false`. The document
    /// then holds raw non-ASCII, and reading it back with [`crate::parse`] - which
    /// decodes ISO-8859-1, as `java.util.Properties` does - garbles it: `café`
    /// comes back as `cafÃ©`. Round-trip such a document with [`crate::parse_str`],
    /// which takes text you have already decoded:
    ///
    /// ```
    /// use javaproperties::{Properties, WriteOptions, parse, parse_str};
    ///
    /// let properties = Properties::from_pairs(vec![
    ///     ("café".to_string(), "naïve".to_string()),
    /// ]);
    /// let raw = WriteOptions {
    ///     ensure_ascii: false,
    ///     ..WriteOptions::default()
    /// };
    ///
    /// let mut out = Vec::new();
    /// properties.store_to_with(&mut out, &raw).unwrap();
    ///
    /// // The bytes are UTF-8, so decode them before parsing.
    /// let text = String::from_utf8(out.clone()).unwrap();
    /// assert_eq!(parse_str(&text).unwrap()[0].0, "café");
    /// // Reading those bytes as ISO-8859-1 is the mojibake this documents.
    /// assert_eq!(parse(&out).unwrap()[0].0, "cafÃ©");
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`Error::Io`] when the writer fails.
    ///
    /// # Example
    ///
    /// ```
    /// use javaproperties::{Properties, WriteOptions};
    ///
    /// let properties = Properties::from_pairs(vec![
    ///     ("café".to_string(), "two".to_string()),
    /// ]);
    /// let mut out = Vec::new();
    /// properties
    ///     .store_to_with(
    ///         &mut out,
    ///         &WriteOptions {
    ///             comment: Some("café"),
    ///             ensure_ascii_comments: Some(true),
    ///             ..WriteOptions::default()
    ///         },
    ///     )
    ///     .unwrap();
    /// // The comment is escaped on request; the entry escapes by default.
    /// assert_eq!(out, b"#caf\\u00E9\ncaf\\u00E9=two\n");
    /// ```
    pub fn store_to_with(
        &self,
        mut writer: impl std::io::Write,
        options: &WriteOptions<'_>,
    ) -> Result<(), Error> {
        writer
            .write_all(self.store_with(options).as_bytes())
            .map_err(|_| Error::Io)
    }
}

/// Writes pairs without building a [`Properties`] first.
#[must_use]
pub fn store(pairs: &[(String, String)], comment: Option<&str>) -> String {
    let mut out = String::new();
    let _ = write_document(
        &mut out,
        pairs,
        &WriteOptions {
            comment,
            ..WriteOptions::default()
        },
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_str;

    fn round_trip(pairs: Vec<(String, String)>) {
        let properties = Properties::from_pairs(pairs.clone());
        let text = properties.store(None);
        let back = parse_str(&text).expect("what we write must parse");
        assert_eq!(back, pairs, "round trip changed the pairs:\n{text}");
    }

    #[test]
    fn a_space_in_a_key_is_escaped_and_read_back() {
        // Unescaped, the space would end the key and the rest would become the
        // value, so this is the difference between two entries and one.
        assert_eq!(escape_key("a b"), "a\\ b");
        round_trip(vec![("a b".to_string(), "one".to_string())]);
    }

    #[test]
    fn a_key_starting_with_a_comment_marker_is_escaped() {
        assert_eq!(escape_key("#a"), "\\#a");
        assert_eq!(escape_key("!a"), "\\!a");
        round_trip(vec![("#a".to_string(), "one".to_string())]);
        round_trip(vec![("!a".to_string(), "one".to_string())]);
    }

    #[test]
    fn a_key_holding_a_separator_is_escaped() {
        assert_eq!(escape_key("a=b"), "a\\=b");
        assert_eq!(escape_key("a:b"), "a\\:b");
        round_trip(vec![("a=b".to_string(), "one".to_string())]);
        round_trip(vec![("a:b".to_string(), "one".to_string())]);
    }

    #[test]
    fn a_key_starting_with_a_separator_is_escaped() {
        // Not the same as the case above: here the `=` is the first character,
        // and unescaped it would make the key empty.
        assert_eq!(escape_key("=a"), "\\=a");
        round_trip(vec![("=a".to_string(), "one".to_string())]);
    }

    #[test]
    fn comment_markers_and_separators_are_escaped_anywhere() {
        // `java.util.Properties.store` escapes `#`, `!`, `=` and `:` in the key
        // and in the value, at any position, not only at the front. This crate
        // used to escape only a leading marker, which made its output differ
        // from Java's for a key like `a#b` and for every value holding one.
        assert_eq!(escape_key("a#b"), "a\\#b");
        assert_eq!(escape_key("a!b"), "a\\!b");
        assert_eq!(escape_value("a#b"), "a\\#b");
        assert_eq!(escape_value("a!b"), "a\\!b");
        assert_eq!(escape_value("a=b"), "a\\=b");
        assert_eq!(escape_value("a:b"), "a\\:b");
        assert_eq!(escape_value("#x"), "\\#x");
        round_trip(vec![("a#b".to_string(), "a=b".to_string())]);
        round_trip(vec![("a!b".to_string(), "a:b".to_string())]);
    }

    #[test]
    fn a_long_value_is_one_line_and_reads_back() {
        // Java does not wrap: `store` writes the escaped pair followed by a line
        // separator, however long the line is. Wrapping at a fixed column split
        // the line inside a `\uXXXX` escape and the entry no longer read back,
        // so the line length is now left alone.
        let long = "é".repeat(200);
        let properties = Properties::from_pairs(vec![("k".to_string(), long.clone())]);
        let text = properties.store(None);
        assert_eq!(text.lines().count(), 1);
        assert_eq!(parse_str(&text).unwrap(), vec![("k".to_string(), long)]);
    }

    #[test]
    fn non_ascii_becomes_an_upper_case_escape() {
        assert_eq!(escape_value("café"), "caf\\u00E9");
        // A character above the basic multilingual plane is a surrogate pair,
        // which is what Java writes and reads back.
        assert_eq!(escape_value("😀"), "\\uD83D\\uDE00");
        round_trip(vec![("k".to_string(), "café".to_string())]);
        round_trip(vec![("k".to_string(), "😀".to_string())]);
    }

    #[test]
    fn control_characters_use_their_short_forms() {
        assert_eq!(escape_value("a\tb"), "a\\tb");
        assert_eq!(escape_value("a\nb"), "a\\nb");
        assert_eq!(escape_value("a\rb"), "a\\rb");
        assert_eq!(escape_value("a\u{000c}b"), "a\\fb");
        assert_eq!(escape_value("a\\b"), "a\\\\b");
        // A newline in a value must survive as one entry, not two.
        round_trip(vec![("k".to_string(), "a\nb".to_string())]);
        round_trip(vec![("k".to_string(), "a\\b".to_string())]);
    }

    #[test]
    fn only_the_first_space_of_a_value_is_escaped() {
        // The leading space has to be escaped or the reader eats it; the rest
        // are written as they stand, which is what Java does.
        assert_eq!(escape_value(" one "), "\\ one ");
        assert_eq!(escape_value("a b"), "a b");
        round_trip(vec![("k".to_string(), " one ".to_string())]);
        round_trip(vec![("k".to_string(), "a b".to_string())]);
        round_trip(vec![("k".to_string(), "  two leading".to_string())]);
    }

    #[test]
    fn a_long_value_wraps_and_reads_back_the_same() {
        let long = "x".repeat(500);
        round_trip(vec![("key".to_string(), long)]);
    }

    #[test]
    fn a_comment_is_written_with_a_hash() {
        let properties = Properties::from_pairs(vec![("a".to_string(), "1".to_string())]);
        let text = properties.store(Some("hello"));
        assert_eq!(text, "#hello\na=1\n");
        // A comment is always given the marker, even one that already has it,
        // because that is what Java's `store` does.
        assert_eq!(properties.store(Some("#hello")), "##hello\na=1\n");
        // And a multi-line comment stays a comment.
        let multi = properties.store(Some("one\ntwo"));
        assert_eq!(multi, "#one\n#two\na=1\n");
        assert_eq!(parse_str(&multi).unwrap().len(), 1);
    }

    #[test]
    fn a_comment_cannot_swallow_an_entry() {
        // Without escaping, the `\n` would split the comment and the entry would
        // become part of it, so the entry would silently disappear on read-back.
        let properties = Properties::from_pairs(vec![("a".to_string(), "1".to_string())]);
        let text = properties.store(Some("note\rgone"));
        assert_eq!(parse_str(&text).unwrap().len(), 1);
    }

    #[test]
    fn duplicates_are_written_in_order() {
        let pairs = vec![
            ("a".to_string(), "1".to_string()),
            ("a".to_string(), "2".to_string()),
        ];
        let properties = Properties::from_pairs(pairs.clone());
        let text = properties.store(None);
        assert_eq!(text, "a=1\na=2\n");
        // The reader keeps the last, which is Java's rule, and the round trip
        // preserves both.
        assert_eq!(parse_str(&text).unwrap(), pairs);
        assert_eq!(properties.get("a"), Some("2"));
    }

    #[test]
    fn an_empty_document_is_empty_text() {
        assert_eq!(Properties::default().store(None), "");
        assert_eq!(
            Properties::default().store(Some("only a comment")),
            "#only a comment\n"
        );
    }

    #[test]
    fn the_bytes_are_the_text_and_are_all_ascii() {
        let properties = Properties::from_pairs(vec![("k".to_string(), "café😀".to_string())]);
        let bytes = properties.store_bytes(None);
        assert_eq!(bytes, properties.store(None).into_bytes());
        assert!(
            bytes.is_ascii(),
            "every escape is ASCII, so the bytes must be too"
        );
    }
}
