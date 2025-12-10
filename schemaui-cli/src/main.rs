#![doc = include_str!("../cli_usage.md")]

use clap::Parser;
use color_eyre::eyre::Result;

use schemaui_cli::cli::{Cli, Commands};
use schemaui_cli::tui;

#[cfg(feature = "web")]
use schemaui_cli::web;

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Tui) | None => tui::run_cli(&cli.common),
        #[cfg(feature = "web")]
        Some(Commands::Web(args)) => web::run_cli(args),
        #[cfg(feature = "web")]
        Some(Commands::WebSnapshot(args)) => web::run_snapshot_cli(args),
    }
}
