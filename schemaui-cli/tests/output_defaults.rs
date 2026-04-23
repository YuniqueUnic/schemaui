use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use schemaui::OutputDestination;
use schemaui_cli::cli::CommonArgs;
use schemaui_cli::session::prepare_session;
use serde_json::{Value, json};

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "schemaui_output_defaults_{label}_{}_{}",
        std::process::id(),
        nanos
    ));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_json(path: &Path, value: &Value) {
    fs::write(
        path,
        serde_json::to_vec_pretty(value).expect("serialize json"),
    )
    .expect("write json file");
}

fn base_args(schema: String, config: String) -> CommonArgs {
    CommonArgs {
        schema: Some(schema),
        config: Some(config),
        title: None,
        description: None,
        outputs: vec![],
        temp_file: None,
        no_temp_file: false,
        no_pretty: false,
        force: false,
    }
}

#[test]
fn prepare_session_defaults_to_stdout_when_no_output_is_given() {
    let temp = unique_temp_dir("stdout");
    let schema_path = temp.join("schema.json");
    let config_path = temp.join("config.json");
    write_json(
        &schema_path,
        &json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" }
            }
        }),
    );
    write_json(&config_path, &json!({ "enabled": true }));

    let session = prepare_session(&base_args(
        schema_path.to_string_lossy().into_owned(),
        config_path.to_string_lossy().into_owned(),
    ))
    .expect("prepare session");

    let output = session.output.expect("output settings");
    assert_eq!(output.destinations.len(), 1);
    assert!(matches!(output.destinations[0], OutputDestination::Stdout));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn prepare_session_respects_explicit_temp_file_override() {
    let temp = unique_temp_dir("temp_file");
    let schema_path = temp.join("schema.json");
    let config_path = temp.join("config.json");
    let explicit_output = temp.join("custom-output.json");
    write_json(
        &schema_path,
        &json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" }
            }
        }),
    );
    write_json(&config_path, &json!({ "enabled": true }));

    let mut args = base_args(
        schema_path.to_string_lossy().into_owned(),
        config_path.to_string_lossy().into_owned(),
    );
    args.temp_file = Some(explicit_output.clone());

    let session = prepare_session(&args).expect("prepare session");

    let output = session.output.expect("output settings");
    assert_eq!(output.destinations.len(), 1);
    assert!(matches!(
        &output.destinations[0],
        OutputDestination::File(path) if path == &explicit_output
    ));

    let _ = fs::remove_dir_all(temp);
}
