use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};
use schemaui::precompile::build_ui_artifact_bundle_from_file;
use schemaui::{DocumentFormat, SchemaUI, TuiFrontend, UiOptions, parse_document_str};

/// Run the TUI using a prepared UI artifact bundle.
///
/// This example:
/// - reads a JSON Schema from disk
/// - uses `build_ui_artifact_bundle_from_file` to build UiAst + FormSchema + LayoutNav
/// - feeds those prepared artifacts into `SchemaUI` + `TuiFrontend`
///
/// You can run it with:
///
/// ```bash
/// cargo run --example tui_artifact_demo --features tui,precompile -- [schema-path]
/// ```
///
/// If no schema path is provided, `examples/config-schema.json` is used.
fn main() -> Result<()> {
    // 1) Resolve schema path (from CLI or default example).
    let schema_arg = env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/config-schema.json".to_string());
    let schema_path = PathBuf::from(schema_arg);

    if !schema_path.exists() {
        bail!("schema path {:?} does not exist", schema_path);
    }

    let format = DocumentFormat::from_extension(&schema_path).unwrap_or(DocumentFormat::Json);

    // 2) Precompile the shared UI bundle plus TUI-specific derivatives.
    let artifact_bundle = build_ui_artifact_bundle_from_file(&schema_path, format, None)?;

    // 3) Load the raw schema value for SchemaUI.
    let contents = fs::read_to_string(&schema_path)?;
    let schema_value = parse_document_str(&contents, format)?;

    // 4) Configure UI options (tune as needed).
    let options = UiOptions::default()
        .with_auto_validate(true)
        .with_confirm_exit(true);

    // 5) Build SchemaUI with prepared artifacts.
    let ui = SchemaUI::from_schema(schema_value)
        .with_title(format!(
            "Artifact TUI demo - {:?}",
            schema_path.file_name().unwrap_or_default()
        ))
        .with_options(options.clone())
        .with_ui_artifact_bundle(artifact_bundle.clone());

    // 6) Run the TUI using the prepared TUI artifacts.
    let frontend = TuiFrontend {
        options,
        tui_artifacts: Some(artifact_bundle.tui.clone()),
    };

    let result = ui.run_with_frontend(frontend)?;

    // 7) Print the resulting JSON document on exit.
    let json = serde_json::to_string_pretty(&result)?;
    println!("{}", json);

    Ok(())
}
