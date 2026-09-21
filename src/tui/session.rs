use anyhow::Result;

use crate::core::frontend::{Frontend, FrontendContext, SessionOutcome};
use crate::precompile::TuiArtifacts;
use crate::tui::app::{App, UiOptions};
use crate::tui::model::form_schema_from_ui_ast;
use crate::tui::state::{FormState, LayoutNavModel};
use crate::ui_ast::{UiAst, UiLayout};

/// TUI frontend implementation that consumes a prepared `FrontendContext`
/// and runs the interactive terminal UI.
#[derive(Debug)]
pub struct TuiFrontend {
    pub options: UiOptions,
    pub tui_artifacts: Option<TuiArtifacts>,
}

pub(crate) fn resolve_tui_artifacts(
    ui_ast: &UiAst,
    layout: &UiLayout,
    tui_artifacts: Option<TuiArtifacts>,
) -> TuiArtifacts {
    tui_artifacts.unwrap_or_else(|| TuiArtifacts {
        form_schema: form_schema_from_ui_ast(ui_ast),
        layout_nav: LayoutNavModel::from_uilayout(layout),
    })
}

impl Frontend for TuiFrontend {
    fn run(self, ctx: FrontendContext) -> Result<SessionOutcome> {
        let TuiFrontend {
            options,
            tui_artifacts,
        } = self;

        let FrontendContext {
            title,
            description: _,
            ui_ast,
            layout,
            initial_data: _,
            schema: _,
            validator,
            deadline,
            draft,
        } = ctx;

        let resolved = resolve_tui_artifacts(&ui_ast, &layout, tui_artifacts);

        let palette = options.component_palette();
        let mut form_state = FormState::from_schema_with_palette(&resolved.form_schema, palette);
        form_state.set_layout_nav(resolved.layout_nav);

        let mut app = App::new(form_state, validator, options);
        app.set_session_title(title);
        app.set_deadline(deadline);
        app.set_draft(draft.clone());

        let outcome = app.run()?;
        // The session produced a value, so the draft has nothing left to
        // protect. A timeout leaves it exactly where it is.
        if let (SessionOutcome::Completed(_), Some(draft)) = (&outcome, draft.as_ref()) {
            draft.store.discard();
        }
        Ok(outcome)
    }
}
