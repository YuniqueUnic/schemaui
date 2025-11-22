use std::collections::HashMap;

use serde_json::json;

use crate::{
    tui::model::{CompositeField, CompositeMode, CompositeVariant, FieldKind, FieldSchema},
    tui::state::FieldState,
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
    assert_eq!(selected, 0, "first entry should be selected");
    assert_eq!(entries.len(), 2, "one entry per selected variant");
    assert!(
        entries[0].contains("Target object"),
        "first entry should summarize the 'Target object' variant: {:?}",
        entries[0]
    );
    assert!(
        entries[1].contains("Integer entry"),
        "second entry should summarize the 'Integer entry' variant: {:?}",
        entries[1]
    );
}

#[test]
fn composite_list_adds_entries_with_selected_variants() {
    let mut field = composite_list_field();

    // Add an entry explicitly as the target object variant
    assert!(
        field.composite_list_add_entry_with_variant(0),
        "should add entry for 'Target object' variant"
    );

    // Add another entry explicitly as the integer variant
    assert!(
        field.composite_list_add_entry_with_variant(2),
        "should add entry for 'Integer entry' variant"
    );

    let (entries, selected) = field
        .composite_list_panel()
        .expect("panel state must exist after adding entries");

    assert_eq!(entries.len(), 2, "expected two entries after explicit adds");
    assert_eq!(selected, 1, "last added entry should be selected");

    assert!(
        entries[0].contains("Target object"),
        "first entry should summarize the 'Target object' variant: {:?}",
        entries[0]
    );
    assert!(
        entries[1].contains("Integer entry"),
        "second entry should summarize the 'Integer entry' variant: {:?}",
        entries[1]
    );
}
