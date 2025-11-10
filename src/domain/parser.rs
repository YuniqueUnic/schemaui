use anyhow::Result;
use serde_json::Value;

use crate::schema::build_form_schema;

use super::schema::FormSchema;

/// Parse a JSON schema document into the internal `FormSchema`.
pub fn parse_form_schema(value: &Value) -> Result<FormSchema> {
    build_form_schema(value)
}
