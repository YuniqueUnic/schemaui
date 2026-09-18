use anyhow::Result;
use serde_json::{Value, json};

use crate::SchemaUI;
use crate::core::frontend::{Frontend, FrontendContext, SessionOutcome};
#[cfg(feature = "tui")]
use crate::precompile::build_ui_artifact_bundle;
#[cfg(feature = "tui")]
use crate::tui::app::UiOptions;

struct CaptureFrontend;

impl Frontend for CaptureFrontend {
    fn run(self, ctx: FrontendContext) -> Result<SessionOutcome> {
        // Read through the public accessor rather than the raw `deadline`, so
        // the pipeline tests also cover what a third-party frontend would use.
        let remaining = ctx.time_remaining();
        Ok(SessionOutcome::Completed(json!({
            "title": ctx.title,
            "description": ctx.description,
            "data": ctx.initial_data,
            "schema": ctx.schema,
            "deadline_remaining_ms": remaining.map(|left| left.as_millis() as u64),
        })))
    }
}

/// Unwrap a completed session for assertions. `CaptureFrontend` has no
/// deadline, so a timeout here means the plumbing is broken.
fn completed(outcome: SessionOutcome) -> Value {
    match outcome {
        SessionOutcome::Completed(value) => value,
        SessionOutcome::TimedOut => panic!("capture frontend never times out"),
    }
}

#[test]
fn schema_ui_accepts_inline_data_and_validation_schema() {
    let schema = r#"
        {
            "title": "Service Config",
            "description": "Runtime-validated configuration",
            "type": "object",
            "properties": {
                "host": { "type": "string" },
                "port": { "type": "integer" }
            }
        }
    "#;

    let result = SchemaUI::new(r#"{ "host": "127.0.0.1", "port": 8080 }"#)
        .with_schema(schema)
        .run_with_frontend(CaptureFrontend)
        .map(completed)
        .expect("inline document sources should parse at runtime");

    assert_eq!(result["title"], "Service Config");
    assert_eq!(result["description"], "Runtime-validated configuration");
    assert_eq!(result["data"], json!({ "host": "127.0.0.1", "port": 8080 }));
    assert_eq!(
        result["schema"]["properties"]["host"]["default"],
        "127.0.0.1"
    );
    assert_eq!(result["schema"]["properties"]["port"]["default"], 8080);
}

#[test]
fn schema_ui_infers_schema_from_data_when_no_validation_schema_is_provided() {
    let result = SchemaUI::new(json!({
        "enabled": true,
        "tags": ["blue", "green"]
    }))
    .run_with_frontend(CaptureFrontend)
    .map(completed)
    .expect("schema inference from config data should succeed");

    assert_eq!(
        result["data"],
        json!({
            "enabled": true,
            "tags": ["blue", "green"]
        })
    );
    assert_eq!(result["schema"]["type"], "object");
    assert_eq!(result["schema"]["properties"]["enabled"]["type"], "boolean");
    assert_eq!(result["schema"]["properties"]["tags"]["type"], "array");
}

#[test]
fn schema_ui_treats_schema_like_input_as_schema_when_no_explicit_validation_schema_exists() {
    let schema = json!({
        "title": "Schema-like input",
        "description": "Keep schema-first usage ergonomic",
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean" }
        }
    });

    let result = SchemaUI::new(schema)
        .run_with_frontend(CaptureFrontend)
        .map(completed)
        .expect("schema-like documents should be treated as validation schemas");

    assert_eq!(result["title"], "Schema-like input");
    assert_eq!(result["description"], "Keep schema-first usage ergonomic");
    assert_eq!(result["data"], json!({}));
    assert_eq!(result["schema"]["properties"]["enabled"]["type"], "boolean");
}

#[test]
fn schema_ui_description_override_wins_over_schema_description() {
    let schema = json!({
        "title": "Service Config",
        "description": "Schema description",
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean" }
        }
    });

    let result = SchemaUI::from_schema(schema)
        .with_title("CLI override")
        .with_description("Manual description")
        .run_with_frontend(CaptureFrontend)
        .map(completed)
        .expect("description overrides should flow into frontend context");

    assert_eq!(result["title"], "CLI override");
    assert_eq!(result["description"], "Manual description");
}

#[cfg(feature = "tui")]
#[test]
fn schema_ui_builds_tui_frontend_from_frontend_options_payload() {
    let schema = json!({
        "title": "Service Config",
        "type": "object",
        "properties": {
            "enabled": { "type": "boolean" }
        }
    });
    let defaults = json!({ "enabled": true });
    let bundle = build_ui_artifact_bundle(&schema, Some(&defaults))
        .expect("artifact bundle should build for TUI frontend preparation");
    let options = UiOptions::default()
        .with_auto_validate(false)
        .with_confirm_exit(false);

    let frontend = SchemaUI::new(defaults)
        .with_schema(schema)
        .with_ui_artifact_bundle(bundle.clone())
        .build_tui_frontend(options.clone());

    assert!(!frontend.options.auto_validate);
    assert!(!frontend.options.confirm_exit);
    assert_eq!(frontend.tui_artifacts, Some(bundle.tui));
}

