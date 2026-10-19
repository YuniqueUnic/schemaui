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
    with_configured_session(|ui| ui, checks);
}

/// [`with_session`], with a say in how the session is built.
fn with_configured_session(
    configure: impl FnOnce(SchemaUI) -> SchemaUI + Send + 'static,
    checks: impl FnOnce(&Agent, &str),
) {
    let port = reserve_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let schema = contract_schema();

    let handle = thread::spawn(move || {
        configure(SchemaUI::from_schema(schema)).run(FrontendOptions::Web(ServeOptions::new(
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

fn post_json(agent: &Agent, url: &str, body: &str) -> (u16, Value) {
    let mut response = agent
        .post(url)
        .content_type("application/json")
        .send(body)
        .expect("request should reach the server");
    let status = response.status().as_u16();
    let raw = response
        .body_mut()
        .read_to_string()
        .expect("response body should be readable");
    (
        status,
        serde_json::from_str(&raw).unwrap_or_else(|_| Value::String(raw)),
    )
}

#[test]
fn saving_an_untouched_document_succeeds() {
    with_session(|agent, base_url| {
        let (status, body) = post_json(
            agent,
            &format!("{base_url}/api/v1/save"),
            r#"{"data":{"port":1}}"#,
        );
        assert_eq!(status, 200);
        assert_eq!(body["status"], "saved");

        // Saving again with identical data is not an error either: a user who
        // presses Save twice must not be told the second one failed.
        let (status, _) = post_json(
            agent,
            &format!("{base_url}/api/v1/save"),
            r#"{"data":{"port":1}}"#,
        );
        assert_eq!(status, 200);
    });
}

#[test]
fn saving_an_empty_document_succeeds() {
    with_session(|agent, base_url| {
        let (status, body) = post_json(agent, &format!("{base_url}/api/v1/save"), r#"{"data":{}}"#);
        assert_eq!(status, 200);
        assert_eq!(body["status"], "saved");
    });
}

#[test]
fn a_malformed_save_is_refused_without_closing_the_session() {
    with_session(|agent, base_url| {
        let (status, _) = post_json(agent, &format!("{base_url}/api/v1/save"), r#"{"nope":1}"#);
        assert!(
            (400..500).contains(&status),
            "a body with no `data` is the client's mistake, got {status}"
        );

        // The session survives it: the next well-formed save still works.
        let (status, _) = post_json(
            agent,
            &format!("{base_url}/api/v1/save"),
            r#"{"data":{"port":2}}"#,
        );
        assert_eq!(status, 200);
    });
}

#[test]
fn a_draft_that_cannot_be_written_is_reported_with_its_reason() {
    // A path *under a regular file* can never be created, on any platform.
    let blocker =
        std::env::temp_dir().join(format!("schemaui-draft-blocker-{}", std::process::id()));
    std::fs::write(&blocker, b"not a directory").expect("create blocker file");
    let draft_path = blocker.join("draft.json");

    with_configured_session(
        move |ui| ui.with_draft(crate::DraftStore::at(draft_path)),
        |agent, base_url| {
            let (status, body) = post_json(
                agent,
                &format!("{base_url}/api/v1/save"),
                r#"{"data":{"port":1}}"#,
            );
            assert_eq!(status, 500);
            let reason = body["error"].as_str().unwrap_or_default();
            assert!(
                reason.contains("draft"),
                "the frontend shows this text to the user, so it has to say what failed: {reason:?}"
            );
        },
    );

    let _ = std::fs::remove_file(&blocker);
}

#[test]
fn preview_renders_every_advertised_format_for_the_same_document() {
    with_session(|agent, base_url| {
        let session = get_json(agent, &format!("{base_url}/api/v1/session"));
        let formats = session["formats"].as_array().expect("formats");
        assert!(!formats.is_empty());

        for format in formats {
            let format = format.as_str().expect("format keyword");
            let body = format!(r#"{{"data":{{"port":8080}},"format":"{format}","pretty":true}}"#);
            let (status, response) = post_json(agent, &format!("{base_url}/api/v1/preview"), &body);
            assert_eq!(status, 200, "{format} preview should render");
            assert!(
                response["payload"]
                    .as_str()
                    .is_some_and(|payload| payload.contains("8080")),
                "{format} preview should carry the document: {response}"
            );
        }
    });
}

#[test]
fn preview_of_an_unknown_format_is_a_client_error_with_a_message() {
    with_session(|agent, base_url| {
        let (status, body) = post_json(
            agent,
            &format!("{base_url}/api/v1/preview"),
            r#"{"data":{"port":1},"format":"xml","pretty":true}"#,
        );
        assert_eq!(status, 400);
        assert!(
            body["error"].as_str().is_some_and(|m| !m.is_empty()),
            "an empty error is a preview pane with nothing to show: {body}"
        );
    });
}
