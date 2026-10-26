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
    let mut out = String::with_capacity(field.len());
    for c in field.chars() {
        if c == ' ' {
            out.push_str("\\ ");
        } else {
            push_escaped(&mut out, c);
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
    let mut out = String::with_capacity(key.len() + value.len() + separator.len());
    out.push_str(&escape_key_with(key, true));
    out.push_str(separator);
    out.push_str(&escape_value_with(value, true));
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
        push_comment_char(&mut out, c);
    }
    out
}

/// Writes one comment character, escaping only what is outside Latin-1.
///
/// The reference's default for a comment escapes non-Latin-1 and nothing else, so
/// a tab or an accented letter is written as it stands.
fn push_comment_char(out: &mut String, c: char) {
    if (c as u32) <= 0xff {
        out.push(c);
    } else {
        push_unicode_escape(out, c);
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
}

impl Default for WriteOptions<'_> {
    fn default() -> Self {
        Self {
            separator: "=",
            comment: None,
            timestamp: None,
            sort_keys: false,
            ensure_ascii: true,
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
        out.push_str(&to_comment(comment));
        out.push('\n');
    }
    if let Some(seconds) = options.timestamp {
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

    /// Writes the document as ISO-8859-1 bytes, the way Java writes a stream.
    ///
    /// Every character in the result is ASCII, because anything else was
    /// escaped, so this is the same text as [`Properties::store`] encoded as
    /// bytes.
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
