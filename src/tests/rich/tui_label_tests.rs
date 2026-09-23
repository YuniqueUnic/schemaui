//! What the terminal keeps of `x-options`: the label overrides.

use serde_json::json;

use crate::tui::model::{FieldKind, form_schema_from_ui_ast};
use crate::ui_ast::build_ui_ast;

#[test]
fn option_labels_are_overridden_in_place_and_survive_without_details() {
    let ast = build_ui_ast(&json!({
        "type": "object",
        "properties": {
            "topology": {
                "type": "string",
                "enum": ["mesh", "star", "ring"],
                "x-options": [
                    { "label": "全互联 Mesh" },
                    {},
                    { "label": "环形 Ring" }
                ]
            }
        }
    }))
    .expect("schema builds");

    let schema = form_schema_from_ui_ast(&ast);
    let field = schema
        .roots
        .iter()
        .flat_map(|root| &root.sections)
        .flat_map(|section| &section.fields)
        .find(|field| field.pointer == "/topology")
        .expect("field exists");
    let FieldKind::Enum { labels, .. } = &field.kind else {
        panic!("the field keeps its enum kind");
    };
    // The author's label where declared, the derived one everywhere else —
    // selection indexes by position, so the list keeps the enum's length.
    assert_eq!(
        labels,
        &[
            "全互联 Mesh".to_string(),
            "star".to_string(),
            "环形 Ring".to_string()
        ]
    );
}
