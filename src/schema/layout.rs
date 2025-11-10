use std::collections::HashSet;

use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec,
};
use serde_json::Value;

use crate::domain::{
    CompositeField, CompositeMode, CompositeVariant, FieldKind, FieldSchema, FormSchema,
    FormSection, KeyValueField, RootSection,
};

use super::{
    loader::load_root_schema,
    metadata::{
        SectionInfo, general_section_info, metadata_map, prettify_label, section_info_for_object,
    },
    resolver::SchemaResolver,
};

#[derive(Debug, Clone)]
struct RootBuilder {
    id: String,
    title: String,
    description: Option<String>,
    sections: Vec<FormSection>,
}

impl RootBuilder {
    fn new(name: &str, schema: &SchemaObject) -> Self {
        let meta = section_info_for_object(schema, name, None);
        Self {
            id: name.to_string(),
            title: meta.title,
            description: meta.description,
            sections: Vec::new(),
        }
    }

    fn into_root(self) -> RootSection {
        RootSection {
            id: self.id,
            title: self.title,
            description: self.description,
            sections: self.sections,
        }
    }
}

pub fn build_form_schema(schema_value: &Value) -> Result<FormSchema> {
    let root = load_root_schema(schema_value)?;
    let resolver = SchemaResolver::new(schema_value, &root);
    let root_object = resolver
        .root_object()
        .cloned()
        .ok_or_else(|| anyhow!("root schema must be an object"))?;
    ensure_object_schema(&root_object)?;

    let mut roots: IndexMap<String, RootBuilder> = IndexMap::new();
    let mut general_fields: Vec<(usize, FieldSchema)> = Vec::new();
    let mut order_counter = 0usize;
    let object = root_object
        .object
        .as_ref()
        .context("root schema must define properties")?;
    let required = required_set(object);

    for (name, property_schema) in &object.properties {
        let path = vec![name.clone()];
        let resolved = resolver.resolve_schema(property_schema)?;
        if should_descend(&resolved) {
            let entry = roots
                .entry(name.clone())
                .or_insert_with(|| RootBuilder::new(name, &resolved));
            let section = build_section_tree(&resolver, &resolved, path, None, &mut order_counter)?;
            entry.sections.push(section);
        } else {
            let field = build_field_schema(
                &resolver,
                &resolved,
                name,
                vec![name.clone()],
                general_section_info(),
                required.contains(name),
            )?;
            general_fields.push((order_counter, field));
            order_counter += 1;
        }
    }

    if let Some(additional) = object.additional_properties.as_ref() {
        let resolved = resolver.resolve_schema(additional)?;
        let field = build_field_schema(
            &resolver,
            &resolved,
            "additional",
            Vec::new(),
            general_section_info(),
            false,
        )?;
        general_fields.push((order_counter, field));
    }

    general_fields.sort_by_key(|(order, _)| *order);

    let mut roots_out = Vec::new();
    if !general_fields.is_empty() {
        let fields = general_fields.into_iter().map(|(_, field)| field).collect();
        roots_out.push(RootSection {
            id: "general".to_string(),
            title: "General".to_string(),
            description: None,
            sections: vec![FormSection {
                id: "general".to_string(),
                title: "General".to_string(),
                description: None,
                path: Vec::new(),
                fields,
                children: Vec::new(),
            }],
        });
    }

    for (_, builder) in roots {
        if !builder.sections.is_empty() {
            roots_out.push(builder.into_root());
        }
    }

    if roots_out.is_empty() {
        roots_out.push(RootSection {
            id: "general".to_string(),
            title: "General".to_string(),
            description: None,
            sections: vec![FormSection {
                id: "general".to_string(),
                title: "General".to_string(),
                description: None,
                path: Vec::new(),
                fields: Vec::new(),
                children: Vec::new(),
            }],
        });
    }

    Ok(FormSchema {
        title: root_object.metadata.as_ref().and_then(|m| m.title.clone()),
        description: root_object
            .metadata
            .as_ref()
            .and_then(|m| m.description.clone()),
        roots: roots_out,
    })
}

