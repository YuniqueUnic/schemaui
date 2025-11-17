use std::fmt::Write as FmtWrite;
use std::path::{Path, PathBuf};

use color_eyre::eyre::{Result, eyre};
use schemaui::{
    DocumentFormat, OutputDestination, OutputOptions, schema_from_data_value, schema_with_defaults,
};
use serde_json::Value;

use crate::cli::CommonArgs;
use crate::io::load_value;

const DEFAULT_TEMP_FILE: &str = "/tmp/schemaui.json";

#[derive(Debug)]
pub struct SessionBundle {
    pub schema: Value,
    pub defaults: Option<Value>,
    pub title: Option<String>,
    pub output: Option<OutputOptions>,
}

pub fn prepare_session(args: &CommonArgs) -> Result<SessionBundle> {
    let mut diagnostics = DiagnosticCollector::default();

    let schema_spec = args.schema.as_deref();
    let config_spec = args.config.as_deref();
    let schema_stdin = schema_spec == Some("-");
    let config_stdin = config_spec == Some("-");
    if schema_stdin && config_stdin {
        diagnostics.push_input(
            "schema/config",
            "cannot read schema and config from stdin simultaneously; provide inline content or files",
        );
    }

    let schema_hint = resolve_format_hint(schema_spec, "schema", &mut diagnostics);
    let config_hint = resolve_format_hint(config_spec, "config", &mut diagnostics);

    let schema_value = load_optional_value(
        schema_spec,
        schema_hint.hint.format,
        "schema",
        schema_hint.blocked || (schema_stdin && config_stdin),
        &mut diagnostics,
    );
    let config_value = load_optional_value(
        config_spec,
        config_hint.hint.format,
        "config",
        config_hint.blocked || (schema_stdin && config_stdin),
        &mut diagnostics,
    );

    let (output_settings, output_paths) = build_output_options(
        args,
        config_hint.hint.extension_value(),
        schema_hint.hint.extension_value(),
        &mut diagnostics,
    );
    ensure_output_paths_available(&output_paths, args.force, &mut diagnostics);

    diagnostics.into_result()?;

    let mut schema_value = schema_value;
    let mut config_value = config_value;
    if schema_value.is_none() {
        if let Some(config_doc) = config_value.as_ref()
            && looks_like_json_schema(config_doc)
        {
            eprintln!(
                "detected JSON Schema provided via --config; treating it as the active schema"
            );
            schema_value = config_value.take();
        }
    }

    if schema_value.is_none() && config_value.is_none() {
        return Err(eyre!("provide at least --schema or --config"));
    }

    let schema = match (schema_value, config_value.as_ref()) {
        (Some(schema), Some(defaults)) => schema_with_defaults(&schema, defaults),
        (Some(schema), None) => schema,
        (None, Some(defaults)) => schema_from_data_value(defaults),
        (None, None) => unreachable!("validated above"),
    };

    Ok(SessionBundle {
        schema,
        defaults: config_value,
        title: args.title.clone(),
        output: output_settings,
    })
}

