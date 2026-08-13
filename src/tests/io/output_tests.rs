use crate::io::output::OutputOptions;
use crate::{DocumentFormat, parse_document_str};
use serde_json::json;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn writes_to_stdout_noop_when_not_configured() {
    let options = OutputOptions {
        format: DocumentFormat::default(),
        pretty: true,
        destinations: Vec::new(),
    };
    crate::io::output::emit(&json!({"ok": true}), &options).unwrap();
}

#[test]
fn renders_payload_without_writing() {
    let options = OutputOptions::default();
    let payload = options.render(&json!({"ok": true})).unwrap();
    let parsed = parse_document_str(&payload, options.format).unwrap();
    assert_eq!(parsed, json!({"ok": true}));
}

#[test]
fn writes_to_file_destination() {
    let dir = std::env::temp_dir();
    let filename = format!(
        "schemaui-test-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    );
    let path = dir.join(filename);
    let options = OutputOptions {
        format: DocumentFormat::default(),
        pretty: true,
        destinations: vec![crate::io::output::OutputDestination::file(&path)],
    };
    crate::io::output::emit(&json!({"ok": true}), &options).unwrap();
    let contents = fs::read_to_string(&path).unwrap();
    let parsed = parse_document_str(&contents, options.format).unwrap();
    assert_eq!(parsed, json!({"ok": true}));
    let _ = fs::remove_file(path);
}

#[cfg(feature = "toml")]
fn render_toml(value: &serde_json::Value, pretty: bool) -> anyhow::Result<String> {
    OutputOptions::new(DocumentFormat::Toml)
        .with_pretty(pretty)
        .render(value)
}

#[cfg(feature = "toml")]
#[test]
fn toml_omits_null_object_properties() {
    let value = json!({"myprop": null});
    let pretty = render_toml(&value, true).expect("pretty toml should omit object nulls");
    let compact = render_toml(&value, false).expect("compact toml should omit object nulls");

    assert!(!pretty.contains("myprop"), "pretty payload: {pretty:?}");
    assert!(!compact.contains("myprop"), "compact payload: {compact:?}");
    assert_eq!(
        parse_document_str(&pretty, DocumentFormat::Toml).unwrap(),
        json!({})
    );
    assert_eq!(
        parse_document_str(&compact, DocumentFormat::Toml).unwrap(),
        json!({})
    );
}

#[cfg(feature = "toml")]
#[test]
fn toml_omits_nested_null_object_properties() {
    let value = json!({"a": {"b": null, "c": 1}});
    let payload = render_toml(&value, true).expect("nested object nulls should be omitted");
    assert_eq!(
        parse_document_str(&payload, DocumentFormat::Toml).unwrap(),
        json!({"a": {"c": 1}})
    );
}

#[cfg(feature = "toml")]
#[test]
fn toml_rejects_null_array_items_with_pointer() {
    let err = render_toml(&json!({"items": [1, null]}), true)
        .expect_err("array nulls must fail instead of dropping items");
    assert!(
        err.to_string()
            .contains("TOML cannot represent null at /items/1"),
        "unexpected error: {err}"
    );
}

#[cfg(feature = "toml")]
#[test]
fn toml_rejects_root_null_with_pointer() {
    let err = render_toml(&serde_json::Value::Null, true)
        .expect_err("root null must remain unrepresentable");
    assert!(
        err.to_string().contains("TOML cannot represent null at /"),
        "unexpected error: {err}"
    );
}

#[cfg(all(feature = "json", feature = "toml"))]
#[test]
fn json_preview_keeps_null_values() {
    let payload = OutputOptions::new(DocumentFormat::Json)
        .with_pretty(true)
        .render(&json!({"myprop": null}))
        .expect("json should keep null");
    assert_eq!(
        parse_document_str(&payload, DocumentFormat::Json).unwrap(),
        json!({"myprop": null})
    );
}

#[cfg(all(feature = "yaml", feature = "toml"))]
#[test]
fn yaml_preview_keeps_null_values() {
    let payload = OutputOptions::new(DocumentFormat::Yaml)
        .render(&json!({"myprop": null}))
        .expect("yaml should keep null");
    assert_eq!(
        parse_document_str(&payload, DocumentFormat::Yaml).unwrap(),
        json!({"myprop": null})
    );
}
