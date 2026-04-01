use anyhow::{Context, Result, anyhow, bail};
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec,
};
use serde_json::{Map, Value};

use crate::schema::{
    loader::load_root_schema,
    resolver::{SchemaResolver, schema_reference},
};

use super::types::{
    CompositeMode, ScalarKind, UiAst, UiKeyValueNode, UiNode, UiNodeKind, UiVariant,
};

pub fn build_ui_ast(raw: &Value) -> Result<UiAst> {
    let root_schema = load_root_schema(raw)?;
    let resolver = SchemaResolver::new(raw, &root_schema);
    let root_object = resolver
        .root_object()
        .cloned()
        .ok_or_else(|| anyhow!("root schema must be an object"))?;

    if !is_object_schema(&root_object) {
        bail!("root schema must describe an object");
    }

    let object = root_object
        .object
        .as_ref()
        .context("root schema must define properties")?;
    let required = required_list(object);

    let mut active_refs = Vec::new();
    let mut roots = Vec::new();
    for (name, schema) in &object.properties {
        let pointer = append_pointer("", name);
        let node = visit_schema_entry(
            &resolver,
            schema,
            pointer,
            required.contains(name),
            &mut active_refs,
        )?;
        roots.push(node);
    }

    Ok(UiAst { roots })
}

fn visit_schema_entry(
    resolver: &SchemaResolver<'_>,
    schema: &Schema,
    pointer: String,
    required: bool,
    active_refs: &mut Vec<String>,
) -> Result<UiNode> {
    let recursive_pointer = pointer.clone();
    with_resolved_schema(
        resolver,
        schema,
        active_refs,
        move |resolved| {
            Ok(recursive_boundary_node(
                &resolved,
                recursive_pointer,
                required,
            ))
        },
        move |resolved, active_refs| {
            visit_schema(resolver, &resolved, pointer, required, active_refs)
        },
    )
}

fn visit_schema(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    pointer: String,
    required: bool,
    active_refs: &mut Vec<String>,
) -> Result<UiNode> {
    if let Some(subs) = schema.subschemas.as_ref()
        && let Some(all_of) = subs.all_of.as_ref()
        && !all_of.is_empty()
    {
        let merged = merge_all_of(resolver, all_of)?;
        return visit_schema(resolver, &merged, pointer, required, active_refs);
    }

    if let Some(subs) = schema.subschemas.as_ref() {
        if let Some(one_of) = subs.one_of.as_ref() {
            return build_composite_node(
                resolver,
                one_of,
                CompositeMode::OneOf,
                schema,
                pointer,
                required,
                active_refs,
            );
        }
        if let Some(any_of) = subs.any_of.as_ref() {
            return build_composite_node(
                resolver,
                any_of,
                CompositeMode::AnyOf,
                schema,
                pointer,
                required,
                active_refs,
            );
        }
    }

    if let Some(template) = build_key_value_template(resolver, schema, active_refs)? {
        return Ok(UiNode {
            pointer,
            title: schema_title(schema),
            description: schema_description(schema),
            required,
            default_value: schema_default_or_const(schema),
            kind: UiNodeKind::KeyValue {
                template: Box::new(template),
            },
        });
    }

    if is_array_schema(schema) {
        let array = schema.array.as_ref();
        let item_node = match array {
            Some(array) if array.items.is_some() => {
                visit_array_item_kind(resolver, array, active_refs)?
            }
            _ => array_boundary_item_kind(),
        };
        let default_value =
            schema_default_or_const(schema).or_else(|| Some(Value::Array(Vec::new())));
        return Ok(UiNode {
            pointer,
            title: schema_title(schema),
            description: schema_description(schema),
            required,
            default_value,
            kind: UiNodeKind::Array {
                item: Box::new(item_node),
                min_items: array.and_then(|inner| inner.min_items).map(|v| v as u64),
                max_items: array.and_then(|inner| inner.max_items).map(|v| v as u64),
            },
        });
    }

    if is_object_schema(schema) {
        let obj = schema
            .object
            .as_ref()
            .context("object schema missing properties")?;
        let mut children = Vec::new();
        let required_fields = required_list(obj);
        for (name, child_schema) in &obj.properties {
            let child_ptr = append_pointer(&pointer, name);
            let child = visit_schema_entry(
                resolver,
                child_schema,
                child_ptr,
                required_fields.contains(name),
                active_refs,
            )?;
            children.push(child);
        }
        let default_value = schema_default_or_const(schema).or(Some(Value::Object(Map::new())));
        return Ok(UiNode {
            pointer,
            title: schema_title(schema),
            description: schema_description(schema),
            required,
            default_value,
            kind: UiNodeKind::Object {
                children,
                required: required_fields,
            },
        });
    }

    let (scalar, enum_options, enum_values) = detect_scalar(schema)?;
    let default_value = schema_default_or_const(schema)
        .or_else(|| infer_default_scalar(scalar, enum_values.as_ref()));
    Ok(UiNode {
        pointer,
        title: schema_title(schema),
        description: schema_description(schema),
        required,
        default_value,
        kind: UiNodeKind::Field {
            scalar,
            enum_options,
            enum_values,
        },
    })
}

