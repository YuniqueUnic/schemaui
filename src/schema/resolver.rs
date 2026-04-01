use anyhow::{Context, Result, bail};
use percent_encoding::percent_decode_str;
use schemars::schema::{Metadata, RootSchema, Schema, SchemaObject};
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
                    let resolved = self.follow_reference(reference)?;
                    Ok(overlay_reference_annotations(resolved, object))
                } else {
                    Ok(object.clone())
                }
            }
        }
    }

    pub fn definitions_snapshot(&self) -> Option<Value> {
        let obj = self.raw.as_object()?;
        obj.get("$defs").or_else(|| obj.get("definitions")).cloned()
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

pub fn schema_reference(schema: &Schema) -> Option<&str> {
    match schema {
        Schema::Object(object) => object.reference.as_deref(),
        Schema::Bool(_) => None,
    }
}

fn overlay_reference_annotations(mut target: SchemaObject, source: &SchemaObject) -> SchemaObject {
    if source.metadata.is_some() {
        target.metadata = Some(Box::new(merge_metadata(
            target.metadata.as_deref(),
            source.metadata.as_deref(),
        )));
    }

    if !source.extensions.is_empty() {
        for (key, value) in &source.extensions {
            if key.starts_with("x-") {
                target.extensions.insert(key.clone(), value.clone());
            }
        }
    }

    target
}

fn merge_metadata(target: Option<&Metadata>, source: Option<&Metadata>) -> Metadata {
    let mut merged = target.cloned().unwrap_or_default();
    let Some(source) = source else {
        return merged;
    };

    if let Some(title) = source.title.clone() {
        merged.title = Some(title);
    }
    if let Some(description) = source.description.clone() {
        merged.description = Some(description);
    }
    if source.default.is_some() {
        merged.default = source.default.clone();
    }
    if source.deprecated {
        merged.deprecated = true;
    }
    if source.read_only {
        merged.read_only = true;
    }
    if source.write_only {
        merged.write_only = true;
    }
    if !source.examples.is_empty() {
        merged.examples = source.examples.clone();
    }

    merged
}
