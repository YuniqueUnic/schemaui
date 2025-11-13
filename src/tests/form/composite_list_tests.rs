use std::collections::HashMap;

use serde_json::json;

use crate::{
    domain::{CompositeField, CompositeMode, CompositeVariant, FieldKind, FieldSchema},
    form::FieldState,
};

fn composite_list_field() -> FieldState {
    let variants = vec![
        CompositeVariant {
            id: "target".to_string(),
            title: "Target object".to_string(),
            description: None,
            schema: json!({
                "type": "object",
                "properties": {
                    "url": {"type": "string"}
                }
            }),
            is_object: true,
        },
        CompositeVariant {
            id: "string".to_string(),
            title: "String entry".to_string(),
            description: None,
            schema: json!({"type": "string"}),
            is_object: false,
        },
        CompositeVariant {
            id: "integer".to_string(),
            title: "Integer entry".to_string(),
            description: None,
            schema: json!({"type": "integer"}),
            is_object: false,
        },
    ];
    let template = CompositeField {
        mode: CompositeMode::AnyOf,
        variants,
    };
    FieldState::from_schema(FieldSchema {
        name: "deepItems".to_string(),
        path: vec!["deepItems".to_string()],
        pointer: "/deepItems".to_string(),
        title: "Deep Items".to_string(),
        description: None,
        section_id: "root".to_string(),
        kind: FieldKind::Array(Box::new(FieldKind::Composite(Box::new(template)))),
        required: false,
        default: None,
        metadata: HashMap::new(),
    })
}

#[test]
fn composite_list_popup_exposes_entry_variants() {
    let mut field = composite_list_field();
    assert!(field.composite_popup().is_none(), "no entry yet");
    assert!(
        field.ensure_composite_list_popup_entry(),
        "first popup should seed an entry"
    );
    let popup = field.composite_popup().expect("popup available");
    assert_eq!(popup.options.len(), 3);
    assert!(popup.multi, "anyOf should expose multi-select popup");
    assert_eq!(popup.options[0], "Target object");
}

#[test]
fn composite_list_selection_updates_summary() {
    let mut field = composite_list_field();
    field.ensure_composite_list_popup_entry();
    let popup = field.composite_popup().expect("popup");
    let mut toggles = vec![false; popup.options.len()];
    toggles[0] = true;
    toggles[2] = true;
    field.apply_composite_selection(0, Some(toggles));
    let (entries, selected) = field
        .composite_list_panel()
        .expect("panel state must exist");
    assert_eq!(selected, 0);
    assert!(
        entries[0].contains("Target object") && entries[0].contains("Integer entry"),
        "summary should mention active variants: {:?}",
        entries[0]
    );
}
