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
    Composite(Box<CompositeField>),
    KeyValue(Box<KeyValueField>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct CompositeField {
    pub mode: CompositeMode,
    pub variants: Vec<CompositeVariant>,
}

impl CompositeField {
    pub fn variant_titles(&self) -> Vec<String> {
        self.variants.iter().map(|v| v.title.clone()).collect()
    }

    pub fn variant_count(&self) -> usize {
        self.variants.len()
    }
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
    pub is_object: bool,
}

#[derive(Debug, Clone)]
pub struct FieldSchema {
    pub name: String,
    pub path: Vec<String>,
    pub pointer: String,
    pub title: String,
    pub description: Option<String>,
    pub kind: FieldKind,
    pub required: bool,
    pub default: Option<Value>,
    pub metadata: HashMap<String, Value>,
}

impl FieldSchema {
    pub fn display_label(&self) -> String {
        // Field names starting with "__" are reserved for internal plumbing
        // (e.g. composite wrappers) and should not surface their raw name.
        if self.name.starts_with("__") {
            return self.title.clone();
        }
        if self.title.eq_ignore_ascii_case(&self.name) {
            self.title.clone()
        } else {
            format!("{} ({})", self.title, self.name)
        }
    }
}
