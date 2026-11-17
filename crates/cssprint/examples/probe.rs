//! probe: print the crate's box tree for one HTML string, in fixture columns.
use cssprint::{LayoutBox, Stylesheet, layout};

fn walk(b: &LayoutBox, path: &str) {
    let kind = match b.kind {
        cssprint::BoxKind::Block => "BlockBox",
        cssprint::BoxKind::Inline => "InlineBox",
        cssprint::BoxKind::Line => "LineBox",
        cssprint::BoxKind::Text => "TextBox",
        cssprint::BoxKind::Page => "PageBox",
    };
    let n = |v: f64| format!("{v:.3}");
    println!(
        "{path}\t{}\t{kind}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
        b.tag,
        n(b.content_x),
        n(b.content_y),
        n(b.width),
        n(b.height),
        n(b.margin.left),
        n(b.margin.right),
        n(b.margin.top),
        n(b.margin.bottom),
        n(b.padding.left),
        n(b.padding.right),
        n(b.padding.top),
        n(b.padding.bottom),
        n(b.border.left),
        n(b.border.right),
        n(b.border.top),
        n(b.border.bottom),
        b.text.clone().unwrap_or_default(),
    );
    for (i, c) in b.children.iter().enumerate() {
        walk(c, &format!("{path}.{i}"));
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let sheet = Stylesheet::parse(&args[1]).unwrap();
    let doc = layout(&args[2], &sheet, 200.0).unwrap();
    for (i, c) in doc.root.children.iter().enumerate() {
        walk(c, &i.to_string());
    }
}
