use anyhow::{Context, Result};
use jsonschema::validator_for;
use serde_json::Value;
use std::{sync::Arc, time::Duration};

use crate::{
    domain::parse_form_schema,
    form::FormState,
    io::{
        self, DocumentFormat,
        output::{self, OutputOptions},
    },
};

use super::{input::KeyBindingMap, keymap::KeymapStore, options::UiOptions, runtime::App};

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

    pub fn from_schema_and_data(schema: Value, defaults: Value) -> Self {
        let enriched = io::input::schema_with_defaults(&schema, &defaults);
        Self::new(enriched)
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

    pub fn with_default_data(mut self, defaults: &Value) -> Self {
        self.schema = io::input::schema_with_defaults(&self.schema, defaults);
        self
    }

    pub fn with_keymap(mut self, keymap: KeyBindingMap) -> Self {
        self.options = self.options.clone().with_keymap(keymap);
        self
    }

    pub fn with_keymap_json(mut self, json: &str) -> Result<Self> {
        let store = KeymapStore::from_json(json)?;
        self.options = self.options.clone().with_keymap_store(Arc::new(store));
        Ok(self)
    }

    pub fn with_auto_validate(mut self, enabled: bool) -> Self {
        self.options = self.options.clone().with_auto_validate(enabled);
        self
    }

    pub fn with_help(mut self, show: bool) -> Self {
        self.options = self.options.clone().with_help(show);
        self
    }

    pub fn with_confirm_exit(mut self, confirm: bool) -> Self {
        self.options = self.options.clone().with_confirm_exit(confirm);
        self
    }

    pub fn with_tick_rate(mut self, tick_rate: Duration) -> Self {
        self.options = self.options.clone().with_tick_rate(tick_rate);
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
