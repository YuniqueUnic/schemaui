use anyhow::{Context, Result};
use std::sync::Arc;

use crate::core::frontend::{Frontend, FrontendContext, SessionOutcome, format_budget};
use crate::rich::collect_assets;

use super::session::{ServeOptions, WebSessionConfig, bind_session};

/// Web frontend implementation that consumes a prepared `FrontendContext`
/// and runs the browser-based UI via a temporary HTTP server.
#[derive(Debug, Clone)]
pub struct WebFrontend {
    pub serve: ServeOptions,
}

impl WebFrontend {
    pub async fn run_async(self, ctx: FrontendContext) -> Result<SessionOutcome> {
        let budget = ctx.time_remaining();
        if let Some(draft) = ctx.draft.as_ref()
            && draft.restored
        {
            eprintln!(
                "Restored an unfinished draft from {}",
                draft.store.path().display()
            );
        }
        let addr = self.serve.socket_addr();
        let config = web_session_config(ctx, &self.serve);
        let title = &config.title.clone().unwrap_or_default();
        let bound = bind_session(config, addr)
            .await
            .context("failed to bind web session")?;
        let addr = bound.local_addr();
        eprintln!("{title} schemaui UI available at http://{addr}/");
        // Announced on stderr, not just drawn in the browser: whoever is
        // watching this terminal -- a human, or a wrapper script relaying the
        // stream -- has no other way to learn that the session will end itself.
        if let Some(budget) = budget {
            eprintln!(
                "Session closes automatically in {} (or as soon as you save).",
                format_budget(budget)
            );
        }
        eprintln!("Press Ctrl+C to abort the session.");
        bound.run().await.context("web UI session failed")
    }
}

impl Frontend for WebFrontend {
    fn run(self, ctx: FrontendContext) -> Result<SessionOutcome> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("failed to initialize tokio runtime")?;

        runtime.block_on(self.run_async(ctx))
    }
}

fn web_session_config(ctx: FrontendContext, serve: &ServeOptions) -> WebSessionConfig {
    let FrontendContext {
        title,
        description,
        ui_ast,
        layout,
        initial_data,
        schema,
        validator: _,
        deadline,
        draft,
    } = ctx;

    WebSessionConfig {
        title,
        description,
        // The pipeline hands over a bare AST; rendering its rich content is
        // web's job, so the TUI path never pays for it.
        rich: collect_assets(&ui_ast),
        ui_ast,
        layout,
        data: initial_data,
        schema,
        asset_provider: Arc::clone(&serve.assets),
        deadline,
        theme: serve.theme.clone(),
        draft,
    }
}
