use std::{fs, path::PathBuf};

use anyhow::Result;
use serde::Serialize;
use serde_json::{Value, json};

use crate::core::frontend::{Frontend, FrontendContext};
use crate::core::pipeline::SchemaPipeline;
use crate::io::{DocumentFormat, input::parse_document_str, input::schema_with_defaults};
use crate::ui_ast::{UiAst, UiAstBundle, UiLayout, build_ui_ast_bundle};

fn schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("schemas")
        .join("test-comprehensive.schema.json")
}

fn schema_value() -> Value {
    let path = schema_path();
    let contents = fs::read_to_string(path).expect("schema file readable");
    parse_document_str(&contents, DocumentFormat::Json).expect("schema parses")
}

fn defaults_value() -> Value {
    json!({
        "simpleTypes": {
            "text": "hello from defaults",
            "number": 7,
            "toggle": true,
            "dropdown": "option2"
        }
    })
}

#[derive(Serialize)]
struct CapturedFrontendContext {
    ui_ast: UiAst,
    layout: UiLayout,
    schema: Value,
    initial_data: Value,
}

#[derive(Debug)]
struct CaptureFrontend;

impl Frontend for CaptureFrontend {
    fn run(self, ctx: FrontendContext) -> Result<Value> {
        Ok(serde_json::to_value(CapturedFrontendContext {
            ui_ast: ctx.ui_ast,
            layout: ctx.layout,
            schema: ctx.schema,
            initial_data: ctx.initial_data,
        })?)
    }
}

fn capture_pipeline_output(schema: Value, defaults: Value, bundle: Option<UiAstBundle>) -> Value {
    let pipeline = SchemaPipeline::new(schema)
        .with_defaults(Some(defaults))
        .with_precompiled_ui_bundle(bundle);
    pipeline
        .run_with_frontend(CaptureFrontend)
        .expect("pipeline capture succeeds")
}

#[test]
fn schema_pipeline_with_precompiled_ui_bundle_matches_runtime_context() {
    let schema = schema_value();
    let defaults = defaults_value();
    let enriched = schema_with_defaults(&schema, &defaults);
    let bundle = build_ui_ast_bundle(&enriched).expect("build precompiled bundle");

    let runtime = capture_pipeline_output(schema.clone(), defaults.clone(), None);
    let precompiled = capture_pipeline_output(schema, defaults, Some(bundle));

    assert_eq!(runtime, precompiled);
}
