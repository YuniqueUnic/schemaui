use std::path::PathBuf;

use argh::{ArgsInfo, FromArgValue, FromArgs};

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

#[derive(FromArgValue, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    Nushell,
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

    pub fn try_parse_from<I, T>(args: I) -> Result<Self, argh::EarlyExit>
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        let raw = args.into_iter().map(Into::into).collect::<Vec<_>>();
        let program = raw
            .first()
            .cloned()
            .unwrap_or_else(|| "schemaui".to_string());

        let normalized = normalize_args(&raw[1..]);
        let scan = scan_for_command(&normalized);
        let mut parse_args = normalized.clone();
        let injected_default_tui = matches!(scan, CommandScan::None);
        if injected_default_tui {
            parse_args.push("tui".to_string());
        }
        let parse_args = expand_output_values(&parse_args);
        let parse_refs = parse_args.iter().map(String::as_str).collect::<Vec<_>>();
        let parsed = ArghCli::from_args(&[program.as_str()], &parse_refs)?;
        Ok(Self::from_argh(parsed, injected_default_tui))
    }

    fn from_argh(parsed: ArghCli, injected_default_tui: bool) -> Self {
        let common = common_args_from_root(&parsed);
        match parsed.command {
            ArghCommands::Tui(_command) if injected_default_tui => Self {
                common,
                command: None,
            },
            ArghCommands::Completion(command) => Self {
                common,
                command: Some(Commands::Completion(CompletionCommand {
                    shell: command.shell,
                })),
            },
            ArghCommands::Tui(command) => Self {
                common,
                command: Some(Commands::Tui(TuiCommand {
                    common: common_args_from_tui(command),
                })),
            },
            #[cfg(feature = "web")]
            ArghCommands::Web(command) => Self {
                common,
                command: Some(Commands::Web(WebCommand {
                    common: common_args_from_web(&command),
                    host: command.host,
                    port: command.port,
                })),
            },
            #[cfg(feature = "web")]
            ArghCommands::WebSnapshot(command) => Self {
                common,
                command: Some(Commands::WebSnapshot(WebSnapshotCommand {
                    common: common_args_from_web_snapshot(&command),
                    out_dir: command.out_dir,
                    ts_export: command.ts_export,
                })),
            },
            ArghCommands::TuiSnapshot(command) => Self {
                common,
                command: Some(Commands::TuiSnapshot(TuiSnapshotCommand {
                    common: common_args_from_tui_snapshot(&command),
                    out_dir: command.out_dir,
                    tui_fn: command.tui_fn,
                    form_fn: command.form_fn,
                    layout_fn: command.layout_fn,
                })),
            },
        }
    }
}

pub fn command_info() -> argh::CommandInfoWithArgs {
    ArghCli::get_args_info()
}

fn common_args_from_root(args: &ArghCli) -> CommonArgs {
    CommonArgs {
        schema: args.schema.clone(),
        config: args.config.clone(),
        title: args.title.clone(),
        description: args.description.clone(),
        outputs: args.outputs.clone(),
        temp_file: args.temp_file.clone(),
        no_temp_file: args.no_temp_file,
        no_pretty: args.no_pretty,
        force: args.force,
    }
}

fn common_args_from_tui(args: ArghTuiCommand) -> CommonArgs {
    CommonArgs {
        schema: args.schema,
        config: args.config,
        title: args.title,
        description: args.description,
        outputs: args.outputs,
        temp_file: args.temp_file,
        no_temp_file: args.no_temp_file,
        no_pretty: args.no_pretty,
        force: args.force,
    }
}

#[cfg(feature = "web")]
fn common_args_from_web(args: &ArghWebCommand) -> CommonArgs {
    CommonArgs {
        schema: args.schema.clone(),
        config: args.config.clone(),
        title: args.title.clone(),
        description: args.description.clone(),
        outputs: args.outputs.clone(),
        temp_file: args.temp_file.clone(),
        no_temp_file: args.no_temp_file,
        no_pretty: args.no_pretty,
        force: args.force,
    }
}

#[cfg(feature = "web")]
fn common_args_from_web_snapshot(args: &ArghWebSnapshotCommand) -> CommonArgs {
    CommonArgs {
        schema: args.schema.clone(),
        config: args.config.clone(),
        title: args.title.clone(),
        description: args.description.clone(),
        outputs: args.outputs.clone(),
        temp_file: args.temp_file.clone(),
        no_temp_file: args.no_temp_file,
        no_pretty: args.no_pretty,
        force: args.force,
    }
}

