//! A small HTML parser: enough of a document tree to lay out.

use crate::error::Error;

/// An element with the attributes a selector can match, and its children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The lower-cased tag name, such as `div`.
    pub tag: String,
    /// The `id` attribute, if present.
    pub id: Option<String>,
    /// The `class` attribute split on whitespace, empty when absent.
    pub classes: Vec<String>,
    /// The `style` attribute, if the element carries one.
    pub style: Option<String>,
    /// The children, in document order.
    pub children: Vec<Node>,
}

/// A node is either an element or a run of text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Node {
    /// An element and its subtree.
    Element(Element),
    /// Text between elements.
    Text(String),
}

/// Elements that start a new block, matching the reference's test user-agent
/// stylesheet. Everything else is treated as a non-rendered container, which is
/// what makes `style` and `script` disappear.
const BLOCK_TAGS: &[&str] = &[
    "html", "body", "div", "h1", "h2", "h3", "h4", "ol", "p", "ul", "hr", "pre", "section",
    "article",
];

/// Elements the reference's test stylesheet hides outright.
const HIDDEN_TAGS: &[&str] = &["head", "style", "script", "title", "meta", "link"];

/// Is `tag` a block element under the test user-agent stylesheet?
pub(crate) fn is_block(tag: &str) -> bool {
    BLOCK_TAGS.contains(&tag)
}

/// Is `tag` hidden under the test user-agent stylesheet?
pub(crate) fn is_hidden(tag: &str) -> bool {
    HIDDEN_TAGS.contains(&tag)
}

/// Parse `html` into an element tree.
///
/// # Errors
///
/// Returns [`Error::Malformed`] when a start tag is never closed.
pub(crate) fn parse(html: &str) -> Result<Element, Error> {
    let mut stack: Vec<Element> = vec![Element {
        tag: "html".to_owned(),
        id: None,
        classes: Vec::new(),
        style: None,
        children: Vec::new(),
    }];
    let mut rest = html;

    while let Some(open) = rest.find('<') {
        push_text(&mut stack, &rest[..open]);
        let after = &rest[open..];
        if let Some(close) = after.find('>') {
            let tag = &after[1..close];
            handle_tag(&mut stack, tag)?;
            rest = &after[close + 1..];
        } else {
            // A lone `<` with no `>` is text, not a tag.
            push_text(&mut stack, after);
            rest = "";
        }
    }
    push_text(&mut stack, rest);

    if stack.len() > 1 {
        return Err(Error::Malformed(format!(
            "unclosed element `{}`",
            stack.last().map(|e| e.tag.as_str()).unwrap_or("?")
        )));
    }
    let root = stack.remove(0);
    Ok(insert_head_and_body(root))
}

/// Wrap the parsed children in `head` and `body`, as an HTML parser would.
///
/// A document fragment such as `<style>...</style><div></div>` has no `body`,
/// but the reference always produces `html > body > div` because a real parser
/// inserts one. The head holds the tags the user-agent stylesheet hides, which
/// is why they produce no box; everything else goes into the body.
fn insert_head_and_body(root: Element) -> Element {
    if root
        .children
        .iter()
        .any(|node| matches!(node, Node::Element(element) if element.tag == "body"))
    {
        return root;
    }

    let mut head = Element {
        tag: "head".to_owned(),
        id: None,
        classes: Vec::new(),
        style: None,
        children: Vec::new(),
    };
    let mut body = Element {
        tag: "body".to_owned(),
        id: None,
        classes: Vec::new(),
        style: None,
        children: Vec::new(),
    };

    for node in root.children {
        match &node {
            Node::Element(element) if is_head_tag(&element.tag) => head.children.push(node),
            _ => body.children.push(node),
        }
    }

    Element {
        tag: "html".to_owned(),
        id: root.id,
        classes: root.classes,
        style: root.style,
        children: vec![Node::Element(head), Node::Element(body)],
    }
}

/// Tags that belong in the head, and are hidden by the user-agent stylesheet.
fn is_head_tag(tag: &str) -> bool {
    matches!(
        tag,
        "head" | "style" | "script" | "title" | "meta" | "link" | "base"
    )
}

/// Read one tag body - the text between `<` and `>` - and apply it.
fn handle_tag(stack: &mut Vec<Element>, tag: &str) -> Result<(), Error> {
    let tag = tag.trim();
    if tag.starts_with('!') || tag.starts_with('?') {
        return Ok(());
    }
    if let Some(name) = tag.strip_prefix('/') {
        let name = name.trim().to_ascii_lowercase();
        // Close the nearest matching open element. A stray close tag is
        // ignored rather than treated as an error, as browsers do.
        if let Some(index) = stack.iter().rposition(|e| e.tag == name) {
            let closed = stack.split_off(index);
            let element = closed.into_iter().next().expect("index is in range");
            stack
                .last_mut()
                .expect("html is always on the stack")
                .children
                .push(Node::Element(element));
        }
        return Ok(());
    }

    let (name, attributes) = split_name_and_attributes(tag);
    if name.is_empty() {
        return Ok(());
    }
    let element = Element {
        id: attribute(attributes, "id"),
        classes: attribute(attributes, "class")
            .map(|value| value.split_whitespace().map(str::to_owned).collect())
            .unwrap_or_default(),
        tag: name,
        style: attribute(attributes, "style"),
        children: Vec::new(),
    };
    if VOID_TAGS.contains(&element.tag.as_str()) {
        stack
            .last_mut()
            .expect("html is always on the stack")
            .children
            .push(Node::Element(element));
    } else {
        stack.push(element);
    }
    Ok(())
}

/// Tags that never have children, so they are closed immediately.
const VOID_TAGS: &[&str] = &["br", "hr", "img", "meta", "link", "input"];

/// Split `div style="a"` into the tag name and the attribute string.
fn split_name_and_attributes(tag: &str) -> (String, &str) {
    let name_end = tag
        .find(|c: char| c.is_ascii_whitespace())
        .unwrap_or(tag.len());
    let name = tag[..name_end].to_ascii_lowercase();
    (name, &tag[name_end..])
}

/// Read one attribute value from an attribute string.
///
/// `wanted` is matched at a word boundary, so looking for `id` does not match
/// the `id` inside `data-id`.
fn attribute(attributes: &str, wanted: &str) -> Option<String> {
    let mut rest = attributes;
    loop {
        let start = rest.find(wanted)?;
        let preceded_by_word = start > 0
            && rest[..start]
                .chars()
                .next_back()
                .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if preceded_by_word {
            rest = &rest[start + wanted.len()..];
            continue;
        }
        let followed_by_word = rest[start + wanted.len()..]
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if followed_by_word {
            rest = &rest[start + wanted.len()..];
            continue;
        }
        let after = &rest[start + wanted.len()..];
        let after = after.trim_start();
        let Some(after) = after.strip_prefix('=') else {
            rest = after;
            continue;
        };
        let after = after.trim_start();
        let quote = after.chars().next()?;
        if quote == '"' || quote == '\'' {
            let value = &after[1..];
            let end = value.find(quote)?;
            return Some(value[..end].to_owned());
        }
        let end = after
            .find(|c: char| c.is_ascii_whitespace())
            .unwrap_or(after.len());
        return Some(after[..end].to_owned());
    }
}

/// Append text to the element on top of the stack.
fn push_text(stack: &mut [Element], text: &str) {
    if text.trim().is_empty() {
        return;
    }
    stack
        .last_mut()
        .expect("html is always on the stack")
        .children
        .push(Node::Text(text.to_owned()));
}
