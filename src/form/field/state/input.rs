use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::convert::adjust_numeric_value;
use super::super::value::FieldValue;
use super::FieldState;

impl FieldState {
    pub fn handle_key(&mut self, key: &KeyEvent) -> bool {
        match &mut self.value {
            FieldValue::Text(buffer) | FieldValue::Array(buffer) => match key.code {
                KeyCode::Left => {
                    if adjust_numeric_value(buffer, &self.schema.kind, -1) {
                        self.after_edit();
                        true
                    } else {
                        false
                    }
                }
                KeyCode::Right => {
                    if adjust_numeric_value(buffer, &self.schema.kind, 1) {
                        self.after_edit();
                        true
                    } else {
                        false
                    }
                }
                KeyCode::Char(c) => {
                    if key.modifiers.contains(KeyModifiers::CONTROL) {
                        return false;
                    }
                    buffer.push(c);
                    self.after_edit();
                    true
                }
                KeyCode::Backspace => {
                    buffer.pop();
                    self.after_edit();
                    true
                }
                KeyCode::Delete => {
                    buffer.clear();
                    self.after_edit();
                    true
                }
                _ => false,
            },
            FieldValue::Bool(value) => match key.code {
                KeyCode::Char(' ') | KeyCode::Left | KeyCode::Right => {
                    *value = !*value;
                    self.after_edit();
                    true
                }
                _ => false,
            },
            FieldValue::Enum { options, selected } => match key.code {
                KeyCode::Up | KeyCode::Left => {
                    if *selected == 0 {
                        *selected = options.len().saturating_sub(1);
                    } else {
                        *selected -= 1;
                    }
                    self.after_edit();
                    true
                }
                KeyCode::Down | KeyCode::Right => {
                    if !options.is_empty() {
                        *selected = (*selected + 1) % options.len();
                    }
                    self.after_edit();
                    true
                }
                _ => false,
            },
            _ => false,
        }
    }

    pub fn set_bool(&mut self, value: bool) {
        if let FieldValue::Bool(current) = &mut self.value
            && *current != value
        {
            *current = value;
            self.after_edit();
        }
    }

    pub fn set_enum_selected(&mut self, index: usize) {
        if let FieldValue::Enum { options, selected } = &mut self.value {
            if options.is_empty() {
                return;
            }
            let bounded = index.min(options.len() - 1);
            if *selected != bounded {
                *selected = bounded;
                self.after_edit();
            }
        }
    }

    pub fn set_multi_selection(&mut self, selections: &[bool]) {
        if let FieldValue::MultiSelect { selected, .. } = &mut self.value
            && selected.len() == selections.len()
            && selected != selections
        {
            selected.clone_from_slice(selections);
            self.after_edit();
        }
    }
}
