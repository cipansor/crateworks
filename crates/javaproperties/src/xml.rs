//! The XML properties format, `java.util.Properties.loadFromXML` and
//! `storeToXML`.
//!
//! Java writes a second form of the same key/value data as an XML document, and
//! reads it back. It is used where a `.properties` file has to travel as XML,
//! such as a configuration fragment inside a larger document or an attachment.
//! The escapes are XML's own rather than the backslash ones, so this is a
//! separate module from [`crate::escape`].
//!
//! The document is fixed in shape:
//!
//! ```xml
//! <!DOCTYPE properties SYSTEM "http://java.sun.com/dtd/properties.dtd">
//! <properties>
//! <comment>an optional comment</comment>
//! <entry key="name">cipansor</entry>
//! </properties>
//! ```
//!
//! Writing a value escapes `&`, `<` and `>`; a key is quoted with single or
//! double quotes, whichever the key itself does not contain, and a tab, newline
//! or carriage return in a key becomes a numeric reference. Reading is lenient
//! in the same way the reference is: only the root name and the presence of a
//! `key` attribute are checked.

use crate::escape::Error;
use crate::reader::Properties;

/// The doctype line every document carries, whether or not it is used.
const DOCTYPE: &str = r#"<!DOCTYPE properties SYSTEM "http://java.sun.com/dtd/properties.dtd">"#;

/// Reads an XML properties document.
///
/// The root element must be `<properties>`, and every `<entry>` must carry a
/// `key` attribute; anything else is an error. Duplicate keys are kept in order.
///
/// # Errors
///
/// Returns [`Error::MalformedXml`] when the text is not well-formed XML,
/// [`Error::NotPropertiesRoot`] when the root is not `<properties>`, and
/// [`Error::MissingEntryKey`] when an `<entry>` has no `key` attribute.
///
/// # Example
///
/// ```
/// use javaproperties::loads_xml;
///
/// let entries = loads_xml(
///     r#"<properties><entry key="name">cipansor</entry></properties>"#,
/// )
/// .unwrap();
/// assert_eq!(entries, vec![("name".to_string(), "cipansor".to_string())]);
/// ```
pub fn loads_xml(text: &str) -> Result<Vec<(String, String)>, Error> {
    let root = xmlkit::parse(text).map_err(|_| Error::MalformedXml)?;
    if root.name() != "properties" {
        return Err(Error::NotPropertiesRoot);
    }
    let mut pairs = Vec::new();
    for entry in root.children_named("entry") {
        let key = entry.attr("key").ok_or(Error::MissingEntryKey)?;
        pairs.push((key.to_string(), entry.text()));
    }
    Ok(pairs)
}

/// Writes key/value pairs as an XML properties document.
///
/// A comment, when given, is written as a `<comment>` element before the
/// entries. When `sort_keys` is true the entries are ordered by key and then by
/// value, which is what the reference does for an iterable of pairs.
///
/// # Example
///
/// ```
/// use javaproperties::dumps_xml;
///
/// let xml = dumps_xml(&[("name".to_string(), "cipansor".to_string())], None, false);
/// assert!(xml.contains(r#"<entry key="name">cipansor</entry>"#));
/// ```
#[must_use]
pub fn dumps_xml(pairs: &[(String, String)], comment: Option<&str>, sort_keys: bool) -> String {
    let mut out = String::new();
    out.push_str(DOCTYPE);
    out.push('\n');
    out.push_str("<properties>\n");
    if let Some(comment) = comment {
        out.push_str("<comment>");
        out.push_str(&escape_text(comment));
        out.push_str("</comment>\n");
    }

    let mut ordered: Vec<&(String, String)> = pairs.iter().collect();
    if sort_keys {
        ordered.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
    }
    for (key, value) in ordered {
        out.push_str("<entry key=");
        out.push_str(&quote_attr(key));
        out.push('>');
        out.push_str(&escape_text(value));
        out.push_str("</entry>\n");
    }

    out.push_str("</properties>\n");
    out
}

impl Properties {
    /// Reads an XML properties document.
    ///
    /// # Errors
    ///
    /// The same conditions as [`loads_xml`].
    pub fn from_xml(text: &str) -> Result<Self, Error> {
        Ok(Self::from_pairs(loads_xml(text)?))
    }

    /// Writes the entries as an XML properties document.
    #[must_use]
    pub fn to_xml(&self, comment: Option<&str>, sort_keys: bool) -> String {
        dumps_xml(&self.pairs, comment, sort_keys)
    }
}

