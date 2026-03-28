use anyhow::Result;
use serde_json::Value;

use crate::core::frontend::{Frontend, FrontendContext};
use crate::tui::app::{App, UiOptions};
use crate::tui::model::{FormSchema, form_schema_from_ui_ast};
use crate::tui::state::{FormState, LayoutNavModel};
use crate::ui_ast::{UiAst, UiLayout};

/// TUI frontend implementation that consumes a prepared `FrontendContext`
/// and runs the interactive terminal UI.
#[derive(Debug)]
pub struct TuiFrontend {
    pub options: UiOptions,
    pub precompiled_form_schema: Option<FormSchema>,
    pub precompiled_layout_nav: Option<LayoutNavModel>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedTuiArtifacts {
    pub form_schema: FormSchema,
    pub layout_nav: LayoutNavModel,
}

pub(crate) fn resolve_tui_artifacts(
    ui_ast: &UiAst,
    layout: &UiLayout,
    precompiled_form_schema: Option<FormSchema>,
    precompiled_layout_nav: Option<LayoutNavModel>,
) -> ResolvedTuiArtifacts {
    let form_schema = precompiled_form_schema.unwrap_or_else(|| form_schema_from_ui_ast(ui_ast));
    let layout_nav =
        precompiled_layout_nav.unwrap_or_else(|| LayoutNavModel::from_uilayout(layout));
    ResolvedTuiArtifacts {
        form_schema,
        layout_nav,
    }
}

impl Frontend for TuiFrontend {
    fn run(self, ctx: FrontendContext) -> Result<Value> {
        let TuiFrontend {
            options,
            precompiled_form_schema,
            precompiled_layout_nav,
        } = self;

        let FrontendContext {
            title: _,
            ui_ast,
            layout,
            initial_data: _,
            schema: _,
            validator,
        } = ctx;

        let resolved = resolve_tui_artifacts(
            &ui_ast,
            &layout,
            precompiled_form_schema,
            precompiled_layout_nav,
        );

        let palette = options.component_palette();
        let mut form_state = FormState::from_schema_with_palette(&resolved.form_schema, palette);
        form_state.set_layout_nav(resolved.layout_nav);

        let mut app = App::new(form_state, validator, options);
        let result = app.run()?;
        Ok(result)
    }
}
