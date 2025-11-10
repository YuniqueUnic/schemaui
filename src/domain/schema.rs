use std::collections::HashMap;

use serde_json::Value;

#[derive(Debug, Clone)]
pub struct FormSchema {
    #[allow(dead_code)]
    pub title: Option<String>,
    #[allow(dead_code)]
    pub description: Option<String>,
    pub roots: Vec<RootSection>,
}

#[derive(Debug, Clone)]
pub struct RootSection {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub sections: Vec<FormSection>,
}

#[derive(Debug, Clone)]
pub struct FormSection {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub path: Vec<String>,
    pub fields: Vec<FieldSchema>,
    pub children: Vec<FormSection>,
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
    KeyValue(KeyValueField),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeField {
    pub mode: CompositeMode,
    pub variants: Vec<CompositeVariant>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KeyValueField {
    pub key_title: String,
    pub key_description: Option<String>,
    pub key_default: Option<Value>,
    pub key_schema: Value,
    pub value_title: String,
    pub value_description: Option<String>,
    pub value_default: Option<Value>,
    pub value_schema: Value,
    pub value_kind: Box<FieldKind>,
    pub entry_schema: Value,
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
    #[allow(dead_code)]
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
