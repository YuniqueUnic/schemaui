use assert_cmd::cargo::{self};
use predicates::str::contains;

#[test]
fn prints_help() {
    let mut cmd = cargo::cargo_bin_cmd!("schemaui");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(contains("schemaui"));
}
