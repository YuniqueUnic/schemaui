use crate::{
    app::{App, UiOptions},
    form::FormState,
    schema::build_form_schema,
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use jsonschema::validator_for;
use serde_json::json;

fn build_nested_overlay_app() -> App {
    let schema = json!({
        "type": "object",
        "properties": {
            "service": {
                "oneOf": [
                    {
                        "title": "http",
                        "type": "object",
                        "properties": {
                            "routes": {
                                "type": "array",
                                "default": [
                                    {"path": "/"},
                                    {"path": "/status"}
                                ],
                                "items": {
                                    "type": "object",
                                    "properties": {
                                        "path": {"type": "string"}
                                    },
                                    "required": ["path"]
                                }
                            }
                        }
                    }
                ]
            }
        }
    });
    let form_schema = build_form_schema(&schema).expect("schema");
    let form_state = FormState::from_schema(&form_schema);
    let validator = validator_for(&schema).expect("validator");
    App::new(form_state, validator, UiOptions::default())
}

fn activate_service_variant(app: &mut App) {
    let form_state = app.form_state_mut_for_test();
    let field = form_state
        .field_mut_by_pointer("/service")
        .expect("service field present");
    field.apply_composite_selection(0, None);
}

fn focus_field(state: &mut FormState, pointer: &str) {
    for (root_idx, root) in state.roots.iter().enumerate() {
        for (section_idx, section) in root.sections.iter().enumerate() {
            if let Some(field_idx) = section.fields.iter().position(|field| {
                field.schema.pointer == pointer || field.schema.pointer.ends_with(pointer)
            }) {
                state.root_index = root_idx;
                state.section_index = section_idx;
                state.field_index = field_idx;
                return;
            }
        }
    }
    let mut available = Vec::new();
    for root in &state.roots {
        for section in &root.sections {
            for field in &section.fields {
                available.push(field.schema.pointer.clone());
            }
        }
    }
    panic!("field {pointer} not found; available: {available:?}");
}

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn ctrl_s_saves_overlay_without_popping_stack() {
    let mut app = build_nested_overlay_app();
    activate_service_variant(&mut app);
    app.open_overlay_for_test();
    assert_eq!(app.overlay_depth_for_test(), 1);

    {
        let overlay_form = app
            .active_overlay_form_state_for_test()
            .expect("overlay form");
        focus_field(overlay_form, "/routes");
    }
    app.open_overlay_for_test();
    assert_eq!(app.overlay_depth_for_test(), 2);

    app.handle_key_for_test(key(KeyCode::Char('s'), KeyModifiers::CONTROL))
        .expect("ctrl+s");
    assert_eq!(
        app.overlay_depth_for_test(),
        2,
        "save should not close overlay"
    );
}

#[test]
fn esc_pops_only_top_overlay() {
    let mut app = build_nested_overlay_app();
    activate_service_variant(&mut app);
    app.open_overlay_for_test();
    {
        let overlay_form = app
            .active_overlay_form_state_for_test()
            .expect("overlay form");
        focus_field(overlay_form, "/routes");
    }
    app.open_overlay_for_test();
    assert_eq!(app.overlay_depth_for_test(), 2);

    app.handle_key_for_test(key(KeyCode::Esc, KeyModifiers::NONE))
        .expect("close overlay2");
    assert_eq!(
        app.overlay_depth_for_test(),
        1,
        "esc closes only top overlay"
    );

    app.handle_key_for_test(key(KeyCode::Esc, KeyModifiers::NONE))
        .expect("close overlay1");
    assert_eq!(app.overlay_depth_for_test(), 0);
}

#[test]
fn tab_cycles_entry_strip_inside_overlay() {
    let mut app = build_nested_overlay_app();
    activate_service_variant(&mut app);
    app.open_overlay_for_test();
    {
        let overlay_form = app
            .active_overlay_form_state_for_test()
            .expect("overlay form level1");
        focus_field(overlay_form, "/routes");
    }
    app.open_overlay_for_test();
    assert_eq!(app.overlay_depth_for_test(), 2);
    assert_eq!(app.overlay_entry_focus_for_test(), Some(false));

    app.handle_key_for_test(key(KeyCode::Tab, KeyModifiers::NONE))
        .expect("tab to entry strip");
    assert_eq!(app.overlay_entry_focus_for_test(), Some(true));

    app.handle_key_for_test(key(KeyCode::Tab, KeyModifiers::NONE))
        .expect("tab back to fields");
    assert_eq!(app.overlay_entry_focus_for_test(), Some(false));

    app.handle_key_for_test(key(KeyCode::BackTab, KeyModifiers::SHIFT))
        .expect("shift+tab to entry strip");
    assert_eq!(app.overlay_entry_focus_for_test(), Some(true));

    app.handle_key_for_test(key(KeyCode::BackTab, KeyModifiers::SHIFT))
        .expect("shift+tab back to last field");
    assert_eq!(app.overlay_entry_focus_for_test(), Some(false));
}

#[test]
fn ctrl_arrows_manage_entries_without_closing_overlay() {
    let mut app = build_nested_overlay_app();
    activate_service_variant(&mut app);
    app.open_overlay_for_test();
    {
        let overlay_form = app
            .active_overlay_form_state_for_test()
            .expect("overlay form level1");
        focus_field(overlay_form, "/routes");
    }
    app.open_overlay_for_test();
    assert_eq!(app.overlay_depth_for_test(), 2);
    assert_eq!(app.overlay_selected_entry_for_test(), Some(0));

    app.handle_key_for_test(key(KeyCode::Right, KeyModifiers::CONTROL))
        .expect("ctrl+right");
    assert_eq!(app.overlay_depth_for_test(), 2);
    assert_eq!(app.overlay_selected_entry_for_test(), Some(1));

    app.handle_key_for_test(key(KeyCode::Left, KeyModifiers::CONTROL))
        .expect("ctrl+left");
    assert_eq!(app.overlay_selected_entry_for_test(), Some(0));

    app.handle_key_for_test(key(KeyCode::Left, KeyModifiers::CONTROL))
        .expect("ctrl+left boundary");
    assert_eq!(app.overlay_depth_for_test(), 2);
    assert_eq!(app.overlay_selected_entry_for_test(), Some(0));

    app.handle_key_for_test(key(KeyCode::Down, KeyModifiers::CONTROL))
        .expect("ctrl+down reorder");
    assert_eq!(app.overlay_depth_for_test(), 2);
    assert_eq!(
        app.overlay_selected_entry_for_test(),
        Some(1),
        "entry should move to index 1 after reorder"
    );

    app.handle_key_for_test(key(KeyCode::Up, KeyModifiers::CONTROL))
        .expect("ctrl+up reorder back");
    assert_eq!(app.overlay_depth_for_test(), 2);
    assert_eq!(app.overlay_selected_entry_for_test(), Some(0));
}
