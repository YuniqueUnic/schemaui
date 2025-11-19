use std::collections::HashSet;

use anyhow::{Context, Result, anyhow, bail};
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec,
};
use serde_json::{Map, Value};

use crate::schema::{loader::load_root_schema, resolver::SchemaResolver};

use super::types::{CompositeMode, ScalarKind, UiAst, UiNode, UiNodeKind, UiVariant};

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
    let required = required_set(object);

    let mut roots = Vec::new();
    for (name, schema) in &object.properties {
        let resolved = resolver.resolve_schema(schema)?;
        let pointer = append_pointer("", name);
        let node = visit_schema(&resolver, &resolved, pointer, required.contains(name))?;
        roots.push(node);
    }

    Ok(UiAst { roots })
}

fn visit_schema(
    resolver: &SchemaResolver<'_>,
    schema: &SchemaObject,
    pointer: String,
    required: bool,
) -> Result<UiNode> {
    if let Some(subs) = schema.subschemas.as_ref()
        && let Some(all_of) = subs.all_of.as_ref()
        && !all_of.is_empty()
    {
        let merged = merge_all_of(resolver, all_of)?;
        return visit_schema(resolver, &merged, pointer, required);
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
            );
        }
    }

    if is_array_schema(schema) {
        let array = schema
            .array
            .as_ref()
            .context("array schema must define array metadata")?;
        let items_schema = resolve_array_items(resolver, array)?;
        let item_node = visit_kind(resolver, &items_schema)?;
        let default_value = schema_default(schema).or_else(|| Some(Value::Array(Vec::new())));
        return Ok(UiNode {
            pointer,
            title: schema_title(schema),
            description: schema_description(schema),
            required,
            default_value,
            kind: UiNodeKind::Array {
                item: Box::new(item_node),
                min_items: array.min_items.map(|v| v as u64),
                max_items: array.max_items.map(|v| v as u64),
            },
        });
    }

    if is_object_schema(schema) {
        let obj = schema
            .object
            .as_ref()
            .context("object schema missing properties")?;
        let mut children = Vec::new();
        let required_set = required_set(obj);
        for (name, child_schema) in &obj.properties {
            let resolved = resolver.resolve_schema(child_schema)?;
            let child_ptr = append_pointer(&pointer, name);
            let child = visit_schema(resolver, &resolved, child_ptr, required_set.contains(name))?;
            children.push(child);
        }
        let default_value = schema_default(schema).or(Some(Value::Object(Map::new())));
        return Ok(UiNode {
            pointer,
            title: schema_title(schema),
            description: schema_description(schema),
            required,
            default_value,
            kind: UiNodeKind::Object {
                children,
                required: required_set.into_iter().collect(),
            },
        });
    }

    let (scalar, enum_options) = detect_scalar(schema)?;
    let default_value =
        schema_default(schema).or_else(|| infer_default_scalar(scalar, enum_options.as_ref()));
    Ok(UiNode {
        pointer,
        title: schema_title(schema),
        description: schema_description(schema),
        required,
        default_value,
        kind: UiNodeKind::Field {
            scalar,
            enum_options,
        },
    })
}

