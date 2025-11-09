use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy)]
pub enum KeyCommand {
    Save,
    Quit,
    SwitchSection(i32),
    NextField,
    PrevField,
    ResetStatus,
    TogglePopup,
    Edit(KeyEvent),
    None,
}

pub fn classify(key: &KeyEvent) -> KeyCommand {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => KeyCommand::Save,
            KeyCode::Char('q') | KeyCode::Char('Q') => KeyCommand::Quit,
            KeyCode::Char('c') | KeyCode::Char('C') => KeyCommand::Quit,
            KeyCode::Tab => {
                let delta = if key.modifiers.contains(KeyModifiers::SHIFT) {
                    -1
                } else {
                    1
                };
                KeyCommand::SwitchSection(delta)
            }
            _ => KeyCommand::None,
        };
    }

    match key.code {
        KeyCode::Tab | KeyCode::Down => KeyCommand::NextField,
        KeyCode::BackTab | KeyCode::Up => KeyCommand::PrevField,
        KeyCode::Esc => KeyCommand::ResetStatus,
        KeyCode::Enter => KeyCommand::TogglePopup,
        _ => KeyCommand::Edit(*key),
    }
}
