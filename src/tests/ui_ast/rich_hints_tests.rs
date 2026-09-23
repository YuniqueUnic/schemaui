//! `x-options` and `x-content`, plus the `mermaid` control.
//!
//! Same two halves as the control-hint tests: what well-formed declarations
//! produce, and what malformed ones are refused — a figure hint that is
//! silently dropped would leave an option looking fine with its diagram
//! missing, which is precisely the bug class strictness exists to prevent.

use serde_json::{Value, json};

use crate::rich::MAX_SOURCE_BYTES;
use crate::ui_ast::{FieldControl, RichContent, UiNodeKind, build_ui_ast};

fn node_for(property: Value) -> crate::ui_ast::UiNode {
    let schema = json!({ "type": "object", "properties": { "field": property } });
    let ast = build_ui_ast(&schema).expect("ui ast should build");
    ast.roots
        .into_iter()
        .find(|node| node.pointer == "/field")
        .expect("/field should exist")
}

fn rejection(property: Value) -> String {
    let schema = json!({ "type": "object", "properties": { "field": property } });
    let error = build_ui_ast(&schema).expect_err("this declaration should be rejected");
    error.to_string()
}

// ---------------------------------------------------------------- acceptance

#[test]
fn option_details_arrive_aligned_with_the_enum() {
    let node = node_for(json!({
        "type": "string",
        "enum": ["mesh", "star"],
        "x-control": "radio",
        "x-options": [
            { "label": "Mesh", "description": "全互联，延迟最低",
              "content": { "type": "mermaid", "source": "flowchart LR; A---B" } },
            {}
        ]
    }));

    let UiNodeKind::Field { enum_details, .. } = node.kind else {
        panic!("field kind");
    };
    let details = enum_details.expect("details present");
    assert_eq!(details.len(), 2);
    assert_eq!(details[0].label.as_deref(), Some("Mesh"));
    assert_eq!(details[0].description.as_deref(), Some("全互联，延迟最低"));
    assert_eq!(
        details[0].content,
        Some(RichContent::Mermaid {
            source: "flowchart LR; A---B".to_string()
        })
    );
    // An empty entry is legal: it means "nothing to add for this option".
    assert_eq!(details[1].label, None);
    assert_eq!(details[1].content, None);
}

#[test]
fn an_all_empty_options_list_is_not_carried() {
    let node = node_for(json!({
        "type": "string",
        "enum": ["a", "b"],
        "x-options": [{}, {}]
    }));
    let UiNodeKind::Field { enum_details, .. } = node.kind else {
        panic!("field kind");
    };
    assert_eq!(enum_details, None);
}

#[test]
fn a_figure_declared_on_the_node_lands_on_the_node() {
    let node = node_for(json!({
        "type": "string",
        "x-content": { "type": "svg", "source": "<svg viewBox=\"0 0 4 4\"/>" }
    }));
    assert_eq!(
        node.content,
        Some(RichContent::Svg {
            source: "<svg viewBox=\"0 0 4 4\"/>".to_string()
        })
    );
}

#[test]
fn the_mermaid_control_wants_a_string() {
    let node = node_for(json!({ "type": "string", "x-control": "mermaid" }));
    assert_eq!(node.control, Some(FieldControl::Mermaid));

    let error = rejection(json!({ "type": "integer", "x-control": "mermaid" }));
    assert!(error.contains("mermaid"), "{error}");
    assert!(error.contains("a string"), "{error}");
}

// ------------------------------------------------------------------ rejection

#[test]
fn option_details_need_an_enum() {
    let error = rejection(json!({
        "type": "string",
        "x-options": [{ "label": "orphan" }]
    }));
    assert!(error.contains("x-options"), "{error}");
    assert!(error.contains("enum"), "{error}");
}

#[test]
fn option_details_must_line_up_with_the_enum() {
    let error = rejection(json!({
        "type": "string",
        "enum": ["a", "b", "c"],
        "x-options": [{ "label": "only one" }]
    }));
    assert!(error.contains("1") && error.contains("3"), "{error}");
}

#[test]
fn content_must_name_a_known_kind_and_carry_a_source() {
    assert!(
        rejection(json!({
            "type": "string",
            "x-content": { "type": "png", "source": "…" }
        }))
        .contains("x-content")
    );

    assert!(
        rejection(json!({
            "type": "string",
            "x-content": { "type": "mermaid" }
        }))
        .contains("source")
    );

    assert!(
        rejection(json!({
            "type": "string",
            "x-content": { "type": "mermaid", "source": "" }
        }))
        .contains("non-empty")
    );
}

#[test]
fn oversized_sources_are_refused_at_the_door() {
    let huge = "x".repeat(MAX_SOURCE_BYTES + 1);
    let error = rejection(json!({
        "type": "string",
        "x-content": { "type": "markdown", "source": huge }
    }));
    assert!(error.contains("limit"), "{error}");
}

#[test]
fn a_segmented_control_cannot_promise_figures_it_cannot_show() {
    let error = rejection(json!({
        "type": "string",
        "enum": ["a", "b"],
        "x-control": "segmented",
        "x-options": [
            { "content": { "type": "mermaid", "source": "flowchart LR; A-->B" } },
            {}
        ]
    }));
    assert!(error.contains("segmented"), "{error}");
    assert!(error.contains("radio"), "{error}");
}

#[test]
fn option_detail_entries_must_be_objects_of_the_right_shapes() {
    assert!(
        rejection(json!({
            "type": "string", "enum": ["a"], "x-options": ["just a string"]
        }))
        .contains("objects")
    );

    assert!(
        rejection(json!({
            "type": "string", "enum": ["a"], "x-options": [{ "label": 7 }]
        }))
        .contains("label")
    );
}
