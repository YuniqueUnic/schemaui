use std::collections::HashMap;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct FormSchema {
    #[allow(dead_code)]
    pub title: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub sections: Vec<FormSection>,
}

#[derive(Debug, Clone)]
pub struct FormSection {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub fields: Vec<FieldSchema>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FieldKind {
    String,
    Integer,
    Number,
    Boolean,
    Enum(Vec<String>),
    Array(Box<FieldKind>),
    Json,
    Composite(CompositeField),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeField {
    pub mode: CompositeMode,
    pub variants: Vec<CompositeVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CompositeMode {
    OneOf,
    AnyOf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeVariant {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub schema: Value,
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub path: Vec<String>,
    pub pointer: String,
    pub title: String,
    pub description: Option<String>,
    pub section_id: String,
    pub kind: FieldKind,
    pub required: bool,
    pub default: Option<Value>,
    #[allow(dead_code)]
    pub metadata: HashMap<String, Value>,
}

impl FieldSchema {
    pub fn display_label(&self) -> String {
        if self.title.eq_ignore_ascii_case(&self.name) {
            self.title.clone()
        } else {
            format!("{} ({})", self.title, self.name)
        }
    }
}
