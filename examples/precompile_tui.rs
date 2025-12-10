use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};
use schemaui::precompile::tui::build_tui_form_schema_from_file;
use schemaui::{DocumentFormat, SchemaUI, TuiFrontend, UiOptions, parse_document_str};

/// Run the TUI using precompiled UiAst + FormSchema artifacts.
///
/// This example:
/// - reads a JSON Schema from disk
/// - uses `precompile::tui::build_tui_form_schema_from_file` to build UiAst + FormSchema
/// - feeds those precompiled artifacts into `SchemaUI` + `TuiFrontend`
///
/// You can run it with:
///
/// ```bash
/// cargo run --example precompile_tui --features tui,precompile -- [schema-path]
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

    // 2) Precompile UiAst + FormSchema from the schema file.
    let (precompiled_ast, precompiled_form_schema) =
        build_tui_form_schema_from_file(&schema_path, format)?;

    // 3) Load the raw schema value for SchemaUI.
    let contents = fs::read_to_string(&schema_path)?;
    let schema_value = parse_document_str(&contents, format)?;

    // 4) Configure UI options (tune as needed).
    let options = UiOptions::default()
        .with_auto_validate(true)
        .with_confirm_exit(true);

    // 5) Build SchemaUI with precompiled artifacts.
    let ui = SchemaUI::new(schema_value)
        .with_title(format!(
            "Precompiled TUI demo - {:?}",
            schema_path.file_name().unwrap_or_default()
        ))
        .with_options(options.clone())
        .with_precompiled_ui_ast(precompiled_ast)
        .with_precompiled_form_schema(precompiled_form_schema.clone());

    // 6) Run the TUI using the precompiled FormSchema.
    let frontend = TuiFrontend {
        options,
        precompiled_form_schema: Some(precompiled_form_schema),
    };

    let result = ui.run_with_frontend(frontend)?;

    // 7) Print the resulting JSON document on exit.
    let json = serde_json::to_string_pretty(&result)?;
    println!("{}", json);

    Ok(())
}
