#!/usr/bin/env python3
"""Fixture generator for a Rust CSS layout engine: WeasyPrint's box tree as oracle.

The gap round 28 identified is HTML/CSS to paginated PDF. The reference is
WeasyPrint, and the important thing about it is that its layout tests do not
inspect PDFs at all - `render_pages()` returns the laid-out box tree and the
tests assert exact geometry on it. That is a much better oracle than a PDF diff:
it is exact, it is stable, and a failure names one box and one number.

This script is that oracle, extracted. It renders each case in `CASES` with
WeasyPrint, walks the resulting box tree, and writes a TSV of

    case  path  tag  box  cx  cy  width  height  ml  mr  mt  mb  pl  pr  pt  pb
    bl  br  bt  bb  text

where `cx`/`cy` are the absolute content-box origin and `path` is the
child-index route from the root (for example `0.1.0`), so a Rust implementation
can walk its own tree in the same order and compare position by position. The
margins, paddings and border widths are recorded separately because
`position_x` is the margin-box origin: an `auto` margin is invisible in the
position alone.

    python3 scripts/fuzz/cssprint/gen.py [crate dir]   # rebuild the fixtures
    python3 scripts/fuzz/cssprint/gen.py --show        # print each tree

Needs `pip install weasyprint`. Run from the repository root; with no crate
directory it writes to `crates/cssprint`.

The corpus covers the box model (width, height, margin, padding, border), block
layout, inline layout, line breaking, text alignment, `text-indent`, and selector
matching. The committed fixtures are the hand-picked list in `cases()` and the
deterministic random list in `random_cases()`, with a fixed seed, drawn from the
same implemented property set. Both are written to the crate and replayed by
`tests/layout.rs`, so the random list is a regression gate and not only a
development aid. `--random N` overrides the random count, which is how a fresh
seed is explored: a fresh seed can still surface a divergence the shipped one
misses.

Selector matching has its own pair of corpora, `selector_cases()` and
`selector_random_cases()`, so a selector failure is not buried among the
box-model cases. They draw documents whose elements carry ids and classes and a
stylesheet of descendant and child selectors over them; the box tree only comes
out right when both the matching and the specificity ordering are right.
`--selectors N` overrides the random selector count.

Text is reproducible because the cases load the reference's own test font:
`weasyprint.otf` is derived from Ahem, so every glyph advances exactly one em,
the ascent is 819/1024 em, the descent 205/1024 em and the normal line height is
one em. Line breaking is then arithmetic, not a guess about installed fonts.

The font file belongs to WeasyPrint and is not redistributed here; pass its
location with `--font` or clone WeasyPrint next to the workspace. Only the
generated TSV is committed, and it holds numbers, so it is the same on every
machine.
"""

from __future__ import annotations

import argparse
import os
import pathlib
import random
import sys

PAGE = "@page { size: %dpx %dpx; margin: 0 }"

# The page height the random corpus is written with. It is a page no document
# reaches, so the reference never clips: the crate lays out at the page width
# only and models no pagination, so a page shorter than the content cuts the
# reference's tree off and the two trees differ for a reason that is not a layout
# bug. The page's own height is not compared - the fixture writes the page box and
# `html` at the same path, so the later row wins - but the *clip* it causes is
# real, and a page taller than any random document removes it.
_NO_CLIP_HEIGHT = 100_000_000

# The generator's random corpus draws from the features the crate implements, so
# a difference is a bug rather than an unimplemented property. Keep this list in
# step with the crate: adding a property here is how it gets checked.
_RANDOM_SEED = 20261116
# A few thousand inputs, as the repository standard asks for. At 400 the corpus
# missed a whole class of divergence - a redundant space dropped at a line break
# inside an inline box - because the draw rarely put a whitespace-only node
# between two elements. The TSV is ~30 MB of text but compresses to ~2 MB in git,
# and the replay is a couple of seconds.
_RANDOM_COUNT = 2000
_ALIGNMENTS = ["left", "right", "center", "start", "end", "justify"]
_WORDS = [
    "a", "ab", "abc", "hello", "world", "lorem", "ipsum", "dolor", "sit",
    # Words with a break opportunity inside them, so the corpus exercises line
    # breaking at punctuation and not only at spaces: `foo-bar` may wrap after
    # the hyphen, `3-4` may not.
    "foo-bar", "well-known", "e-mail", "self-test", "a/b", "foo/bar", "3-4",
    "a-b", "x!y", "p|q", "a-b-c",
    # Numeric expressions, which have no break opportunity inside them even
    # where the class-pair rule would allow one: `3/4`, `$(3` and `(3)$` each
    # stay whole, while `a/b` still wraps after the slash.
    "3/4", "$(3", "(3)$", "1,000", "3%", "+3", "3.5",
]


def _length(
    rng: random.Random, *, allow_auto: bool = False, hi: int = 120, percent: bool = True
) -> str:
    """A random length from the properties the crate resolves."""
    kinds = ["px"]
    if percent:
        kinds.append("percent")
    if allow_auto:
        kinds.append("auto")
    kind = rng.choice(kinds)
    if kind == "auto":
        return "auto"
    if kind == "percent":
        return f"{rng.choice([10, 20, 25, 33, 50, 75, 100])}%"
    return f"{rng.randrange(0, hi)}px"


def _sides(
    rng: random.Random, *, allow_auto: bool = False, hi: int = 120, percent: bool = True
) -> str:
    """One to four values for a `margin` or `padding` shorthand."""
    count = rng.randrange(1, 5)
    return " ".join(
        _length(rng, allow_auto=allow_auto, hi=hi, percent=percent) for _ in range(count)
    )


def _block_declarations(rng: random.Random) -> list[str]:
    """Declarations for a block box, from the supported set only."""
    out = []
    if rng.random() < 0.75:
        out.append(f"width:{_length(rng)}")
    if rng.random() < 0.35:
        out.append(f"height:{rng.randrange(0, 60)}px")
    if rng.random() < 0.35:
        out.append("min-width:" + _length(rng, hi=40))
    if rng.random() < 0.35:
        out.append("max-width:" + _length(rng, hi=160))
    if rng.random() < 0.5:
        out.append(f"margin:{_sides(rng, allow_auto=True)}")
    elif rng.random() < 0.5:
        out.append(f"margin-left:{_length(rng, allow_auto=True)}")
        out.append(f"margin-right:{_length(rng, allow_auto=True)}")
    if rng.random() < 0.5:
        out.append(f"padding:{_sides(rng)}")
    if rng.random() < 0.35:
        out.append("border-width:" + _sides(rng, hi=6, percent=False))
        out.append("border-style:solid")
    if rng.random() < 0.4:
        out.append(f"font-size:{rng.choice([8, 10, 12, 16, 20, 24])}px")
    if rng.random() < 0.3:
        out.append(f"line-height:{rng.choice(['normal', '1', '1.5', '2', '24px', '30px'])}")
    if rng.random() < 0.5:
        out.append(f"text-align:{rng.choice(_ALIGNMENTS)}")
    if rng.random() < 0.25:
        out.append(f"text-align-last:{rng.choice(_ALIGNMENTS + ['auto'])}")
    if rng.random() < 0.3:
        out.append("text-indent:" + _length(rng, hi=40))
    return out


