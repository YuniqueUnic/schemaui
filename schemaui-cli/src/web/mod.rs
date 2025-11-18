use color_eyre::eyre::{Report, Result};
use schemaui::SchemaUI;
use schemaui::web::session::ServeOptions as WebServeOptions;

use crate::cli::WebCommand;
use crate::session::{SessionBundle, prepare_session};

pub fn run_cli(cmd: WebCommand) -> Result<()> {
    let session = prepare_session(&cmd.common)?;
    execute_web_session(session, cmd)
}

fn execute_web_session(session: SessionBundle, cmd: WebCommand) -> Result<()> {
    let SessionBundle {
        schema,
        defaults,
        title,
        output,
    } = session;

    let mut ui = SchemaUI::new(schema);
    if let Some(title) = title {
        ui = ui.with_title(title);
    }
    if let Some(ref defaults) = defaults {
        ui = ui.with_default_data(defaults);
    }
    if let Some(options) = output {
        ui = ui.with_output(options);
    }

    let serve = WebServeOptions {
        host: cmd.host,
        port: cmd.port,
    };

    ui.run_web(serve).map_err(Report::msg).map(|_| ())
}
