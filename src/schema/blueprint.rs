#![allow(unused)] // TODO: maybe this blueprint should be total refactor

use anyhow::Result;
use serde_json::{Map, Value, json};

use crate::domain::{
    CompositeField, CompositeMode, CompositeVariant, FieldKind, FieldSchema, FormSchema,
    FormSection, RootSection,
};

use super::layout::build_form_schema;

/// Builds the JSON UI blueprint (“json-ui-schema”) from a JSON Schema document.
/// The blueprint captures the sections, fields, and widgets needed by the TUI (or
/// any other renderer) without binding to ratatui-specific types.
pub fn build_ui_blueprint(schema_value: &Value) -> Result<Value> {
    let schema = build_form_schema(schema_value)?;
    Ok(form_schema_blueprint(&schema))
}

/// Converts an already parsed [`FormSchema`] into the JSON UI blueprint.
pub fn form_schema_blueprint(schema: &FormSchema) -> Value {
    json!({
        "title": schema.title,
        "description": schema.description,
        "roots": schema.roots.iter().map(root_blueprint).collect::<Vec<_>>(),
    })
}

fn root_blueprint(root: &RootSection) -> Value {
    json!({
        "id": root.id,
        "title": root.title,
        "description": root.description,
        "sections": root.sections.iter().map(section_blueprint).collect::<Vec<_>>(),
    })
}

fn section_blueprint(section: &FormSection) -> Value {
    json!({
        "id": section.id,
        "title": section.title,
        "description": section.description,
        "path": section.path,
        "fields": section.fields.iter().map(field_blueprint).collect::<Vec<_>>(),
        "children": section.children.iter().map(section_blueprint).collect::<Vec<_>>(),
    })
}

fn field_blueprint(field: &FieldSchema) -> Value {
    let mut base = Map::new();
    base.insert("name".into(), Value::String(field.name.clone()));
    base.insert("pointer".into(), Value::String(field.pointer.clone()));
    base.insert("title".into(), Value::String(field.title.clone()));
    if let Some(desc) = &field.description {
        base.insert("description".into(), Value::String(desc.clone()));
    }
    base.insert("required".into(), Value::Bool(field.required));
    if let Some(default) = &field.default {
        base.insert("default".into(), default.clone());
    }
    if !field.metadata.is_empty()
        && let Ok(value) = serde_json::to_value(&field.metadata)
    {
        base.insert("metadata".into(), value);
    }
    base.insert("widget".into(), field_kind_blueprint(&field.kind));
    Value::Object(base)
}

fn field_kind_blueprint(kind: &FieldKind) -> Value {
    match kind {
        FieldKind::String => json!({"component": "text", "data_type": "string"}),
        FieldKind::Integer => json!({"component": "text", "data_type": "integer"}),
        FieldKind::Number => json!({"component": "text", "data_type": "number"}),
        FieldKind::Boolean => json!({"component": "boolean"}),
        FieldKind::Enum(options) => json!({"component": "enum", "options": options}),
        FieldKind::Array(inner) => match inner.as_ref() {
            FieldKind::Enum(options) => json!({
                "component": "multi_select",
                "options": options,
            }),
            FieldKind::Composite(meta) => composite_collection_blueprint(meta),
            FieldKind::String | FieldKind::Integer | FieldKind::Number | FieldKind::Boolean => {
                json!({
                    "component": "scalar_array",
                    "item": field_kind_blueprint(inner),
                })
            }
            _ => json!({
                "component": "array",
                "item": field_kind_blueprint(inner),
            }),
        },
        FieldKind::Json => json!({"component": "json"}),
        FieldKind::Composite(meta) => composite_blueprint(meta),
        FieldKind::KeyValue(template) => json!({
            "component": "key_value",
            "key": {
                "title": template.key_title,
                "description": template.key_description,
                "default": template.key_default,
                "schema": template.key_schema,
            },
            "value": {
                "title": template.value_title,
                "description": template.value_description,
                "default": template.value_default,
                "schema": template.value_schema,
                "kind": field_kind_blueprint(&template.value_kind),
            },
            "entry_schema": template.entry_schema,
        }),
    }
}

fn composite_mode_name(mode: &CompositeMode) -> &'static str {
    match mode {
        CompositeMode::OneOf => "oneOf",
        CompositeMode::AnyOf => "anyOf",
    }
}

fn composite_blueprint(meta: &CompositeField) -> Value {
    json!({
        "component": "composite",
        "mode": composite_mode_name(&meta.mode),
        "multi": matches!(meta.mode, CompositeMode::AnyOf),
        "variants": variants_blueprint(&meta.variants),
    })
}

fn composite_collection_blueprint(meta: &CompositeField) -> Value {
    json!({
        "component": "composite_list",
        "mode": composite_mode_name(&meta.mode),
        "multi": matches!(meta.mode, CompositeMode::AnyOf),
        "variants": variants_blueprint(&meta.variants),
    })
}

fn variants_blueprint(variants: &[CompositeVariant]) -> Vec<Value> {
    variants.iter().map(variant_blueprint).collect()
}

fn variant_blueprint(variant: &CompositeVariant) -> Value {
    json!({
        "id": variant.id,
        "title": variant.title,
        "description": variant.description,
        "schema": variant.schema,
        "is_object": variant.is_object,
    })
}