def _text(rng: random.Random) -> str:
    """A run of words, with whitespace the collapsing rules have to fold."""
    count = rng.randrange(1, 8)
    words = [rng.choice(_WORDS) for _ in range(count)]
    joiner = rng.choice([" ", "  ", " \n ", "\t"])
    text = joiner.join(words)
    if rng.random() < 0.3:
        text = " " + text
    if rng.random() < 0.3:
        text += " "
    return text


def _inline(rng: random.Random, depth: int) -> str:
    """A run of text and nested inline elements.

    Only `span` is used: `b` and `i` select a bold or italic face in the
    reference's user-agent stylesheet, and this crate lays out one font, so they
    would compare a face the crate does not have.
    """
    parts = []
    for index in range(rng.randrange(1, 4)):
        if index > 0 and rng.random() < 0.4:
            # A bare whitespace-only text node between two pieces. The first
            # piece is never preceded by one, so a nested `span` is never left
            # holding only whitespace, which the crate does not model as a box.
            # These nodes are what carry the whitespace-collapse state across an
            # element boundary, where a redundant space can otherwise pick up the
            # wrong font.
            parts.append(rng.choice([" ", "  ", " \n ", "\t"]))
        if depth < 2 and rng.random() < 0.4:
            style = ""
            if rng.random() < 0.3:
                style = f' style="font-size:{rng.choice([10, 16, 24])}px"'
            parts.append(f"<span{style}>{_inline(rng, depth + 1)}</span>")
        else:
            parts.append(_text(rng))
    if rng.random() < 0.15:
        # A trailing whitespace-only node, which a line break drops like one in
        # the middle and which leaves the same empty box behind.
        parts.append(rng.choice([" ", "  "]))
    return "".join(parts)


def _block(rng: random.Random, depth: int, tag: str = "div") -> str:
    """A block box, optionally holding inline content or nested blocks."""
    style = ";".join(_block_declarations(rng))
    attribute = f' style="{style}"' if style else ""
    if depth < 2 and rng.random() < 0.5:
        inner = "".join(_block(rng, depth + 1) for _ in range(rng.randrange(1, 3)))
    elif rng.random() < 0.75:
        inner = _inline(rng, 0)
    else:
        inner = ""
    return f"<{tag}{attribute}>{inner}</{tag}>"


def random_document(rng: random.Random) -> str:
    """One random document, built only from the features the crate implements."""
    # A page no random document reaches, so the reference does not clip: the crate
    # lays out at the page width only and models no pagination, so a document
    # taller than its page would have its tree cut off in the reference and the
    # two would differ over boxes the crate lays out correctly. The height is not
    # itself compared (the fixture's page box is shadowed by `html`), but the clip
    # it avoids is.
    page = PAGE % (200, _NO_CLIP_HEIGHT)
    # The body always selects the reference's test font and pins its margin. The
    # crate lays out one font and has no user-agent stylesheet, so a case that
    # left either to the reference's defaults would compare a value the crate
    # does not model. Only `div` and `span` are used below, and neither carries a
    # user-agent style beyond `display`, so pinning these two is enough.
    body = ["font-family:weasyprint", "margin:0"]
    if rng.random() < 0.5:
        body[-1] = f"margin:{_sides(rng)}"
    blocks = "".join(_block(rng, 0) for _ in range(rng.randrange(1, 4)))
    return f"<style>{page}body{{{';'.join(body)}}}</style>{blocks}"


def random_cases(count: int = _RANDOM_COUNT, seed: int = _RANDOM_SEED) -> list[tuple[str, str]]:
    """A deterministic corpus of random documents over the implemented features.

    The curated cases above are hand-picked, so they check the shapes someone
    thought of. These are drawn from the same property set at random, which is
    what turns "the cases we thought of agree" into "the cases we checked
    agree". The seed is fixed so the committed fixture rebuilds byte for byte,
    and the corpus is written to the crate so `tests/layout.rs` replays it: a
    change that keeps the curated cases green but moves a random box fails CI.
    """
    rng = random.Random(seed)
    return [(f"random_{index:04d}", random_document(rng)) for index in range(count)]


# --- Selector corpus -------------------------------------------------------
#
# The block/inline corpus above uses one rule per element kind and never a
# combinator, so it says nothing about how a selector is matched or how the
# cascade resolves two rules that both match. This corpus draws documents whose
# elements carry ids and classes and a stylesheet full of descendant and child
# selectors over them, so the box tree only comes out right if both the matching
# and the specificity ordering are right.
#
# The elements nest: a block holds blocks or inline runs, and an inline run holds
# nested spans, so a selector over an inline ancestor (a span inside a span) has
# something to match. A rule whose subject may be inline carries `font-size`, an
# inherited property that moves the text, so an inline match is visible in the
# tree; a box-model declaration there would be ignored by both engines and the
# broken ancestry would pass unnoticed.

# The tags the selector corpus uses. All three are UA-neutral in WeasyPrint's
# test stylesheet: `div` and `section` are blocks with no margin, `span` is
# inline, and none of them carries a font or size the crate does not model. A
# tag with a UA margin (`p`, `h1`) or a list marker (`li`) would compare a value
# the crate does not model, so they are left out.
_SELECTOR_TAGS = ["div", "section"]
_SELECTOR_INLINE_TAGS = ["span"]
_SELECTOR_CLASSES = ["a", "b", "c", "wide", "box"]

# The random selector corpus: a few thousand documents, fixed seed. The count is
# smaller than the block/inline corpus because each document carries a whole
# stylesheet, so it covers more ground per case.
_SELECTOR_RANDOM_COUNT = 1200
_SELECTOR_RANDOM_SEED = 20261116


def _selector_rule(rng: random.Random) -> tuple[str, bool]:
    """One random complex selector, and whether its subject is a block.

    The subject is the rightmost compound. Whether it is a block decides which
    declarations the rule may carry: a box-model declaration on an inline box
    (margin, padding) is not modelled by the crate, so a rule whose subject may
    be inline is limited to `width` and `height`, which both engines ignore on a
    non-replaced inline.
    """
    depth = rng.randrange(1, 4)
    parts = []
    for index in range(depth):
        tag = rng.choice(_SELECTOR_TAGS + _SELECTOR_INLINE_TAGS + [""])
        if index == depth - 1 and rng.random() < 0.6:
            tag = rng.choice(_SELECTOR_TAGS + _SELECTOR_INLINE_TAGS)
        compound = tag
        if rng.random() < 0.5:
            compound += "." + rng.choice(_SELECTOR_CLASSES)
        if rng.random() < 0.2:
            compound += "." + rng.choice(_SELECTOR_CLASSES)
        if rng.random() < 0.25:
            compound += "#" + rng.choice(["x", "y", "z"])
        if not compound:
            compound = rng.choice(_SELECTOR_TAGS + _SELECTOR_INLINE_TAGS)
        parts.append(compound)
    out = parts[0]
    for part in parts[1:]:
        out += rng.choice([" ", " > ", ">", "  "]) + part
    # The subject is a block only when it names a block tag explicitly; a bare
    # class or id could land on a span.
    subject_is_block = parts[-1].startswith(tuple(_SELECTOR_TAGS))
    return out, subject_is_block


