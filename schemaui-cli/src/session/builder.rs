use anyhow::{Result, anyhow};
use schemaui::DraftStore;
use serde_json::Value;

use crate::cli::CommonArgs;

use super::bundle::SessionBundle;
use super::diagnostics::DiagnosticCollector;
use super::format::resolve_format_hint;
use super::output::{build_output_options, ensure_output_paths_available};
use super::schema_source::{load_optional_document, resolve_session_inputs};

pub fn prepare_session(args: &CommonArgs) -> Result<SessionBundle> {
    let mut diagnostics = DiagnosticCollector::default();

    let schema_spec = args.schema.as_deref();
    let config_spec = args.config.as_deref();
    let schema_stdin = schema_spec == Some("-");
    let config_stdin = config_spec == Some("-");
    if schema_stdin && config_stdin {
        diagnostics.push_input(
            "schema/config",
            "cannot read schema and config from stdin simultaneously; provide inline content or files",
        );
    }

    let schema_hint = resolve_format_hint(schema_spec, "schema", &mut diagnostics);
    let config_hint = resolve_format_hint(config_spec, "config", &mut diagnostics);

    let schema_document = load_optional_document(
        schema_spec,
        schema_hint.hint.format,
        "schema",
        schema_hint.blocked || (schema_stdin && config_stdin),
        &mut diagnostics,
    );
    let config_document = load_optional_document(
        config_spec,
        config_hint.hint.format,
        "config",
        config_hint.blocked || (schema_stdin && config_stdin),
        &mut diagnostics,
    );

    let (output_settings, output_paths) = build_output_options(
        args,
        config_hint.hint.extension_value(),
        schema_hint.hint.extension_value(),
        &mut diagnostics,
    );
    ensure_output_paths_available(&output_paths, args.force, &mut diagnostics);

    diagnostics.into_result()?;

    if schema_document.is_none() && config_document.is_none() {
        return Err(anyhow!("provide at least --schema or --config"));
    }
    let resolved = resolve_session_inputs(schema_document, config_document)?;

    let draft = resolve_draft(args, &resolved.schema);

    Ok(SessionBundle {
        schema: resolved.schema,
        defaults: resolved.defaults,
        title: args.title.clone(),
        description: args.description.clone(),
        output: output_settings,
        draft,
    })
}

/// Where this run keeps its draft.
///
/// Drafts are on by default: a session that is interrupted after an explicit
/// save is exactly the case they exist for, and asking users to opt in would
/// mean only those who already lost work ever turn them on. A machine with no
/// usable state directory runs without one, with a warning — losing drafts is
/// not a reason to refuse to show a form.
fn resolve_draft(args: &CommonArgs, schema: &Value) -> Option<DraftStore> {
    if let Some(path) = args.draft.as_ref() {
        return Some(DraftStore::at(path));
    }
    match DraftStore::for_schema(schema) {
        Ok(store) => Some(store),
        Err(err) => {
            eprintln!("schemaui: drafts are disabled for this run ({err:#})");
            None
        }
    }
}
