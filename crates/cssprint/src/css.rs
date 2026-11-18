//! Stylesheets: parsing rules and matching selectors.
//!
//! The supported selector grammar is a comma-separated list of complex
//! selectors. Each is a chain of compound selectors joined by combinators: a
//! descendant (a space), a child (`>`), an adjacent sibling (`+`) or a general
//! sibling (`~`), so `div p`, `div > p`, `h1 + p` and `h1 ~ p` all work. A
//! compound selector is a tag name, a `#id`, a `.class`, an attribute selector,
//! a structural pseudo-class, the universal `*`, or any combination, such as
//! `div.card#main[href]`.
//!
//! The attribute selectors `[a]`, `[a=v]`, `[a~=v]`, `[a|=v]`, `[a^=v]`,
//! `[a$=v]` and `[a*=v]` are supported, as are the structural pseudo-classes
//! `:root`, `:first-child`, `:last-child`, `:only-child`, `:nth-child()` and
//! `:nth-last-child()`, and the negation `:not()` over a compound. Other
//! pseudo-classes, pseudo-elements and at-rules are not implemented.
//!
//! The cascade orders by specificity, as the reference does: `div p` beats a
//! later `p`, because a tag-tag selector is more specific than a single tag. See
//! [`Selector::specificity`].

use std::collections::HashMap;

use crate::dom::{Element, Node};
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

/// One compound selector: a tag, an `#id`, some `.class`es, attribute
/// selectors, structural pseudo-classes and an optional `:not()`, any
/// combination of which must match.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Compound {
    /// The tag name, when the compound names one. `None` means universal.
    pub tag: Option<String>,
    /// The `#id`, when the compound names one.
    pub id: Option<String>,
    /// The `.class`es, all of which must match.
    pub classes: Vec<String>,
    /// The attribute selectors, all of which must match.
    pub attributes: Vec<AttributeSelector>,
    /// The structural pseudo-classes, all of which must match.
    pub pseudo_classes: Vec<PseudoClass>,
    /// A `:not(...)` argument, which must not match. The reference's
    /// level-3 `:not()` takes a single simple selector; this holds the one
    /// compound it names.
    pub negation: Option<Box<Compound>>,
}

/// An attribute selector: an attribute name and a test on its value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AttributeSelector {
    /// The attribute name, lower-cased as the HTML parser lower-cases it.
    pub name: String,
    /// The test, or `None` for a bare `[name]`, which only asks that the
    /// attribute be present.
    pub test: Option<(AttributeMatch, String)>,
}

/// The operator of an attribute selector's value test.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttributeMatch {
    /// `=`: the value is exactly this.
    Equals,
    /// `~=`: one whitespace-separated word of the value is exactly this.
    Includes,
    /// `|=`: the value is this, or this followed by `-`.
    DashMatch,
    /// `^=`: the value starts with this.
    Prefix,
    /// `$=`: the value ends with this.
    Suffix,
    /// `*=`: the value contains this.
    Substring,
}

/// A structural pseudo-class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PseudoClass {
    /// `:root`, the document element.
    Root,
    /// `:first-child`.
    FirstChild,
    /// `:last-child`.
    LastChild,
    /// `:only-child`.
    OnlyChild,
    /// `:nth-child(an+b)`.
    NthChild(Nth),
    /// `:nth-last-child(an+b)`.
    NthLastChild(Nth),
}

/// The `an+b` of an `:nth-*()` pseudo-class.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Nth {
    /// The `a` coefficient.
    pub a: i64,
    /// The `b` constant.
    pub b: i64,
}

impl Nth {
    /// Does a 1-based index satisfy `an + b` for some non-negative integer `n`?
    fn matches(self, index: i64) -> bool {
        // an + b = index has a non-negative integer solution n. With a == 0 the
        // test is a plain equality; otherwise the division must be exact and
        // the quotient non-negative.
        if self.a == 0 {
            return index == self.b;
        }
        let delta = index - self.b;
        delta % self.a == 0 && delta / self.a >= 0
    }
}

