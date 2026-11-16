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
use crate::style::{Display, Length, Sides, Style, TextAlign};

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
    /// Whether this box's own top and bottom margins are adjoining, so that a
    /// margin passing through it merges with its own on both sides.
    collapses_through: bool,
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
    let padding = resolve_sides(&style.padding, containing_width);

    let inner_top = style.border.top + padding.top;
    let inner_bottom = style.border.bottom + padding.bottom;
    let own_margin_top = resolve_margin(style.margin[0], containing_width);
    let own_margin_bottom = resolve_margin(style.margin[2], containing_width);

    let mut children = layout_children(element, stylesheet, width, &style)?;

    let auto_height = style.height.is_none();
    let declared_height = style.height;

    if children.is_empty() {
        // An empty box with no border or padding and zero height collapses
        // *through*: its top and bottom margins are adjoining, so they merge
        // into one margin that applies on both sides. The box itself stays
        // where it is; the merged margin is what a sibling sees.
        let height = declared_height.unwrap_or(0.0);
        let through = !is_root && height == 0.0 && inner_top == 0.0 && inner_bottom == 0.0;
        let (effective_margin_top, effective_margin_bottom) = if through {
            let merged = collapse_margins(&[own_margin_top, own_margin_bottom]);
            (merged, merged)
        } else {
            (own_margin_top, own_margin_bottom)
        };
        return Ok(Fragment {
            kind: BoxKind::Block,
            tag: element.tag.clone(),
            x_offset,
            y_offset: inner_top,
            width,
            height,
            margin: Sides {
                top: own_margin_top,
                right: margin_right,
                bottom: own_margin_bottom,
                left: margin_left,
            },
            padding,
            border: style.border,
            text: None,
            effective_margin_top,
            effective_margin_bottom,
            collapses_through: through,
            inner_top,
            inner_bottom,
            children,
        });
    }

    let first_effective_top = children[0].effective_margin_top;
    let last_effective_bottom = children.last().expect("not empty").effective_margin_bottom;
    // A box's top margin is adjoining its first child's top margin when nothing
    // separates them, which is a property of *this* box only: its own border and
    // padding. The child's border and padding sit below its margin, so they do
    // not stop the two margins from collapsing, and a child whose own margin
    // collapsed out of it still presents that margin here. A line box carries no
    // margin, so it never collapses; the parent's own margin is what applies. The
    // same holds for an anonymous block, which is built with zero margins.
    //
    // The bottom edge differs: a declared height stops the last child's bottom
    // margin from collapsing out, because the content box has a fixed bottom.
    // The top edge has no such condition - a declared height says nothing about
    // the top.
    let collapses_top = !is_root && inner_top == 0.0;
    let collapses_bottom = !is_root && auto_height && inner_bottom == 0.0;

    // Walk the children, leaving each the gap its margin requires. A first child
    // whose margin collapsed out of this box contributes no gap, because the
    // collapsed margin has already moved this box itself.
    let mut cursor = 0.0_f64;
    let mut previous_bottom = 0.0_f64;
    // The bottom border edge of the last child that did not collapse through.
    // The reference only advances its position for such a child, so a trailing
    // run of collapsed-through boxes adds no height to this one.
    let mut content_bottom = 0.0_f64;
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
        if !child.collapses_through {
            content_bottom = cursor;
        }
        previous_bottom = child.effective_margin_bottom;
    }

    let content_height = if collapses_bottom {
        content_bottom
    } else {
        cursor + previous_bottom
    };

    // A box whose own top and bottom margins meet - no border, no padding and
    // zero content height - collapses through: a margin passing into it merges
    // with its own and escapes the other side.
    let collapses_through = collapses_top
        && collapses_bottom
        && content_height == 0.0
        && declared_height.unwrap_or(0.0) == 0.0;

    // A first or last child that collapses through presents both of its margins
    // to this box, so the run includes the far one too.
    let effective_margin_top = if collapses_top {
        let mut margin = own_margin_top.max(first_effective_top);
        if children[0].collapses_through {
            margin = margin.max(children[0].effective_margin_bottom);
        }
        margin
    } else {
        own_margin_top
    };
    let effective_margin_bottom = if collapses_bottom {
        let last = children.last().expect("not empty");
        let mut margin = own_margin_bottom.max(last_effective_bottom);
        if last.collapses_through {
            margin = margin.max(last.effective_margin_top);
        }
        margin
    } else {
        own_margin_bottom
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
        padding,
        border: style.border,
        text: None,
        effective_margin_top,
        effective_margin_bottom,
        collapses_through,
        inner_top,
        inner_bottom,
        children,
    })
}

