//! Parity between the wasm exports and their HTTP counterparts.
//!
//! `docs/en/decisions/0005-wasm-core-and-hosting.md` commits to the two
//! staying aligned: "a shared fixture asserts they agree." These tests are
//! that fixture. They run the same schema/data through a real HTTP session
//! and through `wasm_core` directly, and diff the parts of the response each
//! side actually produces.

use std::net::IpAddr;
use std::thread;

use serde_json::{Value, json};
use ureq::Agent;

use super::harness::{reserve_port, test_http_agent, wait_until_ready};
use crate::web::session::ServeOptions;
use crate::{FrontendOptions, SchemaUI};

fn fixture_schema() -> Value {
    json!({
        "title": "Parity",
        "type": "object",
        "properties": {
            "name": { "type": "string" },
            "retries": { "type": "integer", "minimum": 0, "default": 3 },
            "mode": { "type": "string", "enum": ["fast", "safe"] }
        },
        "required": ["name"]
    })
}

fn fixture_data() -> Value {
    json!({ "name": "alice" })
}

fn with_session(schema: Value, data: Value, checks: impl FnOnce(&Agent, &str)) {
    let port = reserve_port();
    let base_url = format!("http://127.0.0.1:{port}");

    let handle = thread::spawn(move || {
        SchemaUI::from_schema_and_data(schema, data).run(FrontendOptions::Web(ServeOptions::new(
            IpAddr::from([127, 0, 0, 1]),
            port,
        )))
    });

    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);
    checks(&agent, &base_url);

    agent
        .post(format!("{base_url}/api/v1/exit"))
        .content_type("application/json")
        .send(r#"{"data":{"name":"alice"},"commit":true}"#)
        .expect("post exit request");
    handle
        .join()
        .expect("session thread should not panic")
        .expect("session should end cleanly");
}

fn get_json(agent: &Agent, url: &str) -> Value {
    let body = agent
        .get(url)
        .call()
        .expect("request should reach the server")
        .body_mut()
        .read_to_string()
        .expect("response body should be readable");
    serde_json::from_str(&body).expect("response should be JSON")
}

fn post_json(agent: &Agent, url: &str, payload: &Value) -> Value {
    let body = agent
        .post(url)
        .content_type("application/json")
        .send(payload.to_string())
        .expect("request should reach the server")
        .body_mut()
        .read_to_string()
        .expect("response body should be readable");
    serde_json::from_str(&body).expect("response should be JSON")
}

#[test]
fn build_ui_ast_matches_the_session_bootstrap() {
    with_session(fixture_schema(), fixture_data(), |agent, base_url| {
        let session = get_json(agent, &format!("{base_url}/api/v1/session"));

        let built = crate::wasm_core::build_ui_ast(fixture_schema(), fixture_data())
            .expect("wasm_core::build_ui_ast should succeed on the fixture");
        let built = serde_json::to_value(&built).expect("SessionBuild should serialize");

        assert_eq!(
            built["ui_ast"], session["ui_ast"],
            "buildUiAst must produce the exact ui_ast GET /api/v1/session does"
        );
        assert_eq!(
            built["layout"], session["layout"],
            "buildUiAst must produce the exact layout GET /api/v1/session does"
        );
        assert_eq!(
            built["data"], session["data"],
            "buildUiAst must echo the same data GET /api/v1/session does"
        );
        assert_eq!(
            built["formats"], session["formats"],
            "buildUiAst must offer the same document formats GET /api/v1/session does"
        );
    });
}

#[test]
fn validate_matches_post_validate_byte_for_byte() {
    with_session(fixture_schema(), fixture_data(), |agent, base_url| {
        let invalid = json!({ "retries": -1 });
        let http_result = post_json(
            agent,
            &format!("{base_url}/api/v1/validate"),
            &json!({ "data": invalid }),
        );

        let wasm_result = crate::wasm_core::validate(fixture_schema(), invalid)
            .expect("wasm_core::validate should succeed on a valid schema");
        let wasm_result = serde_json::to_value(&wasm_result).expect("ValidationResult serializes");

        assert_eq!(
            wasm_result, http_result,
            "validate must return exactly what POST /api/v1/validate returns, field names included"
        );
    });
}

#[test]
fn render_matches_post_preview_byte_for_byte() {
    with_session(fixture_schema(), fixture_data(), |agent, base_url| {
        let data = json!({ "name": "bob", "retries": 5 });
        let http_result = post_json(
            agent,
            &format!("{base_url}/api/v1/preview"),
            &json!({ "data": data, "format": "json", "pretty": true }),
        );

        let wasm_result = crate::wasm_core::render(data, "json", true)
            .expect("wasm_core::render should succeed for an enabled format");
        let wasm_result = serde_json::to_value(&wasm_result).expect("RenderResult serializes");

        assert_eq!(
            wasm_result, http_result,
            "render must return exactly what POST /api/v1/preview returns"
        );
    });
}

#[test]
fn schema_with_defaults_matches_the_raw_schema_endpoint() {
    with_session(fixture_schema(), fixture_data(), |agent, base_url| {
        let raw_schema = get_json(agent, &format!("{base_url}/api/v1/schema"));

        let merged = crate::wasm_core::schema_with_defaults(fixture_schema(), fixture_data());

        assert_eq!(
            merged, raw_schema,
            "schemaWithDefaults must merge data the same way session bootstrap does \
             before exposing it as the session's schema"
        );
    });
}
