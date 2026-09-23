//! The SVG whitelist sanitiser.
//!
//! Two halves, like the hint tests: what survives a benign figure, and what
//! never survives a hostile one. The hostile corpus is the point of the
//! module — a schema can arrive from anywhere, and the output must be inert
//! by construction rather than trusted by convention.

use crate::rich::sanitize::sanitize;

fn body_of(source: &str) -> String {
    sanitize(source).expect("benign svg should sanitise").body
}

#[test]
fn a_benign_figure_survives_with_its_essence() {
    let svg = body_of(
        r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 50">
            <g transform="translate(10 5)">
                <rect x="0" y="0" width="10" height="5" fill="#f00"/>
                <text font-size="4">Hello &amp; welcome</text>
            </g>
        </svg>"##,
    );
    assert!(svg.starts_with("<svg"), "root survives: {svg}");
    assert!(
        svg.contains("<rect") && svg.contains("fill=\"#f00\""),
        "shapes keep their paint: {svg}"
    );
    assert!(svg.contains("translate(10 5)"), "transforms survive: {svg}");
    assert!(
        svg.contains("Hello &amp; welcome"),
        "text stays escaped exactly once: {svg}"
    );
}

#[test]
fn the_root_size_comes_from_the_view_box() {
    let svg = sanitize(r#"<svg viewBox="0 0 120 60"><rect width="1" height="1"/></svg>"#)
        .expect("should sanitise");
    assert_eq!(svg.width, Some(120.0));
    assert_eq!(svg.height, Some(60.0));
}

#[test]
fn a_self_closing_root_still_carries_its_size() {
    let svg = sanitize(r#"<svg viewBox="0 0 10 10"/>"#).expect("should sanitise");
    assert_eq!(svg.width, Some(10.0));
    assert!(svg.body.ends_with("/>"));
}

#[test]
fn script_handlers_and_foreign_objects_never_survive() {
    let svg = body_of(
        r##"<svg viewBox="0 0 10 10" onload="steal()">
            <script>alert(1)</script>
            <rect width="1" height="1" onclick="steal()"/>
            <foreignObject><body><p>html</p></body></foreignObject>
        </svg>"##,
    );
    assert!(!svg.to_lowercase().contains("script"));
    assert!(!svg.to_lowercase().contains("onload"));
    assert!(!svg.to_lowercase().contains("onclick"));
    assert!(!svg.contains("foreignObject"));
    assert!(!svg.contains("steal"));
    // The innocent sibling lives on.
    assert!(svg.contains("<rect"));
}

#[test]
fn style_image_and_comments_are_dropped() {
    let svg = body_of(
        r#"<svg viewBox="0 0 10 10">
            <style>rect { fill: url(https://exfil.example/x) }</style>
            <image href="https://tracker.example/pixel"/>
            <!-- a comment -->
            <rect width="1" height="1"/>
        </svg>"#,
    );
    assert!(!svg.contains("style"));
    assert!(!svg.contains("image"));
    assert!(!svg.contains("tracker"));
    assert!(!svg.contains("exfil"));
    assert!(!svg.contains("comment"));
    assert!(svg.contains("<rect"));
}

#[test]
fn dropped_elements_take_their_subtree_with_them() {
    // An allowed shape inside a dropped container must not resurface on the
    // other side of it.
    let svg = body_of(
        r#"<svg viewBox="0 0 10 10"><foreignObject><rect id="gone" width="1" height="1"/></foreignObject><rect id="kept" width="1" height="1"/></svg>"#,
    );
    assert!(!svg.contains("gone"));
    assert!(svg.contains("kept"));
}

#[test]
fn hrefs_survive_only_when_they_point_inside_the_document() {
    let svg = body_of(
        r##"<svg viewBox="0 0 10 10"><defs><linearGradient id="g"><stop stop-color="#000"/></linearGradient></defs><use href="#g"/><use xlink:href="https://evil.example/x"/></svg>"##,
    );
    assert!(svg.contains(r##"href="#g""##));
    assert!(!svg.contains("evil.example"));
}

#[test]
fn cdata_becomes_plainly_escaped_text() {
    let svg = body_of(r#"<svg viewBox="0 0 10 10"><text><![CDATA[a < b && c > d]]></text></svg>"#);
    assert!(
        svg.contains("a &lt; b &amp;&amp; c &gt; d"),
        "same characters, no CDATA left: {svg}"
    );
}

#[test]
fn a_document_that_is_not_svg_is_rejected() {
    let error = sanitize("<html><body>nope</body></html>").expect_err("not an svg document");
    assert!(error.contains("<svg>"), "{error}");
}

#[test]
fn malformed_markup_is_rejected_not_repaired() {
    assert!(sanitize("<svg><rect></svg>").is_err());
    assert!(sanitize("<svg><rect").is_err());
    assert!(sanitize("</svg>").is_err());
    assert!(sanitize("").is_err());
}
