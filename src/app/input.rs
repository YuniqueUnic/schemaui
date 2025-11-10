use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[derive(Debug, Clone, Copy)]
pub enum KeyCommand {
    Save,
    Quit,
    SwitchSection(i32),
    SwitchRoot(i32),
    NextField,
    PrevField,
    ResetStatus,
    TogglePopup,
    EditComposite,
    ListAddEntry,
    ListRemoveEntry,
    ListMove(i32),
    ListSelect(i32),
    Edit(KeyEvent),
    None,
}

const CTRL_LEFT_BRACKET: char = '\u{1b}';
const CTRL_RIGHT_BRACKET: char = '\u{1d}';

pub fn classify(key: &KeyEvent) -> KeyCommand {
    if is_prev_root_combo(key) {
        return KeyCommand::SwitchRoot(-1);
    }
    if is_next_root_combo(key) {
        return KeyCommand::SwitchRoot(1);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        return match key.code {
            KeyCode::Char('s') | KeyCode::Char('S') => KeyCommand::Save,
            KeyCode::Char('q') | KeyCode::Char('Q') => KeyCommand::Quit,
            KeyCode::Char('c') | KeyCode::Char('C') => KeyCommand::Quit,
            KeyCode::Char('e') | KeyCode::Char('E') => KeyCommand::EditComposite,
            KeyCode::Char('n') | KeyCode::Char('N') => KeyCommand::ListAddEntry,
            KeyCode::Char('d') | KeyCode::Char('D') => KeyCommand::ListRemoveEntry,
            KeyCode::Left => KeyCommand::ListSelect(-1),
            KeyCode::Right => KeyCommand::ListSelect(1),
            KeyCode::Up => KeyCommand::ListMove(-1),
            KeyCode::Down => KeyCommand::ListMove(1),
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

fn is_prev_root_combo(key: &KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char('[') | KeyCode::Char('{') => return true,
            KeyCode::Esc => return true,
            KeyCode::Char(ch) if ch == CTRL_LEFT_BRACKET => return true,
            _ => {}
        }
    }
    matches!(key.code, KeyCode::Char(ch) if ch == CTRL_LEFT_BRACKET)
}

fn is_next_root_combo(key: &KeyEvent) -> bool {
    if key.modifiers.contains(KeyModifiers::CONTROL) {
        match key.code {
            KeyCode::Char(']') | KeyCode::Char('}') => return true,
            KeyCode::Char(ch) if ch == CTRL_RIGHT_BRACKET => return true,
            _ => {}
        }
    }
    matches!(key.code, KeyCode::Char(ch) if ch == CTRL_RIGHT_BRACKET)
}