fn common_args_from_tui_snapshot(args: &ArghTuiSnapshotCommand) -> CommonArgs {
    CommonArgs {
        schema: args.schema.clone(),
        config: args.config.clone(),
        title: args.title.clone(),
        description: args.description.clone(),
        outputs: args.outputs.clone(),
        temp_file: args.temp_file.clone(),
        no_temp_file: args.no_temp_file,
        no_pretty: args.no_pretty,
        force: args.force,
    }
}

#[derive(FromArgs, ArgsInfo, Debug, PartialEq)]
#[argh(help_triggers("-h", "--help", "help"))]
/// Render JSON Schemas as interactive TUIs or Web UIs
struct ArghCli {
    /// schema spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 's')]
    schema: Option<String>,

    /// config spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 'c')]
    config: Option<String>,

    /// title shown at the top of the UI
    #[argh(option)]
    title: Option<String>,

    /// description shown under the title in the active UI
    #[argh(option)]
    description: Option<String>,

    /// output destinations ("-" writes to stdout). Repeat the flag to add more.
    #[argh(option, short = 'o', long = "output")]
    outputs: Vec<String>,

    /// write to PATH when no destinations are set (stdout remains the default)
    #[argh(option)]
    temp_file: Option<PathBuf>,

    /// compatibility no-op: stdout is already the default when no destinations are set
    #[argh(switch)]
    no_temp_file: bool,

    /// emit compact JSON/TOML rather than pretty formatting
    #[argh(switch)]
    no_pretty: bool,

    /// overwrite output files even if they already exist
    #[argh(switch, short = 'f')]
    force: bool,

    #[argh(subcommand)]
    command: ArghCommands,
}

#[derive(FromArgs, ArgsInfo, Debug, PartialEq)]
#[argh(subcommand)]
enum ArghCommands {
    Completion(ArghCompletionCommand),
    Tui(ArghTuiCommand),
    #[cfg(feature = "web")]
    Web(ArghWebCommand),
    #[cfg(feature = "web")]
    WebSnapshot(ArghWebSnapshotCommand),
    TuiSnapshot(ArghTuiSnapshotCommand),
}

#[derive(FromArgs, ArgsInfo, Debug, PartialEq)]
/// Generate shell completion scripts for the schemaui CLI
#[argh(subcommand, name = "completion", help_triggers("-h", "--help", "help"))]
struct ArghCompletionCommand {
    /// target shell: bash, zsh, fish, or nushell
    #[argh(positional)]
    shell: CompletionShell,
}

#[derive(FromArgs, ArgsInfo, Debug, PartialEq)]
#[argh(subcommand, name = "tui", help_triggers("-h", "--help", "help"))]
/// Launch the interactive terminal UI
struct ArghTuiCommand {
    /// schema spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 's')]
    schema: Option<String>,

    /// config spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 'c')]
    config: Option<String>,

    /// title shown at the top of the UI
    #[argh(option)]
    title: Option<String>,

    /// description shown under the title in the active UI
    #[argh(option)]
    description: Option<String>,

    /// output destinations ("-" writes to stdout). Repeat the flag to add more.
    #[argh(option, short = 'o', long = "output")]
    outputs: Vec<String>,

    /// write to PATH when no destinations are set (stdout remains the default)
    #[argh(option)]
    temp_file: Option<PathBuf>,

    /// compatibility no-op: stdout is already the default when no destinations are set
    #[argh(switch)]
    no_temp_file: bool,

    /// emit compact JSON/TOML rather than pretty formatting
    #[argh(switch)]
    no_pretty: bool,

    /// overwrite output files even if they already exist
    #[argh(switch, short = 'f')]
    force: bool,
}

#[cfg(feature = "web")]
#[derive(FromArgs, ArgsInfo, Debug, PartialEq)]
#[argh(subcommand, name = "web", help_triggers("-h", "--help", "help"))]
/// Launch the interactive web UI instead of the terminal UI
struct ArghWebCommand {
    /// schema spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 's')]
    schema: Option<String>,

    /// config spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 'c')]
    config: Option<String>,

    /// title shown at the top of the UI
    #[argh(option)]
    title: Option<String>,

    /// description shown under the title in the active UI
    #[argh(option)]
    description: Option<String>,

    /// output destinations ("-" writes to stdout). Repeat the flag to add more.
    #[argh(option, short = 'o', long = "output")]
    outputs: Vec<String>,

    /// write to PATH when no destinations are set (stdout remains the default)
    #[argh(option)]
    temp_file: Option<PathBuf>,

