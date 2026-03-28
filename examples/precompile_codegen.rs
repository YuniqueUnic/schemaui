use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};
use schemaui::DocumentFormat;
use schemaui::precompile::tui::{
    generate_tui_artifacts_module, generate_tui_form_schema_module, generate_tui_layout_nav_module,
};

/// Codegen-style precompile tool for TUI artifacts.
///
/// This example reads a JSON Schema file and generates three Rust modules:
///
/// - a `TuiArtifacts` constructor function
/// - a `FormSchema` constructor function
/// - a `LayoutNavModel` constructor function
///
/// The generated files can be `include!`-d into your application.
///
/// Usage:
///
/// ```bash
/// # From the schemaui crate root
/// cargo run --example precompile_codegen --features tui,precompile -- \
///   examples/config-schema.json \
///   target/precompiled_modules \
///   examples/config-defaults.json
/// ```
///
/// If you omit arguments, it defaults to:
///
/// - schema: `examples/config-schema.json`
/// - out-dir: `target/precompiled_modules`
/// - defaults: none
fn main() -> Result<()> {
    // 1) Parse CLI arguments or fall back to sensible defaults.
    let schema_arg = env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/config-schema.json".to_string());
    let out_dir_arg = env::args()
        .nth(2)
        .unwrap_or_else(|| "target/precompiled_modules".to_string());
    let defaults_arg = env::args().nth(3);

    let schema_path = PathBuf::from(&schema_arg);
    if !schema_path.exists() {
        bail!("schema path {:?} does not exist", schema_path);
    }

    let out_dir = PathBuf::from(&out_dir_arg);
    fs::create_dir_all(&out_dir)?;
    let defaults_path = defaults_arg.map(PathBuf::from);
    if let Some(path) = defaults_path.as_ref()
        && !path.exists()
    {
        bail!("defaults path {:?} does not exist", path);
    }

    let format = DocumentFormat::from_extension(&schema_path).unwrap_or(DocumentFormat::Json);

    // 2) Decide output module paths and function names.
    let tui_module_path = out_dir.join("tui_artifacts.rs");
    let form_module_path = out_dir.join("precompiled_form_schema.rs");
    let layout_module_path = out_dir.join("precompiled_layout_nav.rs");

    let tui_fn_name = "tui_artifacts";
    let form_fn_name = "precompiled_form_schema";
    let layout_fn_name = "precompiled_layout_nav";

    // 3) Generate the Rust modules using the precompile helpers.
    generate_tui_artifacts_module(
        &schema_path,
        format,
        defaults_path.as_deref(),
        &tui_module_path,
        tui_fn_name,
    )?;
    generate_tui_form_schema_module(
        &schema_path,
        format,
        defaults_path.as_deref(),
        &form_module_path,
        form_fn_name,
    )?;

    generate_tui_layout_nav_module(
        &schema_path,
        format,
        defaults_path.as_deref(),
        &layout_module_path,
        layout_fn_name,
    )?;

    // 4) Print a short usage guide for the generated modules.
    println!("Generated precompiled modules:\n");
    println!("  TuiArtifacts:   {:?}", tui_module_path);
    println!("  FormSchema:      {:?}", form_module_path);
    println!("  LayoutNavModel:  {:?}\n", layout_module_path);

    println!("Each module defines a function you can call from your app:\n");
    println!("  pub fn {}() -> schemaui::TuiArtifacts", tui_fn_name);
    println!("  pub fn {}() -> schemaui::FormSchema", form_fn_name);
    println!(
        "  pub fn {}() -> schemaui::LayoutNavModel\n",
        layout_fn_name
    );

    println!("Example include! usage in your crate (pseudo-code):\n");
    println!("  // in some module of your binary or library\n");
    println!(
        "  include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}/tui_artifacts.rs\"));",
        out_dir.display()
    );
    println!(
        "  include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}/precompiled_form_schema.rs\"));",
        out_dir.display()
    );
    println!(
        "  include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}/precompiled_layout_nav.rs\"));\n",
        out_dir.display()
    );

    println!("Then you can call the generated functions, for example:\n");
    println!("  let tui = tui_artifacts();");
    println!("  let form = precompiled_form_schema();");
    println!("  let layout_nav = precompiled_layout_nav();\n");

    Ok(())
}
