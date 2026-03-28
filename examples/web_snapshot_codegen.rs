use std::env;
use std::fs;
use std::path::PathBuf;

use anyhow::{Result, bail};
use schemaui::DocumentFormat;
use schemaui::precompile::web::{
    build_session_snapshot_from_files, write_session_snapshot_json,
    write_session_snapshot_ts_module,
};

/// Web precompile snapshot example.
///
/// This example generates a static Web SessionResponse snapshot from a schema
/// (and optional defaults) using the `precompile::web` helpers. It writes:
///
/// - `session_snapshot.json` – raw JSON payload for `/api/session`
/// - `session_snapshot.ts`   – TypeScript module exporting a `SessionResponse`
///
/// Usage (from the schemaui crate root):
///
/// ```bash
/// # Use defaults: schema=examples/config-schema.json, out-dir=target/web_snapshots
/// cargo run --example web_snapshot_codegen --features web,precompile
///
/// # Or specify schema, defaults, and output directory explicitly
/// cargo run --example web_snapshot_codegen --features web,precompile -- \
///   examples/config-schema.json \
///   examples/config-defaults.json \
///   target/web_snapshots
/// ```
fn main() -> Result<()> {
    // 1) Parse CLI args with sensible defaults.
    let schema_arg = env::args()
        .nth(1)
        .unwrap_or_else(|| "examples/config-schema.json".to_string());
    let defaults_arg = env::args().nth(2);
    let out_dir_arg = env::args()
        .nth(3)
        .unwrap_or_else(|| "target/web_snapshots".to_string());

    let schema_path = PathBuf::from(&schema_arg);
    if !schema_path.exists() {
        bail!("schema path {:?} does not exist", schema_path);
    }

    let defaults_path = defaults_arg.map(PathBuf::from);
    if let Some(ref defaults) = defaults_path
        && !defaults.exists()
    {
        bail!("defaults path {:?} does not exist", defaults);
    }

    let out_dir = PathBuf::from(&out_dir_arg);
    fs::create_dir_all(&out_dir)?;

    let format = DocumentFormat::from_extension(&schema_path).unwrap_or(DocumentFormat::Json);

    // 2) Build a SessionResponse snapshot from schema + defaults.
    let snapshot =
        build_session_snapshot_from_files(&schema_path, format, defaults_path.as_deref())?;

    // 3) Write JSON and TS representations for consumption by the Web UI.
    let json_out = out_dir.join("session_snapshot.json");
    let ts_out = out_dir.join("session_snapshot.ts");

    write_session_snapshot_json(&snapshot, &json_out)?;
    write_session_snapshot_ts_module(&snapshot, &ts_out, "SessionSnapshot")?;

    println!("Generated Web snapshot artifacts:\n");
    println!("  JSON:      {:?}", json_out);
    println!("  TypeScript: {:?}\n", ts_out);

    println!("You can serve the JSON snapshot as /api/session in a static setup,\n");
    println!("or import the TS module directly in your SPA bundle, e.g.:\n");
    println!("  import {{ SessionSnapshot }} from './session_snapshot';");

    Ok(())
}
