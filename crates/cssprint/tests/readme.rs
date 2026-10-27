//! Runs the README examples, so the documentation cannot drift from the API.

use cssprint::{Stylesheet, layout};

#[test]
fn readme_examples() {
    let sheet = Stylesheet::parse("div { width: 50px; height: 20px }").unwrap();
    let document = layout(
        r#"<div></div><div style="width: 80px"></div>"#,
        &sheet,
        200.0,
    )
    .unwrap();

    let body = document.body();
    assert_eq!(body.children.len(), 2);
    assert_eq!(body.children[0].width, 50.0);
    assert_eq!(body.children[0].content_y, 0.0);
    assert_eq!(body.children[1].width, 80.0);
    // The second div sits directly below the first, because the first is 20 tall.
    assert_eq!(body.children[1].content_y, 20.0);
}

#[test]
fn readme_text_example() {
    let sheet = Stylesheet::parse("div { font-family: weasyprint; width: 40px }").unwrap();
    let document = layout("<div>hello world</div>", &sheet, 200.0).unwrap();

    let div = &document.body().children[0];
    assert_eq!(div.height, 32.0); // two 16px lines
    assert_eq!(div.children[0].children[0].text.as_deref(), Some("hello"));
    assert_eq!(div.children[1].children[0].text.as_deref(), Some("world"));
}

#[test]
fn inline_style_overrides_the_sheet() {
    let sheet = Stylesheet::parse("div { width: 50px }").unwrap();
    let document = layout(r#"<div style="width: 90px"></div>"#, &sheet, 200.0).unwrap();
    assert_eq!(document.body().children[0].width, 90.0);
}

#[test]
fn a_class_selector_applies() {
    let sheet = Stylesheet::parse(".wide { width: 90px }").unwrap();
    let document = layout(r#"<div class="wide"></div>"#, &sheet, 200.0).unwrap();
    assert_eq!(document.body().children[0].width, 90.0);
}

#[test]
fn display_none_produces_no_box() {
    let sheet = Stylesheet::parse("").unwrap();
    let document = layout(
        r#"<div></div><div style="display:none"></div><div></div>"#,
        &sheet,
        200.0,
    )
    .unwrap();
    assert_eq!(document.body().children.len(), 2);
}

#[test]
fn whitespace_collapses_within_a_text_node() {
    let sheet = Stylesheet::parse("div { font-family: weasyprint }").unwrap();
    let document = layout("<div>a    b</div>", &sheet, 200.0).unwrap();
    let line = &document.body().children[0].children[0];
    assert_eq!(line.children.len(), 1);
    assert_eq!(line.children[0].text.as_deref(), Some("a b"));
}

#[test]
fn line_height_makes_the_line_taller_without_moving_the_text() {
    let sheet = Stylesheet::parse("div { font-family: weasyprint; line-height: 30px }").unwrap();
    let document = layout("<div>abc</div>", &sheet, 200.0).unwrap();
    let line = &document.body().children[0].children[0];
    assert_eq!(line.height, 30.0);
    // The 16px text sits centred in the 30px line, so it starts 7px down.
    assert_eq!(line.children[0].content_y, 7.0);
    assert_eq!(line.children[0].height, 16.0);
}

#[test]
fn font_size_is_inherited_from_the_block() {
    let sheet = Stylesheet::parse("div { font-family: weasyprint; font-size: 10px }").unwrap();
    let document = layout("<div>abc</div>", &sheet, 200.0).unwrap();
    let line = &document.body().children[0].children[0];
    assert_eq!(line.height, 10.0);
    assert_eq!(line.children[0].width, 30.0);
}

#[test]
fn a_bad_page_width_is_rejected() {
    let sheet = Stylesheet::parse("").unwrap();
    assert!(layout("<div></div>", &sheet, 0.0).is_err());
    assert!(layout("<div></div>", &sheet, -1.0).is_err());
}
