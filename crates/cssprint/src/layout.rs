//! Block and inline layout: the CSS 2.1 formatting model.
//!
//! A block box fills its containing block unless `width` says otherwise, and
//! each later sibling is placed below the previous one. Text and inline
//! elements are gathered into line boxes inside their block, wrapped to the
//! block's width, and placed on a shared baseline. Every rule below was read off
//! the reference rather than assumed, and each is checked in `tests/layout.rs`.
//!
//! **Horizontal resolution** follows `block_level_width` in WeasyPrint's
//! `layout/block.py`: an `auto` width absorbs the space left over, `min-width`
//! and `max-width` clamp it, and then the margins are solved so that
//! `margin-left + border + padding + width + padding + border + margin-right`
//! equals the containing width. When neither margin is `auto` the equation is
//! over-constrained and *both margins keep their specified values* - the right
//! margin is not adjusted, which is a departure from CSS 2.1 that the reference
//! makes and that the fixture records.
//!
//! **Vertical margins collapse.** A box's top margin collapses with its first
//! child's when neither has a top border or padding and both have `auto` height,
//! and the collapsed margin is the larger of the two. The collapsed margin
//! positions the *outermost* box in the group; each box still reports its own
//! declared margin. So for `<body style="margin:0"><div style="margin:10px">`,
//! the body is pushed down by ten while reporting `margin-top: 0`, and the div
//! reports `margin-top: 10`. The root element does not collapse with the page.
//!
//! **Lines share a baseline.** Each box on a line sits on the line's baseline,
//! which is as far down as the deepest box reaches. Writing `f` for the font
//! size and `l` for the used `line-height`, a box's own baseline sits
//! `ascent(f) + (l - f) / 2` below its line-height top, so the line's baseline is
//! the largest of those and the line's height spans every box's line-height box.
//! A box's content box then starts `baseline - ascent(f)` below the line top.
//! That is why raising `line-height` makes the line taller without moving the
//! text, and why a larger font size pushes the baseline down.
//!
//! **Whitespace collapses per text node, then again at the line's edges.** A run
//! of whitespace becomes one space, and a space is dropped when the previous text
//! node already ended in one. A space at the start of a line, and a space at the
//! end of a line, are removed once the line is known. A text node's collapsed
//! text stays in one box, so `a    b` is a single box reading `a b`.
//!
//! **Text is laid out for one font.** This crate does not read font files, so it
//! assumes the font the fixture uses, which is monospace at one em per glyph
//! with an ascent of 819/1024 em. Every width is then a character count times
//! the font size, which is what makes the corpus reproducible on any machine. A
//! proportional font would need a metrics source and is not covered.

use crate::dom::{Element, Node};
use crate::error::Error;
use crate::style::{Display, Length, Sides, Style};

/// The kind of box, named as the reference names it so that a tree can be
/// compared with WeasyPrint's directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxKind {
    /// A block-level box, which stacks vertically.
    Block,
    /// An inline box, which sits in a line.
    Inline,
    /// A line box, which holds one row of inline-level content.
    Line,
    /// A text run.
    Text,
    /// The page itself, the root of the tree.
    Page,
}

/// A laid-out box with resolved geometry.
///
/// `content_x` and `content_y` are absolute, measured from the top-left of the
/// page, and they locate the *content* box - inside the border and padding.
/// Those are the quantities WeasyPrint reports as `content_box_x` and
/// `content_box_y`, which is what makes the two trees comparable.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutBox {
    /// The tag name; empty for the page, and the innermost enclosing element's
    /// tag for a text run.
    pub tag: String,
    /// The kind of box.
    pub kind: BoxKind,
    /// The absolute x of the content box.
    pub content_x: f64,
    /// The absolute y of the content box.
    pub content_y: f64,
    /// The width of the content box.
    pub width: f64,
    /// The height of the content box.
    pub height: f64,
    /// The used margins, with `auto` replaced by its used value.
    pub margin: Sides,
    /// The padding.
    pub padding: Sides,
    /// The border widths.
    pub border: Sides,
    /// The text of a text box, and `None` for every other kind.
    pub text: Option<String>,
    /// The child boxes, in document order.
    pub children: Vec<LayoutBox>,
}

