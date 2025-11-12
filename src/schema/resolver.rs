use anyhow::{Context, Result, bail};
use percent_encoding::percent_decode_str;
use schemars::schema::{RootSchema, Schema, SchemaObject};
use serde_json::Value;

#[derive(Debug)]
pub struct SchemaResolver<'a> {
    raw: &'a Value,
    root: &'a RootSchema,
}

impl<'a> SchemaResolver<'a> {
    pub fn new(raw: &'a Value, root: &'a RootSchema) -> Self {
        Self { raw, root }
    }

    pub fn root_object(&self) -> Option<&SchemaObject> {
        Some(&self.root.schema)
    }

    pub fn resolve_schema(&self, schema: &Schema) -> Result<SchemaObject> {
        match schema {
            Schema::Bool(value) => Ok(Schema::Bool(*value).into_object()),
            Schema::Object(object) => {
                if let Some(reference) = &object.reference {
                    self.follow_reference(reference)
                } else {
                    Ok(object.clone())
                }
            }
        }
    }

    pub fn definitions_snapshot(&self) -> Option<Value> {
        self.raw
            .as_object()
            .and_then(|obj| obj.get("definitions"))
            .cloned()
    }

    fn follow_reference(&self, reference: &str) -> Result<SchemaObject> {
        if let Some(key) = reference.strip_prefix("#/definitions/") {
            let target = self
                .root
                .definitions
                .get(key)
                .with_context(|| format!("definition '{key}' not found"))?;
            return self.resolve_schema(target);
        }

        if let Some(fragment) = reference.strip_prefix('#') {
            let decoded = percent_decode_str(fragment)
                .decode_utf8()
                .context("invalid percent-encoding in $ref")?;
            let pointer = if decoded.is_empty() {
                String::new()
            } else if decoded.starts_with('/') {
                decoded.to_string()
            } else {
                format!("/{}", decoded)
            };
            let target = self
                .raw
                .pointer(&pointer)
                .with_context(|| format!("reference '{reference}' not found"))?;
            let schema: Schema = serde_json::from_value(target.clone())
                .with_context(|| format!("reference '{reference}' is not a valid schema"))?;
            return self.resolve_schema(&schema);
        }

        bail!("unsupported reference {reference}")
    }
}
