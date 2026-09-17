use anyhow::{Context, Result, bail};
use serde_json::{Map, Value};

mod composite;
mod defaults;
mod hints;
mod key_value;
mod naming;
mod schema_helpers;
mod visit;

use crate::schema::{
    loader::load_root_schema,
    model::{ArrayValidation, InstanceType, ObjectValidation, Schema, SchemaObject, SingleOrVec},
    resolver::{SchemaResolver, schema_reference},
};

use super::types::{
    CompositeMode, ScalarKind, UiAst, UiKeyValueNode, UiNode, UiNodeKind, UiVariant, VisibleWhen,
};

pub fn build_ui_ast(raw: &Value) -> Result<UiAst> {
    let root_schema = load_root_schema(raw)?;
    let resolver = SchemaResolver::new(raw);

    if !schema_helpers::is_object_schema(&root_schema) {
        bail!("root schema must describe an object");
    }

    let fallback_object = ObjectValidation::default();
    let object = root_schema
        .object
        .as_ref()
        .map_or(&fallback_object, Box::as_ref);
    let required = schema_helpers::required_list(object);

    let mut active_refs = Vec::new();
    let roots = visit::visit_object_children(
        &resolver,
        &object.properties,
        &required,
        "",
        &mut active_refs,
    )?;

    Ok(UiAst { roots })
}
