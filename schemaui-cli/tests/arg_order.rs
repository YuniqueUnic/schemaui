use schemaui_cli::cli::{Cli, Commands, CompletionShell};

#[test]
fn defaults_to_tui_when_no_subcommand_is_provided() {
    let cli = Cli::parse_from(["schemaui", "--schema", "./schema.json", "-f"]);

    assert!(cli.command.is_none(), "expected implicit default TUI mode");
    assert_eq!(cli.common.schema.as_deref(), Some("./schema.json"));
    assert!(cli.common.force);
}

#[test]
fn default_tui_accepts_description_flag() {
    let cli = Cli::parse_from([
        "schemaui",
        "--schema",
        "./schema.json",
        "--description",
        "Root description",
    ]);

    assert!(cli.command.is_none(), "expected implicit default TUI mode");
    assert_eq!(cli.common.description.as_deref(), Some("Root description"));
}

#[test]
fn inline_description_assignment_is_parsed_by_clap() {
    let cli = Cli::parse_from([
        "schemaui",
        "--schema=./schema.json",
        "--description=Inline description",
    ]);

    assert!(cli.command.is_none(), "expected implicit default TUI mode");
    assert_eq!(
        cli.common.description.as_deref(),
        Some("Inline description")
    );
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
fn subcommand_description_overrides_root_description_during_merge() {
    let cli = Cli::parse_from([
        "schemaui",
        "--description",
        "root description",
        "tui",
        "--description",
        "local description",
    ]);

    let Some(Commands::Tui(cmd)) = cli.command else {
        panic!("expected explicit tui subcommand");
    };

    let merged = cli.common.merged_with(&cmd.common);
    assert_eq!(cli.common.description.as_deref(), Some("root description"));
    assert_eq!(cmd.common.description.as_deref(), Some("local description"));
    assert_eq!(merged.description.as_deref(), Some("local description"));
}

#[test]
fn explicit_completion_subcommand_remains_completion() {
    let cli = Cli::parse_from(["schemaui", "completion", "bash"]);

    let Some(Commands::Completion(cmd)) = cli.command else {
        panic!("expected explicit completion subcommand");
    };

    assert_eq!(cmd.shell, CompletionShell::Bash);
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

#[test]
fn alias_flags_are_parsed_by_clap() {
    let cli = Cli::parse_from([
        "schemaui",
        "--data",
        "./config.json",
        "--yes",
        "--output",
        "./a.json",
        "./b.json",
    ]);

    assert!(cli.command.is_none(), "expected implicit default TUI mode");
    assert_eq!(cli.common.config.as_deref(), Some("./config.json"));
    assert!(cli.common.force);
    assert_eq!(cli.common.outputs, vec!["./a.json", "./b.json"]);
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
fn web_host_aliases_parse_as_host() {
    for (flag, expected) in [("--bind", "0.0.0.0"), ("--listen", "127.0.0.1")] {
        let cli = Cli::parse_from(["schemaui", "web", flag, expected]);

        let Some(Commands::Web(cmd)) = cli.command else {
            panic!("expected explicit web subcommand");
        };

        assert_eq!(cmd.host.to_string(), expected);
    }
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

#[test]
fn version_flags_short_circuit_parsing() {
    for flag in ["--version", "-V"] {
        let exit = Cli::try_parse_from(["schemaui", flag])
            .expect_err("version should short-circuit parsing");
        assert!(exit.status.is_ok(), "{flag} should exit successfully");
        assert!(
            exit.output.contains(env!("CARGO_PKG_VERSION")),
            "{flag} output should include package version: {}",
            exit.output
        );
    }
}

#[test]
fn bare_lowercase_v_is_not_a_version_alias() {
    let exit = Cli::try_parse_from(["schemaui", "-v"]).expect_err("-v should fail to parse");

    assert!(
        exit.status.is_err(),
        "-v should be treated as an unknown flag"
    );
    assert!(
        exit.output.contains("unexpected argument '-v'"),
        "unexpected output: {}",
        exit.output
    );
}

#[test]
fn hyphen_prefixed_option_values_are_allowed_for_title() {
    let cli = Cli::parse_from(["schemaui", "--title", "-v"]);

    assert!(cli.command.is_none(), "expected implicit default TUI mode");
    assert_eq!(cli.common.title.as_deref(), Some("-v"));
}
