use std::env;
use std::path::PathBuf;

use anyhow::{Result, bail};
use schemaui::DocumentFormat;
use schemaui::compile_time::layout::build_ui_layout_from_file;

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args.len() > 2 {
        eprintln!("Usage: ui_layout_dump <schema-path> [out-path]\n");
        eprintln!("Examples:");
        eprintln!("  ui_layout_dump examples/complex.schema.json");
        eprintln!("  ui_layout_dump examples/complex.schema.json layout.json");
        bail!("invalid arguments");
    }

    let schema_path = PathBuf::from(&args[0]);
    let format = infer_format(&schema_path);

    let layout = build_ui_layout_from_file(&schema_path, format)?;
    let json = serde_json::to_string_pretty(&layout)?;

    if let Some(out) = args.get(1) {
        std::fs::write(out, json)?;
    } else {
        println!("{}", json);
    }

    Ok(())
}

fn infer_format(path: &PathBuf) -> DocumentFormat {
    DocumentFormat::from_extension(path).unwrap_or(DocumentFormat::Json)
}
