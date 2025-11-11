use std::{fmt, path::Path};

/// Supported data formats for input/output layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DocumentFormat {
    #[default]
    Json,
    #[cfg(feature = "yaml")]
    Yaml,
    #[cfg(feature = "toml")]
    Toml,
}

impl fmt::Display for DocumentFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DocumentFormat::Json => write!(f, "json"),
            #[cfg(feature = "yaml")]
            DocumentFormat::Yaml => write!(f, "yaml"),
            #[cfg(feature = "toml")]
            DocumentFormat::Toml => write!(f, "toml"),
        }
    }
}

impl DocumentFormat {
    /// Parse a format keyword (json/yaml/toml) into a `DocumentFormat`.
    pub fn from_keyword(keyword: &str) -> Result<Self, String> {
        match keyword.to_ascii_lowercase().as_str() {
            "json" => Ok(DocumentFormat::Json),
            #[cfg(feature = "yaml")]
            "yaml" | "yml" => Ok(DocumentFormat::Yaml),
            #[cfg(feature = "toml")]
            "toml" => Ok(DocumentFormat::Toml),
            other => Err(format!(
                "unsupported format '{other}', available: {}",
                Self::keyword_list().join(", ")
            )),
        }
    }

    /// Try to infer a format from a file extension.
    pub fn from_extension(path: &Path) -> Option<Self> {
        let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
        match ext.as_str() {
            "json" => Some(DocumentFormat::Json),
            #[cfg(feature = "yaml")]
            "yaml" | "yml" => Some(DocumentFormat::Yaml),
            #[cfg(feature = "toml")]
            "toml" => Some(DocumentFormat::Toml),
            _ => None,
        }
    }

    pub fn keyword_list() -> Vec<&'static str> {
        #[allow(unused_mut)]
        let mut items = vec!["json"];
        #[cfg(feature = "yaml")]
        items.push("yaml");
        #[cfg(feature = "toml")]
        items.push("toml");
        items
    }
}
