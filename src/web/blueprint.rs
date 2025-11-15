use anyhow::Result;
use serde::Serialize;
use serde_json::{Value, json};
use ts_rs::TS;

use crate::domain::{
    CompositeMode, CompositeVariant, FieldKind, FieldSchema, FormSchema, FormSection, RootSection,
    parse_form_schema,
};

const WRAPPED_FIELD_NAME: &str = "__value";

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "web/types/")]
pub struct WebBlueprint {
    pub title: Option<String>,
    pub description: Option<String>,
    pub roots: Vec<WebRoot>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "web/types/")]
pub struct WebRoot {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub sections: Vec<WebSection>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "web/types/")]
pub struct WebSection {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<WebField>,
    pub sections: Vec<WebSection>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "web/types/")]
pub struct WebField {
    pub name: String,
    pub label: String,
    pub pointer: String,
    pub description: Option<String>,
    pub required: bool,
    pub kind: WebFieldKind,
    #[ts(type = "Record<string, unknown> | null")]
    pub default_value: Option<Value>,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
#[ts(export, export_to = "web/types/")]
pub enum WebFieldKind {
    String,
    Integer,
    Number,
    Boolean,
    Enum {
        options: Vec<String>,
    },
    Array {
        items: Box<WebFieldKind>,
    },
    Json,
    Composite {
        mode: WebCompositeMode,
        variants: Vec<WebCompositeVariant>,
    },
    KeyValue {
        key_title: String,
        key_description: Option<String>,
        value_title: String,
        value_description: Option<String>,
        value_kind: Box<WebFieldKind>,
    },
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "web/types/")]
pub enum WebCompositeMode {
    OneOf,
    AnyOf,
}

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "web/types/")]
pub struct WebCompositeVariant {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    #[ts(type = "Record<string, unknown>")]
    pub schema: Value,
    pub is_object: bool,
    pub sections: Vec<WebSection>,
}

pub fn blueprint_from_schema(schema: &Value) -> Result<WebBlueprint> {
    let form_schema = parse_form_schema(schema)?;
    Ok(WebBlueprint::from(&form_schema))
}

impl From<&FormSchema> for WebBlueprint {
    fn from(schema: &FormSchema) -> Self {
        Self {
            title: schema.title.clone(),
            description: schema.description.clone(),
            roots: schema.roots.iter().map(WebRoot::from).collect(),
        }
    }
}

impl From<&RootSection> for WebRoot {
    fn from(root: &RootSection) -> Self {
        Self {
            id: root.id.clone(),
            title: root.title.clone(),
            description: root.description.clone(),
            sections: root.sections.iter().map(WebSection::from).collect(),
        }
    }
}

impl From<&FormSection> for WebSection {
    fn from(section: &FormSection) -> Self {
        Self {
            id: section.id.clone(),
            title: section.title.clone(),
            description: section.description.clone(),
            fields: section.fields.iter().map(WebField::from).collect(),
            sections: section.children.iter().map(WebSection::from).collect(),
        }
    }
}

impl From<&FieldSchema> for WebField {
    fn from(field: &FieldSchema) -> Self {
        Self {
            name: field.name.clone(),
            label: field.display_label(),
            pointer: field.pointer.clone(),
            description: field.description.clone(),
            required: field.required,
            kind: WebFieldKind::from_field(&field.kind, &field.pointer),
            default_value: field.default.clone(),
        }
    }
}

impl From<&FieldKind> for WebFieldKind {
    fn from(kind: &FieldKind) -> Self {
        Self::from_field(kind, "/")
    }
}

