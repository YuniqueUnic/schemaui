#[cfg(feature = "web")]
mod assets_tests;

#[cfg(feature = "web")]
mod contract_v1_tests;

#[cfg(feature = "web")]
mod frontend_runtime_tests;

#[cfg(feature = "web")]
mod preview_tests;
#[cfg(feature = "precompile")]
mod web_snapshot_tests;

#[cfg(feature = "web")]
mod theme_tests;

#[cfg(all(feature = "web", feature = "wasm"))]
mod wasm_parity_tests;

#[cfg(feature = "web")]
mod harness {
    //! Starting a real session and talking to it over HTTP.
    //!
    //! Shared rather than copied per test file: these helpers encode the
    //! session's readiness contract, and three drifting copies of "is it up
    //! yet" is how a suite ends up green against a server nobody is testing.

    use std::net::TcpListener;
    use std::thread;
    use std::time::{Duration, Instant};

    use ureq::Agent;

    /// An ephemeral port, released before the caller binds it.
    ///
    /// Racy in principle; in practice the kernel does not hand the same port
    /// out twice in the microseconds between these two calls, and the
    /// alternative — plumbing the bound address back out of a spawned thread —
    /// buys nothing a test needs.
    pub(super) fn reserve_port() -> u16 {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind ephemeral port");
        let port = listener.local_addr().expect("listener addr").port();
        drop(listener);
        port
    }

    pub(super) fn test_http_agent() -> Agent {
        Agent::new_with_config(
            Agent::config_builder()
                .http_status_as_error(false)
                .timeout_global(Some(Duration::from_secs(2)))
                .proxy(None)
                .build(),
        )
    }

    pub(super) fn wait_until_ready(agent: &Agent, base_url: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        let session_url = format!("{base_url}/api/v1/session");

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
}
