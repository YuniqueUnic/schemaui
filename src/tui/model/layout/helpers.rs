use std::collections::HashSet;

use anyhow::{Context, Result, bail};
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec,
};
use serde_json::Value;

/// Collect required property names into a HashSet
pub(super) fn required_set(object: &ObjectValidation) -> HashSet<String> {
    object.required.iter().cloned().collect()
}

/// Convert a path of schema segments into a JSON pointer string
pub(super) fn to_pointer(path: &[String]) -> String {
    if path.is_empty() {
        return String::new();
    }

    path.iter()
        .map(|segment| segment.replace('~', "~0").replace('/', "~1"))
        .fold(String::new(), |mut acc, segment| {
            acc.push('/');
            acc.push_str(&segment);
            acc
        })
}

/// Determine effective instance type, ignoring explicit `null` in unions
pub(super) fn instance_type(schema: &SchemaObject) -> Option<InstanceType> {
    schema.instance_type.as_ref().and_then(|kind| match kind {
        SingleOrVec::Single(single) => Some(**single),
        SingleOrVec::Vec(items) => items
            .iter()
            .cloned()
            .find(|item| *item != InstanceType::Null),
    })
}

/// Check whether a schema describes an object-like value
pub(super) fn is_object_schema(schema: &SchemaObject) -> bool {
    match instance_type(schema) {
        Some(InstanceType::Object) => true,
        None => schema.object.is_some(),
        _ => false,
    }
}

/// Ensure a schema is object-shaped, otherwise return a descriptive error
pub(super) fn ensure_object_schema(schema: &SchemaObject) -> Result<()> {
    if is_object_schema(schema) {
        Ok(())
    } else {
        bail!("schema must describe an object")
    }
}

/// Detect whether schema contains oneOf/anyOf composite subschemas
pub(super) fn has_composite_subschemas(schema: &SchemaObject) -> bool {
    schema
        .subschemas
        .as_ref()
        .map(|subs| subs.one_of.is_some() || subs.any_of.is_some())
        .unwrap_or(false)
}

/// Serialize a SchemaObject back to serde_json::Value
pub(super) fn schema_object_to_value(schema: &SchemaObject) -> Result<Value> {
    serde_json::to_value(Schema::Object(schema.clone()))
        .context("failed to serialize schema object")
}

/// Extract title/description/default triple from schema metadata
pub(super) fn schema_titles(
    schema: &SchemaObject,
    fallback: &str,
) -> (String, Option<String>, Option<Value>) {
    let title = schema
        .metadata
        .as_ref()
        .and_then(|m| m.title.clone())
        .unwrap_or_else(|| fallback.to_string());
    let description = schema.metadata.as_ref().and_then(|m| m.description.clone());
    let default = schema.metadata.as_ref().and_then(|m| m.default.clone());
    (title, description, default)
}

/// Build the object schema used for a single key/value entry
pub(super) fn key_value_entry_schema(key_schema: &Value, value_schema: &Value) -> Value {
    serde_json::json!({
        "type": "object",
        "required": ["key", "value"],
        "properties": {
            "key": key_schema,
            "value": value_schema,
        }
    })
}

/// Describe the high-level shape of a schema for use in variant titles etc.
pub(super) fn describe_schema_shape(schema: &SchemaObject) -> String {
    if let Some(instance) = instance_type(schema) {
        match instance {
            InstanceType::String => {
                // include format information when present
                if let Some(format) = schema.format.as_ref() {
                    return format!("string({})", format);
                }
                return "string".to_string();
            }
            InstanceType::Integer => {
                // include numeric range information when present
                let mut parts = vec!["integer".to_string()];
                if let Some(number) = schema.number.as_ref()
                    && (number.minimum.is_some() || number.maximum.is_some())
                {
                    let min = number
                        .minimum
                        .as_ref()
                        .map(|n| format!("{:.0}", n))
                        .unwrap_or("*".to_string());
                    let max = number
                        .maximum
                        .as_ref()
                        .map(|n| format!("{:.0}", n))
                        .unwrap_or("*".to_string());
                    parts.push(format!("[{}..{}]", min, max));
                }
                return parts.join(" ");
            }
            InstanceType::Number => return "number".to_string(),
            InstanceType::Boolean => return "boolean".to_string(),
            InstanceType::Array => {
                if let Some(array) = schema.array.as_ref()
                    && let Some(item) = describe_array_items(array)
                {
                    // prefer detailed item description when available
                    return if item.is_empty() || item == "any" {
                        "array".to_string()
                    } else {
                        format!("{}[]", item)
                    };
                }
                return "array".to_string();
            }
            InstanceType::Object => return describe_object_shape(schema),
            InstanceType::Null => {}
        }
    }

    if let Some(subschemas) = schema.subschemas.as_ref() {
        if let Some(one_of) = subschemas.one_of.as_ref()
            && !one_of.is_empty()
        {
            return "oneOf".to_string();
        }
        if let Some(any_of) = subschemas.any_of.as_ref()
            && !any_of.is_empty()
        {
            return "anyOf".to_string();
        }
    }

    if schema
        .enum_values
        .as_ref()
        .is_some_and(|values| !values.is_empty())
    {
        return "enum".to_string();
    }

    String::new()
}

/// Describe an object schema using a small sample of property names
pub(super) fn describe_object_shape(schema: &SchemaObject) -> String {
    if let Some(object) = schema.object.as_ref()
        && !object.properties.is_empty()
    {
        let mut props: Vec<String> = object.properties.keys().take(3).cloned().collect();
        if object.properties.len() > 3 {
            props.push("…".to_string());
        }
        return format!("object({})", props.join(", "));
    }
    "object".to_string()
}

/// Describe the element type of an array schema
pub(super) fn describe_array_items(array: &ArrayValidation) -> Option<String> {
    let items = array.items.as_ref()?;
    match items {
        SingleOrVec::Single(schema) => Some(describe_schema_from_single(schema.as_ref())),
        SingleOrVec::Vec(list) => list.first().map(describe_schema_from_single),
    }
}

/// Describe the shape of a single schema entry (from SingleOrVec)
pub(super) fn describe_schema_from_single(schema: &Schema) -> String {
    match schema {
        Schema::Bool(true) => "any".to_string(),
        Schema::Bool(false) => "never".to_string(),
        Schema::Object(obj) => describe_schema_shape(obj),
    }
}
