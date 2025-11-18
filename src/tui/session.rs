use anyhow::{Context, Result};
use jsonschema::{Validator, validator_for};
use serde_json::Value;

use crate::{
    app::{App, UiOptions},
    form::FormState,
    io::output::{self, OutputOptions},
    ui_ast::{build_ui_ast, form_schema::form_schema_from_ui_ast},
};

#[derive(Debug)]
pub(crate) struct TuiSessionConfig {
    pub(crate) schema: Value,
    pub(crate) options: UiOptions,
}

impl TuiSessionConfig {
    pub(crate) fn new(schema: Value, options: UiOptions) -> Self {
        Self { schema, options }
    }

    pub(crate) fn build(self) -> Result<TuiSession> {
        let TuiSessionConfig { schema, options } = self;

        let validator = validator_for(&schema).context("failed to compile JSON schema")?;
        let ui_ast = build_ui_ast(&schema)?;
        let form_schema = form_schema_from_ui_ast(&ui_ast);
        let palette = options.component_palette();
        let form_state = FormState::from_schema_with_palette(&form_schema, palette);

        Ok(TuiSession {
            form_state,
            validator,
            options,
        })
    }
}

#[derive(Debug)]
pub(crate) struct TuiSession {
    form_state: FormState,
    validator: Validator,
    options: UiOptions,
}

impl TuiSession {
    pub(crate) fn run(self, output: Option<OutputOptions>) -> Result<Value> {
        let TuiSession {
            form_state,
            validator,
            options,
        } = self;

        let mut app = App::new(form_state, validator, options);
        let result = app.run()?;
        if let Some(settings) = output {
            output::emit(&result, &settings)?;
        }
        Ok(result)
    }
}
