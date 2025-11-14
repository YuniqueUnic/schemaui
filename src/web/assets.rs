use include_dir::{Dir, include_dir};

pub static DIST: Dir<'static> = include_dir!("$CARGO_MANIFEST_DIR/web/dist");

#[derive(Debug, Clone)]
pub struct EmbeddedAsset {
    pub path: String,
    pub mime: &'static str,
    pub contents: &'static [u8],
}

pub fn asset(path: &str) -> Option<EmbeddedAsset> {
    let normalized = normalize_path(path);
    let file = DIST.get_file(normalized)?;
    Some(EmbeddedAsset {
        path: normalized.to_string(),
        mime: mime_from_path(normalized),
        contents: file.contents(),
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_index_html() {
        let asset = asset("/").expect("index.html embedded");
        assert_eq!(asset.mime, "text/html; charset=utf-8");
        assert!(asset.contents.starts_with(b"<!doctype html>"));
    }
}