def _selector_declarations(rng: random.Random, block: bool) -> str:
    """A declaration list that moves a box, from the supported set only.

    A block subject takes the box-model properties, which move the box. An
    inline subject takes `font-size`, an inherited property that moves the text
    it holds, so a match on an inline element is visible in the tree; `width`
    and `height` are kept too, but both engines ignore them on a non-replaced
    inline, so they are not what makes an inline match observable.
    """
    out = []
    if block:
        if rng.random() < 0.85:
            out.append(f"width:{rng.choice([10, 20, 30, 40, 50, 60, 80, 120])}px")
        if rng.random() < 0.4:
            out.append(f"height:{rng.randrange(0, 40)}px")
        if rng.random() < 0.4:
            out.append(f"margin:{rng.randrange(0, 20)}px")
        if rng.random() < 0.3:
            out.append(f"padding:{rng.randrange(0, 10)}px")
    else:
        out.append(f"font-size:{rng.choice([8, 10, 12, 16, 20, 24])}px")
        if rng.random() < 0.3:
            out.append(f"width:{rng.choice([10, 20, 30, 40])}px")
    if not out:
        out.append("width:25px")
    return ";".join(out)


def _selector_rules(rng: random.Random) -> str:
    """A stylesheet of random complex selectors, some of them conflicting."""
    rules = []
    for _ in range(rng.randrange(2, 7)):
        selector, block = _selector_rule(rng)
        rules.append(f"{selector}{{{_selector_declarations(rng, block)}}}")
    return "".join(rules)


def _selector_tree(rng: random.Random, depth: int, serial: list[int]) -> str:
    """A block element with an id and classes, holding blocks or inline runs."""
    tag = rng.choice(_SELECTOR_TAGS)
    serial[0] += 1
    attributes = []
    if rng.random() < 0.4:
        attributes.append(f'id="{rng.choice(["x", "y", "z"])}"')
    if rng.random() < 0.6:
        count = rng.randrange(1, 3)
        classes = " ".join(rng.choice(_SELECTOR_CLASSES) for _ in range(count))
        attributes.append(f'class="{classes}"')
    attribute = (" " + " ".join(attributes)) if attributes else ""
    if depth < 2 and rng.random() < 0.5:
        inner = "".join(_selector_tree(rng, depth + 1, serial) for _ in range(rng.randrange(1, 3)))
    elif rng.random() < 0.6:
        inner = _selector_inline(rng)
    else:
        inner = ""
    return f"<{tag}{attribute}>{inner}</{tag}>"


def _selector_inline(rng: random.Random, depth: int = 0) -> str:
    """A run of text and inline elements, sometimes nested.

    A span may hold another inline run, so a descendant or child selector over
    an inline ancestor (a span inside a span) has something to match. Nesting
    stops after one level, which is enough for the chain to matter without the
    tree growing without bound. Every span holds text, so no empty inline box is
    produced, which is a separate difference from the reference.
    """
    parts = []
    for _ in range(rng.randrange(1, 3)):
        roll = rng.random()
        if roll < 0.45:
            classes = rng.choice(_SELECTOR_CLASSES)
            parts.append(f'<span class="{classes}">ab</span>')
        elif roll < 0.75 and depth < 1:
            classes = rng.choice(_SELECTOR_CLASSES)
            inner = _selector_inline(rng, depth + 1)
            parts.append(f'<span class="{classes}">{inner}</span>')
        else:
            parts.append("cd")
    return "".join(parts)


def selector_document(rng: random.Random) -> str:
    """One random document whose layout depends on selector matching."""
    page = PAGE % (200, _NO_CLIP_HEIGHT)
    rules = _selector_rules(rng)
    serial = [0]
    blocks = "".join(_selector_tree(rng, 0, serial) for _ in range(rng.randrange(1, 3)))
    return f"<style>{page}body{{margin:0;font-family:weasyprint}}{rules}</style>{blocks}"


def selector_random_cases(
    count: int = _SELECTOR_RANDOM_COUNT, seed: int = _SELECTOR_RANDOM_SEED
) -> list[tuple[str, str]]:
    """A deterministic corpus of random selector-matching documents."""
    rng = random.Random(seed)
    return [(f"sel_random_{index:04d}", selector_document(rng)) for index in range(count)]


