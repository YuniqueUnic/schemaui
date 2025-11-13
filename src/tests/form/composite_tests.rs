use serde_json::json;

use crate::{
    domain::{CompositeField, CompositeMode, CompositeVariant},
    form::CompositeState,
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
    };

    let field = CompositeField {
        mode: CompositeMode::OneOf,
        variants: vec![variant_one, variant_two],
    };

    let mut state = CompositeState::new("/preferences", &field);
    state
        .seed_from_value(&json!({
            "kind": "beta"
        }))
        .expect("seed from value");

    assert_eq!(state.active_indices(), vec![1]);
    assert_eq!(state.summary(), "Variant: Variant 2");
}
