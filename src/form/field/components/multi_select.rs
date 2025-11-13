use std::sync::Arc;

use serde_json::Value;

use crate::domain::FieldSchema;
use crate::form::error::FieldCoercionError;

use super::{ComponentKind, FieldComponent, MultiSelectStateRef, palette::ComponentPalette};

#[derive(Debug, Clone)]
pub struct MultiSelectComponent {
    options: Vec<String>,
    selected: Vec<bool>,
    palette: Arc<ComponentPalette>,
}

impl MultiSelectComponent {
    pub fn new(
        options: &[String],
        default: Option<&Value>,
        palette: Arc<ComponentPalette>,
    ) -> Self {
        let mut selected = vec![false; options.len()];
        if let Some(Value::Array(items)) = default {
            for item in items.iter().filter_map(Value::as_str) {
                if let Some(idx) = options.iter().position(|opt| opt == item) {
                    selected[idx] = true;
                }
            }
        }
        Self {
            options: options.to_vec(),
            selected,
            palette,
        }
    }
}

impl FieldComponent for MultiSelectComponent {
    fn kind(&self) -> ComponentKind {
        ComponentKind::MultiSelect
    }

    fn display_value(&self, _schema: &FieldSchema) -> String {
        let values = self
            .options
            .iter()
            .zip(self.selected.iter())
            .filter_map(|(option, flag)| if *flag { Some(option.clone()) } else { None })
            .collect::<Vec<_>>();
        if values.is_empty() {
            format!("[] {}", self.palette.collection.list_hint)
        } else {
            format!("[{}]", values.join(", "))
        }
    }

    fn seed_value(&mut self, _schema: &FieldSchema, value: &Value) {
        if let Value::Array(items) = value {
            let mut flags = vec![false; self.options.len()];
            for item in items.iter().filter_map(Value::as_str) {
                if let Some(idx) = self.options.iter().position(|opt| opt == item) {
                    flags[idx] = true;
                }
            }
            if flags.len() == self.selected.len() {
                self.selected = flags;
            }
        }
    }

    fn current_value(&self, _schema: &FieldSchema) -> Result<Option<Value>, FieldCoercionError> {
        let values = self
            .options
            .iter()
            .zip(self.selected.iter())
            .filter_map(|(option, flag)| {
                if *flag {
                    Some(Value::String(option.clone()))
                } else {
                    None
                }
            })
            .collect();
        Ok(Some(Value::Array(values)))
    }

    fn multi_state(&self) -> Option<MultiSelectStateRef<'_>> {
        Some(MultiSelectStateRef {
            options: &self.options,
            selected: &self.selected,
        })
    }

    fn set_multi_state(&mut self, flags: &[bool]) -> bool {
        if flags.len() != self.selected.len() {
            return false;
        }
        if self.selected != flags {
            self.selected.clone_from_slice(flags);
            true
        } else {
            false
        }
    }
}
