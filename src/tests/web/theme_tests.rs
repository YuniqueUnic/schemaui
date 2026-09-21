//! Loading a user stylesheet, and serving it.

use std::net::IpAddr;
use std::thread;

use serde_json::{Value, json};

use super::harness::{reserve_port, test_http_agent, wait_until_ready};
use crate::web::session::ServeOptions;
use crate::web::theme::Theme;
use crate::{FrontendOptions, SchemaUI};

/// A directory this test owns outright, removed when it drops even if the
/// assertions fail.
struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new(label: &str) -> Self {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock is after the epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("schemaui-{label}-{unique}"));
        std::fs::create_dir_all(&path).expect("create temp dir");
        Self(path)
    }

    fn join(&self, name: &str) -> std::path::PathBuf {
        self.0.join(name)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn a_stylesheet_is_read_verbatim() {
    let dir = TempDir::new("theme-read");
    let path = dir.join("mine.css");
    std::fs::write(&path, ":root { --color-primary: hotpink; }").expect("write theme");

    let theme = Theme::from_path(&path).expect("theme should load");
    assert_eq!(theme.css(), ":root { --color-primary: hotpink; }");
    assert!(theme.origin().ends_with("mine.css"));
}

#[test]
fn a_missing_stylesheet_is_an_error_rather_than_an_empty_one() {
    let dir = TempDir::new("theme-missing");
    let err = Theme::from_path(dir.join("absent.css")).expect_err("missing file should fail");
    assert!(
        format!("{err:#}").contains("theme file not found"),
        "the message has to name the problem, since the CLI only prints it: {err:#}"
    );
}

#[test]
fn a_directory_is_not_a_stylesheet() {
    let dir = TempDir::new("theme-dir");
    let nested = dir.join("styles");
    std::fs::create_dir_all(&nested).expect("create nested dir");

    let err = Theme::from_path(&nested).expect_err("a directory should fail");
    assert!(format!("{err:#}").contains("not a regular file"));
}

#[test]
fn an_oversized_stylesheet_is_refused_before_it_is_read() {
    // Guards the `--web-theme /dev/urandom` mistake: without a ceiling the
    // process reads until it dies, and the user never sees a form.
    let dir = TempDir::new("theme-huge");
    let path = dir.join("huge.css");
    std::fs::write(&path, vec![b'a'; 5 * 1024 * 1024 + 1]).expect("write oversized theme");

    let err = Theme::from_path(&path).expect_err("oversized file should fail");
    assert!(format!("{err:#}").contains("over the"));
}

#[test]
fn a_themed_session_advertises_and_serves_it() {
    let dir = TempDir::new("theme-serve");
    let path = dir.join("brand.css");
    std::fs::write(&path, ":root { --color-primary: rebeccapurple; }").expect("write theme");
    let theme = Theme::from_path(&path).expect("theme should load");

    let port = reserve_port();
    let base_url = format!("http://127.0.0.1:{port}");
    let schema = json!({ "type": "object", "properties": { "name": { "type": "string" } } });

    let handle = thread::spawn(move || {
        let serve = ServeOptions::new(IpAddr::from([127, 0, 0, 1]), port).with_theme(Some(theme));
        SchemaUI::from_schema(schema).run(FrontendOptions::Web(serve))
    });

    let agent = test_http_agent();
    wait_until_ready(&agent, &base_url);

    let raw = agent
        .get(format!("{base_url}/api/v1/session"))
        .call()
        .expect("session request")
        .body_mut()
        .read_to_string()
        .expect("session body should be readable");
    let session: Value = serde_json::from_str(&raw).expect("session JSON");
    assert!(
        session["capabilities"]
            .as_array()
            .expect("capabilities")
            .iter()
            .any(|capability| capability == "theme"),
        "the frontend links the stylesheet off this capability; without it the \
         theme is served to nobody"
    );

    let mut response = agent
        .get(format!("{base_url}/api/v1/theme.css"))
        .call()
        .expect("theme request");
    assert_eq!(response.status().as_u16(), 200);
    assert_eq!(
        response
            .headers()
            .get("content-type")
            .and_then(|value| value.to_str().ok()),
        Some("text/css; charset=utf-8")
    );
    assert_eq!(
        response.body_mut().read_to_string().expect("theme body"),
        ":root { --color-primary: rebeccapurple; }"
    );

    agent
        .post(format!("{base_url}/api/v1/exit"))
        .content_type("application/json")
        .send(r#"{"data":{},"commit":true}"#)
        .expect("post exit request");
    handle
        .join()
        .expect("session thread should not panic")
        .expect("session should end cleanly");
}

/// `examples/themes/midnight.css` is the worked example `--web-theme` docs
/// point to. A regression guard, not a design test: this fails loudly if the
/// file is ever renamed or deleted out from under the docs that reference it,
/// rather than that going unnoticed until someone tries the command.
#[test]
fn the_example_theme_loads() {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/themes/midnight.css");
    let theme = Theme::from_path(&path).expect("the example theme should load");
    assert!(theme.css().contains("--color-primary"));
}

/// `examples/frontend/` is the worked example for `--frontend`. Same
/// reasoning as above: this exists to catch the directory going missing or
/// losing its `index.html`, not to test `FilesystemAssets` itself.
#[test]
fn the_example_frontend_has_an_entry_point() {
    use crate::web::assets::{FilesystemAssets, WebAssetProvider};

    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/frontend");
    let assets = FilesystemAssets::new(root);
    let index = assets.load("/").expect("index.html should be served at /");
    assert!(
        String::from_utf8_lossy(&index.contents).contains("/api/v1/session"),
        "the example frontend should actually call the contract it demonstrates"
    );
}
