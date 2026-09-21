//! Contract tests for the Web Session API v1.
//!
//! These exist to fail when the *shape* of the contract moves — a renamed
//! field, a changed status code, a dropped endpoint — because a third-party
//! frontend has nothing else to rely on. They deliberately assert names and
//! codes rather than behaviour already covered by the runtime tests: when one
//! of these breaks, the question is "did we mean to break every client", not
//! "is the server working".

use std::net::IpAddr;
use std::thread;

use serde_json::{Value, json};
use ureq::Agent;

use super::harness::{reserve_port, test_http_agent, wait_until_ready};
use crate::web::session::ServeOptions;
use crate::{FrontendOptions, SchemaUI};

fn contract_schema() -> Value {
    json!({
        "title": "Contract",
        "type": "object",
        "properties": {
            "port": { "type": "integer", "minimum": 1 }
        },
        "required": ["port"]
    })
}

/// Start a session and run `checks` against it, then close it down.
///
/// The session is ended through the contract itself rather than by killing the
/// thread, so every test also exercises the shutdown path it would otherwise
/// leave untested.
fn with_session(checks: impl FnOnce(&Agent, &str)) {
    let port = reserve_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let schema = contract_schema();

    let handle = thread::spawn(move || {
        SchemaUI::from_schema(schema).run(FrontendOptions::Web(ServeOptions::new(
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
        .send(r#"{"data":{"port":1},"commit":true}"#)
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

#[test]
fn session_bootstrap_carries_every_field_a_frontend_needs() {
    with_session(|agent, base_url| {
        let session = get_json(agent, &format!("{base_url}/api/v1/session"));

        assert_eq!(session["api_version"], crate::API_VERSION);
        assert!(
            session["capabilities"].is_array(),
            "capabilities must always be present, even when empty: a client that \
             has to distinguish `missing` from `none` cannot negotiate"
        );
        assert!(session["ui_ast"]["roots"].is_array());
        assert!(
            session["layout"].is_object(),
            "layout is not optional: an always-present field typed as nullable \
             makes every client write a branch that never runs"
        );
        assert!(session["formats"].is_array());
        assert_eq!(session["title"], "Contract");
        assert_eq!(session["draft_restored"], false);
        assert_eq!(session["expires_in_ms"], Value::Null);
    });
}

#[test]
fn the_raw_schema_is_reachable_for_frontends_that_render_it_themselves() {
    with_session(|agent, base_url| {
        let schema = get_json(agent, &format!("{base_url}/api/v1/schema"));
        assert_eq!(schema["properties"]["port"]["minimum"], 1);
        assert_eq!(schema["required"][0], "port");
    });
}

#[test]
fn validation_reports_the_failing_pointer_and_a_message() {
    with_session(|agent, base_url| {
        let response = agent
            .post(format!("{base_url}/api/v1/validate"))
            .content_type("application/json")
            .send(r#"{"data":{"port":0}}"#)
            .expect("post validate request");
        assert_eq!(response.status().as_u16(), 200);

        let raw = response
            .into_body()
            .read_to_string()
            .expect("validation body should be readable");
        let body: Value = serde_json::from_str(&raw).expect("validation response should be JSON");
        assert_eq!(body["ok"], false);
        assert_eq!(body["errors"][0]["pointer"], "/port");
        assert!(
            body["errors"][0]["message"]
                .as_str()
                .is_some_and(|message| !message.is_empty()),
            "an error without a message is an error a user cannot act on"
        );
    });
}

#[test]
fn unversioned_paths_are_gone_rather_than_quietly_aliased() {
    with_session(|agent, base_url| {
        for path in ["/api/session", "/api/validate", "/api/preview", "/api/exit"] {
            let status = agent
                .get(format!("{base_url}{path}"))
                .call()
                .expect("request should reach the server")
                .status()
                .as_u16();
            assert_eq!(
                status, 404,
                "{path} should be gone: a client pointed at the old contract has to \
                 find out now, not on the one request whose shape changed"
            );
        }
    });
}

#[test]
fn a_session_without_a_theme_does_not_pretend_to_have_one() {
    with_session(|agent, base_url| {
        let session = get_json(agent, &format!("{base_url}/api/v1/session"));
        let capabilities = session["capabilities"].as_array().expect("capabilities");
        assert!(!capabilities.iter().any(|capability| capability == "theme"));

        let status = agent
            .get(format!("{base_url}/api/v1/theme.css"))
            .call()
            .expect("request should reach the server")
            .status()
            .as_u16();
        assert_eq!(
            status, 404,
            "an unadvertised capability must not be silently routable"
        );
    });
}
