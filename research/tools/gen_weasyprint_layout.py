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

where `cx`/`cy` are the absolute content-box origin.

where `path` is the child-index route from the root (for example `0.1.0`), so a
Rust implementation can walk its own tree in the same order and compare
position by position. The margins, paddings and border widths are recorded
separately because `position_x` is the margin-box origin: an `auto` margin is
invisible in the position alone.

    python3 gen_weasyprint_layout.py            # writes tests/fixtures/layout.tsv
    python3 gen_weasyprint_layout.py --show     # print the tree for each case

Needs `pip install weasyprint`. Run from the repository root:

    python3 research/tools/gen_weasyprint_layout.py crates/cssprint

The corpus covers the box model (width, height, margin, padding, border), block
layout, inline layout, text alignment and `text-indent`. The committed fixture is
the hand-picked list in `cases()`. `--random N` appends a deterministic random
list from the same implemented property set, with a fixed seed, for differential
testing; the crate does not match the reference on every random document yet, so
those cases are not written to the fixture. Text is reproducible because the
cases load the reference's own test font: `weasyprint.otf` is derived from Ahem,
so every glyph advances exactly one em, the ascent is 819/1024 em, the descent
205/1024 em and the normal line height is one em. Line breaking is then
arithmetic, not a guess about installed fonts.

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

# The generator's random corpus draws from the features the crate implements, so
# a difference is a bug rather than an unimplemented property. Keep this list in
# step with the crate: adding a property here is how it gets checked.
_RANDOM_SEED = 20261116
_RANDOM_COUNT = 400
_ALIGNMENTS = ["left", "right", "center", "start", "end", "justify"]
_WORDS = ["a", "ab", "abc", "hello", "world", "lorem", "ipsum", "dolor", "sit"]


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
    for _ in range(rng.randrange(1, 4)):
        if depth < 2 and rng.random() < 0.4:
            style = ""
            if rng.random() < 0.3:
                style = f' style="font-size:{rng.choice([10, 16, 24])}px"'
            parts.append(f"<span{style}>{_inline(rng, depth + 1)}</span>")
        else:
            parts.append(_text(rng))
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
    # A tall page, so a random document stays on one page: the crate lays out at
    # the page width only and the fixture records absolute positions, so a case
    # that paginated would not be comparable.
    page = PAGE % (200, 5000)
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
    agree". The seed is fixed so the committed fixture rebuilds byte for byte.
    """
    rng = random.Random(seed)
    return [(f"random_{index:04d}", random_document(rng)) for index in range(count)]


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
        "--random", type=int, default=0, metavar="N",
        help="append N random documents, for differential testing; the crate does "
             "not match the reference on all of them yet, so they are not written "
             "to the committed fixture",
    )
    parser.add_argument("--font", help="path to the reference's weasyprint.otf")
    args = parser.parse_args()

    try:
        from weasyprint.text.fonts import FontConfiguration
    except ImportError:
        print("weasyprint is not installed; run `pip install weasyprint`", file=sys.stderr)
        return 2

    font = find_font(args.font)
    font_config = FontConfiguration()

    rows: list[tuple[str, ...]] = []
    corpus = all_cases(args.random)
    for case, html in corpus:
        page = render(html, font, font_config)
        out: list[tuple] = []
        walk(page, "", out)
        if args.show:
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
        for row in out:
            rows.append((case, row[0] or "0", *row[1:]))

    if args.show:
        return 0

    if not args.crate:
        parser.error("give a crate directory to write into, or use --show")

    fixtures = pathlib.Path(args.crate) / "tests" / "fixtures"
    fixtures.mkdir(parents=True, exist_ok=True)

    # The Rust test needs the same inputs, so they are written beside the
    # expected tree. A tab or a newline would break the TSV, so they are escaped
    # and the test unescapes them; the layout fixture needs no escaping, because
    # whitespace collapsing means a text box never contains either.
    inputs = fixtures / "cases.tsv"
    with inputs.open("w") as handle:
        handle.write("# Generated by research/tools/gen_weasyprint_layout.py."
                     " Do not edit.\n")
        handle.write("# case\thtml\n")
        for case, html in corpus:
            handle.write(f"{case}\t{escape(html)}\n")

    target = fixtures / "layout.tsv"
    with target.open("w") as handle:
        handle.write("# Generated by research/tools/gen_weasyprint_layout.py."
                     " Do not edit.\n")
        handle.write(
            "# WeasyPrint box tree: case\tpath\ttag\tbox\tcontent_x\tcontent_y"
            "\twidth\theight"
            "\tcontent_x\tcontent_y"
            "\tmargin_l\tmargin_r\tmargin_t\tmargin_b"
            "\tpadding_l\tpadding_r\tpadding_t\tpadding_b"
            "\tborder_l\tborder_r\tborder_t\tborder_b\ttext\n")
        for row in rows:
            handle.write("\t".join(row) + "\n")
    print(f"{len(rows)} boxes from {len(corpus)} cases -> {target}")
    print(f"{len(corpus)} inputs -> {inputs}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
