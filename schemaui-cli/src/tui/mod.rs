use std::fs;

use color_eyre::eyre::{Report, Result};
use schemaui::SchemaUI;
use schemaui::precompile::tui as pre_tui;

use crate::cli::{CommonArgs, TuiSnapshotCommand};
use crate::session::diagnostics::DiagnosticCollector;
use crate::session::format::resolve_format_hint;
use crate::session::schema_source::{load_optional_document, resolve_session_inputs};
use crate::session::{SessionBundle, prepare_session};

pub fn run_cli(args: &CommonArgs) -> Result<()> {
    let session = prepare_session(args)?;
    execute_session(session)
}

pub(crate) fn execute_session(session: SessionBundle) -> Result<()> {
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
    let value = ui.run_tui().map_err(Report::msg)?;
    if let Some(options) = output {
        options.write(&value).map_err(Report::msg)?;
    }
    Ok(())
}

pub fn run_snapshot_cli(cmd: TuiSnapshotCommand) -> Result<()> {
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

    fs::create_dir_all(&cmd.out_dir)?;
    let tui_module = cmd.out_dir.join("tui_artifacts.rs");
    let form_module = cmd.out_dir.join("tui_form_schema.rs");
    let layout_module = cmd.out_dir.join("tui_layout_nav.rs");

    pre_tui::generate_tui_artifacts_module_from_value(
        &resolved.schema,
        resolved.defaults.as_ref(),
        &tui_module,
        &cmd.tui_fn,
    )
    .map_err(Report::msg)?;
    pre_tui::generate_tui_form_schema_module_from_value(
        &resolved.schema,
        resolved.defaults.as_ref(),
        &form_module,
        &cmd.form_fn,
    )
    .map_err(Report::msg)?;
    pre_tui::generate_tui_layout_nav_module_from_value(
        &resolved.schema,
        resolved.defaults.as_ref(),
        &layout_module,
        &cmd.layout_fn,
    )
    .map_err(Report::msg)?;

    eprintln!("Generated TUI artifact modules:");
    eprintln!("  TuiArtifacts module:    {:?}", tui_module);
    eprintln!("  FormSchema module:      {:?}", form_module);
    eprintln!("  LayoutNavModel module:  {:?}", layout_module);

    Ok(())
}
