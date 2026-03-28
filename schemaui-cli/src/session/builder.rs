use color_eyre::eyre::{Result, eyre};
use schemaui::{DocumentFormat, schema_from_data_value};
use serde_json::Value;

use crate::cli::CommonArgs;
use crate::io::load_value;

use super::bundle::SessionBundle;
use super::diagnostics::DiagnosticCollector;
use super::format::resolve_format_hint;
use super::output::{build_output_options, ensure_output_paths_available};

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

    let schema_value = load_optional_value(
        schema_spec,
        schema_hint.hint.format,
        "schema",
        schema_hint.blocked || (schema_stdin && config_stdin),
        &mut diagnostics,
    );
    let config_value = load_optional_value(
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

    let mut schema_value = schema_value;
    let mut config_value = config_value;
    if schema_value.is_none()
        && let Some(config_doc) = config_value.as_ref()
        && looks_like_json_schema(config_doc)
    {
        eprintln!("detected JSON Schema provided via --config; treating it as the active schema");
        schema_value = config_value.take();
    }

    if schema_value.is_none() && config_value.is_none() {
        return Err(eyre!("provide at least --schema or --config"));
    }

    let schema = match (schema_value, config_value.as_ref()) {
        (Some(schema), _) => schema,
        (None, Some(defaults)) => schema_from_data_value(defaults),
        (None, None) => unreachable!("validated above"),
    };

    Ok(SessionBundle {
        schema,
        defaults: config_value,
        title: args.title.clone(),
        output: output_settings,
    })
}

fn load_optional_value(
    spec: Option<&str>,
    format: DocumentFormat,
    label: &str,
    skip: bool,
    diagnostics: &mut DiagnosticCollector,
) -> Option<Value> {
    if skip {
        return None;
    }
    let raw = spec?;
    match load_value(raw, format, label) {
        Ok(value) => Some(value),
        Err(err) => {
            diagnostics.push_input(label, err.to_string());
            None
        }
    }
}

fn looks_like_json_schema(value: &Value) -> bool {
    let obj = match value.as_object() {
        Some(map) => map,
        None => return false,
    };

    if obj
        .get("properties")
        .and_then(Value::as_object)
        .map(|props| props.len())
        .unwrap_or(0)
        == 0
    {
        return false;
    }

    if obj.contains_key("$schema") {
        return true;
    }

    if matches!(obj.get("type"), Some(Value::String(t)) if t == "object") {
        return true;
    }

    if let Some(props) = obj.get("properties").and_then(Value::as_object) {
        let mut scored = 0usize;

        for value in props.values() {
            if value.get("type").is_some() {
                scored += 1;
            }
            if value.get("properties").is_some() {
                scored += 1;
            }
            if value.get("enum").is_some() {
                scored += 1;
            }
        }

        return scored >= 2;
    }

    false
}
