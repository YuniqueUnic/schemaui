use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::{Report, Result};
use schemaui::precompile::tui as pre_tui;
use schemaui::{DocumentFormat, SchemaUI};

use crate::cli::{CommonArgs, TuiSnapshotCommand};
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
    let mut ui = SchemaUI::new(schema);
    if let Some(title) = title {
        ui = ui.with_title(title);
    }
    if let Some(ref defaults) = defaults {
        ui = ui.with_default_data(defaults);
    }
    if let Some(options) = output {
        ui = ui.with_output(options);
    }
    ui.run().map_err(Report::msg).map(|_| ())
}

pub fn run_snapshot_cli(cmd: TuiSnapshotCommand) -> Result<()> {
    // For TUI snapshots we require a schema file path (no stdin/inline).
    let schema_spec = cmd
        .common
        .schema
        .as_deref()
        .ok_or_else(|| Report::msg("tui-snapshot requires --schema <PATH>"))?;
    if schema_spec == "-" {
        return Err(Report::msg(
            "tui-snapshot does not support --schema - (stdin); please pass a file path",
        ));
    }

    let schema_path = PathBuf::from(schema_spec);
    if !schema_path.exists() {
        return Err(Report::msg(format!(
            "schema path {:?} does not exist",
            schema_path
        )));
    }

    let config_spec = cmd.common.config.as_deref();
    if config_spec == Some("-") {
        return Err(Report::msg(
            "tui-snapshot does not support --config - (stdin); please pass a file path",
        ));
    }

    let defaults_path = config_spec.map(PathBuf::from);
    if let Some(ref path) = defaults_path
        && !path.exists()
    {
        return Err(Report::msg(format!(
            "config path {:?} does not exist",
            path
        )));
    }

    let format = DocumentFormat::from_extension(&schema_path).unwrap_or(DocumentFormat::Json);

    fs::create_dir_all(&cmd.out_dir)?;
    let tui_module = cmd.out_dir.join("tui_artifacts.rs");
    let form_module = cmd.out_dir.join("precompiled_form_schema.rs");
    let layout_module = cmd.out_dir.join("precompiled_layout_nav.rs");

    pre_tui::generate_tui_artifacts_module(
        &schema_path,
        format,
        defaults_path.as_deref(),
        &tui_module,
        &cmd.tui_fn,
    )
    .map_err(Report::msg)?;
    pre_tui::generate_tui_form_schema_module(
        &schema_path,
        format,
        defaults_path.as_deref(),
        &form_module,
        &cmd.form_fn,
    )
    .map_err(Report::msg)?;
    pre_tui::generate_tui_layout_nav_module(
        &schema_path,
        format,
        defaults_path.as_deref(),
        &layout_module,
        &cmd.layout_fn,
    )
    .map_err(Report::msg)?;

    eprintln!("Generated TUI precompiled modules:");
    eprintln!("  TuiArtifacts module:    {:?}", tui_module);
    eprintln!("  FormSchema module:      {:?}", form_module);
    eprintln!("  LayoutNavModel module:  {:?}", layout_module);

    Ok(())
}
