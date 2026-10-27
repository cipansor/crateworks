//! Lay out HTML and CSS for paged output.
//!
//! The job this crate takes on is turning a small HTML document and its
//! stylesheet into a tree of boxes with resolved geometry - the step that comes
//! before writing a PDF. Doing that in Rust has meant either driving a browser
//! or placing every element by hand.
//!
//! What is implemented is the CSS 2.1 block formatting context. A block box
//! fills the width of its containing block unless `width` says otherwise, and
//! each later sibling is placed below the previous one. Margins, padding and
//! borders are resolved, `auto` margins are distributed, percentages resolve
//! against the containing block, `min-width` and `max-width` are applied, and
//! adjacent vertical margins collapse.
//!
//! Text and inline layout are deliberately absent, because they need font
//! metrics and would make the result depend on the fonts installed.
//!
//! The behaviour is checked against [WeasyPrint](https://github.com/Kozea/WeasyPrint),
//! the reference implementation for CSS paged media: `tests/layout.rs` lays out
//! each document in the fixture corpus and compares the resulting box tree with
//! the one WeasyPrint produced, box by box.
//!
//! ```
//! use cssprint::{Stylesheet, layout};
//!
//! let sheet = Stylesheet::parse("div { width: 50px; height: 20px }")?;
//! let document = layout("<div></div><div style=\"width: 80px\"></div>", &sheet, 200.0)?;
//!
//! let body = document.body();
//! assert_eq!(body.children[0].width, 50.0);
//! assert_eq!(body.children[1].content_y, 20.0);
//! # Ok::<(), cssprint::Error>(())
//! ```

#![forbid(unsafe_code)]

mod css;
mod dom;
mod error;
mod layout;
mod style;

pub use css::{Declaration, Rule, Selector, Stylesheet};
pub use dom::{Element, Node};
pub use error::Error;
pub use layout::{BoxKind, Document, LayoutBox};
pub use style::Sides;

/// Lay out `html` at `page_width` and return the box tree.
///
/// The returned [`Document`] has a page root whose single child is `html`, and
/// beneath that `body`, the same shape WeasyPrint produces, so a tree can be
/// compared with the reference position by position.
///
/// # Errors
///
/// Returns [`Error::Unsupported`] when the page width is not a positive, finite
/// number, or [`Error::Malformed`] when the HTML or a declaration cannot be
/// parsed.
pub fn layout(html: &str, stylesheet: &Stylesheet, page_width: f64) -> Result<Document, Error> {
    if !page_width.is_finite() || page_width <= 0.0 {
        return Err(Error::Unsupported(
            "page width must be a positive, finite number".to_owned(),
        ));
    }
    let dom = dom::parse(html)?;
    layout::layout_document(&dom, stylesheet, page_width)
}
