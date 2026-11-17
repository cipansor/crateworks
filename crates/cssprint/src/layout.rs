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
    /// Whether this box's own top and bottom margins are adjoining, so that a
    /// margin passing through it merges with its own on both sides.
    collapses_through: bool,
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
    let (html, _, _) = layout_element(
        dom,
        stylesheet,
        page_width,
        true,
        None,
        Pos {
            cursor_y: 0.0,
            parent_content_y: 0.0,
        },
        fresh_margins(),
    )?;
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

/// A run of adjoining margins, shared between a box and its collapsing
/// descendants.
///
/// The reference passes one list down the tree and every box appends its own top
/// margin to it; a box that collapses through leaves its margins in the list for
/// the next sibling, and a box that stops the collapse starts a fresh list.
/// Sharing the list rather than copying it is what makes a chain of
/// collapsed-through boxes share one collapsed margin: a parent's own top margin
/// is the collapse of every margin that ended up adjoining inside it.
type Margins = std::rc::Rc<std::cell::RefCell<Vec<f64>>>;

fn fresh_margins() -> Margins {
    std::rc::Rc::new(std::cell::RefCell::new(Vec::new()))
}

fn collapsed(run: &Margins) -> f64 {
    collapse_margins(&run.borrow())
}

/// Where a box is being laid out: the parent's running bottom edge, and the
/// absolute y of the parent's content box, which its own offset is measured
/// against.
#[derive(Clone, Copy)]
struct Pos {
    cursor_y: f64,
    parent_content_y: f64,
}

/// Lay out one block-level element and its subtree.
///
/// `is_root` marks the `html` element, which neither collapses with the page nor
/// with its own first child in the way an ordinary box does.
///
/// Vertical placement follows `block_container_layout` in the reference. The box
/// joins the run of adjoining margins it was handed, places itself at the
/// collapsed value of that run, and either stops the run - when a border or
/// padding separates its margin from its content - or passes it on to its
/// children. A box that collapses through keeps the run alive for its next
/// sibling, so a row of empty boxes occupies one collapsed margin rather than
/// one margin each.
///
/// Returns the box, the absolute y of its content box, and the run it leaves
/// behind for its next sibling.
fn layout_element(
    element: &Element,
    stylesheet: &crate::Stylesheet,
    containing_width: f64,
    is_root: bool,
    parent_style: Option<&Style>,
    at: Pos,
    adjoining: Margins,
) -> Result<(Fragment, f64, Margins), Error> {
    let Pos {
        cursor_y,
        parent_content_y,
    } = at;
    let style = crate::style::compute(element, stylesheet, parent_style)?;
    let (width, x_offset, margin_left, margin_right) = resolve_horizontal(&style, containing_width);
    let padding = resolve_sides(&style.padding, containing_width);
    let inner_top = style.border.top + padding.top;
    let inner_bottom = style.border.bottom + padding.bottom;
    let margin_top = resolve_margin(style.margin[0], containing_width);
    let margin_bottom = resolve_margin(style.margin[2], containing_width);

    let collapses_top = !is_root && inner_top == 0.0;
    let collapses_bottom = !is_root && inner_bottom == 0.0;

    adjoining.borrow_mut().push(margin_top);
    let this_run = std::rc::Rc::clone(&adjoining);

    let mut adjoining = adjoining;
    let mut box_y = cursor_y;
    // A box whose top margin collapses keeps the position it was handed and is
    // moved by the collapsed run later; otherwise it stops the run here.
    let cursor = if collapses_top {
        box_y
    } else {
        box_y += collapsed(&adjoining) - margin_top;
        adjoining = fresh_margins();
        box_y + margin_top + inner_top
    };
    let content_box_y = box_y + margin_top + inner_top;

    // Any block-level child makes this a block container, and then every run of
    // inline content it holds is wrapped in an anonymous block.
    let mut has_block = false;
    for node in &element.children {
        if let Node::Element(child) = node {
            let child_style = crate::style::compute(child, stylesheet, Some(&style))?;
            if child_style.display == Display::Block {
                has_block = true;
                break;
            }
        }
    }

    let ctx = RunCtx {
        element,
        stylesheet,
        width,
        style: &style,
        has_block,
        content_box_y,
    };
    let mut flow = Flow {
        cursor,
        adjoining,
        children: Vec::new(),
        has_in_flow: false,
    };
    let mut run: Vec<&Node> = Vec::new();

    for node in &element.children {
        let block_child = match node {
            Node::Text(_) => None,
            Node::Element(child) => {
                let child_style = crate::style::compute(child, stylesheet, Some(&style))?;
                match child_style.display {
                    Display::None => continue,
                    Display::Block => Some(child),
                    Display::Inline => None,
                }
            }
        };
        match block_child {
            Some(child) => {
                flush_run(&mut run, &ctx, &mut flow)?;
                let (mut frag, child_content_y, child_run) = layout_element(
                    child,
                    stylesheet,
                    width,
                    false,
                    Some(&style),
                    Pos {
                        cursor_y: flow.cursor,
                        parent_content_y: content_box_y,
                    },
                    std::rc::Rc::clone(&flow.adjoining),
                )?;
                flow.adjoining = child_run;
                frag.y_offset = child_content_y - content_box_y;
                if !frag.collapses_through {
                    flow.cursor = child_content_y + frag.height + frag.inner_bottom;
                }
                flow.adjoining.borrow_mut().push(frag.margin.bottom);
                flow.children.push(frag);
                flow.has_in_flow = true;
            }
            None => run.push(node),
        }
    }
    flush_run(&mut run, &ctx, &mut flow)?;

    let Flow {
        mut cursor,
        mut adjoining,
        mut children,
        has_in_flow,
    } = flow;

    // With no in-flow child the box's own top and bottom margins are adjoining,
    // so a zero-height box with no border or padding collapses through and its
    // margins join the run for the next sibling.
    let mut collapses_through = false;
    if !has_in_flow {
        let margin = collapsed(&adjoining);
        if style.height.unwrap_or(0.0) == 0.0 && inner_top == 0.0 && inner_bottom == 0.0 {
            collapses_through = true;
        } else {
            cursor += margin;
            adjoining = fresh_margins();
        }
    } else if style.height.is_some() {
        // A declared height gives the content box a fixed bottom, so the last
        // child's bottom margin cannot escape.
        adjoining = fresh_margins();
    }
    if !collapses_bottom {
        cursor += collapsed(&adjoining);
        adjoining = fresh_margins();
    }

    // A box whose top margin collapses is moved by the collapsed run it joined,
    // but that move is the box's alone: the children were placed against the
    // position the box had before the move, so their offsets come back by the
    // same amount.
    if collapses_top {
        let delta = collapsed(&this_run) - margin_top;
        box_y += delta;
        for child in &mut children {
            child.y_offset -= delta;
        }
    }
    let content_box_y = box_y + margin_top + inner_top;
    let height = style
        .height
        .unwrap_or_else(|| (cursor - content_box_y).max(0.0));

    let fragment = Fragment {
        kind: BoxKind::Block,
        tag: element.tag.clone(),
        x_offset,
        y_offset: content_box_y - parent_content_y,
        width,
        height,
        margin: Sides {
            top: margin_top,
            right: margin_right,
            bottom: margin_bottom,
            left: margin_left,
        },
        padding,
        border: style.border,
        text: None,
        collapses_through,
        inner_bottom,
        children,
    };
    Ok((fragment, content_box_y, adjoining))
}

