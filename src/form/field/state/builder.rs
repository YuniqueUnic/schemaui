use serde_json::Value;

use crate::domain::{FieldKind, FieldSchema};
use crate::form::array::ScalarArrayState;
use crate::form::composite::{CompositeListState, CompositeState};
use crate::form::key_value::KeyValueState;

use super::super::convert::{array_to_string, default_text, value_to_string};
use super::super::value::FieldValue;
use super::FieldState;

impl FieldState {
    pub fn from_schema(schema: FieldSchema) -> Self {
        let value = match &schema.kind {
            FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Json => {
                FieldValue::Text(default_text(&schema))
            }
            FieldKind::Boolean => {
                let default = schema
                    .default
                    .as_ref()
                    .and_then(|value| value.as_bool())
                    .unwrap_or(false);
                FieldValue::Bool(default)
            }
            FieldKind::Enum(options) => {
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
                FieldValue::Enum {
                    options: options.clone(),
                    selected,
                }
            }
            FieldKind::Array(inner) => match inner.as_ref() {
                FieldKind::Enum(options) => {
                    let defaults = schema
                        .default
                        .as_ref()
                        .and_then(|value| value.as_array())
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_string)
                                .collect::<Vec<_>>()
                        })
                        .unwrap_or_default();
                    let mut selected = vec![false; options.len()];
                    for (idx, option) in options.iter().enumerate() {
                        if defaults.iter().any(|value| value == option) {
                            selected[idx] = true;
                        }
                    }
                    FieldValue::MultiSelect {
                        options: options.clone(),
                        selected,
                    }
                }
                FieldKind::Composite(meta) => FieldValue::CompositeList(CompositeListState::new(
                    &schema.pointer,
                    meta,
                    schema.default.as_ref(),
                )),
                FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Boolean => {
                    FieldValue::ScalarArray(ScalarArrayState::new(
                        &schema.pointer,
                        schema.display_label(),
                        schema.description.clone(),
                        inner.as_ref(),
                        schema.default.as_ref(),
                    ))
                }
                _ => {
                    let default = schema
                        .default
                        .as_ref()
                        .and_then(|value| value.as_array())
                        .map(|items| array_to_string(items))
                        .unwrap_or_default();
                    FieldValue::Array(default)
                }
            },
            FieldKind::Composite(meta) => {
                FieldValue::Composite(CompositeState::new(&schema.pointer, meta))
            }
            FieldKind::KeyValue(template) => FieldValue::KeyValue(KeyValueState::new(
                &schema.pointer,
                template,
                schema.default.as_ref(),
            )),
        };

        Self {
            schema,
            value,
            dirty: false,
            error: None,
        }
    }
}
