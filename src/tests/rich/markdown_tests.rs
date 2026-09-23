//! Markdown to sanitised HTML.

use crate::rich::markdown::asset;

fn html_of(source: &str) -> String {
    let crate::rich::RichAsset::Html { html } = asset(source) else {
        panic!("markdown always renders");
    };
    html
}

#[test]
fn commonmark_structure_renders() {
    let html = html_of("# Title\n\n- one\n- two\n\nSome **bold** text.");
    assert!(html.contains("<h1>Title</h1>"));
    assert!(html.contains("<li>one</li>"));
    assert!(html.contains("<strong>bold</strong>"));
}

#[test]
fn tables_and_strikethrough_are_enabled() {
    // Agent-authored option descriptions lean on both constantly; a table
    // that renders as a wall of pipes is worse than no table at all.
    let html = html_of("| a | b |\n| --- | --- |\n| 1 | 2 |\n\n~~gone~~");
    assert!(html.contains("<table>"));
    assert!(html.contains("<del>gone</del>"));
}

#[test]
fn embedded_html_is_sanitised() {
    let html = html_of("fine\n\n<script>alert(1)</script>\n\n<img src=x onerror=alert(1)>");
    assert!(!html.to_lowercase().contains("script"));
    assert!(!html.contains("onerror"));
    assert!(html.contains("fine"));
}

#[test]
fn links_survive_but_are_annotated() {
    let html = html_of("[docs](https://example.com)");
    assert!(html.contains(r#"href="https://example.com""#));
    assert!(
        html.contains("rel="),
        "ammonia annotates outbound links: {html}"
    );
}
