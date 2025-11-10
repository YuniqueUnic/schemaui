use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use percent_encoding::percent_decode_str;
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, RootSchema, Schema, SchemaObject, SingleOrVec,
};
use serde_json::Value;

use super::schema::{
    CompositeField, CompositeMode, CompositeVariant, FieldKind, FieldSchema, FormSchema,
    FormSection,
};

#[derive(Debug, Clone)]
struct SectionInfo {
    id: String,
    title: String,
    description: Option<String>,
}

pub fn parse_form_schema(schema_value: &Value) -> Result<FormSchema> {
    let root: RootSchema = serde_json::from_value(schema_value.clone())
        .context("schema is not a valid JSON Schema document")?;
    let context = SchemaContext::new(schema_value, &root);
    let root_object = context
        .root_object()
        .cloned()
        .ok_or_else(|| anyhow!("root schema must be an object"))?;
    ensure_object_schema(&root_object)?;

    let mut section_meta: IndexMap<String, SectionInfo> = IndexMap::new();
    let mut section_fields: IndexMap<String, Vec<FieldSchema>> = IndexMap::new();

    parse_object_fields(
        &context,
        &root_object,
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
        title: root_object.metadata.as_ref().and_then(|m| m.title.clone()),
        description: root_object
            .metadata
            .as_ref()
            .and_then(|m| m.description.clone()),
        sections,
    })
}

fn parse_object_fields(
    context: &SchemaContext<'_>,
    schema: &SchemaObject,
    path_prefix: Vec<String>,
    parent_section: Option<SectionInfo>,
    meta: &mut IndexMap<String, SectionInfo>,
    slots: &mut IndexMap<String, Vec<FieldSchema>>,
) -> Result<()> {
    let object = schema
        .object
        .as_ref()
        .context("object schema must define properties")?;
    let required = required_set(object);

    for (name, property_schema) in &object.properties {
        let mut next_path = path_prefix.clone();
        next_path.push(name.clone());
        let resolved = context.resolve_schema(property_schema)?;

        let descend = is_object_schema(&resolved)
            && resolved
                .object
                .as_ref()
                .map(|obj| !obj.properties.is_empty())
                .unwrap_or(false)
            && !has_composite_subschemas(&resolved);

        if descend {
            let next_section = section_info_for_object(&resolved, name, parent_section.as_ref());
            meta.entry(next_section.id.clone())
                .or_insert_with(|| next_section.clone());
            slots.entry(next_section.id.clone()).or_default();
            parse_object_fields(
                context,
                &resolved,
                next_path,
                Some(next_section),
                meta,
                slots,
            )?;
            continue;
        }

        let section = section_info_for_field(&resolved, &next_path, parent_section.as_ref());
        meta.entry(section.id.clone())
            .or_insert_with(|| section.clone());

        let field = build_field_schema(
            context,
            &resolved,
            name,
            next_path,
            section,
            required.contains(name),
        )?;
        slots
            .entry(field.section_id.clone())
            .or_default()
            .push(field);
    }

    if let Some(additional) = object.additional_properties.as_ref() {
        let resolved = context.resolve_schema(additional)?;
        let field_name = path_prefix
            .last()
            .cloned()
            .unwrap_or_else(|| "additional".to_string());
        let section = section_info_for_field(&resolved, &path_prefix, parent_section.as_ref());
        meta.entry(section.id.clone())
            .or_insert_with(|| section.clone());
        let field = build_field_schema(
            context,
            &resolved,
            &field_name,
            path_prefix.clone(),
            section,
            false,
        )?;
        slots
            .entry(field.section_id.clone())
            .or_default()
            .push(field);
    }

    Ok(())
}

fn build_field_schema(
    context: &SchemaContext<'_>,
    schema: &SchemaObject,
    name: &str,
    path: Vec<String>,
    section: SectionInfo,
    required: bool,
) -> Result<FieldSchema> {
    let metadata = metadata_map(schema);
    let kind = detect_kind(context, schema)
        .with_context(|| format!("unsupported schema for field '{name}'"))?;
    let title = schema
        .metadata
        .as_ref()
        .and_then(|m| m.title.clone())
        .unwrap_or_else(|| prettify_label(name));
    let default = schema.metadata.as_ref().and_then(|m| m.default.clone());
    let description = schema.metadata.as_ref().and_then(|m| m.description.clone());

    Ok(FieldSchema {
        name: name.to_string(),
        path: path.clone(),
        pointer: to_pointer(&path),
        title,
        description,
        section_id: section.id,
        kind,
        required,
        default,
        metadata,
    })
}

