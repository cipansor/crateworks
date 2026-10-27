//! Computed styles: the values layout needs, resolved from the cascade.
//!
//! Lengths are kept as written until they are resolved against a containing
//! block, because `50%` and `auto` mean different things at different depths.

use crate::css::{Declaration, Stylesheet};
use crate::dom::{Element, is_block, is_hidden};
use crate::error::Error;

/// The `display` property, reduced to what this crate lays out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Display {
    /// A block box, laid out vertically.
    Block,
    /// An inline box, laid out in lines inside a block.
    Inline,
    /// No box at all; the element and its subtree are removed.
    None,
}

/// The `line-height` property, before the used value is worked out.
///
/// `normal` and a bare number both depend on the element's font size, so they
/// are kept unresolved here; `Px` does not.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum LineHeight {
    /// `normal`, which is one em for the fonts this crate supports.
    Normal,
    /// A length in pixels.
    Px(f64),
    /// A multiple of the font size.
    Number(f64),
}

/// A length, either absolute or a percentage of the containing block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Length {
    /// A value in pixels.
    Px(f64),
    /// A percentage of the containing block's width.
    Percent(f64),
    /// The `auto` keyword.
    Auto,
}

/// The four sides of a box, in one value.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Sides {
    /// The top side.
    pub top: f64,
    /// The right side.
    pub right: f64,
    /// The bottom side.
    pub bottom: f64,
    /// The left side.
    pub left: f64,
}

/// The computed style of one element.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Style {
    /// Whether the element produces a block box, or none at all.
    pub(crate) display: Display,
    /// The `width`, when it was declared.
    pub(crate) width: Option<Length>,
    /// The `height`, when it was declared.
    pub(crate) height: Option<f64>,
    /// The `min-width`, when it was declared.
    pub(crate) min_width: Option<Length>,
    /// The `max-width`, when it was declared.
    pub(crate) max_width: Option<Length>,
    /// The margins, which may be `auto`.
    pub(crate) margin: [Length; 4],
    /// The padding, which may not be `auto`.
    pub(crate) padding: Sides,
    /// The border widths, which are zero unless a border style is set.
    pub(crate) border: Sides,
    /// The `font-size` in pixels, which is inherited.
    pub(crate) font_size: f64,
    /// The `line-height`, which is inherited.
    pub(crate) line_height: LineHeight,
}

/// The font size an element has when nothing sets one.
const DEFAULT_FONT_SIZE: f64 = 16.0;

impl Default for Style {
    fn default() -> Self {
        Self {
            display: Display::Block,
            width: None,
            height: None,
            min_width: None,
            max_width: None,
            margin: [Length::Px(0.0); 4],
            padding: Sides::default(),
            border: Sides::default(),
            font_size: DEFAULT_FONT_SIZE,
            line_height: LineHeight::Normal,
        }
    }
}

impl Style {
    /// The used `line-height`: a length, with `normal` and a bare number
    /// resolved against the font size.
    pub(crate) fn used_line_height(&self) -> f64 {
        match self.line_height {
            LineHeight::Normal => self.font_size,
            LineHeight::Px(value) => value,
            LineHeight::Number(factor) => factor * self.font_size,
        }
    }
}

/// The `display` an element has before any author style is applied.
///
/// This mirrors the reference's test user-agent stylesheet: the block tags it
/// lists are blocks, the tags it hides produce no box, and everything else is
/// inline.
fn initial_display(element: &Element) -> Display {
    if is_hidden(&element.tag) {
        Display::None
    } else if is_block(&element.tag) {
        Display::Block
    } else {
        Display::Inline
    }
}

