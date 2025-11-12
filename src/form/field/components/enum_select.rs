use serde_json::Value;

use crate::domain::FieldSchema;

use super::{ComponentKind, EnumStateRef, FieldComponent};
use crate::form::field::convert::value_to_string;

#[derive(Debug, Clone)]
pub struct EnumComponent {
    options: Vec<String>,
    selected: usize,
}

impl EnumComponent {
    pub fn new(options: &[String], schema: &FieldSchema) -> Self {
        let default_value = schema
            .default
            .as_ref()
            .map(value_to_string)
            .and_then(|value| if value.is_empty() { None } else { Some(value) })
            .unwrap_or_else(|| options.first().cloned().unwrap_or_default());
        let selected = options
            .iter()
            .position(|item| item == &default_value)
            .unwrap_or(0);
        Self {
            options: options.to_vec(),
            selected,
        }
    }
}

impl FieldComponent for EnumComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Enum
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        self.options
            .get(self.selected)
            .cloned()
            .unwrap_or_else(|| "<none>".to_string())
    }

    fn handle_key(&mut self, _schema: &FieldSchema, key: &crossterm::event::KeyEvent) -> bool {
        match key.code {
            crossterm::event::KeyCode::Up | crossterm::event::KeyCode::Left => {
                if self.options.is_empty() {
                    return false;
                }
                if self.selected == 0 {
                    self.selected = self.options.len().saturating_sub(1);
                } else {
                    self.selected -= 1;
                }
                true
            }
            crossterm::event::KeyCode::Down | crossterm::event::KeyCode::Right => {
                if self.options.is_empty() {
                    return false;
                }
                self.selected = (self.selected + 1) % self.options.len();
                true
            }
            _ => false,
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Some(text) = value.as_str()
            && let Some(idx) = self.options.iter().position(|opt| opt == text)
        {
            self.selected = idx;
        }
    }

    fn current_value(
        &self,
        _schema: &FieldSchema,
    ) -> Result<Option<Value>, crate::form::error::FieldCoercionError> {
        Ok(self.options.get(self.selected).cloned().map(Value::String))
    }

    fn enum_state(&self) -> Option<EnumStateRef<'_>> {
        Some(EnumStateRef {
            options: &self.options,
            selected: self.selected,
        })
    }

    fn set_enum_index(&mut self, index: usize) -> bool {
        if self.options.is_empty() {
            return false;
        }
        let bounded = index.min(self.options.len() - 1);
        if self.selected != bounded {
            self.selected = bounded;
            true
        } else {
            false
        }
    }
}
