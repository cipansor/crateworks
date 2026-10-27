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

The corpus is deliberately small and covers only the CSS subset a first slice
should implement: the box model (width, height, margin, padding, border) and
block layout, plus the page size. It is meant to grow.
"""

from __future__ import annotations

import argparse
import pathlib
import sys

PAGE = "@page { size: %dpx %dpx; margin: 0 }"


def cases() -> list[tuple[str, str]]:
    """The corpus: (case id, HTML). Every case is self-contained.

    Text is deliberately absent. Laying out a line needs font metrics - "hello"
    at 16px is 39.64px wide in DejaVu Sans and something else in any other font -
    so a corpus that included it would only be reproducible on a machine with the
    same fonts installed. Inline layout and line breaking need a font-metrics
    oracle, which is the next slice, not this one.
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


def render(html: str):
    """Lay out `html` and return the page box, using WeasyPrint's own entry point."""
    from weasyprint import HTML

    document = HTML(string=html).render()
    page, = document.pages
    # The public `Page` object does not expose the tree; the reference's own
    # test helper (`tests/testing_utils.py`) reaches it the same way.
    return page._page_box


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("crate", nargs="?", help="crate directory, e.g. crates/cssprint")
    parser.add_argument("--show", action="store_true", help="print each tree to stdout")
    args = parser.parse_args()

    try:
        import weasyprint  # noqa: F401
    except ImportError:
        print("weasyprint is not installed; run `pip install weasyprint`", file=sys.stderr)
        return 2

    rows: list[tuple[str, ...]] = []
    for case, html in cases():
        page = render(html)
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
    # expected tree. Tabs and newlines would break the TSV, so both are
    # rejected rather than silently mangled.
    inputs = fixtures / "cases.tsv"
    with inputs.open("w") as handle:
        handle.write("# Generated by research/tools/gen_weasyprint_layout.py."
                     " Do not edit.\n")
        handle.write("# case\thtml\n")
        for case, html in cases():
            if "\t" in html or "\n" in html:
                print(f"case {case} contains a tab or newline; cannot be a TSV row",
                      file=sys.stderr)
                return 2
            handle.write(f"{case}\t{html}\n")

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
    print(f"{len(rows)} boxes from {len(cases())} cases -> {target}")
    print(f"{len(cases())} inputs -> {inputs}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