impl WebFieldKind {
    fn from_field(kind: &FieldKind, pointer: &str) -> Self {
        match kind {
            FieldKind::String => WebFieldKind::String,
            FieldKind::Integer => WebFieldKind::Integer,
            FieldKind::Number => WebFieldKind::Number,
            FieldKind::Boolean => WebFieldKind::Boolean,
            FieldKind::Enum(options) => WebFieldKind::Enum {
                options: options.clone(),
            },
            FieldKind::Array(inner) => WebFieldKind::Array {
                items: Box::new(WebFieldKind::from_field(inner.as_ref(), pointer)),
            },
            FieldKind::Json => WebFieldKind::Json,
            FieldKind::Composite(field) => WebFieldKind::Composite {
                mode: match field.mode {
                    CompositeMode::OneOf => WebCompositeMode::OneOf,
                    CompositeMode::AnyOf => WebCompositeMode::AnyOf,
                },
                variants: field
                    .variants
                    .iter()
                    .map(|variant| build_web_variant(pointer, variant))
                    .collect(),
            },
            FieldKind::KeyValue(spec) => WebFieldKind::KeyValue {
                key_title: spec.key_title.clone(),
                key_description: spec.key_description.clone(),
                value_title: spec.value_title.clone(),
                value_description: spec.value_description.clone(),
                value_kind: Box::new(WebFieldKind::from_field(spec.value_kind.as_ref(), pointer)),
            },
        }
    }
}

fn build_web_variant(base_pointer: &str, variant: &CompositeVariant) -> WebCompositeVariant {
    let overlay = overlay_schema(variant);
    let sections = parse_form_schema(&overlay)
        .map(|schema| {
            let blueprint = WebBlueprint::from(&schema);
            blueprint
                .roots
                .into_iter()
                .flat_map(|root| {
                    root.sections
                        .into_iter()
                        .map(|mut section| {
                            prefix_section_pointers(&mut section, base_pointer);
                            section
                        })
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|err| {
            eprintln!(
                "Failed to build composite variant blueprint for '{}': {err}",
                variant.title
            );
            Vec::new()
        });

    WebCompositeVariant {
        id: variant.id.clone(),
        title: variant.title.clone(),
        description: variant.description.clone(),
        schema: variant.schema.clone(),
        is_object: variant.is_object,
        sections,
    }
}

fn overlay_schema(variant: &CompositeVariant) -> Value {
    if variant.is_object {
        variant.schema.clone()
    } else {
        wrap_non_object_schema(
            &variant.schema,
            &variant.title,
            variant.description.as_deref(),
        )
    }
}

fn wrap_non_object_schema(schema: &Value, title: &str, description: Option<&str>) -> Value {
    let mut property = schema.clone();
    if let Value::Object(ref mut map) = property {
        map.entry("title".to_string())
            .or_insert_with(|| Value::String(title.to_string()));
        if let Some(desc) = description
            && !map.contains_key("description")
        {
            map.insert("description".to_string(), Value::String(desc.to_string()));
        }
    }
    json!({
        "type": "object",
        "title": title,
        "properties": {
            WRAPPED_FIELD_NAME: property
        },
        "required": [WRAPPED_FIELD_NAME]
    })
}

fn prefix_section_pointers(section: &mut WebSection, base_pointer: &str) {
    for field in &mut section.fields {
        field.pointer = join_pointer(base_pointer, &field.pointer);
    }
    for child in &mut section.sections {
        prefix_section_pointers(child, base_pointer);
    }
}

fn join_pointer(base: &str, child: &str) -> String {
    let base = if base.is_empty() { "/" } else { base };
    if child.is_empty() || child == "/" {
        return base.to_string();
    }
    if base.is_empty() || base == "/" {
        return child.to_string();
    }
    let separator = if base.ends_with('/') || child.starts_with('/') {
        ""
    } else {
        "/"
    };
    format!(
        "{}{}{}",
        base.trim_end_matches('/'),
        separator,
        child.trim_start_matches('/')
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn builds_basic_blueprint() {
        let schema = json!({
            "type": "object",
            "title": "Server",
            "properties": {
                "host": {"type": "string", "default": "localhost"},
                "port": {"type": "integer", "default": 3000},
                "roles": {
                    "type": "array",
                    "items": {"type": "string"}
                }
            },
            "required": ["host"]
        });
        let blueprint = blueprint_from_schema(&schema).unwrap();
        assert_eq!(blueprint.title.as_deref(), Some("Server"));
        let root = blueprint.roots.first().unwrap();
        let section = root.sections.first().unwrap();
        assert_eq!(section.fields.len(), 3);
        assert!(
            section
                .fields
                .iter()
                .any(|f| f.name == "host" && f.required)
        );
    }
}
