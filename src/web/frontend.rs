use anyhow::{Context, Result};
use serde_json::Value;
use std::sync::Arc;

use crate::core::frontend::{Frontend, FrontendContext};

use super::assets::{EmbeddedAssets, WebAssetProvider};
use super::session::{ServeOptions, WebSessionConfig, bind_session};

/// Web frontend implementation that consumes a prepared `FrontendContext`
/// and runs the browser-based UI via a temporary HTTP server.
#[derive(Debug, Clone, Copy)]
pub struct WebFrontend {
    pub serve: ServeOptions,
}

impl Frontend for WebFrontend {
    fn run(self, ctx: FrontendContext) -> Result<Value> {
        let FrontendContext {
            title,
            ui_ast,
            layout,
            initial_data,
            schema,
            validator: _,
        } = ctx;

        let asset_provider: Arc<dyn WebAssetProvider> = Arc::new(EmbeddedAssets);

        let config = WebSessionConfig {
            title,
            ui_ast,
            layout,
            data: initial_data,
            schema,
            asset_provider,
        };

        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .context("failed to initialize tokio runtime")?;

        let _guard = runtime.enter();

        let serve = self.serve;
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
        Ok(value)
    }
}
