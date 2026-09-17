use schemaui::ui_ast::{UiNode, UiNodeKind, VisibleWhenOp, build_ui_ast};
use serde_json::json;

fn find_node<'a>(nodes: &'a [UiNode], pointer: &str) -> Option<&'a UiNode> {
    for node in nodes {
        if node.pointer == pointer {
            return Some(node);
        }
        if let UiNodeKind::Object { children, .. } = &node.kind
            && let Some(found) = find_node(children, pointer)
        {
            return Some(found);
        }
    }
    None
}

fn recursive_tree_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "tree": {"$ref": "#/definitions/treeNode"}
        },
        "definitions": {
            "treeNode": {
                "type": "object",
                "properties": {
                    "name": {"type": "string"},
                    "children": {
                        "type": "array",
                        "items": {"$ref": "#/definitions/treeNode"}
                    }
                },
                "required": ["name"]
            }
        }
    })
}

#[test]
fn ui_ast_preserves_numeric_enum_values() {
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": {
            "EnumInteger": {
                "enum": [2, 5, 42]
            }
        }
    });

    let ast = build_ui_ast(&schema).expect("ui ast");
    let field = find_node(&ast.roots, "/EnumInteger").expect("enum field");
    match &field.kind {
        UiNodeKind::Field {
            enum_options,
            enum_values,
            ..
        } => {
            assert_eq!(
                enum_options.as_ref(),
                Some(&vec!["2".to_string(), "5".to_string(), "42".to_string()])
            );
            assert_eq!(
                enum_values.as_ref(),
                Some(&vec![json!(2), json!(5), json!(42)])
            );
            assert_eq!(field.default_value, Some(json!(2)));
        }
        other => panic!("expected field node, got {:?}", other),
    }
}

#[test]
fn ui_ast_treats_const_as_single_option_default() {
    let schema = json!({
        "type": "object",
        "properties": {
            "kind": {
                "type": "string",
                "const": "beta"
            }
        }
    });

    let ast = build_ui_ast(&schema).expect("ui ast");
    let field = find_node(&ast.roots, "/kind").expect("const field");
    match &field.kind {
        UiNodeKind::Field {
            enum_options,
            enum_values,
            ..
        } => {
            assert_eq!(enum_options.as_ref(), Some(&vec!["beta".to_string()]));
            assert_eq!(enum_values.as_ref(), Some(&vec![json!("beta")]));
            assert_eq!(field.default_value, Some(json!("beta")));
        }
        other => panic!("expected const field to become a single-option field, got {other:?}"),
    }
}

#[test]
fn ui_ast_prefers_instance_metadata_over_definition_metadata() {
    let schema = json!({
        "definitions": {
            "duration_ms": {
                "title": "Definition Title",
                "description": "Definition description",
                "type": "integer"
            }
        },
        "type": "object",
        "properties": {
            "timeout": {
                "$ref": "#/definitions/duration_ms",
                "title": "Request Timeout",
                "description": "Per-request timeout",
                "default": 5
            }
        }
    });

    let ast = build_ui_ast(&schema).expect("ui ast");
    let field = find_node(&ast.roots, "/timeout").expect("timeout field");
    assert_eq!(field.title.as_deref(), Some("Request Timeout"));
    assert_eq!(field.description.as_deref(), Some("Per-request timeout"));
    assert_eq!(field.default_value, Some(json!(5)));
}

#[test]
fn ui_ast_wraps_recursive_array_items_as_editable_single_variant_composites() {
    let ast = build_ui_ast(&recursive_tree_schema()).expect("recursive schema should build");
    let root = find_node(&ast.roots, "/tree").expect("tree root");

    match &root.kind {
        UiNodeKind::Object { children, required } => {
            let child_pointers: Vec<_> = children
                .iter()
                .map(|child| child.pointer.as_str())
                .collect();
            assert_eq!(required, &vec!["name".to_string()]);
            assert!(child_pointers.contains(&"/tree/name"));
            assert!(child_pointers.contains(&"/tree/children"));
        }
        other => panic!("expected tree root to remain object, got {:?}", other),
    }

    let children_field = find_node(&ast.roots, "/tree/children").expect("children field");
    match &children_field.kind {
        UiNodeKind::Array { item, .. } => match item.as_ref() {
            UiNodeKind::Composite { variants, .. } => {
                assert_eq!(
                    variants.len(),
                    1,
                    "recursive array item should keep one lazy variant"
                );
                let variant = &variants[0];
                assert!(
                    variant.is_object,
                    "tree node entries should stay object-like"
                );
                let properties = variant
                    .schema
                    .get("properties")
                    .and_then(|value| value.as_object())
                    .expect("recursive variant properties");
                assert!(properties.contains_key("name"));
                assert!(properties.contains_key("children"));
            }
            other => panic!(
                "expected recursive array item boundary composite, got {:?}",
                other
            ),
        },
        other => panic!("expected children to remain an array, got {:?}", other),
    }
}

