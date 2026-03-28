use std::{fs, path::Path};

use anyhow::Result;

use crate::io::DocumentFormat;
use crate::precompile::build_ui_ast_bundle_from_file;
use crate::tui::model::{FormSchema, form_schema_from_ui_ast};
use crate::tui::state::LayoutNavModel;
use crate::ui_ast::UiAst;

/// Build UiAst and TUI FormSchema from a schema file.
///
/// This is a pure helper intended for use in build scripts or external
/// codegen tools. It does not depend on any runtime state.
pub fn build_tui_form_schema_from_file(
    path: &Path,
    format: DocumentFormat,
) -> Result<(UiAst, FormSchema)> {
    let bundle = build_ui_ast_bundle_from_file(path, format)?;
    let form = form_schema_from_ui_ast(&bundle.ui_ast);
    Ok((bundle.ui_ast, form))
}

/// Build UiAst and a TUI LayoutNavModel from a schema file.
///
/// This is similar to [`build_tui_form_schema_from_file`], but derives the
/// navigation model used by the TUI from the intermediate UiLayout. It is
/// intended for compile-time tooling that wants to embed layout navigation
/// alongside the form schema.
pub fn build_tui_layout_nav_from_file(
    path: &Path,
    format: DocumentFormat,
) -> Result<(UiAst, LayoutNavModel)> {
    let bundle = build_ui_ast_bundle_from_file(path, format)?;
    let nav = LayoutNavModel::from_uilayout(&bundle.layout);
    Ok((bundle.ui_ast, nav))
}

/// Generate a Rust module under OUT_DIR that exposes a constructor for
/// `FormSchema` built from the given schema.
///
/// The generated module will contain a function with the given `fn_name`:
///
/// ```ignore
/// pub fn <fn_name>() -> schemaui::tui::model::FormSchema { ... }
/// ```
///
/// The implementation deserializes `FormSchema` from an embedded JSON
/// representation produced at codegen time.
pub fn generate_tui_form_schema_module(
    schema_path: &Path,
    format: DocumentFormat,
    out_module_path: &Path,
    fn_name: &str,
) -> Result<()> {
    let (_ast, form_schema) = build_tui_form_schema_from_file(schema_path, format)?;
    let json = serde_json::to_string_pretty(&form_schema)?;
    let src = format!(
        "pub fn {fn_name}() -> schemaui::tui::model::FormSchema {{\n    serde_json::from_str::<schemaui::tui::model::FormSchema>(r#\"{json}\"#).expect(\"invalid precompiled FormSchema JSON\")\n}}\n",
    );
    fs::write(out_module_path, src)?;
    Ok(())
}

/// Generate a Rust module under OUT_DIR that exposes a constructor for
/// `LayoutNavModel` built from the given schema.
///
/// The generated module will contain a function with the given `fn_name`:
///
/// ```ignore
/// pub fn <fn_name>() -> schemaui::tui::state::LayoutNavModel { ... }
/// ```
///
/// The implementation deserializes `LayoutNavModel` from an embedded JSON
/// representation produced at codegen time.
pub fn generate_tui_layout_nav_module(
    schema_path: &Path,
    format: DocumentFormat,
    out_module_path: &Path,
    fn_name: &str,
) -> Result<()> {
    let (_ast, layout_nav) = build_tui_layout_nav_from_file(schema_path, format)?;
    let json = serde_json::to_string_pretty(&layout_nav)?;
    let src = format!(
        "pub fn {fn_name}() -> schemaui::tui::state::LayoutNavModel {{\n    serde_json::from_str::<schemaui::tui::state::LayoutNavModel>(r#\"{json}\"#).expect(\"invalid precompiled LayoutNavModel JSON\")\n}}\n",
    );
    fs::write(out_module_path, src)?;
    Ok(())
}