fn build_composite_kind(
    resolver: &SchemaResolver<'_>,
    schemas: &[Schema],
    mode: CompositeMode,
) -> Result<UiNodeKind> {
    let variants = build_variants(resolver, schemas)?;
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
) -> Result<UiNode> {
    let kind = build_composite_kind(resolver, schemas, mode)?;
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

fn visit_kind(resolver: &SchemaResolver<'_>, schema: &SchemaObject) -> Result<UiNodeKind> {
    if let Some(subs) = schema.subschemas.as_ref()
        && let Some(all_of) = subs.all_of.as_ref()
        && !all_of.is_empty()
    {
        let merged = merge_all_of(resolver, all_of)?;
        return visit_kind(resolver, &merged);
    }

    if let Some(subs) = schema.subschemas.as_ref() {
        if let Some(one_of) = subs.one_of.as_ref() {
            return build_composite_kind(resolver, one_of, CompositeMode::OneOf);
        }
        if let Some(any_of) = subs.any_of.as_ref() {
            return build_composite_kind(resolver, any_of, CompositeMode::AnyOf);
        }
    }

    if is_array_schema(schema) {
        let array = schema
            .array
            .as_ref()
            .context("array schema must define array metadata")?;
        let items_schema = resolve_array_items(resolver, array)?;
        let item_node = visit_kind(resolver, &items_schema)?;
        return Ok(UiNodeKind::Array {
            item: Box::new(item_node),
            min_items: array.min_items.map(|v| v as u64),
            max_items: array.max_items.map(|v| v as u64),
        });
    }

    if is_object_schema(schema) {
        let obj = schema
            .object
            .as_ref()
            .context("object schema missing properties")?;
        let required = required_set(obj);
        let mut children = Vec::new();
        for (name, schema) in &obj.properties {
            let resolved = resolver.resolve_schema(schema)?;
            let pointer = append_pointer("", name);
            let node = visit_schema(resolver, &resolved, pointer, required.contains(name))?;
            children.push(node);
        }
        return Ok(UiNodeKind::Object {
            children,
            required: required.into_iter().collect(),
        });
    }

    let (scalar, enum_options) = detect_scalar(schema)?;
    Ok(UiNodeKind::Field {
        scalar,
        enum_options,
    })
}

fn build_variants(resolver: &SchemaResolver<'_>, schemas: &[Schema]) -> Result<Vec<UiVariant>> {
    let mut out = Vec::new();
    for (index, variant) in schemas.iter().enumerate() {
        let resolved = resolver.resolve_schema(variant)?;
        let node = visit_kind(resolver, &resolved)?;
        let mut schema_value = schema_to_value(&resolved)?;
        if let Some(defs) = resolver.definitions_snapshot()
            && let Value::Object(ref mut map) = schema_value
        {
            map.entry("$defs".to_string()).or_insert(defs);
        }
        let title = resolved
            .metadata
            .as_ref()
            .and_then(|m| m.title.clone())
            .or_else(|| Some(default_variant_title(index, &resolved)));
        let description = resolved
            .metadata
            .as_ref()
            .and_then(|m| m.description.clone());
        let is_object = is_object_schema(&resolved);
        out.push(UiVariant {
            id: format!("variant_{}", index),
            title,
            description,
            is_object,
            node,
            schema: schema_value,
        });
    }
    Ok(out)
}

fn schema_default(schema: &SchemaObject) -> Option<Value> {
    schema.metadata.as_ref().and_then(|m| m.default.clone())
}

fn infer_default_scalar(scalar: ScalarKind, opts: Option<&Vec<String>>) -> Option<Value> {
    let val = match scalar {
        ScalarKind::String => Value::String(String::new()),
        ScalarKind::Integer => Value::Number(0.into()),
        ScalarKind::Number => Value::Number(0.into()),
        ScalarKind::Boolean => Value::Bool(false),
    };
    if let Some(options) = opts
        && let Some(first) = options.first()
    {
        return Some(Value::String(first.clone()));
    }
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
            enum_options,
        } => infer_default_scalar(*scalar, enum_options.as_ref()),
        UiNodeKind::Array { .. } => Some(Value::Array(Vec::new())),
        UiNodeKind::Composite {
            variants,
            allow_multiple,
            ..
        } => infer_default_for_composite(variants, *allow_multiple),
        UiNodeKind::Object { .. } => Some(Value::Object(Map::new())),
    }
}

fn detect_scalar(schema: &SchemaObject) -> Result<(ScalarKind, Option<Vec<String>>)> {
    if let Some(enum_values) = schema.enum_values.as_ref()
        && !enum_values.is_empty()
    {
        let options = enum_values
            .iter()
            .map(|v| match v {
                Value::String(s) => Ok(s.clone()),
                other => Ok(other.to_string()),
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;
        return Ok((ScalarKind::String, Some(options)));
    }

    let instance = instance_type(schema);
    let scalar = match instance {
        Some(InstanceType::String) | None => ScalarKind::String,
        Some(InstanceType::Integer) => ScalarKind::Integer,
        Some(InstanceType::Number) => ScalarKind::Number,
        Some(InstanceType::Boolean) => ScalarKind::Boolean,
        Some(InstanceType::Null) => ScalarKind::String,
        Some(InstanceType::Array | InstanceType::Object) => {
            bail!("composite types should be handled earlier")
        }
    };
    Ok((scalar, None))
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

fn resolve_array_items(
    resolver: &SchemaResolver<'_>,
    array: &ArrayValidation,
) -> Result<SchemaObject> {
    let items = array
        .items
        .as_ref()
        .context("array items must be present")?;
    let first = match items {
        SingleOrVec::Single(schema) => schema.as_ref(),
        SingleOrVec::Vec(list) => list
            .first()
            .context("tuple arrays must have at least one item")?,
    };
    resolver.resolve_schema(first)
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

fn is_array_schema(schema: &SchemaObject) -> bool {
    match instance_type(schema) {
        Some(InstanceType::Array) => true,
        _ => schema.array.is_some(),
    }
}

fn required_set(object: &ObjectValidation) -> HashSet<String> {
    object.required.iter().cloned().collect()
}

fn schema_to_value(schema: &SchemaObject) -> Result<Value> {
    serde_json::to_value(Schema::Object(schema.clone())).context("failed to serialize schema")
}

fn schema_title(schema: &SchemaObject) -> Option<String> {
    schema.metadata.as_ref()?.title.clone()
}

fn schema_description(schema: &SchemaObject) -> Option<String> {
    schema.metadata.as_ref()?.description.clone()
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
                if let Schema::Object(item_obj) = item_schema.as_ref()
                    && let Some(item_ref) = item_obj.reference.as_ref()
                    && let Some(name) = item_ref.split('/').next_back()
                {
                    return format!("{} array", humanize_identifier(name));
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
            InstanceType::Null => format!("Option {}", index + 1),
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
