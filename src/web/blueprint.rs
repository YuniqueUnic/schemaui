use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

use crate::domain::{
    CompositeMode, FieldKind, FieldSchema, FormSchema, FormSection, RootSection, parse_form_schema,
};

#[derive(Debug, Clone, Serialize)]
pub struct WebBlueprint {
    pub title: Option<String>,
    pub description: Option<String>,
    pub roots: Vec<WebRoot>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebRoot {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub sections: Vec<WebSection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebSection {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<WebField>,
    pub sections: Vec<WebSection>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebField {
    pub name: String,
    pub label: String,
    pub pointer: String,
    pub description: Option<String>,
    pub required: bool,
    pub kind: WebFieldKind,
    pub default_value: Option<Value>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum WebCompositeMode {
    OneOf,
    AnyOf,
}

#[derive(Debug, Clone, Serialize)]
pub struct WebCompositeVariant {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub schema: Value,
    pub is_object: bool,
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
            kind: WebFieldKind::from(&field.kind),
            default_value: field.default.clone(),
        }
    }
}

impl From<&FieldKind> for WebFieldKind {
    fn from(kind: &FieldKind) -> Self {
        match kind {
            FieldKind::String => WebFieldKind::String,
            FieldKind::Integer => WebFieldKind::Integer,
            FieldKind::Number => WebFieldKind::Number,
            FieldKind::Boolean => WebFieldKind::Boolean,
            FieldKind::Enum(options) => WebFieldKind::Enum {
                options: options.clone(),
            },
            FieldKind::Array(inner) => WebFieldKind::Array {
                items: Box::new(WebFieldKind::from(inner.as_ref())),
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
                    .map(|variant| WebCompositeVariant {
                        id: variant.id.clone(),
                        title: variant.title.clone(),
                        description: variant.description.clone(),
                        schema: variant.schema.clone(),
                        is_object: variant.is_object,
                    })
                    .collect(),
            },
            FieldKind::KeyValue(spec) => WebFieldKind::KeyValue {
                key_title: spec.key_title.clone(),
                key_description: spec.key_description.clone(),
                value_title: spec.value_title.clone(),
                value_description: spec.value_description.clone(),
                value_kind: Box::new(WebFieldKind::from(spec.value_kind.as_ref())),
            },
        }
    }
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