fn build_section_tree(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    path: Vec<String>,
    parent_section: Option<&SectionInfo>,
    order: &mut usize,
) -> Result<FormSection> {
    let name = path
        .last()
        .cloned()
        .unwrap_or_else(|| "section".to_string());
    let section_info = section_info_for_object(schema, &name, parent_section);
    let object = schema
        .object
        .as_ref()
        .context("object schema must define properties")?;
    let required = required_set(object);

    let mut fields: Vec<(usize, FieldSchema)> = Vec::new();
    let mut children = Vec::new();

    for (child_name, child_schema) in &object.properties {
        let mut next_path = path.clone();
        next_path.push(child_name.clone());
        let resolved = resolver.resolve_schema(child_schema)?;
        if should_descend(&resolved) {
            let child =
                build_section_tree(resolver, &resolved, next_path, Some(&section_info), order)?;
            children.push(child);
        } else {
            let field = build_field_schema(
                resolver,
                &resolved,
                child_name,
                next_path,
                section_info.clone(),
                required.contains(child_name),
            )?;
            fields.push((*order, field));
            *order += 1;
        }
    }

    if let Some(additional) = object.additional_properties.as_ref() {
        let resolved = resolver.resolve_schema(additional)?;
        let field_name = path
            .last()
            .cloned()
            .unwrap_or_else(|| "additional".to_string());
        let field = build_field_schema(
            resolver,
            &resolved,
            &field_name,
            path.clone(),
            section_info.clone(),
            false,
        )?;
        fields.push((*order, field));
        *order += 1;
    }

    fields.sort_by_key(|(pos, _)| *pos);

    Ok(FormSection {
        id: section_info.id,
        title: section_info.title,
        description: section_info.description,
        path,
        fields: fields.into_iter().map(|(_, field)| field).collect(),
        children,
    })
}

fn should_descend(schema: &SchemaObject) -> bool {
    is_object_schema(schema)
        && schema
            .object
            .as_ref()
            .map(|obj| !obj.properties.is_empty())
            .unwrap_or(false)
        && !has_composite_subschemas(schema)
}

fn build_field_schema(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    name: &str,
    path: Vec<String>,
    section: SectionInfo,
    required: bool,
) -> Result<FieldSchema> {
    let metadata = metadata_map(schema);
    let kind = detect_kind(resolver, schema)
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

fn detect_kind(resolver: &SchemaResolver<'_>, schema: &SchemaObject) -> Result<FieldKind> {
    if let Some(key_value) = key_value_field(resolver, schema)? {
        return Ok(FieldKind::KeyValue(Box::new(key_value)));
    }
    if let Some(composite) = composite_field(resolver, schema)? {
        return Ok(FieldKind::Composite(Box::new(composite)));
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
            let inner = resolve_array_items(resolver, array)?;
            let inner_kind = detect_kind(resolver, &inner)?;
            match inner_kind {
                FieldKind::String
                | FieldKind::Integer
                | FieldKind::Number
                | FieldKind::Boolean
                | FieldKind::Enum(_)
                | FieldKind::Composite(_) => Ok(FieldKind::Array(Box::new(inner_kind))),
                FieldKind::Json => {
                    if let Some(composite) = inline_object_composite(&inner)? {
                        Ok(FieldKind::Array(Box::new(FieldKind::Composite(Box::new(
                            composite,
                        )))))
                    } else {
                        Ok(FieldKind::Array(Box::new(FieldKind::Json)))
                    }
                }
                FieldKind::KeyValue(_) => bail!("arrays of key/value maps are not supported"),
                FieldKind::Array(_) => bail!("nested arrays are not supported"),
            }
        }
        Some(other) => bail!("unsupported field type {other:?}"),
    }
}

fn key_value_field(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
) -> Result<Option<KeyValueField>> {
    let Some(object) = schema.object.as_ref() else {
        return Ok(None);
    };
    if !object.properties.is_empty() {
        return Ok(None);
    }

    if let Some(additional) = object.additional_properties.as_ref() {
        return build_key_value_from_schema(resolver, schema, additional, None);
    }

    if let Some((pattern, pattern_schema)) = object.pattern_properties.iter().next() {
        let key_schema = serde_json::json!({
            "type": "string",
            "pattern": pattern,
            "title": "Key",
        });
        return build_key_value_from_schema(resolver, schema, pattern_schema, Some(key_schema));
    }

    Ok(None)
}

