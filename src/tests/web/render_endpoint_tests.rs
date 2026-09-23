//! `POST /api/v1/render`: on-demand diagram rendering.
//!
//! The contract under test: 200 with `{svg}` for a renderable source, 400 for
//! a malformed request (unknown kind or theme), 422 with the renderer's
//! message for a source that will not parse. The route only exists — and the
//! `content_render` capability is only advertised — in builds with the
//! `mermaid` feature, so the whole file is gated on it.

use std::net::{IpAddr, SocketAddr};
use std::thread;

use serde_json::{Value, json};
use ureq::Agent;

use super::harness::{reserve_port, test_http_agent, wait_until_ready};
use crate::web::session::ServeOptions;
use crate::{FrontendOptions, SchemaUI, SessionOutcome};

fn post_render(agent: &Agent, base_url: &str, body: Value) -> (u16, Value) {
    let mut response = agent
        .post(format!("{base_url}/api/v1/render"))
        .content_type("application/json")
        .send(serde_json::to_string(&body).expect("serialize render payload"))
        .expect("post render request");
    let status = response.status().as_u16();
    let body: Value = serde_json::from_str(
        &response
            .body_mut()
            .read_to_string()
            .expect("render response should be readable"),
    )
    .expect("render response should be json");
    (status, body)
}

#[test]
fn a_valid_diagram_renders_and_the_capability_is_advertised() {
    let (agent, base_url, handle) = start_session();
    let response = agent
        .get(format!("{base_url}/api/v1/session"))
        .call()
        .expect("get session");
    let session: Value =
        serde_json::from_str(&response.into_body().read_to_string().expect("session body"))
            .expect("session json");
    assert!(
        session["capabilities"]
            .as_array()
            .expect("capabilities")
            .iter()
            .any(|c| c == "content_render"),
        "the route exists, so the capability must be advertised: {session}"
    );

    let (status, body) = post_render(
        &agent,
        &base_url,
        json!({ "kind": "mermaid", "source": "flowchart LR; A-->B", "theme": "dark" }),
    );
    assert_eq!(status, 200);
    assert!(
        body["svg"]["body"]
            .as_str()
            .expect("svg body")
            .contains("<svg"),
        "{body}"
    );

    close_session(&agent, &base_url, handle);
}

#[test]
fn an_unparsable_diagram_is_a_422_with_the_renderers_message() {
    let (agent, base_url, handle) = start_session();
    let (status, body) = post_render(
        &agent,
        &base_url,
        json!({ "kind": "mermaid", "source": "flowchart LR; A-->", "theme": "light" }),
    );
    assert_eq!(status, 422);
    let message = body["error"]["message"].as_str().expect("error message");
    assert!(
        !message.is_empty(),
        "the author gets the renderer's words: {body}"
    );
    close_session(&agent, &base_url, handle);
}

#[test]
fn an_unknown_kind_or_theme_is_a_400() {
    let (agent, base_url, handle) = start_session();

    let (status, body) = post_render(
        &agent,
        &base_url,
        json!({ "kind": "dot", "source": "digraph { a -> b }", "theme": "light" }),
    );
    assert_eq!(status, 400);
    assert!(body["error"]["message"].is_string(), "{body}");

    let (status, body) = post_render(
        &agent,
        &base_url,
        json!({ "kind": "mermaid", "source": "flowchart LR; A-->B", "theme": "forest" }),
    );
    assert_eq!(status, 400);
    assert!(body["error"]["message"].is_string(), "{body}");

    close_session(&agent, &base_url, handle);
}

fn start_session() -> (
    Agent,
    String,
    thread::JoinHandle<anyhow::Result<SessionOutcome>>,
) {
    let port = reserve_port();
    let handle = thread::spawn(move || {
        SchemaUI::from_schema(json!({
            "type": "object",
            "properties": { "name": { "type": "string" } }
        }))
        .run(FrontendOptions::Web(ServeOptions::new(
            IpAddr::from([127, 0, 0, 1]),
            port,
        )))
    });
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");
    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);
    (agent, base_url, handle)
}

fn close_session(
    agent: &Agent,
    base_url: &str,
    handle: thread::JoinHandle<anyhow::Result<SessionOutcome>>,
) {
    let response = agent
        .post(format!("{base_url}/api/v1/exit"))
        .content_type("application/json")
        .send(r#"{"data":{},"commit":true}"#)
        .expect("post exit request");
    assert_eq!(response.status().as_u16(), 200);
    handle
        .join()
        .expect("web frontend thread should not panic")
        .expect("web frontend should return");
}
