use serde_json::Value;

use crate::domain::{FieldKind, FieldSchema};
use crate::form::error::FieldCoercionError;

use super::helpers::handle_text_edit;
use super::{ComponentKind, FieldComponent};
use crate::form::field::convert::{integer_value, number_value, string_value, value_to_string};

#[derive(Debug, Clone)]
pub struct TextComponent {
    buffer: String,
}

impl TextComponent {
    pub fn new(schema: &FieldSchema) -> Self {
        let buffer = schema
            .default
            .as_ref()
            .map(value_to_string)
            .unwrap_or_default();
        Self { buffer }
    }
}

impl FieldComponent for TextComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::TextInput
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        self.buffer.clone()
    }

    fn handle_key(&mut self, schema: &FieldSchema, key: &crossterm::event::KeyEvent) -> bool {
        handle_text_edit(&mut self.buffer, schema, key)
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        match value {
            Value::String(text) => self.buffer = text.clone(),
            Value::Number(num) => self.buffer = num.to_string(),
            other => self.buffer = value_to_string(other),
        }
    }

    fn current_value(&self, schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        match schema.kind {
            FieldKind::String => string_value(&self.buffer, schema),
            FieldKind::Integer => integer_value(&self.buffer, schema),
            FieldKind::Number => number_value(&self.buffer, schema),
            FieldKind::Json => string_value(&self.buffer, schema),
            _ => Ok(None),
        }
    }
}