/// A laid-out document: the page box, with `html` and `body` beneath it.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    /// The page box, whose only child is `html`.
    pub root: LayoutBox,
}

impl Document {
    /// The `body` box, which holds the document's own elements.
    ///
    /// # Panics
    ///
    /// Panics only if the tree was built by something other than [`crate::layout`],
    /// which always produces `page > html > body`.
    pub fn body(&self) -> &LayoutBox {
        &self.root.children[0].children[0]
    }
}

/// The advance width of one glyph, in ems.
const ADVANCE_PER_GLYPH: f64 = 1.0;
/// The ascent of the font, in ems, from its top to the baseline.
const ASCENT: f64 = 819.0 / 1024.0;

/// A box during layout, before its absolute position is known.
///
/// Offsets are relative to the parent's content box, which is what lets a tree
/// be built bottom-up and then placed top-down in one pass.
#[derive(Debug)]
struct Fragment {
    kind: BoxKind,
    tag: String,
    /// Distance from the parent's content-box left edge to this box's content box.
    x_offset: f64,
    /// Distance from the parent's content-box top edge to this box's content box.
    y_offset: f64,
    width: f64,
    height: f64,
    margin: Sides,
    padding: Sides,
    border: Sides,
    text: Option<String>,
    /// The margin this box presents to its parent, after any collapse.
    effective_margin_top: f64,
    effective_margin_bottom: f64,
    /// Border plus padding on the top.
    inner_top: f64,
    /// Border plus padding on the bottom.
    inner_bottom: f64,
    children: Vec<Fragment>,
}

/// Lay out a parsed document into a page box.
pub(crate) fn layout_document(
    dom: &Element,
    stylesheet: &crate::Stylesheet,
    page_width: f64,
) -> Result<Document, Error> {
    let mut html = layout_element(dom, stylesheet, page_width, true, None)?;
    // The root element does not collapse its margin with the page, so it starts
    // at the origin.
    html.y_offset = html.inner_top;
    let html = place(&html, 0.0, 0.0);
    let root = LayoutBox {
        tag: String::new(),
        kind: BoxKind::Page,
        content_x: 0.0,
        content_y: 0.0,
        width: page_width,
        height: page_width,
        margin: Sides::default(),
        padding: Sides::default(),
        border: Sides::default(),
        text: None,
        children: vec![html],
    };
    Ok(Document { root })
}