/// Compute the style of `element` from the stylesheet and its own `style`.
///
/// # Errors
///
/// Returns [`Error::Malformed`] when a declaration cannot be read.
/// `parent` supplies the inherited properties, which are `font-size` and
/// `line-height`. The root element passes `None` and gets the defaults.
pub(crate) fn compute(
    element: &Element,
    stylesheet: &Stylesheet,
    parent: Option<&Style>,
) -> Result<Style, Error> {
    let inherited = parent.cloned().unwrap_or_default();
    let mut style = Style {
        display: initial_display(element),
        font_size: inherited.font_size,
        line_height: inherited.line_height,
        ..Style::default()
    };

    // The stylesheet comes first so that the inline attribute, applied second,
    // wins. A later declaration of the same property always replaces an earlier
    // one, which is the cascade this crate supports.
    let mut declarations: Vec<Declaration> = stylesheet
        .declarations_for(element)
        .into_iter()
        .cloned()
        .collect();
    if let Some(inline) = &element.style {
        declarations.extend(parse_inline(inline)?);
    }

    for declaration in &declarations {
        apply(&mut style, declaration)?;
    }
    Ok(style)
}

/// Parse the contents of a `style` attribute, which has no braces.
fn parse_inline(text: &str) -> Result<Vec<Declaration>, Error> {
    crate::css::parse_declaration_list(text)
}

/// Apply one declaration to `style`.
fn apply(style: &mut Style, declaration: &Declaration) -> Result<(), Error> {
    let property = declaration.property.as_str();
    let value = declaration.value.as_str();

    match property {
        "display" => {
            style.display = match value {
                "block" => Display::Block,
                "inline" => Display::Inline,
                "none" => Display::None,
                other => {
                    return Err(Error::Unsupported(format!(
                        "display: {other} is not supported"
                    )));
                }
            };
        }
        "width" => style.width = Some(length(value)?),
        "min-width" => style.min_width = Some(length(value)?),
        "max-width" => style.max_width = Some(length(value)?),
        "height" => {
            style.height =
                Some(pixels(value).ok_or_else(|| {
                    Error::Unsupported(format!("height: {value} is not a length"))
                })?);
        }
        "margin" => style.margin = sides_of(value, true)?,
        "margin-top" => style.margin[0] = length(value)?,
        "margin-right" => style.margin[1] = length(value)?,
        "margin-bottom" => style.margin[2] = length(value)?,
        "margin-left" => style.margin[3] = length(value)?,
        "padding" => {
            let sides = sides_of(value, false)?;
            style.padding = Sides {
                top: px(&sides[0]),
                right: px(&sides[1]),
                bottom: px(&sides[2]),
                left: px(&sides[3]),
            };
        }
        "padding-top" => style.padding.top = px(&length(value)?),
        "padding-right" => style.padding.right = px(&length(value)?),
        "padding-bottom" => style.padding.bottom = px(&length(value)?),
        "padding-left" => style.padding.left = px(&length(value)?),
        "border-width" => {
            let sides = sides_of(value, false)?;
            style.border = Sides {
                top: px(&sides[0]),
                right: px(&sides[1]),
                bottom: px(&sides[2]),
                left: px(&sides[3]),
            };
        }
        "border-top-width" => style.border.top = px(&length(value)?),
        "border-right-width" => style.border.right = px(&length(value)?),
        "border-bottom-width" => style.border.bottom = px(&length(value)?),
        "border-left-width" => style.border.left = px(&length(value)?),
        // `border: 2px solid` sets the width as its length component. A border
        // style of `none` is not distinguished, because the corpus never uses
        // it; the widths are simply taken.
        "font-size" => {
            style.font_size = pixels(value)
                .ok_or_else(|| Error::Unsupported(format!("font-size: {value} is not a length")))?;
        }
        "line-height" => style.line_height = line_height(value)?,
        "border" => style.border = border_shorthand(value)?,
        "border-top" => style.border.top = border_width(value)?,
        "border-right" => style.border.right = border_width(value)?,
        "border-bottom" => style.border.bottom = border_width(value)?,
        "border-left" => style.border.left = border_width(value)?,
        // Recognised and ignored, so that a document that sets them still lays
        // out rather than failing.
        "border-style"
        | "border-top-style"
        | "border-right-style"
        | "border-bottom-style"
        | "border-left-style"
        | "color"
        | "background"
        | "background-color"
        | "font-family"
        | "direction"
        | "box-sizing" => {}
        other => {
            return Err(Error::Unsupported(format!(
                "the property `{other}` is not supported"
            )));
        }
    }
    Ok(())
}