impl Compound {
    /// The specificity this compound contributes, `(ids, classes, tags)`.
    ///
    /// A tag counts as a type; the universal selector counts as nothing. An
    /// attribute selector and a structural pseudo-class each count as a class.
    /// `:not()` counts as its argument and nothing for itself, as the level-3
    /// reference does.
    fn specificity(&self) -> (u32, u32, u32) {
        let mut ids = u32::from(self.id.is_some());
        let mut classes = self.classes.len() as u32
            + self.attributes.len() as u32
            + self.pseudo_classes.len() as u32;
        let mut tags = u32::from(self.tag.is_some());
        if let Some(negation) = &self.negation {
            let (id, class, tag) = negation.specificity();
            ids += id;
            classes += class;
            tags += tag;
        }
        (ids, classes, tags)
    }

    /// Does this compound match `element`, ignoring its ancestry and its
    /// structural pseudo-classes?
    ///
    /// This is the part answerable from the element alone: its tag, id, class
    /// and attributes. [`Self::matches`] adds the structural pseudo-classes and
    /// the `:not()`, which need the sibling context.
    pub(crate) fn matches_simple(&self, element: &Element) -> bool {
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
        if !self
            .classes
            .iter()
            .all(|class| element.classes.contains(class))
        {
            return false;
        }
        self.attributes
            .iter()
            .all(|attribute| attribute.matches(element))
    }

    /// Does this compound match `element` given its siblings and ancestry?
    ///
    /// The structural pseudo-classes and a `:not()` argument read the sibling
    /// position, which is why they need `context`.
    pub(crate) fn matches(&self, element: &Element, context: &MatchContext<'_>) -> bool {
        self.matches_simple(element)
            && self
                .pseudo_classes
                .iter()
                .all(|pseudo| pseudo.matches(context))
            && self
                .negation
                .as_deref()
                .is_none_or(|negation| !negation.matches(element, context))
    }

    /// The bucket this compound files under, for the rule index.
    ///
    /// An element the selector matches has every key the rightmost compound
    /// names, so keying on the compound's tag, else its id, else its first
    /// class, else the universal bucket, finds every selector that could match
    /// and only those. An attribute or pseudo-class constraint is not a bucket
    /// key, so a compound that names only those files under the universal
    /// bucket and is tested in full.
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

impl AttributeSelector {
    /// Does `element` carry an attribute satisfying this selector?
    fn matches(&self, element: &Element) -> bool {
        let Some((_, value)) = element
            .attributes
            .iter()
            .find(|(name, _)| name == &self.name)
        else {
            return false;
        };
        let Some((operator, wanted)) = &self.test else {
            // A bare `[name]` asks only for presence, which the `find` proved.
            return true;
        };
        // A valueless attribute matches `[name=""]` and `[name^=""]` but not
        // `[name="x"]`; treat its value as the empty string.
        let value = value.as_deref().unwrap_or("");
        match operator {
            AttributeMatch::Equals => value == wanted,
            AttributeMatch::Includes => {
                !wanted.is_empty() && value.split_whitespace().any(|word| word == wanted)
            }
            AttributeMatch::DashMatch => {
                value == wanted
                    || value
                        .strip_prefix(wanted)
                        .is_some_and(|rest| rest.starts_with('-'))
            }
            AttributeMatch::Prefix => !wanted.is_empty() && value.starts_with(wanted),
            AttributeMatch::Suffix => !wanted.is_empty() && value.ends_with(wanted),
            AttributeMatch::Substring => !wanted.is_empty() && value.contains(wanted),
        }
    }
}

impl PseudoClass {
    /// Does an element satisfy this pseudo-class in `context`?
    fn matches(&self, context: &MatchContext<'_>) -> bool {
        let index = context.index;
        let total = context.total;
        let position = (index + 1) as i64;
        let from_end = (total - index) as i64;
        match self {
            // `:root` is the element with no parent; the crate's document root
            // is `html`, which the reference makes the root too.
            Self::Root => context.parent.is_none(),
            Self::FirstChild => index == 0,
            Self::LastChild => index + 1 == total,
            Self::OnlyChild => total == 1,
            Self::NthChild(nth) => nth.matches(position),
            Self::NthLastChild(nth) => nth.matches(from_end),
        }
    }
}

/// The context a compound is matched in, beyond the element itself.
///
/// A compound that names no structural pseudo-class does not need it, but the
/// structural pseudo-classes read the element's position among its siblings,
/// which is not in the element, and a sibling combinator steps to a sibling
/// that shares the same context.
#[derive(Clone, Copy)]
pub struct MatchContext<'a> {
    /// The elements from the root down to the element's parent, root first.
    pub ancestors: &'a [&'a Element],
    /// The element's parent, when it has one.
    pub parent: Option<&'a Element>,
    /// The element's position among its parent's element children, 0-based.
    pub index: usize,
    /// How many element children the parent has.
    pub total: usize,
}

