use std::collections::HashSet;

use anyhow::{Context, Result, anyhow, bail};
use schemars::schema::{
    ArrayValidation, InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec,
};
use serde::Serialize;
use serde_json::{Map, Value};
use ts_rs::TS;

use crate::schema::{loader::load_root_schema, resolver::SchemaResolver};

#[derive(Debug, Clone, Serialize, TS)]
#[ts(export, export_to = "web/types/ui-ast.ts")]
pub struct UiAst {
    pub roots: Vec<UiNode>,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct UiNode {
    pub pointer: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub required: bool,
    #[ts(type = "Record<string, unknown> | null")]
    pub default_value: Option<Value>,
    pub kind: UiNodeKind,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum UiNodeKind {
    Field {
        scalar: ScalarKind,
        enum_options: Option<Vec<String>>,
    },
    Array {
        item: Box<UiNodeKind>,
        min_items: Option<u64>,
        max_items: Option<u64>,
    },
    Composite {
        mode: CompositeMode,
        allow_multiple: bool,
        variants: Vec<UiVariant>,
    },
    Object {
        children: Vec<UiNode>,
        required: Vec<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum ScalarKind {
    String,
    Integer,
    Number,
    Boolean,
}

#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum CompositeMode {
    OneOf,
    AnyOf,
}

#[derive(Debug, Clone, Serialize, TS)]
pub struct UiVariant {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub is_object: bool,
    pub node: UiNodeKind,
    #[ts(type = "Record<string, unknown>")]
    pub schema: Value,
}

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
        let normalized = normalize_schema(&resolved)?;
        let pointer = append_pointer("", name);
        let node = visit_schema(&resolver, &normalized, pointer, required.contains(name))?;
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
            let variants = build_variants(resolver, one_of)?;
            let default_value = infer_default_for_composite(&variants, false);
            return Ok(UiNode {
                pointer,
                title: schema_title(schema),
                description: schema_description(schema),
                required,
                default_value,
                kind: UiNodeKind::Composite {
                    mode: CompositeMode::OneOf,
                    allow_multiple: false,
                    variants,
                },
            });
        }
        if let Some(any_of) = subs.any_of.as_ref() {
            let variants = build_variants(resolver, any_of)?;
            let allow_multiple = variants
                .iter()
                .any(|v| matches!(&v.node, UiNodeKind::Object { .. }));
            let default_value = infer_default_for_composite(&variants, allow_multiple);
            return Ok(UiNode {
                pointer,
                title: schema_title(schema),
                description: schema_description(schema),
                required,
                default_value,
                kind: UiNodeKind::Composite {
                    mode: CompositeMode::AnyOf,
                    allow_multiple,
                    variants,
                },
            });
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
            let normalized = normalize_schema(&resolved)?;
            let child_ptr = append_pointer(&pointer, name);
            let child = visit_schema(
                resolver,
                &normalized,
                child_ptr,
                required_set.contains(name),
            )?;
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
            let variants = build_variants(resolver, one_of)?;
            return Ok(UiNodeKind::Composite {
                mode: CompositeMode::OneOf,
                allow_multiple: false,
                variants,
            });
        }
        if let Some(any_of) = subs.any_of.as_ref() {
            let variants = build_variants(resolver, any_of)?;
            let allow_multiple = variants
                .iter()
                .any(|v| matches!(&v.node, UiNodeKind::Object { .. }));
            return Ok(UiNodeKind::Composite {
                mode: CompositeMode::AnyOf,
                allow_multiple,
                variants,
            });
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
            let normalized = normalize_schema(&resolved)?;
            let pointer = append_pointer("", name);
            let node = visit_schema(resolver, &normalized, pointer, required.contains(name))?;
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
        let normalized = normalize_schema(&resolved)?;
        let node = visit_kind(resolver, &normalized)?;
        let mut schema_value = schema_to_value(&resolved)?;
        if let Some(defs) = resolver.definitions_snapshot()
            && let Value::Object(ref mut map) = schema_value
        {
            map.entry("$defs".to_string()).or_insert(defs);
        }
        let title = normalized
            .metadata
            .as_ref()
            .and_then(|m| m.title.clone())
            .or_else(|| Some(default_variant_title(index, &normalized)));
        let description = normalized
            .metadata
            .as_ref()
            .and_then(|m| m.description.clone());
        let is_object = is_object_schema(&normalized);
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

fn normalize_schema(schema: &SchemaObject) -> Result<SchemaObject> {
    Ok(schema.clone())
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
    variants.first().and_then(|v| default_for_kind(&v.node))
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
    if let Some(instance) = instance_type(schema) {
        return match instance {
            InstanceType::String => "string".to_string(),
            InstanceType::Integer => "integer".to_string(),
            InstanceType::Number => "number".to_string(),
            InstanceType::Boolean => "boolean".to_string(),
            InstanceType::Array => "array".to_string(),
            InstanceType::Object => "object".to_string(),
            InstanceType::Null => format!("Variant {}", index + 1),
        };
    }
    format!("Variant {}", index + 1)
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
