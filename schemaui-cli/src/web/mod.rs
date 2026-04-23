use std::fs;

use color_eyre::eyre::{Report, Result};
use schemaui::SchemaUI;
use schemaui::precompile::web::{
    build_session_snapshot, write_session_snapshot_json, write_session_snapshot_ts_module,
};
use schemaui::web::session::ServeOptions as WebServeOptions;

use crate::cli::{WebCommand, WebSnapshotCommand};
use crate::session::diagnostics::DiagnosticCollector;
use crate::session::format::resolve_format_hint;
use crate::session::schema_source::{load_optional_document, resolve_session_inputs};
use crate::session::{SessionBundle, prepare_session};

pub fn run_cli(cmd: WebCommand) -> Result<()> {
    let session = prepare_session(&cmd.common)?;
    execute_web_session(session, cmd)
}

fn execute_web_session(session: SessionBundle, cmd: WebCommand) -> Result<()> {
    let SessionBundle {
        schema,
        defaults,
        title,
        output,
    } = session;

    let mut ui = if let Some(defaults) = defaults {
        SchemaUI::new(defaults).with_schema(schema)
    } else {
        SchemaUI::from_schema(schema)
    };
    if let Some(title) = title {
        ui = ui.with_title(title);
    }

    let serve = WebServeOptions {
        host: cmd.host,
        port: cmd.port,
    };

    let value = ui.run_web(serve).map_err(Report::msg)?;
    if let Some(options) = output {
        options.write(&value).map_err(Report::msg)?;
    }
    Ok(())
}

pub fn run_snapshot_cli(cmd: WebSnapshotCommand) -> Result<()> {
    let schema_spec = cmd.common.schema.as_deref();
    let config_spec = cmd.common.config.as_deref();
    let mut diagnostics = DiagnosticCollector::default();
    let schema_stdin = schema_spec == Some("-");
    let config_stdin = config_spec == Some("-");
    if schema_stdin && config_stdin {
        diagnostics.push_input(
            "schema/config",
            "cannot read schema and config from stdin simultaneously; provide inline content, files, or a remote schema",
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
    diagnostics.into_result()?;
    let resolved = resolve_session_inputs(schema_document, config_document).map_err(Report::msg)?;
    let snapshot = build_session_snapshot(&resolved.schema, resolved.defaults.as_ref())
        .map_err(Report::msg)?;

    fs::create_dir_all(&cmd.out_dir)?;
    let json_out = cmd.out_dir.join("session_snapshot.json");
    let ts_out = cmd.out_dir.join("session_snapshot.ts");

    write_session_snapshot_json(&snapshot, &json_out).map_err(Report::msg)?;
    write_session_snapshot_ts_module(&snapshot, &ts_out, &cmd.ts_export).map_err(Report::msg)?;

    eprintln!("Generated Web precompile snapshots:");
    eprintln!("  JSON:      {:?}", json_out);
    eprintln!("  TypeScript: {:?}", ts_out);

    Ok(())
}