fn detect_kind(context: &SchemaContext<'_>, schema: &SchemaObject) -> Result<FieldKind> {
    if let Some(composite) = composite_field(context, schema)? {
        return Ok(FieldKind::Composite(composite));
    }
    if let Some(options) = &schema.enum_values {
        let enum_values = options
            .iter()
            .map(|value| match value {
                Value::String(s) => Ok(s.clone()),
                other => Ok(other.to_string()),
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        return Ok(FieldKind::Enum(enum_values));
    }

    match instance_type(schema) {
        Some(InstanceType::String) | None => Ok(FieldKind::String),
        Some(InstanceType::Integer) => Ok(FieldKind::Integer),
        Some(InstanceType::Number) => Ok(FieldKind::Number),
        Some(InstanceType::Boolean) => Ok(FieldKind::Boolean),
        Some(InstanceType::Object) => Ok(FieldKind::Json),
        Some(InstanceType::Array) => {
            let array = schema
                .array
                .as_ref()
                .context("array schema must define items")?;
            let inner = resolve_array_items(context, array)?;
            let inner_kind = detect_kind(context, &inner)?;
            match inner_kind {
                FieldKind::String
                | FieldKind::Integer
                | FieldKind::Number
                | FieldKind::Boolean
                | FieldKind::Enum(_)
                | FieldKind::Json
                | FieldKind::Composite(_) => Ok(FieldKind::Array(Box::new(inner_kind))),
                FieldKind::Array(_) => bail!("nested arrays are not supported"),
            }
        }
        Some(other) => bail!("unsupported field type {other:?}"),
    }
}

fn composite_field(
    context: &SchemaContext<'_>,
    schema: &SchemaObject,
) -> Result<Option<CompositeField>> {
    let Some(subschemas) = schema.subschemas.as_ref() else {
        return Ok(None);
    };
    if let Some(one_of) = subschemas.one_of.as_ref() {
        return build_composite(context, CompositeMode::OneOf, one_of);
    }
    if let Some(any_of) = subschemas.any_of.as_ref() {
        return build_composite(context, CompositeMode::AnyOf, any_of);
    }
    Ok(None)
}

fn build_composite(
    context: &SchemaContext<'_>,
    mode: CompositeMode,
    schemas: &[Schema],
) -> Result<Option<CompositeField>> {
    if schemas.is_empty() {
        return Ok(None);
    }

    let mut variants = Vec::new();
    for (index, variant) in schemas.iter().enumerate() {
        let resolved = context.resolve_schema(variant)?;
        ensure_object_schema(&resolved)?;
        let schema_value = serde_json::to_value(&Schema::Object(resolved.clone()))
            .context("failed to serialize composite variant schema")?;
        let title = resolved
            .metadata
            .as_ref()
            .and_then(|m| m.title.clone())
            .unwrap_or_else(|| format!("Variant {}", index + 1));
        let description = resolved
            .metadata
            .as_ref()
            .and_then(|m| m.description.clone());
        variants.push(CompositeVariant {
            id: format!("variant_{}", index),
            title,
            description,
            schema: schema_value,
        });
    }

    Ok(Some(CompositeField { mode, variants }))
}

fn resolve_array_items(
    context: &SchemaContext<'_>,
    array: &ArrayValidation,
) -> Result<SchemaObject> {
    let items = array
        .items
        .as_ref()
        .context("array schema must define items")?;
    match items {
        SingleOrVec::Single(schema) => context.resolve_schema(schema),
        SingleOrVec::Vec(list) => match list.first() {
            Some(first) => context.resolve_schema(first),
            None => bail!("tuple arrays without items are not supported"),
        },
    }
}

fn section_info_for_object(
    schema: &SchemaObject,
    name: &str,
    parent: Option<&SectionInfo>,
) -> SectionInfo {
    if let Some(group) = extension_string(schema, "x-group") {
        let title =
            extension_string(schema, "x-group-title").unwrap_or_else(|| prettify_label(&group));
        let description = extension_string(schema, "x-group-description");
        return SectionInfo {
            id: group,
            title,
            description,
        };
    }

    SectionInfo {
        id: name.to_string(),
        title: schema
            .metadata
            .as_ref()
            .and_then(|m| m.title.clone())
            .unwrap_or_else(|| prettify_label(name)),
        description: schema
            .metadata
            .as_ref()
            .and_then(|m| m.description.clone())
            .or_else(|| parent.and_then(|p| p.description.clone())),
    }
}

fn section_info_for_field(
    schema: &SchemaObject,
    path: &[String],
    parent: Option<&SectionInfo>,
) -> SectionInfo {
    if let Some(group) = extension_string(schema, "x-group") {
        let title =
            extension_string(schema, "x-group-title").unwrap_or_else(|| prettify_label(&group));
        let description = extension_string(schema, "x-group-description");
        return SectionInfo {
            id: group,
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

fn metadata_map(schema: &SchemaObject) -> HashMap<String, Value> {
    schema
        .extensions
        .iter()
        .filter(|(key, _)| key.starts_with("x-"))
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn extension_string(schema: &SchemaObject, key: &str) -> Option<String> {
    schema
        .extensions
        .get(key)
        .and_then(|value| value.as_str().map(str::to_string))
}

fn required_set(object: &ObjectValidation) -> HashSet<String> {
    object.required.iter().cloned().collect()
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

fn is_object_schema(schema: &SchemaObject) -> bool {
    match instance_type(schema) {
        Some(InstanceType::Object) => true,
        None => schema.object.is_some(),
        _ => false,
    }
}

fn instance_type(schema: &SchemaObject) -> Option<InstanceType> {
    schema.instance_type.as_ref().and_then(|kind| match kind {
        SingleOrVec::Single(single) => Some((**single).clone()),
        SingleOrVec::Vec(items) => items
            .iter()
            .cloned()
            .find(|item| *item != InstanceType::Null),
    })
}

fn ensure_object_schema(schema: &SchemaObject) -> Result<()> {
    if is_object_schema(schema) {
        Ok(())
    } else {
        bail!("schema must describe an object")
    }
}

fn has_composite_subschemas(schema: &SchemaObject) -> bool {
    schema
        .subschemas
        .as_ref()
        .map(|subs| subs.one_of.is_some() || subs.any_of.is_some())
        .unwrap_or(false)
}

struct SchemaContext<'a> {
    raw: &'a Value,
    root: &'a RootSchema,
}

impl<'a> SchemaContext<'a> {
    fn new(raw: &'a Value, root: &'a RootSchema) -> Self {
        Self { raw, root }
    }

    fn root_object(&self) -> Option<&SchemaObject> {
        Some(&self.root.schema)
    }

    fn resolve_schema(&self, schema: &Schema) -> Result<SchemaObject> {
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