/// Lay out one block-level element and its subtree.
///
/// `is_root` marks the `html` element, which neither collapses with the page nor
/// with its own first child in the way an ordinary box does.
fn layout_element(
    element: &Element,
    stylesheet: &crate::Stylesheet,
    containing_width: f64,
    is_root: bool,
    parent_style: Option<&Style>,
) -> Result<Fragment, Error> {
    let style = crate::style::compute(element, stylesheet, parent_style)?;
    let (width, x_offset, margin_left, margin_right) = resolve_horizontal(&style, containing_width);

    let inner_top = style.border.top + style.padding.top;
    let inner_bottom = style.border.bottom + style.padding.bottom;
    let own_margin_top = resolve_margin(style.margin[0], containing_width);
    let own_margin_bottom = resolve_margin(style.margin[2], containing_width);

    let mut children = layout_children(element, stylesheet, width, &style)?;

    let auto_height = style.height.is_none();
    let declared_height = style.height;

    if children.is_empty() {
        return Ok(Fragment {
            kind: BoxKind::Block,
            tag: element.tag.clone(),
            x_offset,
            y_offset: inner_top,
            width,
            height: declared_height.unwrap_or(0.0),
            margin: Sides {
                top: own_margin_top,
                right: margin_right,
                bottom: own_margin_bottom,
                left: margin_left,
            },
            padding: style.padding,
            border: style.border,
            text: None,
            effective_margin_top: own_margin_top,
            effective_margin_bottom: own_margin_bottom,
            inner_top,
            inner_bottom,
            children,
        });
    }

    let first = &children[0];
    let last = children.last().expect("not empty");
    // A line box carries no margin, so it never collapses; the parent's own
    // margin is what applies. The same holds for an anonymous block, which is
    // built with zero margins.
    let collapses_top = !is_root && auto_height && inner_top == 0.0 && first.inner_top == 0.0;
    let collapses_bottom =
        !is_root && auto_height && inner_bottom == 0.0 && last.inner_bottom == 0.0;

    let effective_margin_top = if collapses_top {
        own_margin_top.max(first.effective_margin_top)
    } else {
        own_margin_top
    };
    let effective_margin_bottom = if collapses_bottom {
        own_margin_bottom.max(last.effective_margin_bottom)
    } else {
        own_margin_bottom
    };

    // Walk the children, leaving each the gap its margin requires. A first child
    // whose margin collapsed out of this box contributes no gap, because the
    // collapsed margin has already moved this box itself.
    let mut cursor = 0.0_f64;
    let mut previous_bottom = 0.0_f64;
    for (index, child) in children.iter_mut().enumerate() {
        let gap = if index == 0 {
            if collapses_top {
                0.0
            } else {
                child.effective_margin_top
            }
        } else {
            previous_bottom.max(child.effective_margin_top)
        };
        cursor += gap;
        child.y_offset = cursor + child.inner_top;
        cursor = child.y_offset + child.height + child.inner_bottom;
        previous_bottom = child.effective_margin_bottom;
    }

    let content_height = if collapses_bottom {
        cursor
    } else {
        cursor + previous_bottom
    };

    Ok(Fragment {
        kind: BoxKind::Block,
        tag: element.tag.clone(),
        x_offset,
        y_offset: inner_top,
        width,
        height: declared_height.unwrap_or(content_height),
        margin: Sides {
            top: own_margin_top,
            right: margin_right,
            bottom: own_margin_bottom,
            left: margin_left,
        },
        padding: style.padding,
        border: style.border,
        text: None,
        effective_margin_top,
        effective_margin_bottom,
        inner_top,
        inner_bottom,
        children,
    })
}

/// Lay out an element's children in document order.
///
/// Block-level children become block boxes. Everything else is a run of inline
/// content, which becomes line boxes; when such a run sits between block-level
/// children it is wrapped in an anonymous block that carries the parent's tag,
/// which is what the reference produces and why `a<p>b</p>c` inside a `div`
/// yields three boxes tagged `div`.
fn layout_children(
    element: &Element,
    stylesheet: &crate::Stylesheet,
    width: f64,
    style: &Style,
) -> Result<Vec<Fragment>, Error> {
    let mut out = Vec::new();
    let mut run: Vec<&Node> = Vec::new();
    let mut saw_block = false;

    for node in &element.children {
        let block_child = match node {
            Node::Text(_) => None,
            Node::Element(child) => {
                let child_style = crate::style::compute(child, stylesheet, Some(style))?;
                match child_style.display {
                    Display::None => continue,
                    Display::Block => Some(child),
                    Display::Inline => None,
                }
            }
        };
        match block_child {
            Some(child) => {
                saw_block = true;
                out.extend(layout_run(&run, element, stylesheet, width, style, true)?);
                run.clear();
                out.push(layout_element(
                    child,
                    stylesheet,
                    width,
                    false,
                    Some(style),
                )?);
            }
            None => run.push(node),
        }
    }
    out.extend(layout_run(
        &run, element, stylesheet, width, style, !saw_block,
    )?);
    Ok(out)
}

/// Lay out one run of inline content.
///
/// `direct` says the run is the element's entire content, so its line boxes are
/// the element's own children; otherwise they go inside an anonymous block. A
/// run that produces no line box - whitespace between two blocks, say - produces
/// nothing at all, which is what the reference does.
fn layout_run(
    run: &[&Node],
    element: &Element,
    stylesheet: &crate::Stylesheet,
    width: f64,
    parent_style: &Style,
    direct: bool,
) -> Result<Vec<Fragment>, Error> {
    if run.is_empty() {
        return Ok(Vec::new());
    }
    let items = collect_items(run, stylesheet, parent_style)?;
    let mut lines = shape_lines(&items, width, &element.tag);
    if lines.is_empty() {
        return Ok(Vec::new());
    }
    if direct {
        return Ok(lines);
    }
    let height: f64 = lines.iter().map(|line| line.height).sum();
    let mut cursor = 0.0;
    for line in &mut lines {
        line.y_offset = cursor;
        cursor += line.height;
    }
    Ok(vec![Fragment {
        kind: BoxKind::Block,
        tag: element.tag.clone(),
        x_offset: 0.0,
        y_offset: 0.0,
        width,
        height,
        margin: Sides::default(),
        padding: Sides::default(),
        border: Sides::default(),
        text: None,
        effective_margin_top: 0.0,
        effective_margin_bottom: 0.0,
        inner_top: 0.0,
        inner_bottom: 0.0,
        children: lines,
    }])
}

