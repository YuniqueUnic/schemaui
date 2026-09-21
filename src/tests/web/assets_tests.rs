//! What the server is willing to hand out at paths the API does not claim.

use crate::web::assets::{EmbeddedAssets, FilesystemAssets, WebAssetProvider};

/// A directory this test owns outright, removed when it drops.
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

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn the_embedded_frontend_answers_the_root_path() {
    let asset = EmbeddedAssets.load("/").expect("index.html embedded");
    assert_eq!(asset.mime, "text/html; charset=utf-8");

    // Case-insensitive, because which case the minifier emits is its business.
    let lowered: Vec<u8> = asset
        .contents
        .iter()
        .map(|byte| byte.to_ascii_lowercase())
        .collect();
    let needle = b"<!doctype html>";
    assert!(
        lowered.windows(needle.len()).any(|window| window == needle),
        "embedded index.html must contain a DOCTYPE"
    );
}

#[test]
fn an_empty_path_means_the_index() {
    let asset = EmbeddedAssets.load("").expect("falls back to index");
    assert_eq!(asset.path, "index.html");
}

#[test]
fn a_directory_frontend_serves_its_own_files() {
    let dir = TempDir::new("assets-serve");
    std::fs::write(dir.path().join("index.html"), "<!doctype html><p>mine").expect("write index");
    std::fs::create_dir_all(dir.path().join("nested")).expect("create nested dir");
    std::fs::write(dir.path().join("nested/app.js"), "export default 1;").expect("write js");

    let assets = FilesystemAssets::new(dir.path());

    let index = assets.load("/").expect("index should be served");
    assert_eq!(index.mime, "text/html; charset=utf-8");
    assert_eq!(index.contents.as_ref(), b"<!doctype html><p>mine");

    let nested = assets.load("/nested/app.js").expect("nested file");
    assert_eq!(nested.mime, "text/javascript; charset=utf-8");
}

#[test]
fn a_directory_frontend_cannot_be_walked_out_of() {
    // The embedded provider is indexed by an exact key and never had this
    // problem. A directory on disk does, and this is the whole reason
    // `--frontend` could not simply be wired to the existing loader.
    let root = TempDir::new("assets-contained");
    std::fs::write(root.path().join("index.html"), "inside").expect("write index");

    let secret = root.path().join("secret.txt");
    std::fs::write(&secret, "outside").expect("write secret");

    let served = root.path().join("public");
    std::fs::create_dir_all(&served).expect("create served dir");
    std::fs::write(served.join("index.html"), "served").expect("write served index");

    let assets = FilesystemAssets::new(&served);
    assert!(
        assets.load("/index.html").is_some(),
        "sanity: root is served"
    );

    for escape in [
        "/../secret.txt",
        "/nested/../../secret.txt",
        "/./../secret.txt",
        "//../secret.txt",
    ] {
        assert!(
            assets.load(escape).is_none(),
            "{escape} escaped the served directory"
        );
    }
}

#[test]
fn a_path_that_only_traverses_resolves_to_nothing() {
    let dir = TempDir::new("assets-empty");
    std::fs::write(dir.path().join("index.html"), "inside").expect("write index");
    let assets = FilesystemAssets::new(dir.path());

    // Without the emptiness check these would join to the root itself, and
    // reading a directory is an error rather than a leak — but relying on that
    // makes the guarantee an accident of the filesystem.
    assert!(assets.load("/.").is_none());
    assert!(assets.load("/./").is_none());
}
