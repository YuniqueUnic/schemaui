#![allow(dead_code)]

use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec,
};
use serde_json::Value;

use crate::schema::{
    loader::load_root_schema,
    metadata::{
        SectionInfo, general_section_info, metadata_map, prettify_label, section_info_for_object,
    },
    resolver::SchemaResolver,
};

use crate::tui::model::form_schema::{
    CompositeField, CompositeMode, CompositeVariant, FieldKind, FieldSchema, FormSchema,
    FormSection, KeyValueField, RootSection,
};

use super::types::RootBuilder;

use super::helpers::{
    describe_schema_shape, ensure_object_schema, has_composite_subschemas, instance_type,
    is_object_schema, key_value_entry_schema, required_set, schema_object_to_value, schema_titles,
    to_pointer,
};

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
        let normalized = normalize_schema(&resolver, &resolved)?;
        if should_descend(&normalized) {
            let entry = roots
                .entry(name.clone())
                .or_insert_with(|| RootBuilder::new(name, &normalized));
            let section =
                build_section_tree(&resolver, &normalized, path, None, &mut order_counter)?;
            entry.sections.push(section);
        } else {
            let field = build_field_schema(
                &resolver,
                &normalized,
                name,
                vec![name.clone()],
                general_section_info(),
                required.contains(name),
            )?;
            general_fields.push((order_counter, field));
            order_counter += 1;
        }
    }

    if let Some(additional) = object.additional_properties.as_ref()
        && let Some(resolved) = resolve_additional_properties(&resolver, additional)?
    {
        let normalized = normalize_schema(&resolver, &resolved)?;
        let field = build_field_schema(
            &resolver,
            &normalized,
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

fn normalize_schema(resolver: &SchemaResolver<'_>, schema: &SchemaObject) -> Result<SchemaObject> {
    if has_all_of(schema) {
        merge_all_of_schema(resolver, schema)
    } else {
        Ok(schema.clone())
    }
}

fn has_all_of(schema: &SchemaObject) -> bool {
    schema
        .subschemas
        .as_ref()
        .and_then(|subs| subs.all_of.as_ref())
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

fn merge_all_of_schema(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
) -> Result<SchemaObject> {
    let Some(subs) = schema
        .subschemas
        .as_ref()
        .and_then(|validation| validation.all_of.as_ref())
        .filter(|items| !items.is_empty())
    else {
        return Ok(schema.clone());
    };

    let mut merged = schema.clone();
    let mut object = merged
        .object
        .take()
        .unwrap_or_else(|| Box::new(ObjectValidation::default()));
    let mut contributed = false;

    for part in subs {
        let resolved = resolver.resolve_schema(part)?;
        let normalized = normalize_schema(resolver, &resolved)?;
        if let Some(source) = normalized.object.as_ref() {
            merge_object_validation(&mut object, source);
            contributed = true;
        }
    }

    if !contributed {
        return Ok(schema.clone());
    }

    merged.object = Some(object);
    if let Some(mut validation) = merged.subschemas.take() {
        validation.all_of = None;
        merged.subschemas = Some(validation);
    }
    Ok(merged)
}

fn merge_object_validation(target: &mut ObjectValidation, source: &ObjectValidation) {
    for (key, schema) in &source.properties {
        target
            .properties
            .entry(key.clone())
            .or_insert(schema.clone());
    }
    for (key, schema) in &source.pattern_properties {
        target
            .pattern_properties
            .entry(key.clone())
            .or_insert(schema.clone());
    }
    for required in &source.required {
        target.required.insert(required.clone());
    }
    if target.additional_properties.is_none() {
        target.additional_properties = source.additional_properties.clone();
    }
    if target.property_names.is_none() && source.property_names.is_some() {
        target.property_names = source.property_names.clone();
    }
    if target.max_properties.is_none() {
        target.max_properties = source.max_properties;
    }
    if target.min_properties.is_none() {
        target.min_properties = source.min_properties;
    }
    // ObjectValidation does not track dependencies in this schema version.
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
        let normalized = normalize_schema(resolver, &resolved)?;
        if should_descend(&normalized) {
            let child =
                build_section_tree(resolver, &normalized, next_path, Some(&section_info), order)?;
            children.push(child);
        } else {
            let field = build_field_schema(
                resolver,
                &normalized,
                child_name,
                next_path,
                section_info.clone(),
                required.contains(child_name),
            )?;
            fields.push((*order, field));
            *order += 1;
        }
    }

    if let Some(additional) = object.additional_properties.as_ref()
        && let Some(resolved) = resolve_additional_properties(resolver, additional)?
    {
        let normalized = normalize_schema(resolver, &resolved)?;
        let field_name = path
            .last()
            .cloned()
            .unwrap_or_else(|| "additional".to_string());
        let field = build_field_schema(
            resolver,
            &normalized,
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

fn resolve_additional_properties(
    resolver: &SchemaResolver<'_>,
    schema: &Schema,
) -> Result<Option<SchemaObject>> {
    match schema {
        Schema::Bool(false) => Ok(None),
        Schema::Bool(true) => Ok(None),
        other => {
            let resolved = resolver.resolve_schema(other)?;
            normalize_schema(resolver, &resolved).map(Some)
        }
    }
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
    _section: SectionInfo,
    required: bool,
) -> Result<FieldSchema> {
    let normalized = normalize_schema(resolver, schema)?;
    let metadata = metadata_map(&normalized);
    let kind = detect_kind(resolver, &normalized)
        .with_context(|| format!("unsupported schema for field '{name}'"))?;
    let title = normalized
        .metadata
        .as_ref()
        .and_then(|m| m.title.clone())
        .unwrap_or_else(|| prettify_label(name));
    let default = normalized.metadata.as_ref().and_then(|m| m.default.clone());
    let description = normalized
        .metadata
        .as_ref()
        .and_then(|m| m.description.clone());

    Ok(FieldSchema {
        name: name.to_string(),
        path: path.clone(),
        pointer: to_pointer(&path),
        title,
        description,
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
        let labels = options.iter().map(enum_label).collect::<Vec<_>>();
        return Ok(FieldKind::Enum {
            labels,
            values: options.clone(),
        });
    }

    match instance_type(schema) {
        Some(InstanceType::String) | None => Ok(FieldKind::String),
        Some(InstanceType::Integer) => Ok(FieldKind::Integer),
        Some(InstanceType::Number) => Ok(FieldKind::Number),
        Some(InstanceType::Boolean) => Ok(FieldKind::Boolean),
        Some(InstanceType::Object) => Ok(FieldKind::Json),
        Some(InstanceType::Array) => match schema.array.as_ref() {
            Some(array) if array.items.is_some() => {
                let inner = resolve_array_items(resolver, array)?;
                let inner_kind = detect_kind(resolver, &inner)?;
                match inner_kind {
                    FieldKind::String
                    | FieldKind::Integer
                    | FieldKind::Number
                    | FieldKind::Boolean
                    | FieldKind::Enum { .. }
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
            _ => Ok(FieldKind::Array(Box::new(FieldKind::Json))),
        },
        Some(other) => bail!("unsupported field type {other:?}"),
    }
}

fn enum_label(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Number(num) => num.to_string(),
        Value::Bool(flag) => flag.to_string(),
        Value::Array(items) => items.iter().map(enum_label).collect::<Vec<_>>().join(", "),
        other => other.to_string(),
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
        let normalized = normalize_schema(resolver, &resolved)?;
        let is_object = is_object_schema(&normalized);
        let mut schema_value = schema_object_to_value(&normalized)
            .context("failed to serialize composite variant schema")?;
        if let Some(definitions) = resolver.definitions_snapshot()
            && let Value::Object(ref mut map) = schema_value
        {
            map.entry("definitions".to_string()).or_insert(definitions);
        }
        let title = normalized
            .metadata
            .as_ref()
            .and_then(|m| m.title.clone())
            .unwrap_or_else(|| default_variant_title(index, &normalized));
        let description = normalized
            .metadata
            .as_ref()
            .and_then(|m| m.description.clone());
        variants.push(CompositeVariant {
            id: format!("variant_{}", index),
            title,
            description,
            schema: schema_value,
            is_object,
        });
    }

    Ok(Some(CompositeField { mode, variants }))
}

fn default_variant_title(index: usize, schema: &SchemaObject) -> String {
    let mut shape = describe_schema_shape(schema);
    if shape.is_empty()
        && let Some(reference) = schema.reference.as_ref()
    {
        shape = reference.trim_start_matches("#/$defs/").to_string();
    }
    if shape.is_empty() {
        format!("Variant {}", index + 1)
    } else {
        shape
    }
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
        SingleOrVec::Single(schema) => {
            let resolved = resolver.resolve_schema(schema)?;
            normalize_schema(resolver, &resolved)
        }
        SingleOrVec::Vec(list) => match list.first() {
            Some(first) => {
                let resolved = resolver.resolve_schema(first)?;
                normalize_schema(resolver, &resolved)
            }
            None => bail!("tuple arrays without items are not supported"),
        },
    }
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
        is_object: true,
    };
    Ok(Some(CompositeField {
        mode: CompositeMode::OneOf,
        variants: vec![variant],
    }))
}
