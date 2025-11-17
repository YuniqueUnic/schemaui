use std::path::PathBuf;

use clap::{ArgAction, Parser, Subcommand};

#[cfg(feature = "web")]
use std::net::IpAddr;

#[derive(Debug, Parser)]
#[command(
    name = "schemaui",
    version,
    about = "Render JSON Schemas as interactive TUIs or Web UIs"
)]
pub struct Cli {
    #[command(flatten)]
    pub common: CommonArgs,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Launch the interactive terminal UI
    Tui,

    #[cfg(feature = "web")]
    /// Launch the interactive web UI instead of the terminal UI
    Web(WebCommand),
}

#[cfg(feature = "web")]
#[derive(Debug, Parser, Clone)]
pub struct WebCommand {
    #[command(flatten)]
    pub common: CommonArgs,

    /// Bind address for the temporary HTTP server
    #[arg(long = "host", value_name = "IP", default_value = "127.0.0.1")]
    pub host: IpAddr,

    /// Bind port for the temporary HTTP server (0 picks a random free port)
    #[arg(long = "port", value_name = "PORT", default_value_t = 0)]
    pub port: u16,
}

#[derive(Debug, Parser, Clone)]
pub struct CommonArgs {
    /// Schema spec: file path, inline payload, or "-" for stdin
    #[arg(short = 's', long = "schema", value_name = "SPEC")]
    pub schema: Option<String>,

    /// Config spec: file path, inline payload, or "-" for stdin
    #[arg(short = 'c', long = "config", alias = "data", value_name = "SPEC")]
    pub config: Option<String>,

    /// Title shown at the top of the UI
    #[arg(long = "title", value_name = "TEXT")]
    pub title: Option<String>,

    /// Output destinations ("-" writes to stdout). Accepts multiple values per flag use.
    #[arg(short = 'o', long = "output", value_name = "DEST", num_args = 1.., action = ArgAction::Append)]
    pub outputs: Vec<String>,

    /// Override the default temp file location (only used when no other destinations are set)
    #[arg(long = "temp-file", value_name = "PATH")]
    pub temp_file: Option<PathBuf>,

    /// Disable writing to the default temp file when no destinations are provided
    #[arg(long = "no-temp-file")]
    pub no_temp_file: bool,

    /// Emit compact JSON/TOML rather than pretty formatting
    #[arg(long = "no-pretty")]
    pub no_pretty: bool,

    /// Overwrite output files even if they already exist
    #[arg(short = 'f', long = "force", short_alias = 'y', alias = "yes")]
    pub force: bool,
}
