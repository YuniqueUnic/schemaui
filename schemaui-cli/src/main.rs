#![doc = include_str!("../cli_usage.md")]

use clap::Parser;
use color_eyre::eyre::Result;

use schemaui_cli::cli::Cli;
use schemaui_cli::tui;

#[cfg(feature = "web")]
use schemaui_cli::{cli::Commands, web};

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    #[cfg(feature = "web")]
    if let Some(command) = cli.command {
        return match command {
            Commands::Web(args) => web::run_cli(args),
        };
    }

    tui::run_cli(&cli.common)
}
