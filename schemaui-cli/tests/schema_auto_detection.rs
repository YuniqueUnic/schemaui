use std::fs;
#[cfg(all(feature = "remote-schema", any(feature = "toml", feature = "tui")))]
use std::io::{Read, Write};
#[cfg(all(feature = "remote-schema", any(feature = "toml", feature = "tui")))]
use std::net::TcpListener;
use std::path::{Path, PathBuf};
#[cfg(all(feature = "remote-schema", any(feature = "toml", feature = "tui")))]
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use schemaui_cli::cli::CommonArgs;
use schemaui_cli::session::prepare_session;
use serde_json::{Value, json};

#[cfg(all(feature = "remote-schema", feature = "tui"))]
use schemaui_cli::cli::TuiSnapshotCommand;
#[cfg(feature = "web")]
use schemaui_cli::cli::WebSnapshotCommand;

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!(
        "schemaui_schema_auto_{label}_{}_{}",
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

fn base_args(schema: Option<String>, config: Option<String>) -> CommonArgs {
    CommonArgs {
        schema,
        config,
        title: None,
        description: None,
        outputs: vec![],
        temp_file: None,
        no_temp_file: true,
        no_pretty: false,
        force: false,
    }
}

#[cfg(all(feature = "remote-schema", any(feature = "toml", feature = "tui")))]
fn spawn_schema_server(schema: Value) -> (String, thread::JoinHandle<()>) {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind schema server");
    let addr = listener.local_addr().expect("local addr");
    let url = format!("http://{addr}/schema.json");
    let body = serde_json::to_vec(&schema).expect("serialize schema body");

    let handle = thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept request");
        let mut buffer = [0_u8; 2048];
        let _ = stream.read(&mut buffer);
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            body.len()
        );
        stream
            .write_all(response.as_bytes())
            .expect("write response head");
        stream.write_all(&body).expect("write response body");
    });

    (url, handle)
}

#[cfg(all(feature = "web", feature = "remote-schema", feature = "toml"))]
fn opaque_object_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Opaque Object Schema",
        "type": "object",
        "properties": {
            "permissions": {
                "allOf": [
                    { "$ref": "#/definitions/PermissionsToml" }
                ],
                "description": "Named permission profiles."
            },
            "features": {
                "type": "object",
                "properties": {
                    "apps": { "type": "boolean" }
                }
            }
        },
        "definitions": {
            "PermissionsToml": {
                "type": "object"
            }
        }
    })
}

#[test]
fn prepare_session_uses_json_root_schema_and_strips_metadata_from_defaults() {
    let temp = unique_temp_dir("json_root");
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Local Schema",
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        },
        "required": ["name"]
    });
    let schema_path = temp.join("schema.json");
    let config_path = temp.join("config.json");
    write_json(&schema_path, &schema);
    write_json(
        &config_path,
        &json!({
            "$schema": "./schema.json",
            "name": "alice"
        }),
    );

    let session = prepare_session(&base_args(
        None,
        Some(config_path.to_string_lossy().into_owned()),
    ))
    .expect("prepare session");

    assert_eq!(session.schema, schema);
    assert_eq!(session.defaults, Some(json!({ "name": "alice" })));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn prepare_session_preserves_cli_description_override() {
    let temp = unique_temp_dir("description_override");
    let schema_path = temp.join("schema.json");
    let config_path = temp.join("config.json");
    write_json(
        &schema_path,
        &json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Local Schema",
            "description": "Schema description",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        }),
    );
    write_json(&config_path, &json!({ "name": "alice" }));

    let mut args = base_args(
        Some(schema_path.to_string_lossy().into_owned()),
        Some(config_path.to_string_lossy().into_owned()),
    );
    args.description = Some("CLI description".to_string());

    let session = prepare_session(&args).expect("prepare session");

    assert_eq!(session.description.as_deref(), Some("CLI description"));

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn prepare_session_reports_missing_file_like_config_instead_of_inlining_it() {
    let temp = unique_temp_dir("missing_config");
    let missing_path = temp.join("missing.json");

    let err = prepare_session(&base_args(
        None,
        Some(missing_path.to_string_lossy().into_owned()),
    ))
    .expect_err("missing file-like config should fail");
    let message = err.to_string();

    assert!(
        message.contains("failed to load config from"),
        "missing file should be surfaced as a file-loading error: {message}"
    );
    assert!(
        message.contains("missing.json"),
        "error should include the missing file path: {message}"
    );
    assert!(
        !message.contains("root schema must describe an object"),
        "missing file should not fall back into bogus schema inference: {message}"
    );

    let _ = fs::remove_dir_all(temp);
}

