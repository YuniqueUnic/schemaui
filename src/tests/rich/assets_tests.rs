//! The asset walk: what `collect_assets` makes of a built AST.

use serde_json::json;

use crate::rich::{RichAsset, RichContentId, collect_assets};
use crate::ui_ast::{RichContent, build_ui_ast};

const FIGURE: &str = r#"<svg viewBox="0 0 24 24"><circle cx="12" cy="12" r="10"/></svg>"#;

/// An enum whose options each declare a figure, plus a markdown block on the
/// node itself — the shapes a rich schema actually takes.
fn rich_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "topology": {
                "type": "string",
                "enum": ["mesh", "star"],
                "x-control": "radio",
                "x-options": [
                    { "label": "Mesh",
                      "content": { "type": "svg", "source": FIGURE } },
                    { "label": "Star",
                      "content": { "type": "svg", "source": FIGURE } }
                ]
            },
            "notes": {
                "type": "string",
                "x-content": { "type": "markdown", "source": "# Notes\n\nRead me." }
            }
        }
    })
}

fn figure_id() -> RichContentId {
    RichContentId::of(&RichContent::Svg {
        source: FIGURE.to_string(),
    })
}

#[test]
fn declared_content_lands_in_the_map_keyed_by_id() {
    let ast = build_ui_ast(&rich_schema()).expect("schema builds");
    let assets = collect_assets(&ast).expect("assets should exist");

    let figure = assets
        .get(&figure_id())
        .expect("the option figure is addressable by its content id");
    assert!(matches!(figure, RichAsset::Svg { .. }));

    let markdown = assets
        .get(&RichContentId::of(&RichContent::Markdown {
            source: "# Notes\n\nRead me.".to_string(),
        }))
        .expect("node content is collected too");
    let RichAsset::Html { html } = markdown else {
        panic!("markdown renders to html");
    };
    assert!(html.contains("<h1>Notes</h1>"));
}

#[test]
fn the_same_figure_declared_twice_is_one_entry() {
    let ast = build_ui_ast(&rich_schema()).expect("schema builds");
    let assets = collect_assets(&ast).expect("assets should exist");
    // Two options, one figure source: the map holds it once, which is the
    // whole point of addressing content rather than counting occurrences.
    assert_eq!(assets.len(), 2, "one svg figure + one markdown block");
}

#[test]
fn a_schema_without_rich_content_yields_no_map() {
    let ast = build_ui_ast(&json!({
        "type": "object",
        "properties": { "plain": { "type": "string" } }
    }))
    .expect("schema builds");
    assert!(collect_assets(&ast).is_none());
}

#[cfg(feature = "mermaid")]
mod mermaid_assets {
    use super::*;

    #[test]
    fn a_valid_diagram_renders_both_theme_variants() {
        let ast = build_ui_ast(&json!({
            "type": "object",
            "properties": {
                "arch": {
                    "type": "string",
                    "enum": ["a"],
                    "x-options": [
                        { "content": { "type": "mermaid", "source": "flowchart LR; A-->B" } }
                    ]
                }
            }
        }))
        .expect("schema builds");
        let assets = collect_assets(&ast).expect("assets should exist");
        let id = RichContentId::of(&RichContent::Mermaid {
            source: "flowchart LR; A-->B".to_string(),
        });
        let RichAsset::Diagram { light, dark } = assets.get(&id).expect("diagram asset") else {
            panic!("a valid diagram renders as a diagram");
        };
        assert!(light.body.contains("<svg"));
        assert!(dark.body.contains("<svg"));
        // The themes must actually differ — a diagram that ignores its theme
        // would make the frontend's switch a no-op.
        assert_ne!(light.body, dark.body);
    }

    #[test]
    fn a_broken_diagram_reports_failure_with_the_source_still_usable() {
        let ast = build_ui_ast(&json!({
            "type": "object",
            "properties": {
                "arch": {
                    "type": "string",
                    "x-content": { "type": "mermaid", "source": "flowchart LR; A-->" }
                }
            }
        }))
        .expect("schema builds");
        let assets = collect_assets(&ast).expect("assets should exist");
        let id = RichContentId::of(&RichContent::Mermaid {
            source: "flowchart LR; A-->".to_string(),
        });
        let RichAsset::Failed { message } = assets.get(&id).expect("failure asset") else {
            panic!("an unparsable diagram is a failure, not an absence");
        };
        assert!(!message.is_empty());
    }
}

#[cfg(not(feature = "mermaid"))]
mod no_mermaid_feature {
    use super::*;

    #[test]
    fn diagrams_are_absent_but_other_content_renders() {
        // Absent, not failed: the build simply cannot render them, and the
        // frontend shows the source rather than an error.
        let ast = build_ui_ast(&json!({
            "type": "object",
            "properties": {
                "arch": {
                    "type": "string",
                    "x-content": { "type": "mermaid", "source": "flowchart LR; A-->B" }
                },
                "icon": {
                    "type": "string",
                    "x-content": { "type": "svg", "source": FIGURE }
                }
            }
        }))
        .expect("schema builds");
        let assets = collect_assets(&ast).expect("the svg content still renders");
        assert!(assets.contains_key(&figure_id()));
        assert_eq!(assets.len(), 1, "the mermaid entry is simply not there");
    }
}
