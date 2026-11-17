//! Stylesheets: parsing rules and matching selectors.
//!
//! The supported selector grammar is a comma-separated list of complex
//! selectors. Each is a chain of compound selectors joined by combinators: a
//! descendant (a space) or a child (`>`), so `div p`, `div > p` and `div.card p`
//! all work. A compound selector is a tag name, a `#id`, a `.class`, the
//! universal `*`, or any combination, such as `div.card#main`. Pseudo-classes,
//! pseudo-elements, attribute selectors and at-rules are not implemented.
//!
//! The cascade orders by specificity, as the reference does: `div p` beats a
//! later `p`, because a tag-tag selector is more specific than a single tag. See
//! [`Selector::specificity`].

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

/// One compound selector: a tag, an `#id`, some `.class`es, or the universal
/// `*` when all three are absent.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Compound {
    /// The tag name, when the compound names one. `None` means universal.
    pub tag: Option<String>,
    /// The `#id`, when the compound names one.
    pub id: Option<String>,
    /// The `.class`es, all of which must match.
    pub classes: Vec<String>,
}

impl Compound {
    /// Does this compound match `element`, ignoring its ancestry?
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

    /// The bucket this compound files under, for the rule index.
    ///
    /// An element the selector matches has every key the rightmost compound
    /// names, so keying on the compound's tag, else its id, else its first
    /// class, else the universal bucket, finds every selector that could match
    /// and only those.
    fn key(&self) -> Bucket {
        if let Some(tag) = &self.tag {
            Bucket::Tag(tag.clone())
        } else if let Some(id) = &self.id {
            Bucket::Id(id.clone())
        } else if let Some(class) = self.classes.first() {
            Bucket::Class(class.clone())
        } else {
            Bucket::Universal
        }
    }
}

/// A combinator joining two compound selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Whitespace: the right compound is a descendant of the left.
    Descendant,
    /// `>`: the right compound is a child of the left.
    Child,
}

/// One complex selector: a chain of compound selectors joined by combinators,
/// such as `div.card > p`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    /// The compound selectors, left to right (ancestor first, subject last).
    pub compounds: Vec<Compound>,
    /// The combinator before each compound after the first, so this has one
    /// fewer entry than `compounds`.
    combinators: Vec<Combinator>,
}

impl Selector {
    /// The specificity `(ids, classes, tags)`, the reference's ordering key.
    ///
    /// A tag and the universal selector differ here: a tag counts as a type
    /// (the third field), the universal selector counts as nothing.
    pub fn specificity(&self) -> (u32, u32, u32) {
        let mut ids = 0;
        let mut classes = 0;
        let mut tags = 0;
        for compound in &self.compounds {
            if compound.id.is_some() {
                ids += 1;
            }
            classes += compound.classes.len() as u32;
            if compound.tag.is_some() {
                tags += 1;
            }
        }
        (ids, classes, tags)
    }

    /// Does this selector match `element`, whose ancestors are `ancestors`
    /// (root first, parent last)?
    ///
    /// The rightmost compound is the subject. The chain is matched right to
    /// left: a child combinator steps to the immediate parent, a descendant
    /// combinator to any ancestor. Backtracking is needed because a nearer
    /// ancestor may satisfy one step but leave no ancestor for the next, as in
    /// `div > p span` where the nearest `p` sits under no `div`.
    pub(crate) fn matches(&self, element: &Element, ancestors: &[&Element]) -> bool {
        let last = self
            .compounds
            .len()
            .checked_sub(1)
            .expect("a selector has at least one compound");
        self.compounds[last].matches(element) && self.match_chain(last, ancestors)
    }

