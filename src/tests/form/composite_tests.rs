use std::sync::Arc;

use serde_json::json;

use crate::{
    domain::{CompositeField, CompositeMode, CompositeVariant},
    form::CompositeState,
    form::field::components::ComponentPalette,
};

#[test]
fn seed_from_value_uses_enum_discriminator() {
    let variant_one = CompositeVariant {
        id: "variant_one".to_string(),
        title: "Variant 1".to_string(),
        description: None,
        schema: json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["alpha"]
                }
            }
        }),
        is_object: true,
    };
    let variant_two = CompositeVariant {
        id: "variant_two".to_string(),
        title: "Variant 2".to_string(),
        description: None,
        schema: json!({
            "type": "object",
            "properties": {
                "kind": {
                    "type": "string",
                    "enum": ["beta"]
                }
            }
        }),
        is_object: true,
    };

    let field = CompositeField {
        mode: CompositeMode::OneOf,
        variants: vec![variant_one, variant_two],
    };

    let mut state = CompositeState::new(
        "/preferences",
        &field,
        Arc::new(ComponentPalette::default()),
    );
    state
        .seed_from_value(&json!({
            "kind": "beta"
        }))
        .expect("seed from value");

    assert_eq!(state.active_indices(), vec![1]);
    assert_eq!(state.summary(), "Variant: Variant 2");
}

#[test]
fn non_object_variant_round_trips_through_wrapper() {
    let variant = CompositeVariant {
        id: "list".to_string(),
        title: "String List".to_string(),
        description: None,
        schema: json!({
            "type": "array",
            "items": {"type": "string"}
        }),
        is_object: false,
    };
    let field = CompositeField {
        mode: CompositeMode::OneOf,
        variants: vec![variant],
    };
    let palette = Arc::new(ComponentPalette::default());
    let mut state = CompositeState::new("/options", &field, Arc::clone(&palette));
    state
        .seed_from_value(&json!(["a", "b"]))
        .expect("seed array value");
    assert_eq!(
        state.build_value(true).expect("build value"),
        Some(json!(["a", "b"]))
    );

    let mut session = state
        .take_editor_session("/options", 0)
        .expect("editor session");
    assert!(
        session
            .schema
            .get("properties")
            .and_then(|props| props.get("__value"))
            .is_some(),
        "overlay schema should expose wrapped value field"
    );
    session
        .form_state
        .seed_from_value(&json!({"__value": ["z"]}));
    state.restore_editor_session(session);

    assert_eq!(
        state.build_value(true).expect("build updated value"),
        Some(json!(["z"]))
    );
}