fn build_composite_kind(
    resolver: &SchemaResolver<'_>,
    schemas: &[Schema],
    mode: CompositeMode,
    active_refs: &mut Vec<String>,
) -> Result<UiNodeKind> {
    let variants = build_variants(resolver, schemas, active_refs)?;
    // For both oneOf and anyOf, use single selection (allow_multiple = false)
    // This ensures proper radio group UI for single-value composites
    // and correct behavior for array items
    let allow_multiple = false;
    Ok(UiNodeKind::Composite {
        mode,
        allow_multiple,
        variants,
    })
}

fn build_composite_node(
    resolver: &SchemaResolver<'_>,
    schemas: &[Schema],
    mode: CompositeMode,
    schema: &SchemaObject,
    pointer: String,
    required: bool,
    active_refs: &mut Vec<String>,
) -> Result<UiNode> {
    let kind = build_composite_kind(resolver, schemas, mode, active_refs)?;
    let default_value = if let UiNodeKind::Composite {
        variants,
        allow_multiple,
        ..
    } = &kind
    {
        infer_default_for_composite(variants, *allow_multiple)
    } else {
        None
    };
    Ok(UiNode {
        pointer,
        title: schema_title(schema),
        description: schema_description(schema),
        required,
        default_value,
        kind,
    })
}

fn visit_kind(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    active_refs: &mut Vec<String>,
) -> Result<UiNodeKind> {
    if let Some(subs) = schema.subschemas.as_ref()
        && let Some(all_of) = subs.all_of.as_ref()
        && !all_of.is_empty()
    {
        let merged = merge_all_of(resolver, all_of)?;
        return visit_kind(resolver, &merged, active_refs);
    }

    if let Some(subs) = schema.subschemas.as_ref() {
        if let Some(one_of) = subs.one_of.as_ref() {
            return build_composite_kind(resolver, one_of, CompositeMode::OneOf, active_refs);
        }
        if let Some(any_of) = subs.any_of.as_ref() {
            return build_composite_kind(resolver, any_of, CompositeMode::AnyOf, active_refs);
        }
    }

    if let Some(template) = build_key_value_template(resolver, schema, active_refs)? {
        return Ok(UiNodeKind::KeyValue {
            template: Box::new(template),
        });
    }

    if is_array_schema(schema) {
        let array = schema.array.as_ref();
        let item_node = match array {
            Some(array) if array.items.is_some() => {
                visit_array_item_kind(resolver, array, active_refs)?
            }
            _ => array_boundary_item_kind(),
        };
        return Ok(UiNodeKind::Array {
            item: Box::new(item_node),
            min_items: array.and_then(|inner| inner.min_items).map(|v| v as u64),
            max_items: array.and_then(|inner| inner.max_items).map(|v| v as u64),
        });
    }

    if is_object_schema(schema) {
        let obj = schema
            .object
            .as_ref()
            .context("object schema missing properties")?;
        let required_fields = required_list(obj);
        let mut children = Vec::new();
        for (name, schema) in &obj.properties {
            let pointer = append_pointer("", name);
            let node = visit_schema_entry(
                resolver,
                schema,
                pointer,
                required_fields.contains(name),
                active_refs,
            )?;
            children.push(node);
        }
        return Ok(UiNodeKind::Object {
            children,
            required: required_fields,
        });
    }

    let (scalar, enum_options, enum_values) = detect_scalar(schema)?;
    Ok(UiNodeKind::Field {
        scalar,
        enum_options,
        enum_values,
    })
}