    /// compatibility no-op: stdout is already the default when no destinations are set
    #[argh(switch)]
    no_temp_file: bool,

    /// emit compact JSON/TOML rather than pretty formatting
    #[argh(switch)]
    no_pretty: bool,

    /// overwrite output files even if they already exist
    #[argh(switch, short = 'f')]
    force: bool,

    /// bind address for the temporary HTTP server
    #[argh(option, short = 'l', default = "default_host()")]
    host: IpAddr,

    /// bind port for the temporary HTTP server (0 picks a random free port)
    #[argh(option, short = 'p', default = "0")]
    port: u16,
}

#[cfg(feature = "web")]
#[derive(FromArgs, ArgsInfo, Debug, PartialEq)]
#[argh(
    subcommand,
    name = "web-snapshot",
    help_triggers("-h", "--help", "help")
)]
/// Precompute Web session snapshots instead of launching the UI
struct ArghWebSnapshotCommand {
    /// schema spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 's')]
    schema: Option<String>,

    /// config spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 'c')]
    config: Option<String>,

    /// title shown at the top of the UI
    #[argh(option)]
    title: Option<String>,

    /// description shown under the title in the active UI
    #[argh(option)]
    description: Option<String>,

    /// output destinations ("-" writes to stdout). Repeat the flag to add more.
    #[argh(option, short = 'o', long = "output")]
    outputs: Vec<String>,

    /// write to PATH when no destinations are set (stdout remains the default)
    #[argh(option)]
    temp_file: Option<PathBuf>,

    /// compatibility no-op: stdout is already the default when no destinations are set
    #[argh(switch)]
    no_temp_file: bool,

    /// emit compact JSON/TOML rather than pretty formatting
    #[argh(switch)]
    no_pretty: bool,

    /// overwrite output files even if they already exist
    #[argh(switch, short = 'f')]
    force: bool,

    /// output directory for generated Web snapshots (JSON + TS)
    #[argh(option, default = "PathBuf::from(\"web_snapshots\")")]
    out_dir: PathBuf,

    /// name of the exported constant in the generated TS module
    #[argh(option, default = "String::from(\"SessionSnapshot\")")]
    ts_export: String,
}

#[derive(FromArgs, ArgsInfo, Debug, PartialEq)]
#[argh(
    subcommand,
    name = "tui-snapshot",
    help_triggers("-h", "--help", "help")
)]
/// Precompute TUI FormSchema/LayoutNavModel modules instead of launching the UI
struct ArghTuiSnapshotCommand {
    /// schema spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 's')]
    schema: Option<String>,

    /// config spec: local path, file/HTTP URL, inline payload, or "-" for stdin
    #[argh(option, short = 'c')]
    config: Option<String>,

    /// title shown at the top of the UI
    #[argh(option)]
    title: Option<String>,

    /// description shown under the title in the active UI
    #[argh(option)]
    description: Option<String>,

    /// output destinations ("-" writes to stdout). Repeat the flag to add more.
    #[argh(option, short = 'o', long = "output")]
    outputs: Vec<String>,

    /// write to PATH when no destinations are set (stdout remains the default)
    #[argh(option)]
    temp_file: Option<PathBuf>,

    /// compatibility no-op: stdout is already the default when no destinations are set
    #[argh(switch)]
    no_temp_file: bool,

    /// emit compact JSON/TOML rather than pretty formatting
    #[argh(switch)]
    no_pretty: bool,

    /// overwrite output files even if they already exist
    #[argh(switch, short = 'f')]
    force: bool,

    /// output directory for generated TUI artifact modules (Rust source)
    #[argh(option, default = "PathBuf::from(\"tui_artifacts\")")]
    out_dir: PathBuf,

    /// name of the generated TuiArtifacts constructor function
    #[argh(option, default = "String::from(\"tui_artifacts\")")]
    tui_fn: String,

    /// name of the generated FormSchema constructor function
    #[argh(option, default = "String::from(\"tui_form_schema\")")]
    form_fn: String,

    /// name of the generated LayoutNavModel constructor function
    #[argh(option, default = "String::from(\"tui_layout_nav\")")]
    layout_fn: String,
}

