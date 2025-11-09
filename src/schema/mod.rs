use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};
use indexmap::IndexMap;
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

#[derive(Debug, Clone)]
struct SectionInfo {
    id: String,
    title: String,
    description: Option<String>,
}

pub fn parse_form_schema(schema: &Value) -> Result<FormSchema> {
    ensure_object(schema)?;
    let schema_type = read_type(schema).unwrap_or_else(|| "object".to_string());
    if schema_type != "object" {
        bail!("root schema must be an object, found {schema_type}");
    }

    let mut section_meta: IndexMap<String, SectionInfo> = IndexMap::new();
    let mut section_fields: IndexMap<String, Vec<FieldSchema>> = IndexMap::new();

    parse_object_fields(
        schema,
        Vec::new(),
        None,
        &mut section_meta,
        &mut section_fields,
    )?;

    let sections = section_meta
        .into_iter()
        .map(|(id, meta)| FormSection {
            title: meta.title,
            description: meta.description,
            fields: section_fields.shift_remove(&id).unwrap_or_default(),
            id,
        })
        .collect();

    Ok(FormSchema {
        title: schema
            .get("title")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        description: schema
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        sections,
    })
}

fn parse_object_fields(
    schema: &Value,
    path_prefix: Vec<String>,
    parent_section: Option<SectionInfo>,
    meta: &mut IndexMap<String, SectionInfo>,
    slots: &mut IndexMap<String, Vec<FieldSchema>>,
) -> Result<()> {
    let properties = schema
        .get("properties")
        .and_then(Value::as_object)
        .context("object schema must define properties")?;
    let required = required_set(schema);

    for (name, value) in properties {
        let mut next_path = path_prefix.clone();
        next_path.push(name.clone());
        if is_object(value) {
            let next_section = section_info_for_object(value, name, parent_section.as_ref());
            meta.entry(next_section.id.clone())
                .or_insert_with(|| next_section.clone());
            slots.entry(next_section.id.clone()).or_default();
            parse_object_fields(value, next_path, Some(next_section), meta, slots)?;
            continue;
        }

        let section = section_info_for_field(value, &next_path, parent_section.as_ref());
        meta.entry(section.id.clone())
            .or_insert_with(|| section.clone());

        let field = build_field_schema(value, name, next_path, section, required.contains(name))?;
        slots
            .entry(field.section_id.clone())
            .or_default()
            .push(field);
    }

    Ok(())
}

fn build_field_schema(
    value: &Value,
    name: &str,
    path: Vec<String>,
    section: SectionInfo,
    required: bool,
) -> Result<FieldSchema> {
    let metadata = metadata_map(value);
    let kind =
        detect_kind(value).with_context(|| format!("unsupported schema for field '{name}'"))?;
    let title = value
        .get("title")
        .and_then(|v| v.as_str())
        .map(str::to_string)
        .unwrap_or_else(|| prettify_label(name));

    Ok(FieldSchema {
        name: name.to_string(),
        path: path.clone(),
        pointer: to_pointer(&path),
        title,
        description: value
            .get("description")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        section_id: section.id,
        kind,
        required,
        default: value.get("default").cloned(),
        metadata,
    })
}

fn detect_kind(value: &Value) -> Result<FieldKind> {
    if let Some(options) = value.get("enum").and_then(Value::as_array) {
        let enum_values = options
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                other => Ok(other.to_string()),
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        return Ok(FieldKind::Enum(enum_values));
    }

    match read_type(value).as_deref() {
        Some("string") | None => Ok(FieldKind::String),
        Some("integer") => Ok(FieldKind::Integer),
        Some("number") => Ok(FieldKind::Number),
        Some("boolean") => Ok(FieldKind::Boolean),
        Some("array") => {
            let items = value
                .get("items")
                .context("array schema must define items")?;
            let inner = detect_kind(items)?;
            match inner {
                FieldKind::String
                | FieldKind::Integer
                | FieldKind::Number
                | FieldKind::Boolean
                | FieldKind::Enum(_) => Ok(FieldKind::Array(Box::new(inner))),
                FieldKind::Array(_) => bail!("nested arrays are not supported"),
            }
        }
        Some(other) => bail!("unsupported field type {other}"),
    }
}

