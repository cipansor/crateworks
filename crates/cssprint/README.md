# cssprint

[![crates.io](https://img.shields.io/crates/v/cssprint.svg)](https://crates.io/crates/cssprint)
[![docs.rs](https://docs.rs/cssprint/badge.svg)](https://docs.rs/cssprint)

Lay out HTML and CSS for paged output, in pure Rust.

Generating a PDF from HTML is normally done by driving a browser, which means
shipping a Chromium binary, waiting for it, and accepting that the same document
may lay out differently on a different version. The alternative in Rust has been
to hand-place every element. This crate is the third option: a real box model and
a real layout algorithm that turn a small HTML document and its stylesheet into a
tree of positioned boxes.

```rust
use cssprint::{Stylesheet, layout};

let sheet = Stylesheet::parse("div { width: 50px; height: 20px }")?;
let document = layout(
    r#"<div></div><div style="width: 80px"></div>"#,
    &sheet,
    200.0, // page width
)?;

let body = document.body();
assert_eq!(body.children.len(), 2);
assert_eq!(body.children[0].width, 50.0);
assert_eq!(body.children[0].content_y, 0.0);
assert_eq!(body.children[1].width, 80.0);
// The second div sits directly below the first, because the first is 20 tall.
assert_eq!(body.children[1].content_y, 20.0);
# Ok::<(), cssprint::Error>(())
```

Text is laid out too, and wrapped into lines:

```rust
use cssprint::{Stylesheet, layout};

let sheet = Stylesheet::parse("div { font-family: weasyprint; width: 40px }")?;
let document = layout("<div>hello world</div>", &sheet, 200.0)?;

let div = &document.body().children[0];
assert_eq!(div.height, 32.0); // two 16px lines
assert_eq!(div.children[0].children[0].text.as_deref(), Some("hello"));
assert_eq!(div.children[1].children[0].text.as_deref(), Some("world"));
# Ok::<(), cssprint::Error>(())
```

`text-align` places each line within the block, and `justify` stretches the
spaces so every line but the last fills the width:

```rust
use cssprint::{Stylesheet, layout};

let sheet = Stylesheet::parse(
    "div { font-family: weasyprint; width: 64px; text-align: center }",
)?;
let document = layout("<div>aa</div>", &sheet, 200.0)?;

let line = &document.body().children[0].children[0];
// Two 16px glyphs in a 64px block, centred: (64 - 32) / 2 = 16.
assert_eq!(line.content_x, 16.0);
# Ok::<(), cssprint::Error>(())
```

`text-indent` shifts the first line of a block, in pixels or as a percentage of
the block's width:

```rust
use cssprint::{Stylesheet, layout};

let sheet = Stylesheet::parse("div { font-family: weasyprint; text-indent: 20px }")?;
let document = layout("<div>ab cd</div>", &sheet, 200.0)?;

// The indent shifts the line's content, so the text box starts 20px in.
let text = &document.body().children[0].children[0].children[0];
assert_eq!(text.content_x, 20.0);
# Ok::<(), cssprint::Error>(())
```

A selector can reach a descendant or a direct child, and the cascade orders by
specificity first, then source order:

```rust
use cssprint::{Stylesheet, layout};

// `div p` is more specific than `p`, so it wins even though it comes first.
let sheet = Stylesheet::parse("div p { width: 60px; height: 10px } p { width: 30px }")?;
let document = layout("<div><p></p></div><p></p>", &sheet, 200.0)?;

let div = &document.body().children[0];
assert_eq!(div.children[0].width, 60.0); // matched by `div p`
assert_eq!(document.body().children[1].width, 30.0); // only `p` matches
# Ok::<(), cssprint::Error>(())
```

## What it does

The CSS 2.1 block formatting context and inline layout. A block box fills the
width of its containing block unless `width` says otherwise, and each subsequent
sibling is placed below the previous one. `margin`, `padding` and `border` are
resolved, `auto` margins are distributed, percentages are taken against the
containing block, and `min-width` and `max-width` are applied. Adjacent vertical
margins collapse, including through a parent.

Text and inline elements are gathered into line boxes, wrapped to the block's
width, and placed on a shared baseline. Whitespace collapses, a space at a line
break is removed, and `font-size` and `line-height` are inherited and honoured.
Each line is aligned by `text-align` (`left`, `right`, `center`, `start`, `end`
and `justify`); `justify` stretches the spaces so the line fills the block, and
`text-align-last` overrides the final line, as in the reference. `text-indent`
shifts the first line of a block, in pixels or as a percentage of the block's
width.

Styles come from a stylesheet, an inline `style` attribute, or both, with the
inline attribute winning. A stylesheet rule's selector may be a comma-separated
list of complex selectors: each is a chain of compound selectors (a tag, an
`#id`, a `.class`, the universal `*`, or a combination) joined by a descendant
(whitespace) or child (`>`) combinator, so `div p`, `div > p`, `div.card#main`
and `div > p span` all work. The cascade orders by specificity first and source
order second, as the reference does: a more specific selector wins however early
it appears, and equal specificity falls back to the later rule. `display: none`
removes an element from the tree.

### Text and the font

This crate does not read font files. It assumes the font the test corpus uses,
which is monospace: every glyph advances exactly one em, and the ascent is
819/1024 em. A word's width is therefore its character count times the font size,
which is what makes the checked output identical on every machine.

Text laid out in a proportional font will not match a browser. Passing a font
with real metrics is future work; the box model, the line breaking and the
baseline arithmetic would not change, only the width function.

## What it does not do

Floats, `position`, tables, flex, grid, and pagination. `white-space: pre` and
`br` are also absent, as are attribute selectors, pseudo-classes, pseudo-elements
and at-rules. Sibling combinators (`+`, `~`) and a block box inside an inline box
(anonymous block splitting) are not implemented.

## How it is checked

Every layout rule here is checked against
[WeasyPrint](https://github.com/Kozea/WeasyPrint), the reference implementation
for CSS paged media. `tests/fixtures/layout.tsv` records the box tree WeasyPrint
produces for each document in `tests/fixtures/cases.tsv`, and `tests/layout.rs`
lays out the same document and compares the two box by box: position, size,
margins, padding and border widths.

`tests/fixtures/fuzz.tsv` and `tests/fixtures/fuzz_cases.tsv` hold the same pair
for a deterministic random corpus of 400 documents over the same property set.

Selector matching has its own pair of corpora, `selector_layout.tsv` /
`selector_cases.tsv` and `selector_fuzz.tsv` / `selector_fuzz_cases.tsv`, drawn
from documents whose elements carry ids and classes and a stylesheet of
descendant and child selectors over them: the box tree only comes out right when
both the matching and the specificity ordering are right.

The curated cases are the shapes someone thought of; the random ones are the
shapes nobody wrote down, so a change that keeps the curated cases green but
moves a random box fails the test. Every corpus is generated by
`scripts/fuzz/cssprint/gen.py`, so the oracle is the reference's own output
rather than a transcription of it. The generator loads the reference's own test
font, which is derived from Ahem and is monospace, so the text cases are
arithmetic and the committed fixtures are the same on every machine. The font
file itself is not redistributed; see `NOTICE`.

A random document may use a character the corpus's monospace font does not
define, and then the two engines' glyph metrics differ for a reason that is not a
layout bug; the shipped word pool is drawn from the covered alphabet for that
reason.

## Licence

Apache-2.0. See `NOTICE` for the WeasyPrint attribution.
