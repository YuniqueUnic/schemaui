use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::io::{DocumentFormat, input::parse_document_str, input::schema_with_defaults};
use crate::ui_ast::{UiAst, build_ui_ast};
use crate::web::session::SessionResponse;

/// Build a minimal Web session snapshot (SessionResponse) from a schema file
/// and optional defaults file. This mirrors the runtime WebSessionBuilder
/// logic but is intended for use at compile-time or in external codegen
/// tools.
pub fn build_session_snapshot_from_files(
    schema_path: &Path,
    schema_format: DocumentFormat,
    defaults_path: Option<&Path>,
) -> Result<SessionResponse> {
    let schema_raw = fs::read_to_string(schema_path)?;
    let schema_value: Value = parse_document_str(&schema_raw, schema_format)?;

    let defaults_value: Value = if let Some(path) = defaults_path {
        let raw = fs::read_to_string(path)?;
        parse_document_str(&raw, schema_format)?
    } else {
        Value::Object(Default::default())
    };

    let enriched = schema_with_defaults(&schema_value, &defaults_value);
    let ui_ast: UiAst = build_ui_ast(&enriched)?;

    let formats: Vec<String> = DocumentFormat::available_formats()
        .into_iter()
        .map(|f| f.to_string())
        .collect();

    Ok(SessionResponse {
        title: None,
        ui_ast,
        data: defaults_value,
        formats,
    })
}

/// Write a SessionResponse snapshot as pretty JSON to a file.
pub fn write_session_snapshot_json(snapshot: &SessionResponse, out_path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(snapshot)?;
    fs::write(out_path, json)?;
    Ok(())
}

/// Write a TypeScript module exporting a SessionResponse constant. This is
/// useful for static SPA deployments where the session payload is embedded
/// directly into the frontend bundle.
pub fn write_session_snapshot_ts_module(
    snapshot: &SessionResponse,
    out_path: &Path,
    export_name: &str,
) -> Result<()> {
    let json = serde_json::to_string_pretty(snapshot)?;
    let src = format!(
        "import type {{ SessionResponse }} from '@schemaui/types/SessionResponse';\n\nexport const {export_name}: SessionResponse = {json} as const;\n",
    );
    fs::write(out_path, src)?;
    Ok(())
}