fn section_info_for_object(
    schema: &Value,
    name: &str,
    parent: Option<&SectionInfo>,
) -> SectionInfo {
    if let Some(group) = schema.get("x-group").and_then(Value::as_str) {
        let title = schema
            .get("x-group-title")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| prettify_label(group));
        let description = schema
            .get("x-group-description")
            .and_then(Value::as_str)
            .map(str::to_string);
        return SectionInfo {
            id: group.to_string(),
            title,
            description,
        };
    }

    SectionInfo {
        id: name.to_string(),
        title: schema
            .get("title")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| prettify_label(name)),
        description: schema
            .get("description")
            .and_then(Value::as_str)
            .map(str::to_string)
            .or_else(|| parent.and_then(|p| p.description.clone())),
    }
}

fn section_info_for_field(
    schema: &Value,
    path: &[String],
    parent: Option<&SectionInfo>,
) -> SectionInfo {
    if let Some(group) = schema.get("x-group").and_then(Value::as_str) {
        let title = schema
            .get("x-group-title")
            .and_then(Value::as_str)
            .map(str::to_string)
            .unwrap_or_else(|| prettify_label(group));
        let description = schema
            .get("x-group-description")
            .and_then(Value::as_str)
            .map(str::to_string);
        return SectionInfo {
            id: group.to_string(),
            title,
            description,
        };
    }

    if let Some(parent_section) = parent {
        return parent_section.clone();
    }

    if path.len() > 1 {
        let id = path
            .first()
            .cloned()
            .unwrap_or_else(|| "general".to_string());
        return SectionInfo {
            title: prettify_label(&id),
            description: None,
            id,
        };
    }

    SectionInfo {
        id: "general".to_string(),
        title: "General".to_string(),
        description: None,
    }
}

fn read_type(value: &Value) -> Option<String> {
    match value.get("type")? {
        Value::String(s) => Some(s.to_lowercase()),
        Value::Array(items) => items
            .iter()
            .filter_map(Value::as_str)
            .map(|s| s.to_lowercase())
            .find(|s| s != "null"),
        _ => None,
    }
}

fn metadata_map(value: &Value) -> HashMap<String, Value> {
    match value.as_object() {
        Some(obj) => obj
            .iter()
            .filter(|(key, _)| key.starts_with("x-"))
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect(),
        None => HashMap::new(),
    }
}

fn required_set(schema: &Value) -> HashSet<String> {
    schema
        .get("required")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn to_pointer(path: &[String]) -> String {
    if path.is_empty() {
        return String::new();
    }

    path.iter()
        .map(|segment| segment.replace('~', "~0").replace('/', "~1"))
        .fold(String::new(), |mut acc, segment| {
            acc.push('/');
            acc.push_str(&segment);
            acc
        })
}

fn prettify_label(raw: &str) -> String {
    if raw.is_empty() {
        return String::new();
    }

    let mut result = String::with_capacity(raw.len());
    let mut capitalize = true;
    for ch in raw.chars() {
        if ch == '_' || ch == '-' {
            result.push(' ');
            capitalize = true;
            continue;
        }

        if capitalize {
            result.push(ch.to_ascii_uppercase());
            capitalize = false;
        } else {
            result.push(ch);
        }
    }

    result.trim().to_string()
}

fn is_object(value: &Value) -> bool {
    match read_type(value) {
        Some(ty) => ty == "object",
        None => value.get("properties").is_some(),
    }
}

fn ensure_object(value: &Value) -> Result<()> {
    if value.is_object() {
        Ok(())
    } else {
        bail!("schema must be a JSON object")
    }
}