/// Escapes `&`, `<` and `>` for element text.
///
/// The reference's `xml.sax.saxutils.escape`, which does not touch quotes: they
/// are legal in text.
fn escape_text(s: &str) -> String {
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

/// Quotes an attribute value the way `xml.sax.saxutils.quoteattr` does.
///
/// Whitespace that XML would otherwise normalise away is written as a numeric
/// reference, and the quoting character is the one the value does not contain,
/// so the value never needs an escape that a reader could undo differently.
fn quote_attr(s: &str) -> String {
    let mut escaped = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '\n' => escaped.push_str("&#10;"),
            '\r' => escaped.push_str("&#13;"),
            '\t' => escaped.push_str("&#9;"),
            _ => escaped.push(c),
        }
    }

    if escaped.contains('"') {
        if escaped.contains('\'') {
            format!("\"{}\"", escaped.replace('"', "&quot;"))
        } else {
            format!("'{escaped}'")
        }
    } else {
        format!("\"{escaped}\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(entries: &[(&str, &str)]) -> Vec<(String, String)> {
        entries
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect()
    }

    #[test]
    fn an_empty_document_is_just_the_wrapper() {
        assert_eq!(
            dumps_xml(&[], None, false),
            format!("{DOCTYPE}\n<properties>\n</properties>\n")
        );
    }

    #[test]
    fn a_comment_is_written_before_the_entries() {
        assert_eq!(
            dumps_xml(&pairs(&[("a", "1")]), Some("hi"), false),
            format!(
                "{DOCTYPE}\n<properties>\n<comment>hi</comment>\n<entry key=\"a\">1</entry>\n</properties>\n"
            )
        );
    }

    #[test]
    fn the_three_text_entities_are_escaped_in_a_value() {
        let xml = dumps_xml(&pairs(&[("k", "a&b<c>d")]), None, false);
        assert!(xml.contains(r#"<entry key="k">a&amp;b&lt;c&gt;d</entry>"#));
    }

    #[test]
    fn a_key_is_quoted_with_the_other_quote_when_it_holds_one() {
        assert!(dumps_xml(&pairs(&[("k\"q", "v")]), None, false).contains("<entry key='k\"q'>"));
        assert!(dumps_xml(&pairs(&[("k'q", "v")]), None, false).contains("<entry key=\"k'q\">"));
    }

    #[test]
    fn a_key_holding_both_quotes_escapes_the_double() {
        let xml = dumps_xml(&pairs(&[("both\"'x", "v")]), None, false);
        assert!(xml.contains(r#"<entry key="both&quot;'x">"#));
    }

    #[test]
    fn whitespace_in_a_key_becomes_a_numeric_reference() {
        let xml = dumps_xml(&pairs(&[("a\tb\nc", "v")]), None, false);
        assert!(xml.contains(r#"<entry key="a&#9;b&#10;c">"#));
    }

    #[test]
    fn sorting_orders_by_key_then_value() {
        let xml = dumps_xml(&pairs(&[("b", "2"), ("a", "2"), ("a", "1")]), None, true);
        let a1 = xml.find("key=\"a\">1").unwrap();
        let a2 = xml.find("key=\"a\">2").unwrap();
        let b = xml.find("key=\"b\"").unwrap();
        assert!(a1 < a2 && a2 < b);
    }

    #[test]
    fn reading_is_the_inverse_of_writing() {
        let original = pairs(&[
            ("name", "cipansor"),
            ("caf\u{00e9}", "a&b<c>d"),
            ("k\"q", "x"),
            ("tab\tkey", "y"),
        ]);
        let xml = dumps_xml(&original, Some("note"), true);
        let mut expected = original.clone();
        expected.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1.cmp(&b.1)));
        assert_eq!(loads_xml(&xml).unwrap(), expected);
    }

    #[test]
    fn a_document_without_the_doctype_still_reads() {
        let entries = loads_xml(r#"<properties><entry key="a">1</entry></properties>"#).unwrap();
        assert_eq!(entries, pairs(&[("a", "1")]));
    }

    #[test]
    fn a_wrong_root_is_rejected() {
        assert_eq!(
            loads_xml("<config><entry key=\"a\">1</entry></config>").unwrap_err(),
            Error::NotPropertiesRoot
        );
    }

    #[test]
    fn an_entry_without_a_key_is_rejected() {
        assert_eq!(
            loads_xml("<properties><entry>1</entry></properties>").unwrap_err(),
            Error::MissingEntryKey
        );
    }

    #[test]
    fn malformed_xml_is_rejected() {
        assert_eq!(
            loads_xml("<properties><entry key=\"a\">1</properties>").unwrap_err(),
            Error::MalformedXml
        );
    }

    #[test]
    fn duplicate_keys_are_kept_in_order() {
        let entries = loads_xml(
            r#"<properties><entry key="a">1</entry><entry key="a">2</entry></properties>"#,
        )
        .unwrap();
        assert_eq!(entries, pairs(&[("a", "1"), ("a", "2")]));
    }

    #[test]
    fn a_numeric_reference_is_decoded_on_read() {
        let entries =
            loads_xml(r#"<properties><entry key="k">a&#38;b&#x3C;c</entry></properties>"#).unwrap();
        assert_eq!(entries, pairs(&[("k", "a&b<c")]));
    }
}