/// Collapse a set of adjoining margins: the largest positive plus the most
/// negative, which is the reference's `collapse_margin`.
fn collapse_margins(margins: &[f64]) -> f64 {
    let positive = margins.iter().copied().fold(0.0_f64, f64::max);
    let negative = margins.iter().copied().fold(0.0_f64, f64::min);
    positive + negative
}

/// The immutable context a box lays its runs out in.
struct RunCtx<'a> {
    element: &'a Element,
    stylesheet: &'a crate::Stylesheet,
    width: f64,
    style: &'a Style,
    has_block: bool,
    content_box_y: f64,
}

/// The state a box accumulates while it places its children: its running bottom
/// edge, the adjoining-margin run it hands on, the children so far, and whether
/// any of them is in flow.
struct Flow {
    cursor: f64,
    adjoining: Margins,
    children: Vec<Fragment>,
    has_in_flow: bool,
}

/// Lay out one run of inline content and append its line boxes to the flow.
///
/// `has_block` says the element also holds a block-level child, so the run is
/// wrapped in an anonymous block; otherwise its line boxes are the element's own
/// children. Either way the run ends the adjoining-margin chain, because a line
/// box carries no margin to collapse.
fn flush_run(run: &mut Vec<&Node>, ctx: &RunCtx, flow: &mut Flow) -> Result<(), Error> {
    let RunCtx {
        element,
        stylesheet,
        width,
        style,
        has_block,
        content_box_y,
    } = *ctx;
    if run.is_empty() {
        return Ok(());
    }
    let items = collect_items(run, stylesheet, style)?;
    let align = style.text_align;
    let align_last = style.text_align_last.unwrap_or(align);
    let indent = resolve(style.text_indent, width);
    let lines = shape_lines(
        &items,
        width,
        &element.tag,
        align,
        align_last,
        indent,
        style,
    );
    run.clear();
    if lines.is_empty() {
        return Ok(());
    }

    if !flow.adjoining.borrow().is_empty() {
        flow.cursor += collapsed(&flow.adjoining);
    }
    flow.adjoining = fresh_margins();

    if has_block {
        let mut height = 0.0;
        let mut inner = lines;
        for line in &mut inner {
            line.y_offset = height;
            height += line.height;
        }
        flow.children.push(Fragment {
            kind: BoxKind::Block,
            tag: element.tag.clone(),
            x_offset: 0.0,
            y_offset: flow.cursor - content_box_y,
            width,
            height,
            margin: Sides::default(),
            padding: Sides::default(),
            border: Sides::default(),
            text: None,
            collapses_through: false,
            inner_bottom: 0.0,
            children: inner,
        });
        flow.cursor += height;
        // The anonymous block is a block with zero margins, so it leaves a zero
        // in the run for its next sibling.
        flow.adjoining.borrow_mut().push(0.0);
    } else {
        for mut line in lines {
            line.y_offset = flow.cursor - content_box_y;
            flow.cursor += line.height;
            flow.children.push(line);
        }
    }
    flow.has_in_flow = true;
    Ok(())
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
///
/// A word with a break opportunity inside it (`foo-bar`) is split into several
/// pieces, one per unit, so the line breaker can choose between them.
struct Piece {
    ancestors: Vec<InlineAncestor>,
    style: Style,
    text: String,
    is_space: bool,
    /// Whether this space is the whole of its text node (`<span>a</span> <span>b</span>`),
    /// as opposed to a space inside a node that holds other text (`a b`). The
    /// reference strips a dropped space from its text box and leaves the box in
    /// place; only a node that was nothing but that space is left as an empty
    /// text box, which is what `break_boundary` reads.
    lone: bool,
    /// Whether a line may break immediately before this piece with no space,
    /// because it follows a break opportunity inside a word (`foo-bar`).
    breakable: bool,
}

/// Whether two pieces sit in the same inline element, i.e. belong to the same
/// text box. The reference splits one text box at a time, so a decision about a
/// space must not see pieces from a sibling box on the same line.
fn same_ancestors(left: &[InlineAncestor], right: &[InlineAncestor]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .all(|(left, right)| left.id == right.id)
}

/// A piece placed on a line, with the x it ended up at.
struct Placed {
    ancestors: Vec<InlineAncestor>,
    style: Style,
    text: String,
    x: f64,
    width: f64,
}

/// A run of pieces a line may not be split inside, and where a break before it
/// would fall.
///
/// An atom is a whole word, however many inline elements it crosses
/// (`ab<span>cd</span>`), or one unit of a word that has a break opportunity
/// inside it (`foo-` and `bar`). The break before an atom is either at a space -
/// `space` holds it, and the break drops it - or inside a word, where `space` is
/// `None` and no character is dropped.
struct Atom {
    pieces: Vec<Piece>,
    width: f64,
    space: Option<Piece>,
}

/// Flatten collapsed text nodes into words and single spaces.
///
/// A space belongs to the item it came from, which is what keeps `a    b` one
/// box reading `a b` while leaving `<i>a</i> <b>c</b>` with a space box of its
/// own between the two elements.
fn pieces(items: &[Item]) -> Vec<Piece> {
    let space = |item: &Item, lone: bool| Piece {
        ancestors: item.ancestors.clone(),
        style: item.style.clone(),
        text: " ".to_owned(),
        is_space: true,
        lone,
        breakable: false,
    };
    let mut out = Vec::new();
    for item in items {
        let words: Vec<&str> = item
            .text
            .split(' ')
            .filter(|word| !word.is_empty())
            .collect();
        if words.is_empty() {
            // The node was nothing but whitespace, so the reference keeps its
            // text box and leaves it empty when the space is dropped.
            out.push(space(item, true));
            continue;
        }
        if item.text.starts_with(' ') {
            out.push(space(item, false));
        }
        for (index, word) in words.iter().enumerate() {
            if index > 0 {
                out.push(space(item, false));
            }
            for (unit_index, unit) in break_units(word).into_iter().enumerate() {
                // Every unit but the first begins after a break opportunity
                // inside the word, so a line may start there. The first unit of
                // an item's first word can also begin after a break opportunity
                // at a seam with the previous item, which is where two adjacent
                // text nodes meet with no space between them.
                let breakable = if unit_index > 0 {
                    true
                } else if index == 0 {
                    seam_breakable(out.last(), unit)
                } else {
                    false
                };
                out.push(Piece {
                    ancestors: item.ancestors.clone(),
                    style: item.style.clone(),
                    text: unit.to_owned(),
                    is_space: false,
                    lone: false,
                    breakable,
                });
            }
        }
        if item.text.ends_with(' ') {
            out.push(space(item, false));
        }
    }
    out
}

/// Whether a line may break between the piece before this one and the first
/// unit of a new text node.
///
/// Two adjacent text nodes with no space between them meet at a *seam*
/// (`3%<span>$(3</span>`), and the reference allows a break there exactly when
/// the two characters beside it are a break opportunity on their own - the same
/// rule that governs a break inside one word, but read from the two characters
/// alone, because the reference asks Pango about `last_letter + first_letter`
/// and nothing wider. `3/` followed by `4` therefore breaks at the seam even
/// though `3/4` inside one text node does not: the numeric rule needs the whole
/// expression, and the seam sees only two characters.
fn seam_breakable(previous: Option<&Piece>, unit: &str) -> bool {
    let Some(previous) = previous else {
        return false;
    };
    if previous.is_space {
        return false;
    }
    let (Some(before), Some(after)) = (previous.text.chars().next_back(), unit.chars().next())
    else {
        return false;
    };
    break_allowed(&[before, after], 1)
}

/// The break opportunities inside one word, as maximal runs that may not be
/// split further.
///
/// A break is allowed after certain punctuation even with no space, which the
/// reference takes from Unicode's line-breaking algorithm (UAX #14) through
/// Pango: `foo-bar` wraps as `foo-` / `bar` in a 64px block. The split is by
/// class pair, not by a fixed character list, because `a-b` breaks after the
/// hyphen while `3-4` does not - a hyphen between two numbers is part of the
/// number, not an opportunity.
///
/// The units are the substrings between the opportunities. `ab` yields one unit,
/// so a word with no punctuation stays indivisible; `foo-bar` yields `foo-` and
/// `bar`.
fn break_units(word: &str) -> Vec<&str> {
    let characters: Vec<char> = word.chars().collect();
    // The byte offset each character starts at, plus the word's length, so a
    // split at a character boundary can slice the original string.
    let mut offsets = Vec::with_capacity(characters.len() + 1);
    let mut offset = 0;
    for character in &characters {
        offsets.push(offset);
        offset += character.len_utf8();
    }
    offsets.push(offset);

    let mut units = Vec::new();
    let mut start = 0;
    for index in 1..characters.len() {
        if break_allowed(&characters, index) {
            units.push(&word[start..offsets[index]]);
            start = offsets[index];
        }
    }
    if start < word.len() {
        units.push(&word[start..]);
    }
    if units.is_empty() {
        units.push(word);
    }
    units
}

/// Whether a line may break before the character at `index`, which is at least
/// one character into `characters`.
///
/// The context-free part is the class-pair rule; the two rules that need more
/// than the adjacent characters are the word-initial hyphen and the numeric
/// expression, both of which the reference applies across the whole word.
fn break_allowed(characters: &[char], index: usize) -> bool {
    let before = characters[index - 1];
    let after = characters[index];
    // A word-initial hyphen does not break before a letter or a digit (UAX #14
    // LB20a): `-bar` and `-3` stay whole even though `a-bar` breaks after its
    // hyphen. Before anything else (`-$`, `-(`) the hyphen still breaks.
    if before == '-'
        && index == 1
        && matches!(line_class(after), Some(LineClass::Al | LineClass::Nu))
    {
        return false;
    }
    // A break inside a numeric expression is not allowed even though the pair
    // rule would permit it (UAX #14 LB25).
    if inside_numeric_expression(characters, index) {
        return false;
    }
    break_after(before, after)
}

/// Whether the boundary at `index` falls inside a numeric expression.
///
/// The pair rule decides every boundary from the two characters beside it, but
/// LB25 is not a pair rule: `3/4`, `$(3` and `(3)$` each contain a boundary the
/// pair rule would break at and the reference keeps whole. The expression is
/// recognised from the characters around the boundary, reading outwards:
///
/// * `NU (SY|IS)* (CL|CP)? x (PO|PR)` - a currency sign after a number, with an
///   optional closing bracket in between.
/// * `NU (SY|IS)* x NU` - a second digit, as in `3/4` or `1,000`.
/// * `NU (SY|IS)* x (PO|PR)` - a postfix percent after a number, as in `3%`.
/// * `(PO|PR) x (OP|HY)? (IS)? NU` - a numeric prefix before a number, as in
///   `$(3` or `+3`, with an optional opening bracket and separator in between.
///
/// The last rule is where the prefix sits before the boundary; the others read
/// the run before it. Together they reproduce Pango's table exactly over every
/// printable ASCII string up to length three and hundreds of thousands of
/// longer random ones.
fn inside_numeric_expression(characters: &[char], index: usize) -> bool {
    let before = characters[index - 1];
    let after = characters[index];
    if is_numeric_prefix_or_postfix(after) {
        // `NU (SY|IS)* (CL|CP)? x (PO|PR)`
        if is_close_bracket(before) && numeric_run_before(characters, index - 1) {
            return true;
        }
        // `NU (SY|IS)* x (PO|PR)`
        if numeric_run_before(characters, index) {
            return true;
        }
    }
    // `NU (SY|IS)* x NU`
    if after.is_ascii_digit() && numeric_run_before(characters, index) {
        return true;
    }
    // `(PO|PR) x (OP|HY)? (IS)? NU`
    if is_numeric_prefix_or_postfix(before) {
        let mut look = index;
        if look < characters.len() && (is_open_bracket(characters[look]) || characters[look] == '-')
        {
            look += 1;
        }
        if look < characters.len() && is_infix_separator(characters[look]) {
            look += 1;
        }
        if look < characters.len() && characters[look].is_ascii_digit() {
            return true;
        }
    }
    false
}

/// Whether `characters[..end]` ends with `NU (SY|IS)*`, so the boundary at
/// `end` follows a number that may still be continued.
fn numeric_run_before(characters: &[char], end: usize) -> bool {
    let mut start = end;
    while start > 0 && is_infix_or_symbol(characters[start - 1]) {
        start -= 1;
    }
    start > 0 && characters[start - 1].is_ascii_digit()
}

/// Whether `character` is a numeric prefix (`$`, `+`, `\`) or postfix (`%`).
fn is_numeric_prefix_or_postfix(character: char) -> bool {
    matches!(character, '$' | '+' | '\\' | '%')
}

/// Whether `character` is an infix separator (`,`, `.`, `:`, `;`).
fn is_infix_separator(character: char) -> bool {
    matches!(character, ',' | '.' | ':' | ';')
}

/// Whether `character` is a symbol that may sit between digits (`/`).
fn is_symbol(character: char) -> bool {
    character == '/'
}

/// Whether `character` may sit between two digits of one number.
fn is_infix_or_symbol(character: char) -> bool {
    is_infix_separator(character) || is_symbol(character)
}

/// Whether `character` is an opening bracket (`(`, `[`, `{`).
fn is_open_bracket(character: char) -> bool {
    matches!(character, '(' | '[' | '{')
}

/// Whether `character` is a closing bracket (`)`, `]`, `}`).
fn is_close_bracket(character: char) -> bool {
    matches!(character, ')' | ']' | '}')
}

/// Whether a line may break between two adjacent characters with no space.
///
/// This is the ASCII subset of UAX #14 that the reference exercises. Both
/// characters must be ASCII: the crate lays out one font and the reference's
/// line breaking outside ASCII is not something the corpus pins, so anything
/// else stays unbreakable rather than guessing.
///
/// The rule was read off the reference: every one of the 94 printable ASCII
/// characters was set against every other in `aaX Yaa` at a width of three
/// characters, and this predicate reproduces that table exactly.
fn break_after(before: char, after: char) -> bool {
    let (Some(before), Some(after)) = (line_class(before), line_class(after)) else {
        return false;
    };
    use LineClass::*;
    // Where the second character may not start a line.
    if matches!(after, Cl | Cp | Ex | Is | Sy | Qu | Ba | Hy) {
        return false;
    }
    // Where the first character may not end a line.
    if matches!(before, Al | Nu | Op | Qu) {
        return false;
    }
    // No break between a numeric prefix or postfix and the number it belongs to.
    if matches!(before, Pr | Po | Is | Cp) && matches!(after, Al | Nu) {
        return false;
    }
    // `3-4` is a number, not two.
    if before == Hy && after == Nu {
        return false;
    }
    true
}

/// The UAX #14 line-breaking class of an ASCII character.
///
/// Only the classes the pair rule above distinguishes are named; a character
/// that is not ASCII has no class, which keeps it out of the rule.
fn line_class(character: char) -> Option<LineClass> {
    use LineClass::*;
    Some(match character {
        '0'..='9' => Nu,
        '!' | '?' => Ex,
        '"' | '\'' => Qu,
        '$' | '+' | '\\' => Pr,
        '%' => Po,
        '(' | '[' | '{' => Op,
        ')' | ']' => Cp,
        '}' => Cl,
        ',' | '.' | ':' | ';' => Is,
        '/' => Sy,
        '-' => Hy,
        '|' => Ba,
        c if c.is_ascii() => Al,
        _ => return None,
    })
}

/// The line-breaking class of an ASCII character, for the pair rule.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LineClass {
    /// Letters, and the symbols with no special break behaviour.
    Al,
    /// Digits.
    Nu,
    /// Opening brackets.
    Op,
    /// Closing brackets.
    Cp,
    /// Closing punctuation.
    Cl,
    /// Quotation marks.
    Qu,
    /// Exclamation and question marks.
    Ex,
    /// Infix separators.
    Is,
    /// Numeric prefixes.
    Pr,
    /// Numeric postfixes.
    Po,
    /// Hyphen.
    Hy,
    /// Symbols allowing a break after.
    Sy,
    /// Break-after marks.
    Ba,
}