def selector_cases() -> list[tuple[str, str]]:
    """The hand-picked selector cases: the shapes someone thought to write down.

    The tags are `div`, `section` and `span`, which WeasyPrint's test stylesheet
    leaves with no margin, padding or font of their own, so a box that differs is
    a selector bug and not a user-agent rule the crate does not model.
    """
    page = PAGE % (200, 200)
    body = "body{margin:0;font-family:weasyprint}"
    return [
        # A descendant selector reaches a grandchild and leaves a sibling alone.
        ("descendant_matches_nested", f"<style>{page}{body}"
                                      "div section{width:30px;height:10px}</style>"
                                      "<div><div><section></section></div></div>"
                                      "<section></section>"),
        # The child combinator stops at the immediate parent: only the `section`
        # whose parent is the `div` is matched, not the one nested in a section.
        ("child_stops_at_parent", f"<style>{page}{body}"
                                  "div > section{width:40px;height:10px}</style>"
                                  "<div><section><section></section></section></div>"),
        # A descendant chain of depth three.
        ("descendant_chain_three", f"<style>{page}{body}"
                                   "div div section{width:44px;height:10px}</style>"
                                   "<div><div><section></section></div></div>"),
        # A class in the middle of a chain.
        ("descendant_with_class", f"<style>{page}{body}"
                                  "div.card section{width:33px;height:10px}</style>"
                                  '<div class="card"><section></section></div>'
                                  "<div><section></section></div>"),
        # Specificity: `div section` (0,0,2) beats a later `section` (0,0,1).
        ("specificity_beats_later", f"<style>{page}{body}"
                                    "section{width:30px;height:10px}"
                                    "div section{width:60px}</style>"
                                    "<div><section></section></div>"),
        # Specificity: a class (0,1,0) beats two tags (0,0,2).
        ("class_beats_two_tags", f"<style>{page}{body}"
                                 "div section{width:30px;height:10px}"
                                 ".c{width:70px}</style>"
                                 '<div><section class="c"></section></div>'),
        # Specificity: an id (1,0,0) beats a class and any number of tags.
        ("id_beats_class", f"<style>{page}{body}"
                           '.c{width:30px;height:10px}#x{width:70px}</style>'
                           '<div><section id="x" class="c"></section></div>'),
        # Equal specificity falls back to source order: the later rule wins.
        ("equal_specificity_later_wins", f"<style>{page}{body}"
                                         "div section{width:30px;height:10px}"
                                         "div section{width:70px}</style>"
                                         "<div><section></section></div>"),
        # The universal selector contributes no specificity, so a later tag rule
        # of the same specificity still wins, and `*` alone matches everything.
        ("universal_loses_to_tag", f"<style>{page}{body}"
                                   "*{width:30px;height:10px}"
                                   "section{width:70px}</style>"
                                   "<div><section></section></div>"),
        # Backtracking: `div > section div` must skip the inner section that has
        # no div above it and use the outer one. The subject is a block so the
        # match shows up as a width; an inline subject would only differ by the
        # empty line box both engines agree to leave out.
        ("backtracking_child_step", f"<style>{page}{body}"
                                    "div > section div{width:18px;height:10px}</style>"
                                    "<div><section><section><div></div>"
                                    "</section></section></div>"),
        # A comma-separated list where one selector is complex.
        ("selector_list_mixed", f"<style>{page}{body}"
                                "span, div > section{width:20px;height:10px}</style>"
                                "<div><section></section></div>"),
        # An inline `style` attribute wins over a sheet rule that also matches,
        # which both engines agree on for a class selector. (WeasyPrint gives the
        # attribute id specificity, so it differs from an id selector; the crate
        # follows the spec and lets the attribute win, so that pair is left out.)
        ("inline_beats_class_selector", f"<style>{page}{body}"
                                        ".c{width:30px;height:10px}</style>"
                                        '<div><section class="c" style="width:90px">'
                                        "</section></div>"),
        # A selector that matches nothing leaves the box at its default width.
        ("selector_matches_nothing", f"<style>{page}{body}"
                                     "section > div{width:30px;height:10px}</style>"
                                     "<div><section></section></div>"),
        # A descendant selector over an inline ancestor: the inner span's
        # `font-size` is inherited by its text, so the match shows as a taller,
        # wider line. A chain that dropped the outer span would leave the text at
        # the default size and disagree here.
        ("inline_ancestor_descendant", f"<style>{page}{body}"
                                       "span span{font-size:24px}</style>"
                                       "<div><span>a<span>b</span></span></div>"),
        # The child combinator at the inline level: the inner span's parent is a
        # span, so the rule matches and the text grows.
        ("inline_ancestor_child", f"<style>{page}{body}"
                                  "span > span{font-size:24px}</style>"
                                  "<div><span>a<span>b</span></span></div>"),
        # A descendant selector over an inline ancestor that removes the subject:
        # `display` is not inherited, so only the inner span goes and its text
        # disappears. A broken chain would leave it visible.
        ("inline_ancestor_display_none", f"<style>{page}{body}"
                                         "div span span{display:none}</style>"
                                         "<div><span>a<span>b</span></span></div>"),
        # The child step must be the immediate parent, not any ancestor: the
        # outer span's parent is a `section`, so `div > span` never matches and
        # the inner span keeps the default size. Read as a descendant, the rule
        # would reach the inner span through the outer one and grow the text.
        ("inline_child_needs_immediate_parent", f"<style>{page}{body}"
                                                "div > span span{font-size:24px}</style>"
                                                "<div><section><span>a<span>b</span>"
                                                "</span></section></div>"),
        # A non-ASCII tag name: the HTML parser keeps `café` as written, so a
        # rule for it must parse and match. The subject is inline, so `font-size`
        # is what makes the match visible.
        ("non_ascii_tag_matches", f"<style>{page}{body}"
                                  "café{font-size:24px}</style>"
                                  "<div><café>ab</café></div>"),
    ]


def all_cases(random_count: int = 0) -> list[tuple[str, str]]:
    """The curated corpus, and optionally the random one after it."""
    return cases() + (random_cases(random_count) if random_count else [])


