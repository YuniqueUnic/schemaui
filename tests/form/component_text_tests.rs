use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::domain::{FieldKind, FieldSchema};

fn integer_schema() -> FieldSchema {
    FieldSchema {
        name: "count".to_string(),
        path: vec!["count".to_string()],
        pointer: "/count".to_string(),
        title: "count".to_string(),
        description: None,
        section_id: "section".to_string(),
        kind: FieldKind::Integer,
        required: false,
        default: None,
        metadata: Default::default(),
    }
}

#[test]
fn text_component_supports_numeric_stepper() {
    let schema = integer_schema();
    let mut component = super::TextComponent::new(&schema);
    let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
    assert!(component.handle_key(&schema, &key));
    assert_eq!(component.display_value(&schema), "1");
    let key = KeyEvent::new(KeyCode::Left, KeyModifiers::NONE);
    assert!(component.handle_key(&schema, &key));
    assert_eq!(component.display_value(&schema), "0");
}

#[test]
fn text_component_rejects_control_characters() {
    let schema = integer_schema();
    let mut component = super::TextComponent::new(&schema);
    let ctrl_a = KeyEvent::new(KeyCode::Char('a'), KeyModifiers::CONTROL);
    assert!(!component.handle_key(&schema, &ctrl_a));
    assert_eq!(component.display_value(&schema), "");
}
