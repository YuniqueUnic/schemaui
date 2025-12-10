use std::path::PathBuf;

use crate::io::DocumentFormat;
use crate::precompile::web;
use crate::web::session::SessionResponse;

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

#[test]
fn json_snapshot_roundtrip_via_file_is_deserializable() {
    let path = schema_path();

    let snapshot = web::build_session_snapshot_from_files(&path, DocumentFormat::Json, None)
        .expect("session snapshot");

    // Write JSON snapshot to a temp-ish file and ensure we can read it back
    // into a SessionResponse. This mirrors what the web frontend expects when
    // consuming the snapshot as raw JSON.
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "schemaui_web_snapshot_{}_json.json",
        std::process::id()
    ));

    web::write_session_snapshot_json(&snapshot, &out_path).expect("write JSON snapshot");

    let contents = std::fs::read_to_string(&out_path).expect("read JSON snapshot");
    let decoded: SessionResponse =
        serde_json::from_str(&contents).expect("deserialize JSON snapshot into SessionResponse");

    // Basic structural sanity checks
    assert_eq!(decoded.formats, snapshot.formats);
    assert!(!decoded.ui_ast.roots.is_empty());
    assert!(decoded.layout.is_some());

    let _ = std::fs::remove_file(&out_path);
}

#[test]
fn ts_snapshot_module_has_expected_export_shape() {
    let path = schema_path();

    let snapshot = web::build_session_snapshot_from_files(&path, DocumentFormat::Json, None)
        .expect("session snapshot");

    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "schemaui_web_snapshot_{}_module.ts",
        std::process::id()
    ));

    let export_name = "PrecompiledSession";
    web::write_session_snapshot_ts_module(&snapshot, &out_path, export_name)
        .expect("write TS snapshot module");

    let src = std::fs::read_to_string(&out_path).expect("read TS snapshot module");

    // We don't run tsc here (that belongs in the web/ui repo), but we do check
    // that the generated module has the expected import and export signature.
    assert!(
        src.contains("import type { SessionResponse } from '@schemaui/types/SessionResponse';",)
    );
    assert!(src.contains("export const PrecompiledSession: SessionResponse ="));

    let _ = std::fs::remove_file(&out_path);
}