fn build_key_value_from_schema(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    value_schema: &Schema,
    key_override: Option<Value>,
) -> Result<Option<KeyValueField>> {
    let object = schema.object.as_ref().expect("object schema");
    let value_resolved = resolver.resolve_schema(value_schema)?;
    let value_kind = detect_kind(resolver, &value_resolved)?;
    let value_schema =
        schema_object_to_value(&value_resolved).context("failed to serialize value schema")?;
    let (value_title, value_description, value_default) = schema_titles(&value_resolved, "Value");

    let (key_schema_value, key_title, key_description, key_default) =
        if let Some(override_schema) = key_override {
            (override_schema, "Key".to_string(), None, None)
        } else if let Some(names) = object.property_names.as_ref() {
            let resolved = resolver.resolve_schema(names)?;
            let serialized = schema_object_to_value(&resolved)
                .context("failed to serialize propertyNames schema")?;
            let (title, description, default) = schema_titles(&resolved, "Key");
            (serialized, title, description, default)
        } else {
            (
                serde_json::json!({"type": "string", "title": "Key"}),
                "Key".to_string(),
                None,
                None,
            )
        };

    let entry_schema = key_value_entry_schema(&key_schema_value, &value_schema);

    Ok(Some(KeyValueField {
        key_title,
        key_description,
        key_default,
        key_schema: key_schema_value,
        value_title,
        value_description,
        value_default,
        value_schema,
        value_kind: Box::new(value_kind),
        entry_schema,
    }))
}

fn composite_field(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
) -> Result<Option<CompositeField>> {
    let Some(subschemas) = schema.subschemas.as_ref() else {
        return Ok(None);
    };
    if let Some(one_of) = subschemas.one_of.as_ref() {
        return build_composite(resolver, CompositeMode::OneOf, one_of);
    }
    if let Some(any_of) = subschemas.any_of.as_ref() {
        return build_composite(resolver, CompositeMode::AnyOf, any_of);
    }
    Ok(None)
}

