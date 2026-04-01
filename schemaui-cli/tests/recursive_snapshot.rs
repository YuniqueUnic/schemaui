use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

use assert_cmd::cargo::{self};

fn unique_temp_dir(prefix: &str) -> std::path::PathBuf {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!(
        "schemaui_cli_{prefix}_{}_{}",
        std::process::id(),
        stamp
    ))
}

#[test]
fn tui_snapshot_handles_recursive_example_schema() {
    let schema_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("examples")
        .join("ultra-complex.schema.json");
    let out_dir = unique_temp_dir("recursive_snapshot");
    fs::create_dir_all(&out_dir).expect("temp output dir");

    let mut cmd = cargo::cargo_bin_cmd!("schemaui");
    cmd.args([
        "tui-snapshot",
        "--schema",
        schema_path.to_str().expect("schema path utf-8"),
        "--out-dir",
        out_dir.to_str().expect("out dir utf-8"),
    ])
    .assert()
    .success();

    for filename in [
        "tui_artifacts.rs",
        "tui_form_schema.rs",
        "tui_layout_nav.rs",
    ] {
        let path = out_dir.join(filename);
        let contents = fs::read_to_string(&path).expect("generated snapshot module readable");
        assert!(
            contents.contains("serde_json::from_str"),
            "expected generated constructor in {}",
            path.display()
        );
    }

    let _ = fs::remove_dir_all(&out_dir);
}