fn load_optional_value(
    spec: Option<&str>,
    format: DocumentFormat,
    label: &str,
    skip: bool,
    diagnostics: &mut DiagnosticCollector,
) -> Option<Value> {
    if skip {
        return None;
    }
    let Some(raw) = spec else {
        return None;
    };
    match load_value(raw, format, label) {
        Ok(value) => Some(value),
        Err(err) => {
            diagnostics.push_input(label, err.to_string());
            None
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct FormatHint {
    format: DocumentFormat,
    from_extension: bool,
}

impl Default for FormatHint {
    fn default() -> Self {
        Self {
            format: DocumentFormat::default(),
            from_extension: false,
        }
    }
}

impl FormatHint {
    fn extension_value(&self) -> Option<DocumentFormat> {
        self.from_extension.then_some(self.format)
    }
}

#[derive(Debug, Clone, Copy)]
struct FormatResolution {
    hint: FormatHint,
    blocked: bool,
}

fn resolve_format_hint(
    path_hint: Option<&str>,
    label: &str,
    diagnostics: &mut DiagnosticCollector,
) -> FormatResolution {
    if let Some(path) = path_hint {
        if path != "-" {
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
    }

    FormatResolution {
        hint: FormatHint::default(),
        blocked: false,
    }
}

fn build_output_options(
    args: &CommonArgs,
    config_hint: Option<DocumentFormat>,
    schema_hint: Option<DocumentFormat>,
    diagnostics: &mut DiagnosticCollector,
) -> (Option<OutputOptions>, Vec<PathBuf>) {
    let mut destinations = Vec::new();
    let explicit_outputs = !args.outputs.is_empty();

    for raw in &args.outputs {
        if raw.trim().is_empty() {
            diagnostics.push_output("output destination cannot be empty");
            continue;
        }
        if raw == "-" {
            destinations.push(OutputDestination::Stdout);
        } else {
            destinations.push(OutputDestination::file(raw));
        }
    }

    if destinations.is_empty() && !explicit_outputs {
        if args.no_temp_file {
            return (None, Vec::new());
        }
        let fallback = args
            .temp_file
            .clone()
            .unwrap_or_else(|| PathBuf::from(DEFAULT_TEMP_FILE));
        destinations.push(OutputDestination::file(fallback.clone()));
    }

    if destinations.is_empty() {
        return (None, Vec::new());
    }

    let file_paths: Vec<PathBuf> = destinations
        .iter()
        .filter_map(|dest| match dest {
            OutputDestination::File(path) => Some(path.clone()),
            OutputDestination::Stdout => None,
        })
        .collect();

    let start = diagnostics.len();
    let format = if file_paths.is_empty() {
        determine_stdout_format(config_hint, schema_hint)
    } else {
        infer_format_from_files(&file_paths, diagnostics)
            .or(config_hint)
            .or(schema_hint)
            .unwrap_or_else(DocumentFormat::default)
    };

    if diagnostics.len() > start {
        return (None, file_paths);
    }

    let options = OutputOptions {
        destinations,
        pretty: !args.no_pretty,
        format,
    };

    (Some(options), file_paths)
}

fn determine_stdout_format(
    config_hint: Option<DocumentFormat>,
    schema_hint: Option<DocumentFormat>,
) -> DocumentFormat {
    config_hint
        .or(schema_hint)
        .unwrap_or_else(DocumentFormat::default)
}

fn infer_format_from_files(
    file_paths: &[PathBuf],
    diagnostics: &mut DiagnosticCollector,
) -> Option<DocumentFormat> {
    let mut detected: Option<DocumentFormat> = None;
    for path in file_paths {
        match probe_format_from_extension(path) {
            ExtensionFormat::Known(format) => {
                if let Some(existing) = detected {
                    if existing != format {
                        diagnostics.push_output(format!(
                            "output file {} uses {format} but other destinations use {existing}; align extensions",
                            path.display()
                        ));
                    }
                } else {
                    detected = Some(format);
                }
            }
            ExtensionFormat::UnsupportedFeature {
                format_name,
                feature_flag,
            } => diagnostics.push_output(format!(
                "output file {} requires {format_name} support, but this build was compiled without the '{feature_flag}' feature",
                path.display()
            )),
            ExtensionFormat::Unknown => diagnostics.push_output(format!(
                "cannot infer format from output file {}; use .json/.yaml/.toml",
                path.display()
            )),
        }
    }
    detected
}

fn probe_format_from_extension(path: &Path) -> ExtensionFormat {
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

#[derive(Debug)]
enum ExtensionFormat {
    Known(DocumentFormat),
    #[allow(dead_code)]
    UnsupportedFeature {
        format_name: &'static str,
        feature_flag: &'static str,
    },
    Unknown,
}

fn ensure_output_paths_available(
    paths: &[PathBuf],
    force: bool,
    diagnostics: &mut DiagnosticCollector,
) {
    if force {
        return;
    }
    for path in paths {
        if path.exists() {
            diagnostics.push_output(format!(
                "file {} already exists (pass --force to overwrite)",
                path.display()
            ));
        }
    }
}

fn looks_like_json_schema(value: &Value) -> bool {
    let obj = match value.as_object() {
        Some(map) => map,
        None => return false,
    };

    if obj
        .get("properties")
        .and_then(Value::as_object)
        .map(|props| props.len())
        .unwrap_or(0)
        == 0
    {
        return false;
    }

    if obj.contains_key("$schema") {
        return true;
    }

    if matches!(obj.get("type"), Some(Value::String(t)) if t == "object") {
        return true;
    }

    if let Some(props) = obj.get("properties").and_then(Value::as_object) {
        let mut scored = 0usize;

        for value in props.values() {
            if value.get("type").is_some() {
                scored += 1;
            }
            if value.get("properties").is_some() {
                scored += 1;
            }
            if value.get("enum").is_some() {
                scored += 1;
            }
        }

        return scored >= 2;
    }

    false
}

#[derive(Debug, Default)]
struct DiagnosticCollector {
    messages: Vec<String>,
}

impl DiagnosticCollector {
    fn push_input(&mut self, label: &str, message: impl Into<String>) {
        self.messages.push(format!("{label}: {}", message.into()));
    }

    fn push_output(&mut self, message: impl Into<String>) {
        self.messages.push(format!("output: {}", message.into()));
    }

    fn len(&self) -> usize {
        self.messages.len()
    }

    fn into_result(self) -> Result<()> {
        if self.messages.is_empty() {
            return Ok(());
        }
        let mut body = String::from("encountered input/output issues:\n");
        for (idx, msg) in self.messages.iter().enumerate() {
            let _ = writeln!(body, "  {}. {}", idx + 1, msg);
        }
        Err(eyre!(body))
    }
}