fn build_composite(
    resolver: &SchemaResolver<'_>,
    mode: CompositeMode,
    schemas: &[Schema],
) -> Result<Option<CompositeField>> {
    if schemas.is_empty() {
        return Ok(None);
    }

    let mut variants = Vec::new();
    for (index, variant) in schemas.iter().enumerate() {
        let resolved = resolver.resolve_schema(variant)?;
        ensure_object_schema(&resolved)?;
        let mut schema_value = serde_json::to_value(Schema::Object(resolved.clone()))
            .context("failed to serialize composite variant schema")?;
        if let Some(definitions) = resolver.definitions_snapshot()
            && let Value::Object(ref mut map) = schema_value
        {
            map.entry("definitions".to_string()).or_insert(definitions);
        }
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
    resolver: &SchemaResolver<'_>,
    array: &ArrayValidation,
) -> Result<SchemaObject> {
    let items = array
        .items
        .as_ref()
        .context("array schema must define items")?;
    match items {
        SingleOrVec::Single(schema) => resolver.resolve_schema(schema),
        SingleOrVec::Vec(list) => match list.first() {
            Some(first) => resolver.resolve_schema(first),
            None => bail!("tuple arrays without items are not supported"),
        },
    }
}

fn schema_object_to_value(schema: &SchemaObject) -> Result<Value> {
    serde_json::to_value(Schema::Object(schema.clone()))
        .context("failed to serialize schema object")
}

fn schema_titles(schema: &SchemaObject, fallback: &str) -> (String, Option<String>, Option<Value>) {
    let title = schema
        .metadata
        .as_ref()
        .and_then(|m| m.title.clone())
        .unwrap_or_else(|| fallback.to_string());
    let description = schema.metadata.as_ref().and_then(|m| m.description.clone());
    let default = schema.metadata.as_ref().and_then(|m| m.default.clone());
    (title, description, default)
}

fn key_value_entry_schema(key_schema: &Value, value_schema: &Value) -> Value {
    serde_json::json!({
        "type": "object",
        "required": ["key", "value"],
        "properties": {
            "key": key_schema,
            "value": value_schema,
        }
    })
}

fn inline_object_composite(schema: &SchemaObject) -> Result<Option<CompositeField>> {
    if !is_object_schema(schema) {
        return Ok(None);
    }
    let schema_value = schema_object_to_value(schema)?;
    let title = schema
        .metadata
        .as_ref()
        .and_then(|m| m.title.clone())
        .unwrap_or_else(|| "Entry".to_string());
    let description = schema.metadata.as_ref().and_then(|m| m.description.clone());
    let variant = CompositeVariant {
        id: "variant_0".to_string(),
        title,
        description,
        schema: schema_value,
    };
    Ok(Some(CompositeField {
        mode: CompositeMode::OneOf,
        variants: vec![variant],
    }))
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

fn is_object_schema(schema: &SchemaObject) -> bool {
    match instance_type(schema) {
        Some(InstanceType::Object) => true,
        None => schema.object.is_some(),
        _ => false,
    }
}

fn instance_type(schema: &SchemaObject) -> Option<InstanceType> {
    schema.instance_type.as_ref().and_then(|kind| match kind {
        SingleOrVec::Single(single) => Some(**single),
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn builds_nested_sections_and_general_root() {
        let schema = json!({
            "type": "object",
            "definitions": {
                "duration": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "integer"}
                    }
                }
            },
            "properties": {
                "metadata": {
                    "type": "object",
                    "properties": {
                        "serviceName": {"type": "string"}
                    }
                },
                "runtime": {
                    "type": "object",
                    "properties": {
                        "http": {
                            "type": "object",
                            "properties": {
                                "port": {"type": "integer"},
                                "limits": {
                                    "type": "object",
                                    "properties": {
                                        "timeout": {"$ref": "#/definitions/duration"}
                                    }
                                }
                            }
                        }
                    }
                },
                "generalFlag": {"type": "string"}
            }
        });

        let form = build_form_schema(&schema).expect("schema parsed");
        assert_eq!(form.roots.len(), 3);
        let runtime = form
            .roots
            .iter()
            .find(|root| root.id == "runtime")
            .expect("runtime root");
        assert_eq!(runtime.sections.len(), 1);
        let http = &runtime.sections[0];
        assert_eq!(http.children.len(), 1);
        let http_child = &http.children[0];
        assert_eq!(http_child.fields.len(), 1);
    }

    #[test]
    fn composite_variants_keep_definitions() {
        let schema = json!({
            "type": "object",
            "definitions": {
                "endpoint": {
                    "type": "object",
                    "properties": {
                        "url": {"type": "string"}
                    }
                }
            },
            "properties": {
                "notifications": {
                    "type": "object",
                    "properties": {
                        "channel": {
                            "oneOf": [
                                {
                                    "title": "HTTP",
                                    "properties": {
                                        "type": {"const": "http"},
                                        "target": {"$ref": "#/definitions/endpoint"}
                                    },
                                    "required": ["type", "target"]
                                }
                            ]
                        }
                    }
                }
            }
        });

        let form = build_form_schema(&schema).expect("schema parsed");
        let notifications = form
            .roots
            .iter()
            .find(|root| root.id == "notifications")
            .expect("notifications root");
        let section = notifications.sections.first().expect("section");
        let channel = section
            .fields
            .iter()
            .find(|field| field.name == "channel")
            .expect("channel field");
        match &channel.kind {
            FieldKind::Composite(composite) => {
                assert!(
                    composite
                        .variants
                        .first()
                        .and_then(|variant| variant.schema.get("definitions"))
                        .is_some(),
                    "variant schema should embed definitions"
                );
            }
            other => panic!("expected composite field, got {:?}", other),
        }
    }

    #[test]
    fn pattern_properties_become_key_value_fields() {
        let schema = json!({
            "type": "object",
            "properties": {
                "labels": {
                    "type": "object",
                    "patternProperties": {
                        "^[a-z]+$": {"type": "string"}
                    },
                    "additionalProperties": false
                }
            }
        });
        let form = build_form_schema(&schema).expect("schema parsed");
        let field = find_field(&form, |field| field.name == "labels").expect("labels field");
        assert!(matches!(field.kind, FieldKind::KeyValue(_)));
    }

    #[test]
    fn multi_level_refs_are_resolved() {
        let schema = json!({
            "type": "object",
            "definitions": {
                "duration": {
                    "type": "object",
                    "properties": {
                        "value": {"type": "integer"}
                    }
                },
                "timeout": {
                    "$ref": "#/definitions/duration"
                }
            },
            "properties": {
                "runtime": {
                    "type": "object",
                    "properties": {
                        "limits": {
                            "type": "object",
                            "properties": {
                                "requestTimeout": {"$ref": "#/definitions/timeout"}
                            }
                        }
                    }
                }
            }
        });
        let form = build_form_schema(&schema).expect("schema parsed");
        let field = find_field(&form, |field| {
            field
                .pointer
                .ends_with("/runtime/limits/requestTimeout/value")
        })
        .expect("requestTimeout value field");
        assert_eq!(field.name, "value");
    }

    fn find_field<'a>(
        form: &'a FormSchema,
        predicate: impl Fn(&FieldSchema) -> bool,
    ) -> Option<&'a FieldSchema> {
        for root in &form.roots {
            if let Some(field) = find_in_sections(&root.sections, &predicate) {
                return Some(field);
            }
        }
        None
    }

    fn find_in_sections<'a>(
        sections: &'a [FormSection],
        predicate: &impl Fn(&FieldSchema) -> bool,
    ) -> Option<&'a FieldSchema> {
        for section in sections {
            for field in &section.fields {
                if predicate(field) {
                    return Some(field);
                }
            }
            if let Some(found) = find_in_sections(&section.children, predicate) {
                return Some(found);
            }
        }
        None
    }
}
