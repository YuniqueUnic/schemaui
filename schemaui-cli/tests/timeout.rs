//! End-to-end coverage for `--timeout`.
//!
//! The point of the flag is that a session nobody answers ends itself and says
//! so in a way a script can act on. That is only observable by running the real
//! binary against a real (unanswered) session, so these tests drive the CLI
//! rather than the library.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo;
use predicates::str::contains;

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "schemaui_timeout_{label}_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_schema(dir: &Path) -> PathBuf {
    let path = dir.join("schema.json");
    let schema = serde_json::json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": { "name": { "type": "string" } }
    });
    fs::write(
        &path,
        serde_json::to_vec_pretty(&schema).expect("serialize schema"),
    )
    .expect("write schema");
    path
}

#[test]
fn help_documents_the_timeout_flag_and_its_exit_code() {
    let mut cmd = cargo::cargo_bin_cmd!("schemaui");
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(contains("--timeout"))
        .stdout(contains("code 4"));
}

#[test]
fn an_unanswered_web_session_exits_with_the_timeout_code_and_writes_nothing() {
    let dir = unique_temp_dir("web");
    let schema = write_schema(&dir);
    let answer = dir.join("answer.json");

    let mut cmd = cargo::cargo_bin_cmd!("schemaui");
    cmd.args(["web", "--timeout", "1"])
        .arg("--schema")
        .arg(&schema)
        .arg("--output")
        .arg(&answer)
        .assert()
        .code(4)
        .stderr(contains("timed out"));

    assert!(
        !answer.exists(),
        "a timed-out session must not leave an output file behind at {}",
        answer.display()
    );
}

#[test]
fn the_timeout_code_is_distinct_from_a_normal_failure() {
    // A missing schema fails for an ordinary reason and must not be mistaken
    // for a timeout by a caller that switches on the exit code.
    let mut cmd = cargo::cargo_bin_cmd!("schemaui");
    cmd.args([
        "web",
        "--timeout",
        "1",
        "--schema",
        "/nonexistent/schema.json",
    ])
    .assert()
    .failure()
    .code(predicates::ord::ne(4));
}