    /// Match `compounds[..=index]`, given that `compounds[index]` matches the
    /// current element and `ancestors` are that element's ancestors.
    fn match_chain(&self, index: usize, ancestors: &[&Element]) -> bool {
        if index == 0 {
            return true;
        }
        let compound = &self.compounds[index - 1];
        match self.combinators[index - 1] {
            Combinator::Child => match ancestors.split_last() {
                None => false,
                Some((parent, rest)) => {
                    compound.matches(parent) && self.match_chain(index - 1, rest)
                }
            },
            Combinator::Descendant => {
                ancestors
                    .iter()
                    .enumerate()
                    .rev()
                    .any(|(position, ancestor)| {
                        compound.matches(ancestor)
                            && self.match_chain(index - 1, &ancestors[..position])
                    })
            }
        }
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

/// The single constraint a selector's rightmost compound names, used as its
/// index key.
enum Bucket {
    /// Names no tag, id or class; the universal selector.
    Universal,
    /// Names a tag, lower-cased.
    Tag(String),
    /// Names an `#id`.
    Id(String),
    /// Names a `.class`.
    Class(String),
}

/// Selectors bucketed by the constraint the subject compound requires.
///
/// A selector is filed under one key, taken from its rightmost compound: its
/// tag, or failing that its id, or failing that its first class, or the
/// universal bucket when it names none of the three. The subject is the element
/// the selector applies to, so any element the selector matches has that key;
/// looking the element's tag, id and classes up finds every selector that could
/// match it, and only those are tested in full. A pair is `(rule, selector)` so
/// the rule's declarations can be recovered without cloning the selector.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
struct RuleIndex {
    /// Selectors whose subject names no tag, id or class.
    universal: Vec<(usize, usize)>,
    /// Selectors keyed by their subject's tag, lower-cased.
    by_tag: HashMap<String, Vec<(usize, usize)>>,
    /// Selectors keyed by their subject's `#id`.
    by_id: HashMap<String, Vec<(usize, usize)>>,
    /// Selectors keyed by their subject's first `.class`.
    by_class: HashMap<String, Vec<(usize, usize)>>,
}

impl RuleIndex {
    fn build(rules: &[Rule]) -> Self {
        let mut index = Self::default();
        for (rule, spec) in rules.iter().enumerate() {
            for (selector, one) in spec.selectors.iter().enumerate() {
                let entry = (rule, selector);
                let subject = one
                    .compounds
                    .last()
                    .expect("a selector has at least one compound");
                match subject.key() {
                    Bucket::Tag(tag) => index.by_tag.entry(tag).or_default().push(entry),
                    Bucket::Id(id) => index.by_id.entry(id).or_default().push(entry),
                    Bucket::Class(class) => index.by_class.entry(class).or_default().push(entry),
                    Bucket::Universal => index.universal.push(entry),
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
    /// `ancestors` are the element's ancestors, root first, parent last.
    ///
    /// The reference orders the cascade by specificity first and source order
    /// second: a more specific selector wins however early it appears, and only
    /// equal specificity falls back to "later wins". Each declaration is
    /// therefore tagged with its selector's specificity and the rules are
    /// visited in ascending `(specificity, source order)`, so a later, more
    /// specific declaration overwrites an earlier one.
    pub(crate) fn declarations_for(
        &self,
        element: &Element,
        ancestors: &[&Element],
    ) -> Vec<&Declaration> {
        // Below the limit a scan touches no more rules than the index would
        // look up, so it skips the index bookkeeping that a small sheet cannot
        // pay for.
        if self.rules.len() <= SCAN_LIMIT {
            let mut matched: Vec<(u32, u32, u32, usize, &Declaration)> = Vec::new();
            for (order, rule) in self.rules.iter().enumerate() {
                for selector in &rule.selectors {
                    if selector.matches(element, ancestors) {
                        let (ids, classes, tags) = selector.specificity();
                        for declaration in &rule.declarations {
                            matched.push((ids, classes, tags, order, declaration));
                        }
                    }
                }
            }
            matched.sort_by_key(|&(ids, classes, tags, order, _)| (ids, classes, tags, order));
            return matched.into_iter().map(|(_, _, _, _, d)| d).collect();
        }
        let mut out = Vec::new();
        for &index in &self.candidates(element, ancestors) {
            out.extend(self.rules[index].declarations.iter());
        }
        out
    }

    /// The indices of the rules that match `element`, ascending by specificity
    /// then source order.
    ///
    /// The candidate selectors are the ones keyed by the element's tag, id or
    /// class, plus the universal ones; each is then tested in full, so a rule
    /// with several selectors matches if any of them does. A rule that matches
    /// through more than one selector appears once per selector, each with that
    /// selector's specificity; applying a rule's declarations twice is harmless,
    /// and the highest-specificity application lands last, which is the weight
    /// the reference gives a rule - the most specific of its matching selectors.
    fn candidates(&self, element: &Element, ancestors: &[&Element]) -> Vec<usize> {
        let mut out: Vec<(u32, u32, u32, usize)> = Vec::new();
        let test = |entries: &[(usize, usize)], out: &mut Vec<(u32, u32, u32, usize)>| {
            for &(rule, selector) in entries {
                let selector = &self.rules[rule].selectors[selector];
                if selector.matches(element, ancestors) {
                    let (ids, classes, tags) = selector.specificity();
                    out.push((ids, classes, tags, rule));
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
        out.into_iter().map(|entry| entry.3).collect()
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

/// Parse a comma-separated selector list into complex selectors.
fn parse_selectors(text: &str) -> Result<Vec<Selector>, Error> {
    let mut selectors = Vec::new();
    for part in text.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        selectors.push(parse_complex(part)?);
    }
    Ok(selectors)
}

/// Parse one complex selector: compounds joined by descendant or child
/// combinators, such as `div.card > p span`.
fn parse_complex(text: &str) -> Result<Selector, Error> {
    let mut compounds = Vec::new();
    let mut combinators = Vec::new();

    let (compound, mut rest) = parse_compound(text)?;
    compounds.push(compound);

    loop {
        let trimmed = rest.trim_start();
        let had_space = trimmed.len() != rest.len();
        if trimmed.is_empty() {
            break;
        }
        if let Some(after) = trimmed.strip_prefix('>') {
            combinators.push(Combinator::Child);
            rest = after.trim_start();
        } else if had_space {
            combinators.push(Combinator::Descendant);
            rest = trimmed;
        } else {
            return Err(Error::Malformed(format!("cannot read selector `{text}`")));
        }
        let (compound, tail) = parse_compound(rest)?;
        compounds.push(compound);
        rest = tail;
    }

    Ok(Selector {
        compounds,
        combinators,
    })
}

/// Characters that end a compound selector's name.
fn ends_name(character: char) -> bool {
    character == '#' || character == '.' || character == '>' || character.is_whitespace()
}

/// Whether `character` may appear in a tag name.
///
/// The HTML parser keeps a tag as it finds it, so a name it can carry - any
/// alphanumeric character, ASCII or not - must parse here too, or a rule for an
/// element the document can hold would be rejected. `-` and `_` are allowed
/// because custom elements and the common escaped names use them.
fn is_tag_character(character: char) -> bool {
    character.is_alphanumeric() || character == '-' || character == '_'
}

/// Parse one compound selector and return it with the text that follows it.
///
/// The grammar is an optional tag (or `*`) followed by any number of `#id` and
/// `.class` parts, so `div.card#main`, `.c`, `#x` and `*` are all compounds.
fn parse_compound(text: &str) -> Result<(Compound, &str), Error> {
    let mut compound = Compound::default();
    let end = text.find(ends_name).unwrap_or(text.len());
    let head = &text[..end];
    let mut rest = &text[end..];
    // Whether the compound named anything at all; `*` names nothing but is not
    // empty, so the two have to be told apart before the compound is accepted.
    let mut named = false;

    if !head.is_empty() {
        if head == "*" {
            named = true;
        } else if head.chars().all(is_tag_character) {
            compound.tag = Some(head.to_ascii_lowercase());
            named = true;
        } else {
            return Err(Error::Malformed(format!("cannot read selector `{text}`")));
        }
    }

    while let Some(first) = rest.chars().next() {
        if first != '#' && first != '.' {
            break;
        }
        let tail = &rest[1..];
        let end = tail.find(ends_name).unwrap_or(tail.len());
        let name = &tail[..end];
        if name.is_empty() {
            return Err(Error::Malformed(format!("empty name in selector `{text}`")));
        }
        if first == '#' {
            if compound.id.is_some() {
                return Err(Error::Malformed(format!(
                    "more than one id in selector `{text}`"
                )));
            }
            compound.id = Some(name.to_owned());
        } else {
            compound.classes.push(name.to_owned());
        }
        named = true;
        rest = &tail[end..];
    }

    if !named {
        return Err(Error::Malformed(format!("cannot read selector `{text}`")));
    }
    Ok((compound, rest))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::Node;

    fn element(tag: &str) -> Element {
        Element {
            tag: tag.to_owned(),
            id: None,
            classes: Vec::new(),
            style: None,
            children: Vec::new(),
        }
    }

    fn sheet(selector: &str) -> Selector {
        let parsed = Stylesheet::parse(&format!("{selector} {{ width: 1px }}")).unwrap();
        parsed.rules[0].selectors[0].clone()
    }

    #[test]
    fn a_descendant_selector_parses_into_a_chain() {
        let selector = sheet("div p");
        assert_eq!(selector.compounds.len(), 2);
        assert_eq!(selector.combinators, vec![Combinator::Descendant]);
        assert_eq!(selector.compounds[0].tag.as_deref(), Some("div"));
        assert_eq!(selector.compounds[1].tag.as_deref(), Some("p"));
    }

    #[test]
    fn a_child_combinator_parses_with_or_without_spaces() {
        for text in ["div > p", "div>p"] {
            let selector = sheet(text);
            assert_eq!(selector.compounds.len(), 2, "{text}");
            assert_eq!(selector.combinators, vec![Combinator::Child], "{text}");
        }
    }

    #[test]
    fn a_compound_selector_keeps_its_tag_id_and_classes() {
        let selector = sheet("div.card#main");
        assert_eq!(selector.compounds.len(), 1);
        let compound = &selector.compounds[0];
        assert_eq!(compound.tag.as_deref(), Some("div"));
        assert_eq!(compound.id.as_deref(), Some("main"));
        assert_eq!(compound.classes, vec!["card".to_owned()]);
    }

    #[test]
    fn the_universal_selector_is_one_compound_naming_nothing() {
        let selector = sheet("*");
        assert_eq!(selector.compounds.len(), 1);
        let compound = &selector.compounds[0];
        assert!(compound.tag.is_none());
        assert!(compound.id.is_none());
        assert!(compound.classes.is_empty());
        assert_eq!(selector.specificity(), (0, 0, 0));
    }

    #[test]
    fn specificity_counts_ids_classes_and_tags() {
        assert_eq!(sheet("div p").specificity(), (0, 0, 2));
        assert_eq!(sheet(".c").specificity(), (0, 1, 0));
        assert_eq!(sheet("#x").specificity(), (1, 0, 0));
        assert_eq!(sheet("div.card#main p").specificity(), (1, 1, 2));
        // The universal selector contributes nothing to any field.
        assert_eq!(sheet("div *").specificity(), (0, 0, 1));
    }

    #[test]
    fn a_descendant_selector_matches_any_ancestor() {
        let selector = sheet("div p");
        let div = element("div");
        let section = element("section");
        let p = element("p");

        // p directly under div.
        assert!(selector.matches(&p, &[&div]));
        // p under a section under the div.
        assert!(selector.matches(&p, &[&div, &section]));
        // p with no div above it.
        assert!(!selector.matches(&p, &[&section]));
        // The subject must be a p.
        assert!(!selector.matches(&div, &[&section]));
    }

    #[test]
    fn a_child_selector_needs_the_immediate_parent() {
        let selector = sheet("div > p");
        let div = element("div");
        let section = element("section");
        let p = element("p");

        assert!(selector.matches(&p, &[&div]));
        // The div is a grandparent, not the parent, so the child combinator fails.
        assert!(!selector.matches(&p, &[&div, &section]));
    }

    #[test]
    fn a_chain_backtracks_to_an_ancestor_that_satisfies_every_step() {
        // `div > p span`: the nearest `p` above the span sits under no `div`, so
        // the child step must backtrack to the p that does.
        let selector = sheet("div > p span");
        let div = element("div");
        let mut outer_p = element("p");
        let mut inner_p = element("p");
        let section = element("section");
        let span = element("span");

        outer_p.children.push(Node::Element(section.clone()));
        inner_p.children.push(Node::Element(span.clone()));
        // span under inner_p under section under outer_p under div.
        assert!(selector.matches(&span, &[&div, &outer_p, &section, &inner_p]));
        // Same span with no div above: the child step has nothing to backtrack to.
        assert!(!selector.matches(&span, &[&outer_p, &section, &inner_p]));
    }

    #[test]
    fn a_malformed_selector_is_an_error() {
        for text in ["div >", "> p", "..a", "div..a", "div >> p"] {
            let source = format!("{text} {{ width: 1px }}");
            assert!(
                Stylesheet::parse(&source).is_err(),
                "`{text}` should not parse"
            );
        }
    }

    #[test]
    fn a_class_after_a_combinator_is_read() {
        let selector = sheet("div > p.c");
        assert_eq!(selector.compounds.len(), 2);
        assert_eq!(selector.compounds[1].tag.as_deref(), Some("p"));
        assert_eq!(selector.compounds[1].classes, vec!["c".to_owned()]);
    }

    #[test]
    fn a_non_ascii_tag_name_is_accepted() {
        // The HTML parser keeps the tag as written, so a rule for an element it
        // can carry has to parse. The tag is lowercased like an ASCII one.
        let selector = sheet("café");
        assert_eq!(selector.compounds[0].tag.as_deref(), Some("café"));
        assert_eq!(sheet("DIV").compounds[0].tag.as_deref(), Some("div"));
        // A character no tag name can hold is still rejected.
        assert!(Stylesheet::parse("@media { width: 1px }").is_err());
    }

    #[test]
    fn the_rule_index_agrees_with_the_scan() {
        // Past SCAN_LIMIT the lookup goes through the index instead of a scan,
        // so the two paths must pick the same winning declaration. The sheet is
        // padded with rules that match nothing to cross the limit, and the one
        // rule that matters is more specific than a later tag rule.
        let mut source = String::from("div p { width: 60px }\np { width: 30px }\n");
        for index in 0..70 {
            source.push_str(&format!(".filler{index} {{ width: 1px }}\n"));
        }
        let stylesheet = Stylesheet::parse(&source).unwrap();
        assert!(stylesheet.rules.len() > SCAN_LIMIT);

        let div = element("div");
        let p = element("p");
        let declarations = stylesheet.declarations_for(&p, &[&div]);
        // The last matching declaration wins, and `div p` is more specific than
        // `p`, so it must land after it despite coming first.
        assert_eq!(declarations.last().unwrap().value, "60px");

        // With no div above it only `p` matches.
        let declarations = stylesheet.declarations_for(&p, &[]);
        assert_eq!(declarations.last().unwrap().value, "30px");
    }
}