impl<'a> MatchContext<'a> {
    /// The context of `element`, whose ancestors are `ancestors` (root first).
    fn of(element: &'a Element, ancestors: &'a [&'a Element]) -> Self {
        let parent = ancestors.last().copied();
        let total = parent.map_or(1, |parent| element_children(parent).count());
        let index = parent.map_or(0, |parent| {
            element_children(parent)
                .position(|child| std::ptr::eq(child, element))
                .unwrap_or(0)
        });
        Self {
            ancestors,
            parent,
            index,
            total,
        }
    }

    /// The element children of `element`.
    fn siblings(self) -> impl Iterator<Item = &'a Element> {
        self.parent.into_iter().flat_map(element_children)
    }
}

/// The element children of `element`, in document order.
fn element_children(element: &Element) -> impl Iterator<Item = &Element> {
    element.children.iter().filter_map(|node| match node {
        Node::Element(child) => Some(child),
        Node::Text(_) => None,
    })
}

/// A combinator joining two compound selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Combinator {
    /// Whitespace: the right compound is a descendant of the left.
    Descendant,
    /// `>`: the right compound is a child of the left.
    Child,
    /// `+`: the right compound is the immediately following sibling of the
    /// left, so the left compound matches the sibling just before the subject.
    AdjacentSibling,
    /// `~`: the right compound is a following sibling of the left, so the left
    /// compound matches any earlier sibling of the subject.
    GeneralSibling,
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
    /// (the third field), the universal selector counts as nothing. An
    /// attribute selector and a structural pseudo-class count as a class;
    /// `:not()` counts as its argument, as the level-3 reference does.
    pub fn specificity(&self) -> (u32, u32, u32) {
        let mut ids = 0;
        let mut classes = 0;
        let mut tags = 0;
        for compound in &self.compounds {
            let (id, class, tag) = compound.specificity();
            ids += id;
            classes += class;
            tags += tag;
        }
        (ids, classes, tags)
    }

    /// Does this selector match `element`, whose ancestors are `ancestors`
    /// (root first, parent last)?
    ///
    /// The rightmost compound is the subject. The chain is matched right to
    /// left: a child combinator steps to the immediate parent, a descendant
    /// combinator to any ancestor, `+` to the immediately preceding sibling and
    /// `~` to any preceding sibling. Backtracking is needed because a nearer
    /// ancestor may satisfy one step but leave no ancestor for the next, as in
    /// `div > p span` where the nearest `p` sits under no `div`.
    pub(crate) fn matches(&self, element: &Element, ancestors: &[&Element]) -> bool {
        let last = self
            .compounds
            .len()
            .checked_sub(1)
            .expect("a selector has at least one compound");
        let context = MatchContext::of(element, ancestors);
        self.compounds[last].matches(element, &context) && self.match_chain(last, &context)
    }

    /// Match `compounds[..=index]`, given that `compounds[index]` matches the
    /// element `context` describes.
    fn match_chain(&self, index: usize, context: &MatchContext<'_>) -> bool {
        if index == 0 {
            return true;
        }
        let compound = &self.compounds[index - 1];
        match self.combinators[index - 1] {
            Combinator::Child => match context.ancestors.split_last() {
                None => false,
                Some((parent, rest)) => {
                    let parent_context = MatchContext::of(parent, rest);
                    compound.matches(parent, &parent_context)
                        && self.match_chain(index - 1, &parent_context)
                }
            },
            Combinator::Descendant => {
                context
                    .ancestors
                    .iter()
                    .enumerate()
                    .rev()
                    .any(|(position, ancestor)| {
                        let ancestor_context =
                            MatchContext::of(ancestor, &context.ancestors[..position]);
                        compound.matches(ancestor, &ancestor_context)
                            && self.match_chain(index - 1, &ancestor_context)
                    })
            }
            Combinator::AdjacentSibling => {
                // The sibling just before `element` shares its ancestry, so the
                // context is the same but for the sibling's own index.
                if context.index == 0 {
                    return false;
                }
                let Some(sibling) = context.siblings().nth(context.index - 1) else {
                    return false;
                };
                let sibling_context = MatchContext {
                    index: context.index - 1,
                    ..*context
                };
                compound.matches(sibling, &sibling_context)
                    && self.match_chain(index - 1, &sibling_context)
            }
            Combinator::GeneralSibling => {
                context
                    .siblings()
                    .take(context.index)
                    .enumerate()
                    .any(|(position, sibling)| {
                        let sibling_context = MatchContext {
                            index: position,
                            ..*context
                        };
                        compound.matches(sibling, &sibling_context)
                            && self.match_chain(index - 1, &sibling_context)
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

/// Parse one complex selector: compounds joined by combinators, such as
/// `div.card > p span` or `h1 + p ~ span`.
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
        // A combinator may be written with or without surrounding spaces; a
        // space alone is the descendant combinator.
        if let Some(after) = trimmed.strip_prefix('>') {
            combinators.push(Combinator::Child);
            rest = after.trim_start();
        } else if let Some(after) = trimmed.strip_prefix('+') {
            combinators.push(Combinator::AdjacentSibling);
            rest = after.trim_start();
        } else if let Some(after) = trimmed.strip_prefix('~') {
            combinators.push(Combinator::GeneralSibling);
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
    matches!(
        character,
        '#' | '.' | '[' | ':' | '>' | '+' | '~' | '*' | ',' | '(' | ')'
    ) || character.is_whitespace()
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
/// The grammar is an optional tag (or `*`) followed by any number of `#id`,
/// `.class`, `[attribute]` and `:pseudo-class` parts, so `div.card#main[href]`,
/// `.c`, `#x`, `*` and `:first-child` are all compounds.
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
    } else if let Some(after) = rest.strip_prefix('*') {
        // The universal selector at the head of the compound, which `ends_name`
        // also treats as a boundary so that `*:first-child` splits here.
        named = true;
        rest = after;
    }

    while let Some(first) = rest.chars().next() {
        match first {
            '#' | '.' => {
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
            '[' => {
                let (attribute, tail) = parse_attribute(rest, text)?;
                compound.attributes.push(attribute);
                named = true;
                rest = tail;
            }
            ':' => {
                let (pseudo, tail) = parse_pseudo_class(rest, text)?;
                match pseudo {
                    ParsedPseudo::Class(class) => compound.pseudo_classes.push(class),
                    ParsedPseudo::Negation(argument) => {
                        if compound.negation.is_some() {
                            return Err(Error::Malformed(format!(
                                "more than one :not() in selector `{text}`"
                            )));
                        }
                        compound.negation = Some(Box::new(argument));
                    }
                }
                named = true;
                rest = tail;
            }
            _ => break,
        }
    }

    if !named {
        return Err(Error::Malformed(format!("cannot read selector `{text}`")));
    }
    Ok((compound, rest))
}

/// One parsed `:pseudo-class` part.
enum ParsedPseudo {
    /// A structural pseudo-class.
    Class(PseudoClass),
    /// A `:not()` and the compound it negates.
    Negation(Compound),
}

/// Parse an attribute selector `[...]` and return it with the text after `]`.
fn parse_attribute<'t>(text: &'t str, whole: &str) -> Result<(AttributeSelector, &'t str), Error> {
    let body_start = text
        .strip_prefix('[')
        .ok_or_else(|| Error::Malformed(format!("expected `[` in selector `{whole}`")))?;
    let close = find_attribute_close(body_start)
        .ok_or_else(|| Error::Malformed(format!("unclosed attribute selector `{whole}`")))?;
    let body = body_start[..close].trim();
    let rest = &body_start[close + 1..];

    // The name runs to an operator or the end. The value operators all begin
    // with one of `~ | ^ $ *`, so the name ends there too; a bare `|` would be
    // a namespace, which is not supported, so the parse fails below.
    let name_end = body
        .find(|c: char| c.is_whitespace() || c == '=' || matches!(c, '~' | '|' | '^' | '$' | '*'))
        .unwrap_or(body.len());
    let name = &body[..name_end];
    if name.is_empty() {
        return Err(Error::Malformed(format!(
            "empty attribute name in selector `{whole}`"
        )));
    }
    let after_name = body[name_end..].trim_start();
    if after_name.is_empty() {
        return Ok((
            AttributeSelector {
                name: name.to_ascii_lowercase(),
                test: None,
            },
            rest,
        ));
    }
    // The operator is one or two characters ending in `=`.
    let (operator, after_operator) = if let Some(rest) = after_name.strip_prefix('=') {
        (AttributeMatch::Equals, rest)
    } else {
        let mut chars = after_name.chars();
        let sigil = chars.next().unwrap_or(' ');
        let operator = match sigil {
            '~' => AttributeMatch::Includes,
            '|' => AttributeMatch::DashMatch,
            '^' => AttributeMatch::Prefix,
            '$' => AttributeMatch::Suffix,
            '*' => AttributeMatch::Substring,
            _ => {
                return Err(Error::Malformed(format!(
                    "cannot read attribute selector `{whole}`"
                )));
            }
        };
        let Some(rest) = after_name[sigil.len_utf8()..].strip_prefix('=') else {
            return Err(Error::Malformed(format!(
                "cannot read attribute selector `{whole}`"
            )));
        };
        (operator, rest)
    };
    let value = parse_attribute_value(after_operator.trim_start(), whole)?;
    Ok((
        AttributeSelector {
            name: name.to_ascii_lowercase(),
            test: Some((operator, value)),
        },
        rest,
    ))
}

/// Find the `]` that closes an attribute selector, ignoring one inside a quoted
/// value, so `[title="a]b"]` reads its whole value. Returns the byte offset of
/// the `]` within `body_start`.
fn find_attribute_close(body_start: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    for (position, character) in body_start.char_indices() {
        match quote {
            Some(open) => {
                if character == open {
                    quote = None;
                }
            }
            None => match character {
                '"' | '\'' => quote = Some(character),
                ']' => return Some(position),
                _ => {}
            },
        }
    }
    None
}

/// Read an attribute selector's value, quoted or bare.
fn parse_attribute_value(text: &str, whole: &str) -> Result<String, Error> {
    match text.chars().next() {
        Some(quote @ ('"' | '\'')) => {
            let body = &text[quote.len_utf8()..];
            let end = body.find(quote).ok_or_else(|| {
                Error::Malformed(format!("unclosed attribute value in selector `{whole}`"))
            })?;
            Ok(body[..end].to_owned())
        }
        Some(_) => {
            let end = text.find(']').unwrap_or(text.len());
            Ok(text[..end].trim().to_owned())
        }
        None => Err(Error::Malformed(format!(
            "missing attribute value in selector `{whole}`"
        ))),
    }
}

/// Parse a `:pseudo-class` part and return it with the text after it.
fn parse_pseudo_class<'t>(text: &'t str, whole: &str) -> Result<(ParsedPseudo, &'t str), Error> {
    let body = text
        .strip_prefix(':')
        .ok_or_else(|| Error::Malformed(format!("expected `:` in selector `{whole}`")))?;
    let name_end = body
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-'))
        .unwrap_or(body.len());
    let name = &body[..name_end];
    let after = &body[name_end..];

    let simple = |class: PseudoClass| Ok((ParsedPseudo::Class(class), after));
    match name {
        "root" => return simple(PseudoClass::Root),
        "first-child" => return simple(PseudoClass::FirstChild),
        "last-child" => return simple(PseudoClass::LastChild),
        "only-child" => return simple(PseudoClass::OnlyChild),
        "nth-child" | "nth-last-child" => {
            let (argument, rest) = read_parenthesized(after, whole)?;
            let nth = parse_nth(&argument)
                .ok_or_else(|| Error::Malformed(format!("cannot read `:{name}({argument})`")))?;
            let class = if name == "nth-child" {
                PseudoClass::NthChild(nth)
            } else {
                PseudoClass::NthLastChild(nth)
            };
            return Ok((ParsedPseudo::Class(class), rest));
        }
        "not" => {
            let (argument, rest) = read_parenthesized(after, whole)?;
            let (compound, leftover) = parse_compound(argument.trim())?;
            if !leftover.trim().is_empty() {
                return Err(Error::Malformed(format!(
                    "`:not()` takes a single compound in selector `{whole}`"
                )));
            }
            return Ok((ParsedPseudo::Negation(compound), rest));
        }
        _ => {}
    }
    Err(Error::Malformed(format!(
        "unsupported pseudo-class `:{name}` in selector `{whole}`"
    )))
}

/// Read a parenthesized argument, returning it and the text after `)`.
///
/// The argument may itself hold parentheses, as `:not(:nth-child(2n+1))` does,
/// so the closing `)` is the one that matches the opening `(` rather than the
/// first one found.
fn read_parenthesized<'t>(text: &'t str, whole: &str) -> Result<(String, &'t str), Error> {
    let body = text
        .strip_prefix('(')
        .ok_or_else(|| Error::Malformed(format!("expected `(` in selector `{whole}`")))?;
    let mut depth = 1usize;
    for (position, character) in body.char_indices() {
        match character {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((
                        body[..position].to_owned(),
                        &body[position + character.len_utf8()..],
                    ));
                }
            }
            _ => {}
        }
    }
    Err(Error::Malformed(format!(
        "unclosed `(` in selector `{whole}`"
    )))
}

