use anyhow::Result;
use serde_json::Value;

use crate::core::frontend::{Frontend, FrontendContext};
use crate::tui::app::{App, UiOptions};
use crate::tui::model::form_schema_from_ui_ast;
use crate::tui::state::FormState;

/// TUI frontend implementation that consumes a prepared `FrontendContext`
/// and runs the interactive terminal UI.
#[derive(Debug)]
pub struct TuiFrontend {
    pub options: UiOptions,
}

impl Frontend for TuiFrontend {
    fn run(self, ctx: FrontendContext) -> Result<Value> {
        let FrontendContext {
            title: _,
            ui_ast,
            initial_data: _,
            schema: _,
            validator,
        } = ctx;

        let form_schema = form_schema_from_ui_ast(&ui_ast);
        let palette = self.options.component_palette();
        let form_state = FormState::from_schema_with_palette(&form_schema, palette);

        let mut app = App::new(form_state, validator, self.options);
        let result = app.run()?;
        Ok(result)
    }
}
