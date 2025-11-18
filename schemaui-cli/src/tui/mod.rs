use color_eyre::eyre::{Report, Result};
use schemaui::{SchemaUI, TuiFrontend};

use crate::cli::CommonArgs;
use crate::session::{SessionBundle, prepare_session};

pub fn run_cli(args: &CommonArgs) -> Result<()> {
    let session = prepare_session(args)?;
    execute_session(session)
}

pub(crate) fn execute_session(session: SessionBundle) -> Result<()> {
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
    let options = ui.options().clone();
    let frontend = TuiFrontend { options };
    ui.run_with_frontend(frontend)
        .map_err(Report::msg)
        .map(|_| ())
}
