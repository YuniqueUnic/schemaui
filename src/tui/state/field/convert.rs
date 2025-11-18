use serde_json::Value;

use crate::domain::{FieldKind, FieldSchema};

use crate::form::error::FieldCoercionError;

pub(super) fn value_to_string(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(num) => num.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Array(items) => items
            .iter()
            .map(value_to_string)
            .collect::<Vec<_>>()
            .join(", "),
        other => other.to_string(),
    }
}

pub(super) fn array_to_string(items: &[Value]) -> String {
    items
        .iter()
        .map(value_to_string)
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn string_value(
    contents: &str,
    schema: &FieldSchema,
) -> Result<Option<Value>, FieldCoercionError> {
    if contents.is_empty() && !schema.required {
        return Ok(None);
    }
    Ok(Some(Value::String(contents.to_string())))
}

pub(super) fn integer_value(
    contents: &str,
    schema: &FieldSchema,
) -> Result<Option<Value>, FieldCoercionError> {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<i64>()
        .map(Value::from)
        .map(Some)
        .map_err(|_| FieldCoercionError {
            pointer: schema.pointer.clone(),
            message: "expected integer".to_string(),
        })
}

pub(super) fn number_value(
    contents: &str,
    schema: &FieldSchema,
) -> Result<Option<Value>, FieldCoercionError> {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    trimmed
        .parse::<f64>()
        .map(Value::from)
        .map(Some)
        .map_err(|_| FieldCoercionError {
            pointer: schema.pointer.clone(),
            message: "expected number".to_string(),
        })
}

pub(super) fn array_value(
    contents: &str,
    inner: &FieldKind,
    schema: &FieldSchema,
) -> Result<Option<Value>, FieldCoercionError> {
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        if schema.required {
            return Ok(Some(Value::Array(Vec::new())));
        }
        return Ok(None);
    }

    let mut values = Vec::new();
    for raw in contents.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }
        let value = match inner {
            FieldKind::String => Value::String(item.to_string()),
            FieldKind::Integer => {
                item.parse::<i64>()
                    .map(Value::from)
                    .map_err(|_| FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("'{item}' is not a valid integer"),
                    })?
            }
            FieldKind::Number => {
                item.parse::<f64>()
                    .map(Value::from)
                    .map_err(|_| FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("'{item}' is not a valid number"),
                    })?
            }
            FieldKind::Boolean => match item.to_ascii_lowercase().as_str() {
                "true" => Value::Bool(true),
                "false" => Value::Bool(false),
                _ => {
                    return Err(FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("'{item}' is not a valid boolean"),
                    });
                }
            },
            FieldKind::Enum(options) => {
                if options.iter().any(|opt| opt == item) {
                    Value::String(item.to_string())
                } else {
                    return Err(FieldCoercionError {
                        pointer: schema.pointer.clone(),
                        message: format!("value '{item}' is not one of: {}", options.join(", ")),
                    });
                }
            }
            FieldKind::Json | FieldKind::Composite(_) => Value::String(item.to_string()),
            FieldKind::KeyValue(_) => {
                return Err(FieldCoercionError {
                    pointer: schema.pointer.clone(),
                    message: "arrays of key/value maps are not supported".to_string(),
                });
            }
            FieldKind::Array(_) => {
                return Err(FieldCoercionError {
                    pointer: schema.pointer.clone(),
                    message: "nested arrays are not supported".to_string(),
                });
            }
        };
        values.push(value);
    }

    Ok(Some(Value::Array(values)))
}

#[derive(Debug, Clone, Copy)]
pub enum NumericStepValue {
    Integer(i64),
    Float(f64),
}

pub(super) fn adjust_numeric_value(
    buffer: &mut String,
    kind: &FieldKind,
    delta: NumericStepValue,
) -> bool {
    match (kind, delta) {
        (FieldKind::Integer, NumericStepValue::Integer(step)) => {
            let current = buffer.trim().parse::<i64>().unwrap_or(0);
            let next = current.saturating_add(step);
            *buffer = next.to_string();
            true
        }
        (FieldKind::Integer, NumericStepValue::Float(step)) => {
            let current = buffer.trim().parse::<i64>().unwrap_or(0);
            let next = current as f64 + step;
            *buffer = next.round().to_string();
            true
        }
        (FieldKind::Number, NumericStepValue::Integer(step)) => {
            let current = buffer.trim().parse::<f64>().unwrap_or(0.0);
            *buffer = (current + step as f64).to_string();
            true
        }
        (FieldKind::Number, NumericStepValue::Float(step)) => {
            let current = buffer.trim().parse::<f64>().unwrap_or(0.0);
            *buffer = (current + step).to_string();
            true
        }
        _ => false,
    }
}
