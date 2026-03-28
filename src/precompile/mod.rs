use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::io::DocumentFormat;
use crate::io::input::parse_document_str;
use crate::ui_ast::{UiAst, UiAstBundle, build_ui_ast_bundle};

pub mod defaults;
pub mod layout;

#[cfg(feature = "tui")]
pub mod tui;

#[cfg(feature = "web")]
pub mod web;

/// Read a schema file, parse it as JSON/YAML/TOML, and build a UiAst.
pub fn build_ui_ast_from_file(path: &Path, format: DocumentFormat) -> Result<UiAst> {
    Ok(build_ui_ast_bundle_from_file(path, format)?.ui_ast)
}

/// Read a schema file, parse it as JSON/YAML/TOML, and build a shared UI
/// artifact bundle.
pub fn build_ui_ast_bundle_from_file(path: &Path, format: DocumentFormat) -> Result<UiAstBundle> {
    let contents = fs::read_to_string(path)?;
    let schema: Value = parse_document_str(&contents, format)?;
    // For compile-time we typically do not apply data-driven defaults.
    build_ui_ast_bundle(&schema)
}

/// Serialize a UiAst value to pretty-printed JSON.
pub fn ui_ast_to_json(ast: &UiAst) -> Result<String> {
    Ok(serde_json::to_string_pretty(ast)?)
}

/// Generate a Rust module under OUT_DIR that exposes a UiAst JSON constant.
///
/// The generated module will contain:
/// `pub const <const_name>: &str = r#"<UiAst-json>"#;`
pub fn generate_ui_ast_rust_module(
    schema_path: &Path,
    format: DocumentFormat,
    out_module_path: &Path,
    const_name: &str,
) -> Result<()> {
    let ast = build_ui_ast_from_file(schema_path, format)?;
    let json = ui_ast_to_json(&ast)?;
    let src = format!(
        "pub const {name}: &str = r#\"{json}\"#;\n",
        name = const_name,
        json = json,
    );
    fs::write(out_module_path, src)?;
    Ok(())
}

/// Decode a UiAst from a JSON string produced by `ui_ast_to_json`.
pub fn decode_ui_ast(json: &str) -> Result<UiAst> {
    Ok(serde_json::from_str(json)?)
}

pub fn ui_ast_bundle_to_json(bundle: &UiAstBundle) -> Result<String> {
    Ok(serde_json::to_string_pretty(bundle)?)
}

pub fn decode_ui_ast_bundle(json: &str) -> Result<UiAstBundle> {
    Ok(serde_json::from_str(json)?)
}
