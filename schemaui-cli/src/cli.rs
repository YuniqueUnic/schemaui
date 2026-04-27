use std::path::PathBuf;

use clap::{
    Arg, ArgAction, ArgMatches, Command, CommandFactory, ValueEnum, builder::EnumValueParser,
    value_parser,
};

#[cfg(feature = "web")]
use std::net::IpAddr;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Cli {
    pub common: CommonArgs,
    pub command: Option<Commands>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Commands {
    Completion(CompletionCommand),
    Tui(TuiCommand),
    #[cfg(feature = "web")]
    Web(WebCommand),
    #[cfg(feature = "web")]
    WebSnapshot(WebSnapshotCommand),
    TuiSnapshot(TuiSnapshotCommand),
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TuiCommand {
    pub common: CommonArgs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompletionCommand {
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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebCommand {
    pub common: CommonArgs,
    pub host: IpAddr,
    pub port: u16,
}

#[cfg(feature = "web")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSnapshotCommand {
    pub common: CommonArgs,
    pub out_dir: PathBuf,
    pub ts_export: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TuiSnapshotCommand {
    pub common: CommonArgs,
    pub out_dir: PathBuf,
    pub tui_fn: String,
    pub form_fn: String,
    pub layout_fn: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CommonArgs {
    pub schema: Option<String>,
    pub config: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub outputs: Vec<String>,
    pub temp_file: Option<PathBuf>,
    pub no_temp_file: bool,
    pub no_pretty: bool,
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
        let matches = command_info()
            .try_get_matches_from(argv)
            .map_err(clap_error_to_exit)?;
        Ok(Self::from_matches(&matches))
    }

    fn from_matches(matches: &ArgMatches) -> Self {
        let common = common_args_from_matches(matches);
        let command = match matches.subcommand() {
            Some(("completion", sub_matches)) => Some(Commands::Completion(CompletionCommand {
                shell: *sub_matches
                    .get_one::<CompletionShell>("shell")
                    .expect("completion shell is required"),
            })),
            Some(("tui", sub_matches)) => Some(Commands::Tui(TuiCommand {
                common: common_args_from_matches(sub_matches),
            })),
            #[cfg(feature = "web")]
            Some(("web", sub_matches)) => Some(Commands::Web(WebCommand {
                common: common_args_from_matches(sub_matches),
                host: *sub_matches
                    .get_one::<IpAddr>("host")
                    .expect("web host has a default"),
                port: *sub_matches
                    .get_one::<u16>("port")
                    .expect("web port has a default"),
            })),
            #[cfg(feature = "web")]
            Some(("web-snapshot", sub_matches)) => {
                Some(Commands::WebSnapshot(WebSnapshotCommand {
                    common: common_args_from_matches(sub_matches),
                    out_dir: sub_matches
                        .get_one::<PathBuf>("out_dir")
                        .cloned()
                        .expect("web snapshot out-dir has a default"),
                    ts_export: sub_matches
                        .get_one::<String>("ts_export")
                        .cloned()
                        .expect("web snapshot ts-export has a default"),
                }))
            }
            Some(("tui-snapshot", sub_matches)) => {
                Some(Commands::TuiSnapshot(TuiSnapshotCommand {
                    common: common_args_from_matches(sub_matches),
                    out_dir: sub_matches
                        .get_one::<PathBuf>("out_dir")
                        .cloned()
                        .expect("tui snapshot out-dir has a default"),
                    tui_fn: sub_matches
                        .get_one::<String>("tui_fn")
                        .cloned()
                        .expect("tui snapshot tui-fn has a default"),
                    form_fn: sub_matches
                        .get_one::<String>("form_fn")
                        .cloned()
                        .expect("tui snapshot form-fn has a default"),
                    layout_fn: sub_matches
                        .get_one::<String>("layout_fn")
                        .cloned()
                        .expect("tui snapshot layout-fn has a default"),
                }))
            }
            None => None,
            Some((other, _)) => unreachable!("unexpected subcommand: {other}"),
        };

        Self { common, command }
    }
}

pub fn command_info() -> clap::Command {
    let command = Command::new("schemaui")
        .about("Render JSON Schemas as interactive TUIs or Web UIs")
        .version(env!("CARGO_PKG_VERSION"))
        .propagate_version(true)
        .disable_help_subcommand(true)
        .disable_version_flag(true)
        .subcommand_precedence_over_arg(true)
        .arg(version_arg());

    let command = with_common_args(command)
        .subcommand(completion_command())
        .subcommand(tui_command())
        .subcommand(tui_snapshot_command());

    #[cfg(feature = "web")]
    let command = command
        .subcommand(web_command())
        .subcommand(web_snapshot_command());

    command
}

impl CommandFactory for Cli {
    fn command() -> Command {
        command_info()
    }

    fn command_for_update() -> Command {
        command_info()
    }
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

fn common_args_from_matches(matches: &ArgMatches) -> CommonArgs {
    CommonArgs {
        schema: matches.get_one::<String>("schema").cloned(),
        config: matches.get_one::<String>("config").cloned(),
        title: matches.get_one::<String>("title").cloned(),
        description: matches.get_one::<String>("description").cloned(),
        outputs: matches
            .get_many::<String>("output")
            .map(|values| values.cloned().collect())
            .unwrap_or_default(),
        temp_file: matches.get_one::<PathBuf>("temp_file").cloned(),
        no_temp_file: matches.get_flag("no_temp_file"),
        no_pretty: matches.get_flag("no_pretty"),
        force: matches.get_flag("force"),
    }
}

fn with_common_args(command: Command) -> Command {
    common_args()
        .into_iter()
        .fold(command, |command, arg| command.arg(arg))
}

fn common_args() -> Vec<Arg> {
    vec![
        Arg::new("schema")
            .short('s')
            .long("schema")
            .help("schema spec: local path, file/HTTP URL, inline payload, or \"-\" for stdin")
            .action(ArgAction::Set)
            .allow_hyphen_values(true),
        Arg::new("config")
            .short('c')
            .long("config")
            .visible_alias("data")
            .help("config spec: local path, file/HTTP URL, inline payload, or \"-\" for stdin")
            .action(ArgAction::Set)
            .allow_hyphen_values(true),
        Arg::new("title")
            .long("title")
            .help("title shown at the top of the UI")
            .action(ArgAction::Set)
            .allow_hyphen_values(true),
        Arg::new("description")
            .long("description")
            .help("description shown under the title in the active UI")
            .action(ArgAction::Set)
            .allow_hyphen_values(true),
        Arg::new("output")
            .short('o')
            .long("output")
            .value_name("DEST")
            .help("output destinations (\"-\" writes to stdout). Repeat the flag to add more")
            .action(ArgAction::Append)
            .num_args(1..)
            .allow_hyphen_values(true),
        Arg::new("temp_file")
            .long("temp-file")
            .value_name("PATH")
            .help("write to PATH when no destinations are set (stdout remains the default)")
            .value_parser(value_parser!(PathBuf)),
        Arg::new("no_temp_file")
            .long("no-temp-file")
            .help("compatibility no-op: stdout is already the default when no destinations are set")
            .action(ArgAction::SetTrue),
        Arg::new("no_pretty")
            .long("no-pretty")
            .help("emit compact JSON/TOML rather than pretty formatting")
            .action(ArgAction::SetTrue),
        Arg::new("force")
            .short('f')
            .long("force")
            .visible_short_alias('y')
            .visible_alias("yes")
            .help("overwrite output files even if they already exist")
            .action(ArgAction::SetTrue),
    ]
}

fn version_arg() -> Arg {
    Arg::new("version_flag")
        .long("version")
        .short('V')
        .visible_short_alias('v')
        .global(true)
        .help("Print version")
        .action(ArgAction::Version)
}

fn completion_command() -> Command {
    Command::new("completion")
        .about("Generate shell completion scripts for the schemaui CLI")
        .arg(
            Arg::new("shell")
                .help("target shell: bash, zsh, fish, or powershell")
                .required(true)
                .value_parser(EnumValueParser::<CompletionShell>::new()),
        )
}

fn tui_command() -> Command {
    with_common_args(Command::new("tui").about("Launch the interactive terminal UI"))
}

#[cfg(feature = "web")]
fn web_command() -> Command {
    with_common_args(
        Command::new("web").about("Launch the interactive web UI instead of the terminal UI"),
    )
    .arg(
        Arg::new("host")
            .short('l')
            .long("host")
            .visible_aliases(["bind", "listen"])
            .value_name("IP")
            .help("bind address for the temporary HTTP server")
            .value_parser(value_parser!(IpAddr))
            .default_value("127.0.0.1"),
    )
    .arg(
        Arg::new("port")
            .short('p')
            .long("port")
            .value_name("PORT")
            .help("bind port for the temporary HTTP server (0 picks a random free port)")
            .value_parser(value_parser!(u16))
            .default_value("0"),
    )
}

#[cfg(feature = "web")]
fn web_snapshot_command() -> Command {
    with_common_args(
        Command::new("web-snapshot")
            .about("Precompute Web session snapshots instead of launching the UI"),
    )
    .arg(
        Arg::new("out_dir")
            .long("out-dir")
            .value_name("DIR")
            .help("output directory for generated Web snapshots (JSON + TS)")
            .value_parser(value_parser!(PathBuf))
            .default_value("web_snapshots"),
    )
    .arg(
        Arg::new("ts_export")
            .long("ts-export")
            .value_name("NAME")
            .help("name of the exported constant in the generated TS module")
            .default_value("SessionSnapshot"),
    )
}

fn tui_snapshot_command() -> Command {
    with_common_args(
        Command::new("tui-snapshot")
            .about("Precompute TUI FormSchema/LayoutNavModel modules instead of launching the UI"),
    )
    .arg(
        Arg::new("out_dir")
            .long("out-dir")
            .value_name("DIR")
            .help("output directory for generated TUI artifact modules (Rust source)")
            .value_parser(value_parser!(PathBuf))
            .default_value("tui_artifacts"),
    )
    .arg(
        Arg::new("tui_fn")
            .long("tui-fn")
            .value_name("NAME")
            .help("name of the generated TuiArtifacts constructor function")
            .default_value("tui_artifacts"),
    )
    .arg(
        Arg::new("form_fn")
            .long("form-fn")
            .value_name("NAME")
            .help("name of the generated FormSchema constructor function")
            .default_value("tui_form_schema"),
    )
    .arg(
        Arg::new("layout_fn")
            .long("layout-fn")
            .value_name("NAME")
            .help("name of the generated LayoutNavModel constructor function")
            .default_value("tui_layout_nav"),
    )
}