#[test]
fn ui_ast_wraps_nested_array_items_as_editable_single_variant_composites() {
    let schema = json!({
        "type": "object",
        "properties": {
            "matrix": {
                "type": "array",
                "items": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    }
                }
            }
        }
    });

    let ast = build_ui_ast(&schema).expect("ui ast");
    let matrix = find_node(&ast.roots, "/matrix").expect("matrix field");
    match &matrix.kind {
        UiNodeKind::Array { item, .. } => match item.as_ref() {
            UiNodeKind::Composite { variants, .. } => {
                assert_eq!(
                    variants.len(),
                    1,
                    "matrix rows should open through one lazy variant"
                );
                assert!(
                    !variants[0].is_object,
                    "nested arrays should stay wrapped instead of pretending to be objects"
                );
                assert_eq!(
                    variants[0]
                        .schema
                        .get("type")
                        .and_then(|value| value.as_str()),
                    Some("array")
                );
            }
            other => panic!("expected nested array item to be wrapped as composite, got {other:?}"),
        },
        other => panic!("expected matrix to remain an array, got {other:?}"),
    }
}

fn field_is_multiline(nodes: &[UiNode], pointer: &str) -> bool {
    match &find_node(nodes, pointer).expect("field node").kind {
        UiNodeKind::Field { multiline, .. } => *multiline,
        other => panic!("expected {pointer} to be a field, got {other:?}"),
    }
}

#[test]
fn ui_ast_attaches_visible_when_from_extension() {
    let schema = json!({
        "type": "object",
        "properties": {
            "delivery": { "type": "string", "enum": ["file", "other"] },
            "delivery_custom": {
                "type": "string",
                "x-visible-when": { "field": "delivery", "op": "equals", "value": "other" }
            }
        }
    });

    let ast = build_ui_ast(&schema).expect("ui ast");

    let controller = find_node(&ast.roots, "/delivery").expect("delivery field");
    assert!(
        controller.visible_when.is_none(),
        "a node without the extension is always visible"
    );

    let dependent = find_node(&ast.roots, "/delivery_custom").expect("custom field");
    let rule = dependent.visible_when.as_ref().expect("visibility rule");
    assert_eq!(rule.field, "delivery");
    assert_eq!(rule.op, VisibleWhenOp::Equals);
    assert_eq!(rule.value, json!("other"));
}

#[test]
fn ui_ast_reads_contains_rules_for_multi_select_escape_hatches() {
    let schema = json!({
        "type": "object",
        "properties": {
            "features": {
                "type": "array",
                "items": { "type": "string", "enum": ["mount", "other"] },
                "uniqueItems": true
            },
            "features_custom": {
                "type": "string",
                "x-visible-when": { "field": "features", "op": "contains", "value": "other" }
            }
        }
    });

    let ast = build_ui_ast(&schema).expect("ui ast");
    let dependent = find_node(&ast.roots, "/features_custom").expect("custom field");
    let rule = dependent.visible_when.as_ref().expect("visibility rule");
    assert_eq!(rule.op, VisibleWhenOp::Contains);
    assert_eq!(rule.value, json!("other"));
}

#[test]
fn ui_ast_rejects_malformed_visible_when() {
    let schema = json!({
        "type": "object",
        "properties": {
            "mode": { "type": "string" },
            "detail": {
                "type": "string",
                "x-visible-when": { "field": "mode", "equals": "custom" }
            }
        }
    });

    let error = build_ui_ast(&schema).expect_err("a rule without `op` must be rejected");
    assert!(
        error.to_string().contains("x-visible-when"),
        "error should name the keyword, got: {error}"
    );
}

#[test]
fn ui_ast_rejects_visible_when_pointing_at_an_unknown_sibling() {
    let schema = json!({
        "type": "object",
        "properties": {
            "mode": { "type": "string" },
            "detail": {
                "type": "string",
                "x-visible-when": { "field": "modes", "op": "equals", "value": "custom" }
            }
        }
    });

    let error = build_ui_ast(&schema).expect_err("a typo'd sibling must be rejected");
    assert!(
        error.to_string().contains("modes"),
        "error should name the unknown sibling, got: {error}"
    );
}

#[test]
fn ui_ast_marks_multiline_string_fields() {
    let schema = json!({
        "type": "object",
        "properties": {
            "summary": { "type": "string" },
            "notes": { "type": "string", "x-multiline": true }
        }
    });

    let ast = build_ui_ast(&schema).expect("ui ast");
    assert!(!field_is_multiline(&ast.roots, "/summary"));
    assert!(field_is_multiline(&ast.roots, "/notes"));
}

#[test]
fn ui_ast_rejects_non_boolean_multiline() {
    let schema = json!({
        "type": "object",
        "properties": {
            "notes": { "type": "string", "x-multiline": "yes" }
        }
    });

    let error = build_ui_ast(&schema).expect_err("a non-boolean hint must be rejected");
    assert!(
        error.to_string().contains("x-multiline"),
        "error should name the keyword, got: {error}"
    );
}
