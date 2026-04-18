#![doc = include_str!("../cli_usage.md")]

use color_eyre::eyre::Result;

use schemaui_cli::cli::{Cli, Commands, TuiSnapshotCommand};
use schemaui_cli::completion;
use schemaui_cli::tui;

#[cfg(feature = "web")]
use schemaui_cli::cli::{WebCommand, WebSnapshotCommand};
#[cfg(feature = "web")]
use schemaui_cli::web;

fn main() -> Result<()> {
    color_eyre::install()?;
    let Cli { common, command } = Cli::from_env_or_exit();

    match command {
        Some(Commands::Completion(args)) => completion::run_cli(args),
        Some(Commands::Tui(args)) => {
            let common = common.merged_with(&args.common);
            tui::run_cli(&common)
        }
        None => tui::run_cli(&common),
        Some(Commands::TuiSnapshot(args)) => tui::run_snapshot_cli(TuiSnapshotCommand {
            common: common.merged_with(&args.common),
            out_dir: args.out_dir,
            tui_fn: args.tui_fn,
            form_fn: args.form_fn,
            layout_fn: args.layout_fn,
        }),
        #[cfg(feature = "web")]
        Some(Commands::Web(args)) => web::run_cli(WebCommand {
            common: common.merged_with(&args.common),
            host: args.host,
            port: args.port,
        }),
        #[cfg(feature = "web")]
        Some(Commands::WebSnapshot(args)) => web::run_snapshot_cli(WebSnapshotCommand {
            common: common.merged_with(&args.common),
            out_dir: args.out_dir,
            ts_export: args.ts_export,
        }),
    }
}