#[test]
fn prepare_session_accepts_inline_config_payloads_that_look_like_documents() {
    let temp = unique_temp_dir("inline_config_payload");
    let schema_path = temp.join("schema.json");
    write_json(
        &schema_path,
        &json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "type": "object",
            "properties": {
                "name": { "type": "string" }
            }
        }),
    );

    let session = prepare_session(&base_args(
        Some(schema_path.to_string_lossy().into_owned()),
        Some("name: alice".to_string()),
    ))
    .expect("inline yaml config should still be accepted");

    assert_eq!(session.defaults, Some(json!({ "name": "alice" })));

    let _ = fs::remove_dir_all(temp);
}

#[cfg(feature = "yaml")]
#[test]
fn prepare_session_prefers_explicit_local_schema_over_yaml_header_declaration() {
    let temp = unique_temp_dir("yaml_override");
    let explicit_schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Explicit Local Schema",
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        }
    });
    let explicit_schema_path = temp.join("explicit.schema.json");
    let config_path = temp.join("config.yaml");
    write_json(&explicit_schema_path, &explicit_schema);
    fs::write(
        &config_path,
        "# yaml-language-server: $schema=http://127.0.0.1:9/should-not-be-fetched.json\nname: alice\n",
    )
    .expect("write yaml config");

    let session = prepare_session(&base_args(
        Some(explicit_schema_path.to_string_lossy().into_owned()),
        Some(config_path.to_string_lossy().into_owned()),
    ))
    .expect("prepare session");

    assert_eq!(session.schema, explicit_schema);
    assert_eq!(session.defaults, Some(json!({ "name": "alice" })));

    let _ = fs::remove_dir_all(temp);
}

#[cfg(all(feature = "remote-schema", feature = "toml"))]
#[test]
fn prepare_session_uses_remote_toml_schema_directive() {
    let temp = unique_temp_dir("toml_remote");
    let remote_schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Remote TOML Schema",
        "type": "object",
        "properties": {
            "port": { "type": "integer" }
        },
        "required": ["port"]
    });
    let (schema_url, handle) = spawn_schema_server(remote_schema.clone());
    let config_path = temp.join("config.toml");
    fs::write(
        &config_path,
        format!("#:schema {schema_url}\nport = 8080\n"),
    )
    .expect("write toml config");

    let session = prepare_session(&base_args(
        None,
        Some(config_path.to_string_lossy().into_owned()),
    ))
    .expect("prepare session");

    handle.join().expect("schema server thread");

    assert_eq!(session.schema, remote_schema);
    assert_eq!(session.defaults, Some(json!({ "port": 8080 })));

    let _ = fs::remove_dir_all(temp);
}

#[cfg(all(feature = "remote-schema", feature = "tui"))]
#[test]
fn tui_snapshot_accepts_explicit_remote_schema() {
    let temp = unique_temp_dir("tui_snapshot_remote");
    let out_dir = temp.join("generated");
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Remote TUI Snapshot Schema",
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean" }
        }
    });
    let (schema_url, handle) = spawn_schema_server(schema);
    let config_path = temp.join("config.json");
    write_json(&config_path, &json!({ "enabled": true }));

    schemaui_cli::tui::run_snapshot_cli(TuiSnapshotCommand {
        common: base_args(
            Some(schema_url),
            Some(config_path.to_string_lossy().into_owned()),
        ),
        out_dir: out_dir.clone(),
        tui_fn: "tui_artifacts".to_string(),
        form_fn: "tui_form_schema".to_string(),
        layout_fn: "tui_layout_nav".to_string(),
    })
    .expect("run tui snapshot");

    handle.join().expect("schema server thread");

    assert!(out_dir.join("tui_artifacts.rs").exists());
    assert!(out_dir.join("tui_form_schema.rs").exists());
    assert!(out_dir.join("tui_layout_nav.rs").exists());

    let _ = fs::remove_dir_all(temp);
}

