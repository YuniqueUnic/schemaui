use std::path::PathBuf;

#[cfg(feature = "web")]
use std::net::IpAddr;

use clap::{ArgAction, Args, CommandFactory, Parser, Subcommand, ValueEnum, value_parser};

#[derive(Parser, Debug, Clone, Default, PartialEq, Eq)]
#[command(
    name = "schemaui",
    about = "Render JSON Schemas as interactive TUIs or Web UIs",
    version,
    propagate_version = true,
    disable_help_subcommand = true,
    subcommand_precedence_over_arg = true
)]
pub struct Cli {
    #[command(flatten)]
    pub common: CommonArgs,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    #[command(about = "Generate shell completion scripts for the schemaui CLI")]
    Completion(CompletionCommand),
    #[command(about = "Launch the interactive terminal UI")]
    Tui(TuiCommand),
    #[cfg(feature = "web")]
    #[command(about = "Launch the interactive web UI instead of the terminal UI")]
    Web(WebCommand),
    #[cfg(feature = "web")]
    #[command(about = "Precompute Web session snapshots instead of launching the UI")]
    WebSnapshot(WebSnapshotCommand),
    #[command(
        about = "Precompute TUI FormSchema/LayoutNavModel modules instead of launching the UI"
    )]
    TuiSnapshot(TuiSnapshotCommand),
}

#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct TuiCommand {
    #[command(flatten)]
    pub common: CommonArgs,
}

#[derive(Args, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionCommand {
    #[arg(help = "target shell: bash, zsh, fish, or powershell")]
    pub shell: CompletionShell,
}

#[derive(ValueEnum, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionShell {
    #[value(name = "bash")]
    Bash,
    #[value(name = "zsh")]
    Zsh,
    #[value(name = "fish")]
    Fish,
    #[value(name = "powershell")]
    PowerShell,
}

#[cfg(feature = "web")]
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct WebCommand {
    #[command(flatten)]
    pub common: CommonArgs,
    #[arg(
        short = 'l',
        long = "host",
        visible_aliases = ["bind", "listen"],
        value_name = "IP",
        help = "bind address for the temporary HTTP server",
        value_parser = value_parser!(IpAddr),
        default_value = "127.0.0.1"
    )]
    pub host: IpAddr,
    #[arg(
        short = 'p',
        long = "port",
        value_name = "PORT",
        help = "bind port for the temporary HTTP server (0 picks a random free port)",
        value_parser = value_parser!(u16),
        default_value_t = 0
    )]
    pub port: u16,
}

#[cfg(feature = "web")]
#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct WebSnapshotCommand {
    #[command(flatten)]
    pub common: CommonArgs,
    #[arg(
        long = "out-dir",
        value_name = "DIR",
        help = "output directory for generated Web snapshots (JSON + TS)",
        value_parser = value_parser!(PathBuf),
        default_value = "web_snapshots"
    )]
    pub out_dir: PathBuf,
    #[arg(
        long = "ts-export",
        value_name = "NAME",
        help = "name of the exported constant in the generated TS module",
        default_value = "SessionSnapshot"
    )]
    pub ts_export: String,
}

#[derive(Args, Debug, Clone, PartialEq, Eq)]
pub struct TuiSnapshotCommand {
    #[command(flatten)]
    pub common: CommonArgs,
    #[arg(
        long = "out-dir",
        value_name = "DIR",
        help = "output directory for generated TUI artifact modules (Rust source)",
        value_parser = value_parser!(PathBuf),
        default_value = "tui_artifacts"
    )]
    pub out_dir: PathBuf,
    #[arg(
        long = "tui-fn",
        value_name = "NAME",
        help = "name of the generated TuiArtifacts constructor function",
        default_value = "tui_artifacts"
    )]
    pub tui_fn: String,
    #[arg(
        long = "form-fn",
        value_name = "NAME",
        help = "name of the generated FormSchema constructor function",
        default_value = "tui_form_schema"
    )]
    pub form_fn: String,
    #[arg(
        long = "layout-fn",
        value_name = "NAME",
        help = "name of the generated LayoutNavModel constructor function",
        default_value = "tui_layout_nav"
    )]
    pub layout_fn: String,
}