/// One collapsed text node, with the inline elements it sits inside.
struct Item {
    /// The enclosing inline elements, outermost first.
    ancestors: Vec<InlineAncestor>,
    style: Style,
    /// The collapsed text, which may keep a leading or trailing space.
    text: String,
}

/// An enclosing inline element, identified so that two sibling elements of the
/// same tag are not merged into one box.
#[derive(Clone)]
struct InlineAncestor {
    id: usize,
    tag: String,
    style: Style,
}

/// Walk inline content, collapsing whitespace and emitting one item per text
/// node. The collapsing state is carried across nodes, which is what stops the
/// space between two inline elements from being doubled.
fn collect_items(
    nodes: &[&Node],
    stylesheet: &crate::Stylesheet,
    parent_style: &Style,
) -> Result<Vec<Item>, Error> {
    let mut items = Vec::new();
    let mut following_space = false;
    let mut next_id = 0;
    for node in nodes {
        collect(
            node,
            stylesheet,
            parent_style,
            &mut Vec::new(),
            &mut next_id,
            &mut following_space,
            &mut items,
        )?;
    }
    Ok(items)
}

/// The recursive half of [`collect_items`].
fn collect(
    node: &Node,
    stylesheet: &crate::Stylesheet,
    parent_style: &Style,
    ancestors: &mut Vec<InlineAncestor>,
    next_id: &mut usize,
    following_space: &mut bool,
    out: &mut Vec<Item>,
) -> Result<(), Error> {
    match node {
        Node::Text(text) => {
            let collapsed = collapse(text, following_space);
            if !collapsed.is_empty() {
                out.push(Item {
                    ancestors: ancestors.clone(),
                    style: parent_style.clone(),
                    text: collapsed,
                });
            }
            Ok(())
        }
        Node::Element(element) => {
            let style = crate::style::compute(element, stylesheet, Some(parent_style))?;
            if style.display == Display::None {
                return Ok(());
            }
            let id = *next_id;
            *next_id += 1;
            ancestors.push(InlineAncestor {
                id,
                tag: element.tag.clone(),
                style: style.clone(),
            });
            for child in &element.children {
                collect(
                    child,
                    stylesheet,
                    &style,
                    ancestors,
                    next_id,
                    following_space,
                    out,
                )?;
            }
            ancestors.pop();
            Ok(())
        }
    }
}

/// Collapse one text node: whitespace runs become one space, and a leading space
/// is dropped when the previous node already ended in one.
///
/// This mirrors `process_whitespace` in the reference's
/// `formatting_structure/boxes.py`. Leading and trailing spaces are kept here
/// and removed later, when the line they land on is known.
fn collapse(text: &str, following_space: &mut bool) -> String {
    let mut collapsed = String::with_capacity(text.len());
    let mut pending_space = false;
    for character in text.chars() {
        if character.is_whitespace() {
            pending_space = true;
            continue;
        }
        if pending_space {
            collapsed.push(' ');
            pending_space = false;
        }
        collapsed.push(character);
    }
    if pending_space {
        collapsed.push(' ');
    }
    if *following_space && collapsed.starts_with(' ') {
        collapsed.remove(0);
    }
    *following_space = collapsed.ends_with(' ');
    collapsed
}

/// A piece of inline content: a word, or a single space between two words.
struct Piece {
    ancestors: Vec<InlineAncestor>,
    style: Style,
    text: String,
    is_space: bool,
}

