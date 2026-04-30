#[cfg(any(feature = "tui", feature = "web"))]
use assert_cmd::cargo::cargo_bin_cmd;
#[cfg(any(feature = "tui", feature = "web"))]
use predicates::prelude::PredicateBooleanExt;
#[cfg(any(feature = "tui", feature = "web"))]
use predicates::str::contains;

#[cfg(feature = "tui")]
#[test]
fn generates_bash_completion_script_with_known_subcommands() {
    let mut cmd = cargo_bin_cmd!("schemaui");
    cmd.args(["completion", "bash"]).assert().success().stdout(
        contains("completion")
            .and(contains("tui"))
            .and(contains("tui-snapshot")),
    );
}

#[cfg(feature = "web")]
#[test]
fn generated_completion_mentions_web_commands_when_enabled() {
    let mut cmd = cargo_bin_cmd!("schemaui");
    cmd.args(["completion", "bash"])
        .assert()
        .success()
        .stdout(contains("web").and(contains("web-snapshot")));
}
