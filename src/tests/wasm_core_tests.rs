//! Unit coverage for `wasm_core`, independent of the wasm target or the
//! `schemaui-wasm` crate — these run as plain host tests.

use serde_json::json;

use crate::wasm_core::{build_ui_ast, render, schema_from_data, schema_with_defaults, validate};

fn fixture_schema() -> serde_json::Value {
    json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "retries": { "type": "integer", "minimum": 0 }
        },
        "required": ["name"]
    })
}

#[test]
fn build_ui_ast_seeds_data_and_lists_formats() {
    let built = build_ui_ast(fixture_schema(), json!({ "name": "alice" }))
        .expect("a valid schema should build");

    assert_eq!(built.data, json!({ "name": "alice" }));
    assert!(!built.ui_ast.roots.is_empty());
    assert!(!built.formats.is_empty());
}

#[test]
fn build_ui_ast_rejects_an_invalid_schema() {
    let broken = json!({ "type": "not-a-real-type" });
    assert!(build_ui_ast(broken, json!({})).is_err());
}

#[test]
fn validate_reports_ok_and_errors_like_the_http_route() {
    let ok = validate(fixture_schema(), json!({ "name": "alice" })).expect("schema compiles");
    assert!(ok.ok);
    assert!(ok.errors.is_empty());

    let bad = validate(fixture_schema(), json!({ "retries": -1 })).expect("schema compiles");
    assert!(!bad.ok);
    assert_eq!(bad.errors[0].pointer, "");
}

#[test]
fn render_serializes_to_the_requested_format() {
    let result = render(json!({ "name": "alice" }), "json", false).expect("json is always on");
    assert_eq!(result.payload, r#"{"name":"alice"}"#);
}

#[test]
fn render_rejects_an_unknown_format_keyword() {
    assert!(render(json!({}), "not-a-format", false).is_err());
}

#[test]
fn schema_with_defaults_merges_data_into_the_schema() {
    let merged = schema_with_defaults(fixture_schema(), json!({ "name": "alice" }));
    assert_eq!(merged["properties"]["name"]["default"], json!("alice"));
}

#[test]
fn schema_from_data_infers_types_and_defaults() {
    let inferred = schema_from_data(json!({ "port": 8080 }));
    assert_eq!(inferred["properties"]["port"]["type"], json!("integer"));
    assert_eq!(inferred["properties"]["port"]["default"], json!(8080));
}