/// Collapse a set of adjoining margins: the largest positive plus the most
/// negative, which is the reference's `collapse_margin`.
fn collapse_margins(margins: &[f64]) -> f64 {
    let positive = margins.iter().copied().fold(0.0_f64, f64::max);
    let negative = margins.iter().copied().fold(0.0_f64, f64::min);
    positive + negative
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
    // Any block-level child makes the element a block container, and then *every*
    // run of inline content it holds is wrapped in an anonymous block - the
    // leading run included, not only the ones between or after blocks. An element
    // with no block child keeps its line boxes as its own children.
    let mut has_block = false;
    for node in &element.children {
        if let Node::Element(child) = node {
            let child_style = crate::style::compute(child, stylesheet, Some(style))?;
            if child_style.display == Display::Block {
                has_block = true;
                break;
            }
        }
    }

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
                out.extend(layout_run(&run, element, stylesheet, width, style, !has_block)?);
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
    out.extend(layout_run(&run, element, stylesheet, width, style, !has_block)?);
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
    let align = parent_style.text_align;
    let align_last = parent_style.text_align_last.unwrap_or(align);
    let indent = resolve(parent_style.text_indent, width);
    let mut lines = shape_lines(
        &items,
        width,
        &element.tag,
        align,
        align_last,
        indent,
        parent_style,
    );
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
        collapses_through: false,
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

/// The empty boxes a line break leaves at the end of a line.
///
/// A break drops the space that would have separated the two lines, and the
/// reference keeps the box that space was in when the line has nothing else in
/// it. A space in the same box as the line's last content simply goes away. A
/// space in a box the last content sits deeper inside leaves nothing, because
/// that box is still on the line. A space at the block level, with the last
/// content inside an inline box, leaves an empty text box for the block. A space
/// in an inline box the last content is not inside leaves that box, and the
/// boxes between them, nested and empty. `None` means no box is left.
fn break_boundary(current: &[Piece], space: &Piece) -> Option<(Vec<InlineAncestor>, usize)> {
    let last = current.last()?;
    let space_path = &space.ancestors;
    let common = last
        .ancestors
        .iter()
        .zip(space_path)
        .take_while(|(left, right)| left.id == right.id)
        .count();
    if common == space_path.len() {
        // The space's box holds the last content, so that box stays on the line
        // whether or not the space was in it. Nothing is left empty.
        None
    } else {
        Some((space_path[common..].to_vec(), common))
    }
}

/// A line's content, the indent applied to it, and the empty inline chain a break
/// left behind with the depth in the line where that chain attaches. The chain is
/// `None` when no break ended the line.
type PendingLine = (Vec<Piece>, f64, Option<(Vec<InlineAncestor>, usize)>);

/// Break the pieces into lines.
///
/// A line takes words until the next one would not fit, at which point the space
/// that would have separated them is dropped, as is a space at the start of a
/// line and a space at the end of one. That is what the reference does, and it
/// is why `hello world` in a 40px block becomes two boxes reading `hello` and
/// `world` with no space between them.
fn shape_lines(
    items: &[Item],
    width: f64,
    tag: &str,
    align: TextAlign,
    align_last: TextAlign,
    indent: f64,
    block_style: &Style,
) -> Vec<Fragment> {
    let mut lines: Vec<PendingLine> = Vec::new();
    let mut current: Vec<Piece> = Vec::new();
    let mut x = 0.0_f64;
    let mut pending_space: Option<Piece> = None;
    // `text-indent` shifts the first line's content, so that line wraps against a
    // narrower width; every later line starts at the block's edge.
    let mut leading = indent;

    // Group the pieces into words first. A word is a maximal run of pieces with
    // no space between them, however many inline elements that run crosses:
    // `ab<span>cd</span>` is one word, because a break is only allowed at a
    // space and there is none between `ab` and `cd`. Each word carries the space
    // that preceded it, which is where a break before that word would fall.
    let mut words: Vec<(Vec<Piece>, f64, Option<Piece>)> = Vec::new();
    let mut word: Vec<Piece> = Vec::new();
    let mut word_width = 0.0_f64;
    for piece in pieces(items) {
        if piece.is_space {
            if !word.is_empty() {
                let width = word_width;
                words.push((std::mem::take(&mut word), width, pending_space.take()));
                word_width = 0.0;
            }
            pending_space = Some(piece);
            continue;
        }
        word_width += advance(&piece.text, piece.style.font_size);
        word.push(piece);
    }
    if !word.is_empty() {
        words.push((word, word_width, pending_space.take()));
    }

    for (word, word_width, leading_space) in words {
        let available = width - leading;
        let space_width = leading_space
            .as_ref()
            .map_or(0.0, |space| advance(" ", space.style.font_size));
        // The first word of a line is taken even when it does not fit, because
        // there is nowhere earlier to break.
        if let Some(space) = &leading_space
            && !current.is_empty()
            && x + space_width + word_width > available
        {
            // The break drops the space. When the space was the only content its
            // box had on this line, the reference leaves that box behind empty.
            let boundary = break_boundary(&current, space);
            lines.push((std::mem::take(&mut current), leading, boundary));
            x = 0.0;
            leading = 0.0;
        }
        if let Some(space) = leading_space
            && !current.is_empty()
        {
            x += space_width;
            current.push(space);
        }
        x += word_width;
        current.extend(word);
    }
    if !current.is_empty() {
        lines.push((current, leading, None));
    }

    // The last line is aligned by `text-align-last`, every other line by
    // `text-align`, which is what the reference does.
    let count = lines.len();
    lines
        .into_iter()
        .enumerate()
        .filter_map(|(index, (line, leading, boundary))| {
            let last = index + 1 == count;
            build_line(
                line,
                tag,
                width,
                if last { align_last } else { align },
                leading,
                block_style,
                boundary,
            )
        })
        .collect()
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

/// Turn one line's pieces into a line box, aligned within `available`.
///
/// `indent` is the `text-indent` that applies to this line, which is non-zero
/// only for the first line of a block. The reference measures the line box from
/// the block's edge to the end of the content, so the indent widens the line box
/// and shifts its content, and the alignment offset is taken against that wider
/// box.
fn build_line(
    pieces: Vec<Piece>,
    tag: &str,
    available: f64,
    align: TextAlign,
    indent: f64,
    block_style: &Style,
    boundary: Option<(Vec<InlineAncestor>, usize)>,
) -> Option<Fragment> {
    let mut items = place_pieces(pieces);
    if items.is_empty() && boundary.is_none() {
        return None;
    }

    let content_width: f64 = items.iter().map(|item| item.width).sum();
    // WeasyPrint leaves a line alone once it already fills the line box, and
    // `justify` never touches the last line (that is `text-align-last`'s job).
    let line_box_width = indent + content_width;
    let offset = if line_box_width >= available {
        0.0
    } else {
        let gap = available - line_box_width;
        match align {
            TextAlign::Start | TextAlign::Left | TextAlign::Justify => 0.0,
            TextAlign::Center => gap / 2.0,
            TextAlign::End | TextAlign::Right => gap,
        }
    };
    if align == TextAlign::Justify && line_box_width < available {
        justify(&mut items, available - line_box_width);
    }

    // The line box has the block's own font as its "strut": a zero-width box on
    // the baseline, so a line whose inline content is smaller or larger than the
    // block still gets the block's line height. Every box on the line is
    // baseline-aligned, so with the baseline at zero a box of style `s` spans
    // `-baseline(s)` to `-baseline(s) + line-height(s)`; the line box is the
    // union of those spans, the strut included.
    let strut_baseline = child_baseline(block_style);
    let mut min_y = -strut_baseline;
    let mut max_y = -strut_baseline + block_style.used_line_height();
    // Every box on the line contributes, including the inline elements a run of
    // text sits inside: an empty wrapper with a larger font still raises the
    // line, because its own box is on the line even though its text is not.
    for item in &items {
        for style in item
            .ancestors
            .iter()
            .map(|ancestor| &ancestor.style)
            .chain(std::iter::once(&item.style))
        {
            let baseline = child_baseline(style);
            min_y = min_y.min(-baseline);
            max_y = max_y.max(-baseline + style.used_line_height());
        }
    }
    if let Some((chain, _)) = &boundary {
        for ancestor in chain {
            let baseline = child_baseline(&ancestor.style);
            min_y = min_y.min(-baseline);
            max_y = max_y.max(-baseline + ancestor.style.used_line_height());
        }
    }
    let baseline = -min_y;
    let height = max_y - min_y;

    let content_width: f64 = items.iter().map(|item| item.width).sum();
    let mut children = nest(
        &items,
        0,
        baseline,
        tag,
        boundary
            .as_ref()
            .map(|(chain, attach)| (chain.as_slice(), *attach)),
        content_width,
    );
    // The indent shifts the line's content, not the line box itself.
    for child in &mut children {
        child.x_offset += indent;
    }

    Some(Fragment {
        kind: BoxKind::Line,
        tag: tag.to_owned(),
        x_offset: offset,
        y_offset: 0.0,
        width: indent + content_width,
        height,
        margin: Sides::default(),
        padding: Sides::default(),
        border: Sides::default(),
        text: None,
        effective_margin_top: 0.0,
        effective_margin_bottom: 0.0,
        collapses_through: false,
        inner_top: 0.0,
        inner_bottom: 0.0,
        children,
    })
}

/// Distribute `extra` across the expandable spaces of a justified line.
///
/// This mirrors `justify_line` in the reference: the extra width is split evenly
/// over the spaces, and each space widens its own box, so a line that wraps is
/// only stretched on the lines that are not last.
fn justify(items: &mut [Placed], extra: f64) {
    if extra <= 0.0 {
        return;
    }
    let spaces: usize = items
        .iter()
        .map(|item| item.text.matches(' ').count())
        .sum();
    if spaces == 0 {
        return;
    }
    let per_space = extra / spaces as f64;
    let mut shift = 0.0;
    for item in items.iter_mut() {
        item.x += shift;
        let count = item.text.matches(' ').count();
        if count > 0 {
            let added = per_space * count as f64;
            item.width += added;
            shift += added;
        }
    }
}

/// Group a line's boxes into inline boxes, returning the outermost ones.
///
/// `line_baseline` is the baseline measured from the line's content-box top, so
/// a box's content-box top is `line_baseline - ascent(font-size)`. A child's
/// offset is made relative to the box it goes into, so that placing the tree
/// top-down adds each level exactly once.
///
/// `tail` is the chain of boxes a line break left empty, with the depth it
/// belongs at: the number of boxes the line's last content and the dropped space
/// share. The empty boxes go inside that many levels, because the reference
/// keeps them where the space sat rather than beside the line's content.
/// `end_x` is the line's content end, from the line's left edge, which is where
/// the outermost empty box goes.
fn nest(
    items: &[Placed],
    depth: usize,
    line_baseline: f64,
    block_tag: &str,
    tail: Option<(&[InlineAncestor], usize)>,
    end_x: f64,
) -> Vec<Fragment> {
    // Split the items into runs that share the ancestor at this depth.
    struct Group {
        ancestor: Option<InlineAncestor>,
        start: usize,
        end: usize,
    }
    let mut groups: Vec<Group> = Vec::new();
    let mut index = 0;
    while index < items.len() {
        if items[index].ancestors.len() <= depth {
            groups.push(Group {
                ancestor: None,
                start: index,
                end: index + 1,
            });
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
        groups.push(Group {
            ancestor: Some(ancestor),
            start: index,
            end,
        });
        index = end;
    }

    let mut out = Vec::new();
    for group in &groups {
        let Some(ancestor) = &group.ancestor else {
            out.push(text_box(&items[group.start], line_baseline, block_tag));
            continue;
        };
        // The tail travels down the run that holds the line's last content,
        // because that is the box the empty chain belongs inside.
        let is_last_run = group.end == items.len();
        let inner_tail = tail.filter(|(_, attach)| is_last_run && *attach > depth);
        let mut children = nest(
            &items[group.start..group.end],
            depth + 1,
            line_baseline,
            block_tag,
            inner_tail,
            end_x,
        );
        let left = children.first().map_or(0.0, |child| child.x_offset);
        let right = children
            .last()
            .map_or(left, |child| child.x_offset + child.width);
        // A child's offset is relative to the box it goes into, so the inline
        // box's own offset comes off both axes. The x offsets were laid out from
        // the line's left edge; the y offsets are measured from the line's top.
        let top = line_baseline - ascent(&ancestor.style);
        for child in &mut children {
            child.x_offset -= left;
            child.y_offset -= top;
        }
        out.push(Fragment {
            kind: BoxKind::Inline,
            tag: ancestor.tag.clone(),
            x_offset: left,
            y_offset: top,
            width: right - left,
            height: ancestor.style.font_size,
            margin: half_leading_sides(&ancestor.style),
            padding: Sides::default(),
            border: Sides::default(),
            text: None,
            effective_margin_top: 0.0,
            effective_margin_bottom: 0.0,
            collapses_through: false,
            inner_top: 0.0,
            inner_bottom: 0.0,
            children,
        });
    }

    // The tail's depth is this one, so its boxes are siblings of the boxes here.
    if let Some((chain, attach)) = tail
        && attach == depth
    {
        out.extend(empty_boxes(chain, end_x, line_baseline, block_tag, true));
    }
    out
}

/// Build the empty boxes a line break leaves at the end of a line.
///
/// An empty chain is the block itself, which the reference leaves as an empty
/// text box for the block. A non-empty chain nests, so each box wraps the next
/// and only the outermost is placed; the innermost holds nothing, and the boxes
/// inside sit at the origin of the box that holds them.
fn empty_boxes(
    chain: &[InlineAncestor],
    x: f64,
    line_baseline: f64,
    block_tag: &str,
    outer: bool,
) -> Vec<Fragment> {
    let Some(ancestor) = chain.first() else {
        return vec![Fragment {
            kind: BoxKind::Text,
            tag: block_tag.to_owned(),
            x_offset: x,
            y_offset: 0.0,
            width: 0.0,
            height: 0.0,
            margin: Sides::default(),
            padding: Sides::default(),
            border: Sides::default(),
            text: Some(String::new()),
            effective_margin_top: 0.0,
            effective_margin_bottom: 0.0,
            collapses_through: false,
            inner_top: 0.0,
            inner_bottom: 0.0,
            children: Vec::new(),
        }];
    };
    let children = match chain.get(1..) {
        Some(rest) if !rest.is_empty() => empty_boxes(rest, 0.0, line_baseline, block_tag, false),
        _ => Vec::new(),
    };
    vec![Fragment {
        kind: BoxKind::Inline,
        tag: ancestor.tag.clone(),
        x_offset: x,
        y_offset: if outer {
            line_baseline - ascent(&ancestor.style)
        } else {
            0.0
        },
        width: 0.0,
        height: ancestor.style.font_size,
        margin: half_leading_sides(&ancestor.style),
        padding: Sides::default(),
        border: Sides::default(),
        text: None,
        effective_margin_top: 0.0,
        effective_margin_bottom: 0.0,
        collapses_through: false,
        inner_top: 0.0,
        inner_bottom: 0.0,
        children,
    }]
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
        collapses_through: false,
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
    let padding = resolve_sides(&style.padding, cb);
    let padding_plus_border = padding.left + padding.right + style.border.left + style.border.right;
    let mut left_auto = style.margin[3] == Length::Auto;
    let mut right_auto = style.margin[1] == Length::Auto;
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

    // An `auto` margin becomes zero when the box would otherwise overflow, and
    // it stops being `auto`: the reference zeroes it before it distributes the
    // remaining space, so an over-constrained box keeps zero margins rather than
    // a negative one.
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
            left_auto = false;
        }
        if right_auto {
            margin_right = 0.0;
            right_auto = false;
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

    let x_offset = margin_left + style.border.left + padding.left;
    (width, x_offset, margin_left, margin_right)
}

/// Resolve a padding shorthand, whose percentages use the containing block's
/// width on every side - the vertical ones too, as CSS 2.1 specifies.
fn resolve_sides(sides: &[Length; 4], cb: f64) -> Sides {
    Sides {
        top: resolve(sides[0], cb),
        right: resolve(sides[1], cb),
        bottom: resolve(sides[2], cb),
        left: resolve(sides[3], cb),
    }
}

/// Apply `min-width` and `max-width` to a used width.
///
/// `min-width`'s initial value is zero, so the used width is floored at zero
/// even when neither property is set: a box whose padding alone exceeds its
/// containing block is zero wide, not negative. The reference computes
/// `max(min, min(max, width))`, so `min-width` is applied last and wins when it
/// is larger than `max-width`.
fn clamp_width(width: f64, style: &Style, cw: f64) -> f64 {
    let mut width = width;
    if let Some(max) = style.max_width {
        width = width.min(resolve(max, cw));
    }
    let min = style.min_width.map_or(0.0, |min| resolve(min, cw));
    width.max(min)
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