fn build_variants(
    resolver: &SchemaResolver<'_>,
    schemas: &[Schema],
    active_refs: &mut Vec<String>,
) -> Result<Vec<UiVariant>> {
    let mut out = Vec::new();
    for (index, variant) in schemas.iter().enumerate() {
        out.push(build_variant(resolver, variant, index, active_refs)?);
    }
    Ok(out)
}

fn schema_default(schema: &SchemaObject) -> Option<Value> {
    schema.metadata.as_ref().and_then(|m| m.default.clone())
}

fn schema_const_value(schema: &SchemaObject) -> Option<&Value> {
    schema
        .const_value
        .as_ref()
        .or_else(|| schema.extensions.get("const"))
}

fn schema_default_or_const(schema: &SchemaObject) -> Option<Value> {
    schema_default(schema).or_else(|| schema_const_value(schema).cloned())
}

fn infer_default_scalar(scalar: ScalarKind, opts: Option<&Vec<Value>>) -> Option<Value> {
    if let Some(options) = opts
        && let Some(first) = options.first()
    {
        return Some(first.clone());
    }

    let val = match scalar {
        ScalarKind::String => Value::String(String::new()),
        ScalarKind::Integer => Value::Number(0.into()),
        ScalarKind::Number => Value::Number(0.into()),
        ScalarKind::Boolean => Value::Bool(false),
    };
    Some(val)
}

fn infer_default_for_composite(variants: &[UiVariant], allow_multiple: bool) -> Option<Value> {
    if allow_multiple {
        return Some(Value::Array(Vec::new()));
    }

    // Generate a unique default value for the first variant
    // that can be unambiguously identified
    variants.first().and_then(generate_variant_default)
}

/// Generate a default value for a variant that uniquely identifies it
fn generate_variant_default(variant: &UiVariant) -> Option<Value> {
    // For object variants, check if there are const fields that uniquely identify the variant
    if variant.is_object
        && let UiNodeKind::Object { children, required } = &variant.node
    {
        let mut obj = Map::new();

        // First, check the schema for const fields that uniquely identify this variant
        if let Value::Object(schema_obj) = &variant.schema
            && let Some(Value::Object(props)) = schema_obj.get("properties")
        {
            for (key, prop_schema) in props {
                if let Value::Object(prop_obj) = prop_schema {
                    // Set const fields to uniquely identify the variant
                    if let Some(const_val) = prop_obj.get("const") {
                        obj.insert(key.clone(), const_val.clone());
                    }
                }
            }
        }

        // Then add defaults for all required fields
        for child in children {
            let field_name = child.pointer.split('/').next_back().unwrap_or("");
            if !field_name.is_empty()
                && required.contains(&field_name.to_string())
                && !obj.contains_key(field_name)
            {
                if let Some(default) = &child.default_value {
                    obj.insert(field_name.to_string(), default.clone());
                } else if let Some(default) = default_for_kind(&child.kind) {
                    obj.insert(field_name.to_string(), default);
                }
            }
        }

        return Some(Value::Object(obj));
    }

    // For array variants, return an array with a sample element to distinguish between types
    if let UiNodeKind::Array { item, .. } = &variant.node
        && let Some(item_default) = default_for_kind(item)
    {
        // Return array with one default element to make it distinguishable
        return Some(Value::Array(vec![item_default]));
    }

    // For other variants, use the standard default
    default_for_kind(&variant.node)
}

fn default_for_kind(kind: &UiNodeKind) -> Option<Value> {
    match kind {
        UiNodeKind::Field {
            scalar,
            enum_values,
            ..
        } => infer_default_scalar(*scalar, enum_values.as_ref()),
        UiNodeKind::Array { .. } => Some(Value::Array(Vec::new())),
        UiNodeKind::KeyValue { .. } => Some(Value::Object(Map::new())),
        UiNodeKind::Composite {
            variants,
            allow_multiple,
            ..
        } => infer_default_for_composite(variants, *allow_multiple),
        UiNodeKind::Object { .. } => Some(Value::Object(Map::new())),
    }
}

