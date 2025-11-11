use crate::domain::{FieldKind, FieldSchema};
use crate::form::FieldState;
use ratatui::style::{Color, Modifier};

fn make_field() -> FieldState {
    FieldState::from_schema(FieldSchema {
        name: "test".into(),
        path: vec!["test".into()],
        pointer: "/test".into(),
        title: "Test".into(),
        description: None,
        section_id: "sec".into(),
        kind: FieldKind::String,
        required: false,
        default: None,
        metadata: Default::default(),
    })
}

#[test]
fn meta_line_selected_uses_dark_text() {
    let field = make_field();
    let line = meta_line(&field, true);
    let span = line.spans.first().expect("type span");
    assert_eq!(span.style.fg, Some(Color::Black));
    assert!(span.style.add_modifier.contains(Modifier::BOLD));
}

#[test]
fn meta_line_unselected_uses_gray() {
    let field = make_field();
    let line = meta_line(&field, false);
    let span = line.spans.first().expect("type span");
    assert_eq!(span.style.fg, Some(Color::DarkGray));
    assert!(!span.style.add_modifier.contains(Modifier::BOLD));
}
