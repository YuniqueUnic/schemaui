use std::{fs, path::Path};

use anyhow::Result;
use serde_json::Value;

use crate::io::{DocumentFormat, input::parse_document_str};
use crate::ui_ast::build_ui_ast;
use crate::ui_ast::layout::{UiLayout, build_ui_layout};

/// Build a UiLayout tree from a schema file.
///
/// This is a pure helper intended for use in build scripts or external
/// codegen tools. It relies only on the canonical UiAst visitor and the
/// internal ui_ast::layout representation.
pub fn build_ui_layout_from_file(path: &Path, format: DocumentFormat) -> Result<UiLayout> {
    let contents = fs::read_to_string(path)?;
    let schema: Value = parse_document_str(&contents, format)?;
    let ast = build_ui_ast(&schema)?;
    Ok(build_ui_layout(&ast))
}
