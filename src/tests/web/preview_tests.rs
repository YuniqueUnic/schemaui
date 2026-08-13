use std::net::{IpAddr, SocketAddr, TcpListener};
use std::thread;
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use ureq::Agent;

use crate::web::session::ServeOptions;
use crate::{FrontendOptions, SchemaUI};

fn reserve_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
    let port = listener.local_addr().expect("listener addr").port();
    drop(listener);
    port
}

fn test_http_agent() -> Agent {
    Agent::new_with_config(
        Agent::config_builder()
            .http_status_as_error(false)
            .timeout_global(Some(Duration::from_secs(2)))
            .proxy(None)
            .build(),
    )
}

fn wait_until_ready(agent: &Agent, base_url: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let session_url = format!("{base_url}/api/session");

    loop {
        let outcome = match agent.get(&session_url).call() {
            Ok(response) if response.status().as_u16() == 200 => return,
            Ok(response) => format!("unexpected status: {}", response.status()),
            Err(err) => err.to_string(),
        };

        assert!(
            Instant::now() < deadline,
            "web session did not become ready at {base_url}; last outcome: {outcome}"
        );
        thread::sleep(Duration::from_millis(50));
    }
}

fn start_preview_session(schema: Value) -> (u16, thread::JoinHandle<anyhow::Result<Value>>) {
    let port = reserve_port();
    let handle = thread::spawn(move || {
        SchemaUI::from_schema(schema).run(FrontendOptions::Web(ServeOptions {
            host: IpAddr::from([127, 0, 0, 1]),
            port,
        }))
    });
    (port, handle)
}

fn post_preview(
    agent: &Agent,
    base_url: &str,
    data: Value,
    format: &str,
) -> ureq::http::Response<ureq::Body> {
    agent
        .post(format!("{base_url}/api/preview"))
        .content_type("application/json")
        .send(
            serde_json::to_string(&json!({
                "data": data,
                "format": format,
                "pretty": true
            }))
            .expect("serialize preview payload"),
        )
        .expect("post preview request")
}

fn close_session(agent: &Agent, base_url: &str, handle: thread::JoinHandle<anyhow::Result<Value>>) {
    let response = agent
        .post(format!("{base_url}/api/exit"))
        .content_type("application/json")
        .send(
            serde_json::to_string(&json!({
                "data": {},
                "commit": true
            }))
            .expect("serialize exit payload"),
        )
        .expect("post exit request");
    assert_eq!(response.status().as_u16(), 200);
    handle
        .join()
        .expect("web frontend thread should not panic")
        .expect("web frontend should return");
}

fn issue161_schema() -> Value {
    json!({
        "$schema": "http://json-schema.org/draft-07/schema#",
        "type": "object",
        "properties": {
            "myprop": {
                "type": "null"
            }
        },
        "propertyNames": true,
        "required": [],
        "additionalProperties": false
    })
}

#[cfg(feature = "toml")]
#[test]
fn preview_toml_omits_null_object_property_from_issue_schema() {
    let (port, handle) = start_preview_session(issue161_schema());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");
    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    let mut response = post_preview(&agent, &base_url, json!({"myprop": null}), "toml");
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = serde_json::from_str(
        &response
            .body_mut()
            .read_to_string()
            .expect("preview response should be readable"),
    )
    .expect("preview response should be json");
    let payload = body["payload"].as_str().expect("payload string");
    assert!(
        !payload.contains("myprop"),
        "toml preview should omit null object keys: {payload:?}"
    );
    assert!(
        !payload.contains("unsupported unit type"),
        "toml preview should not leak serde unit errors: {payload:?}"
    );

    close_session(&agent, &base_url, handle);
}

#[cfg(all(feature = "json", feature = "toml"))]
#[test]
fn preview_json_keeps_null_object_property() {
    let (port, handle) = start_preview_session(issue161_schema());
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");
    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    let mut response = post_preview(&agent, &base_url, json!({"myprop": null}), "json");
    assert_eq!(response.status().as_u16(), 200);
    let body: Value = serde_json::from_str(
        &response
            .body_mut()
            .read_to_string()
            .expect("preview response should be readable"),
    )
    .expect("preview response should be json");
    let payload = body["payload"].as_str().expect("payload string");
    let parsed: Value = serde_json::from_str(payload).expect("json payload parses");
    assert_eq!(parsed, json!({"myprop": null}));

    close_session(&agent, &base_url, handle);
}

#[cfg(feature = "toml")]
#[test]
fn preview_toml_rejects_null_array_items_with_readable_error() {
    let (port, handle) = start_preview_session(json!({
        "type": "object",
        "properties": {
            "items": {
                "type": "array",
                "items": { "type": ["integer", "null"] }
            }
        }
    }));
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");
    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    let mut response = post_preview(&agent, &base_url, json!({"items": [1, null]}), "toml");
    assert_eq!(response.status().as_u16(), 400);
    let body = response
        .body_mut()
        .read_to_string()
        .expect("error body should be readable");
    assert!(
        body.contains("TOML cannot represent null at /items/1"),
        "unexpected error body: {body}"
    );
    assert!(
        !body.contains("unsupported unit type"),
        "error should not leak serde unit type: {body}"
    );

    close_session(&agent, &base_url, handle);
}
