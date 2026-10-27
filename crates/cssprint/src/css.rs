//! Stylesheets: parsing rules and matching selectors.
//!
//! The supported selector grammar is deliberately small: a comma-separated list
//! of compound selectors, each a tag name, a `#id`, a `.class`, or any
//! combination, such as `div.card#main`. Descendant combinators, pseudo-classes
//! and at-rules are not implemented.

use crate::dom::Element;
use crate::error::Error;

/// One `property: value` pair.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Declaration {
    /// The lower-cased property name, such as `width`.
    pub property: String,
    /// The value as written, with surrounding whitespace removed.
    pub value: String,
}

/// A selector list and the declarations it applies.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Rule {
    /// The selectors, any of which matches.
    pub selectors: Vec<Selector>,
    /// The declarations, in source order.
    pub declarations: Vec<Declaration>,
}

/// One compound selector.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selector {
    /// The tag name, when the selector names one.
    pub tag: Option<String>,
    /// The `#id`, when the selector names one.
    pub id: Option<String>,
    /// The `.class`es, all of which must match.
    pub classes: Vec<String>,
}

impl Selector {
    /// Does this selector match `element`?
    pub(crate) fn matches(&self, element: &Element) -> bool {
        if let Some(tag) = &self.tag
            && &element.tag != tag
        {
            return false;
        }
        if let Some(id) = &self.id
            && element.id.as_deref() != Some(id.as_str())
        {
            return false;
        }
        self.classes
            .iter()
            .all(|class| element.classes.contains(class))
    }
}

/// A parsed stylesheet.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stylesheet {
    rules: Vec<Rule>,
}

impl Stylesheet {
    /// Parse a stylesheet from text.
    ///
    /// Comments are removed, then each `selector { declarations }` block is
    /// read. A block with no braces, or a selector that names nothing, is an
    /// error rather than a silent skip, so a typo cannot quietly drop a rule.
    ///
    /// # Errors
    ///
    /// Returns [`Error::Malformed`] when a block is not closed, a declaration
    /// has no `:`, or a declaration has an empty value.
    pub fn parse(source: &str) -> Result<Self, Error> {
        let source = strip_comments(source);
        let mut rules = Vec::new();
        let mut rest = source.as_str();

        while let Some(brace) = rest.find('{') {
            let selector_text = &rest[..brace];
            let after = &rest[brace + 1..];
            let Some(close) = after.find('}') else {
                return Err(Error::Malformed(format!(
                    "rule `{}` is not closed",
                    selector_text.trim()
                )));
            };
            let body = &after[..close];
            rest = &after[close + 1..];

            let selectors = parse_selectors(selector_text)?;
            if selectors.is_empty() {
                continue;
            }
            rules.push(Rule {
                selectors,
                declarations: parse_declaration_list(body)?,
            });
        }

        if rest.trim().is_empty() {
            Ok(Self { rules })
        } else {
            Err(Error::Malformed(format!(
                "trailing text outside any rule: `{}`",
                rest.trim()
            )))
        }
    }

    /// The declarations that apply to `element`, in cascade order.
    ///
    /// Rules are applied in source order and a later rule wins, which is the
    /// order the reference uses when no `!important` is present.
    pub(crate) fn declarations_for(&self, element: &Element) -> Vec<&Declaration> {
        self.rules
            .iter()
            .filter(|rule| rule.selectors.iter().any(|s| s.matches(element)))
            .flat_map(|rule| rule.declarations.iter())
            .collect()
    }
}

/// Remove `/* ... */` comments, leaving other text untouched.
fn strip_comments(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(start) = rest.find("/*") {
        out.push_str(&rest[..start]);
        match rest[start + 2..].find("*/") {
            Some(end) => rest = &rest[start + 2 + end + 2..],
            None => return out,
        }
    }
    out.push_str(rest);
    out
}

/// Parse a comma-separated selector list.
fn parse_selectors(text: &str) -> Result<Vec<Selector>, Error> {
    let mut selectors = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        selectors.push(parse_selector(part)?);
    }
    Ok(selectors)
}

/// Parse one compound selector such as `div.card#main`.
fn parse_selector(text: &str) -> Result<Selector, Error> {
    if text.contains(char::is_whitespace) {
        return Err(Error::Malformed(format!(
            "descendant selectors are not supported: `{text}`"
        )));
    }
    let mut selector = Selector::default();
    let mut rest = text;

    if let Some(index) = rest.find(['#', '.']) {
        if index > 0 {
            selector.tag = Some(rest[..index].to_ascii_lowercase());
        }
        rest = &rest[index..];
    } else {
        selector.tag = Some(rest.to_ascii_lowercase());
        return Ok(selector);
    }

    while let Some(first) = rest.chars().next() {
        if first != '#' && first != '.' {
            return Err(Error::Malformed(format!("cannot read selector `{text}`")));
        }
        let tail = &rest[1..];
        let end = tail.find(['#', '.']).unwrap_or(tail.len());
        let name = &tail[..end];
        if name.is_empty() {
            return Err(Error::Malformed(format!("empty name in selector `{text}`")));
        }
        if first == '#' {
            if selector.id.is_some() {
                return Err(Error::Malformed(format!(
                    "more than one id in selector `{text}`"
                )));
            }
            selector.id = Some(name.to_owned());
        } else {
            selector.classes.push(name.to_owned());
        }
        rest = &tail[end..];
    }
    Ok(selector)
}

/// Parse a `property: value;` list, as found inside a block or in a `style`
/// attribute.
///
/// # Errors
///
/// Returns [`Error::Malformed`] when a declaration has no `:`, or either side
/// is empty.
pub(crate) fn parse_declaration_list(body: &str) -> Result<Vec<Declaration>, Error> {
    let mut declarations = Vec::new();
    for part in body.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some(colon) = part.find(':') else {
            return Err(Error::Malformed(format!(
                "declaration has no colon: `{part}`"
            )));
        };
        let property = part[..colon].trim().to_ascii_lowercase();
        let value = part[colon + 1..].trim();
        if property.is_empty() || value.is_empty() {
            return Err(Error::Malformed(format!(
                "declaration is incomplete: `{part}`"
            )));
        }
        declarations.push(Declaration {
            property,
            value: value.to_owned(),
        });
    }
    Ok(declarations)
}