#[cfg(feature = "web")]
fn default_host() -> IpAddr {
    IpAddr::from([127, 0, 0, 1])
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CommandScan {
    None,
    Help,
    Explicit,
}

fn scan_for_command(args: &[String]) -> CommandScan {
    let mut index = 0usize;
    while index < args.len() {
        let token = args[index].as_str();
        if is_help_trigger(token) {
            return CommandScan::Help;
        }
        if is_known_subcommand(token) {
            return CommandScan::Explicit;
        }
        if consumes_multiple_values(token) {
            index += 1;
            while index < args.len() {
                let next = args[index].as_str();
                if next.starts_with('-') || is_known_subcommand(next) || is_help_trigger(next) {
                    break;
                }
                index += 1;
            }
            continue;
        }
        if consumes_single_value(token) {
            index += 2;
            continue;
        }
        index += 1;
    }
    CommandScan::None
}

fn normalize_args(args: &[String]) -> Vec<String> {
    let mut normalized = Vec::new();
    let mut index = 0usize;
    let mut segment_start = 0usize;

    while index < args.len() {
        let token = args[index].as_str();

        if let Some((flag, value)) = normalize_inline_option(token) {
            if consumes_single_value(&flag) {
                upsert_single_value_option(&mut normalized, segment_start, flag, value);
            } else {
                normalized.push(flag);
                normalized.push(value);
            }
            index += 1;
            continue;
        }

        let token = match token {
            "--data" => "--config",
            "--bind" | "--listen" => "--host",
            "-y" | "--yes" => "--force",
            other => other,
        };

        if is_known_subcommand(token) {
            normalized.push(token.to_string());
            segment_start = normalized.len();
            index += 1;
            continue;
        }

        if consumes_single_value(token)
            && let Some(value) = args.get(index + 1)
        {
            upsert_single_value_option(
                &mut normalized,
                segment_start,
                token.to_string(),
                value.clone(),
            );
            index += 2;
            continue;
        }

        normalized.push(token.to_string());
        index += 1;
    }

    normalized
}

fn upsert_single_value_option(
    normalized: &mut Vec<String>,
    segment_start: usize,
    flag: String,
    value: String,
) {
    if let Some(position) = normalized[segment_start..]
        .windows(2)
        .position(|window| window[0] == flag)
    {
        normalized[segment_start + position + 1] = value;
        return;
    }

    normalized.push(flag);
    normalized.push(value);
}

fn normalize_inline_option(token: &str) -> Option<(String, String)> {
    const INLINE_ALIASES: &[(&str, &str)] = &[
        ("--schema=", "--schema"),
        ("--config=", "--config"),
        ("--data=", "--config"),
        ("--title=", "--title"),
        ("--description=", "--description"),
        ("--output=", "--output"),
        ("--temp-file=", "--temp-file"),
        ("--host=", "--host"),
        ("--bind=", "--host"),
        ("--listen=", "--host"),
        ("--port=", "--port"),
        ("--out-dir=", "--out-dir"),
        ("--tui-fn=", "--tui-fn"),
        ("--form-fn=", "--form-fn"),
        ("--layout-fn=", "--layout-fn"),
        ("--ts-export=", "--ts-export"),
    ];

    for (prefix, canonical) in INLINE_ALIASES {
        if let Some(value) = token.strip_prefix(prefix) {
            return Some(((*canonical).to_string(), value.to_string()));
        }
    }
    None
}

fn expand_output_values(args: &[String]) -> Vec<String> {
    let mut expanded = Vec::new();
    let mut index = 0usize;
    while index < args.len() {
        let token = args[index].as_str();
        if consumes_multiple_values(token) {
            let canonical = "--output".to_string();
            expanded.push(canonical.clone());
            index += 1;

            let mut consumed_any = false;
            while index < args.len() {
                let next = args[index].as_str();
                if next.starts_with('-') || is_known_subcommand(next) {
                    break;
                }

                if consumed_any {
                    expanded.push(canonical.clone());
                }
                expanded.push(args[index].clone());
                consumed_any = true;
                index += 1;
            }
            continue;
        }

        expanded.push(args[index].clone());
        index += 1;
    }
    expanded
}

fn consumes_single_value(token: &str) -> bool {
    matches!(
        token,
        "-s" | "--schema"
            | "-c"
            | "--config"
            | "--title"
            | "--description"
            | "--temp-file"
            | "-l"
            | "--host"
            | "-p"
            | "--port"
            | "--out-dir"
            | "--tui-fn"
            | "--form-fn"
            | "--layout-fn"
            | "--ts-export"
    )
}

fn consumes_multiple_values(token: &str) -> bool {
    matches!(token, "-o" | "--output")
}

fn is_help_trigger(token: &str) -> bool {
    matches!(token, "-h" | "--help" | "help")
}

fn is_known_subcommand(token: &str) -> bool {
    matches!(token, "completion" | "tui" | "tui-snapshot")
        || cfg!(feature = "web") && matches!(token, "web" | "web-snapshot")
}
