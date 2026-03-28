use std::fs;
use std::path::PathBuf;

use color_eyre::eyre::{Report, Result};
use schemaui::precompile::web::{
    build_session_snapshot_from_files, write_session_snapshot_json,
    write_session_snapshot_ts_module,
};
use schemaui::web::session::ServeOptions as WebServeOptions;
use schemaui::{DocumentFormat, SchemaUI};

use crate::cli::{WebCommand, WebSnapshotCommand};
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

    let serve = WebServeOptions {
        host: cmd.host,
        port: cmd.port,
    };

    ui.run_web(serve).map_err(Report::msg).map(|_| ())
}

pub fn run_snapshot_cli(cmd: WebSnapshotCommand) -> Result<()> {
    // For snapshots we intentionally use a simpler pipeline that only supports
    // file-based schema/config. This keeps behaviour predictable and mirrors
    // the `web_precompile_snapshot` example.

    let schema_spec = cmd
        .common
        .schema
        .as_deref()
        .ok_or_else(|| Report::msg("web-snapshot requires --schema <PATH>"))?;
    if schema_spec == "-" {
        return Err(Report::msg(
            "web-snapshot does not support --schema - (stdin); please pass a file path",
        ));
    }

    let config_spec = cmd.common.config.as_deref();
    if config_spec == Some("-") {
        return Err(Report::msg(
            "web-snapshot does not support --config - (stdin); please pass a file path",
        ));
    }

    let schema_path = PathBuf::from(schema_spec);
    if !schema_path.exists() {
        return Err(Report::msg(format!(
            "schema path {:?} does not exist",
            schema_path
        )));
    }

    let defaults_path = config_spec.map(PathBuf::from);
    if let Some(ref p) = defaults_path
        && !p.exists()
    {
        return Err(Report::msg(format!("config path {:?} does not exist", p)));
    }

    let format = DocumentFormat::from_extension(&schema_path).unwrap_or(DocumentFormat::Json);

    let snapshot =
        build_session_snapshot_from_files(&schema_path, format, defaults_path.as_deref())
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
