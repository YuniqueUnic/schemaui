use color_eyre::eyre::{Result, WrapErr, eyre};
use schemaui::io::output;
use schemaui::web::session::{ServeOptions as WebServeOptions, WebSessionBuilder, bind_session};

use crate::cli::WebCommand;
use crate::session::{SessionBundle, prepare_session};

pub fn run_cli(cmd: WebCommand) -> Result<()> {
    let session = prepare_session(&cmd.common)?;
    let SessionBundle {
        schema,
        defaults,
        title,
        output,
    } = session;
    let mut builder = WebSessionBuilder::new(schema);
    if let Some(title) = title.clone() {
        builder = builder.with_title(title);
    }
    if let Some(defaults) = defaults {
        builder = builder.with_initial_data(defaults);
    }
    let config = builder.build().map_err(|err| eyre!(err))?;

    // Use tokio's Builder to create a runtime with proper configuration
    // This avoids the panic when dropping the runtime
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .wrap_err("failed to initialize tokio runtime")?;

    let host = cmd.host;
    let port = cmd.port;

    // Enter the runtime context to avoid nested runtime issues
    let _guard = runtime.enter();

    let value = runtime.block_on(async move {
        let bound = bind_session(config, WebServeOptions { host, port })
            .await
            .map_err(|err| eyre!(err))?;
        let addr = bound.local_addr();
        eprintln!("schemaui web UI available at http://{addr}/");
        eprintln!("Press Ctrl+C to abort the session.");
        bound.run().await.map_err(|err| eyre!(err))
    })?;

    // Explicitly shutdown the runtime to ensure clean exit
    runtime.shutdown_background();

    if let Some(options) = output {
        output::emit(&value, &options).map_err(|err| eyre!(err))?;
    }
    Ok(())
}
