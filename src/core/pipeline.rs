use anyhow::Result;
use jsonschema::validator_for;
use serde_json::{Map, Value};

use crate::core::frontend::{Frontend, FrontendContext};
use crate::core::io::input::schema_with_defaults;
use crate::core::ui_ast::build_ui_ast;

/// Core pipeline for preparing a `FrontendContext` from a base JSON Schema,
/// optional title, and optional default data.
///
/// This is the shared part of the flow:
///
/// ```text
/// io::input -> (schema, defaults) -> enriched schema -> ui_ast -> FrontendContext
/// ```
#[derive(Debug)]
pub struct SchemaPipeline {
    schema: Value,
    title: Option<String>,
    defaults: Option<Value>,
}

impl SchemaPipeline {
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            title: None,
            defaults: None,
        }
    }

    pub fn with_title(mut self, title: Option<String>) -> Self {
        self.title = title;
        self
    }

    pub fn with_defaults(mut self, defaults: Option<Value>) -> Self {
        self.defaults = defaults;
        self
    }

    fn build_frontend_context(self) -> Result<FrontendContext> {
        let SchemaPipeline {
            schema,
            title,
            defaults,
        } = self;

        let data = defaults.unwrap_or_else(|| Value::Object(Map::new()));
        let enriched = schema_with_defaults(&schema, &data);

        let validator = validator_for(&enriched)?;
        let ui_ast = build_ui_ast(&enriched)?;

        Ok(FrontendContext {
            title,
            ui_ast,
            initial_data: data,
            schema: enriched,
            validator,
        })
    }

    pub fn run_with_frontend<F>(self, frontend: F) -> Result<Value>
    where
        F: Frontend,
    {
        let ctx = self.build_frontend_context()?;
        frontend.run(ctx)
    }
}
