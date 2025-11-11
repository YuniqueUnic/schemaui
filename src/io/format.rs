use std::fmt;

/// Supported data formats for input/output layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentFormat {
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
