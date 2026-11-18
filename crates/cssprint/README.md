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

A selector can reach a descendant, a direct child or a sibling, and can test an
attribute or a structural position. The cascade orders by specificity first,
then source order:

```rust
use cssprint::{Stylesheet, layout};

// `div p` is more specific than `p`, so it wins even though it comes first.
let sheet = Stylesheet::parse("div p { width: 60px; height: 10px } p { width: 30px }")?;
let document = layout("<div><p></p></div><p></p>", &sheet, 200.0)?;

let div = &document.body().children[0];
assert_eq!(div.children[0].width, 60.0); // matched by `div p`
assert_eq!(document.body().children[1].width, 30.0); // only `p` matches

// A sibling combinator and an attribute selector both work.
let sheet = Stylesheet::parse("h1 + p { width: 40px } [title] { height: 5px }")?;
let document = layout(
    "<h1></h1><p title=\"a\"></p><p></p>",
    &sheet,
    200.0,
)?;
assert_eq!(document.body().children[1].width, 40.0); // the `p` after the `h1`
assert_eq!(document.body().children[1].height, 5.0); // and it has a title
assert_eq!(document.body().children[2].width, 200.0); // the second `p` is untouched
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
list of complex selectors: each is a chain of compound selectors joined by a
combinator. A compound is a tag, an `#id`, a `.class`, an attribute selector, a
structural pseudo-class, the universal `*`, or any combination of those, so
`div.card#main[href]:first-child` is one compound. The combinators are the
descendant (whitespace), the child (`>`), the adjacent sibling (`+`) and the
general sibling (`~`), so `div p`, `div > p`, `h1 + p` and `h1 ~ p` all work.
The attribute selectors `[a]`, `[a=v]`, `[a~=v]`, `[a|=v]`, `[a^=v]`, `[a$=v]`
and `[a*=v]` are supported. So are the structural pseudo-classes `:root`,
`:first-child`, `:last-child`, `:only-child`, `:nth-child()` and
`:nth-last-child()` (with `odd`, `even`, an index or an `an+b` argument), and the
negation `:not()`, which takes a single compound; a compound may carry several
negations, as in `div:not(.a):not(.b)`. Pseudo-class names are ASCII
case-insensitive, so `:FIRST-CHILD` and `:first-child` are the same. A tag name
may contain any alphanumeric character, ASCII or not, plus `-` and `_`, matching
what the HTML parser keeps, so `café` parses like `div` does.

A quoted attribute value is read as a CSS string: a comma, `]` or `)` inside it
is text rather than syntax, a backslash escapes the character after it, and the
string's escapes are decoded before the value is compared, so `[title="Doe, Jane"]`
and `[title="a\62"]` (which names `ab`) both work. The cascade orders by
specificity first and source order second, as the reference does: a more
specific selector wins however early it appears, and equal specificity falls
back to the later rule. An attribute selector and a structural pseudo-class each
count as a class; `:not()` counts as its argument, and two negations add
together. `display: none` removes an element from the tree.

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
`br` are also absent. Pseudo-elements and at-rules are not implemented, nor are
the non-structural pseudo-classes (`:hover`, `:link`, `:nth-of-type` and the
like). `:not()` takes a single compound, as CSS level 3 allows, not the
selector list level 4 permits. A block box inside an inline box (anonymous block
splitting) is not implemented.

## How it is checked

Every layout rule here is checked against
[WeasyPrint](https://github.com/Kozea/WeasyPrint), the reference implementation
for CSS paged media. `tests/fixtures/layout.tsv` records the box tree WeasyPrint
produces for each document in `tests/fixtures/cases.tsv`, and `tests/layout.rs`
lays out the same document and compares the two box by box: position, size,
margins, padding and border widths.

`tests/fixtures/fuzz.tsv` and `tests/fixtures/fuzz_cases.tsv` hold the same pair
for a deterministic random corpus of 2000 documents over the same property set.

Selector matching has its own pair of corpora, `selector_layout.tsv` /
`selector_cases.tsv` and `selector_fuzz.tsv` / `selector_fuzz_cases.tsv`, drawn
from documents whose elements carry ids, classes and attributes and a stylesheet
of complex selectors over them, including the sibling combinators, the attribute
selectors and the structural pseudo-classes: the box tree only comes out right
when both the matching and the specificity ordering are right. The elements nest,
including spans inside spans, and a rule whose subject may be inline carries
`font-size`, an inherited property that moves the text: a box-model declaration
is ignored on a non-replaced inline, so it would hide a broken inline ancestry
instead of exposing it.

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
