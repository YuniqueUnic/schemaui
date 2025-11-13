use anyhow::{Context, Result};
use jsonschema::validator_for;
use serde_json::Value;
use std::{borrow::Cow, sync::Arc, time::Duration};

use crate::{
    domain::parse_form_schema,
    form::FormState,
    io::{
        self, DocumentFormat,
        output::{self, OutputOptions},
    },
};

use super::{input::KeyBindingMap, keymap::KeymapStore, options::UiOptions, runtime::App};
use crate::form::field::components::ComponentPalette;

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

    pub fn with_component_palette(mut self, palette: ComponentPalette) -> Self {
        self.options = self.options.clone().with_component_palette(palette);
        self
    }

    pub fn with_integer_step(mut self, step: i64) -> Self {
        self.options = self.options.clone().with_integer_step(step);
        self
    }

    pub fn with_integer_fast_step(mut self, step: i64) -> Self {
        self.options = self.options.clone().with_integer_fast_step(step);
        self
    }

    pub fn with_float_step(mut self, step: f64) -> Self {
        self.options = self.options.clone().with_float_step(step);
        self
    }

    pub fn with_float_fast_step(mut self, step: f64) -> Self {
        self.options = self.options.clone().with_float_fast_step(step);
        self
    }

    pub fn with_bool_labels(
        mut self,
        true_label: impl Into<Cow<'static, str>>,
        false_label: impl Into<Cow<'static, str>>,
    ) -> Self {
        self.options = self
            .options
            .clone()
            .with_bool_labels(true_label, false_label);
        self
    }

    pub fn with_bool_toggle_arrows(mut self, enabled: bool) -> Self {
        self.options = self.options.clone().with_bool_toggle_arrows(enabled);
        self
    }

    pub fn with_bool_toggle_space(mut self, enabled: bool) -> Self {
        self.options = self.options.clone().with_bool_toggle_space(enabled);
        self
    }

    pub fn with_enum_wrap(mut self, wrap: bool) -> Self {
        self.options = self.options.clone().with_enum_wrap(wrap);
        self
    }

    pub fn with_overlay_instructions(mut self, instructions: impl Into<Cow<'static, str>>) -> Self {
        self.options = self.options.clone().with_overlay_instructions(instructions);
        self
    }

    pub fn with_list_hint(mut self, hint: impl Into<Cow<'static, str>>) -> Self {
        self.options = self.options.clone().with_list_hint(hint);
        self
    }

    pub fn with_composite_single_hint(mut self, hint: impl Into<Cow<'static, str>>) -> Self {
        self.options = self.options.clone().with_composite_single_hint(hint);
        self
    }

    pub fn with_composite_multi_hint(mut self, hint: impl Into<Cow<'static, str>>) -> Self {
        self.options = self.options.clone().with_composite_multi_hint(hint);
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
        let palette = options.component_palette();
        let form_state = FormState::from_schema_with_palette(&form_schema, palette);

        let mut app = App::new(form_state, validator, options);
        let result = app.run()?;
        if let Some(settings) = output {
            output::emit(&result, &settings)?;
        }
        Ok(result)
    }
}