/// A piece placed on a line, with the x it ended up at.
struct Placed {
    ancestors: Vec<InlineAncestor>,
    style: Style,
    text: String,
    x: f64,
    width: f64,
}

/// Flatten collapsed text nodes into words and single spaces.
///
/// A space belongs to the item it came from, which is what keeps `a    b` one
/// box reading `a b` while leaving `<i>a</i> <b>c</b>` with a space box of its
/// own between the two elements.
fn pieces(items: &[Item]) -> Vec<Piece> {
    let space = |item: &Item| Piece {
        ancestors: item.ancestors.clone(),
        style: item.style.clone(),
        text: " ".to_owned(),
        is_space: true,
    };
    let mut out = Vec::new();
    for item in items {
        let words: Vec<&str> = item
            .text
            .split(' ')
            .filter(|word| !word.is_empty())
            .collect();
        if words.is_empty() {
            out.push(space(item));
            continue;
        }
        if item.text.starts_with(' ') {
            out.push(space(item));
        }
        for (index, word) in words.iter().enumerate() {
            if index > 0 {
                out.push(space(item));
            }
            out.push(Piece {
                ancestors: item.ancestors.clone(),
                style: item.style.clone(),
                text: (*word).to_owned(),
                is_space: false,
            });
        }
        if item.text.ends_with(' ') {
            out.push(space(item));
        }
    }
    out
}

/// Break the pieces into lines.
///
/// A line takes words until the next one would not fit, at which point the space
/// that would have separated them is dropped, as is a space at the start of a
/// line and a space at the end of one. That is what the reference does, and it
/// is why `hello world` in a 40px block becomes two boxes reading `hello` and
/// `world` with no space between them.
fn shape_lines(items: &[Item], width: f64, tag: &str) -> Vec<Fragment> {
    let mut lines: Vec<Fragment> = Vec::new();
    let mut current: Vec<Piece> = Vec::new();
    let mut x = 0.0_f64;
    let mut pending_space: Option<Piece> = None;

    for piece in pieces(items) {
        if piece.is_space {
            if !current.is_empty() {
                pending_space = Some(piece);
            }
            continue;
        }
        let word_width = advance(&piece.text, piece.style.font_size);
        let space_width = pending_space
            .as_ref()
            .map_or(0.0, |space| advance(" ", space.style.font_size));
        if !current.is_empty() && x + space_width + word_width > width {
            if let Some(line) = build_line(std::mem::take(&mut current), tag) {
                lines.push(line);
            }
            x = 0.0;
            pending_space = None;
        }
        if let Some(space) = pending_space.take()
            && !current.is_empty()
        {
            x += space_width;
            current.push(space);
        }
        x += word_width;
        current.push(piece);
    }
    if let Some(line) = build_line(current, tag) {
        lines.push(line);
    }
    lines
}

/// Merge adjacent pieces that share their inline ancestry into one text box,
/// then give them x positions.
fn place_pieces(pieces: Vec<Piece>) -> Vec<Placed> {
    let mut out: Vec<Placed> = Vec::new();
    for piece in pieces {
        let width = advance(&piece.text, piece.style.font_size);
        match out.last_mut() {
            Some(last)
                if last.ancestors.len() == piece.ancestors.len()
                    && last.style == piece.style
                    && last
                        .ancestors
                        .iter()
                        .zip(&piece.ancestors)
                        .all(|(left, right)| left.id == right.id) =>
            {
                last.text.push_str(&piece.text);
                last.width += width;
            }
            _ => out.push(Placed {
                x: 0.0,
                ancestors: piece.ancestors,
                style: piece.style,
                text: piece.text,
                width,
            }),
        }
    }
    let mut x = 0.0;
    for placed in &mut out {
        placed.x = x;
        x += placed.width;
    }
    out
}

