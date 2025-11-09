use anyhow::{Context, Result};
use jsonschema::validator_for;
use serde_json::Value;

use crate::{domain::parse_form_schema, form::FormState};

use super::{options::UiOptions, runtime::App};

#[derive(Debug)]
pub struct SchemaUI {
    schema: Value,
    title: Option<String>,
    options: UiOptions,
}

impl SchemaUI {
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            title: None,
            options: UiOptions::default(),
        }
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_options(mut self, options: UiOptions) -> Self {
        self.options = options;
        self
    }

    pub fn run(self) -> Result<Value> {
        let SchemaUI {
            schema,
            title: _,
            options,
        } = self;

        let validator = validator_for(&schema).context("failed to compile JSON schema")?;
        let form_schema = parse_form_schema(&schema)?;
        let form_state = FormState::from_schema(&form_schema);

        let mut app = App::new(form_state, validator, options);
        app.run()
    }
}