#[derive(Args, Debug, Clone, Default, PartialEq, Eq)]
pub struct CommonArgs {
    #[arg(
        short = 's',
        long = "schema",
        help = "schema spec: local path, file/HTTP URL, inline payload, or \"-\" for stdin",
        allow_hyphen_values = true
    )]
    pub schema: Option<String>,
    #[arg(
        short = 'c',
        long = "config",
        visible_alias = "data",
        help = "config spec: local path, file/HTTP URL, inline payload, or \"-\" for stdin",
        allow_hyphen_values = true
    )]
    pub config: Option<String>,
    #[arg(
        long = "title",
        help = "title shown at the top of the UI",
        allow_hyphen_values = true
    )]
    pub title: Option<String>,
    #[arg(
        long = "description",
        help = "description shown under the title in the active UI",
        allow_hyphen_values = true
    )]
    pub description: Option<String>,
    #[arg(
        short = 'o',
        long = "output",
        value_name = "DEST",
        help = "output destinations (\"-\" writes to stdout). Repeat the flag to add more",
        action = ArgAction::Append,
        num_args = 1..,
        allow_hyphen_values = true
    )]
    pub outputs: Vec<String>,
    #[arg(
        long = "temp-file",
        value_name = "PATH",
        help = "write to PATH when no destinations are set (stdout remains the default)",
        value_parser = value_parser!(PathBuf)
    )]
    pub temp_file: Option<PathBuf>,
    #[arg(
        long = "no-temp-file",
        help = "compatibility no-op: stdout is already the default when no destinations are set",
        action = ArgAction::SetTrue
    )]
    pub no_temp_file: bool,
    #[arg(
        long = "no-pretty",
        help = "emit compact JSON/TOML rather than pretty formatting",
        action = ArgAction::SetTrue
    )]
    pub no_pretty: bool,
    #[arg(
        short = 'f',
        long = "force",
        visible_short_alias = 'y',
        visible_alias = "yes",
        help = "overwrite output files even if they already exist",
        action = ArgAction::SetTrue
    )]
    pub force: bool,
}

impl CommonArgs {
    pub fn merged_with(&self, local: &Self) -> Self {
        let mut outputs = self.outputs.clone();
        outputs.extend(local.outputs.clone());

        Self {
            schema: local.schema.clone().or_else(|| self.schema.clone()),
            config: local.config.clone().or_else(|| self.config.clone()),
            title: local.title.clone().or_else(|| self.title.clone()),
            description: local
                .description
                .clone()
                .or_else(|| self.description.clone()),
            outputs,
            temp_file: local.temp_file.clone().or_else(|| self.temp_file.clone()),
            no_temp_file: self.no_temp_file || local.no_temp_file,
            no_pretty: self.no_pretty || local.no_pretty,
            force: self.force || local.force,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliParseExit {
    pub output: String,
    pub status: Result<(), ()>,
}

impl CliParseExit {
    fn success(output: String) -> Self {
        Self {
            output,
            status: Ok(()),
        }
    }

    fn error(output: String) -> Self {
        Self {
            output,
            status: Err(()),
        }
    }
}

impl Cli {
    pub fn parse() -> Self {
        Self::from_env_or_exit()
    }

    pub fn from_env_or_exit() -> Self {
        match Self::try_parse_from(std::env::args()) {
            Ok(cli) => cli,
            Err(exit) => {
                if exit.status.is_ok() {
                    print!("{}", exit.output);
                    std::process::exit(0);
                }
                eprint!("{}", exit.output);
                std::process::exit(1);
            }
        }
    }

    pub fn parse_from<I, T>(args: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        Self::try_parse_from(args).unwrap_or_else(|exit| {
            panic!("failed to parse args: {}", exit.output);
        })
    }

    pub fn try_parse_from<I, T>(args: I) -> Result<Self, CliParseExit>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let argv = args.into_iter().map(Into::into).collect::<Vec<_>>();
        <Self as Parser>::try_parse_from(argv).map_err(clap_error_to_exit)
    }
}

pub fn command_info() -> clap::Command {
    <Cli as CommandFactory>::command()
}

fn clap_error_to_exit(err: clap::Error) -> CliParseExit {
    let output = err.to_string();
    match err.kind() {
        clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion => {
            CliParseExit::success(output)
        }
        _ => CliParseExit::error(output),
    }
}
