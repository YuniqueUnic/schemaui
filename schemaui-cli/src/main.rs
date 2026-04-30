#![doc = include_str!("../cli_usage.md")]

use anyhow::Result;
#[cfg(not(feature = "tui"))]
use clap::CommandFactory;

use schemaui_cli::cli::Cli;
#[cfg(any(feature = "completion", feature = "tui", feature = "web"))]
use schemaui_cli::cli::Commands;
#[cfg(feature = "tui")]
use schemaui_cli::cli::TuiSnapshotCommand;
#[cfg(feature = "completion")]
use schemaui_cli::completion;
#[cfg(feature = "tui")]
use schemaui_cli::tui;

#[cfg(feature = "web")]
use schemaui_cli::cli::{WebCommand, WebSnapshotCommand};
#[cfg(feature = "web")]
use schemaui_cli::web;

fn main() -> Result<()> {
    let Cli { common, command } = Cli::from_env_or_exit();
    #[cfg(not(any(feature = "tui", feature = "web")))]
    let _ = &common;

    match command {
        #[cfg(feature = "completion")]
        Some(Commands::Completion(args)) => completion::run_cli(args),
        #[cfg(feature = "tui")]
        Some(Commands::Tui(args)) => {
            let common = common.merged_with(&args.common);
            tui::run_cli(&common)
        }
        #[cfg(feature = "tui")]
        None => tui::run_cli(&common),
        #[cfg(not(feature = "tui"))]
        None => {
            let mut cmd = Cli::command();
            cmd.print_help()?;
            println!();
            Err(anyhow::anyhow!(
                "this schemaui-cli build does not include the `tui` feature; use a subcommand supported by the active feature set"
            ))
        }
        #[cfg(feature = "tui")]
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
