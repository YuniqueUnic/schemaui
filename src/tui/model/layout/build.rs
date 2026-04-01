#![allow(dead_code)]

use anyhow::{Context, Result, anyhow, bail};
use indexmap::IndexMap;
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec,
};
use serde_json::Value;

use crate::schema::{
    loader::load_root_schema,
    metadata::{SectionInfo, metadata_map, prettify_label, section_info_for_object},
    resolver::{SchemaResolver, schema_reference},
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

enum BuiltProperty {
    Section {
        section: FormSection,
        schema: SchemaObject,
    },
    Field(FieldSchema),
}

struct PropertyContext<'a> {
    name: &'a str,
    path: Vec<String>,
    required: bool,
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
    let mut active_refs = Vec::new();
    let object = root_object
        .object
        .as_ref()
        .context("root schema must define properties")?;
    let required = required_set(object);

    for (name, property_schema) in &object.properties {
        let path = vec![name.clone()];
        match build_property(
            &resolver,
            property_schema,
            PropertyContext {
                name,
                path,
                required: required.contains(name),
            },
            None,
            &mut order_counter,
            &mut active_refs,
        )? {
            BuiltProperty::Section { section, schema } => {
                let entry = roots
                    .entry(name.clone())
                    .or_insert_with(|| RootBuilder::new(name, &schema));
                entry.sections.push(section);
            }
            BuiltProperty::Field(field) => {
                general_fields.push((order_counter, field));
                order_counter += 1;
            }
        }
    }

    if let Some(additional) = object.additional_properties.as_ref()
        && !matches!(&**additional, Schema::Bool(false) | Schema::Bool(true))
    {
        let field = build_field_from_schema_entry(
            &resolver,
            additional,
            "additional",
            Vec::new(),
            false,
            &mut active_refs,
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

fn build_property(
    resolver: &SchemaResolver<'_>,
    schema: &Schema,
    property: PropertyContext<'_>,
    parent_section: Option<&SectionInfo>,
    order: &mut usize,
    active_refs: &mut Vec<String>,
) -> Result<BuiltProperty> {
    let PropertyContext {
        name,
        path,
        required,
    } = property;
    with_resolved_schema(
        resolver,
        schema,
        active_refs,
        {
            let path = path.clone();
            move |normalized| {
                Ok(BuiltProperty::Field(field_schema_from_kind(
                    &normalized,
                    name,
                    path,
                    recursive_boundary_field_kind(&normalized),
                    required,
                )))
            }
        },
        move |normalized, active_refs| {
            if should_descend(&normalized) {
                let section = build_section_tree(
                    resolver,
                    &normalized,
                    path,
                    parent_section,
                    order,
                    active_refs,
                )?;
                Ok(BuiltProperty::Section {
                    section,
                    schema: normalized,
                })
            } else {
                build_field_schema(resolver, &normalized, name, path, required, active_refs)
                    .map(BuiltProperty::Field)
            }
        },
    )
}

fn build_field_from_schema_entry(
    resolver: &SchemaResolver<'_>,
    schema: &Schema,
    name: &str,
    path: Vec<String>,
    required: bool,
    active_refs: &mut Vec<String>,
) -> Result<FieldSchema> {
    let recursive_path = path.clone();
    with_resolved_schema(
        resolver,
        schema,
        active_refs,
        move |normalized| {
            Ok(field_schema_from_kind(
                &normalized,
                name,
                recursive_path,
                recursive_boundary_field_kind(&normalized),
                required,
            ))
        },
        move |normalized, active_refs| {
            build_field_schema(resolver, &normalized, name, path, required, active_refs)
        },
    )
}

fn build_section_tree(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    path: Vec<String>,
    parent_section: Option<&SectionInfo>,
    order: &mut usize,
    active_refs: &mut Vec<String>,
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
        match build_property(
            resolver,
            child_schema,
            PropertyContext {
                name: child_name,
                path: next_path,
                required: required.contains(child_name),
            },
            Some(&section_info),
            order,
            active_refs,
        )? {
            BuiltProperty::Section { section, .. } => children.push(section),
            BuiltProperty::Field(field) => {
                fields.push((*order, field));
                *order += 1;
            }
        }
    }

    if let Some(additional) = object.additional_properties.as_ref()
        && !matches!(&**additional, Schema::Bool(false) | Schema::Bool(true))
    {
        let field_name = path
            .last()
            .cloned()
            .unwrap_or_else(|| "additional".to_string());
        let field = build_field_from_schema_entry(
            resolver,
            additional,
            &field_name,
            path.clone(),
            false,
            active_refs,
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
    required: bool,
    active_refs: &mut Vec<String>,
) -> Result<FieldSchema> {
    let normalized = normalize_schema(resolver, schema)?;
    let kind = detect_kind(resolver, &normalized, active_refs)
        .with_context(|| format!("unsupported schema for field '{name}'"))?;
    Ok(field_schema_from_kind(
        &normalized,
        name,
        path,
        kind,
        required,
    ))
}

fn field_schema_from_kind(
    schema: &SchemaObject,
    name: &str,
    path: Vec<String>,
    kind: FieldKind,
    required: bool,
) -> FieldSchema {
    let metadata = metadata_map(schema);
    let title = schema
        .metadata
        .as_ref()
        .and_then(|m| m.title.clone())
        .unwrap_or_else(|| prettify_label(name));
    let default = schema.metadata.as_ref().and_then(|m| m.default.clone());
    let description = schema.metadata.as_ref().and_then(|m| m.description.clone());

    FieldSchema {
        name: name.to_string(),
        path: path.clone(),
        pointer: to_pointer(&path),
        title,
        description,
        kind,
        required,
        default,
        metadata,
    }
}

fn recursive_boundary_field_kind(schema: &SchemaObject) -> FieldKind {
    if let Some(options) = &schema.enum_values {
        return FieldKind::Enum {
            labels: options.iter().map(enum_label).collect(),
            values: options.clone(),
        };
    }

    match instance_type(schema) {
        Some(InstanceType::String) | None => FieldKind::String,
        Some(InstanceType::Integer) => FieldKind::Integer,
        Some(InstanceType::Number) => FieldKind::Number,
        Some(InstanceType::Boolean) => FieldKind::Boolean,
        Some(InstanceType::Array) => FieldKind::Array(Box::new(FieldKind::Json)),
        Some(InstanceType::Object | InstanceType::Null) => FieldKind::Json,
    }
}

fn detect_kind(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    active_refs: &mut Vec<String>,
) -> Result<FieldKind> {
    if let Some(key_value) = key_value_field(resolver, schema, active_refs)? {
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
                let item_schema = array_item_schema(array)?;
                let recursive_item_ref = schema_reference(item_schema)
                    .is_some_and(|reference| active_refs.iter().any(|active| active == reference));
                let inner_kind = with_resolved_schema(
                    resolver,
                    item_schema,
                    active_refs,
                    |normalized| Ok(recursive_boundary_field_kind(&normalized)),
                    |normalized, active_refs| detect_kind(resolver, &normalized, active_refs),
                )?;
                match inner_kind {
                    FieldKind::String
                    | FieldKind::Integer
                    | FieldKind::Number
                    | FieldKind::Boolean
                    | FieldKind::Enum { .. }
                    | FieldKind::Composite(_) => Ok(FieldKind::Array(Box::new(inner_kind))),
                    FieldKind::Json => {
                        if recursive_item_ref {
                            return Ok(FieldKind::Array(Box::new(FieldKind::Json)));
                        }
                        let inner = with_resolved_schema(
                            resolver,
                            item_schema,
                            active_refs,
                            Ok,
                            |normalized, _| Ok(normalized),
                        )?;
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
    active_refs: &mut Vec<String>,
) -> Result<Option<KeyValueField>> {
    let Some(object) = schema.object.as_ref() else {
        return Ok(None);
    };
    if !object.properties.is_empty() {
        return Ok(None);
    }

    if let Some(additional) = object.additional_properties.as_ref() {
        return build_key_value_from_schema(resolver, schema, additional, None, active_refs);
    }

    if let Some((pattern, pattern_schema)) = object.pattern_properties.iter().next() {
        let key_schema = serde_json::json!({
            "type": "string",
            "pattern": pattern,
            "title": "Key",
        });
        return build_key_value_from_schema(
            resolver,
            schema,
            pattern_schema,
            Some(key_schema),
            active_refs,
        );
    }

    Ok(None)
}

fn build_key_value_from_schema(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    value_schema: &Schema,
    key_override: Option<Value>,
    active_refs: &mut Vec<String>,
) -> Result<Option<KeyValueField>> {
    let object = schema.object.as_ref().expect("object schema");
    let (value_resolved, value_kind) = with_resolved_schema(
        resolver,
        value_schema,
        active_refs,
        |normalized| {
            Ok((
                normalized.clone(),
                recursive_boundary_field_kind(&normalized),
            ))
        },
        |normalized, active_refs| {
            let kind = detect_kind(resolver, &normalized, active_refs)?;
            Ok((normalized, kind))
        },
    )?;
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

fn array_item_schema(array: &ArrayValidation) -> Result<&Schema> {
    let items = array
        .items
        .as_ref()
        .context("array schema must define items")?;
    match items {
        SingleOrVec::Single(schema) => Ok(schema.as_ref()),
        SingleOrVec::Vec(list) => list
            .first()
            .context("tuple arrays without items are not supported"),
    }
}

fn with_resolved_schema<T, F, R>(
    resolver: &SchemaResolver<'_>,
    schema: &Schema,
    active_refs: &mut Vec<String>,
    on_recursive: R,
    on_resolved: F,
) -> Result<T>
where
    F: FnOnce(SchemaObject, &mut Vec<String>) -> Result<T>,
    R: FnOnce(SchemaObject) -> Result<T>,
{
    let resolved = resolver.resolve_schema(schema)?;
    let normalized = normalize_schema(resolver, &resolved)?;
    if let Some(reference) = schema_reference(schema) {
        if active_refs.iter().any(|active| active == reference) {
            return on_recursive(normalized);
        }
        active_refs.push(reference.to_string());
        let result = on_resolved(normalized, active_refs);
        active_refs.pop();
        result
    } else {
        on_resolved(normalized, active_refs)
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
