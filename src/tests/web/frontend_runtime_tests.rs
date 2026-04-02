use std::net::{IpAddr, SocketAddr, TcpListener};
use std::thread;
use std::time::{Duration, Instant};

use reqwest::StatusCode;
use reqwest::blocking::Client;
use serde_json::json;

use crate::SchemaUI;
use crate::web::session::ServeOptions;

fn reserve_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
    let port = listener.local_addr().expect("listener addr").port();
    drop(listener);
    port
}

fn wait_until_ready(client: &Client, base_url: &str) {
    let deadline = Instant::now() + Duration::from_secs(5);
    let session_url = format!("{base_url}/api/session");

    loop {
        let outcome = match client.get(&session_url).send() {
            Ok(response) if response.status() == StatusCode::OK => return,
            Ok(response) => format!("unexpected status: {}", response.status()),
            Err(err) => err.to_string(),
        };

        assert!(
            Instant::now() < deadline,
            "web session did not become ready at {base_url}; last outcome: {}",
            outcome
        );
        thread::sleep(Duration::from_millis(50));
    }
}

#[test]
fn web_frontend_exit_does_not_panic_when_runtime_is_dropped() {
    let port = reserve_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");
    let expected = json!({ "name": "alice" });
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        }
    });

    let handle = thread::spawn(move || {
        SchemaUI::new(schema).run_web(ServeOptions {
            host: IpAddr::from([127, 0, 0, 1]),
            port,
        })
    });

    let client = Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(2))
        .build()
        .expect("build blocking reqwest client");
    wait_until_ready(&client, &base_url);

    let response = client
        .post(format!("{base_url}/api/exit"))
        .json(&json!({
            "data": expected.clone(),
            "commit": true
        }))
        .send()
        .expect("post exit request");
    assert_eq!(response.status(), StatusCode::OK);

    let result = handle
        .join()
        .expect("web frontend thread should not panic")
        .expect("web frontend should return the committed payload");
    assert_eq!(result, expected);
}
