use std::{
    borrow::Cow,
    fs,
    path::{Path, PathBuf},
};

use include_dir::{Dir, include_dir};

pub static DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/web/dist");

#[derive(Debug, Clone)]
pub struct AssetResponse {
    pub path: String,
    pub mime: &'static str,
    pub contents: Cow<'static, [u8]>,
}

pub trait WebAssetProvider: Send + Sync + std::fmt::Debug + 'static {
    fn load(&self, path: &str) -> Option<AssetResponse>;
}

#[derive(Debug, Default, Clone)]
pub struct EmbeddedAssets;

impl WebAssetProvider for EmbeddedAssets {
    fn load(&self, path: &str) -> Option<AssetResponse> {
        let normalized = normalize_path(path);
        let file = DIST.get_file(normalized)?;
        Some(AssetResponse {
            path: normalized.to_string(),
            mime: mime_from_path(normalized),
            contents: Cow::Borrowed(file.contents()),
        })
    }
}

#[derive(Debug, Clone)]
pub struct FilesystemAssets {
    root: PathBuf,
}

impl FilesystemAssets {
    pub fn new<P: Into<PathBuf>>(root: P) -> Self {
        Self { root: root.into() }
    }

    pub fn root(&self) -> &Path {
        self.root.as_path()
    }
}

impl WebAssetProvider for FilesystemAssets {
    fn load(&self, path: &str) -> Option<AssetResponse> {
        let normalized = normalize_path(path);
        let relative = contained_relative_path(normalized)?;
        let contents = fs::read(self.root.join(&relative)).ok()?;
        Some(AssetResponse {
            path: normalized.to_string(),
            mime: mime_from_path(normalized),
            contents: Cow::Owned(contents),
        })
    }
}

/// Resolve a request path to something that cannot escape the served root.
///
/// The embedded provider is indexed by an exact key and never had this
/// problem; a directory on disk does. `..` is rejected outright rather than
/// popped, because a request containing one is a request for a file the
/// frontend was never meant to reach — resolving it to something else would
/// answer a question nobody asked.
fn contained_relative_path(path: &str) -> Option<PathBuf> {
    use std::path::Component;

    let candidate = Path::new(path);
    if candidate.is_absolute() {
        return None;
    }

    let mut relative = PathBuf::new();
    for component in candidate.components() {
        match component {
            Component::Normal(part) => relative.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }

    (!relative.as_os_str().is_empty()).then_some(relative)
}

pub fn embedded_asset(path: &str) -> Option<AssetResponse> {
    EmbeddedAssets.load(path)
}

fn normalize_path(raw: &str) -> &str {
    let trimmed = raw.trim_start_matches('/');
    if trimmed.is_empty() {
        return "index.html";
    }
    trimmed
}

fn mime_from_path(path: &str) -> &'static str {
    match path.rsplit('.').next().map(|ext| ext.to_ascii_lowercase()) {
        Some(ref ext) if ext == "html" => "text/html; charset=utf-8",
        Some(ref ext) if ext == "css" => "text/css; charset=utf-8",
        Some(ref ext) if ext == "js" => "text/javascript; charset=utf-8",
        Some(ref ext) if ext == "json" => "application/json; charset=utf-8",
        Some(ref ext) if ext == "svg" => "image/svg+xml",
        _ => "application/octet-stream",
    }
}
