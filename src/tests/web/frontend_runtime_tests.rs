use std::future::IntoFuture;
use std::net::{IpAddr, SocketAddr};
use std::sync::mpsc::{self, Receiver};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use serde_json::{Value, json};
use ureq::Agent;

use super::harness::{reserve_port, test_http_agent, wait_until_ready};
use crate::web::session::ServeOptions;
use crate::{FrontendOptions, SchemaUI, SessionOutcome};

/// How long a test will wait for a session to end on its own before failing.
///
/// Deliberately a timeout rather than a plain `join`: the bug this guards
/// against is a shutdown that waits forever on an idle keep-alive connection,
/// and a hung `join` would hang the whole test binary instead of failing.
const SESSION_WATCHDOG: Duration = Duration::from_secs(10);

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
        SchemaUI::from_schema(schema).run(FrontendOptions::Web(ServeOptions::new(
            IpAddr::from([127, 0, 0, 1]),
            port,
        )))
    });

    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    let response = agent
        .post(format!("{base_url}/api/v1/exit"))
        .content_type("application/json")
        .send(
            serde_json::to_string(&json!({
                "data": expected.clone(),
                "commit": true
            }))
            .expect("serialize exit payload"),
        )
        .expect("post exit request");
    assert_eq!(response.status().as_u16(), 200);

    let result = handle
        .join()
        .expect("web frontend thread should not panic")
        .expect("web frontend should return the committed payload");
    assert_eq!(result, SessionOutcome::Completed(expected));
}

#[tokio::test(flavor = "multi_thread")]
async fn web_frontend_async_runner_works_inside_existing_runtime() {
    let port = reserve_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");
    let expected = json!({ "name": "bob" });
    let schema = json!({
        "type": "object",
        "properties": {
            "name": { "type": "string" }
        }
    });

    let task = tokio::spawn(async move {
        SchemaUI::from_schema(schema)
            .run_web_async(ServeOptions::new(IpAddr::from([127, 0, 0, 1]), port))
            .await
    });

    let wait_url = base_url.clone();
    tokio::task::spawn_blocking(move || {
        let agent = test_http_agent();
        wait_until_ready(&agent, &wait_url);
    })
    .await
    .expect("wait task should not panic");

    let exit_url = format!("{base_url}/api/v1/exit");
    let exit_payload = json!({
        "data": expected.clone(),
        "commit": true
    });
    let response = tokio::task::spawn_blocking(move || {
        test_http_agent()
            .post(exit_url)
            .content_type("application/json")
            .send(serde_json::to_string(&exit_payload).expect("serialize exit payload"))
    })
    .await
    .expect("exit task should not panic")
    .expect("post exit request");
    assert_eq!(response.status().as_u16(), 200);

    let result = task
        .await
        .expect("web frontend task should not panic")
        .expect("web frontend should return the committed payload");
    assert_eq!(result, SessionOutcome::Completed(expected));
}

/// Start a web session on its own thread, with a channel that fires when it ends.
///
/// The channel is what makes a hang visible: `join` would block forever, while
/// `recv_timeout` turns "never finished" into an ordinary assertion failure.
fn spawn_web_session(
    schema: Value,
    port: u16,
    timeout: Option<Duration>,
) -> (Receiver<()>, JoinHandle<anyhow::Result<SessionOutcome>>) {
    let (done_tx, done_rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let mut ui = SchemaUI::from_schema(schema);
        if let Some(timeout) = timeout {
            ui = ui.with_timeout(timeout);
        }
        let outcome = ui.run(FrontendOptions::Web(ServeOptions::new(
            IpAddr::from([127, 0, 0, 1]),
            port,
        )));
        let _ = done_tx.send(());
        outcome
    });
    (done_rx, handle)
}

fn fetch_session_payload(agent: &Agent, base_url: &str) -> Value {
    let mut response = agent
        .get(format!("{base_url}/api/v1/session"))
        .call()
        .expect("get session payload");
    assert_eq!(response.status().as_u16(), 200);
    let body = response
        .body_mut()
        .read_to_string()
        .expect("read session body");
    serde_json::from_str(&body).expect("session payload should be JSON")
}

fn post_exit(agent: &Agent, base_url: &str, data: Value) {
    let response = agent
        .post(format!("{base_url}/api/v1/exit"))
        .content_type("application/json")
        .send(
            serde_json::to_string(&json!({ "data": data, "commit": true }))
                .expect("serialize exit payload"),
        )
        .expect("post exit request");
    assert_eq!(response.status().as_u16(), 200);
}

