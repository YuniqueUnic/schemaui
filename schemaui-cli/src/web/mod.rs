use color_eyre::eyre::Result;
use schemaui::UiFrontend;
use schemaui::web::session::ServeOptions as WebServeOptions;

use crate::cli::WebCommand;
use crate::session::prepare_session;
use crate::tui::execute_session;

pub fn run_cli(cmd: WebCommand) -> Result<()> {
    let session = prepare_session(&cmd.common)?;
    let serve = WebServeOptions {
        host: cmd.host,
        port: cmd.port,
    };
    execute_session(session, UiFrontend::Web(serve))
}
