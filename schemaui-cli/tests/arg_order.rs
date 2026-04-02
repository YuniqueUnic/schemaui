use clap::Parser;
use schemaui_cli::cli::{Cli, Commands};

#[test]
fn defaults_to_tui_when_no_subcommand_is_provided() {
    let cli = Cli::parse_from(["schemaui", "--schema", "./schema.json", "-f"]);

    assert!(cli.command.is_none(), "expected implicit default TUI mode");
    assert_eq!(cli.common.schema.as_deref(), Some("./schema.json"));
    assert!(cli.common.force);
}

#[test]
fn explicit_tui_subcommand_accepts_common_args() {
    let cli = Cli::parse_from(["schemaui", "tui", "--schema", "./schema.json", "-f"]);

    let Some(Commands::Tui(cmd)) = cli.command else {
        panic!("expected explicit tui subcommand");
    };

    assert_eq!(cmd.common.schema.as_deref(), Some("./schema.json"));
    assert!(cmd.common.force);
}

#[test]
fn explicit_tui_snapshot_subcommand_accepts_common_args() {
    let cli = Cli::parse_from([
        "schemaui",
        "tui-snapshot",
        "--schema",
        "./schema.json",
        "--out-dir",
        "./out",
    ]);

    let Some(Commands::TuiSnapshot(cmd)) = cli.command else {
        panic!("expected explicit tui-snapshot subcommand");
    };

    assert_eq!(cmd.common.schema.as_deref(), Some("./schema.json"));
    assert_eq!(cmd.out_dir.to_str(), Some("./out"));
}

#[cfg(feature = "web")]
#[test]
fn explicit_web_subcommand_accepts_common_args() {
    let cli = Cli::parse_from([
        "schemaui",
        "web",
        "--schema",
        "./schema.json",
        "--port",
        "3000",
        "-f",
    ]);

    let Some(Commands::Web(cmd)) = cli.command else {
        panic!("expected explicit web subcommand");
    };

    assert_eq!(cmd.common.schema.as_deref(), Some("./schema.json"));
    assert_eq!(cmd.port, 3000);
    assert!(cmd.common.force);
}

#[cfg(feature = "web")]
#[test]
fn trailing_web_token_is_parsed_as_subcommand_after_root_args() {
    let cli = Cli::parse_from(["schemaui", "--schema", "./schema.json", "-f", "web"]);

    let Some(Commands::Web(cmd)) = cli.command else {
        panic!("expected trailing web token to become the web subcommand");
    };

    assert_eq!(cli.common.schema.as_deref(), Some("./schema.json"));
    assert!(cli.common.force);
    assert_eq!(cmd.common.schema, None);
}

#[cfg(feature = "web")]
#[test]
fn explicit_web_snapshot_subcommand_accepts_common_args() {
    let cli = Cli::parse_from([
        "schemaui",
        "web-snapshot",
        "--schema",
        "./schema.json",
        "--out-dir",
        "./snapshots",
    ]);

    let Some(Commands::WebSnapshot(cmd)) = cli.command else {
        panic!("expected explicit web-snapshot subcommand");
    };

    assert_eq!(cmd.common.schema.as_deref(), Some("./schema.json"));
    assert_eq!(cmd.out_dir.to_str(), Some("./snapshots"));
}
