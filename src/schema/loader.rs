use anyhow::{Context, Result};
use schemars::schema::RootSchema;
use serde_json::Value;

/// Deserialize a JSON value into a `RootSchema`.
pub fn load_root_schema(value: &Value) -> Result<RootSchema> {
    serde_json::from_value(value.clone()).context("schema is not a valid JSON Schema document")
}
