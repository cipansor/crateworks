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
fn a_bad_page_width_is_rejected() {
    let sheet = Stylesheet::parse("").unwrap();
    assert!(layout("<div></div>", &sheet, 0.0).is_err());
    assert!(layout("<div></div>", &sheet, -1.0).is_err());
}
