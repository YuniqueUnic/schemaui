use anyhow::{Context, Result};
use serde_json::{Map, Number, Value, json};

use super::DocumentFormat;

const JSON_SCHEMA_DRAFT: &str = "http://json-schema.org/draft-07/schema#";

/// Parse structured data in any supported format into a `serde_json::Value`.
pub fn parse_document_str(contents: &str, format: DocumentFormat) -> Result<Value> {
    match format {
        DocumentFormat::Json => {
            serde_json::from_str::<Value>(contents).with_context(|| "failed to parse JSON document")
        }
        #[cfg(feature = "yaml")]
        DocumentFormat::Yaml => {
            serde_yaml::from_str::<Value>(contents).with_context(|| "failed to parse YAML document")
        }
        #[cfg(feature = "toml")]
        DocumentFormat::Toml => contents
            .parse::<toml::Value>()
            .with_context(|| "failed to parse TOML document")
            .and_then(|value| {
                serde_json::to_value(value).context("failed to convert TOML to JSON")
            }),
    }
}

/// Convert structured data into a JSON Schema with inferred defaults.
pub fn schema_from_data_str(contents: &str, format: DocumentFormat) -> Result<Value> {
    let value = parse_document_str(contents, format)?;
    Ok(schema_from_data_value(&value))
}

/// Convert structured data into a JSON Schema with inferred defaults.
pub fn schema_from_data_value(value: &Value) -> Value {
    let mut schema = infer_schema(value);
    if let Value::Object(ref mut map) = schema {
        map.entry("$schema".to_string())
            .or_insert_with(|| Value::String(JSON_SCHEMA_DRAFT.to_string()));
    }
    schema
}

fn infer_schema(value: &Value) -> Value {
    match value {
        Value::Null => schema_with_type("null", value),
        Value::Bool(_) => schema_with_type("boolean", value),
        Value::Number(num) => schema_with_type(number_type(num), value),
        Value::String(_) => schema_with_type("string", value),
        Value::Array(items) => array_schema(items),
        Value::Object(map) => object_schema(map),
    }
}

fn schema_with_type(kind: &str, default: &Value) -> Value {
    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String(kind.to_string()));
    schema.insert("default".to_string(), default.clone());
    Value::Object(schema)
}

fn object_schema(values: &Map<String, Value>) -> Value {
    let mut properties = Map::new();
    let mut required = Vec::new();
    for (key, value) in values {
        properties.insert(key.clone(), infer_schema(value));
        required.push(Value::String(key.clone()));
    }
    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("object".to_string()));
    schema.insert("default".to_string(), Value::Object(values.clone()));
    schema.insert("additionalProperties".to_string(), Value::Bool(true));
    if !properties.is_empty() {
        schema.insert("properties".to_string(), Value::Object(properties));
    }
    if !required.is_empty() {
        schema.insert("required".to_string(), Value::Array(required));
    }
    Value::Object(schema)
}

fn array_schema(items: &[Value]) -> Value {
    let mut schema = Map::new();
    schema.insert("type".to_string(), Value::String("array".to_string()));
    schema.insert("default".to_string(), Value::Array(items.to_vec()));
    if let Some(item_schema) = infer_items_schema(items) {
        schema.insert("items".to_string(), item_schema);
    }
    Value::Object(schema)
}

fn infer_items_schema(items: &[Value]) -> Option<Value> {
    let (first, rest) = items.split_first()?;
    if rest.is_empty() {
        return Some(infer_schema(first));
    }

    let mut variants = vec![infer_schema(first)];
    for item in rest {
        let schema = infer_schema(item);
        if !variants.iter().any(|existing| existing == &schema) {
            variants.push(schema);
        }
    }
    if variants.len() == 1 {
        variants.pop()
    } else {
        Some(json!({ "anyOf": variants }))
    }
}

fn number_type(number: &Number) -> &'static str {
    if number.is_i64() || number.is_u64() {
        "integer"
    } else {
        "number"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_schema_for_object_defaults() {
        let value = json!({"host": "localhost", "port": 8080});
        let schema = schema_from_data_value(&value);
        assert_eq!(schema["properties"]["host"]["default"], json!("localhost"));
        assert_eq!(schema["properties"]["port"]["type"], json!("integer"));
        assert_eq!(schema["default"], value);
    }

    #[test]
    fn builds_schema_for_array_defaults() {
        let value = json!(["a", "b"]);
        let schema = schema_from_data_value(&value);
        assert_eq!(schema["type"], json!("array"));
        assert_eq!(schema["default"], value);
        assert_eq!(schema["items"]["type"], json!("string"));
    }

    #[test]
    fn parse_json_documents() {
        let raw = "{\"enabled\":true}";
        let parsed = parse_document_str(raw, DocumentFormat::Json).unwrap();
        assert_eq!(parsed["enabled"], Value::Bool(true));
    }

    #[cfg(feature = "yaml")]
    #[test]
    fn parse_yaml_documents() {
        let raw = "enabled: true\nname: dev";
        let parsed = parse_document_str(raw, DocumentFormat::Yaml).unwrap();
        assert_eq!(parsed["enabled"], Value::Bool(true));
        assert_eq!(parsed["name"], json!("dev"));
    }

    #[cfg(feature = "toml")]
    #[test]
    fn parse_toml_documents() {
        let raw = "enabled = true\nname = \"dev\"";
        let parsed = parse_document_str(raw, DocumentFormat::Toml).unwrap();
        assert_eq!(parsed["enabled"], Value::Bool(true));
        assert_eq!(parsed["name"], json!("dev"));
    }
}