/// Turn one line's pieces into a line box.
fn build_line(pieces: Vec<Piece>, tag: &str) -> Option<Fragment> {
    let items = place_pieces(pieces);
    if items.is_empty() {
        return None;
    }

    // The baseline is as far down as the deepest box on the line reaches.
    let baseline = items
        .iter()
        .map(|item| child_baseline(&item.style))
        .fold(f64::NEG_INFINITY, f64::max);
    let top = items
        .iter()
        .map(|item| baseline - child_baseline(&item.style))
        .fold(f64::INFINITY, f64::min);
    let bottom = items
        .iter()
        .map(|item| baseline - child_baseline(&item.style) + item.style.used_line_height())
        .fold(f64::NEG_INFINITY, f64::max);
    let height = bottom - top;

    let line_width: f64 = items.iter().map(|item| item.width).sum();
    let children = nest(&items, 0, baseline - top, tag);

    Some(Fragment {
        kind: BoxKind::Line,
        tag: tag.to_owned(),
        x_offset: 0.0,
        y_offset: 0.0,
        width: line_width,
        height,
        margin: Sides::default(),
        padding: Sides::default(),
        border: Sides::default(),
        text: None,
        effective_margin_top: 0.0,
        effective_margin_bottom: 0.0,
        inner_top: 0.0,
        inner_bottom: 0.0,
        children,
    })
}

/// Group a line's boxes into inline boxes, returning the outermost ones.
///
/// `line_baseline` is the baseline measured from the line's content-box top, so
/// a box's content-box top is `line_baseline - ascent(font-size)`. A child's
/// offset is made relative to the box it goes into, so that placing the tree
/// top-down adds each level exactly once.
fn nest(items: &[Placed], depth: usize, line_baseline: f64, block_tag: &str) -> Vec<Fragment> {
    let mut out = Vec::new();
    let mut index = 0;
    while index < items.len() {
        if items[index].ancestors.len() <= depth {
            out.push(text_box(&items[index], line_baseline, block_tag));
            index += 1;
            continue;
        }
        let ancestor = items[index].ancestors[depth].clone();
        let mut end = index;
        while end < items.len()
            && items[end].ancestors.len() > depth
            && items[end].ancestors[depth].id == ancestor.id
        {
            end += 1;
        }
        let mut children = nest(&items[index..end], depth + 1, line_baseline, block_tag);
        let left = children.first().map_or(0.0, |child| child.x_offset);
        let right = children
            .last()
            .map_or(left, |child| child.x_offset + child.width);
        for child in &mut children {
            child.x_offset -= left;
        }
        out.push(Fragment {
            kind: BoxKind::Inline,
            tag: ancestor.tag,
            x_offset: left,
            y_offset: line_baseline - ascent(&ancestor.style),
            width: right - left,
            height: ancestor.style.font_size,
            margin: half_leading_sides(&ancestor.style),
            padding: Sides::default(),
            border: Sides::default(),
            text: None,
            effective_margin_top: 0.0,
            effective_margin_bottom: 0.0,
            inner_top: 0.0,
            inner_bottom: 0.0,
            children,
        });
        index = end;
    }
    out
}

/// A text box on a line.
fn text_box(item: &Placed, line_baseline: f64, block_tag: &str) -> Fragment {
    let tag = item
        .ancestors
        .last()
        .map_or_else(|| block_tag.to_owned(), |ancestor| ancestor.tag.clone());
    Fragment {
        kind: BoxKind::Text,
        tag,
        x_offset: item.x,
        y_offset: line_baseline - ascent(&item.style),
        width: item.width,
        height: item.style.font_size,
        margin: half_leading_sides(&item.style),
        padding: Sides::default(),
        border: Sides::default(),
        text: Some(item.text.clone()),
        effective_margin_top: 0.0,
        effective_margin_bottom: 0.0,
        inner_top: 0.0,
        inner_bottom: 0.0,
        children: Vec::new(),
    }
}

/// The half-leading a box reports as its margins.
///
/// The reference sets a box's top and bottom margins to the half-leading, so
/// that the box's margin box is exactly its line-height tall while its content
/// box stays the font size. The fixture records both, so they are reproduced.
fn half_leading_sides(style: &Style) -> Sides {
    let half = (style.used_line_height() - style.font_size) / 2.0;
    Sides {
        top: half,
        right: 0.0,
        bottom: half,
        left: 0.0,
    }
}

