use std::sync::Arc;

use serde_json::Value;

use crate::tui::model::FieldSchema;
use crate::tui::state::error::FieldCoercionError;

use super::{ComponentKind, FieldComponent, palette::ComponentPalette};

#[derive(Debug, Clone)]
pub struct BoolComponent {
    value: Option<bool>,
    palette: Arc<ComponentPalette>,
}

impl BoolComponent {
    pub fn new(schema: &FieldSchema, palette: Arc<ComponentPalette>) -> Self {
        let value = schema
            .default
            .as_ref()
            .and_then(Value::as_bool)
            .or_else(|| {
                (!matches!(schema.kind, crate::tui::model::FieldKind::Nullable(_))).then_some(false)
            });
        Self { value, palette }
    }
}

impl FieldComponent for BoolComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::Bool
    }

    fn display_value(&self, schema: &FieldSchema) -> String {
        let Some(value) = self.value else {
            return "null".to_string();
        };
        if schema.presentation.control == Some(crate::ui_ast::FieldControl::Checkbox) {
            return format!(
                "[{}] {}",
                if value { "x" } else { " " },
                if value {
                    &self.palette.bools.true_label
                } else {
                    &self.palette.bools.false_label
                }
            );
        }
        if schema.presentation.control == Some(crate::ui_ast::FieldControl::Switch) {
            return if value {
                "[ ON | off ]"
            } else {
                "[ on | OFF ]"
            }
            .to_string();
        }
        if value {
            self.palette.bools.true_label.to_string()
        } else {
            self.palette.bools.false_label.to_string()
        }
    }

    fn handle_key(&mut self, schema: &FieldSchema, key: &crossterm::event::KeyEvent) -> bool {
        if matches!(schema.kind, crate::tui::model::FieldKind::Nullable(_))
            && matches!(
                key.code,
                crossterm::event::KeyCode::Backspace | crossterm::event::KeyCode::Delete
            )
        {
            return self.value.take().is_some();
        }
        match key.code {
            crossterm::event::KeyCode::Char(' ') if self.palette.bools.toggle_with_space => {
                self.value = Some(!self.value.unwrap_or(false));
                true
            }
            crossterm::event::KeyCode::Left | crossterm::event::KeyCode::Right
                if self.palette.bools.toggle_with_arrows =>
            {
                self.value = Some(!self.value.unwrap_or(false));
                true
            }
            _ => false,
        }
    }

    fn seed_value(&mut self, schema: &FieldSchema, value: &Value) {
        if value.is_boolean()
            || (value.is_null() && matches!(schema.kind, crate::tui::model::FieldKind::Nullable(_)))
        {
            self.value = value.as_bool();
        }
    }

    fn current_value(&self, _schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        Ok(Some(self.value.map_or(Value::Null, Value::Bool)))
    }

    fn bool_value(&self) -> Option<bool> {
        self.value
    }

    fn set_bool(&mut self, value: bool) -> bool {
        if self.value != Some(value) {
            self.value = Some(value);
            true
        } else {
            false
        }
    }
}
