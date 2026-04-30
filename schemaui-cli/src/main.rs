#![doc = include_str!("../cli_usage.md")]

#[cfg(not(feature = "tui"))]
use clap::CommandFactory;
use color_eyre::eyre::Result;

#[cfg(feature = "tui")]
use schemaui_cli::cli::TuiSnapshotCommand;
use schemaui_cli::cli::{Cli, Commands};
use schemaui_cli::completion;
#[cfg(feature = "tui")]
use schemaui_cli::tui;

#[cfg(feature = "web")]
use schemaui_cli::cli::{WebCommand, WebSnapshotCommand};
#[cfg(feature = "web")]
use schemaui_cli::web;

fn main() -> Result<()> {
    color_eyre::install()?;
    let Cli { common, command } = Cli::from_env_or_exit();
    #[cfg(not(any(feature = "tui", feature = "web")))]
    let _ = &common;

    match command {
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
            Err(color_eyre::eyre::eyre!(
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
