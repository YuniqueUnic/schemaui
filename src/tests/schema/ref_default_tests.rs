use std::path::PathBuf;

use serde_json::{Value, json};

use crate::{
    io::{DocumentFormat, input::parse_document_str, input::schema_with_defaults},
    ui_ast::{UiNode, UiNodeKind, build_ui_ast},
};

fn schema_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("schemas")
        .join(name)
}

fn load_schema(name: &str) -> Value {
    let contents = std::fs::read_to_string(schema_path(name)).expect("schema fixture readable");
    parse_document_str(&contents, DocumentFormat::Json).expect("schema fixture parses")
}

fn find_node<'a>(nodes: &'a [UiNode], pointer: &str) -> &'a UiNode {
    for node in nodes {
        if node.pointer == pointer {
            return node;
        }
        if let UiNodeKind::Object { children, .. } = &node.kind
            && let Some(found) = find_node_opt(children, pointer)
        {
            return found;
        }
    }
    panic!("node {pointer} not found")
}

fn find_node_opt<'a>(nodes: &'a [UiNode], pointer: &str) -> Option<&'a UiNode> {
    nodes.iter().find_map(|node| {
        if node.pointer == pointer {
            Some(node)
        } else if let UiNodeKind::Object { children, .. } = &node.kind {
            find_node_opt(children, pointer)
        } else {
            None
        }
    })
}

fn assert_validates(schema: &Value, instance: &Value) {
    let validator = jsonschema::validator_for(schema).expect("fixture must compile as jsonschema");
    assert!(
        validator.is_valid(instance),
        "fixture should accept the reproduction instance: {instance}"
    );
}

fn assert_ref_defaults_are_scoped(
    schema: Value,
    definition_default_pointer: &str,
    property_without_config_default_pointer: &str,
) {
    let defaults = json!({ "second": 40 });
    assert_validates(&schema, &defaults);

    let enriched = schema_with_defaults(&schema, &defaults);

    assert_eq!(
        enriched.pointer("/properties/first/default"),
        None,
        "config data must not synthesize a default on absent sibling ref properties"
    );
    assert_eq!(
        enriched.pointer("/properties/second/default"),
        Some(&json!(40)),
        "config data should remain scoped to the ref property that supplied it"
    );
    assert_eq!(
        enriched.pointer(definition_default_pointer),
        Some(&json!(20)),
        "shared definition default must not be overwritten by one ref use-site"
    );

    let ast = build_ui_ast(&enriched).expect("ui ast builds from enriched schema");
    assert_eq!(
        find_node(&ast.roots, "/first").default_value,
        Some(json!(20)),
        "absent sibling should use the referenced schema default"
    );
    assert_eq!(
        find_node(&ast.roots, "/second").default_value,
        Some(json!(40)),
        "configured sibling should use the instance value"
    );
    assert_eq!(
        find_node(&ast.roots, "/first").default_value,
        Some(json!(20)),
        "{property_without_config_default_pointer} must not inherit /second's value"
    );
}

#[test]
fn draft7_dollar_defs_keeps_ref_defaults_scoped() {
    assert_ref_defaults_are_scoped(
        load_schema("ref-default-scope.draft7.schema.json"),
        "/$defs/mydef/default",
        "/properties/first/default",
    );
}

#[test]
fn draft7_definitions_keeps_ref_defaults_scoped() {
    assert_ref_defaults_are_scoped(
        load_schema("ref-default-scope.definitions.draft7.schema.json"),
        "/definitions/mydef/default",
        "/properties/first/default",
    );
}

#[test]
fn draft_2020_12_dollar_defs_keeps_ref_defaults_scoped() {
    assert_ref_defaults_are_scoped(
        load_schema("ref-default-scope.2020-12.schema.json"),
        "/$defs/mydef/default",
        "/properties/first/default",
    );
}

#[test]
fn nested_object_ref_defaults_are_scoped_per_ref_use() {
    let schema = json!({
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "$defs": {
            "endpoint": {
                "type": "object",
                "properties": {
                    "host": { "type": "string", "default": "localhost" },
                    "port": { "type": "integer", "default": 80 }
                },
                "additionalProperties": false
            }
        },
        "type": "object",
        "properties": {
            "primary": { "$ref": "#/$defs/endpoint" },
            "secondary": { "$ref": "#/$defs/endpoint" }
        },
        "additionalProperties": false
    });
    let defaults = json!({
        "secondary": { "host": "api.internal", "port": 8080 }
    });

    assert_validates(&schema, &defaults);

    let enriched = schema_with_defaults(&schema, &defaults);
    assert_eq!(
        enriched.pointer("/$defs/endpoint/properties/host/default"),
        Some(&json!("localhost")),
        "nested object definition must keep its original host default"
    );
    assert_eq!(
        enriched.pointer("/$defs/endpoint/properties/port/default"),
        Some(&json!(80)),
        "nested object definition must keep its original port default"
    );

    let ast = build_ui_ast(&enriched).expect("ui ast builds");
    assert_eq!(
        find_node(&ast.roots, "/primary/host").default_value,
        Some(json!("localhost"))
    );
    assert_eq!(
        find_node(&ast.roots, "/primary/port").default_value,
        Some(json!(80))
    );
    assert_eq!(
        find_node(&ast.roots, "/secondary/host").default_value,
        Some(json!("api.internal"))
    );
    assert_eq!(
        find_node(&ast.roots, "/secondary/port").default_value,
        Some(json!(8080))
    );
}