/// Read `line-height`: `normal`, a length, or a bare multiple of the font size.
fn line_height(value: &str) -> Result<LineHeight, Error> {
    let value = value.trim();
    if value == "normal" {
        return Ok(LineHeight::Normal);
    }
    // A length carries a unit; a bare number is a multiple of the font size.
    if let Some(number) = value.strip_suffix("px") {
        return number
            .trim()
            .parse::<f64>()
            .map(LineHeight::Px)
            .map_err(|_| Error::Malformed(format!("cannot read line-height `{value}`")));
    }
    value
        .parse::<f64>()
        .map(LineHeight::Number)
        .map_err(|_| Error::Malformed(format!("cannot read line-height `{value}`")))
}

/// Read a length: pixels, a percentage, `auto`, or a bare zero.
fn length(value: &str) -> Result<Length, Error> {
    let value = value.trim();
    if value == "auto" {
        return Ok(Length::Auto);
    }
    if let Some(percent) = value.strip_suffix('%') {
        let number: f64 = percent
            .trim()
            .parse()
            .map_err(|_| Error::Malformed(format!("cannot read percentage `{value}`")))?;
        return Ok(Length::Percent(number));
    }
    let number =
        pixels(value).ok_or_else(|| Error::Malformed(format!("cannot read length `{value}`")))?;
    Ok(Length::Px(number))
}

/// Read a pixel length, accepting a bare number as pixels.
fn pixels(value: &str) -> Option<f64> {
    let value = value.trim();
    let number = value.strip_suffix("px").unwrap_or(value).trim();
    if number.is_empty() {
        // `border-width: 0` is the only zero with no unit in the corpus, and
        // `0px` is handled above; anything else empty is a mistake.
        return None;
    }
    number.parse::<f64>().ok()
}

/// Resolve a length to pixels against `base`, treating `auto` as zero.
fn px(length: &Length) -> f64 {
    match length {
        Length::Px(value) => *value,
        Length::Percent(percent) => *percent,
        Length::Auto => 0.0,
    }
}

/// Read the one-to-four values of a shorthand into top, right, bottom, left.
fn sides_of(value: &str, allow_auto: bool) -> Result<[Length; 4], Error> {
    let parts: Vec<&str> = value.split_whitespace().collect();
    let read = |text: &str| -> Result<Length, Error> {
        let length = length(text)?;
        if !allow_auto && matches!(length, Length::Auto) {
            return Err(Error::Malformed(format!("`{text}` cannot be auto here")));
        }
        Ok(length)
    };
    Ok(match parts.as_slice() {
        [all] => [read(all)?; 4],
        [vertical, horizontal] => [
            read(vertical)?,
            read(horizontal)?,
            read(vertical)?,
            read(horizontal)?,
        ],
        [top, horizontal, bottom] => [
            read(top)?,
            read(horizontal)?,
            read(bottom)?,
            read(horizontal)?,
        ],
        [top, right, bottom, left] => [read(top)?, read(right)?, read(bottom)?, read(left)?],
        _ => {
            return Err(Error::Malformed(format!(
                "expected one to four values, found `{value}`"
            )));
        }
    })
}

/// Read `border: 2px solid` into the four widths, which are all the same.
fn border_shorthand(value: &str) -> Result<Sides, Error> {
    let width = border_width(value)?;
    Ok(Sides {
        top: width,
        right: width,
        bottom: width,
        left: width,
    })
}

/// Read the length out of a `border` shorthand, defaulting to zero.
fn border_width(value: &str) -> Result<f64, Error> {
    for part in value.split_whitespace() {
        if let Some(number) = pixels(part) {
            return Ok(number);
        }
        match part {
            "thin" => return Ok(1.0),
            "medium" => return Ok(3.0),
            "thick" => return Ok(5.0),
            _ => {}
        }
    }
    Err(Error::Malformed(format!("no border width in `{value}`")))
}
