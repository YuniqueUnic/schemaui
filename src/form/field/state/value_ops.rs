use serde_json::Value;

use crate::domain::FieldKind;
use crate::form::error::FieldCoercionError;

use super::super::convert::{
    array_to_string, array_value, integer_value, number_value, string_value,
};
use super::super::value::FieldValue;
use super::FieldState;

impl FieldState {
    pub fn seed_value(&mut self, value: &Value) {
        match (&mut self.value, value) {
            (FieldValue::Text(buffer), Value::String(text)) => {
                *buffer = text.clone();
            }
            (FieldValue::Text(buffer), Value::Number(num)) => {
                *buffer = num.to_string();
            }
            (FieldValue::Bool(current), Value::Bool(flag)) => {
                *current = *flag;
            }
            (FieldValue::Enum { options, selected }, Value::String(choice)) => {
                if let Some(idx) = options.iter().position(|opt| opt == choice) {
                    *selected = idx;
                }
            }
            (FieldValue::MultiSelect { options, selected }, Value::Array(items)) => {
                let mut flags = vec![false; options.len()];
                for item in items {
                    if let Some(label) = item.as_str()
                        && let Some(pos) = options.iter().position(|opt| opt == label)
                    {
                        flags[pos] = true;
                    }
                }
                if selected.len() == flags.len() {
                    selected.clone_from_slice(&flags);
                }
            }
            (FieldValue::Array(buffer), Value::Array(items)) => {
                *buffer = array_to_string(items);
            }
            (FieldValue::Composite(state), Value::Object(_)) => {
                let _ = state.seed_from_value(value);
            }
            (FieldValue::CompositeList(state), Value::Array(items)) => {
                state.seed_entries_from_array(items);
            }
            (FieldValue::ScalarArray(state), Value::Array(items)) => {
                state.seed_entries_from_array(items);
            }
            (FieldValue::KeyValue(state), Value::Object(map)) => {
                state.seed_entries_from_object(map);
            }
            _ => {}
        }

        self.dirty = false;
        self.error = None;
    }

    pub fn display_value(&self) -> String {
        match &self.value {
            FieldValue::Text(text) => text.clone(),
            FieldValue::Bool(value) => value.to_string(),
            FieldValue::Enum { options, selected } => options
                .get(*selected)
                .cloned()
                .unwrap_or_else(|| "<none>".to_string()),
            FieldValue::MultiSelect { options, selected } => {
                let values = options
                    .iter()
                    .zip(selected.iter())
                    .filter_map(|(option, flag)| if *flag { Some(option.clone()) } else { None })
                    .collect::<Vec<_>>();
                if values.is_empty() {
                    "[]".to_string()
                } else {
                    format!("[{}]", values.join(", "))
                }
            }
            FieldValue::Array(buffer) => format!("[{}]", buffer.trim()),
            FieldValue::Composite(state) => {
                let mut label = state.summary();
                if !state.is_multi() {
                    label.push_str(" (Enter to choose)");
                } else {
                    label.push_str(" (Enter to toggle)");
                }
                label
            }
            FieldValue::CompositeList(state) => {
                let len = state.len();
                if len == 0 {
                    "List: empty (Ctrl+N add)".to_string()
                } else {
                    let selection = state
                        .selected_label()
                        .unwrap_or_else(|| "<no selection>".to_string());
                    format!("List[{len}] • {selection} (Ctrl+Left/Right select, Ctrl+E edit)")
                }
            }
            FieldValue::ScalarArray(state) => {
                let len = state.len();
                if len == 0 {
                    "Array: empty (Ctrl+N add)".to_string()
                } else {
                    let selection = state
                        .selected_label()
                        .unwrap_or_else(|| "<no selection>".to_string());
                    format!("Array[{len}] • {selection} (Ctrl+Left/Right select, Ctrl+E edit)")
                }
            }
            FieldValue::KeyValue(state) => {
                let len = state.len();
                if len == 0 {
                    "Map: empty (Ctrl+N add)".to_string()
                } else {
                    let selection = state
                        .selected_label()
                        .unwrap_or_else(|| "<no selection>".to_string());
                    format!("Map[{len}] • {selection} (Ctrl+Left/Right select, Ctrl+E edit)")
                }
            }
        }
    }

    pub fn current_value(&self) -> Result<Option<Value>, FieldCoercionError> {
        match (&self.schema.kind, &self.value) {
            (FieldKind::String, FieldValue::Text(text)) => string_value(text, &self.schema),
            (FieldKind::Integer, FieldValue::Text(text)) => integer_value(text, &self.schema),
            (FieldKind::Number, FieldValue::Text(text)) => number_value(text, &self.schema),
            (FieldKind::Json, FieldValue::Text(text)) => string_value(text, &self.schema),
            (FieldKind::Boolean, FieldValue::Bool(value)) => Ok(Some(Value::Bool(*value))),
            (FieldKind::Enum(options), FieldValue::Enum { selected, .. }) => {
                let value = options.get(*selected).cloned().unwrap_or_default();
                Ok(Some(Value::String(value)))
            }
            (FieldKind::Array(_), FieldValue::MultiSelect { options, selected }) => {
                let values = options
                    .iter()
                    .zip(selected.iter())
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
            (FieldKind::Array(inner), FieldValue::Array(buffer)) => {
                array_value(buffer, inner.as_ref(), &self.schema)
            }
            (FieldKind::Composite(_), FieldValue::Composite(state)) => {
                state.build_value(self.schema.required)
            }
            (FieldKind::Array(inner), FieldValue::CompositeList(state))
                if matches!(inner.as_ref(), FieldKind::Composite(_)) =>
            {
                state.build_value()
            }
            (FieldKind::Array(_), FieldValue::ScalarArray(state)) => {
                state.build_value(self.schema.required)
            }
            (FieldKind::KeyValue(_), FieldValue::KeyValue(state)) => {
                state.build_value(self.schema.required)
            }
            _ => Ok(None),
        }
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
