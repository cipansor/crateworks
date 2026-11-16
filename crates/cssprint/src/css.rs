//! Stylesheets: parsing rules and matching selectors.
//!
//! The supported selector grammar is deliberately small: a comma-separated list
//! of compound selectors, each a tag name, a `#id`, a `.class`, or any
//! combination, such as `div.card#main`. Descendant combinators, pseudo-classes
//! and at-rules are not implemented.

use std::collections::HashMap;

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

/// A parsed stylesheet, with an index of the rules by the tag, id and class
/// each of them names.
///
/// Matching every rule against every element is the layout's hot loop, and
/// testing M rules against N elements costs M*N. The index narrows a lookup to
/// the rules that can possibly match, so a document whose elements carry a
/// class or id does not walk the whole stylesheet per element.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Stylesheet {
    rules: Vec<Rule>,
    index: RuleIndex,
}

/// The number of rules below which a plain scan is used instead of the index.
///
/// The index allocates a small result vector per element; a sheet this small
/// tests fewer rules than that costs, so the scan is both simpler and faster.
const SCAN_LIMIT: usize = 64;

/// Selectors bucketed by one constraint each of them requires.
///
/// Each selector is filed under a single key: its tag, or failing that its id,
/// or failing that its first class, or the universal bucket when it names none
/// of the three. Any element the selector matches has that key, so looking the
/// element's tag, id and classes up finds every selector that could match it,
/// and only those are tested for real. A pair is `(rule, selector)` so the
/// rule's declarations can be recovered without cloning the selector.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct RuleIndex {
    /// Selectors that name no tag, id or class.
    universal: Vec<(usize, usize)>,
    /// Selectors keyed by their tag, lower-cased.
    by_tag: HashMap<String, Vec<(usize, usize)>>,
    /// Selectors keyed by their `#id`.
    by_id: HashMap<String, Vec<(usize, usize)>>,
    /// Selectors keyed by their first `.class`.
    by_class: HashMap<String, Vec<(usize, usize)>>,
}

impl RuleIndex {
    fn build(rules: &[Rule]) -> Self {
        let mut index = Self::default();
        for (rule, spec) in rules.iter().enumerate() {
            for (selector, one) in spec.selectors.iter().enumerate() {
                let entry = (rule, selector);
                if let Some(tag) = &one.tag {
                    index.by_tag.entry(tag.clone()).or_default().push(entry);
                } else if let Some(id) = &one.id {
                    index.by_id.entry(id.clone()).or_default().push(entry);
                } else if let Some(class) = one.classes.first() {
                    index.by_class.entry(class.clone()).or_default().push(entry);
                } else {
                    index.universal.push(entry);
                }
            }
        }
        index
    }
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
            Ok(Self {
                index: RuleIndex::build(&rules),
                rules,
            })
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
        // Below the limit a scan touches no more rules than the index would
        // look up, so it skips the index bookkeeping that a small sheet cannot
        // pay for.
        if self.rules.len() <= SCAN_LIMIT {
            return self
                .rules
                .iter()
                .filter(|rule| rule.selectors.iter().any(|s| s.matches(element)))
                .flat_map(|rule| rule.declarations.iter())
                .collect();
        }
        let mut out = Vec::new();
        for &index in &self.candidates(element) {
            out.extend(self.rules[index].declarations.iter());
        }
        out
    }

    /// The indices of the rules that match `element`, ascending.
    ///
    /// The candidate selectors are the ones keyed by the element's tag, id or
    /// class, plus the universal ones; each is then tested in full, so a rule
    /// with several selectors matches if any of them does.
    fn candidates(&self, element: &Element) -> Vec<usize> {
        let mut out = Vec::new();
        let test = |entries: &[(usize, usize)], out: &mut Vec<usize>| {
            for &(rule, selector) in entries {
                if self.rules[rule].selectors[selector].matches(element) {
                    out.push(rule);
                }
            }
        };
        test(&self.index.universal, &mut out);
        if let Some(bucket) = self.index.by_tag.get(&element.tag) {
            test(bucket, &mut out);
        }
        if let Some(id) = &element.id
            && let Some(bucket) = self.index.by_id.get(id)
        {
            test(bucket, &mut out);
        }
        for class in &element.classes {
            if let Some(bucket) = self.index.by_class.get(class) {
                test(bucket, &mut out);
            }
        }
        out.sort_unstable();
        out.dedup();
        out
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