#[cfg(all(feature = "web", feature = "yaml"))]
#[test]
fn web_snapshot_accepts_yaml_fallback_local_schema_declaration() {
    let temp = unique_temp_dir("web_snapshot_local");
    let schema = json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "title": "Web Snapshot Local Schema",
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        }
    });
    let schema_path = temp.join("schema.json");
    let config_path = temp.join("config.yaml");
    let out_dir = temp.join("snapshots");
    write_json(&schema_path, &schema);
    fs::write(&config_path, "# @schema ./schema.json\nname: \"bob\"\n").expect("write yaml config");

    schemaui_cli::web::run_snapshot_cli(WebSnapshotCommand {
        common: base_args(None, Some(config_path.to_string_lossy().into_owned())),
        out_dir: out_dir.clone(),
        ts_export: "SessionSnapshot".to_string(),
    })
    .expect("run web snapshot");

    let snapshot_path = out_dir.join("session_snapshot.json");
    let snapshot: Value =
        serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("read web snapshot json"))
            .expect("parse web snapshot json");

    assert_eq!(snapshot["title"], "Web Snapshot Local Schema");
    assert_eq!(snapshot["data"], json!({ "name": "bob" }));
    assert!(out_dir.join("session_snapshot.ts").exists());

    let _ = fs::remove_dir_all(temp);
}

#[cfg(feature = "web")]
#[test]
fn web_snapshot_applies_cli_header_overrides() {
    let temp = unique_temp_dir("web_snapshot_override");
    let schema_path = temp.join("schema.json");
    let config_path = temp.join("config.json");
    let out_dir = temp.join("snapshots");
    write_json(
        &schema_path,
        &json!({
            "$schema": "http://json-schema.org/draft-07/schema#",
            "title": "Schema title",
            "description": "Schema description",
            "type": "object",
            "properties": {
                "enabled": { "type": "boolean" }
            }
        }),
    );
    write_json(&config_path, &json!({ "enabled": true }));

    let mut common = base_args(
        Some(schema_path.to_string_lossy().into_owned()),
        Some(config_path.to_string_lossy().into_owned()),
    );
    common.title = Some("CLI title".to_string());
    common.description = Some("CLI description".to_string());

    schemaui_cli::web::run_snapshot_cli(WebSnapshotCommand {
        common,
        out_dir: out_dir.clone(),
        ts_export: "SessionSnapshot".to_string(),
    })
    .expect("run web snapshot");

    let snapshot_path = out_dir.join("session_snapshot.json");
    let snapshot: Value =
        serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("read web snapshot json"))
            .expect("parse web snapshot json");

    assert_eq!(snapshot["title"], "CLI title");
    assert_eq!(snapshot["description"], "CLI description");

    let _ = fs::remove_dir_all(temp);
}

#[cfg(all(feature = "web", feature = "remote-schema", feature = "toml"))]
#[test]
fn web_snapshot_accepts_remote_schema_with_opaque_object_fields() {
    let temp = unique_temp_dir("web_snapshot_remote_opaque_object");
    let out_dir = temp.join("snapshots");
    let (schema_url, handle) = spawn_schema_server(opaque_object_schema());
    let config_path = temp.join("config.toml");
    fs::write(
        &config_path,
        format!("#:schema {schema_url}\n[features]\napps = true\n"),
    )
    .expect("write toml config");

    schemaui_cli::web::run_snapshot_cli(WebSnapshotCommand {
        common: base_args(None, Some(config_path.to_string_lossy().into_owned())),
        out_dir: out_dir.clone(),
        ts_export: "SessionSnapshot".to_string(),
    })
    .expect("run web snapshot");

    handle.join().expect("schema server thread");

    let snapshot_path = out_dir.join("session_snapshot.json");
    let snapshot: Value =
        serde_json::from_str(&fs::read_to_string(&snapshot_path).expect("read web snapshot json"))
            .expect("parse web snapshot json");

    assert_eq!(snapshot["title"], "Opaque Object Schema");
    assert_eq!(snapshot["data"], json!({ "features": { "apps": true } }));
    assert!(
        snapshot["ui_ast"]["roots"]
            .as_array()
            .expect("ui ast roots")
            .iter()
            .any(|node| node["pointer"] == "/permissions"),
        "opaque object fields should remain representable in the generated UI AST",
    );

    let _ = fs::remove_dir_all(temp);
}
