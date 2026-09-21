use std::{fs, path::PathBuf};

use anyhow::Result;
use schemaui::precompile::web::{
    build_session_snapshot, write_session_snapshot_json, write_session_snapshot_ts_module,
};
use schemaui::web::session::ServeOptions as WebServeOptions;
use schemaui::{SchemaUI, Theme};

use crate::cli::{WebCommand, WebSnapshotCommand};
use crate::session::diagnostics::DiagnosticCollector;
use crate::session::format::resolve_format_hint;
use crate::session::lifecycle;
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
        description,
        output,
        draft,
    } = session;

    let mut ui = if let Some(defaults) = defaults {
        SchemaUI::new(defaults).with_schema(schema)
    } else {
        SchemaUI::from_schema(schema)
    };
    if let Some(title) = title {
        ui = ui.with_title(title);
    }
    if let Some(description) = description {
        ui = ui.with_description(description);
    }
    if let Some(deadline) = lifecycle::deadline_from_seconds(cmd.common.timeout) {
        ui = ui.with_timeout(deadline);
    }
    if let Some(draft) = draft {
        ui = ui.with_draft(draft);
    }

    let mut serve =
        WebServeOptions::new(cmd.host, cmd.port).with_theme(load_theme(cmd.common.web_theme));
    if let Some(dir) = cmd.frontend {
        serve = serve.with_frontend_dir(dir);
    }

    lifecycle::finish(ui.run_web(serve)?, output)
}

/// Read `--web-theme`, or carry on without one.
///
/// A stylesheet that will not load is worth saying out loud but not worth
/// refusing to open the form over: the session is still perfectly usable in the
/// default theme, and aborting would cost the user their actual task.
fn load_theme(path: Option<PathBuf>) -> Option<Theme> {
    let path = path?;
    match Theme::from_path(&path) {
        Ok(theme) => Some(theme),
        Err(err) => {
            eprintln!(
                "schemaui: ignoring --web-theme {} and using the default theme ({err:#})",
                path.display()
            );
            None
        }
    }
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
    let resolved = resolve_session_inputs(schema_document, config_document)?;
    let mut snapshot = build_session_snapshot(&resolved.schema, resolved.defaults.as_ref())?;
    if let Some(title) = cmd.common.title {
        snapshot.title = Some(title);
    }
    if let Some(description) = cmd.common.description {
        snapshot.description = Some(description);
    }

    fs::create_dir_all(&cmd.out_dir)?;
    let json_out = cmd.out_dir.join("session_snapshot.json");
    let ts_out = cmd.out_dir.join("session_snapshot.ts");

    write_session_snapshot_json(&snapshot, &json_out)?;
    write_session_snapshot_ts_module(&snapshot, &ts_out, &cmd.ts_export)?;

    eprintln!("Generated Web precompile snapshots:");
    eprintln!("  JSON:      {:?}", json_out);
    eprintln!("  TypeScript: {:?}", ts_out);

    Ok(())
}
