use serde_json::Value;

use crate::form::error::FieldCoercionError;

use super::FieldState;

impl FieldState {
    pub fn seed_value(&mut self, value: &Value) {
        self.component.seed_value(&self.schema, value);
        self.dirty = false;
        self.error = None;
    }

    pub fn display_value(&self) -> String {
        self.component.display_value(&self.schema)
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
        self.error = None;
    }
}
