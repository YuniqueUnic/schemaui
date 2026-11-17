use crossterm::event::KeyEvent;

use super::FieldState;

impl FieldState {
    pub fn captures_key(&self, key: &KeyEvent) -> bool {
        use crossterm::event::{KeyCode, KeyModifiers};
        let unmodified = key
            .modifiers
            .intersection(KeyModifiers::CONTROL | KeyModifiers::ALT)
            .is_empty();
        unmodified
            && ((self.is_multiline()
                && matches!(key.code, KeyCode::Enter | KeyCode::Up | KeyCode::Down))
                || (self.range_endpoint().is_some()
                    && matches!(key.code, KeyCode::Up | KeyCode::Down)))
    }

    pub fn handle_key(&mut self, key: &KeyEvent) -> bool {
        if self.component.range_endpoint().is_some() && self.captures_key(key) {
            return self.component.handle_key(&self.schema, key);
        }
        let previous = self.component.current_value(&self.schema);
        let buffer = self.component.edit_buffer().map(str::to_owned);
        if self.component.handle_key(&self.schema, key) {
            let current = self.component.current_value(&self.schema);
            if previous != current || buffer.as_deref() != self.component.edit_buffer() {
                self.after_edit();
            }
            true
        } else {
            false
        }
    }

    pub fn paste(&mut self, text: &str) -> bool {
        if self.component.paste(&self.schema, text) {
            self.after_edit();
            true
        } else {
            false
        }
    }

    pub fn set_bool(&mut self, value: bool) {
        if self.component.set_bool(value) {
            self.after_edit();
        }
    }

    pub fn set_enum_selected(&mut self, index: usize) {
        if self.component.set_enum_index(index) {
            self.after_edit();
        }
    }

    pub fn set_multi_selection(&mut self, selections: &[bool]) {
        if self.component.set_multi_state(selections) {
            self.after_edit();
        }
    }
}