type DetectedScalar = (ScalarKind, Option<Vec<String>>, Option<Vec<Value>>);

fn detect_scalar(schema: &SchemaObject) -> Result<DetectedScalar> {
    if let Some(enum_values) = schema.enum_values.as_ref()
        && !enum_values.is_empty()
    {
        let labels = enum_values.iter().map(enum_label).collect::<Vec<_>>();
        return Ok((
            infer_enum_scalar(enum_values),
            Some(labels),
            Some(enum_values.clone()),
        ));
    }

    if let Some(const_value) = schema_const_value(schema) {
        let labels = vec![enum_label(const_value)];
        let values = vec![const_value.clone()];
        return Ok((infer_enum_scalar(&values), Some(labels), Some(values)));
    }

    let instance = instance_type(schema);
    if matches!(instance, Some(InstanceType::Null)) {
        return Ok((
            ScalarKind::String,
            Some(vec!["null".to_string()]),
            Some(vec![Value::Null]),
        ));
    }

    let scalar = match instance {
        Some(InstanceType::String) | None => ScalarKind::String,
        Some(InstanceType::Integer) => ScalarKind::Integer,
        Some(InstanceType::Number) => ScalarKind::Number,
        Some(InstanceType::Boolean) => ScalarKind::Boolean,
        Some(InstanceType::Null) => unreachable!("null instance is handled as a fixed null enum"),
        Some(InstanceType::Array | InstanceType::Object) => {
            bail!("composite types should be handled earlier")
        }
    };
    Ok((scalar, None, None))
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

fn infer_enum_scalar(values: &[Value]) -> ScalarKind {
    let mut inferred = None;
    for value in values {
        let current = match value {
            Value::Number(num) if num.is_i64() || num.is_u64() => ScalarKind::Integer,
            Value::Number(_) => ScalarKind::Number,
            Value::Bool(_) => ScalarKind::Boolean,
            Value::String(_) => ScalarKind::String,
            _ => return ScalarKind::String,
        };
        match inferred {
            Some(existing) if existing != current => return ScalarKind::String,
            Some(_) => {}
            None => inferred = Some(current),
        }
    }
    inferred.unwrap_or(ScalarKind::String)
}

fn merge_all_of(resolver: &SchemaResolver<'_>, all_of: &[Schema]) -> Result<SchemaObject> {
    if all_of.is_empty() {
        bail!("allOf must contain at least one schema");
    }
    let mut acc = Value::Object(Map::new());
    for schema in all_of {
        let resolved = resolver.resolve_schema(schema)?;
        let value = schema_to_value(&resolved)?;
        acc = deep_merge(acc, value);
    }
    serde_json::from_value::<SchemaObject>(acc).context("failed to deserialize merged allOf schema")
}

fn array_item_schema(array: &ArrayValidation) -> Result<&Schema> {
    let items = array
        .items
        .as_ref()
        .context("array items must be present")?;
    match items {
        SingleOrVec::Single(schema) => Ok(schema.as_ref()),
        SingleOrVec::Vec(list) => list
            .first()
            .context("tuple arrays must have at least one item"),
    }
}

fn array_boundary_item_kind() -> UiNodeKind {
    UiNodeKind::Object {
        children: Vec::new(),
        required: Vec::new(),
    }
}

fn visit_array_item_kind(
    resolver: &SchemaResolver<'_>,
    array: &ArrayValidation,
    active_refs: &mut Vec<String>,
) -> Result<UiNodeKind> {
    let item_schema = array_item_schema(array)?;
    with_resolved_schema(
        resolver,
        item_schema,
        active_refs,
        |resolved| normalize_embedded_kind(resolver, &resolved, recursive_boundary_kind(&resolved)),
        |resolved, active_refs| {
            if is_object_schema(&resolved) && !has_composite_subschemas(&resolved) {
                build_single_variant_composite_kind(resolver, &resolved, active_refs)
            } else {
                let kind = visit_kind(resolver, &resolved, active_refs)?;
                normalize_embedded_kind(resolver, &resolved, kind)
            }
        },
    )
}

fn normalize_embedded_kind(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    kind: UiNodeKind,
) -> Result<UiNodeKind> {
    match kind {
        kind @ UiNodeKind::Array { .. } | kind @ UiNodeKind::Object { .. } => {
            build_single_variant_overlay_kind(resolver, schema, kind)
        }
        other => Ok(other),
    }
}

fn build_variant(
    resolver: &SchemaResolver<'_>,
    schema: &Schema,
    index: usize,
    active_refs: &mut Vec<String>,
) -> Result<UiVariant> {
    with_resolved_schema(
        resolver,
        schema,
        active_refs,
        |resolved| {
            build_variant_from_resolved_schema(
                resolver,
                index,
                &resolved,
                recursive_boundary_kind(&resolved),
            )
        },
        |resolved, active_refs| {
            let node = visit_kind(resolver, &resolved, active_refs)?;
            build_variant_from_resolved_schema(resolver, index, &resolved, node)
        },
    )
}

fn build_variant_from_resolved_schema(
    resolver: &SchemaResolver<'_>,
    index: usize,
    schema: &SchemaObject,
    node: UiNodeKind,
) -> Result<UiVariant> {
    let schema_value = schema_to_value_with_defs(resolver, schema)?;
    let title = schema
        .metadata
        .as_ref()
        .and_then(|m| m.title.clone())
        .or_else(|| Some(default_variant_title(index, schema)));
    let description = schema.metadata.as_ref().and_then(|m| m.description.clone());
    let is_object = is_object_schema(schema);
    Ok(UiVariant {
        id: format!("variant_{}", index),
        title,
        description,
        is_object,
        node,
        schema: schema_value,
    })
}

fn build_single_variant_composite_kind(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    active_refs: &mut Vec<String>,
) -> Result<UiNodeKind> {
    let node = visit_kind(resolver, schema, active_refs)?;
    build_single_variant_overlay_kind(resolver, schema, node)
}

fn build_single_variant_overlay_kind(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    node: UiNodeKind,
) -> Result<UiNodeKind> {
    let variant = build_variant_from_resolved_schema(resolver, 0, schema, node)?;
    Ok(UiNodeKind::Composite {
        mode: CompositeMode::OneOf,
        allow_multiple: false,
        variants: vec![variant],
    })
}

fn recursive_boundary_node(schema: &SchemaObject, pointer: String, required: bool) -> UiNode {
    let kind = recursive_boundary_kind(schema);
    let default_value = match &kind {
        UiNodeKind::Field {
            scalar,
            enum_values,
            ..
        } => schema_default_or_const(schema)
            .or_else(|| infer_default_scalar(*scalar, enum_values.as_ref())),
        UiNodeKind::Array { .. } => {
            schema_default_or_const(schema).or_else(|| Some(Value::Array(Vec::new())))
        }
        UiNodeKind::KeyValue { .. } => {
            schema_default_or_const(schema).or_else(|| Some(Value::Object(Map::new())))
        }
        UiNodeKind::Composite {
            variants,
            allow_multiple,
            ..
        } => schema_default_or_const(schema)
            .or_else(|| infer_default_for_composite(variants, *allow_multiple)),
        UiNodeKind::Object { .. } => {
            schema_default_or_const(schema).or_else(|| Some(Value::Object(Map::new())))
        }
    };
    UiNode {
        pointer,
        title: schema_title(schema),
        description: schema_description(schema),
        required,
        default_value,
        kind,
    }
}

fn recursive_boundary_kind(schema: &SchemaObject) -> UiNodeKind {
    if is_array_schema(schema) {
        let array = schema.array.as_ref();
        return UiNodeKind::Array {
            item: Box::new(array_boundary_item_kind()),
            min_items: array.and_then(|inner| inner.min_items).map(|v| v as u64),
            max_items: array.and_then(|inner| inner.max_items).map(|v| v as u64),
        };
    }

    if let Ok((scalar, enum_options, enum_values)) = detect_scalar(schema) {
        return UiNodeKind::Field {
            scalar,
            enum_options,
            enum_values,
        };
    }

    UiNodeKind::Object {
        children: Vec::new(),
        required: Vec::new(),
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
    if let Some(reference) = schema_reference(schema) {
        if active_refs.iter().any(|active| active == reference) {
            return on_recursive(resolved);
        }
        active_refs.push(reference.to_string());
        let result = on_resolved(resolved, active_refs);
        active_refs.pop();
        result
    } else {
        on_resolved(resolved, active_refs)
    }
}

fn instance_type(schema: &SchemaObject) -> Option<InstanceType> {
    schema.instance_type.as_ref().and_then(|inner| match inner {
        SingleOrVec::Single(single) => Some(**single),
        SingleOrVec::Vec(list) => list
            .iter()
            .cloned()
            .find(|item| *item != InstanceType::Null),
    })
}

fn is_object_schema(schema: &SchemaObject) -> bool {
    match instance_type(schema) {
        Some(InstanceType::Object) => true,
        None => schema.object.is_some(),
        _ => false,
    }
}

fn has_composite_subschemas(schema: &SchemaObject) -> bool {
    schema.subschemas.as_ref().is_some_and(|subs| {
        subs.one_of.as_ref().is_some_and(|list| !list.is_empty())
            || subs.any_of.as_ref().is_some_and(|list| !list.is_empty())
    })
}

fn is_array_schema(schema: &SchemaObject) -> bool {
    match instance_type(schema) {
        Some(InstanceType::Array) => true,
        _ => schema.array.is_some(),
    }
}

fn required_list(object: &ObjectValidation) -> Vec<String> {
    // Preserve the order in which required field names appear in the schema,
    // so that UI representations match the top-down order in the
    // config/schema file.
    object.required.iter().cloned().collect()
}

fn schema_to_value(schema: &SchemaObject) -> Result<Value> {
    serde_json::to_value(Schema::Object(schema.clone())).context("failed to serialize schema")
}

fn schema_to_value_with_defs(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
) -> Result<Value> {
    let mut value = schema_to_value(schema)?;
    if let Some(defs) = resolver.definitions_snapshot()
        && let Value::Object(ref mut map) = value
    {
        map.entry("definitions".to_string()).or_insert(defs);
    }
    Ok(value)
}

fn schema_title(schema: &SchemaObject) -> Option<String> {
    schema.metadata.as_ref()?.title.clone()
}

fn schema_description(schema: &SchemaObject) -> Option<String> {
    schema.metadata.as_ref()?.description.clone()
}

fn schema_titles(schema: &SchemaObject, fallback: &str) -> (String, Option<String>, Option<Value>) {
    (
        schema_title(schema).unwrap_or_else(|| fallback.to_string()),
        schema_description(schema),
        schema_default_or_const(schema),
    )
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

fn build_key_value_template(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    active_refs: &mut Vec<String>,
) -> Result<Option<UiKeyValueNode>> {
    let Some(object) = schema.object.as_ref() else {
        return Ok(None);
    };
    if !object.properties.is_empty() {
        return Ok(None);
    }

    let (value_schema_ref, key_schema_override) =
        if let Some((pattern, pattern_schema)) = object.pattern_properties.iter().next() {
            (
                pattern_schema,
                Some(serde_json::json!({
                    "type": "string",
                    "pattern": pattern,
                    "title": "Key",
                })),
            )
        } else if let Some(additional) = object.additional_properties.as_ref() {
            if matches!(&**additional, Schema::Bool(false) | Schema::Bool(true)) {
                return Ok(None);
            }
            (additional.as_ref(), None)
        } else {
            return Ok(None);
        };

    let (value_resolved, value_kind) = with_resolved_schema(
        resolver,
        value_schema_ref,
        active_refs,
        |resolved| {
            let kind =
                normalize_embedded_kind(resolver, &resolved, recursive_boundary_kind(&resolved))?;
            Ok((resolved, kind))
        },
        |resolved, active_refs| {
            let kind = visit_kind(resolver, &resolved, active_refs)?;
            let kind = normalize_embedded_kind(resolver, &resolved, kind)?;
            Ok((resolved, kind))
        },
    )?;

    let value_schema = schema_to_value_with_defs(resolver, &value_resolved)?;
    let (value_title, value_description, value_default) = schema_titles(&value_resolved, "Value");

    let (key_schema, key_title, key_description, key_default) =
        if let Some(override_schema) = key_schema_override {
            (override_schema, "Key".to_string(), None, None)
        } else if let Some(property_names) = object.property_names.as_ref() {
            let key_resolved = resolver.resolve_schema(property_names)?;
            let key_schema = schema_to_value_with_defs(resolver, &key_resolved)?;
            let (title, description, default) = schema_titles(&key_resolved, "Key");
            (key_schema, title, description, default)
        } else {
            (
                serde_json::json!({"type": "string", "title": "Key"}),
                "Key".to_string(),
                None,
                None,
            )
        };

    Ok(Some(UiKeyValueNode {
        key_title,
        key_description,
        key_default,
        key_schema: key_schema.clone(),
        value_title,
        value_description,
        value_default,
        value_schema: value_schema.clone(),
        value_kind: Box::new(value_kind),
        entry_schema: key_value_entry_schema(&key_schema, &value_schema),
    }))
}

fn default_variant_title(index: usize, schema: &SchemaObject) -> String {
    // First check if this is a reference, use the reference name
    if let Some(reference) = schema.reference.as_ref()
        && let Some(name) = reference.split('/').next_back()
    {
        // Convert camelCase/PascalCase to readable format
        return humanize_identifier(name);
    }

    // For objects with const fields, try to generate a meaningful name
    if let Some(obj) = schema.object.as_ref() {
        // Check for a 'type' const field which often identifies variants
        if let Some(type_prop) = obj.properties.get("type")
            && let Some(const_val) = get_const_value(type_prop)
            && let Some(s) = const_val.as_str()
        {
            return s.to_string();
        }

        // NEW: also check for a 'kind' const field, which we use as a
        // discriminant in many schemas (e.g. simple vs numeric item).
        if let Some(kind_prop) = obj.properties.get("kind")
            && let Some(const_val) = get_const_value(kind_prop)
            && let Some(s) = const_val.as_str()
        {
            // Use humanized form, e.g. "simple" -> "Simple".
            return humanize_identifier(s);
        }

        // Check for 'id' or 'name' fields which might identify the variant
        for key in ["id", "name", "key"] {
            if obj.properties.contains_key(key) {
                let base_type = instance_type(schema)
                    .map(|t| format!("{:?}", t).to_lowercase())
                    .unwrap_or_else(|| "variant".to_string());
                return format!("{} with {}", base_type, key);
            }
        }
    }

    // For arrays, describe what kind of array
    if let Some(array) = schema.array.as_ref()
        && let Some(items) = &array.items
    {
        match items {
            SingleOrVec::Single(item_schema) => {
                // Try to get a meaningful name for the item type
                if let Schema::Object(item_obj) = item_schema.as_ref() {
                    if let Some(item_ref) = item_obj.reference.as_ref()
                        && let Some(name) = item_ref.split('/').next_back()
                    {
                        return format!("{} array", humanize_identifier(name));
                    }

                    // NEW: for scalar arrays (string/integer/number/boolean),
                    // produce names like `List<string>` or `List<integer>`.
                    if let Some(item_instance) = instance_type(item_obj) {
                        let kind = match item_instance {
                            InstanceType::String => Some("string"),
                            InstanceType::Integer => Some("integer"),
                            InstanceType::Number => Some("number"),
                            InstanceType::Boolean => Some("boolean"),
                            _ => None,
                        };
                        if let Some(kind) = kind {
                            return format!("List<{}>", kind);
                        }
                    }
                }
            }
            SingleOrVec::Vec(_) => {
                return "Tuple array".to_string();
            }
        }
    }

    // Fallback to basic type description
    if let Some(instance) = instance_type(schema) {
        return match instance {
            InstanceType::String => "Text".to_string(),
            InstanceType::Integer => "Integer".to_string(),
            InstanceType::Number => "Number".to_string(),
            InstanceType::Boolean => "Boolean".to_string(),
            InstanceType::Array => "List".to_string(),
            InstanceType::Object => "Object".to_string(),
            InstanceType::Null => "Null".to_string(),
        };
    }

    format!("Option {}", index + 1)
}

fn humanize_identifier(s: &str) -> String {
    // Convert camelCase or PascalCase to readable format
    // e.g., "simpleItem" -> "Simple Item", "URLConfig" -> "URL Config"
    let mut result = String::new();
    let mut prev_upper = false;

    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() {
            if i > 0 && !prev_upper {
                result.push(' ');
            }
            result.push(ch);
            prev_upper = true;
        } else {
            if i == 0 {
                result.push(ch.to_ascii_uppercase());
            } else {
                result.push(ch);
            }
            prev_upper = false;
        }
    }

    result
}

fn get_const_value(schema: &Schema) -> Option<&Value> {
    if let Schema::Object(obj) = schema {
        if let Some(const_val) = obj.const_value.as_ref() {
            return Some(const_val);
        }
        // Also check in extensions for const
        if let Some(const_val) = obj.extensions.get("const") {
            return Some(const_val);
        }
    }
    None
}

fn deep_merge(base: Value, addition: Value) -> Value {
    match (base, addition) {
        (Value::Object(mut a), Value::Object(b)) => {
            for (key, value) in b {
                let merged = if let Some(existing) = a.remove(&key) {
                    deep_merge(existing, value)
                } else {
                    value
                };
                a.insert(key, merged);
            }
            Value::Object(a)
        }
        (Value::Array(mut a), Value::Array(mut b)) => {
            a.append(&mut b);
            dedup_array(&mut a);
            Value::Array(a)
        }
        (_, new_value) => new_value,
    }
}

fn dedup_array(values: &mut Vec<Value>) {
    let mut index = 0;
    while index < values.len() {
        let is_duplicate = values[..index]
            .iter()
            .any(|existing| existing == &values[index]);
        if is_duplicate {
            values.remove(index);
        } else {
            index += 1;
        }
    }
}

fn append_pointer(base: &str, segment: &str) -> String {
    let encoded = segment.replace('~', "~0").replace('/', "~1");
    if base.is_empty() || base == "/" {
        format!("/{}", encoded)
    } else if base.ends_with('/') {
        format!("{base}{encoded}")
    } else {
        format!("{base}/{encoded}")
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use schemars::schema::SubschemaValidation;

    #[test]
    fn variant_title_uses_kind_const() {
        // Build a simple object schema with a `kind` const discriminator.
        let mut kind_schema = SchemaObject::default();
        kind_schema.const_value = Some(Value::String("simple".to_string()));

        let mut obj = ObjectValidation::default();
        obj.properties
            .insert("kind".to_string(), Schema::Object(kind_schema));

        let mut schema = SchemaObject::default();
        schema.object = Some(Box::new(obj));

        let title = default_variant_title(0, &schema);
        assert_eq!(title, "Simple");
    }

    #[test]
    fn variant_title_for_scalar_array_is_list_of_type() {
        // Build an array schema whose items are scalar strings.
        let mut item_schema = SchemaObject::default();
        item_schema.instance_type = Some(SingleOrVec::Single(Box::new(InstanceType::String)));

        let mut array = ArrayValidation::default();
        array.items = Some(SingleOrVec::Single(Box::new(Schema::Object(item_schema))));

        let mut schema = SchemaObject::default();
        schema.array = Some(Box::new(array));

        let title = default_variant_title(0, &schema);
        assert_eq!(title, "List<string>");
    }

    #[test]
    fn has_composite_subschemas_detects_one_of_and_any_of() {
        // oneOf present
        let mut with_one_of = SchemaObject::default();
        let mut subs_one = SubschemaValidation::default();
        subs_one.one_of = Some(vec![Schema::Bool(true)]);
        with_one_of.subschemas = Some(Box::new(subs_one));
        assert!(has_composite_subschemas(&with_one_of));

        // anyOf present
        let mut with_any_of = SchemaObject::default();
        let mut subs_any = SubschemaValidation::default();
        subs_any.any_of = Some(vec![Schema::Bool(true)]);
        with_any_of.subschemas = Some(Box::new(subs_any));
        assert!(has_composite_subschemas(&with_any_of));

        // neither oneOf nor anyOf
        let plain = SchemaObject::default();
        assert!(!has_composite_subschemas(&plain));
    }
}
