use color_eyre::eyre::{Result, eyre};

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
        return Err(eyre!("provide at least --schema or --config"));
    }
    let resolved = resolve_session_inputs(schema_document, config_document)?;

    Ok(SessionBundle {
        schema: resolved.schema,
        defaults: resolved.defaults,
        title: args.title.clone(),
        output: output_settings,
    })
}