/// The empty boxes a line break leaves at the end of a line.
///
/// A break drops the space that would have separated the two lines, and the
/// reference keeps the box that space was in: `remove_last_whitespace` walks the
/// line's rightmost chain, strips the trailing space from that text box and
/// leaves it zero-width when nothing else was in it. The boxes between the
/// space's box and the block are therefore left empty, and the innermost one
/// keeps an empty text box of its own only when the space was the whole of its
/// text node - a space inside a node that holds other text (`a b`) is stripped
/// from a box the line still needs, so nothing is left empty.
///
/// A space at the block level with the line's last content deeper inside an
/// inline box (`<span>a</span> <span>b</span>`) leaves an empty text box for the
/// block, but only when the space was its own node: `<span>a</span> b` shares
/// the space with `b` and leaves nothing.
///
/// The returned chain is the boxes to leave empty, outermost first, the depth in
/// the line where the outermost one attaches, and whether the innermost box also
/// keeps an empty text box. `None` means no box is left.
fn break_boundary(current: &[Piece], space: &Piece) -> Option<(Vec<InlineAncestor>, usize, bool)> {
    let last = current.last()?;
    let space_path = &space.ancestors;
    let common = last
        .ancestors
        .iter()
        .zip(space_path)
        .take_while(|(left, right)| left.id == right.id)
        .count();
    if common == space_path.len() {
        // The space's box is a prefix of the last content's, so the space was
        // stripped from a box the line keeps, unless the space is at the block
        // level and the content it preceded is inside an inline box, which the
        // reference leaves as an empty text box for the block.
        if space.lone && space_path.is_empty() && !last.ancestors.is_empty() {
            Some((Vec::new(), 0, true))
        } else {
            None
        }
    } else {
        Some((space_path[common..].to_vec(), common, space.lone))
    }
}