def cases() -> list[tuple[str, str]]:
    """The hand-picked corpus: (case id, HTML). Every case is self-contained.

    Text is included. Laying out a line needs font metrics, so the cases load
    the reference's own test font (`weasyprint.otf`, Ahem-derived and monospace)
    rather than relying on whatever is installed: every glyph advances one em, so
    line breaking is arithmetic and the fixture is the same on every machine.
    """
    page = PAGE % (200, 200)
    return [
        ("block_basic", f"<style>{page}body{{margin:0}}div{{width:50px;height:20px}}</style>"
                        "<div></div>"),
        ("block_two", f"<style>{page}body{{margin:0}}div{{width:50px;height:20px}}</style>"
                      "<div></div><div style=\"width:80px\"></div>"),
        ("block_margin", f"<style>{page}body{{margin:0}}div{{width:50px;height:20px;margin:10px}}</style>"
                         "<div></div><div></div>"),
        ("block_padding", f"<style>{page}body{{margin:0}}div{{width:50px;height:20px;padding:5px}}</style>"
                          "<div></div>"),
        ("block_border", f"<style>{page}body{{margin:0}}"
                         "div{width:50px;height:20px;border:2px solid}</style><div></div>"),
        ("block_auto_width", f"<style>{page}body{{margin:0}}div{{height:20px}}</style><div></div>"),
        ("block_percent_width", f"<style>{page}body{{margin:0}}div{{width:50%;height:20px}}</style>"
                                "<div></div>"),
        ("block_nested", f"<style>{page}body{{margin:0}}"
                         "div{width:100px;height:50px}</style>"
                         "<div><div style=\"width:40px;height:10px\"></div></div>"),
        ("body_margin", f"<style>{page}body{{margin:20px}}div{{width:50px;height:20px}}</style>"
                        "<div></div>"),
        ("margin_collapse", f"<style>{page}body{{margin:0}}"
                            "div{height:20px;margin-bottom:10px}"
                            "p{margin:0;height:10px}</style><div></div><p></p>"),
        ("margin_auto", f"<style>{page}body{{margin:0}}"
                        "div{width:50px;height:20px;margin-left:auto;margin-right:auto}</style>"
                        "<div></div>"),
        ("margin_negative", f"<style>{page}body{{margin:0}}"
                            "div{width:50px;height:20px;margin-left:-10px}</style><div></div>"),
        ("display_none", f"<style>{page}body{{margin:0}}"
                         "div{width:50px;height:20px}</style>"
                         "<div></div><div style=\"display:none\"></div><div></div>"),
        ("border_widths", f"<style>{page}body{{margin:0}}"
                          "div{width:50px;height:20px;border-width:1px 2px 3px 4px;"
                          "border-style:solid}</style><div></div>"),
        ("min_width", f"<style>{page}body{{margin:0}}div{{width:10px;min-width:60px;height:20px}}</style>"
                      "<div></div>"),
        ("max_width", f"<style>{page}body{{margin:0}}div{{width:120px;max-width:40px;height:20px}}</style>"
                      "<div></div>"),
        # Inline layout. The font is monospace at one em per glyph, so every
        # width below is the character count times the font size.
        ("text_one_line", f"<style>{page}body{{margin:0}}div{{font-family:weasyprint}}</style>"
                          "<div>hello</div>"),
        ("text_font_size", f"<style>{page}body{{margin:0}}"
                           "div{font-family:weasyprint;font-size:10px}</style><div>abc</div>"),
        ("text_wrap", f"<style>{page}body{{margin:0}}"
                      "div{font-family:weasyprint;width:40px}</style><div>hello world</div>"),
        ("text_wrap_mid_word", f"<style>{page}body{{margin:0}}"
                               "div{font-family:weasyprint;width:48px}</style><div>abcdefgh</div>"),
        ("text_spaces_collapse", f"<style>{page}body{{margin:0}}"
                                 "div{font-family:weasyprint}</style><div>a    b</div>"),
        ("text_trim", f"<style>{page}body{{margin:0}}"
                      "div{font-family:weasyprint}</style><div>   a   </div>"),
        ("text_newline", f"<style>{page}body{{margin:0}}"
                         "div{font-family:weasyprint}</style><div>a\nb</div>"),
        ("text_line_height_px", f"<style>{page}body{{margin:0}}"
                                "div{font-family:weasyprint;line-height:30px}</style><div>abc</div>"),
        ("text_line_height_number", f"<style>{page}body{{margin:0}}"
                                    "div{font-family:weasyprint;line-height:2}</style><div>abc</div>"),
        ("text_inline", f"<style>{page}body{{margin:0}}"
                        "div{font-family:weasyprint}</style><div>ab<i>cd</i>ef</div>"),
        ("text_inline_nested", f"<style>{page}body{{margin:0}}"
                               "div{font-family:weasyprint}</style><div>a<b><i>bc</i></b>d</div>"),
        ("text_inline_wrap", f"<style>{page}body{{margin:0}}"
                             "div{font-family:weasyprint;width:64px}</style>"
                             "<div>ab<i>cd ef</i>gh</div>"),
        ("text_inline_font_size", f"<style>{page}body{{margin:0}}"
                                  "div{font-family:weasyprint}</style>"
                                  "<div>a<span style=\"font-size:24px\">b</span>c</div>"),
        ("text_empty", f"<style>{page}body{{margin:0}}"
                       "div{font-family:weasyprint}</style><div></div>"),
        ("text_whitespace_only", f"<style>{page}body{{margin:0}}"
                                 "div{font-family:weasyprint}</style><div>   </div>"),
        ("text_between_blocks", f"<style>{page}body{{margin:0}}"
                                "div{font-family:weasyprint}</style>"
                                "<div>a</div>   <div>b</div>"),
        # text-align. Each line is placed within the block's content box, so the
        # offset is a function of the line's own width and the alignment.
        ("text_align_center", f"<style>{page}body{{margin:0}}"
                              "div{font-family:weasyprint;text-align:center}</style>"
                              "<div>ab</div>"),
        ("text_align_right", f"<style>{page}body{{margin:0}}"
                             "div{font-family:weasyprint;text-align:right}</style>"
                             "<div>ab</div>"),
        ("text_align_end", f"<style>{page}body{{margin:0}}"
                           "div{font-family:weasyprint;text-align:end}</style>"
                           "<div>ab</div>"),
        ("text_align_center_wrap", f"<style>{page}body{{margin:0}}"
                                   "div{font-family:weasyprint;text-align:center;width:64px}</style>"
                                   "<div>aa bb cc</div>"),
        ("text_align_right_wrap", f"<style>{page}body{{margin:0}}"
                                  "div{font-family:weasyprint;text-align:right;width:64px}</style>"
                                  "<div>aa bb cc</div>"),
        ("text_align_justify", f"<style>{page}body{{margin:0}}"
                               "div{font-family:weasyprint;text-align:justify;width:96px}</style>"
                               "<div>aa bb cc</div>"),
        ("text_align_justify_last_line", f"<style>{page}body{{margin:0}}"
                                         "div{font-family:weasyprint;text-align:justify;width:96px}"
                                         "</style><div>aa bb cc dd</div>"),
        ("text_align_last_overrides", f"<style>{page}body{{margin:0}}"
                                      "div{font-family:weasyprint;text-align:center;"
                                      "text-align-last:right;width:64px}</style>"
                                      "<div>aa bb cc</div>"),
        ("text_align_inherited", f"<style>{page}body{{margin:0}}"
                                 "div{font-family:weasyprint;text-align:center}</style>"
                                 "<div><span>ab</span></div>"),
        ("text_align_overflows", f"<style>{page}body{{margin:0}}"
                                 "div{font-family:weasyprint;text-align:center;width:16px}"
                                 "</style><div>abcd</div>"),
        ("text_align_start", f"<style>{page}body{{margin:0}}"
                             "div{font-family:weasyprint;text-align:start;width:64px}</style>"
                             "<div>ab</div>"),
        ("text_align_left", f"<style>{page}body{{margin:0}}"
                            "div{font-family:weasyprint;text-align:left;width:64px}</style>"
                            "<div>ab</div>"),
        ("text_align_justify_last_justify", f"<style>{page}body{{margin:0}}"
                                            "div{font-family:weasyprint;text-align:justify;"
                                            "text-align-last:justify;width:96px}</style>"
                                            "<div>aa bb cc dd</div>"),
        ("text_align_last_auto", f"<style>{page}body{{margin:0}}"
                                 "div{font-family:weasyprint;text-align:center;"
                                 "text-align-last:auto;width:64px}</style>"
                                 "<div>aa bb cc</div>"),
        ("text_align_justify_single_line", f"<style>{page}body{{margin:0}}"
                                           "div{font-family:weasyprint;text-align:justify;"
                                           "width:96px}</style><div>aa bb</div>"),
        # text-indent. Only the first line is shifted, and the shift narrows the
        # width that line wraps against.
        ("text_indent", f"<style>{page}body{{margin:0}}"
                        "div{font-family:weasyprint;text-indent:20px;width:200px}</style>"
                        "<div>ab cd</div>"),
        ("text_indent_wrap", f"<style>{page}body{{margin:0}}"
                             "div{font-family:weasyprint;text-indent:20px;width:64px}</style>"
                             "<div>aa bb cc dd</div>"),
        ("text_indent_percent", f"<style>{page}body{{margin:0}}"
                                "div{font-family:weasyprint;text-indent:10%;width:64px}</style>"
                                "<div>aa bb cc</div>"),
        ("text_indent_negative", f"<style>{page}body{{margin:0}}"
                                 "div{font-family:weasyprint;text-indent:-10px;width:200px}"
                                 "</style><div>ab cd</div>"),
        ("text_indent_center", f"<style>{page}body{{margin:0}}"
                               "div{font-family:weasyprint;text-indent:20px;text-align:center;"
                               "width:200px}</style><div>ab cd</div>"),
        ("text_indent_right", f"<style>{page}body{{margin:0}}"
                              "div{font-family:weasyprint;text-indent:20px;text-align:right;"
                              "width:200px}</style><div>ab cd</div>"),
        ("text_indent_justify", f"<style>{page}body{{margin:0}}"
                                "div{font-family:weasyprint;text-indent:20px;text-align:justify;"
                                "width:96px}</style><div>aa bb cc dd</div>"),
        # A word that crosses an inline boundary has no space in it, so it is one
        # unbreakable word: `ab<span>cd</span>` wraps as `abcd`, not as `ab` and
        # `cd`. The span box moves to the next line with its text.
        ("inline_word_break", f"<style>{page}body{{margin:0}}"
                              "div{font-family:weasyprint;width:78px;font-size:16px}</style>"
                              "<div>ipsum sit a ab<span>lorem ipsum</span></div>"),
        # A nested inline box with a larger font raises the line even when its
        # own text is smaller, because the box itself is on the line.
        ("inline_nested_font", f"<style>{page}body{{margin:0}}"
                               "div{font-family:weasyprint}</style>"
                               '<div><span style="font-size:24px">'
                               '<span style="font-size:16px">a</span></span></div>'),
        # A run of inline content before the first block child is wrapped in an
        # anonymous block like any other run, not left as the element's own lines.
        ("inline_before_block", f"<style>{page}body{{margin:0}}"
                                "div{font-family:weasyprint}</style>"
                                '<div>a<p style="margin:0">b</p>c</div>'),
        ("collapse_sibling_bottom_top", f"<style>{page}body{{margin:0}}"
                                        "div{font-family:weasyprint}</style>"
                                        '<div style="height:5px;margin-bottom:30px"></div>'
                                        '<div style="height:5px;margin-top:10px"></div>'),
        ("collapse_through_run", f"<style>{page}body{{margin:0}}"
                                 "div{font-family:weasyprint}</style>"
                                 '<div style="margin:20px"></div>'
                                 '<div style="margin:30px"></div>'),
        ("collapse_through_then_content", f"<style>{page}body{{margin:0}}"
                                          "div{font-family:weasyprint}</style>"
                                          '<div style="margin:76px 0"></div>'
                                          '<div style="height:5px"></div>'),
        ("collapse_parent_child_top", f"<style>{page}body{{margin:0}}"
                                      "div{font-family:weasyprint}</style>"
                                      '<div style="margin-top:20px">'
                                      '<div style="height:10px"></div></div>'),
        ("collapse_parent_child_bottom_padding", f"<style>{page}body{{margin:0}}"
                                                 "div{font-family:weasyprint}</style>"
                                                 '<div style="padding:32px">'
                                                 '<div style="margin-bottom:76px"></div></div>'),
        ("collapse_child_bottom_contained", f"<style>{page}body{{margin:0}}"
                                            "div{font-family:weasyprint}</style>"
                                            '<div><div style="height:5px;margin-bottom:30px"></div></div>'
                                            '<div style="height:5px"></div>'),
        # A line break inside nested inline boxes leaves the inner boxes on the
        # line, aligned on the shared baseline rather than at the parent's top.
        # A smaller nested box therefore sits lower than its parent, which the
        # absolute offsets have to reproduce.
        ("inline_empty_chain_nested", f"<style>{page}body{{margin:0}}"
                                      "div{font-family:weasyprint}</style>"
                                      '<div style="width:40px">world'
                                      '<span><span style="font-size:10px"> lorem dolor</span>'
                                      '</span></div>'),
        # A trailing collapsible space that overflows a line with a break in it
        # starts a line of its own, so the line it left is not last and takes
        # `text-align` rather than `text-align-last`. The single-word block is
        # the counter-case: with no break, the space stays and the line is last.
        ("align_last_trailing_space_overflow", f"<style>{page}body{{margin:0}}"
                                               "div{font-family:weasyprint;font-size:16px;"
                                               "width:90px;text-align:end;text-align-last:start}"
                                               "</style><div>ab cd </div><div>lorem </div>"),
        # The phantom line only happens when the trailing space shares its text
        # node with the line's content. A whitespace-only node is dropped from a
        # box the line keeps, so it leaves the empty box behind (above) rather
        # than a phantom line, and the line stays last. Same content and width as
        # the case above, so the two differ only in how the space was written.
        ("align_last_trailing_lone_space_keeps_line", f"<style>{page}body{{margin:0}}"
                                                      "div{font-family:weasyprint;font-size:16px;"
                                                      "width:90px;text-align:end;text-align-last:start}"
                                                      "</style><div>ab cd<span> </span></div>"),
        # When the space itself overflows but the word after it would fit without
        # it, the reference drops the space and keeps the word on the line. The
        # 24px space cannot follow `abc`, but the 16px `a` can, so the line is
        # `abca` rather than `abc` / `a`. This only holds while the box the space
        # closes is a single unbreakable word.
        ("drop_space_when_space_overflows", f"<style>{page}body{{margin:0}}"
                                            "div{font-family:weasyprint;font-size:16px;"
                                            "width:88px}</style>"
                                            '<div><span style="font-size:24px">abc </span>a</div>'),
        # The counter-case: the box before the space holds a break opportunity
        # (`a-b `) or an earlier space (`ab cd `), so the reference ends the line
        # at that opportunity and keeps the space. Same widths as the case above,
        # so only the presence of the opportunity separates them.
        ("keep_space_when_word_breaks", f"<style>{page}body{{margin:0}}"
                                        "div{font-family:weasyprint;font-size:8px;"
                                        "width:38px}</style>"
                                        '<div><span style="font-size:10px">a-b </span>a</div>'),
        ("keep_space_after_earlier_space", f"<style>{page}body{{margin:0}}"
                                           "div{font-family:weasyprint;font-size:8px;"
                                           "width:58px}</style>"
                                           '<div><span style="font-size:10px">ab cd </span>a</div>'),
        # A break is not allowed inside a numeric expression even where the
        # class-pair rule would permit it, so the whole expression overflows one
        # line rather than splitting. `3/4` is the fraction case: the pair rule
        # sees `Sy` before `Nu` and breaks, but Pango keeps the number whole. The
        # 32px block holds two 16px glyphs and not three, so a wrong rule splits
        # it into `3/` and `4`.
        ("numeric_fraction", f"<style>{page}body{{margin:0}}"
                             "div{font-family:weasyprint;font-size:16px;width:32px}</style>"
                             "<div>3/4</div>"),
        # A numeric prefix before an opening bracket and the number it opens:
        # `$(3` stays whole. The pair rule breaks after `$` because `$` is not in
        # its no-break set, but the prefix belongs to the number inside.
        ("numeric_prefix_open_bracket", f"<style>{page}body{{margin:0}}"
                                        "div{font-family:weasyprint;font-size:16px;width:32px}"
                                        "</style><div>$(3</div>"),
        # A closing bracket followed by a numeric suffix: `(3)$` stays whole. The
        # pair rule breaks after `)` before `$`, but the suffix closes the number
        # the bracket holds.
        ("numeric_close_bracket_postfix", f"<style>{page}body{{margin:0}}"
                                          "div{font-family:weasyprint;font-size:16px;width:32px}"
                                          "</style><div>(3)$</div>"),
        # The counter-case the fix must keep: a slash between letters is still a
        # break opportunity, so `a/b` wraps after the slash as before. Same width
        # as the fraction, so only the operands' classes separate them.
        ("numeric_slash_between_letters", f"<style>{page}body{{margin:0}}"
                                          "div{font-family:weasyprint;font-size:16px;width:32px}"
                                          "</style><div>a/b</div>"),
        # A break at the seam between two adjacent text nodes is decided from the
        # two characters beside it alone, not from the whole word. `3%` then
        # `$(3` in two spans breaks at the seam (`%` before `$`), where the same
        # text in one node does not, because the numeric rule needs the whole
        # expression and the seam sees only the pair.
        ("seam_numeric_breaks_between_nodes", f"<style>{page}body{{margin:0}}"
                                              "div{font-family:weasyprint;font-size:16px;width:32px}"
                                              "</style><div><span>3%</span><span>$(3</span></div>"),
        # The same characters in one text node stay whole: the counter-case that
        # separates the seam rule from the intra-word rule.
        ("seam_numeric_whole_in_one_node", f"<style>{page}body{{margin:0}}"
                                           "div{font-family:weasyprint;font-size:16px;width:32px}"
                                           "</style><div>3%$(3</div>"),
        # A whitespace-only text node between two inline elements is a space the
        # line has to see: `<span>a</span> <span>b</span>` wraps between the two
        # boxes with a zero-width text box left at the line end, not `ab` on one
        # line. The node must survive the parse rather than be dropped as
        # whitespace.
        ("whitespace_between_inline_elements", f"<style>{page}body{{margin:0}}"
                                               "div{font-family:weasyprint;font-size:16px;width:32px}"
                                               "</style><div><span>a</span> <span>b</span></div>"),
        # The counter-case: no whitespace node, so the two boxes are one atom and
        # the line does not break between them.
        ("no_whitespace_between_inline_elements", f"<style>{page}body{{margin:0}}"
                                                  "div{font-family:weasyprint;font-size:16px;width:32px}"
                                                  "</style><div><span>a</span><span>b</span></div>"),
        # What is left of a dropped space depends on whether the space was the
        # whole of its text node. A whitespace-only node in its own inline box
        # keeps that box and an empty text box inside it; a space inside a node
        # that holds text is stripped from a box the line still needs, so nothing
        # is left. The two cases differ only in which side of `b` the space sits.
        ("dropped_lone_space_in_inline_box", f"<style>{page}body{{margin:0}}"
                                             "div{font-family:weasyprint;font-size:16px;width:16px}"
                                             "</style><div>a<span> </span>b</div>"),
        ("dropped_shared_space_in_inline_box", f"<style>{page}body{{margin:0}}"
                                               "div{font-family:weasyprint;font-size:16px;width:16px}"
                                               "</style><div>a<span> b</span></div>"),
        # A lone space dropped at the block level with the line's last content
        # inside an inline box leaves an empty text box for the block itself.
        ("dropped_lone_space_at_block", f"<style>{page}body{{margin:0}}"
                                        "div{font-family:weasyprint;font-size:16px;width:16px}"
                                        "</style><div><span>a</span> <span>b</span></div>"),
        # A lone space whose own box is a *strict ancestor* of the line's last
        # content is the same rule one level in: the empty text box belongs to
        # the box the space sat in, not to the block. The space's box is the
        # outer span and the content's box is the inner span, so the empty text
        # box appears inside the outer span, after the content.
        ("dropped_lone_space_in_ancestor_box", f"<style>{page}body{{margin:0}}"
                                               "div{font-family:weasyprint;font-size:16px;width:32px}"
                                               "</style><div><span><span>aa</span> <span>bb</span></span></div>"),
        # A whitespace-only node at the *end* of the block is dropped like one at
        # a break, and leaves the same empty box when it was the whole of its
        # text node: here the trailing box is the space's own inline box.
        ("dropped_trailing_lone_space_in_box", f"<style>{page}body{{margin:0}}"
                                               "div{font-family:weasyprint;font-size:16px;width:32px}"
                                               "</style><div><span>aa</span><span> </span></div>"),
        # The nested variant: the whole chain of inline boxes the space sat in is
        # left empty, each one wrapping the next down to the empty text box.
        ("dropped_lone_space_nested", f"<style>{page}body{{margin:0}}"
                                      "div{font-family:weasyprint;font-size:16px;width:16px}"
                                      "</style><div>a<span><span> </span></span>b</div>"),
        # A lone space dropped in a nested box whose font is smaller keeps the
        # inner box on the shared baseline, so its offset is the parent's ascent
        # less its own rather than the line's top.
        ("dropped_lone_space_smaller_box", f"<style>{page}body{{margin:0}}"
                                           "div{font-family:weasyprint;font-size:16px;width:16px}"
                                           "</style><div>a<span style=\"font-size:10px\"> </span>b</div>"),
        # A duplicate space removed from a whitespace-only node must not clear
        # the collapse state: the first space (16px) survives, so the line is
        # `a ` + `b` = 56px and fits in 60px, rather than keeping the 24px space
        # of the next element and wrapping. The state is read from the text
        # before the duplicate is dropped, as the reference does. A whitespace-only
        # node between two elements is what makes the state carry across a node
        # boundary; a run of spaces inside one node is folded before it is read.
        ("redundant_space_between_inline_elements", f"<style>{page}body{{margin:0}}"
                                                    "div{font-family:weasyprint;font-size:16px;width:60px}"
                                                    "</style><div><span>a </span> <span style=\"font-size:24px\"> b</span></div>"),
        # The same with the redundant space written as a line break, which
        # collapses to the same single space.
        ("redundant_space_newline_between_elements", f"<style>{page}body{{margin:0}}"
                                                     "div{font-family:weasyprint;font-size:16px;width:60px}"
                                                     "</style><div><span>a </span>\n <span style=\"font-size:24px\"> b</span></div>"),
        # A whitespace-only node whose space is dropped at the end of a line
        # still leaves the state set, so the next element's leading space is
        # folded away rather than starting a new line of its own.
        ("redundant_space_at_line_end", f"<style>{page}body{{margin:0}}"
                                        "div{font-family:weasyprint;font-size:16px;width:48px}"
                                        "</style><div><span>abcd </span> <span style=\"font-size:24px\"> b</span></div>"),
    ]


