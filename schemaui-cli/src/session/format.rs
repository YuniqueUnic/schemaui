use std::path::Path;

use schemaui::DocumentFormat;

use super::diagnostics::DiagnosticCollector;

#[derive(Debug, Clone, Copy, Default)]
pub struct FormatHint {
    pub format: DocumentFormat,
    pub from_extension: bool,
}

impl FormatHint {
    pub fn extension_value(&self) -> Option<DocumentFormat> {
        self.from_extension.then_some(self.format)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FormatResolution {
    pub hint: FormatHint,
    pub blocked: bool,
}

pub fn resolve_format_hint(
    path_hint: Option<&str>,
    label: &str,
    diagnostics: &mut DiagnosticCollector,
) -> FormatResolution {
    if let Some(path) = path_hint
        && path != "-"
    {
        match probe_format_from_extension(Path::new(path)) {
            ExtensionFormat::Known(format) => {
                return FormatResolution {
                    hint: FormatHint {
                        format,
                        from_extension: true,
                    },
                    blocked: false,
                };
            }
            ExtensionFormat::UnsupportedFeature {
                format_name,
                feature_flag,
            } => {
                diagnostics.push_input(
                    label,
                    format!(
                        "{label} '{path}' requires {format_name} support, but this build lacks the '{feature_flag}' feature"
                    ),
                );
                return FormatResolution {
                    hint: FormatHint::default(),
                    blocked: true,
                };
            }
            ExtensionFormat::Unknown => {}
        }
    }

    FormatResolution {
        hint: FormatHint::default(),
        blocked: false,
    }
}

#[derive(Debug)]
pub enum ExtensionFormat {
    Known(DocumentFormat),
    #[allow(dead_code)]
    UnsupportedFeature {
        format_name: &'static str,
        feature_flag: &'static str,
    },
    Unknown,
}

pub fn probe_format_from_extension(path: &Path) -> ExtensionFormat {
    let Some(ext) = path.extension() else {
        return ExtensionFormat::Unknown;
    };
    let normalized = ext.to_string_lossy().to_ascii_lowercase();
    match normalized.as_str() {
        "json" => ExtensionFormat::Known(DocumentFormat::Json),
        #[cfg(feature = "yaml")]
        "yaml" | "yml" => ExtensionFormat::Known(DocumentFormat::Yaml),
        #[cfg(not(feature = "yaml"))]
        "yaml" | "yml" => ExtensionFormat::UnsupportedFeature {
            format_name: "yaml",
            feature_flag: "yaml",
        },
        #[cfg(feature = "toml")]
        "toml" => ExtensionFormat::Known(DocumentFormat::Toml),
        #[cfg(not(feature = "toml"))]
        "toml" => ExtensionFormat::UnsupportedFeature {
            format_name: "toml",
            feature_flag: "toml",
        },
        _ => ExtensionFormat::Unknown,
    }
}