fn simple_schema() -> Value {
    json!({
        "type": "object",
        "properties": { "name": { "type": "string" } }
    })
}

#[test]
fn web_session_ends_itself_when_the_deadline_passes() {
    let port = reserve_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");

    let (done_rx, handle) =
        spawn_web_session(simple_schema(), port, Some(Duration::from_millis(1_200)));

    // `wait_until_ready` leaves an idle keep-alive connection open in the agent
    // on purpose: a graceful shutdown that waits for idle connections to close
    // would never return here.
    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    done_rx
        .recv_timeout(SESSION_WATCHDOG)
        .expect("a session with a deadline must end itself, without a client request");

    let outcome = handle
        .join()
        .expect("web frontend thread should not panic")
        .expect("timing out is not an error");
    assert_eq!(outcome, SessionOutcome::TimedOut);
}

#[test]
fn web_session_reports_the_time_left_before_its_deadline() {
    let port = reserve_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");

    let (done_rx, handle) = spawn_web_session(simple_schema(), port, Some(Duration::from_secs(30)));

    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    let payload = fetch_session_payload(&agent, &base_url);
    let remaining = payload["expires_in_ms"]
        .as_u64()
        .expect("a bounded session should report expires_in_ms as a number");
    assert!(
        remaining <= 30_000,
        "reported {remaining}ms remaining, more than the 30000ms that was requested"
    );
    assert!(
        remaining > 25_000,
        "reported only {remaining}ms remaining; most of the budget went missing"
    );

    post_exit(&agent, &base_url, json!({ "name": "alice" }));
    done_rx
        .recv_timeout(SESSION_WATCHDOG)
        .expect("session should close after the exit request");
    handle
        .join()
        .expect("thread should not panic")
        .expect("no error");
}

#[test]
fn web_session_without_a_deadline_reports_no_expiry() {
    let port = reserve_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");

    let (done_rx, handle) = spawn_web_session(simple_schema(), port, None);

    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    let payload = fetch_session_payload(&agent, &base_url);
    assert!(
        payload["expires_in_ms"].is_null(),
        "an unbounded session must report no expiry, got {}",
        payload["expires_in_ms"]
    );

    post_exit(&agent, &base_url, json!({ "name": "bob" }));
    done_rx
        .recv_timeout(SESSION_WATCHDOG)
        .expect("session should close after the exit request");
    handle
        .join()
        .expect("thread should not panic")
        .expect("no error");
}

#[test]
fn finishing_before_the_deadline_still_reports_completed() {
    let port = reserve_port();
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let base_url = format!("http://{addr}");
    let expected = json!({ "name": "carol" });

    let (done_rx, handle) = spawn_web_session(simple_schema(), port, Some(Duration::from_secs(30)));

    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    // A save that lands before the deadline must not be relabelled as a
    // timeout: the two outcomes are the difference between a real answer and
    // none at all.
    post_exit(&agent, &base_url, expected.clone());

    done_rx
        .recv_timeout(SESSION_WATCHDOG)
        .expect("session should close after the exit request");
    let outcome = handle
        .join()
        .expect("web frontend thread should not panic")
        .expect("no error");
    assert_eq!(outcome, SessionOutcome::Completed(expected));
}

#[tokio::test(flavor = "multi_thread")]
async fn a_session_wired_through_session_router_also_times_out() {
    use crate::web::session::{WebSessionBuilder, session_router};
    use tokio::net::TcpListener;

    let config = WebSessionBuilder::new(simple_schema())
        .with_deadline(Some(Instant::now() + Duration::from_millis(600)))
        .build()
        .expect("build session config");

    // `session_router` is the documented extension point for mounting the UI on
    // a custom HTTP stack. A session wired this way must expire too — a
    // deadline that only works through `bind_session` would be a trap.
    let (router, handles) = session_router(config).expect("wire the session router");
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind ephemeral listener");
    let (result_rx, shutdown_rx) = handles.into_parts();

    tokio::spawn(
        axum::serve(listener, router.into_make_service())
            .with_graceful_shutdown(async move {
                let _ = shutdown_rx.await;
            })
            .into_future(),
    );

    let outcome = tokio::time::timeout(SESSION_WATCHDOG, result_rx)
        .await
        .expect("a router-mounted session must still end itself")
        .expect("the session should report an outcome");
    assert_eq!(outcome, SessionOutcome::TimedOut);
}
