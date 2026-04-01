use schemaui::ui_ast::{UiNode, UiNodeKind, build_ui_ast};
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
fn ui_ast_truncates_recursive_refs_to_finite_boundary() {
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
            UiNodeKind::Object { children, required } => {
                assert!(
                    children.is_empty(),
                    "recursive item should not expand infinitely"
                );
                assert!(
                    required.is_empty(),
                    "truncated recursive item should have no nested required fields"
                );
            }
            other => panic!(
                "expected recursive array item boundary object, got {:?}",
                other
            ),
        },
        other => panic!("expected children to remain an array, got {:?}", other),
    }
}
