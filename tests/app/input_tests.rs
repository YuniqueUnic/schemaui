use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn key(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, modifiers)
}

#[test]
fn router() -> InputRouter {
    InputRouter::new(super::keymap::default_store())
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
