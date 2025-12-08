use std::path::PathBuf;

use crate::compile_time::web;
use crate::io::DocumentFormat;

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("schemas")
        .join("test-comprehensive.schema.json")
}

#[test]
fn build_session_snapshot_for_comprehensive_schema_succeeds() {
    let path = schema_path();

    let snapshot = web::build_session_snapshot_from_files(&path, DocumentFormat::Json, None)
        .expect("session snapshot");

    assert!(!snapshot.formats.is_empty());
    assert!(!snapshot.ui_ast.roots.is_empty());
}
