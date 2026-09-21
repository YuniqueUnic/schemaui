//! Pure orchestration shared by the `schemaui-wasm` bindings.
//!
//! Each function here is the same computation `src/web/session.rs` performs
//! for the matching HTTP route, minus the session state (no validator cache,
//! no draft, no deadline) — there is nothing to hold between calls in a
//! request/response wasm binding. Keeping the two in lock-step is what stops
//! the HTTP contract and the wasm API from drifting into different
//! behaviours; see `docs/en/decisions/0005-wasm-core-and-hosting.md`.
//!
//! This module has no `wasm32` in it: it is ordinary, host-portable Rust, so
//! it is exercised by the regular test suite rather than only under a wasm
//! target.
//!
//! [`parse_document`] is the one function here with no HTTP counterpart — it
//! exists for the Playground's paste-a-schema screen, which has no server to
//! have already parsed the input.

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

use crate::io::{
    DocumentFormat,
    input::{
        parse_document_auto, schema_from_data_value, schema_with_defaults as merge_schema_defaults,
    },
    output::OutputOptions,
};
use crate::ui_ast::{UiAst, UiLayout, build_ui_ast_bundle};

/// Mirrors the fields `GET /api/v1/session` returns, minus the server-side
/// metadata (`title`, `description`, `api_version`, `capabilities`,
/// `expires_in_ms`, `draft_restored`) that only make sense for a live session.
#[derive(Debug, Clone, Serialize)]
pub struct SessionBuild {
    pub ui_ast: UiAst,
    pub layout: UiLayout,
    pub data: Value,
    pub formats: Vec<String>,
}

/// Mirrors `POST /api/v1/validate`'s response shape exactly.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationResult {
    pub ok: bool,
    pub errors: Vec<FieldError>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FieldError {
    pub pointer: String,
    pub message: String,
}

/// Mirrors `POST /api/v1/preview`'s response shape exactly.
#[derive(Debug, Clone, Serialize)]
pub struct RenderResult {
    pub payload: String,
}

/// Build the `UiAst`/`UiLayout` a session would bootstrap with, for `schema`
/// seeded with `defaults`.
pub fn build_ui_ast(schema: Value, defaults: Value) -> Result<SessionBuild> {
    let enriched_schema = merge_schema_defaults(&schema, &defaults);
    let bundle = build_ui_ast_bundle(&enriched_schema)?;
    let (ui_ast, layout) = bundle.into_parts();
    Ok(SessionBuild {
        ui_ast,
        layout,
        data: defaults,
        formats: DocumentFormat::available_formats()
            .into_iter()
            .map(|format| format.to_string())
            .collect(),
    })
}

/// Validate `data` against `schema`.
pub fn validate(schema: Value, data: Value) -> Result<ValidationResult> {
    let validator = jsonschema::validator_for(&schema).context("failed to compile JSON schema")?;
    let errors: Vec<FieldError> = validator
        .iter_errors(&data)
        .map(|error| FieldError {
            pointer: error.instance_path().to_string(),
            message: error.to_string(),
        })
        .collect();
    Ok(ValidationResult {
        ok: errors.is_empty(),
        errors,
    })
}

/// Render `data` as a `format` document, honoring the same enabled-formats
/// check the HTTP route applies.
pub fn render(data: Value, format: &str, pretty: bool) -> Result<RenderResult> {
    let format = DocumentFormat::from_keyword(format).map_err(anyhow::Error::msg)?;
    if !DocumentFormat::available_formats().contains(&format) {
        anyhow::bail!("format '{format}' is disabled for this build");
    }
    let payload = OutputOptions::new(format)
        .with_pretty(pretty)
        .render(&data)?;
    Ok(RenderResult { payload })
}

/// Merge `data` into `schema` as `default` values. A thin wrapper so the wasm
/// binding does not reach past this module into `crate::io`.
pub fn schema_with_defaults(schema: Value, data: Value) -> Value {
    merge_schema_defaults(&schema, &data)
}

/// Infer a JSON Schema from a data value, the way the CLI does when no schema
/// is given — see `schemaui-cli/src/session/schema_source.rs`.
pub fn schema_from_data(data: Value) -> Value {
    schema_from_data_value(&data)
}

/// Parse `text` as JSON, YAML or TOML, trying each enabled format in turn.
///
/// Not an HTTP mirror like the functions above: a live session only ever
/// receives already-parsed JSON over the wire, so the server has no
/// equivalent route. The Playground's paste-a-schema screen does need this —
/// pasting a schema or a data document in whichever of the three formats the
/// author had on hand, the way `schemaui-cli` accepts any of them from a
/// file, rather than forcing JSON specifically.
pub fn parse_document(text: &str) -> Result<Value> {
    parse_document_auto(text)
}
