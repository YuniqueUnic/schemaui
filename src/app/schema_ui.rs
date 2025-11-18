use anyhow::{Context, Result};
use serde_json::Value;
use std::{borrow::Cow, sync::Arc, time::Duration};

use crate::io::{
    self, DocumentFormat,
    output::{self, OutputOptions},
};

use super::{input::KeyBindingMap, keymap::KeymapStore, options::UiOptions};
use crate::form::field::components::ComponentPalette;
use crate::tui::session::TuiSessionConfig;

#[cfg(feature = "web")]
use crate::web::session::ServeOptions as WebServeOptions;

#[derive(Debug, Clone, Copy)]
pub enum UiFrontend {
    Tui,
    #[cfg(feature = "web")]
    Web(WebServeOptions),
}

#[derive(Debug)]
pub struct SchemaUI {
    schema: Value,
    title: Option<String>,
    options: UiOptions,
    output: Option<OutputOptions>,
    initial_data: Option<Value>,
    frontend: UiFrontend,
}

impl SchemaUI {
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            title: None,
            options: UiOptions::default(),
            output: None,
            initial_data: None,
            frontend: UiFrontend::Tui,
        }
    }

    pub fn from_schema_str(contents: &str, format: DocumentFormat) -> Result<Self> {
        let schema = io::input::parse_document_str(contents, format)?;
        Ok(Self::new(schema))
    }

    pub fn from_data_value(value: Value) -> Self {
        let schema = io::input::schema_from_data_value(&value);
        let mut ui = Self::new(schema);
        ui.initial_data = Some(value);
        ui
    }

    pub fn from_data_str(contents: &str, format: DocumentFormat) -> Result<Self> {
        let value = io::input::parse_document_str(contents, format)?;
        Ok(Self::from_data_value(value))
    }

    pub fn from_schema_and_data(schema: Value, defaults: Value) -> Self {
        let enriched = io::input::schema_with_defaults(&schema, &defaults);
        let mut ui = Self::new(enriched);
        ui.initial_data = Some(defaults);
        ui
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
        self.initial_data = Some(defaults.clone());
        self
    }

    pub fn with_frontend(mut self, frontend: UiFrontend) -> Self {
        self.frontend = frontend;
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
        match self.frontend {
            UiFrontend::Tui => self.run_tui(),
            #[cfg(feature = "web")]
            UiFrontend::Web(options) => self.run_web(options),
        }
    }

    fn run_tui(self) -> Result<Value> {
        let SchemaUI {
            schema,
            title: _,
            options,
            output,
            initial_data: _,
            frontend: _,
        } = self;

        let config = TuiSessionConfig::new(schema, options);
        let session = config.build()?;
        session.run(output)
    }

    #[cfg(feature = "web")]
    fn run_web(self, serve: WebServeOptions) -> Result<Value> {
        use crate::web::session::{WebSessionBuilder, bind_session};

        let SchemaUI {
            schema,
            title,
            options: _,
            output,
            initial_data,
            frontend: _,
        } = self;

        let mut builder = WebSessionBuilder::new(schema);
        if let Some(title) = title {
            builder = builder.with_title(title);
        }
        if let Some(data) = initial_data {
            builder = builder.with_initial_data(data);
        }
        let config = builder.build()?;

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("failed to initialize tokio runtime")?;

        let _guard = runtime.enter();

        let value = runtime.block_on(async move {
            let bound = bind_session(config, serve)
                .await
                .context("failed to bind web session")?;
            let addr = bound.local_addr();
            eprintln!("schemaui web UI available at http://{addr}/");
            eprintln!("Press Ctrl+C to abort the session.");
            bound.run().await.context("web UI session failed")
        })?;

        runtime.shutdown_background();

        if let Some(settings) = output {
            output::emit(&value, &settings)?;
        }

        Ok(value)
    }
}