def walk(box, path: str, out: list[tuple]) -> None:
    """Append one row per box, depth-first, numbering children from zero.

    The full box model is recorded, not only size. Two points about the geometry,
    both established by probing the reference rather than by assumption:

    * The origins are the *absolute* `content_box_x`/`content_box_y`, not the
      relative `position_x`/`position_y`. A child's `position_x` is measured from
      its parent's content box, so comparing the relative values would let two
      differently-placed trees agree.
    * `margin_left`, `padding_left` and `border_left_width` are recorded
      separately. They have to be: `position_x` is the margin-box origin, so an
      `auto` margin is invisible in a position alone.
    """
    from weasyprint.formatting_structure import boxes

    name = type(box).__name__
    tag = getattr(box, "element_tag", None) or ""
    text = box.text if isinstance(box, boxes.TextBox) else ""
    out.append((
        path,
        tag,
        name,
        _accessor(box, "content_box_x"),
        _accessor(box, "content_box_y"),
        _num(getattr(box, "width", None)),
        _num(getattr(box, "height", None)),
        _num(getattr(box, "margin_left", None)),
        _num(getattr(box, "margin_right", None)),
        _num(getattr(box, "margin_top", None)),
        _num(getattr(box, "margin_bottom", None)),
        _num(getattr(box, "padding_left", None)),
        _num(getattr(box, "padding_right", None)),
        _num(getattr(box, "padding_top", None)),
        _num(getattr(box, "padding_bottom", None)),
        _num(getattr(box, "border_left_width", None)),
        _num(getattr(box, "border_right_width", None)),
        _num(getattr(box, "border_top_width", None)),
        _num(getattr(box, "border_bottom_width", None)),
        text,
    ))
    for index, child in enumerate(getattr(box, "children", ())):
        walk(child, f"{path}.{index}" if path else str(index), out)


