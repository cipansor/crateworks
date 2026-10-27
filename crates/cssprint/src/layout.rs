//! Block layout: the CSS 2.1 block formatting context.
//!
//! A block box fills its containing block unless `width` says otherwise, and
//! each later sibling is placed below the previous one. Two details decide most
//! of the numbers in the fixture, and both were read off the reference rather
//! than assumed.
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
//! Each rule is checked in `tests/layout.rs`.

use crate::dom::{Element, Node, is_block};
use crate::error::Error;
use crate::style::{Display, Length, Sides, Style};

/// The kind of box, named as the reference names it so that a tree can be
/// compared with WeasyPrint's directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoxKind {
    /// A block-level box.
    Block,
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
    /// The tag name, or an empty string for the page box.
    pub tag: String,
    /// Whether this is a block or the page.
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

/// A box during layout, before its absolute position is known.
#[derive(Debug)]
struct Fragment {
    tag: String,
    width: f64,
    /// Distance from the parent's content-left edge to this box's content box.
    x_offset: f64,
    /// Distance from the parent's content-top edge to this box's content box,
    /// with the collapsed margin above it already included.
    y_offset: f64,
    /// Border plus padding on the top.
    inner_top: f64,
    /// Border plus padding on the bottom.
    inner_bottom: f64,
    height: f64,
    /// The margin as declared, which is what the box reports.
    margin_top: f64,
    /// The margin as declared.
    margin_bottom: f64,
    /// The margin this box presents to its parent, after any collapse.
    effective_margin_top: f64,
    /// The margin this box presents to its parent, after any collapse.
    effective_margin_bottom: f64,
    margin_left: f64,
    margin_right: f64,
    padding: Sides,
    border: Sides,
    children: Vec<Fragment>,
}

/// Lay out a parsed document into a page box.
pub(crate) fn layout_document(
    dom: &Element,
    stylesheet: &crate::Stylesheet,
    page_width: f64,
) -> Result<Document, Error> {
    let mut html = layout_element(dom, stylesheet, page_width, true)?;
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
        children: vec![html],
    };
    Ok(Document { root })
}

/// Lay out one element and its subtree.
///
/// `is_root` marks the `html` element, which neither collapses with the page nor
/// with its own first child in the way an ordinary box does.
fn layout_element(
    element: &Element,
    stylesheet: &crate::Stylesheet,
    containing_width: f64,
    is_root: bool,
) -> Result<Fragment, Error> {
    let style = crate::style::compute(element, stylesheet)?;
    let (width, x_offset, margin_left, margin_right) = resolve_horizontal(&style, containing_width);

    let inner_top = style.border.top + style.padding.top;
    let inner_bottom = style.border.bottom + style.padding.bottom;
    let own_margin_top = resolve_margin(style.margin[0], containing_width);
    let own_margin_bottom = resolve_margin(style.margin[2], containing_width);

    let mut children = Vec::new();
    for node in &element.children {
        let Some(child) = as_block_element(node) else {
            continue;
        };
        let child_style = crate::style::compute(child, stylesheet)?;
        if child_style.display == Display::None {
            continue;
        }
        children.push(layout_element(child, stylesheet, width, false)?);
    }

    let auto_height = style.height.is_none();

    if children.is_empty() {
        return Ok(Fragment {
            tag: element.tag.clone(),
            width,
            x_offset,
            y_offset: inner_top,
            inner_top,
            inner_bottom,
            height: style.height.unwrap_or(0.0),
            margin_top: own_margin_top,
            margin_bottom: own_margin_bottom,
            effective_margin_top: own_margin_top,
            effective_margin_bottom: own_margin_bottom,
            margin_left,
            margin_right,
            padding: style.padding,
            border: style.border,
            children,
        });
    }

    let first = &children[0];
    let last = children.last().expect("not empty");
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
        tag: element.tag.clone(),
        width,
        x_offset,
        y_offset: inner_top,
        inner_top,
        inner_bottom,
        height: style.height.unwrap_or(content_height),
        margin_top: own_margin_top,
        margin_bottom: own_margin_bottom,
        effective_margin_top,
        effective_margin_bottom,
        margin_left,
        margin_right,
        padding: style.padding,
        border: style.border,
        children,
    })
}

/// Return the node if it is a block-level element, which is the only kind this
/// crate lays out. Text and inline containers produce nothing.
fn as_block_element(node: &Node) -> Option<&Element> {
    match node {
        Node::Element(element) if is_block(&element.tag) => Some(element),
        _ => None,
    }
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
        kind: BoxKind::Block,
        content_x,
        content_y,
        width: fragment.width,
        height: fragment.height,
        margin: Sides {
            top: fragment.margin_top,
            right: fragment.margin_right,
            bottom: fragment.margin_bottom,
            left: fragment.margin_left,
        },
        padding: fragment.padding,
        border: fragment.border,
        children,
    }
}
