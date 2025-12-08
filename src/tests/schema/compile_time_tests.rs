use std::path::PathBuf;

use serde_json::Value;

use crate::compile_time;
use crate::io::{DocumentFormat, input::parse_document_str};
use crate::ui_ast::build_ui_ast;

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("schemas")
        .join("test-comprehensive.schema.json")
}

#[test]
fn compile_time_and_runtime_ui_ast_match_for_comprehensive_schema() {
    let path = schema_path();

    let compile_time_ast = compile_time::build_ui_ast_from_file(&path, DocumentFormat::Json)
        .expect("compile_time UiAst");

    let contents = std::fs::read_to_string(&path).expect("schema file readable");
    let schema_value: Value =
        parse_document_str(&contents, DocumentFormat::Json).expect("schema parses at runtime");
    let runtime_ast = build_ui_ast(&schema_value).expect("runtime UiAst");

    assert_eq!(runtime_ast, compile_time_ast);
}

#[test]
fn ui_ast_json_roundtrip_preserves_structure() {
    let path = schema_path();

    let original = compile_time::build_ui_ast_from_file(&path, DocumentFormat::Json)
        .expect("compile_time UiAst");
    let json = compile_time::ui_ast_to_json(&original).expect("UiAst to JSON");
    let decoded = compile_time::decode_ui_ast(&json).expect("decode UiAst from JSON");

    assert_eq!(original, decoded);
}