/// A line's content, the indent applied to it, and the empty inline chain a break
/// left behind with the depth in the line where that chain attaches and whether
/// its innermost box keeps an empty text box. The chain is `None` when no break
/// ended the line.
type PendingLine = (Vec<Piece>, f64, Option<(Vec<InlineAncestor>, usize, bool)>);

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

    // Group the pieces into atoms first. An atom is a whole word, however many
    // inline elements it crosses - `ab<span>cd</span>` is one atom, because
    // there is no space and no break opportunity between `ab` and `cd` - or one
    // unit of a word that may break inside it, `foo-` and `bar` being two. Each
    // atom carries the space that preceded it, when there was one; that is where
    // a break before the atom falls, and it drops the space. An intra-word atom
    // carries no space, so a break before it drops nothing.
    let mut atoms: Vec<Atom> = Vec::new();
    let mut atom: Vec<Piece> = Vec::new();
    let mut atom_width = 0.0_f64;
    let mut atom_space: Option<Piece> = None;
    for piece in pieces(items) {
        if piece.is_space {
            if !atom.is_empty() {
                let width = atom_width;
                atoms.push(Atom {
                    pieces: std::mem::take(&mut atom),
                    width,
                    space: atom_space.take(),
                });
                atom_width = 0.0;
            }
            pending_space = Some(piece);
            continue;
        }
        // A piece with a space before it, or one that begins after a break
        // opportunity inside a word, starts a new atom. The first piece of an
        // atom keeps the space that led to it.
        if !atom.is_empty() && (piece.breakable || pending_space.is_some()) {
            let width = atom_width;
            atoms.push(Atom {
                pieces: std::mem::take(&mut atom),
                width,
                space: atom_space.take(),
            });
            atom_width = 0.0;
        }
        if atom.is_empty() {
            atom_space = pending_space.take();
        }
        atom_width += advance(&piece.text, piece.style.font_size);
        atom.push(piece);
    }
    if !atom.is_empty() {
        atoms.push(Atom {
            pieces: atom,
            width: atom_width,
            space: atom_space.take(),
        });
    }

    for Atom {
        pieces,
        width: atom_width,
        space,
    } in atoms
    {
        let available = width - leading;
        let space_width = space
            .as_ref()
            .map_or(0.0, |space| advance(" ", space.style.font_size));
        // The break happens when the atom does not fit and the line already has
        // content. At a space the break drops the space, and when the space was
        // the only content its box had on this line the reference leaves that
        // box behind empty. Inside a word there is no space, so nothing is left.
        //
        // One case is not a break at all: when the space itself is what
        // overflows and the atom after it would fit without the space, the
        // reference drops the space and keeps the atom on the line rather than
        // starting a new one. `abc ` at 24px against a 16px `a` in an 88px block
        // wraps to `abca`, not `abc` / `a`, because the 24px space cannot follow
        // `abc` but the 16px `a` can. The space fitting is what keeps this
        // apart from an ordinary break: at 96px the space fits and the atom does
        // not, so the line breaks normally instead.
        //
        // This only happens when the text box the space closes holds a single
        // unbreakable unit on this line. The reference splits a text box at its
        // last opportunity before deciding, so a box that already carries a
        // space (`ab cd `) or a word with an intra-word opportunity (`a-b `)
        // ends the line there and the space is kept. The check is scoped to the
        // space's own box - the run of pieces sharing its inline ancestors - not
        // to the whole line. A `foo-bar` whose `foo-` already wrapped leaves only
        // `bar` here, and `bar` is a single unit, so the space drops.
        let single_atom = space.as_ref().is_some_and(|space| {
            let run_start = current
                .iter()
                .rposition(|piece| !same_ancestors(&piece.ancestors, &space.ancestors))
                .map_or(0, |index| index + 1);
            let run = &current[run_start..];
            !run.is_empty()
                && !run.iter().any(|piece| piece.is_space)
                && !run.iter().skip(1).any(|piece| piece.breakable)
        });
        let overflows = !current.is_empty() && x + space_width + atom_width > available;
        let drop_space = overflows
            && space.is_some()
            && single_atom
            && x + space_width > available
            && x + atom_width <= available;
        if overflows && !drop_space {
            let boundary = space
                .as_ref()
                .and_then(|space| break_boundary(&current, space));
            lines.push((std::mem::take(&mut current), leading, boundary));
            x = 0.0;
            leading = 0.0;
        }
        if let Some(space) = space
            && !current.is_empty()
            && !drop_space
        {
            x += space_width;
            current.push(space);
        }
        x += atom_width;
        current.extend(pieces);
    }
    // A trailing collapsible space that overflows a line with an internal break
    // opportunity starts a line of its own, which holds nothing and is dropped.
    // The reference counts that line, so the one the space left is not last and
    // takes `text-align` rather than `text-align-last`. The break has to be
    // *inside* the line: a space between two words, or an intra-word break
    // between two units both on this line. A break at the line's own start does
    // not count, which is why `a/b ` wraps its space onto its own line but the
    // `c` that `a-b-c ` ends with does not - `c` begins the line, so there is
    // nothing before it to break from.
    let phantom = match &pending_space {
        Some(space) => {
            !current.is_empty()
                && current
                    .iter()
                    .enumerate()
                    .any(|(index, piece)| piece.is_space || (piece.breakable && index > 0))
                && x + advance(" ", space.style.font_size) > width - leading
        }
        None => false,
    };
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
            let last = index + 1 == count && !phantom;
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
    boundary: Option<(Vec<InlineAncestor>, usize, bool)>,
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
    if let Some((chain, _, _)) = &boundary {
        // The empty boxes left on the line still contribute their own struts, so
        // an empty wrapper with a larger font raises the line.
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
        block_style,
        tag,
        boundary
            .as_ref()
            .map(|(chain, attach, lone)| (chain.as_slice(), *attach, *lone)),
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
        collapses_through: false,
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
    block_style: &Style,
    block_tag: &str,
    tail: Option<(&[InlineAncestor], usize, bool)>,
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
        let inner_tail = tail.filter(|(_, attach, _)| is_last_run && *attach > depth);
        let mut children = nest(
            &items[group.start..group.end],
            depth + 1,
            line_baseline,
            block_style,
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
            collapses_through: false,
            inner_bottom: 0.0,
            children,
        });
    }

    // The tail's depth is this one, so its boxes are siblings of the boxes here.
    if let Some((chain, attach, lone)) = tail
        && attach == depth
    {
        out.extend(empty_boxes(
            chain,
            lone,
            end_x,
            line_baseline,
            block_style,
            block_tag,
            None,
        ));
    }
    out
}

