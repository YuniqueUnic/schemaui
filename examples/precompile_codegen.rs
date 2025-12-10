use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};
use schemaui::DocumentFormat;
use schemaui::precompile::tui::{generate_tui_form_schema_module, generate_tui_layout_nav_module};

/// Codegen-style precompile tool for TUI artifacts.
///
/// This example reads a JSON Schema file and generates two Rust modules:
///
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
///   target/precompiled_modules
/// ```
///
/// If you omit arguments, it defaults to:
///
/// - schema: `examples/config-schema.json`
/// - out-dir: `target/precompiled_modules`
fn main() -> Result<()> {
    // 1) Parse CLI arguments or fall back to sensible defaults.
    let schema_arg = env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/config-schema.json".to_string());
    let out_dir_arg = env::args()
        .nth(2)
        .unwrap_or_else(|| "target/precompiled_modules".to_string());

    let schema_path = PathBuf::from(&schema_arg);
    if !schema_path.exists() {
        bail!("schema path {:?} does not exist", schema_path);
    }

    let out_dir = PathBuf::from(&out_dir_arg);
    fs::create_dir_all(&out_dir)?;

    let format = DocumentFormat::from_extension(&schema_path).unwrap_or(DocumentFormat::Json);

    // 2) Decide output module paths and function names.
    let form_module_path = out_dir.join("precompiled_form_schema.rs");
    let layout_module_path = out_dir.join("precompiled_layout_nav.rs");

    let form_fn_name = "precompiled_form_schema";
    let layout_fn_name = "precompiled_layout_nav";

    // 3) Generate the Rust modules using the precompile helpers.
    generate_tui_form_schema_module(&schema_path, format, &form_module_path, form_fn_name)?;

    generate_tui_layout_nav_module(&schema_path, format, &layout_module_path, layout_fn_name)?;

    // 4) Print a short usage guide for the generated modules.
    println!("Generated precompiled modules:\n");
    println!("  FormSchema:      {:?}", form_module_path);
    println!("  LayoutNavModel:  {:?}\n", layout_module_path);

    println!("Each module defines a function you can call from your app:\n");
    println!(
        "  pub fn {}() -> schemaui::tui::model::FormSchema",
        form_fn_name
    );
    println!(
        "  pub fn {}() -> schemaui::tui::state::LayoutNavModel\n",
        layout_fn_name
    );

    println!("Example include! usage in your crate (pseudo-code):\n");
    println!("  // in some module of your binary or library\n");
    println!(
        "  include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}/precompiled_form_schema.rs\"));",
        out_dir.display()
    );
    println!(
        "  include!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/{}/precompiled_layout_nav.rs\"));\n",
        out_dir.display()
    );

    println!("Then you can call the generated functions, for example:\n");
    println!("  let form = precompiled_form_schema();");
    println!("  let layout_nav = precompiled_layout_nav();\n");

    Ok(())
}