#[test]
fn nullable_scalar_schema_marks_field_nullable_and_preserves_null_default() {
    let schema = json!({
        "type": "object",
        "properties": {
            "data-size": {
                "type": ["integer", "null"],
                "default": null
            },
            "bind-address-v6": {
                "type": ["string", "null"],
                "format": "ipv6",
                "default": null
            }
        }
    });

    let result = SchemaUI::from_schema(schema)
        .run_with_frontend(CaptureFrontend)
        .map(completed)
        .expect("nullable schema should build frontend context");

    let data = &result["schema"]["properties"]["data-size"];
    assert_eq!(data["default"], Value::Null);
    assert_eq!(data["type"], json!(["integer", "null"]));
}

#[test]
fn session_outcome_separates_a_saved_answer_from_a_timeout() {
    let value = json!({ "host": "127.0.0.1" });
    let completed = SessionOutcome::Completed(value.clone());

    assert_eq!(completed.value(), Some(&value));
    assert!(!completed.is_timed_out());
    assert_eq!(completed.into_value(), Some(value));

    let timed_out = SessionOutcome::TimedOut;

    // No value at all, deliberately: a deadline firing carries no answer, and
    // handing back the half-filled form would let a caller save it as one.
    assert!(timed_out.value().is_none());
    assert!(timed_out.is_timed_out());
    assert_eq!(timed_out.into_value(), None);
}

#[test]
fn schema_ui_with_timeout_arms_the_frontend_deadline() {
    let result = SchemaUI::from_schema(json!({ "type": "object" }))
        .with_timeout(std::time::Duration::from_secs(30))
        .run_with_frontend(CaptureFrontend)
        .map(completed)
        .expect("a timeout should not change how the schema is prepared");

    let remaining = result["deadline_remaining_ms"]
        .as_u64()
        .expect("the requested timeout should reach the frontend as a deadline");
    assert!(
        remaining <= 30_000,
        "deadline is {remaining}ms out, beyond the 30000ms that was requested"
    );
    assert!(
        remaining > 25_000,
        "only {remaining}ms of a 30s budget survived the pipeline"
    );
}

#[test]
fn schema_ui_without_a_timeout_leaves_the_frontend_unbounded() {
    let result = SchemaUI::from_schema(json!({ "type": "object" }))
        .run_with_frontend(CaptureFrontend)
        .map(completed)
        .expect("schema preparation should succeed");

    assert!(
        result["deadline_remaining_ms"].is_null(),
        "a session with no timeout must not report a deadline, got {}",
        result["deadline_remaining_ms"]
    );
}

#[test]
fn format_budget_speaks_in_the_largest_useful_unit() {
    use crate::core::frontend::format_budget;
    use std::time::Duration;

    assert_eq!(format_budget(Duration::from_secs(0)), "0s");
    assert_eq!(format_budget(Duration::from_secs(45)), "45s");
    // A whole minute must not read "1m0s": the announcement is prose, and the
    // trailing zero looks like a clock that stalled.
    assert_eq!(format_budget(Duration::from_secs(60)), "1m");
    assert_eq!(format_budget(Duration::from_secs(90)), "1m30s");
    assert_eq!(format_budget(Duration::from_secs(300)), "5m");
    assert_eq!(format_budget(Duration::from_secs(3_600)), "1h");
    assert_eq!(format_budget(Duration::from_secs(5_400)), "1h30m");
    // Seconds are dropped once an hour is on screen; "1h0m5s" is noise.
    assert_eq!(format_budget(Duration::from_secs(3_605)), "1h");
    // Sub-second remainders round up, so a budget measured a moment into the
    // session still reads as the value that was requested.
    assert_eq!(format_budget(Duration::from_millis(299_500)), "5m");
    assert_eq!(format_budget(Duration::from_millis(89_400)), "1m30s");
    assert_eq!(format_budget(Duration::from_millis(1)), "1s");
}

#[test]
fn frontend_context_reports_no_remaining_time_when_unbounded() {
    // No `with_timeout` call: an unbounded session must report `None` rather
    // than a zero-length deadline, which a frontend would render as 0:00 and
    // then immediately close.
    let result = SchemaUI::from_schema(json!({ "type": "object" }))
        .run_with_frontend(CaptureFrontend)
        .map(completed)
        .expect("schema preparation should succeed");

    assert!(result["deadline_remaining_ms"].is_null());
}

#[test]
fn frontend_context_saturates_at_zero_once_the_deadline_has_passed() {
    // A zero budget puts the deadline in the past by the time the frontend
    // runs. The accessor is saturating on purpose: a frontend can legitimately
    // draw one last frame during the shutdown path, and a plain subtraction
    // there would underflow and panic in debug builds.
    let result = SchemaUI::from_schema(json!({ "type": "object" }))
        .with_timeout(std::time::Duration::ZERO)
        .run_with_frontend(CaptureFrontend)
        .map(completed)
        .expect("schema preparation should succeed");

    assert_eq!(result["deadline_remaining_ms"], json!(0));
}