def escape(text: str) -> str:
    """Make `text` safe for one TSV field, escaping the two characters that
    would otherwise split the row. The Rust reader reverses this."""
    return text.replace("\\", "\\\\").replace("\t", "\\t").replace("\n", "\\n")


def _accessor(box, name: str) -> str:
    """Call a coordinate accessor, tolerating a box that does not have it."""
    method = getattr(box, name, None)
    if method is None:
        return ""
    try:
        return _num(method())
    except Exception:
        return ""


def _num(value) -> str:
    """Format a measurement so it is stable across runs and platforms."""
    if value is None or not isinstance(value, (int, float)):
        return ""
    return f"{value:.2f}"


FONT_SHEET = "@font-face { src: url(weasyprint.otf); font-family: weasyprint }"


def find_font(explicit: str | None) -> pathlib.Path:
    """Locate the reference's test font, which makes text layout reproducible."""
    candidates = []
    if explicit:
        candidates.append(pathlib.Path(explicit))
    if env := os.environ.get("WEASYPRINT_DIR"):
        candidates.append(pathlib.Path(env) / "tests" / "resources" / "weasyprint.otf")
    candidates.append(pathlib.Path("/tmp/weasyprint/tests/resources/weasyprint.otf"))
    for path in candidates:
        if path.is_file():
            return path
    print(
        "the reference's test font (weasyprint.otf) was not found; pass --font, or "
        "set WEASYPRINT_DIR to a WeasyPrint checkout",
        file=sys.stderr,
    )
    raise SystemExit(2)


