use crate::tui::app::{
    input::{InputRouter, KeyAction},
    keymap::{self, KeymapContext},
};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

fn router() -> InputRouter {
    InputRouter::new(keymap::default_store())
}

#[test]
fn ctrl_jl_cycle_root_sections() {
    let router = router();
    let prev = router.classify(&key(KeyCode::Char('j'), KeyModifiers::CONTROL));
    let next = router.classify(&key(KeyCode::Char('l'), KeyModifiers::CONTROL));
    assert!(matches!(prev, KeyAction::RootStep(-1)));
    assert!(matches!(next, KeyAction::RootStep(1)));
}

#[test]
fn ctrl_tab_maps_to_section_steps() {
    let router = router();
    let next = router.classify(&key(KeyCode::Tab, KeyModifiers::CONTROL));
    let prev = router.classify(&key(
        KeyCode::Tab,
        KeyModifiers::CONTROL | KeyModifiers::SHIFT,
    ));
    assert!(matches!(next, KeyAction::SectionStep(1)));
    assert!(matches!(prev, KeyAction::SectionStep(-1)));
}

#[test]
fn shift_tab_triggers_previous_field() {
    let router = router();
    let action = router.classify(&key(KeyCode::BackTab, KeyModifiers::SHIFT));
    assert!(matches!(action, KeyAction::FieldStep(-1)));
}

#[test]
fn help_only_text_bindings_do_not_override_raw_input() {
    let router = router();

    let left = router.classify(&key(KeyCode::Left, KeyModifiers::NONE));
    let space = router.classify(&key(KeyCode::Char(' '), KeyModifiers::NONE));
    let backspace = router.classify(&key(KeyCode::Backspace, KeyModifiers::NONE));
    let ctrl_w = router.classify(&key(KeyCode::Char('w'), KeyModifiers::CONTROL));

    assert!(matches!(left, KeyAction::Input(event) if event.code == KeyCode::Left));
    assert!(matches!(space, KeyAction::Input(event) if event.code == KeyCode::Char(' ')));
    assert!(matches!(backspace, KeyAction::Input(event) if event.code == KeyCode::Backspace));
    assert!(matches!(ctrl_w, KeyAction::Input(event)
        if event.code == KeyCode::Char('w') && event.modifiers == KeyModifiers::CONTROL));
}

#[test]
fn default_keymap_exposes_field_local_edit_help_contexts() {
    let store = keymap::default_store();

    let text_help = store
        .help_text(KeymapContext::TextInput)
        .expect("text input help");
    assert!(text_help.contains("Left -> Move cursor left"));
    assert!(text_help.contains("Right -> Move cursor right"));
    assert!(text_help.contains("Ctrl+W -> Delete previous word"));
    assert!(text_help.contains("Ctrl+Z -> Undo text edit"));
    assert!(text_help.contains("Ctrl+Y -> Redo text edit"));

    let numeric_help = store
        .help_text(KeymapContext::NumericInput)
        .expect("numeric input help");
    assert!(numeric_help.contains("Left -> Step value down"));
    assert!(numeric_help.contains("Right -> Step value up"));
    assert!(numeric_help.contains("Shift+Left -> Fast step value down"));
    assert!(numeric_help.contains("Shift+Right -> Fast step value up"));
    assert!(numeric_help.contains("Ctrl+Z -> Undo numeric edit"));
    assert!(numeric_help.contains("Ctrl+Y -> Redo numeric edit"));

    let boolean_help = store
        .help_text(KeymapContext::BooleanInput)
        .expect("boolean input help");
    assert!(boolean_help.contains("Space/Left/Right -> Toggle boolean value"));

    let enum_help = store
        .help_text(KeymapContext::EnumInput)
        .expect("enum input help");
    assert!(enum_help.contains("Up/Left -> Previous enum option"));
    assert!(enum_help.contains("Down/Right -> Next enum option"));

    let composite_help = store
        .help_text(KeymapContext::CompositeInput)
        .expect("composite input help");
    assert!(composite_help.contains("Left -> Previous composite variant"));
    assert!(composite_help.contains("Right -> Next composite variant"));

    let array_help = store
        .help_text(KeymapContext::ArrayBufferInput)
        .expect("array buffer input help");
    assert!(array_help.contains("Backspace -> Delete previous array buffer character"));
    assert!(array_help.contains("Delete -> Clear array buffer"));
}

#[test]
fn help_context_bindings_are_only_classified_when_requested() {
    let store = keymap::default_store();
    let esc = key(KeyCode::Esc, KeyModifiers::NONE);

    assert!(!matches!(store.classify(&esc), Some(KeyAction::HelpClose)));
    assert!(matches!(
        store.classify_for_contexts(&esc, &[KeymapContext::Help]),
        Some(KeyAction::HelpClose)
    ));
}

#[test]
fn popup_context_bindings_are_only_classified_when_requested() {
    let store = keymap::default_store();
    let esc = key(KeyCode::Esc, KeyModifiers::NONE);
    let space = key(KeyCode::Char(' '), KeyModifiers::NONE);

    assert!(!matches!(store.classify(&esc), Some(KeyAction::PopupClose)));
    assert!(!matches!(
        store.classify(&space),
        Some(KeyAction::PopupToggle)
    ));
    assert!(matches!(
        store.classify_for_contexts(&esc, &[KeymapContext::Popup]),
        Some(KeyAction::PopupClose)
    ));
    assert!(matches!(
        store.classify_for_contexts(&space, &[KeymapContext::Popup]),
        Some(KeyAction::PopupToggle)
    ));
}

#[test]
fn custom_keymap_rejects_duplicate_ids() {
    let raw = r#"
[
  {
    "id": "duplicate",
    "description": "First",
    "descriptionZh": "第一个",
    "contexts": ["default"],
    "action": { "kind": "none" },
    "combos": ["x"]
  },
  {
    "id": "duplicate",
    "description": "Second",
    "descriptionZh": "第二个",
    "contexts": ["default"],
    "action": { "kind": "none" },
    "combos": ["y"]
  }
]
"#;

    let error = keymap::KeymapStore::from_json(raw).expect_err("duplicate id should fail");
    assert!(
        error
            .to_string()
            .contains("duplicate keymap entry id duplicate"),
        "unexpected error: {error}"
    );
}
