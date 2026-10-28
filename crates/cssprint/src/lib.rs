//! Lay out HTML and CSS for paged output.
//!
//! The crate root documentation is the README, so the example a reader sees on
//! docs.rs is the same one `tests/readme.rs` executes and cannot drift from the
//! API. The WeasyPrint comparison, the font assumption and the scope are all
//! documented there.

#![doc = include_str!("../README.md")]
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
