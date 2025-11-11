use anyhow::{Context, Result};
use jsonschema::validator_for;
use serde_json::Value;

use crate::{
    domain::parse_form_schema,
    form::FormState,
    io::{
        self, DocumentFormat,
        output::{self, OutputOptions},
    },
};

use super::{options::UiOptions, runtime::App};

#[derive(Debug)]
pub struct SchemaUI {
    schema: Value,
    title: Option<String>,
    options: UiOptions,
    output: Option<OutputOptions>,
}

impl SchemaUI {
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            title: None,
            options: UiOptions::default(),
            output: None,
        }
    }

    pub fn from_schema_str(contents: &str, format: DocumentFormat) -> Result<Self> {
        let schema = io::input::parse_document_str(contents, format)?;
        Ok(Self::new(schema))
    }

    pub fn from_data_value(value: Value) -> Self {
        let schema = io::input::schema_from_data_value(&value);
        Self::new(schema)
    }

    pub fn from_data_str(contents: &str, format: DocumentFormat) -> Result<Self> {
        let schema = io::input::schema_from_data_str(contents, format)?;
        Ok(Self::new(schema))
    }

    pub fn with_title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    pub fn with_options(mut self, options: UiOptions) -> Self {
        self.options = options;
        self
    }

    pub fn with_output(mut self, output: OutputOptions) -> Self {
        self.output = Some(output);
        self
    }

    pub fn run(self) -> Result<Value> {
        let SchemaUI {
            schema,
            title: _,
            options,
            output,
        } = self;

        let validator = validator_for(&schema).context("failed to compile JSON schema")?;
        let form_schema = parse_form_schema(&schema)?;
        let form_state = FormState::from_schema(&form_schema);

        let mut app = App::new(form_state, validator, options);
        let result = app.run()?;
        if let Some(settings) = output {
            output::emit(&result, &settings)?;
        }
        Ok(result)
    }
}
