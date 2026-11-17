use serde_json::Value;

use crate::tui::state::error::FieldCoercionError;

use super::FieldState;

impl FieldState {
    pub fn seed_value(&mut self, value: &Value) {
        self.component.seed_value(&self.schema, value);
        self.dirty = false;
        self.edited = false;
        self.error = None;
    }

    pub fn display_value(&self) -> String {
        self.component.display_value(&self.schema)
    }

    pub fn display_value_with_limit(&self, max_visible: usize) -> String {
        self.component
            .display_value_with_limit(&self.schema, max_visible)
    }

    pub fn is_multiline(&self) -> bool {
        self.component.kind() == super::super::components::ComponentKind::TextInput
            && self.schema.presentation.is_multiline()
    }

    pub fn range_endpoint(&self) -> Option<usize> {
        self.component.range_endpoint()
    }

    pub fn cursor_offset(&self) -> Option<usize> {
        self.component.cursor_offset(&self.schema)
    }

    pub fn current_value(&self) -> Result<Option<Value>, FieldCoercionError> {
        self.component.current_value(&self.schema)
    }

    pub fn clear_error(&mut self) {
        self.error = None;
    }

    pub fn set_error(&mut self, message: String) {
        self.error = Some(message);
    }

    pub fn after_edit(&mut self) {
        self.dirty = true;
        self.edited = true;
        self.error = None;
    }
}