/// The distance from a box's line-height top to its baseline, in pixels.
fn child_baseline(style: &Style) -> f64 {
    ascent(style) + (style.used_line_height() - style.font_size) / 2.0
}

/// The ascent of a font size, in pixels.
fn ascent(style: &Style) -> f64 {
    ASCENT * style.font_size
}

/// The width of a run of text, for a monospace font.
fn advance(text: &str, font_size: f64) -> f64 {
    text.chars().count() as f64 * ADVANCE_PER_GLYPH * font_size
}

/// Resolve `width` and the horizontal margins against a containing width.
///
/// This mirrors `block_level_width` in the reference's `layout/block.py`,
/// including the over-constrained case, where both margins keep the values they
/// were given.
///
/// Returns the content width, the offset from the containing block's content
/// left edge to this box's content box, and the two used margins.
fn resolve_horizontal(style: &Style, cb: f64) -> (f64, f64, f64, f64) {
    let padding_plus_border =
        style.padding.left + style.padding.right + style.border.left + style.border.right;
    let left_auto = style.margin[3] == Length::Auto;
    let right_auto = style.margin[1] == Length::Auto;
    let mut margin_left = if left_auto {
        0.0
    } else {
        resolve(style.margin[3], cb)
    };
    let mut margin_right = if right_auto {
        0.0
    } else {
        resolve(style.margin[1], cb)
    };

    let mut width = match style.width {
        Some(specified) => resolve(specified, cb),
        None => {
            let mut width = cb - padding_plus_border;
            if !left_auto {
                width -= margin_left;
            }
            if !right_auto {
                width -= margin_right;
            }
            width
        }
    };
    width = clamp_width(width, style, cb);

    // An `auto` margin becomes zero when the box would otherwise overflow.
    let mut margin_width = padding_plus_border + width;
    if !left_auto {
        margin_width += margin_left;
    }
    if !right_auto {
        margin_width += margin_right;
    }
    if margin_width > cb {
        if left_auto {
            margin_left = 0.0;
        }
        if right_auto {
            margin_right = 0.0;
        }
    }

    let margin_sum = cb - padding_plus_border - width;
    match (left_auto, right_auto) {
        (true, true) => {
            margin_left = margin_sum / 2.0;
            margin_right = margin_sum / 2.0;
        }
        (true, false) => margin_left = margin_sum - margin_right,
        (false, true) => margin_right = margin_sum - margin_left,
        // Over-constrained: both margins keep their specified values.
        (false, false) => {}
    }

    let x_offset = margin_left + style.border.left + style.padding.left;
    (width, x_offset, margin_left, margin_right)
}

/// Apply `min-width` and `max-width` to a used width.
fn clamp_width(width: f64, style: &Style, cw: f64) -> f64 {
    let mut width = width;
    if let Some(min) = style.min_width {
        width = width.max(resolve(min, cw));
    }
    if let Some(max) = style.max_width {
        width = width.min(resolve(max, cw));
    }
    width
}

/// Resolve a length to pixels against a containing width.
fn resolve(length: Length, cw: f64) -> f64 {
    match length {
        Length::Px(value) => value,
        Length::Percent(percent) => percent / 100.0 * cw,
        Length::Auto => 0.0,
    }
}

/// Resolve a margin, where `auto` on the vertical axis is zero.
fn resolve_margin(length: Length, cw: f64) -> f64 {
    match length {
        Length::Auto => 0.0,
        other => resolve(other, cw),
    }
}

/// Give a fragment and its subtree absolute coordinates.
fn place(fragment: &Fragment, parent_content_x: f64, parent_content_y: f64) -> LayoutBox {
    let content_x = parent_content_x + fragment.x_offset;
    let content_y = parent_content_y + fragment.y_offset;

    let children = fragment
        .children
        .iter()
        .map(|child| place(child, content_x, content_y))
        .collect();

    LayoutBox {
        tag: fragment.tag.clone(),
        kind: fragment.kind,
        content_x,
        content_y,
        width: fragment.width,
        height: fragment.height,
        margin: fragment.margin,
        padding: fragment.padding,
        border: fragment.border,
        text: fragment.text.clone(),
        children,
    }
}
