use anyhow::Result;
use serde_json::Value;
use std::{borrow::Cow, sync::Arc, time::Duration};

use crate::core::pipeline::SchemaPipeline;
use crate::io::{
    self, DocumentFormat,
    output::{self, OutputOptions},
};

use super::{input::KeyBindingMap, keymap::KeymapStore, options::UiOptions};
use crate::core::frontend::Frontend;
use crate::precompile::{TuiArtifacts, UiArtifactBundle};
use crate::tui::session::TuiFrontend;
use crate::tui::state::field::components::ComponentPalette;
use crate::ui_ast::{UiAst, UiAstBundle};
#[cfg(feature = "web")]
use crate::web::{frontend::WebFrontend, session::ServeOptions as WebServeOptions};

#[derive(Debug)]
pub struct SchemaUI {
    schema: Value,
    title: Option<String>,
    options: UiOptions,
    output: Option<OutputOptions>,
    initial_data: Option<Value>,
    ui_ast: Option<UiAst>,
    ui_bundle: Option<UiAstBundle>,
    ui_artifact_bundle: Option<UiArtifactBundle>,
    tui_artifacts: Option<TuiArtifacts>,
}

impl SchemaUI {
    pub fn new(schema: Value) -> Self {
        Self {
            schema,
            title: None,
            options: UiOptions::default(),
            output: None,
            initial_data: None,
            ui_ast: None,
            ui_bundle: None,
            ui_artifact_bundle: None,
            tui_artifacts: None,
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
        let mut ui = Self::new(schema);
        ui.initial_data = Some(defaults);
        ui
    }

    /// Provide a prepared UiAst so the runtime pipeline can skip rebuilding it
    /// from the schema.
    pub fn with_ui_ast(mut self, ast: UiAst) -> Self {
        self.ui_ast = Some(ast);
        self
    }

    /// Provide a prepared bundle containing shared UI artifacts.
    pub fn with_ui_bundle(mut self, bundle: UiAstBundle) -> Self {
        self.ui_bundle = Some(bundle);
        self
    }

    /// Provide a prepared UI artifact bundle, including shared UI and
    /// frontend-specific derived structures.
    pub fn with_ui_artifact_bundle(mut self, bundle: UiArtifactBundle) -> Self {
        self.ui_artifact_bundle = Some(bundle);
        self
    }

    /// Provide prepared TUI artifacts. When set, the TUI frontend will use
    /// these instead of deriving TUI-specific structures from the UiAst at
    /// runtime.
    pub fn with_tui_artifacts(mut self, artifacts: TuiArtifacts) -> Self {
        self.tui_artifacts = Some(artifacts);
        self
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
        self.initial_data = Some(defaults.clone());
        self
    }

    /// Expose the current UI options so callers can build a frontend
    /// (e.g. TuiFrontend) configured consistently with this builder.
    pub fn options(&self) -> &UiOptions {
        &self.options
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

    /// Run using the default TUI frontend.
    ///
    /// This is equivalent to `self.run_tui()` and keeps backward-compatible
    /// semantics: the crate defaults to TUI mode.
    pub fn run(self) -> Result<Value> {
        self.run_tui()
    }

    /// Run explicitly in TUI mode, using the options configured on this
    /// `SchemaUI` builder.
    pub fn run_tui(self) -> Result<Value> {
        let options = self.options.clone();
        let tui_artifacts = self.tui_artifacts.clone().or_else(|| {
            self.ui_artifact_bundle
                .as_ref()
                .map(|bundle| bundle.tui.clone())
        });
        self.run_with_frontend(TuiFrontend {
            options,
            tui_artifacts,
        })
    }

    /// Run in Web mode, using the given serve options to configure the
    /// temporary HTTP server.
    #[cfg(feature = "web")]
    pub fn run_web(self, serve: WebServeOptions) -> Result<Value> {
        self.run_with_frontend(WebFrontend { serve })
    }

    pub fn run_with_frontend<F>(self, frontend: F) -> Result<Value>
    where
        F: Frontend,
    {
        let SchemaUI {
            schema,
            title,
            options: _,
            output,
            initial_data,
            ui_ast,
            ui_bundle,
            ui_artifact_bundle,
            tui_artifacts: _,
        } = self;

        let ui_bundle = ui_artifact_bundle.map(|bundle| bundle.ui).or(ui_bundle);
        let pipeline = SchemaPipeline::new(schema)
            .with_title(title)
            .with_defaults(initial_data)
            .with_prepared_ui_ast(ui_ast)
            .with_prepared_ui_bundle(ui_bundle);
        let result = pipeline.run_with_frontend(frontend)?;
        if let Some(settings) = output {
            output::emit(&result, &settings)?;
        }
        Ok(result)
    }
}
