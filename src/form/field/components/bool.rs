use serde_json::Value;

use crate::domain::FieldSchema;

use super::{ComponentKind, FieldComponent};

#[derive(Debug, Clone)]
pub struct BoolComponent {
    value: bool,
}

impl BoolComponent {
    pub fn new(schema: &FieldSchema) -> Self {
        let value = schema
            .default
            .as_ref()
            .and_then(|value| value.as_bool())
            .unwrap_or(false);
        Self { value }
    }
}

impl FieldComponent for BoolComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Bool
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        self.value.to_string()
    }

    fn handle_key(&mut self, _schema: &FieldSchema, key: &crossterm::event::KeyEvent) -> bool {
        match key.code {
            crossterm::event::KeyCode::Char(' ')
            | crossterm::event::KeyCode::Left
            | crossterm::event::KeyCode::Right => {
                self.value = !self.value;
                true
            }
            _ => false,
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Some(flag) = value.as_bool() {
            self.value = flag;
        }
    }

    fn current_value(
        &self,
        _schema: &FieldSchema,
    ) -> Result<Option<Value>, crate::form::error::FieldCoercionError> {
        Ok(Some(Value::Bool(self.value)))
    }

    fn bool_value(&self) -> Option<bool> {
        Some(self.value)
    }

    fn set_bool(&mut self, value: bool) -> bool {
        if self.value != value {
            self.value = value;
            true
        } else {
            false
        }
    }
}