/// Build the empty boxes a line break leaves at the end of a line.
///
/// An empty chain with `lone` is the block itself, which the reference leaves as
/// an empty text box for the block. A non-empty chain nests, so each box wraps
/// the next and only the outermost is placed against the line. The innermost box
/// keeps an empty text box only when the dropped space was the whole of its text
/// node (`lone`); otherwise the chain stops at the innermost inline box. The
/// nested boxes stay on the line's shared baseline, so each one's offset is its
/// parent's ascent less its own - the same alignment a text box gets, which is
/// what puts a smaller nested box lower on the line.
fn empty_boxes(
    chain: &[InlineAncestor],
    lone: bool,
    x: f64,
    line_baseline: f64,
    block_style: &Style,
    block_tag: &str,
    parent_ascent: Option<f64>,
) -> Vec<Fragment> {
    let Some(ancestor) = chain.first() else {
        // The block's own box: an empty text box with the block's line height,
        // which is what the reference leaves for a whitespace-only node dropped
        // at the block level when the line's last content sat inside an inline
        // box.
        return vec![Fragment {
            kind: BoxKind::Text,
            tag: block_tag.to_owned(),
            x_offset: x,
            y_offset: line_baseline - ascent(block_style),
            width: 0.0,
            height: block_style.font_size,
            margin: half_leading_sides(block_style),
            padding: Sides::default(),
            border: Sides::default(),
            text: Some(String::new()),
            collapses_through: false,
            inner_bottom: 0.0,
            children: Vec::new(),
        }];
    };
    let own_ascent = ascent(&ancestor.style);
    let children = match chain.get(1..) {
        Some(rest) if !rest.is_empty() => empty_boxes(
            rest,
            lone,
            0.0,
            line_baseline,
            block_style,
            block_tag,
            Some(own_ascent),
        ),
        // The innermost box holds the dropped space's text box, which the
        // reference keeps only when the space was its own text node.
        _ if lone => vec![Fragment {
            kind: BoxKind::Text,
            tag: ancestor.tag.clone(),
            x_offset: 0.0,
            y_offset: 0.0,
            width: 0.0,
            height: ancestor.style.font_size,
            margin: half_leading_sides(&ancestor.style),
            padding: Sides::default(),
            border: Sides::default(),
            text: Some(String::new()),
            collapses_through: false,
            inner_bottom: 0.0,
            children: Vec::new(),
        }],
        _ => Vec::new(),
    };
    let y_offset = match parent_ascent {
        // The outermost box sits against the line box, whose content top is the
        // baseline less this box's ascent.
        None => line_baseline - own_ascent,
        // A nested box aligns its baseline with its parent's, so it sits at the
        // parent's ascent less its own.
        Some(parent) => parent - own_ascent,
    };
    vec![Fragment {
        kind: BoxKind::Inline,
        tag: ancestor.tag.clone(),
        x_offset: x,
        y_offset,
        width: 0.0,
        height: ancestor.style.font_size,
        margin: half_leading_sides(&ancestor.style),
        padding: Sides::default(),
        border: Sides::default(),
        text: None,
        collapses_through: false,
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
        collapses_through: false,
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

#[cfg(test)]
mod tests {
    use super::{Piece, Style, break_units, seam_breakable};

    /// The units a word splits into, as a `Vec` of strings for easy comparison.
    fn units(word: &str) -> Vec<String> {
        break_units(word)
            .iter()
            .map(|unit| (*unit).to_owned())
            .collect()
    }

    #[test]
    fn a_word_with_no_opportunity_is_one_unit() {
        assert_eq!(units("abc"), ["abc"]);
        assert_eq!(units("3-4"), ["3-4"]);
        assert_eq!(units("-bar"), ["-bar"]);
    }

    #[test]
    fn a_hyphen_between_letters_is_an_opportunity() {
        assert_eq!(units("foo-bar"), ["foo-", "bar"]);
        assert_eq!(units("a-b-c"), ["a-", "b-", "c"]);
    }

    #[test]
    fn a_slash_between_letters_is_an_opportunity() {
        assert_eq!(units("a/b"), ["a/", "b"]);
    }

    #[test]
    fn a_numeric_expression_has_no_internal_opportunity() {
        // The class-pair rule would break each of these; LB25 keeps them whole.
        assert_eq!(units("3/4"), ["3/4"]);
        assert_eq!(units("$(3"), ["$(3"]);
        assert_eq!(units("(3)$"), ["(3)$"]);
        assert_eq!(units("1,000"), ["1,000"]);
        assert_eq!(units("3%"), ["3%"]);
        assert_eq!(units("+3"), ["+3"]);
    }

    #[test]
    fn a_word_initial_hyphen_does_not_break_before_a_letter() {
        // LB20a: `-bar` stays whole, but the hyphen before a symbol still breaks.
        assert_eq!(units("-bar"), ["-bar"]);
        assert_eq!(units("-$"), ["-", "$"]);
    }

    #[test]
    fn a_seam_between_text_nodes_uses_the_adjacent_characters_alone() {
        // Two adjacent text nodes meet with no space between them. The reference
        // asks Pango about the last character of one and the first of the next,
        // so the seam sees only that pair, not the whole expression: `3/` before
        // `4` breaks even though `3/4` inside one node does not.
        let word = |text: &str| Piece {
            ancestors: Vec::new(),
            style: Style::default(),
            text: text.to_owned(),
            is_space: false,
            lone: false,
            breakable: false,
        };
        assert!(seam_breakable(Some(&word("3/")), "4"));
        assert!(seam_breakable(Some(&word("a/")), "b"));
        assert!(seam_breakable(Some(&word("%")), "$"));
        assert!(!seam_breakable(Some(&word("a")), "b"));
        assert!(!seam_breakable(Some(&word("1,")), "000"));
        assert!(!seam_breakable(Some(&word("ab")), "cd"));
        // A seam after a space is a normal space break, not this rule.
        let mut space = word(" ");
        space.is_space = true;
        assert!(!seam_breakable(Some(&space), "b"));
        assert!(!seam_breakable(None, "b"));
    }
}
