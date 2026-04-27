use assert_cmd::cargo::{self};
use predicates::str::contains;

#[test]
fn prints_help() {
    let mut cmd = cargo::cargo_bin_cmd!("schemaui");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(contains("schemaui"))
        .stdout(contains("--description"));
}

#[test]
fn prints_version() {
    let mut cmd = cargo::cargo_bin_cmd!("schemaui");
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(contains(env!("CARGO_PKG_VERSION")));
}