/// Parse the `an+b` argument of an `:nth-*()` pseudo-class.
///
/// Accepts `odd`, `even`, a bare integer (`3`), a bare `n` form (`n`, `2n`,
/// `-n`), and the full `an+b` (`2n+1`, `2n - 1`, `-3n+2`). Whitespace around
/// the `+`/`-` sign is allowed, as the grammar permits.
fn parse_nth(text: &str) -> Option<Nth> {
    let compact: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let lowered = compact.to_ascii_lowercase();
    match lowered.as_str() {
        "odd" => return Some(Nth { a: 2, b: 1 }),
        "even" => return Some(Nth { a: 2, b: 0 }),
        _ => {}
    }
    let Some(n_position) = lowered.find('n') else {
        let b: i64 = lowered.parse().ok()?;
        return Some(Nth { a: 0, b });
    };
    let a_text = &lowered[..n_position];
    let a = match a_text {
        "" | "+" => 1,
        "-" => -1,
        other => other.parse().ok()?,
    };
    let b_text = &lowered[n_position + 1..];
    let b = if b_text.is_empty() {
        0
    } else {
        b_text.parse().ok()?
    };
    Some(Nth { a, b })
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
            attributes: Vec::new(),
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

    /// An element with the given attributes, name lower-cased as the HTML
    /// parser lower-cases them.
    fn element_with(tag: &str, attributes: &[(&str, &str)]) -> Element {
        Element {
            tag: tag.to_owned(),
            id: attributes
                .iter()
                .find(|(name, _)| *name == "id")
                .map(|(_, value)| (*value).to_owned()),
            classes: attributes
                .iter()
                .find(|(name, _)| *name == "class")
                .map(|(_, value)| value.split_whitespace().map(str::to_owned).collect())
                .unwrap_or_default(),
            style: None,
            attributes: attributes
                .iter()
                .map(|(name, value)| ((*name).to_owned(), Some((*value).to_owned())))
                .collect(),
            children: Vec::new(),
        }
    }

    #[test]
    fn a_sibling_combinator_parses_with_or_without_spaces() {
        for text in ["h1 + p", "h1+p", "h1 ~ p", "h1~p"] {
            let selector = sheet(text);
            assert_eq!(selector.compounds.len(), 2, "{text}");
            assert_eq!(
                selector.combinators,
                vec![if text.contains('+') {
                    Combinator::AdjacentSibling
                } else {
                    Combinator::GeneralSibling
                }],
                "{text}"
            );
        }
    }

    #[test]
    fn an_attribute_selector_parses_into_its_operator_and_value() {
        let cases = [
            ("[a]", None),
            ("[a=v]", Some((AttributeMatch::Equals, "v"))),
            ("[a~=v]", Some((AttributeMatch::Includes, "v"))),
            ("[a|=v]", Some((AttributeMatch::DashMatch, "v"))),
            ("[a^=v]", Some((AttributeMatch::Prefix, "v"))),
            ("[a$=v]", Some((AttributeMatch::Suffix, "v"))),
            ("[a*=v]", Some((AttributeMatch::Substring, "v"))),
            ("[a='v']", Some((AttributeMatch::Equals, "v"))),
            ("[A=v]", Some((AttributeMatch::Equals, "v"))),
            // A `]` inside a quoted value does not close the selector.
            ("[a='v]w']", Some((AttributeMatch::Equals, "v]w"))),
        ];
        for (text, expected) in cases {
            let selector = sheet(text);
            let attribute = &selector.compounds[0].attributes[0];
            assert_eq!(attribute.name, "a", "{text}");
            assert_eq!(
                attribute.test,
                expected.map(|(operator, value)| (operator, value.to_owned())),
                "{text}"
            );
        }
    }

    #[test]
    fn the_attribute_operators_match_the_reference() {
        let element = element_with(
            "div",
            &[("data-x", "a b c"), ("lang", "en-US"), ("empty", "")],
        );
        let matches = |selector: &str| sheet(selector).matches(&element, &[]);

        assert!(matches("[data-x]"));
        assert!(!matches("[missing]"));
        assert!(matches("[data-x='a b c']"));
        assert!(!matches("[data-x='a']"));
        assert!(matches("[data-x~='b']"));
        // `~=` compares whole whitespace-separated words, so a substring fails.
        assert!(!matches("[data-x~='a b']"));
        assert!(matches("[lang|='en']"));
        assert!(!matches("[lang|='en-']"));
        assert!(matches("[data-x^='a b']"));
        assert!(!matches("[data-x^='b']"));
        assert!(matches("[data-x$='c']"));
        assert!(!matches("[data-x$='b']"));
        assert!(matches("[data-x*='b']"));
        assert!(!matches("[data-x*='z']"));
        // An empty value matches an empty attribute, but `^=`, `$=` and `*=`
        // with an empty argument never match.
        assert!(matches("[empty='']"));
        assert!(!matches("[data-x^='']"));
    }

    #[test]
    fn the_structural_pseudo_classes_read_the_sibling_position() {
        let parent = Element {
            children: (0..3).map(|_| Node::Element(element("div"))).collect(),
            ..element("section")
        };
        let first = match &parent.children[0] {
            Node::Element(element) => element,
            Node::Text(_) => unreachable!(),
        };
        let second = match &parent.children[1] {
            Node::Element(element) => element,
            Node::Text(_) => unreachable!(),
        };
        let third = match &parent.children[2] {
            Node::Element(element) => element,
            Node::Text(_) => unreachable!(),
        };
        let ancestors: Vec<&Element> = vec![&parent];

        let matches =
            |selector: &str, element: &Element| sheet(selector).matches(element, &ancestors);

        assert!(matches("div:first-child", first));
        assert!(!matches("div:first-child", second));
        assert!(matches("div:last-child", third));
        assert!(matches("div:nth-child(2)", second));
        assert!(matches("div:nth-child(odd)", first));
        assert!(matches("div:nth-child(odd)", third));
        assert!(matches("div:nth-child(even)", second));
        assert!(matches("div:nth-child(2n+1)", third));
        assert!(matches("div:nth-last-child(1)", third));
        assert!(matches("div:nth-last-child(2)", second));
        assert!(!matches("div:only-child", first));
        assert!(matches("div:not(:first-child)", second));
        assert!(!matches("div:not(:first-child)", first));
    }

    #[test]
    fn a_sibling_combinator_matches_the_siblings_before_the_subject() {
        let parent = Element {
            children: (0..3).map(|_| Node::Element(element("div"))).collect(),
            ..element("section")
        };
        let first = match &parent.children[0] {
            Node::Element(element) => element,
            Node::Text(_) => unreachable!(),
        };
        let second = match &parent.children[1] {
            Node::Element(element) => element,
            Node::Text(_) => unreachable!(),
        };
        let third = match &parent.children[2] {
            Node::Element(element) => element,
            Node::Text(_) => unreachable!(),
        };
        let ancestors: Vec<&Element> = vec![&parent];

        // `+` matches only the second, whose immediately preceding sibling is a
        // `div`.
        assert!(!sheet("div + div").matches(first, &ancestors));
        assert!(sheet("div + div").matches(second, &ancestors));
        assert!(sheet("div + div").matches(third, &ancestors));
        // `~` matches both later siblings.
        assert!(sheet("div ~ div").matches(third, &ancestors));
        // A sibling combinator needs a shared parent, so an element whose only
        // ancestor is the parent is matched, but a deeper one is not.
        assert!(!sheet("section + div").matches(first, &ancestors));
    }

    #[test]
    fn the_nth_argument_parses_odd_even_and_an_plus_b() {
        assert_eq!(parse_nth("odd"), Some(Nth { a: 2, b: 1 }));
        assert_eq!(parse_nth("even"), Some(Nth { a: 2, b: 0 }));
        assert_eq!(parse_nth("3"), Some(Nth { a: 0, b: 3 }));
        assert_eq!(parse_nth("n"), Some(Nth { a: 1, b: 0 }));
        assert_eq!(parse_nth("2n"), Some(Nth { a: 2, b: 0 }));
        assert_eq!(parse_nth("-n"), Some(Nth { a: -1, b: 0 }));
        assert_eq!(parse_nth("2n+1"), Some(Nth { a: 2, b: 1 }));
        assert_eq!(parse_nth("2n - 1"), Some(Nth { a: 2, b: -1 }));
        assert_eq!(parse_nth("-3n+2"), Some(Nth { a: -3, b: 2 }));
        assert_eq!(parse_nth("nonsense"), None);
    }

    #[test]
    fn a_negated_attribute_and_pseudo_class_are_read() {
        let element = element_with("div", &[("data-x", "a")]);
        assert!(!sheet("div:not([data-x])").matches(&element, &[]));
        assert!(sheet("div:not([data-y])").matches(&element, &[]));
    }

    #[test]
    fn the_new_grammar_rejects_what_it_does_not_support() {
        for text in [
            "div:nth-child()",
            "div:hover",
            "div::before",
            "div:not(.a .b)",
            "[a='unclosed]",
            "[=v]",
        ] {
            let source = format!("{text} {{ width: 1px }}");
            assert!(
                Stylesheet::parse(&source).is_err(),
                "`{text}` should not parse"
            );
        }
    }

    #[test]
    fn an_attribute_and_a_pseudo_class_add_to_specificity() {
        // An attribute selector and a pseudo-class each count as a class.
        assert_eq!(sheet("div[data-x]").specificity(), (0, 1, 1));
        assert_eq!(sheet("div:first-child").specificity(), (0, 1, 1));
        // `:not()` counts as its argument, not for itself.
        assert_eq!(sheet("div:not(.a)").specificity(), (0, 1, 1));
        assert_eq!(sheet("div:not(#a)").specificity(), (1, 0, 1));
    }
}