def render(html: str, font: pathlib.Path, font_config):
    """Lay out `html` and return the page box, using WeasyPrint's own entry point.

    The font is supplied as an extra stylesheet rather than written into each
    case, so the committed inputs stay clean and the cases read like HTML.
    """
    from weasyprint import HTML, CSS

    sheet = CSS(string=FONT_SHEET, base_url=str(font.parent) + "/", font_config=font_config)
    document = HTML(string=html).render(font_config=font_config, stylesheets=[sheet])
    page, = document.pages
    # The public `Page` object does not expose the tree; the reference's own
    # test helper (`tests/testing_utils.py`) reaches it the same way.
    return page._page_box


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", nargs="?", help="crate directory, e.g. crates/cssprint")
    parser.add_argument("--show", action="store_true", help="print each tree to stdout")
    parser.add_argument(
        "--random", type=int, default=_RANDOM_COUNT, metavar="N",
        help="how many random documents to generate (default %(default)s, the "
             "committed corpus); pass 0 for the curated list alone",
    )
    parser.add_argument("--font", help="path to the reference's weasyprint.otf")
    parser.add_argument(
        "--selectors", type=int, default=_SELECTOR_RANDOM_COUNT, metavar="N",
        help="how many random selector documents to generate (default "
             "%(default)s, the committed corpus); pass 0 for the curated list alone",
    )
    args = parser.parse_args()

    try:
        from weasyprint.text.fonts import FontConfiguration
    except ImportError:
        print("weasyprint is not installed; run `pip install weasyprint`", file=sys.stderr)
        return 2

    font = find_font(args.font)
    font_config = FontConfiguration()

    # The curated and random corpora are written to separate fixtures so the
    # hand-picked cases and the regression corpus can be read apart. Each is
    # laid out and walked here, and both are replayed by `tests/layout.rs`.
    curated = cases()
    random_corpus = random_cases(args.random) if args.random else []
    # The selector corpus is separate again, so a selector failure is not buried
    # among the box-model cases.
    selector_curated = selector_cases()
    selector_random = selector_random_cases(args.selectors) if args.selectors else []
    every_case = (
        all_cases(args.random) + selector_curated + selector_random
    )
    rows: list[tuple[str, ...]] = []
    per_case: list[tuple[str, list[tuple]]] = []
    for case, html in every_case:
        page = render(html, font, font_config)
        out: list[tuple] = []
        walk(page, "", out)
        per_case.append((case, out))
        for row in out:
            rows.append((case, row[0] or "0", *row[1:]))

    if args.show:
        for case, out in per_case:
            print(f"\n=== {case} ===")
            for row in out:
                path, tag, name, x, y, w, h = row[:7]
                ml, mr, mt, mb, pl, pr, pt, pb, bl, br, bt, bb = row[7:19]
                text = row[19]
                pad = "  " * path.count(".") if path else ""
                box_model = ""
                if ml or mr or mt or mb:
                    box_model += f" margin={ml},{mr},{mt},{mb}"
                if pl or pr or pt or pb:
                    box_model += f" padding={pl},{pr},{pt},{pb}"
                if bl or br or bt or bb:
                    box_model += f" border={bl},{br},{bt},{bb}"
                print(f"  {pad}{name} <{tag}> cx={x} cy={y} w={w} h={h}{box_model}"
                      + (f" text={text!r}" if text else ""))
        return 0

    if not args.crate:
        parser.error("give a crate directory to write into, or use --show")

    fixtures = pathlib.Path(args.crate) / "tests" / "fixtures"
    fixtures.mkdir(parents=True, exist_ok=True)

    # The Rust test needs the same inputs, so they are written beside the
    # expected tree. A tab or a newline would break the TSV, so they are escaped
    # and the test unescapes them; the layout fixture needs no escaping, because
    # whitespace collapsing means a text box never contains either.
    header = (
        "# WeasyPrint box tree: case\tpath\ttag\tbox\tcontent_x\tcontent_y"
        "\twidth\theight"
        "\tcontent_x\tcontent_y"
        "\tmargin_l\tmargin_r\tmargin_t\tmargin_b"
        "\tpadding_l\tpadding_r\tpadding_t\tpadding_b"
        "\tborder_l\tborder_r\tborder_t\tborder_b\ttext\n"
    )

    def write_corpus(
        inputs_name: str, tree_name: str, corpus: list[tuple[str, str]]
    ) -> None:
        """Write one corpus's inputs and its box tree."""
        ids = {case for case, _ in corpus}
        boxes = sum(len(out) for case, out in per_case if case in ids)
        inputs = fixtures / inputs_name
        with inputs.open("w") as handle:
            handle.write("# Generated by scripts/fuzz/cssprint/gen.py. Do not edit.\n")
            handle.write("# case\thtml\n")
            for case, html in corpus:
                handle.write(f"{case}\t{escape(html)}\n")
        target = fixtures / tree_name
        with target.open("w") as handle:
            handle.write("# Generated by scripts/fuzz/cssprint/gen.py. Do not edit.\n")
            handle.write(header)
            for row in rows:
                if row[0] in ids:
                    handle.write("\t".join(row) + "\n")
        print(f"{len(corpus)} inputs, {boxes} boxes -> {inputs}, {target}")

    write_corpus("cases.tsv", "layout.tsv", curated)
    write_corpus("fuzz_cases.tsv", "fuzz.tsv", random_corpus)
    write_corpus("selector_cases.tsv", "selector_layout.tsv", selector_curated)
    write_corpus("selector_fuzz_cases.tsv", "selector_fuzz.tsv", selector_random)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
